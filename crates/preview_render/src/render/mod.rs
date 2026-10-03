//! Transactional XDV analysis and selected-page rendering.

mod plan;
#[cfg(test)]
mod tests;
mod walker;

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;
use std::sync::Arc;

use tectonic_xdv::{FileType, XdvParser};

use crate::backend::{Backend, FontId, FontStyle};
use crate::fingerprint::page_digest;
use crate::pagesize::preamble_magnification;
use crate::resource::{Resource, ResourceIdentity, ResourceProvider};
use crate::units::{fixed_16_16, sp_to_bp, SP_TO_BP};
use crate::{Destination, Link, RenderError};

pub(crate) use plan::{DrawOperation, ImageKind, PagePlan};
use walker::{FontArtifacts, WalkResume, Walker};

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
    /// Clickable regions, in page coordinates.
    pub links: Vec<Link>,
    /// Named spots on this page that a link can reach.
    pub destinations: Vec<Destination>,
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
    plans: Vec<Arc<PagePlan>>,
    resources: BTreeMap<ResourceIdentity, Resource>,
    pages: Vec<PageVisual>,
    ledger: CoverageLedger,
    reused_pages: usize,
}

impl Analysis {
    /// Exact XDV bytes against which this operation was analyzed.
    pub fn xdv(&self) -> &Arc<[u8]> {
        &self.xdv
    }

    /// Leading pages taken unchanged from the previous analysis.
    pub fn reused_pages(&self) -> usize {
        self.reused_pages
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

/// Last completed walk, reusable while a document keeps its leading bytes.
struct WalkCache {
    xdv: Arc<[u8]>,
    filetype: FileType,
    plans: Vec<Arc<PagePlan>>,
    pages: Vec<PageVisual>,
    resumes: Vec<WalkResume>,
    /// Plans of the pages past `plans` from an earlier, longer walk.
    ///
    /// A viewport-bounded edit cuts the document short, and freeing the
    /// hundreds of plans below it at once cost that keystroke several
    /// milliseconds. They go as fresh pages replace them instead.
    stale: Vec<Arc<PagePlan>>,
}

impl WalkCache {
    /// This walk's plans from page `len` on, which a walk of `len` pages leaves stale.
    fn stale_after(mut self, len: usize) -> Vec<Arc<PagePlan>> {
        self.plans.append(&mut self.stale);
        if self.plans.len() > len {
            self.plans.split_off(len)
        } else {
            Vec::new()
        }
    }
}

/// Stateful renderer that retains only immutable resources and completed facts.
pub struct HotRenderer<P> {
    provider: P,
    next_analysis_id: u64,
    last_completed: Option<(Vec<PageVisual>, CoverageLedger)>,
    fonts: FontArtifacts,
    cache: Option<WalkCache>,
}

impl<P: ResourceProvider> HotRenderer<P> {
    /// Construct a renderer around an immutable-byte resource provider.
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            next_analysis_id: 1,
            last_completed: None,
            fonts: FontArtifacts::default(),
            cache: None,
        }
    }

    /// Analyze every page from byte zero, freezing all resources the walk uses.
    pub fn analyze(&mut self, xdv: Arc<[u8]>) -> Result<Analysis, RenderError> {
        self.analyze_from(xdv, &[])
    }

    /// Analyze every page, reusing leading pages whose bytes are unchanged.
    ///
    /// `page_offsets` are the absolute begin-page offsets in page order. Without
    /// them, or without a matching resume point, the whole document is walked.
    pub fn analyze_from(
        &mut self,
        xdv: Arc<[u8]>,
        page_offsets: &[u32],
    ) -> Result<Analysis, RenderError> {
        let analysis_id = self.next_analysis_id;
        self.next_analysis_id = self.next_analysis_id.wrapping_add(1).max(1);
        let magnification = preamble_magnification(&xdv)?;
        let (mut plans, mut pages, mut walker) = match self.resumption(&xdv, page_offsets) {
            Some(resumption) => {
                let offset = resumption.offset;
                let filetype = resumption.filetype;
                let walker = Walker::resume(
                    &self.provider,
                    &mut self.fonts,
                    resumption.resume,
                    filetype,
                    resumption.earlier,
                );
                let (walker, _) = XdvParser::process_from_page(
                    Cursor::new(&xdv[offset..]),
                    walker,
                    filetype,
                    offset as u64,
                )?;
                (resumption.plans, resumption.pages, walker)
            }
            None => {
                let walker = Walker::new(&self.provider, &mut self.fonts, magnification);
                let (walker, _) = XdvParser::process(Cursor::new(&xdv), walker)?;
                (Vec::new(), Vec::new(), walker)
            }
        };
        walker.finish_page();

        let first_fresh = plans.len();
        plans.reserve(walker.pages.len());
        pages.reserve(walker.pages.len());
        for (page_index, plan) in (first_fresh..).zip(walker.pages) {
            let fingerprint = if plan.complete {
                PageFingerprint::Stable(page_digest(&plan))
            } else {
                PageFingerprint::Uncertain {
                    analysis_id,
                    page_index,
                }
            };
            pages.push(PageVisual {
                fingerprint,
                size_bp: [plan.width_bp, plan.height_bp],
                links: plan.links.clone(),
                destinations: plan.destinations.clone(),
                unsupported: plan.unsupported.clone(),
            });
            plans.push(Arc::new(plan));
        }

        let stale = self
            .cache
            .take()
            .map_or_else(Vec::new, |previous| previous.stale_after(plans.len()));
        self.cache = Some(WalkCache {
            xdv: Arc::clone(&xdv),
            filetype: walker.filetype,
            plans: plans.clone(),
            pages: pages.clone(),
            resumes: walker.resumes,
            stale,
        });
        Ok(Analysis {
            xdv,
            plans,
            resources: walker.state.resources,
            pages,
            ledger: walker.state.ledger,
            reused_pages: first_fresh,
        })
    }

    /// Deepest resume point the new document reaches with unchanged bytes.
    fn resumption(&self, xdv: &[u8], page_offsets: &[u32]) -> Option<Resumption> {
        let cache = self.cache.as_ref()?;
        // Only bytes the renderer compared itself count as unchanged.
        let shared = shared_prefix(xdv, &cache.xdv);
        let (resume, offset) = cache.resumes.iter().rev().find_map(|resume| {
            let offset = *page_offsets.get(resume.page_index)? as usize;
            (resume.page_index <= cache.plans.len() && offset <= shared).then_some((resume, offset))
        })?;
        let page_index = resume.page_index;
        if page_index == 0 {
            return None;
        }
        Some(Resumption {
            resume: resume.clone(),
            offset,
            filetype: cache.filetype,
            plans: cache.plans.get(..page_index)?.to_vec(),
            pages: cache.pages.get(..page_index)?.to_vec(),
            // Kept so a later edit further up can still resume.
            earlier: cache
                .resumes
                .iter()
                .filter(|earlier| earlier.page_index < page_index)
                .cloned()
                .collect(),
        })
    }

    /// Render exactly the selected pages, committing analysis facts only once all succeed.
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

        let mut fonts = FontHandles::default();
        let mut rendered = Vec::with_capacity(page_indices.len());
        for &page_index in page_indices {
            let page = &analysis.plans[page_index];
            backend.begin_page(page.width_bp, page.height_bp)?;
            for operation in &page.operations {
                draw(analysis, &mut fonts, backend, operation)?;
            }
            rendered.push(PageSvg {
                page_index,
                svg: backend.finish_page()?,
            });
        }
        self.last_completed = Some((analysis.pages.clone(), analysis.ledger.clone()));
        Ok(rendered)
    }

    /// Free the plans an earlier, longer walk left past the last one.
    ///
    /// For a host that knows the last walk covered the whole document.
    pub fn release_stale_plans(&mut self) {
        if let Some(cache) = &mut self.cache {
            cache.stale = Vec::new();
        }
    }

    /// Visual and coverage facts from the last transactionally completed render.
    pub fn last_completed(&self) -> Option<(&[PageVisual], &CoverageLedger)> {
        self.last_completed
            .as_ref()
            .map(|(pages, ledger)| (pages.as_slice(), ledger))
    }
}

/// Cached walk data a new analysis can restart from.
struct Resumption {
    resume: WalkResume,
    offset: usize,
    filetype: FileType,
    plans: Vec<Arc<PagePlan>>,
    pages: Vec<PageVisual>,
    earlier: Vec<WalkResume>,
}

/// Backend font handles registered during one `render_pages` call.
#[derive(Default)]
struct FontHandles {
    native: BTreeMap<(ResourceIdentity, u32), FontId>,
    classic: BTreeMap<(ResourceIdentity, ResourceIdentity), FontId>,
}

impl Analysis {
    fn resource(&self, identity: &ResourceIdentity, what: &str) -> Result<&Resource, RenderError> {
        self.resources
            .get(identity)
            .ok_or_else(|| RenderError::Resource(format!("analyzed {what} disappeared")))
    }
}

fn draw<B: Backend>(
    analysis: &Analysis,
    fonts: &mut FontHandles,
    backend: &mut B,
    operation: &DrawOperation,
) -> Result<(), RenderError> {
    match operation {
        DrawOperation::Rule {
            height,
            width,
            transform,
            color,
        } => backend.fill_rule(sp_to_bp(*width), sp_to_bp(*height), *transform, *color),
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
            let font = match fonts.native.entry((*identity, *face_index)) {
                Entry::Occupied(entry) => *entry.get(),
                Entry::Vacant(entry) => {
                    let resource = analysis.resource(identity, "font bytes")?;
                    *entry.insert(backend.register_font(*identity, &resource.bytes, *face_index)?)
                }
            };
            let style = FontStyle {
                extend: fixed_16_16(*extend),
                slant: fixed_16_16(*slant),
                embolden: fixed_16_16(*embolden),
                vertical: *vertical,
            };
            backend.fill_glyphs(font, sp_to_bp(*size), glyphs, *color, style)
        }
        DrawOperation::Image {
            identity,
            kind,
            transform,
            page,
            width,
            height,
        } => {
            let resource = analysis.resource(identity, "image bytes")?;
            match kind {
                ImageKind::Raster => {
                    backend.draw_image(&resource.bytes, *transform, *width, *height)
                }
                ImageKind::Pdf => backend.draw_pdf(&resource.bytes, *page, *transform),
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
            let font = match fonts.classic.entry((*program_identity, *encoding_identity)) {
                Entry::Occupied(entry) => *entry.get(),
                Entry::Vacant(entry) => {
                    let program = analysis.resource(program_identity, "classic font")?;
                    let encoding = analysis.resource(encoding_identity, "font encoding")?;
                    *entry.insert(backend.register_classic_font(
                        *program_identity,
                        &program.bytes,
                        *encoding_identity,
                        &encoding.bytes,
                    )?)
                }
            };
            backend.fill_chars(font, *scale as f32 * SP_TO_BP, chars, *color)
        }
        DrawOperation::PathFill {
            path,
            even_odd,
            transform,
            color,
        } => backend.fill_path(path, *even_odd, *transform, *color),
        DrawOperation::PathStroke {
            path,
            style,
            transform,
            color,
        } => backend.stroke_path(path, *style, *transform, *color),
        DrawOperation::ClipPath {
            path,
            even_odd,
            transform,
        } => backend.clip_path(path, *even_odd, *transform),
        DrawOperation::PopClip => backend.pop_clip(),
    }
}

fn shared_prefix(left: &[u8], right: &[u8]) -> usize {
    const WORD: usize = std::mem::size_of::<usize>();
    let limit = left.len().min(right.len());
    let mut index = 0;
    while index + WORD <= limit {
        if left[index..index + WORD] != right[index..index + WORD] {
            break;
        }
        index += WORD;
    }
    while index < limit && left[index] == right[index] {
        index += 1;
    }
    index
}
