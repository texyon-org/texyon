//! Transactional XDV analysis and selected-page rendering.

mod plan;
#[cfg(test)]
mod tests;
mod walker;

use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;
use std::sync::Arc;

use tectonic_xdv::{FileType, XdvParser};

use crate::backend::{Backend, FontStyle, PositionedChar, PositionedGlyph};
use crate::fingerprint::page_digest;
use crate::pagesize::preamble_magnification;
use crate::resource::{Resource, ResourceIdentity, ResourceProvider};
use crate::{Destination, Link, RenderError};

pub(crate) use plan::{fixed_16_16, DrawOperation, ImageKind, PagePlan, SP_TO_BP};
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

/// Result of the last completed walk, reusable while a document keeps its
/// leading bytes.
struct WalkCache {
    xdv: Arc<[u8]>,
    filetype: FileType,
    plans: Vec<Arc<PagePlan>>,
    pages: Vec<PageVisual>,
    resumes: Vec<WalkResume>,
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

    /// Analyze every page, resolving and freezing all resources used by the walk.
    ///
    /// Equivalent to [`analyze_from`](Self::analyze_from) with no page layout,
    /// which always walks the complete document.
    pub fn analyze(&mut self, xdv: Arc<[u8]>) -> Result<Analysis, RenderError> {
        self.analyze_from(xdv, &[])
    }

    /// Analyze every page, reusing the leading pages of the previous analysis
    /// whose exact bytes are unchanged.
    ///
    /// `page_offsets` are the document's absolute begin-page offsets in page
    /// order, as already established by structural validation. A page whose
    /// bytes lie entirely inside the prefix shared with the previous analysis
    /// has, by construction, the same plan and the same visual identity, so the
    /// walk restarts from the last retained resume point inside that prefix
    /// instead of from byte zero. Supplying no offsets, or offsets that do not
    /// match a retained resume point, simply walks everything.
    pub fn analyze_from(
        &mut self,
        xdv: Arc<[u8]>,
        page_offsets: &[u32],
    ) -> Result<Analysis, RenderError> {
        let analysis_id = self.next_analysis_id;
        self.next_analysis_id = self.next_analysis_id.wrapping_add(1).max(1);
        let magnification = preamble_magnification(&xdv)?;
        let resumption = self.resumption(&xdv, page_offsets);
        let (reused_plans, reused_pages, walker) = match resumption {
            Some((resume, offset)) => {
                let cache = self.cache.as_ref().expect("resumption without a cache");
                let filetype = cache.filetype;
                let plans = cache.plans[..resume.page_index].to_vec();
                let pages = cache.pages[..resume.page_index].to_vec();
                // Resume points above the shared prefix are gone, but the ones
                // below it describe bytes this document still has, so keep them
                // and stay resumable for a later edit further up the document.
                let earlier = cache
                    .resumes
                    .iter()
                    .filter(|earlier| earlier.page_index < resume.page_index)
                    .cloned()
                    .collect::<Vec<_>>();
                let walker =
                    Walker::resume(&self.provider, &mut self.fonts, resume, filetype, earlier);
                let (mut walker, _) = XdvParser::process_from_page(
                    Cursor::new(&xdv[offset..]),
                    walker,
                    filetype,
                    offset as u64,
                )?;
                walker.finish_page();
                (plans, pages, walker)
            }
            None => {
                let walker = Walker::new(&self.provider, &mut self.fonts, magnification);
                let (mut walker, _) = XdvParser::process(Cursor::new(&xdv), walker)?;
                walker.finish_page();
                (Vec::new(), Vec::new(), walker)
            }
        };

        let first_fresh = reused_plans.len();
        let mut plans = reused_plans;
        let mut pages = reused_pages;
        plans.reserve(walker.pages.len());
        pages.reserve(walker.pages.len());
        for (offset, plan) in walker.pages.into_iter().enumerate() {
            let page_index = first_fresh + offset;
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

        self.cache = Some(WalkCache {
            xdv: Arc::clone(&xdv),
            filetype: walker.filetype,
            plans: plans.clone(),
            pages: pages.clone(),
            resumes: walker.resumes,
        });
        Ok(Analysis {
            xdv,
            plans,
            resources: walker.resources,
            pages,
            ledger: walker.ledger,
            reused_pages: first_fresh,
        })
    }

    /// Deepest retained resume point that the new document provably reaches
    /// with unchanged bytes, paired with its begin-page offset.
    fn resumption(&self, xdv: &[u8], page_offsets: &[u32]) -> Option<(WalkResume, usize)> {
        let cache = self.cache.as_ref()?;
        // The engine reports which prefix it retained, but the renderer proves
        // it: only bytes it has actually compared may be treated as unchanged.
        let shared = shared_prefix(xdv, &cache.xdv);
        let resume = cache
            .resumes
            .iter()
            .rev()
            .find(|resume| {
                resume.page_index <= cache.plans.len()
                    && page_offsets
                        .get(resume.page_index)
                        .is_some_and(|offset| *offset as usize <= shared)
            })?
            .clone();
        if resume.page_index == 0 {
            return None;
        }
        let offset = page_offsets[resume.page_index] as usize;
        Some((resume, offset))
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

/// Length of the longest common leading run of two documents.
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
