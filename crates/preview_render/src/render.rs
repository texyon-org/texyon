//! Transactional XDV analysis and selected-page rendering.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::Cursor;
use std::sync::Arc;

use tectonic_xdv::{FileType, XdvError, XdvEvents, XdvParser};

use crate::backend::{
    Backend, FontStyle, Matrix, Path, PositionedChar, PositionedGlyph, StrokeStyle,
};
use crate::color::Color;
use crate::encoding::type1_builtin;
use crate::fontmap::FontMap;
use crate::fingerprint::page_digest;
use crate::pagesize::{is_page_size_special, scan_page_sizes, DEFAULT_BP};
use crate::pdfcode::{PdfGraphics, PdfOperation};
use crate::resource::{
    Resource, ResourceIdentity, ResourceKind, ResourceProvider, ResourceRequest,
};
use crate::tfm::Tfm;

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

        let mut native_handles = BTreeMap::new();
        let mut classic_handles = BTreeMap::new();
        let mut rendered = Vec::with_capacity(page_indices.len());
        for &page_index in page_indices {
            let page = &analysis.plans[page_index];
            backend.begin_page(page.width_bp, page.height_bp)?;
            for operation in &page.operations {
                match operation {
                    DrawOperation::Rule {
                        height,
                        width,
                        transform,
                        color,
                    } => backend.fill_rule(
                        *width as f32 * SP_TO_BP,
                        *height as f32 * SP_TO_BP,
                        *transform,
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
                        let handle = match native_handles.get(identity) {
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
                                native_handles.insert(*identity, handle);
                                handle
                            }
                        };
                        let positioned: Vec<_> = glyphs
                            .iter()
                            .map(|glyph| PositionedGlyph {
                                glyph_id: glyph.glyph_id,
                                transform: glyph.transform,
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
                    DrawOperation::ClassicChars {
                        program_identity,
                        encoding_identity,
                        scale,
                        chars,
                        color,
                        ..
                    } => {
                        let key = (*program_identity, *encoding_identity);
                        let handle = match classic_handles.get(&key) {
                            Some(handle) => *handle,
                            None => {
                                let program =
                                    analysis.resources.get(program_identity).ok_or_else(|| {
                                        RenderError::Resource(
                                            "analyzed classic font disappeared".into(),
                                        )
                                    })?;
                                let encoding =
                                    analysis.resources.get(encoding_identity).ok_or_else(|| {
                                        RenderError::Resource(
                                            "analyzed font encoding disappeared".into(),
                                        )
                                    })?;
                                let handle = backend.register_classic_font(
                                    *program_identity,
                                    &program.bytes,
                                    *encoding_identity,
                                    &encoding.bytes,
                                )?;
                                classic_handles.insert(key, handle);
                                handle
                            }
                        };
                        let positioned: Vec<_> = chars
                            .iter()
                            .map(|character| PositionedChar {
                                character: character.character as u32,
                                transform: character.transform,
                            })
                            .collect();
                        backend.fill_chars(
                            handle,
                            *scale as f32 * SP_TO_BP,
                            &positioned,
                            *color,
                        )?;
                    }
                    DrawOperation::PathFill {
                        path,
                        even_odd,
                        transform,
                        color,
                    } => backend.fill_path(path, *even_odd, *transform, *color)?,
                    DrawOperation::PathStroke {
                        path,
                        style,
                        transform,
                        color,
                    } => backend.stroke_path(path, *style, *transform, *color)?,
                    DrawOperation::ClipPath {
                        path,
                        even_odd,
                        transform,
                    } => backend.clip_path(path, *even_odd, *transform)?,
                    DrawOperation::PopClip => backend.pop_clip()?,
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
        height: i32,
        width: i32,
        transform: Matrix,
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
    ClassicChars {
        tfm_identity: ResourceIdentity,
        program_identity: ResourceIdentity,
        encoding_identity: ResourceIdentity,
        scale: u32,
        chars: Vec<ClassicPosition>,
        color: Color,
    },
    PathFill {
        path: Path,
        even_odd: bool,
        transform: Matrix,
        color: Color,
    },
    PathStroke {
        path: Path,
        style: StrokeStyle,
        transform: Matrix,
        color: Color,
    },
    ClipPath {
        path: Path,
        even_odd: bool,
        transform: Matrix,
    },
    PopClip,
}

#[derive(Clone, Copy)]
pub(crate) enum ImageKind {
    Raster,
    Pdf,
}

#[derive(Clone)]
pub(crate) struct GlyphPosition {
    pub(crate) glyph_id: u16,
    pub(crate) transform: Matrix,
}

#[derive(Clone)]
pub(crate) struct ClassicPosition {
    pub(crate) character: i32,
    pub(crate) transform: Matrix,
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

struct ClassicFont {
    scale: u32,
    tfm: Option<Tfm>,
    tfm_identity: Option<ResourceIdentity>,
    program_identity: Option<ResourceIdentity>,
    encoding_identity: Option<ResourceIdentity>,
}

struct Walker<'a, P> {
    provider: &'a P,
    page: Option<PagePlan>,
    pages: Vec<PagePlan>,
    page_sizes: Vec<[f32; 2]>,
    fonts: BTreeMap<i32, NativeFont>,
    classic_fonts: BTreeMap<i32, ClassicFont>,
    font_map: Option<FontMap>,
    resources: BTreeMap<ResourceIdentity, Resource>,
    ledger: CoverageLedger,
    color: Color,
    color_stack: Vec<(Color, Color)>,
    pdf_color_stacks: BTreeMap<i32, Vec<(Color, Color)>>,
    pdf_graphics: PdfGraphics,
}

impl<'a, P: ResourceProvider> Walker<'a, P> {
    fn new(provider: &'a P, page_sizes: Vec<[f32; 2]>) -> Self {
        Self {
            provider,
            page: None,
            pages: Vec::new(),
            page_sizes,
            fonts: BTreeMap::new(),
            classic_fonts: BTreeMap::new(),
            font_map: None,
            resources: BTreeMap::new(),
            ledger: CoverageLedger::default(),
            color: Color::BLACK,
            color_stack: Vec::new(),
            pdf_color_stacks: BTreeMap::new(),
            pdf_graphics: PdfGraphics::default(),
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

    fn append_pdf_operations(
        &mut self,
        operations: Vec<PdfOperation>,
    ) -> Result<(), RenderError> {
        for operation in operations {
            let operation = match operation {
                PdfOperation::Fill {
                    path,
                    even_odd,
                    transform,
                    color,
                } => DrawOperation::PathFill {
                    path,
                    even_odd,
                    transform,
                    color,
                },
                PdfOperation::Stroke {
                    path,
                    style,
                    transform,
                    color,
                } => DrawOperation::PathStroke {
                    path,
                    style,
                    transform,
                    color,
                },
                PdfOperation::Clip {
                    path,
                    even_odd,
                    transform,
                } => DrawOperation::ClipPath {
                    path,
                    even_odd,
                    transform,
                },
                PdfOperation::PopClip => DrawOperation::PopClip,
            };
            self.page_mut()?.operations.push(operation);
        }
        Ok(())
    }

    fn apply_transform_special(
        &mut self,
        x: i32,
        y: i32,
        contents: &[u8],
    ) -> Result<bool, RenderError> {
        let Ok(text) = std::str::from_utf8(contents) else {
            return Ok(false);
        };
        let text = text.trim();
        if text == "pdf:btrans" || text.starts_with("pdf:btrans ") {
            let args = &text["pdf:btrans".len()..];
            let transform = parse_named_matrix(args).unwrap_or(Matrix {
                a: 1.0,
                b: 0.0,
                c: 0.0,
                d: 1.0,
                e: 0.0,
                f: 0.0,
            });
            self.pdf_graphics
                .begin_transform(transform, x, y, SP_TO_BP);
            self.ledger.record_supported("pdf-begin-transform");
            return Ok(true);
        }
        if text == "pdf:etrans" {
            let Some(operations) = self.pdf_graphics.restore_state() else {
                self.unsupported("pdf-transform-stack-underflow")?;
                return Ok(true);
            };
            self.color = self.pdf_graphics.fill;
            self.append_pdf_operations(operations)?;
            self.ledger.record_supported("pdf-end-transform");
            return Ok(true);
        }
        let Some(args) = text.strip_prefix("x:") else {
            return Ok(false);
        };
        let mut tokens = args.split_whitespace();
        let Some(command) = tokens.next() else {
            return Ok(false);
        };
        match command {
            "scale" => {
                let values: Vec<_> = tokens.filter_map(|value| value.parse::<f32>().ok()).collect();
                if values.len() < 2 {
                    self.unsupported("malformed-x-scale")?;
                } else {
                    self.pdf_graphics.concatenate_about_pen(
                        values[0], 0.0, 0.0, values[1], 0.0, 0.0, x, y, SP_TO_BP,
                    );
                    self.ledger.record_supported("x-scale");
                }
            }
            "rotate" => {
                let Some(degrees) = tokens.next().and_then(|value| value.parse::<f32>().ok()) else {
                    self.unsupported("malformed-x-rotate")?;
                    return Ok(true);
                };
                let radians = degrees.to_radians();
                self.pdf_graphics.concatenate_about_pen(
                    radians.cos(),
                    radians.sin(),
                    -radians.sin(),
                    radians.cos(),
                    0.0,
                    0.0,
                    x,
                    y,
                    SP_TO_BP,
                );
                self.ledger.record_supported("x-rotate");
            }
            "gsave" => {
                self.pdf_graphics.save_state();
                self.ledger.record_supported("x-save");
            }
            "grestore" => {
                let Some(operations) = self.pdf_graphics.restore_state() else {
                    self.unsupported("x-transform-stack-underflow")?;
                    return Ok(true);
                };
                self.color = self.pdf_graphics.fill;
                self.append_pdf_operations(operations)?;
                self.ledger.record_supported("x-restore");
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn apply_nonvisual_special(&mut self, contents: &[u8]) -> bool {
        let Ok(text) = std::str::from_utf8(contents) else {
            return false;
        };
        let text = text.trim();
        if text == "landscape" {
            self.ledger.record_supported("landscape");
            return true;
        }
        if text == "dvipdfmx:config" || text.starts_with("dvipdfmx:config ") {
            self.ledger.record_supported("dvipdfmx-config");
            return true;
        }
        let Some(args) = text.strip_prefix("pdf:") else {
            return false;
        };
        let keyword = args
            .trim_start()
            .chars()
            .take_while(|character| character.is_ascii_alphabetic())
            .collect::<String>();
        if matches!(
            keyword.as_str(),
            "dest"
                | "out"
                | "outline"
                | "bann"
                | "eann"
                | "ann"
                | "obj"
                | "put"
                | "close"
                | "bcolor"
                | "ecolor"
                | "docinfo"
                | "docview"
                | "pagesize"
                | "majorversion"
                | "minorversion"
                | "bcontent"
                | "econtent"
                | "bxobj"
                | "exobj"
                | "bform"
                | "eform"
                | "fstream"
        ) {
            self.ledger.record_supported("pdf-nonvisual");
            return true;
        }
        false
    }

    fn apply_color_special(&mut self, contents: &[u8]) -> Result<bool, RenderError> {
        let Ok(text) = std::str::from_utf8(contents) else {
            return Ok(false);
        };
        let text = text.trim();
        if text == "pdfcolorstackinit" || text.starts_with("pdfcolorstackinit ") {
            let args = &text["pdfcolorstackinit".len()..];
            let Some((fill, line)) = parse_pdf_colors(args, self.pdf_graphics.colors()) else {
                self.unsupported("malformed-pdfcolorstackinit")?;
                return Ok(true);
            };
            self.pdf_graphics.set_colors(fill, line);
            self.color = fill;
            self.ledger.record_supported("pdfcolorstackinit");
            return Ok(true);
        }
        if text == "color" || text.starts_with("color ") {
            let args = &text["color".len()..];
            let args = args.trim();
            if let Some(spec) = args.strip_prefix("push") {
                let Some(color) = parse_color_spec(spec.trim()) else {
                    self.unsupported("malformed-color")?;
                    return Ok(true);
                };
                self.color_stack.push(self.pdf_graphics.colors());
                self.color = color;
                self.pdf_graphics.set_colors(color, color);
            } else if args == "pop" {
                let Some((fill, line)) = self.color_stack.pop() else {
                    self.unsupported("color-stack-underflow")?;
                    return Ok(true);
                };
                self.color = fill;
                self.pdf_graphics.set_colors(fill, line);
            } else {
                let Some(color) = parse_color_spec(args) else {
                    self.unsupported("malformed-color")?;
                    return Ok(true);
                };
                self.color = color;
                self.pdf_graphics.set_colors(color, color);
            }
            self.ledger.record_supported("color");
            return Ok(true);
        }
        if text == "pdfcolorstack" || text.starts_with("pdfcolorstack ") {
            let args = &text["pdfcolorstack".len()..];
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
                    let Some((fill, line)) = parse_pdf_colors(args, self.pdf_graphics.colors()) else {
                        self.unsupported("malformed-pdfcolorstack")?;
                        return Ok(true);
                    };
                    self.pdf_color_stacks
                        .entry(stack_number)
                        .or_default()
                        .push(self.pdf_graphics.colors());
                    self.color = fill;
                    self.pdf_graphics.set_colors(fill, line);
                }
                "pop" => {
                    let colors = self
                        .pdf_color_stacks
                        .get_mut(&stack_number)
                        .and_then(Vec::pop);
                    let Some((fill, line)) = colors else {
                        self.unsupported("pdfcolorstack-underflow")?;
                        return Ok(true);
                    };
                    self.color = fill;
                    self.pdf_graphics.set_colors(fill, line);
                }
                "set" => {
                    let Some((fill, line)) = parse_pdf_colors(args, self.pdf_graphics.colors()) else {
                        self.unsupported("malformed-pdfcolorstack")?;
                        return Ok(true);
                    };
                    self.color = fill;
                    self.pdf_graphics.set_colors(fill, line);
                }
                "current" => {}
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
        self.pdf_graphics.reset_page();
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
        if self.apply_transform_special(x, y, contents)? {
            return Ok(());
        }
        if contents == b"pdf:code" || contents.starts_with(b"pdf:code ") {
            let payload = &contents[b"pdf:code".len()..];
            let run = self.pdf_graphics.run(
                payload.strip_prefix(b" ").unwrap_or(payload),
                x,
                y,
                SP_TO_BP,
            );
            self.color = self.pdf_graphics.fill;
            self.append_pdf_operations(run.operations)?;
            for name in run.supported {
                self.ledger.record_supported(name);
            }
            for name in run.unsupported {
                self.unsupported(&name)?;
            }
            return Ok(());
        }
        let image_origin = self.pdf_graphics.current_transform(x, y, SP_TO_BP);
        if let Some(image) = parse_image_special(contents, image_origin) {
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
        if self.apply_nonvisual_special(contents) {
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

    fn handle_char_run_ext(
        &mut self,
        font_num: i32,
        chars: &[i32],
        x: i32,
        y: i32,
    ) -> Result<i32, Self::Error> {
        let Some(font) = self.classic_fonts.get(&font_num) else {
            self.unsupported("undefined-classic-font")?;
            return Ok(0);
        };
        let Some(tfm) = font.tfm.as_ref() else {
            self.unsupported("missing-tfm")?;
            return Ok(0);
        };
        let mut pen = x;
        let mut positions = Vec::with_capacity(chars.len());
        for &character in chars {
            positions.push(ClassicPosition {
                character,
                transform: self.pdf_graphics.current_transform(pen, y, SP_TO_BP),
            });
            pen = pen.wrapping_add(tfm.advance(character, font.scale));
        }
        let advance = pen.wrapping_sub(x);
        let scale = font.scale;
        let identities = (
            font.tfm_identity,
            font.program_identity,
            font.encoding_identity,
        );
        let (Some(tfm_identity), Some(program_identity), Some(encoding_identity)) = identities
        else {
            self.unsupported("incomplete-classic-font-resources")?;
            return Ok(advance);
        };
        let color = self.color;
        self.page_mut()?
            .operations
            .push(DrawOperation::ClassicChars {
                tfm_identity,
                program_identity,
                encoding_identity,
                scale,
                chars: positions,
                color,
            });
        self.ledger.record_supported("classic-char-run");
        Ok(advance)
    }

    fn handle_define_font(
        &mut self,
        font_num: i32,
        _checksum: u32,
        scale_factor: u32,
        _design_size: u32,
        area: &str,
        name: &str,
    ) -> Result<(), Self::Error> {
        let resource_name = format!("{area}{name}");
        if self.font_map.is_none() {
            let mut map = FontMap::default();
            for map_name in ["pdftex.map", "kanjix.map", "ckx.map"] {
                if let Some(resource) = self.provider.resolve(&ResourceRequest {
                    kind: ResourceKind::FontMap,
                    name: map_name.into(),
                })? {
                    map.parse_into(&resource.bytes);
                }
            }
            self.font_map = Some(map);
        }
        let mapped = self
            .font_map
            .as_ref()
            .and_then(|map| map.get(&resource_name).or_else(|| map.get(name)))
            .cloned();
        let program_name = mapped
            .as_ref()
            .and_then(|entry| entry.program.as_ref())
            .cloned()
            .unwrap_or_else(|| resource_name.clone());
        let encoding_name = mapped
            .as_ref()
            .and_then(|entry| entry.encoding.as_ref())
            .cloned()
            .unwrap_or_else(|| resource_name.clone());
        let tfm_resource = self.provider.resolve(&ResourceRequest {
            kind: ResourceKind::Tfm,
            name: resource_name.clone(),
        })?;
        let program = self.provider.resolve(&ResourceRequest {
            kind: ResourceKind::FontProgram,
            name: program_name,
        })?;
        let mut encoding = self.provider.resolve(&ResourceRequest {
            kind: ResourceKind::Encoding,
            name: encoding_name,
        })?;
        if encoding.is_none() {
            encoding = program
                .as_ref()
                .and_then(|program| type1_builtin(&program.bytes));
        }
        let tfm = tfm_resource
            .as_ref()
            .and_then(|resource| Tfm::parse(&resource.bytes));
        let tfm_identity = tfm_resource.as_ref().map(|resource| resource.identity);
        let program_identity = program.as_ref().map(|resource| resource.identity);
        let encoding_identity = encoding.as_ref().map(|resource| resource.identity);
        for resource in [tfm_resource, program, encoding].into_iter().flatten() {
            self.resources.insert(resource.identity, resource);
        }
        if tfm.is_some() && program_identity.is_some() && encoding_identity.is_some() {
            self.ledger.record_supported("classic-font-resource");
        } else {
            self.ledger.record_unsupported("classic-font-resource");
        }
        self.classic_fonts.insert(
            font_num,
            ClassicFont {
                scale: scale_factor,
                tfm,
                tfm_identity,
                program_identity,
                encoding_identity,
            },
        );
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
            .map(|((&glyph_id, &x), &y)| GlyphPosition {
                glyph_id,
                transform: self.pdf_graphics.current_transform(x, y, SP_TO_BP),
            })
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
            let transform = self.pdf_graphics.current_transform(x, y, SP_TO_BP);
            self.page_mut()?.operations.push(DrawOperation::Rule {
                height,
                width,
                transform,
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

fn parse_pdf_colors(args: &str, current: (Color, Color)) -> Option<(Color, Color)> {
    let open = args.find('(')?;
    let close = args[open + 1..]
        .find(')')
        .map(|index| open + 1 + index)
        .unwrap_or(args.len());
    let (mut fill, mut line) = current;
    let mut values = Vec::new();
    let mut recognized = false;
    for token in args[open + 1..close].split_whitespace() {
        if let Ok(value) = token.parse::<f32>() {
            values.push(value);
            continue;
        }
        let tail = |count: usize| values.get(values.len().checked_sub(count)?..);
        match token {
            "g" => fill = Color::gray(*tail(1)?.first()?),
            "G" => line = Color::gray(*tail(1)?.first()?),
            "rg" => {
                let values = tail(3)?;
                fill = Color::rgb(values[0], values[1], values[2]);
            }
            "RG" => {
                let values = tail(3)?;
                line = Color::rgb(values[0], values[1], values[2]);
            }
            "k" => {
                let values = tail(4)?;
                fill = Color::cmyk(values[0], values[1], values[2], values[3]);
            }
            "K" => {
                let values = tail(4)?;
                line = Color::cmyk(values[0], values[1], values[2], values[3]);
            }
            _ => return None,
        }
        recognized = true;
        values.clear();
    }
    recognized.then_some((fill, line))
}

fn parse_named_matrix(args: &str) -> Option<Matrix> {
    let tokens: Vec<_> = args.split_whitespace().collect();
    let index = tokens.iter().position(|token| *token == "matrix")?;
    let values: Option<Vec<f32>> = tokens
        .get(index + 1..index + 7)?
        .iter()
        .map(|value| value.parse().ok().map(normalize_float))
        .collect();
    let values = values?;
    Some(Matrix {
        a: values[0],
        b: values[1],
        c: values[2],
        d: values[3],
        e: values[4],
        f: values[5],
    })
}

fn parse_image_special(contents: &[u8], mut origin: Matrix) -> Option<ImageSpecial> {
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
                ResourceKind::Tfm => include_bytes!("../../../tests/assets/cmr10.tfm"),
                ResourceKind::FontProgram => include_bytes!("../../../tests/assets/cmr10.pfb"),
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
                    Some(include_bytes!("../../../tests/assets/cmr10.tfm"))
                }
                (ResourceKind::FontProgram, "mapped.pfb") => {
                    Some(include_bytes!("../../../tests/assets/cmr10.pfb"))
                }
                (ResourceKind::Encoding, "mapped.enc") => Some(b"/Encoding [ /A ] def"),
                _ => None,
            };
            Ok(bytes.map(Resource::from_bytes))
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

    fn special_xdv(specials: &[&str]) -> Arc<[u8]> {
        let mut bytes = vec![247, 7];
        bytes.extend_from_slice(&25_400_000u32.to_be_bytes());
        bytes.extend_from_slice(&473_628_672u32.to_be_bytes());
        bytes.extend_from_slice(&1000u32.to_be_bytes());
        bytes.push(0);
        bytes.push(139);
        bytes.extend_from_slice(&[0; 40]);
        bytes.extend_from_slice(&(-1i32).to_be_bytes());
        for special in specials {
            bytes.push(239);
            bytes.push(special.len() as u8);
            bytes.extend_from_slice(special.as_bytes());
        }
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
                "pdf:dest (section.1) [@thispage /XYZ @xpos @ypos null]",
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
        assert_eq!(analysis.ledger().supported().get("pdf-nonvisual"), Some(&4));
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
            .analyze(wrap_page_in_transform(
                &rule_xdv("color rgb 1 0 0"),
                matrix,
            ))
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
        let DrawOperation::ClassicChars { chars, scale, .. } = &analysis.plans[0].operations[0]
        else {
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
        let preview = backend.rasterize_svg(pages[0].svg.as_bytes(), 72.0).unwrap();
        let canonical = backend.rasterize_pdf(&pdf, 0, 72.0).unwrap();
        assert_eq!((preview.width, preview.height), (canonical.width, canonical.height));
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
}
