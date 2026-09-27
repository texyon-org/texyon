use super::*;
use crate::resource::{ResourceKind, ResourceRequest};
use crate::testxdv::XdvBuilder;
use crate::{Color, LinkAction, Matrix, Path, PositionedChar, PositionedGlyph, StrokeStyle};
use sha2::{Digest, Sha256};
use std::cell::RefCell;

const ONE_PAGE: &[u8] = include_bytes!("../../../../tests/tex-outputs/the_letter_a.xdv");
const IMAGE_PAGE: &[u8] = include_bytes!("../../../../tests/tex-outputs/redbox_png.xdv");

struct MutableResources(RefCell<Vec<u8>>);

struct ClassicResources;
struct BuiltinClassicResources;
struct MappedClassicResources;

impl ResourceProvider for MutableResources {
    fn resolve(&self, _request: &ResourceRequest) -> Result<Option<Resource>, RenderError> {
        Ok(Some(Resource::from_bytes(self.0.borrow().clone())))
    }
}

impl ResourceProvider for ClassicResources {
    fn resolve(&self, request: &ResourceRequest) -> Result<Option<Resource>, RenderError> {
        let bytes = match request.kind {
            ResourceKind::Tfm => {
                let mut bytes = crate::tfm::tests::synthetic_tfm();
                bytes[4..6].copy_from_slice(&97u16.to_be_bytes());
                bytes[6..8].copy_from_slice(&97u16.to_be_bytes());
                bytes
            }
            ResourceKind::FontProgram => b"font-program".to_vec(),
            ResourceKind::Encoding => b"/Encoding [ /A ] def".to_vec(),
            _ => return Ok(None),
        };
        Ok(Some(Resource::from_bytes(bytes)))
    }
}

impl ResourceProvider for BuiltinClassicResources {
    fn resolve(&self, request: &ResourceRequest) -> Result<Option<Resource>, RenderError> {
        let bytes: &[u8] = match request.kind {
            ResourceKind::Tfm => include_bytes!("../../../../tests/assets/cmr10.tfm"),
            ResourceKind::FontProgram => include_bytes!("../../../../tests/assets/cmr10.pfb"),
            ResourceKind::Encoding => return Ok(None),
            _ => return Ok(None),
        };
        Ok(Some(Resource::from_bytes(bytes)))
    }
}

impl ResourceProvider for MappedClassicResources {
    fn resolve(&self, request: &ResourceRequest) -> Result<Option<Resource>, RenderError> {
        let bytes: Option<&[u8]> = match (request.kind, request.name.as_str()) {
            (ResourceKind::FontMap, "pdftex.map") => {
                Some(b"cmr10 CMR10 \"TeXBase1Encoding ReEncodeFont\" <mapped.enc <mapped.pfb\n")
            }
            (ResourceKind::Tfm, "cmr10") => {
                Some(include_bytes!("../../../../tests/assets/cmr10.tfm"))
            }
            (ResourceKind::FontProgram, "mapped.pfb") => {
                Some(include_bytes!("../../../../tests/assets/cmr10.pfb"))
            }
            (ResourceKind::Encoding, "mapped.enc") => Some(b"/Encoding [ /A ] def"),
            _ => None,
        };
        Ok(bytes.map(Resource::from_bytes))
    }
}

fn rule_xdv(color_special: &str) -> Arc<[u8]> {
    let mut xdv = XdvBuilder::new(1000);
    xdv.begin_page();
    xdv.special(color_special)
        .set_rule(65_536, 131_072)
        .end_page();
    Arc::from(xdv.finish())
}

/// One rule per page, so documents can share bytes up to a chosen page.
fn paged_xdv(widths: &[i32]) -> (Arc<[u8]>, Vec<u32>) {
    let mut xdv = XdvBuilder::new(1000);
    let offsets = widths
        .iter()
        .map(|&width| {
            let offset = xdv.begin_page();
            xdv.set_rule(65_536, width).end_page();
            offset
        })
        .collect();
    (Arc::from(xdv.finish()), offsets)
}

#[test]
fn resumed_analysis_matches_a_complete_walk_of_the_same_document() {
    let pages = 40;
    let first: Vec<i32> = (0..pages).map(|index| 1_000 + index).collect();
    let mut second = first.clone();
    for width in second.iter_mut().skip(33) {
        *width += 7;
    }
    let (before, before_offsets) = paged_xdv(&first);
    let (after, after_offsets) = paged_xdv(&second);
    assert_eq!(
        before_offsets, after_offsets,
        "page geometry must be stable"
    );

    let mut warm = HotRenderer::new(ClassicResources);
    let baseline = warm
        .analyze_from(Arc::clone(&before), &before_offsets)
        .unwrap();
    assert_eq!(baseline.reused_pages(), 0);
    let resumed = warm
        .analyze_from(Arc::clone(&after), &after_offsets)
        .unwrap();

    let mut cold = HotRenderer::new(ClassicResources);
    let complete = cold
        .analyze_from(Arc::clone(&after), &after_offsets)
        .unwrap();
    assert_eq!(complete.reused_pages(), 0);

    assert!(
        resumed.reused_pages() >= 24,
        "expected substantial reuse, got {}",
        resumed.reused_pages()
    );
    assert_eq!(resumed.pages(), complete.pages());
    assert_eq!(resumed.ledger(), complete.ledger());
    assert_eq!(resumed.plans.len(), complete.plans.len());
    for (index, (left, right)) in resumed.plans.iter().zip(&complete.plans).enumerate() {
        assert_eq!(
            page_digest(left),
            page_digest(right),
            "page {index} differs"
        );
    }
}

#[test]
fn reuse_survives_an_edit_that_moves_back_up_the_document() {
    let pages = 40;
    let base: Vec<i32> = (0..pages).map(|index| 1_000 + index).collect();
    let mut deep = base.clone();
    deep[35] += 7;
    let mut shallow = base.clone();
    shallow[12] += 9;

    let (first, first_offsets) = paged_xdv(&base);
    let (second, second_offsets) = paged_xdv(&deep);
    let (third, third_offsets) = paged_xdv(&shallow);

    let mut warm = HotRenderer::new(ClassicResources);
    warm.analyze_from(first, &first_offsets).unwrap();
    let deep_walk = warm.analyze_from(second, &second_offsets).unwrap();
    assert!(deep_walk.reused_pages() >= 24);
    let shallow_walk = warm
        .analyze_from(Arc::clone(&third), &third_offsets)
        .unwrap();
    assert!(
        shallow_walk.reused_pages() >= 8,
        "an edit above the last resume point lost all reuse: {}",
        shallow_walk.reused_pages()
    );

    let mut cold = HotRenderer::new(ClassicResources);
    let complete = cold.analyze_from(third, &third_offsets).unwrap();
    assert_eq!(shallow_walk.pages(), complete.pages());
    assert_eq!(shallow_walk.ledger(), complete.ledger());
}

#[test]
fn a_changed_leading_page_defeats_reuse() {
    let pages = 40;
    let first: Vec<i32> = (0..pages).map(|index| 1_000 + index).collect();
    let mut second = first.clone();
    second[0] += 3;
    let (before, offsets) = paged_xdv(&first);
    let (after, after_offsets) = paged_xdv(&second);

    let mut warm = HotRenderer::new(ClassicResources);
    warm.analyze_from(before, &offsets).unwrap();
    let resumed = warm
        .analyze_from(Arc::clone(&after), &after_offsets)
        .unwrap();
    assert_eq!(resumed.reused_pages(), 0);

    let mut cold = HotRenderer::new(ClassicResources);
    let complete = cold.analyze_from(after, &after_offsets).unwrap();
    assert_eq!(resumed.pages(), complete.pages());
}

/// Two specials with a rule between them, so they sit at different positions.
fn bracketed_link_xdv(open: &str, close: &str) -> Arc<[u8]> {
    let mut xdv = XdvBuilder::new(1000);
    xdv.begin_page();
    xdv.special(open)
        .set_rule(655_360, 1_310_720)
        .special(close)
        .end_page();
    Arc::from(xdv.finish())
}

fn special_xdv(specials: &[&str]) -> Arc<[u8]> {
    Arc::from(XdvBuilder::specials_page(specials))
}

fn pdf_code_xdv(specials: &[&str]) -> Arc<[u8]> {
    let owned: Vec<_> = specials
        .iter()
        .map(|special| format!("pdf:code {special}"))
        .collect();
    let borrowed: Vec<_> = owned.iter().map(String::as_str).collect();
    special_xdv(&borrowed)
}

fn wrap_page_in_transform(xdv: &[u8], matrix: &str) -> Arc<[u8]> {
    let mut bytes = xdv.to_vec();
    let bop = bytes.iter().position(|byte| *byte == 139).unwrap();
    let content = bop + 45;
    let eop = bytes[content..]
        .iter()
        .position(|byte| *byte == 140)
        .map(|offset| content + offset)
        .unwrap();
    let begin = format!("pdf:btrans matrix {matrix}");
    let special = |text: &str| {
        let mut encoded = vec![239, text.len() as u8];
        encoded.extend_from_slice(text.as_bytes());
        encoded
    };
    let begin = special(&begin);
    let end = special("pdf:etrans");
    bytes.splice(eop..eop, end);
    bytes.splice(content..content, begin);

    let post = bytes.iter().position(|byte| *byte == 248).unwrap();
    let post_post = bytes.iter().position(|byte| *byte == 249).unwrap();
    bytes[post_post + 1..post_post + 5].copy_from_slice(&(post as u32).to_be_bytes());
    Arc::from(bytes)
}

fn styled_native_xdv() -> Arc<[u8]> {
    let mut define = vec![252];
    define.extend(0i32.to_be_bytes());
    define.extend((10i32 << 16).to_be_bytes());
    define.extend(0x7300u16.to_be_bytes());
    define.push(4);
    define.extend(b"font");
    define.extend(2u32.to_be_bytes());
    define.extend(0xff000080u32.to_be_bytes());
    define.extend((2u32 << 16).to_be_bytes());
    define.extend((-16_384i32).to_be_bytes());
    define.extend(6_554u32.to_be_bytes());
    let mut glyphs = vec![253];
    glyphs.extend((12i32 << 16).to_be_bytes());
    glyphs.extend(1u16.to_be_bytes());
    glyphs.extend(0i32.to_be_bytes());
    glyphs.extend(0i32.to_be_bytes());
    glyphs.extend(34u16.to_be_bytes());

    let mut xdv = XdvBuilder::new(1000);
    xdv.begin_page();
    xdv.raw(&define).raw(&glyphs).end_page();
    Arc::from(xdv.finish())
}

#[test]
fn real_xdv_is_analyzed_conservatively_without_classic_font_support() {
    let mut renderer = HotRenderer::new(());
    let analysis = renderer.analyze(Arc::from(ONE_PAGE)).unwrap();
    assert_eq!(analysis.pages().len(), 1);
    assert!(matches!(
        analysis.pages()[0].fingerprint,
        PageFingerprint::Uncertain { page_index: 0, .. }
    ));
    assert!(analysis.ledger().unsupported().contains_key("missing-tfm"));
}

#[test]
fn uncertain_pages_never_compare_equal_across_analyses() {
    let mut renderer = HotRenderer::new(());
    let first = renderer.analyze(Arc::from(ONE_PAGE)).unwrap();
    let second = renderer.analyze(Arc::from(ONE_PAGE)).unwrap();
    assert_ne!(first.pages()[0].fingerprint, second.pages()[0].fingerprint);
}

#[test]
fn resource_identity_uses_exact_bytes() {
    let resource = Resource::from_bytes(b"font".to_vec());
    let expected: [u8; 32] = Sha256::digest(b"font").into();
    assert_eq!(resource.identity, ResourceIdentity(expected));
}

#[test]
fn duplicate_or_missing_page_selection_is_transactional() {
    let mut renderer = HotRenderer::new(());
    let analysis = renderer.analyze(Arc::from(ONE_PAGE)).unwrap();
    let mut backend = crate::NullBackend;
    assert!(renderer
        .render_pages(&analysis, &[0, 0], &mut backend)
        .is_err());
    assert!(renderer.last_completed().is_none());
    assert!(renderer
        .render_pages(&analysis, &[1], &mut backend)
        .is_err());
    assert!(renderer.last_completed().is_none());
    assert_eq!(
        renderer
            .render_pages(&analysis, &[0], &mut backend)
            .unwrap()
            .len(),
        1
    );
    assert!(renderer.last_completed().is_some());
}

#[test]
fn same_name_image_overwrite_changes_the_analyzed_identity() {
    let mut renderer = HotRenderer::new(MutableResources(RefCell::new(b"first".to_vec())));
    let first = renderer.analyze(Arc::from(IMAGE_PAGE)).unwrap();
    let first_identity = first.plans[0]
        .operations
        .iter()
        .find_map(|operation| match operation {
            DrawOperation::Image { identity, .. } => Some(*identity),
            _ => None,
        })
        .unwrap();
    renderer.provider.0.replace(b"second".to_vec());
    let second = renderer.analyze(Arc::from(IMAGE_PAGE)).unwrap();
    let second_identity = second.plans[0]
        .operations
        .iter()
        .find_map(|operation| match operation {
            DrawOperation::Image { identity, .. } => Some(*identity),
            _ => None,
        })
        .unwrap();

    assert_ne!(first_identity, second_identity);
    assert_eq!(&*first.resources[&first_identity].bytes, b"first");
    assert_eq!(&*second.resources[&second_identity].bytes, b"second");
}

#[test]
fn drawing_color_participates_in_a_stable_fingerprint() {
    let mut renderer = HotRenderer::new(());
    let red = renderer.analyze(rule_xdv("color rgb 1 0 0")).unwrap();
    let blue = renderer
        .analyze(rule_xdv("pdfcolorstack 0 set (0 0 1 rg)"))
        .unwrap();
    assert!(matches!(
        red.pages()[0].fingerprint,
        PageFingerprint::Stable(_)
    ));
    assert!(matches!(
        blue.pages()[0].fingerprint,
        PageFingerprint::Stable(_)
    ));
    assert_ne!(red.pages()[0].fingerprint, blue.pages()[0].fingerprint);
}

#[test]
fn pdf_code_fragments_form_a_stable_clipped_path_plan() {
    let mut renderer = HotRenderer::new(());
    let analysis = renderer
        .analyze(pdf_code_xdv(&[
            "q",
            "1 0 0 rg",
            "0 0 10 10 re",
            "W n",
            "0 0 20 20 re f",
            "Q",
        ]))
        .unwrap();
    assert!(matches!(
        analysis.pages()[0].fingerprint,
        PageFingerprint::Stable(_)
    ));
    assert!(analysis.pages()[0].unsupported.is_empty());
    assert!(matches!(
        analysis.plans[0].operations.as_slice(),
        [
            DrawOperation::ClipPath { .. },
            DrawOperation::PathFill { .. },
            DrawOperation::PopClip
        ]
    ));
}

#[test]
fn wrapper_transforms_compose_into_images_and_restore() {
    let mut renderer = HotRenderer::new(MutableResources(RefCell::new(b"image".to_vec())));
    let analysis = renderer
        .analyze(special_xdv(&[
            "pdf:btrans matrix 2 0 0 3 4 5",
            "x:scale 10 20",
            "pdf:image (figure.png)",
            "pdf:etrans",
            "pdf:image (figure.png)",
        ]))
        .unwrap();
    let transforms: Vec<_> = analysis.plans[0]
        .operations
        .iter()
        .filter_map(|operation| match operation {
            DrawOperation::Image { transform, .. } => Some(*transform),
            _ => None,
        })
        .collect();
    assert_eq!(transforms.len(), 2);
    assert_eq!(
        (
            transforms[0].a,
            transforms[0].d,
            transforms[0].e,
            transforms[0].f,
        ),
        (20.0, -60.0, 76.0, 67.0)
    );
    assert_eq!(
        (
            transforms[1].a,
            transforms[1].d,
            transforms[1].e,
            transforms[1].f,
        ),
        (1.0, -1.0, 72.0, 72.0)
    );
    assert!(analysis.pages()[0].unsupported.is_empty());
}

#[test]
fn pdf_color_stack_initializes_fill_and_stroke_independently() {
    let mut renderer = HotRenderer::new(());
    let analysis = renderer
        .analyze(special_xdv(&[
            "pdfcolorstackinit 0 (1 0 0 rg 0 0 1 RG)",
            "pdf:code 0 0 10 10 re B",
        ]))
        .unwrap();
    let [DrawOperation::PathFill { color: fill, .. }, DrawOperation::PathStroke { color: line, .. }] =
        analysis.plans[0].operations.as_slice()
    else {
        panic!("expected one fill and one stroke");
    };
    assert_eq!(*fill, Color::rgb(1.0, 0.0, 0.0));
    assert_eq!(*line, Color::rgb(0.0, 0.0, 1.0));
    assert!(analysis.pages()[0].unsupported.is_empty());
}

#[test]
fn nonvisual_pdf_specials_do_not_make_pages_uncertain() {
    let mut renderer = HotRenderer::new(());
    let analysis = renderer
        .analyze(special_xdv(&[
            "pdf:bann<</Type/Annot/Subtype/Link>>",
            "pdf:eann",
            "pdf:docinfo<</Title(Test)>>",
        ]))
        .unwrap();
    assert!(analysis.plans[0].operations.is_empty());
    assert!(analysis.pages()[0].unsupported.is_empty());
    assert!(matches!(
        analysis.pages()[0].fingerprint,
        PageFingerprint::Stable(_)
    ));
    assert!(analysis.pages()[0].links.is_empty());
    assert_eq!(analysis.ledger().supported().get("pdf-nonvisual"), Some(&3));
}

#[test]
fn a_destination_is_recorded_where_its_special_sits() {
    let mut renderer = HotRenderer::new(());
    let analysis = renderer
        .analyze(special_xdv(&[
            "pdf:dest (section.1) [@thispage /XYZ @xpos @ypos null]",
        ]))
        .unwrap();

    let [destination] = analysis.pages()[0].destinations.as_slice() else {
        panic!("expected exactly one destination");
    };
    assert_eq!(destination.name, "section.1");
    assert!(analysis.plans[0].operations.is_empty());
    assert!(matches!(
        analysis.pages()[0].fingerprint,
        PageFingerprint::Stable(_)
    ));
}

#[test]
fn a_link_takes_its_width_from_the_specials_that_bracket_it() {
    let mut renderer = HotRenderer::new(());
    let analysis = renderer
        .analyze(bracketed_link_xdv(
            "pdf:bann<</Type/Annot/Subtype/Link/A<</S/URI/URI(https://x.test)>>>>",
            "pdf:eann",
        ))
        .unwrap();

    let [link] = analysis.pages()[0].links.as_slice() else {
        panic!(
            "expected exactly one link, got {:?}",
            analysis.pages()[0].links
        );
    };
    assert_eq!(link.action, LinkAction::Uri("https://x.test".into()));
    assert!(
        link.rect.right > link.rect.left,
        "link has no width: {:?}",
        link.rect
    );
    assert!(
        link.rect.bottom > link.rect.top,
        "link has no height: {:?}",
        link.rect
    );
}

#[test]
fn an_annotation_left_open_is_closed_by_the_page() {
    let mut renderer = HotRenderer::new(());
    let analysis = renderer
        .analyze(bracketed_link_xdv(
            "pdf:bann<</A<</S/GoTo/D(section.1)>>>>",
            "pdf:docinfo<</Title(Test)>>",
        ))
        .unwrap();

    let [link] = analysis.pages()[0].links.as_slice() else {
        panic!("expected the dangling annotation to be closed at the page end");
    };
    assert_eq!(link.action, LinkAction::Destination("section.1".into()));
}

#[test]
fn a_link_changes_a_page_that_looks_otherwise_identical() {
    let mut renderer = HotRenderer::new(());
    let first = renderer
        .analyze(bracketed_link_xdv(
            "pdf:bann<</A<</S/URI/URI(https://one.test)>>>>",
            "pdf:eann",
        ))
        .unwrap()
        .pages()[0]
        .fingerprint
        .clone();
    let second = renderer
        .analyze(bracketed_link_xdv(
            "pdf:bann<</A<</S/URI/URI(https://two.test)>>>>",
            "pdf:eann",
        ))
        .unwrap()
        .pages()[0]
        .fingerprint
        .clone();

    assert_ne!(first, second);
}

#[test]
fn native_font_style_and_rgba_are_preserved_in_the_render_plan() {
    let mut renderer = HotRenderer::new(MutableResources(RefCell::new(b"font-data".to_vec())));
    let analysis = renderer.analyze(styled_native_xdv()).unwrap();
    let DrawOperation::NativeGlyphs {
        color,
        extend,
        slant,
        embolden,
        vertical,
        ..
    } = &analysis.plans[0].operations[0]
    else {
        panic!("missing native glyph operation");
    };
    assert_eq!(*color, Color::rgba(1.0, 0.0, 0.0, 128.0 / 255.0));
    assert_eq!(*extend, 2 << 16);
    assert_eq!(fixed_16_16(*slant), -0.25);
    assert!((fixed_16_16(*embolden) - 0.1).abs() < 0.001);
    assert!(*vertical);
    assert!(matches!(
        analysis.pages()[0].fingerprint,
        PageFingerprint::Stable(_)
    ));
}

#[test]
fn pdf_wrapper_transform_applies_to_native_text_and_rules() {
    let matrix = "2 0 0 3 4 5";
    let mut renderer = HotRenderer::new(MutableResources(RefCell::new(b"font-data".to_vec())));
    let native = styled_native_xdv();
    let native = renderer
        .analyze(wrap_page_in_transform(&native, matrix))
        .unwrap();
    let DrawOperation::NativeGlyphs { glyphs, .. } = &native.plans[0].operations[0] else {
        panic!("missing transformed native glyph");
    };
    let rule = renderer
        .analyze(wrap_page_in_transform(&rule_xdv("color rgb 1 0 0"), matrix))
        .unwrap();
    let DrawOperation::Rule { transform, .. } = &rule.plans[0].operations[0] else {
        panic!("missing transformed rule");
    };
    let components = |transform: Matrix| {
        (
            transform.a,
            transform.b,
            transform.c,
            transform.d,
            transform.e,
            transform.f,
        )
    };

    let expected = (2.0, 0.0, 0.0, -3.0, 76.0, 67.0);
    assert_eq!(components(glyphs[0].transform), expected);
    assert_eq!(components(*transform), expected);
}

#[test]
fn classic_font_run_uses_tfm_advance_and_frozen_resources() {
    let mut renderer = HotRenderer::new(ClassicResources);
    let analysis = renderer.analyze(Arc::from(ONE_PAGE)).unwrap();
    let DrawOperation::ClassicChars { chars, scale, .. } = &analysis.plans[0].operations[0] else {
        panic!("missing classic character operation");
    };
    assert_eq!(*scale, 655_360);
    assert_eq!(chars.len(), 1);
    assert_eq!(chars[0].character, 97);
    assert!(matches!(
        analysis.pages()[0].fingerprint,
        PageFingerprint::Stable(_)
    ));
    assert_eq!(analysis.resources.len(), 3);
}

#[test]
fn classic_type1_font_uses_its_builtin_encoding() {
    let mut renderer = HotRenderer::new(BuiltinClassicResources);
    let analysis = renderer.analyze(Arc::from(ONE_PAGE)).unwrap();
    assert!(matches!(
        analysis.pages()[0].fingerprint,
        PageFingerprint::Stable(_)
    ));
    assert_eq!(analysis.resources.len(), 3);
}

#[test]
fn classic_font_uses_mapped_program_and_external_encoding() {
    let mut renderer = HotRenderer::new(MappedClassicResources);
    let analysis = renderer.analyze(Arc::from(ONE_PAGE)).unwrap();
    assert!(matches!(
        analysis.pages()[0].fingerprint,
        PageFingerprint::Stable(_)
    ));
    assert_eq!(analysis.resources.len(), 3);
    assert!(analysis
        .ledger()
        .unsupported()
        .get("classic-font-resource")
        .is_none());
}

#[cfg(all(feature = "backend-mupdf", target_os = "linux"))]
#[test]
fn pdf_code_plan_renders_through_mupdf() {
    let mut renderer = HotRenderer::new(());
    let analysis = renderer
        .analyze(pdf_code_xdv(&[
            "q",
            "0 0 20 20 re W n",
            "0 1 0 rg",
            "0 0 40 40 re f",
            "Q",
        ]))
        .unwrap();
    let mut backend = crate::MupdfBackend::new().unwrap();
    let pages = renderer
        .render_pages(&analysis, &[0], &mut backend)
        .unwrap();
    assert!(pages[0].svg.contains("<clipPath"));
    assert!(pages[0].svg.contains("#00ff00"));
}

#[cfg(all(feature = "backend-mupdf", target_os = "linux"))]
#[test]
fn pdf_wrapper_transform_moves_classic_text() {
    let mut renderer = HotRenderer::new(BuiltinClassicResources);
    let plain = renderer.analyze(Arc::from(ONE_PAGE)).unwrap();
    let transformed = renderer
        .analyze(wrap_page_in_transform(ONE_PAGE, "1 0 0 1 20 0"))
        .unwrap();
    assert_ne!(
        plain.pages()[0].fingerprint,
        transformed.pages()[0].fingerprint
    );
    let mut backend = crate::MupdfBackend::new().unwrap();
    let plain_svg = renderer
        .render_pages(&plain, &[0], &mut backend)
        .unwrap()
        .pop()
        .unwrap()
        .svg;
    let transformed_svg = renderer
        .render_pages(&transformed, &[0], &mut backend)
        .unwrap()
        .pop()
        .unwrap()
        .svg;
    let plain = backend.rasterize_svg(plain_svg.as_bytes(), 72.0).unwrap();
    let transformed = backend
        .rasterize_svg(transformed_svg.as_bytes(), 72.0)
        .unwrap();
    let left_edge = |pixmap: &crate::Pixmap| {
        pixmap
            .samples
            .chunks_exact(pixmap.components)
            .enumerate()
            .filter(|(_, pixel)| pixel[..3] != [255, 255, 255])
            .map(|(index, _)| index % pixmap.width)
            .min()
            .unwrap()
    };

    assert_eq!(left_edge(&transformed), left_edge(&plain) + 20);
}

#[cfg(all(feature = "backend-mupdf", target_os = "linux"))]
#[test]
fn real_classic_xdv_renders_to_path_svg_transactionally() {
    let mut renderer = HotRenderer::new(BuiltinClassicResources);
    let analysis = renderer.analyze(Arc::from(ONE_PAGE)).unwrap();
    let mut backend = crate::MupdfBackend::new().unwrap();
    let pages = renderer
        .render_pages(&analysis, &[0], &mut backend)
        .unwrap();
    assert_eq!(pages.len(), 1);
    assert!(pages[0].svg.contains("path"));
    assert!(renderer.last_completed().is_some());
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let pdf = std::fs::read(root.join("tests/tex-outputs/the_letter_a.pdf")).unwrap();
    let preview = backend
        .rasterize_svg(pages[0].svg.as_bytes(), 72.0)
        .unwrap();
    let canonical = backend.rasterize_pdf(&pdf, 0, 72.0).unwrap();
    assert_eq!(
        (preview.width, preview.height),
        (canonical.width, canonical.height)
    );
    let diff = crate::compare_rasters(
        &canonical.samples,
        &preview.samples,
        canonical.width,
        canonical.height,
        canonical.components,
        64,
    );
    let bounds = |pixmap: &crate::Pixmap| {
        let mut bounds = (usize::MAX, usize::MAX, 0, 0);
        for y in 0..pixmap.height {
            for x in 0..pixmap.width {
                let offset = (y * pixmap.width + x) * pixmap.components;
                if pixmap.samples[offset..offset + 3] != [255, 255, 255] {
                    bounds.0 = bounds.0.min(x);
                    bounds.1 = bounds.1.min(y);
                    bounds.2 = bounds.2.max(x);
                    bounds.3 = bounds.3.max(y);
                }
            }
        }
        bounds
    };
    assert!(
        diff.agreement > 0.95,
        "{diff:?}; canonical={:?}, preview={:?}",
        bounds(&canonical),
        bounds(&preview)
    );
}

/// Records native font registrations and draws nothing.
#[derive(Default)]
struct FaceRecorder(Vec<u32>);

impl Backend for FaceRecorder {
    fn begin_page(&mut self, _: f32, _: f32) -> Result<(), RenderError> {
        Ok(())
    }

    fn register_font(
        &mut self,
        _: ResourceIdentity,
        _: &[u8],
        face_index: u32,
    ) -> Result<FontId, RenderError> {
        self.0.push(face_index);
        Ok(FontId(face_index))
    }

    fn register_classic_font(
        &mut self,
        _: ResourceIdentity,
        _: &[u8],
        _: ResourceIdentity,
        _: &[u8],
    ) -> Result<FontId, RenderError> {
        Ok(FontId(0))
    }

    fn fill_rule(&mut self, _: f32, _: f32, _: Matrix, _: Color) -> Result<(), RenderError> {
        Ok(())
    }

    fn fill_path(&mut self, _: &Path, _: bool, _: Matrix, _: Color) -> Result<(), RenderError> {
        Ok(())
    }

    fn stroke_path(
        &mut self,
        _: &Path,
        _: StrokeStyle,
        _: Matrix,
        _: Color,
    ) -> Result<(), RenderError> {
        Ok(())
    }

    fn clip_path(&mut self, _: &Path, _: bool, _: Matrix) -> Result<(), RenderError> {
        Ok(())
    }

    fn pop_clip(&mut self) -> Result<(), RenderError> {
        Ok(())
    }

    fn fill_glyphs(
        &mut self,
        _: FontId,
        _: f32,
        _: &[PositionedGlyph],
        _: Color,
        _: FontStyle,
    ) -> Result<(), RenderError> {
        Ok(())
    }

    fn fill_chars(
        &mut self,
        _: FontId,
        _: f32,
        _: &[PositionedChar],
        _: Color,
    ) -> Result<(), RenderError> {
        Ok(())
    }

    fn draw_image(
        &mut self,
        _: &[u8],
        _: Matrix,
        _: Option<f32>,
        _: Option<f32>,
    ) -> Result<(), RenderError> {
        Ok(())
    }

    fn draw_pdf(&mut self, _: &[u8], _: u32, _: Matrix) -> Result<(), RenderError> {
        Ok(())
    }

    fn finish_page(&mut self) -> Result<String, RenderError> {
        Ok(String::new())
    }
}

#[test]
fn each_face_of_a_font_collection_gets_its_own_handle() {
    let collection = Resource::from_bytes(b"font-collection".to_vec());
    let glyphs = |face_index| DrawOperation::NativeGlyphs {
        identity: collection.identity,
        face_index,
        size: 10 << 16,
        glyphs: Vec::new(),
        color: Color::BLACK,
        extend: crate::units::FIXED_ONE,
        slant: 0,
        embolden: 0,
        vertical: false,
    };
    let analysis = Analysis {
        xdv: Arc::from(&[][..]),
        plans: vec![Arc::new(PagePlan {
            width_bp: 100.0,
            height_bp: 100.0,
            operations: vec![glyphs(0), glyphs(1), glyphs(0)],
            links: Vec::new(),
            destinations: Vec::new(),
            unsupported: BTreeMap::new(),
            complete: true,
        })],
        resources: BTreeMap::from([(collection.identity, collection.clone())]),
        pages: Vec::new(),
        ledger: CoverageLedger::default(),
        reused_pages: 0,
    };
    let mut backend = FaceRecorder::default();
    HotRenderer::new(())
        .render_pages(&analysis, &[0], &mut backend)
        .unwrap();
    assert_eq!(backend.0, [0, 1]);
}
