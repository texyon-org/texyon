//! Transactional XDV analysis and selected-page rendering.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::Cursor;
use std::sync::Arc;

use tectonic_xdv::{FileType, XdvError, XdvEvents, XdvParser};

use crate::backend::{Backend, FontStyle, Matrix, PositionedGlyph};
use crate::color::Color;
use crate::fingerprint::page_digest;
use crate::pagesize::{is_page_size_special, scan_page_sizes, DEFAULT_BP};
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
        let page_sizes = scan_page_sizes(&xdv)?;
        let walker = Walker::new(&self.provider, page_sizes);
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
                        color,
                    } => backend.fill_rule(
                        *x as f32 * SP_TO_BP,
                        *y as f32 * SP_TO_BP,
                        *width as f32 * SP_TO_BP,
                        *height as f32 * SP_TO_BP,
                        *color,
                    )?,
                    DrawOperation::NativeGlyphs {
                        identity,
                        face_index,
                        size,
                        glyphs,
                        color,
                        extend,
                        slant,
                        embolden,
                        vertical,
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
                        backend.fill_glyphs(
                            handle,
                            *size as f32 * SP_TO_BP,
                            &positioned,
                            *color,
                            FontStyle {
                                extend: fixed_16_16(*extend),
                                slant: fixed_16_16(*slant),
                                embolden: fixed_16_16(*embolden),
                                vertical: *vertical,
                            },
                        )?;
                    }
                    DrawOperation::Image {
                        identity,
                        kind,
                        transform,
                        page,
                        width,
                        height,
                    } => {
                        let resource = analysis.resources.get(identity).ok_or_else(|| {
                            RenderError::Resource("analyzed image bytes disappeared".into())
                        })?;
                        match kind {
                            ImageKind::Raster => {
                                backend.draw_image(&resource.bytes, *transform, *width, *height)?
                            }
                            ImageKind::Pdf => {
                                backend.draw_pdf(&resource.bytes, *page, *transform)?
                            }
                        }
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
        color: Color,
    },
    NativeGlyphs {
        identity: ResourceIdentity,
        face_index: u32,
        size: i32,
        glyphs: Vec<GlyphPosition>,
        color: Color,
        extend: u32,
        slant: u32,
        embolden: u32,
        vertical: bool,
    },
    Image {
        identity: ResourceIdentity,
        kind: ImageKind,
        transform: Matrix,
        page: u32,
        width: Option<f32>,
        height: Option<f32>,
    },
}

#[derive(Clone, Copy)]
pub(crate) enum ImageKind {
    Raster,
    Pdf,
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
    color: Option<Color>,
    extend: u32,
    slant: u32,
    embolden: u32,
    vertical: bool,
}

struct Walker<'a, P> {
    provider: &'a P,
    page: Option<PagePlan>,
    pages: Vec<PagePlan>,
    page_sizes: Vec<[f32; 2]>,
    fonts: BTreeMap<i32, NativeFont>,
    resources: BTreeMap<ResourceIdentity, Resource>,
    ledger: CoverageLedger,
    color: Color,
    color_stack: Vec<Color>,
    pdf_color_stacks: BTreeMap<i32, Vec<Color>>,
}

impl<'a, P: ResourceProvider> Walker<'a, P> {
    fn new(provider: &'a P, page_sizes: Vec<[f32; 2]>) -> Self {
        Self {
            provider,
            page: None,
            pages: Vec::new(),
            page_sizes,
            fonts: BTreeMap::new(),
            resources: BTreeMap::new(),
            ledger: CoverageLedger::default(),
            color: Color::BLACK,
            color_stack: Vec::new(),
            pdf_color_stacks: BTreeMap::new(),
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
            self.pages.push(page);
        }
    }

    fn apply_color_special(&mut self, contents: &[u8]) -> Result<bool, RenderError> {
        let Ok(text) = std::str::from_utf8(contents) else {
            return Ok(false);
        };
        let text = text.trim();
        if let Some(args) = text.strip_prefix("color") {
            let args = args.trim();
            if let Some(spec) = args.strip_prefix("push") {
                self.color_stack.push(self.color);
                let Some(color) = parse_color_spec(spec.trim()) else {
                    self.unsupported("malformed-color")?;
                    return Ok(true);
                };
                self.color = color;
            } else if args == "pop" {
                let Some(color) = self.color_stack.pop() else {
                    self.unsupported("color-stack-underflow")?;
                    return Ok(true);
                };
                self.color = color;
            } else {
                let Some(color) = parse_color_spec(args) else {
                    self.unsupported("malformed-color")?;
                    return Ok(true);
                };
                self.color = color;
            }
            self.ledger.record_supported("color");
            return Ok(true);
        }
        if let Some(args) = text.strip_prefix("pdfcolorstack") {
            let mut tokens = args.split_whitespace();
            let Some(stack_number) = tokens.next().and_then(|value| value.parse::<i32>().ok())
            else {
                self.unsupported("malformed-pdfcolorstack")?;
                return Ok(true);
            };
            let Some(action) = tokens.next() else {
                self.unsupported("malformed-pdfcolorstack")?;
                return Ok(true);
            };
            match action {
                "push" => {
                    self.pdf_color_stacks
                        .entry(stack_number)
                        .or_default()
                        .push(self.color);
                    let Some(color) = parse_pdf_color(args) else {
                        self.unsupported("malformed-pdfcolorstack")?;
                        return Ok(true);
                    };
                    self.color = color;
                }
                "pop" => {
                    let color = self
                        .pdf_color_stacks
                        .get_mut(&stack_number)
                        .and_then(Vec::pop);
                    let Some(color) = color else {
                        self.unsupported("pdfcolorstack-underflow")?;
                        return Ok(true);
                    };
                    self.color = color;
                }
                "set" => {
                    let Some(color) = parse_pdf_color(args) else {
                        self.unsupported("malformed-pdfcolorstack")?;
                        return Ok(true);
                    };
                    self.color = color;
                }
                _ => {
                    self.unsupported("unknown-pdfcolorstack-action")?;
                    return Ok(true);
                }
            }
            self.ledger.record_supported("pdfcolorstack");
            return Ok(true);
        }
        Ok(false)
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
        let size = self
            .page_sizes
            .get(self.pages.len())
            .copied()
            .unwrap_or(DEFAULT_BP);
        self.page = Some(PagePlan {
            width_bp: size[0],
            height_bp: size[1],
            operations: Vec::new(),
            unsupported: BTreeMap::new(),
            complete: true,
        });
        Ok(())
    }

    fn handle_special(&mut self, x: i32, y: i32, contents: &[u8]) -> Result<(), Self::Error> {
        if is_page_size_special(contents) {
            self.ledger.record_supported("page-size");
            return Ok(());
        }
        if self.apply_color_special(contents)? {
            return Ok(());
        }
        if let Some(image) = parse_image_special(contents, x, y) {
            let request = ResourceRequest {
                kind: match image.kind {
                    ImageKind::Raster => ResourceKind::Image,
                    ImageKind::Pdf => ResourceKind::Pdf,
                },
                name: image.path,
            };
            let Some(resource) = self.provider.resolve(&request)? else {
                return self.unsupported("missing-image-resource");
            };
            let identity = resource.identity;
            self.resources.insert(identity, resource);
            self.page_mut()?.operations.push(DrawOperation::Image {
                identity,
                kind: image.kind,
                transform: image.transform,
                page: image.page,
                width: image.width,
                height: image.height,
            });
            self.ledger.record_supported(match image.kind {
                ImageKind::Raster => "raster-image",
                ImageKind::Pdf => "embedded-pdf",
            });
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
        let font_color = font.color;
        let extend = font.extend;
        let slant = font.slant;
        let embolden = font.embolden;
        let vertical = font.vertical;
        debug_assert_eq!(glyphs.len(), x.len());
        debug_assert_eq!(glyphs.len(), y.len());
        let positions = glyphs
            .iter()
            .zip(x)
            .zip(y)
            .map(|((&glyph_id, &x), &y)| GlyphPosition { glyph_id, x, y })
            .collect();
        let color = font_color.unwrap_or(self.color);
        self.page_mut()?
            .operations
            .push(DrawOperation::NativeGlyphs {
                identity,
                face_index,
                size,
                glyphs: positions,
                color,
                extend,
                slant,
                embolden,
                vertical,
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

    fn handle_define_native_font_ext(
        &mut self,
        name: &str,
        font_num: i32,
        size: i32,
        face_index: u32,
        vertical: bool,
        color_rgba: Option<u32>,
        extend: Option<u32>,
        slant: Option<u32>,
        embolden: Option<u32>,
    ) -> Result<(), Self::Error> {
        let color = color_rgba.map(|rgba| {
            Color::rgba(
                ((rgba >> 24) & 0xff) as f32 / 255.0,
                ((rgba >> 16) & 0xff) as f32 / 255.0,
                ((rgba >> 8) & 0xff) as f32 / 255.0,
                (rgba & 0xff) as f32 / 255.0,
            )
        });
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
                color,
                extend: extend.unwrap_or(65_536),
                slant: slant.unwrap_or(0),
                embolden: embolden.unwrap_or(0),
                vertical,
            },
        );
        Ok(())
    }

    fn handle_rule(&mut self, x: i32, y: i32, height: i32, width: i32) -> Result<(), Self::Error> {
        if height > 0 && width > 0 {
            let color = self.color;
            self.page_mut()?.operations.push(DrawOperation::Rule {
                x,
                y,
                height,
                width,
                color,
            });
            self.ledger.record_supported("rule");
        }
        Ok(())
    }
}

struct ImageSpecial {
    path: String,
    kind: ImageKind,
    transform: Matrix,
    page: u32,
    width: Option<f32>,
    height: Option<f32>,
}

fn parse_color_spec(spec: &str) -> Option<Color> {
    let mut tokens = spec.split_whitespace();
    let model = tokens.next()?;
    let values: Vec<f32> = tokens.filter_map(|value| value.parse().ok()).collect();
    match model.to_ascii_lowercase().as_str() {
        "gray" if !values.is_empty() => Some(Color::gray(values[0])),
        "rgb" if values.len() >= 3 => Some(Color::rgb(values[0], values[1], values[2])),
        "cmyk" if values.len() >= 4 => {
            Some(Color::cmyk(values[0], values[1], values[2], values[3]))
        }
        _ => None,
    }
}

fn parse_pdf_color(args: &str) -> Option<Color> {
    let open = args.find('(')?;
    let close = args[open + 1..]
        .find(')')
        .map(|index| open + 1 + index)
        .unwrap_or(args.len());
    let mut tokens: Vec<_> = args[open + 1..close].split_whitespace().collect();
    let operator = tokens.pop()?;
    let values: Vec<f32> = tokens
        .into_iter()
        .filter_map(|value| value.parse().ok())
        .collect();
    match operator {
        "g" if !values.is_empty() => Some(Color::gray(values[0])),
        "rg" if values.len() >= 3 => Some(Color::rgb(values[0], values[1], values[2])),
        "k" if values.len() >= 4 => Some(Color::cmyk(values[0], values[1], values[2], values[3])),
        _ => None,
    }
}

fn parse_image_special(contents: &[u8], x: i32, y: i32) -> Option<ImageSpecial> {
    let text = std::str::from_utf8(contents).ok()?.trim();
    let args = text.strip_prefix("pdf:image")?.trim();
    let open = args.find('(')?;
    let close = args[open + 1..]
        .find(')')
        .map(|index| open + 1 + index)
        .unwrap_or(args.len());
    let path = args[open + 1..close].to_owned();
    let tokens: Vec<_> = args[..open].split_whitespace().collect();
    let mut matrix = Matrix {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };
    let mut page = 0;
    let mut width = None;
    let mut height = None;
    let mut index = 0;
    while index < tokens.len() {
        match tokens[index] {
            "matrix" if index + 6 < tokens.len() => {
                let values: Option<Vec<f32>> = tokens[index + 1..index + 7]
                    .iter()
                    .map(|value| value.parse().ok().map(normalize_float))
                    .collect();
                let values = values?;
                matrix = Matrix {
                    a: values[0],
                    b: values[1],
                    c: values[2],
                    d: values[3],
                    e: values[4],
                    f: values[5],
                };
                index += 7;
            }
            "page" if index + 1 < tokens.len() => {
                let raw: i32 = tokens[index + 1].parse().ok()?;
                page = raw.saturating_sub(1).max(0) as u32;
                index += 2;
            }
            "width" if index + 1 < tokens.len() => {
                width = parse_image_dimension(tokens[index + 1]);
                index += 2;
            }
            "height" if index + 1 < tokens.len() => {
                height = parse_image_dimension(tokens[index + 1]);
                index += 2;
            }
            _ => index += 1,
        }
    }
    let kind = if path
        .rsplit('.')
        .next()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
    {
        ImageKind::Pdf
    } else {
        ImageKind::Raster
    };
    let mut origin = Matrix::dvi_origin(x, y, SP_TO_BP);
    if matches!(kind, ImageKind::Pdf) {
        origin.b = -origin.b;
        origin.d = -origin.d;
    }
    Some(ImageSpecial {
        path,
        kind,
        transform: matrix.concat(origin),
        page,
        width,
        height,
    })
}

fn parse_image_dimension(value: &str) -> Option<f32> {
    let split = value
        .find(|character: char| {
            !character.is_ascii_digit() && !matches!(character, '.' | '-' | '+')
        })
        .unwrap_or(value.len());
    let number: f32 = value[..split].parse().ok()?;
    let scale = match &value[split..] {
        "" | "bp" => 1.0,
        "pt" => 72.0 / 72.27,
        "in" => 72.0,
        _ => return None,
    };
    Some(normalize_float(number * scale))
}

fn normalize_float(value: f32) -> f32 {
    if value == 0.0 {
        0.0
    } else {
        value
    }
}

fn fixed_16_16(value: u32) -> f32 {
    i32::from_be_bytes(value.to_be_bytes()) as f32 / 65_536.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::cell::RefCell;

    const ONE_PAGE: &[u8] = include_bytes!("../../../tests/tex-outputs/the_letter_a.xdv");
    const IMAGE_PAGE: &[u8] = include_bytes!("../../../tests/tex-outputs/redbox_png.xdv");

    struct MutableResources(RefCell<Vec<u8>>);

    impl ResourceProvider for MutableResources {
        fn resolve(&self, _request: &ResourceRequest) -> Result<Option<Resource>, RenderError> {
            Ok(Some(Resource::from_bytes(self.0.borrow().clone())))
        }
    }

    fn rule_xdv(color_special: &str) -> Arc<[u8]> {
        let mut bytes = vec![247, 7];
        bytes.extend_from_slice(&25_400_000u32.to_be_bytes());
        bytes.extend_from_slice(&473_628_672u32.to_be_bytes());
        bytes.extend_from_slice(&1000u32.to_be_bytes());
        bytes.push(0);
        bytes.push(139);
        bytes.extend_from_slice(&[0; 40]);
        bytes.extend_from_slice(&(-1i32).to_be_bytes());
        bytes.push(239);
        bytes.push(color_special.len() as u8);
        bytes.extend_from_slice(color_special.as_bytes());
        bytes.push(132);
        bytes.extend_from_slice(&65_536i32.to_be_bytes());
        bytes.extend_from_slice(&131_072i32.to_be_bytes());
        bytes.push(140);
        bytes.push(248);
        bytes.extend_from_slice(&0u32.to_be_bytes());
        bytes.extend_from_slice(&25_400_000u32.to_be_bytes());
        bytes.extend_from_slice(&473_628_672u32.to_be_bytes());
        bytes.extend_from_slice(&1000u32.to_be_bytes());
        bytes.extend_from_slice(&[0; 8]);
        bytes.extend_from_slice(&2u16.to_be_bytes());
        bytes.extend_from_slice(&1u16.to_be_bytes());
        bytes.push(249);
        bytes.extend_from_slice(&0u32.to_be_bytes());
        bytes.push(7);
        bytes.extend_from_slice(&[223; 4]);
        Arc::from(bytes)
    }

    fn styled_native_xdv() -> Arc<[u8]> {
        let mut bytes = vec![247, 7];
        bytes.extend_from_slice(&25_400_000u32.to_be_bytes());
        bytes.extend_from_slice(&473_628_672u32.to_be_bytes());
        bytes.extend_from_slice(&1000u32.to_be_bytes());
        bytes.push(0);
        bytes.push(139);
        bytes.extend_from_slice(&[0; 40]);
        bytes.extend_from_slice(&(-1i32).to_be_bytes());
        bytes.push(252);
        bytes.extend_from_slice(&0i32.to_be_bytes());
        bytes.extend_from_slice(&(10i32 << 16).to_be_bytes());
        bytes.extend_from_slice(&0x7300u16.to_be_bytes());
        bytes.push(4);
        bytes.extend_from_slice(b"font");
        bytes.extend_from_slice(&2u32.to_be_bytes());
        bytes.extend_from_slice(&0xff000080u32.to_be_bytes());
        bytes.extend_from_slice(&(2u32 << 16).to_be_bytes());
        bytes.extend_from_slice(&(-16_384i32 as u32).to_be_bytes());
        bytes.extend_from_slice(&6_554u32.to_be_bytes());
        bytes.push(253);
        bytes.extend_from_slice(&(12i32 << 16).to_be_bytes());
        bytes.extend_from_slice(&1u16.to_be_bytes());
        bytes.extend_from_slice(&0i32.to_be_bytes());
        bytes.extend_from_slice(&0i32.to_be_bytes());
        bytes.extend_from_slice(&34u16.to_be_bytes());
        bytes.push(140);
        bytes.push(248);
        bytes.extend_from_slice(&0u32.to_be_bytes());
        bytes.extend_from_slice(&25_400_000u32.to_be_bytes());
        bytes.extend_from_slice(&473_628_672u32.to_be_bytes());
        bytes.extend_from_slice(&1000u32.to_be_bytes());
        bytes.extend_from_slice(&[0; 8]);
        bytes.extend_from_slice(&2u16.to_be_bytes());
        bytes.extend_from_slice(&1u16.to_be_bytes());
        bytes.push(249);
        bytes.extend_from_slice(&0u32.to_be_bytes());
        bytes.push(7);
        bytes.extend_from_slice(&[223; 4]);
        Arc::from(bytes)
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
}
