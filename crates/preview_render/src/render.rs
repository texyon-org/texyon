//! Transactional XDV analysis and selected-page rendering.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::Cursor;
use std::sync::Arc;

use tectonic_xdv::{FileType, XdvError, XdvEvents, XdvParser};

use crate::backend::{Backend, PositionedGlyph};
use crate::fingerprint::page_digest;
use crate::pagesize::{parse_special, LETTER_BP};
use crate::resource::{
    Resource, ResourceIdentity, ResourceKind, ResourceProvider, ResourceRequest,
};

const SP_TO_BP: f32 = 72.0 / 72.27 / 65_536.0;

/// Failure from parsing, resource resolution, or a rendering backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderError {
    /// Malformed or unsupported XDV structure.
    Xdv(String),
    /// Resource-provider failure.
    Resource(String),
    /// Backend failure.
    Backend(String),
    /// Invalid selected-page request.
    InvalidSelection(String),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Xdv(message) => write!(f, "XDV analysis failed: {message}"),
            Self::Resource(message) => write!(f, "resource resolution failed: {message}"),
            Self::Backend(message) => write!(f, "render backend failed: {message}"),
            Self::InvalidSelection(message) => write!(f, "invalid page selection: {message}"),
        }
    }
}

impl std::error::Error for RenderError {}

impl From<XdvError> for RenderError {
    fn from(value: XdvError) -> Self {
        Self::Xdv(value.to_string())
    }
}

impl From<std::io::Error> for RenderError {
    fn from(value: std::io::Error) -> Self {
        Self::Xdv(value.to_string())
    }
}

/// Counts supported and unsupported constructs encountered during analysis.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CoverageLedger {
    supported: BTreeMap<String, u64>,
    unsupported: BTreeMap<String, u64>,
}

impl CoverageLedger {
    /// Supported operation counts by stable name.
    pub fn supported(&self) -> &BTreeMap<String, u64> {
        &self.supported
    }

    /// Unsupported operation counts by stable name.
    pub fn unsupported(&self) -> &BTreeMap<String, u64> {
        &self.unsupported
    }

    fn record_supported(&mut self, name: &str) {
        *self.supported.entry(name.into()).or_default() += 1;
    }

    fn record_unsupported(&mut self, name: &str) {
        *self.unsupported.entry(name.into()).or_default() += 1;
    }
}

/// Unsupported constructs local to one page.
pub type UnsupportedSummary = BTreeMap<String, u64>;

/// Renderer-owned visual identity for one page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageFingerprint {
    /// Complete canonical renderer meaning.
    Stable([u8; 32]),
    /// Incomplete meaning; intentionally unequal across analyses.
    Uncertain {
        /// Monotonic analysis identity.
        analysis_id: u64,
        /// Zero-based page index.
        page_index: usize,
    },
}

/// Public analysis facts for one page.
#[derive(Clone, Debug, PartialEq)]
pub struct PageVisual {
    /// Renderer-owned visual identity.
    pub fingerprint: PageFingerprint,
    /// Page size in PostScript big points.
    pub size_bp: [f32; 2],
    /// Unsupported constructs that made this page conservative.
    pub unsupported: UnsupportedSummary,
}

/// One rendered page payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageSvg {
    /// Zero-based document page index.
    pub page_index: usize,
    /// Self-contained SVG markup.
    pub svg: String,
}

/// Immutable result of one XDV analysis.
pub struct Analysis {
    xdv: Arc<[u8]>,
    plans: Vec<PagePlan>,
    resources: BTreeMap<ResourceIdentity, Resource>,
    pages: Vec<PageVisual>,
    ledger: CoverageLedger,
}

impl Analysis {
    /// Exact XDV bytes against which this operation was analyzed.
    pub fn xdv(&self) -> &Arc<[u8]> {
        &self.xdv
    }

    /// Page visual facts in document order.
    pub fn pages(&self) -> &[PageVisual] {
        &self.pages
    }

    /// Coverage facts for the entire operation.
    pub fn ledger(&self) -> &CoverageLedger {
        &self.ledger
    }
}

/// Stateful renderer that retains only immutable resources and completed facts.
pub struct HotRenderer<P> {
    provider: P,
    next_analysis_id: u64,
    last_completed: Option<(Vec<PageVisual>, CoverageLedger)>,
}

impl<P: ResourceProvider> HotRenderer<P> {
    /// Construct a renderer around an immutable-byte resource provider.
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            next_analysis_id: 1,
            last_completed: None,
        }
    }

    /// Analyze every page, resolving and freezing all resources used by the walk.
    pub fn analyze(&mut self, xdv: Arc<[u8]>) -> Result<Analysis, RenderError> {
        let analysis_id = self.next_analysis_id;
        self.next_analysis_id = self.next_analysis_id.wrapping_add(1).max(1);
        let walker = Walker::new(&self.provider);
        let (mut walker, _) = XdvParser::process(Cursor::new(&xdv), walker)?;
        walker.finish_page();
        let mut pages = Vec::with_capacity(walker.pages.len());
        for (page_index, plan) in walker.pages.iter().enumerate() {
            let fingerprint = if plan.complete {
                PageFingerprint::Stable(page_digest(plan))
            } else {
                PageFingerprint::Uncertain {
                    analysis_id,
                    page_index,
                }
            };
            pages.push(PageVisual {
                fingerprint,
                size_bp: [plan.width_bp, plan.height_bp],
                unsupported: plan.unsupported.clone(),
            });
        }
        Ok(Analysis {
            xdv,
            plans: walker.pages,
            resources: walker.resources,
            pages,
            ledger: walker.ledger,
        })
    }

    /// Render exactly the selected pages and commit completed analysis facts only
    /// after every selected page yields one SVG.
    pub fn render_pages<B: Backend>(
        &mut self,
        analysis: &Analysis,
        page_indices: &[usize],
        backend: &mut B,
    ) -> Result<Vec<PageSvg>, RenderError> {
        let mut seen = BTreeSet::new();
        for &page_index in page_indices {
            if page_index >= analysis.plans.len() {
                return Err(RenderError::InvalidSelection(format!(
                    "page {page_index} exceeds page count {}",
                    analysis.plans.len()
                )));
            }
            if !seen.insert(page_index) {
                return Err(RenderError::InvalidSelection(format!(
                    "page {page_index} was requested twice"
                )));
            }
        }

        let mut font_handles = BTreeMap::new();
        let mut rendered = Vec::with_capacity(page_indices.len());
        for &page_index in page_indices {
            let page = &analysis.plans[page_index];
            backend.begin_page(page.width_bp, page.height_bp)?;
            for operation in &page.operations {
                match operation {
                    DrawOperation::Rule {
                        x,
                        y,
                        height,
                        width,
                    } => backend.fill_rule(
                        *x as f32 * SP_TO_BP,
                        *y as f32 * SP_TO_BP,
                        *width as f32 * SP_TO_BP,
                        *height as f32 * SP_TO_BP,
                    )?,
                    DrawOperation::NativeGlyphs {
                        identity,
                        face_index,
                        size,
                        glyphs,
                    } => {
                        let handle = match font_handles.get(identity) {
                            Some(handle) => *handle,
                            None => {
                                let resource =
                                    analysis.resources.get(identity).ok_or_else(|| {
                                        RenderError::Resource(
                                            "analyzed font bytes disappeared".into(),
                                        )
                                    })?;
                                let handle = backend.register_font(
                                    *identity,
                                    &resource.bytes,
                                    *face_index,
                                )?;
                                font_handles.insert(*identity, handle);
                                handle
                            }
                        };
                        let positioned: Vec<_> = glyphs
                            .iter()
                            .map(|glyph| PositionedGlyph {
                                glyph_id: glyph.glyph_id,
                                x_bp: glyph.x as f32 * SP_TO_BP,
                                y_bp: glyph.y as f32 * SP_TO_BP,
                            })
                            .collect();
                        backend.fill_glyphs(handle, *size as f32 * SP_TO_BP, &positioned)?;
                    }
                }
            }
            rendered.push(PageSvg {
                page_index,
                svg: backend.finish_page()?,
            });
        }
        self.last_completed = Some((analysis.pages.clone(), analysis.ledger.clone()));
        Ok(rendered)
    }

    /// Visual and coverage facts from the last transactionally completed render.
    pub fn last_completed(&self) -> Option<(&[PageVisual], &CoverageLedger)> {
        self.last_completed
            .as_ref()
            .map(|(pages, ledger)| (pages.as_slice(), ledger))
    }
}

#[derive(Clone)]
pub(crate) struct PagePlan {
    pub(crate) width_bp: f32,
    pub(crate) height_bp: f32,
    pub(crate) operations: Vec<DrawOperation>,
    pub(crate) unsupported: UnsupportedSummary,
    pub(crate) complete: bool,
}

#[derive(Clone)]
pub(crate) enum DrawOperation {
    Rule {
        x: i32,
        y: i32,
        height: i32,
        width: i32,
    },
    NativeGlyphs {
        identity: ResourceIdentity,
        face_index: u32,
        size: i32,
        glyphs: Vec<GlyphPosition>,
    },
}

#[derive(Clone)]
pub(crate) struct GlyphPosition {
    pub(crate) glyph_id: u16,
    pub(crate) x: i32,
    pub(crate) y: i32,
}

struct NativeFont {
    resource: Option<ResourceIdentity>,
    face_index: u32,
    size: i32,
    options_supported: bool,
}

struct Walker<'a, P> {
    provider: &'a P,
    page: Option<PagePlan>,
    pages: Vec<PagePlan>,
    page_size: [f32; 2],
    fonts: BTreeMap<i32, NativeFont>,
    resources: BTreeMap<ResourceIdentity, Resource>,
    ledger: CoverageLedger,
}

impl<'a, P: ResourceProvider> Walker<'a, P> {
    fn new(provider: &'a P) -> Self {
        Self {
            provider,
            page: None,
            pages: Vec::new(),
            page_size: LETTER_BP,
            fonts: BTreeMap::new(),
            resources: BTreeMap::new(),
            ledger: CoverageLedger::default(),
        }
    }

    fn page_mut(&mut self) -> Result<&mut PagePlan, RenderError> {
        self.page
            .as_mut()
            .ok_or_else(|| RenderError::Xdv("drawing operation outside a page".into()))
    }

    fn unsupported(&mut self, name: &str) -> Result<(), RenderError> {
        self.ledger.record_unsupported(name);
        let page = self.page_mut()?;
        page.complete = false;
        *page.unsupported.entry(name.into()).or_default() += 1;
        Ok(())
    }

    fn finish_page(&mut self) {
        if let Some(page) = self.page.take() {
            self.page_size = [page.width_bp, page.height_bp];
            self.pages.push(page);
        }
    }
}

impl<P: ResourceProvider> XdvEvents for Walker<'_, P> {
    type Error = RenderError;

    fn handle_header(&mut self, filetype: FileType, _comment: &[u8]) -> Result<(), Self::Error> {
        if filetype != FileType::Xdv {
            return Err(RenderError::Xdv(
                "SPX is not a paginated preview document".into(),
            ));
        }
        Ok(())
    }

    fn handle_begin_page(
        &mut self,
        _counters: &[i32],
        _previous_bop: i32,
    ) -> Result<(), Self::Error> {
        self.finish_page();
        self.page = Some(PagePlan {
            width_bp: self.page_size[0],
            height_bp: self.page_size[1],
            operations: Vec::new(),
            unsupported: BTreeMap::new(),
            complete: true,
        });
        Ok(())
    }

    fn handle_special(&mut self, _x: i32, _y: i32, contents: &[u8]) -> Result<(), Self::Error> {
        if let Some(size) = parse_special(contents) {
            let page = self.page_mut()?;
            page.width_bp = size[0];
            page.height_bp = size[1];
            self.ledger.record_supported("page-size");
            return Ok(());
        }
        let name = std::str::from_utf8(contents)
            .ok()
            .and_then(|text| text.split([' ', ':', '=']).next())
            .filter(|name| !name.is_empty())
            .unwrap_or("non-utf8-special");
        self.unsupported(name)
    }

    fn handle_char_run(&mut self, _font_num: i32, chars: &[i32]) -> Result<(), Self::Error> {
        for _ in chars {
            self.unsupported("classic-font-char")?;
        }
        Ok(())
    }

    fn handle_glyph_run(
        &mut self,
        font_num: i32,
        glyphs: &[u16],
        x: &[i32],
        y: &[i32],
    ) -> Result<(), Self::Error> {
        let Some(font) = self.fonts.get(&font_num) else {
            return self.unsupported("undefined-native-font");
        };
        let Some(identity) = font.resource else {
            return self.unsupported("missing-native-font");
        };
        let face_index = font.face_index;
        let size = font.size;
        let options_supported = font.options_supported;
        if !options_supported {
            self.unsupported("native-font-options")?;
        }
        debug_assert_eq!(glyphs.len(), x.len());
        debug_assert_eq!(glyphs.len(), y.len());
        let positions = glyphs
            .iter()
            .zip(x)
            .zip(y)
            .map(|((&glyph_id, &x), &y)| GlyphPosition { glyph_id, x, y })
            .collect();
        self.page_mut()?
            .operations
            .push(DrawOperation::NativeGlyphs {
                identity,
                face_index,
                size,
                glyphs: positions,
            });
        self.ledger.record_supported("native-glyph-run");
        Ok(())
    }

    fn handle_text_and_glyphs(
        &mut self,
        font_num: i32,
        _text: &str,
        _width: i32,
        glyphs: &[u16],
        x: &[i32],
        y: &[i32],
    ) -> Result<(), Self::Error> {
        self.handle_glyph_run(font_num, glyphs, x, y)
    }

    fn handle_define_native_font(
        &mut self,
        name: &str,
        font_num: i32,
        size: i32,
        face_index: u32,
        color_rgba: Option<u32>,
        extend: Option<u32>,
        slant: Option<u32>,
        embolden: Option<u32>,
    ) -> Result<(), Self::Error> {
        let options_supported =
            color_rgba.is_none() && extend.is_none() && slant.is_none() && embolden.is_none();
        if !options_supported {
            self.ledger.record_unsupported("native-font-options");
        }
        let request = ResourceRequest {
            kind: ResourceKind::FontProgram,
            name: name.into(),
        };
        let resource = self.provider.resolve(&request)?;
        let identity = resource.as_ref().map(|resource| resource.identity);
        if let Some(resource) = resource {
            self.resources.insert(resource.identity, resource);
            self.ledger.record_supported("native-font-resource");
        } else {
            self.ledger.record_unsupported("native-font-resource");
        }
        self.fonts.insert(
            font_num,
            NativeFont {
                resource: identity,
                face_index,
                size,
                options_supported,
            },
        );
        Ok(())
    }

    fn handle_rule(&mut self, x: i32, y: i32, height: i32, width: i32) -> Result<(), Self::Error> {
        if height > 0 && width > 0 {
            self.page_mut()?.operations.push(DrawOperation::Rule {
                x,
                y,
                height,
                width,
            });
            self.ledger.record_supported("rule");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    const ONE_PAGE: &[u8] = include_bytes!("../../../tests/tex-outputs/the_letter_a.xdv");

    #[test]
    fn real_xdv_is_analyzed_conservatively_without_classic_font_support() {
        let mut renderer = HotRenderer::new(());
        let analysis = renderer.analyze(Arc::from(ONE_PAGE)).unwrap();
        assert_eq!(analysis.pages().len(), 1);
        assert!(matches!(
            analysis.pages()[0].fingerprint,
            PageFingerprint::Uncertain { page_index: 0, .. }
        ));
        assert!(analysis
            .ledger()
            .unsupported()
            .contains_key("classic-font-char"));
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
}
