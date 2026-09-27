//! The XDV walk that turns a document into page plans.

mod annotation;
mod events;
mod specials;

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use tectonic_xdv::FileType;

use crate::color::Color;
use crate::fontmap::FontMap;
use crate::pagesize::PageSizeState;
use crate::pdfcode::PdfGraphics;
use crate::resource::{
    Resource, ResourceIdentity, ResourceKind, ResourceProvider, ResourceRequest,
};
use crate::tfm::Tfm;
use crate::RenderError;

use super::{CoverageLedger, DrawOperation, PagePlan};
use annotation::OpenLink;

/// Parsed font data memoized across analyses by the identity of its source bytes.
#[derive(Default)]
pub(super) struct FontArtifacts {
    font_map: Option<(
        [Option<ResourceIdentity>; FONT_MAP_NAMES.len()],
        Arc<FontMap>,
    )>,
    tfm: HashMap<ResourceIdentity, Option<Arc<Tfm>>>,
    builtin_encoding: HashMap<ResourceIdentity, Option<Resource>>,
}

const FONT_MAP_NAMES: [&str; 3] = ["pdftex.map", "kanjix.map", "ckx.map"];

/// Pages between resume points, trading one state clone each against re-walked pages.
const RESUME_STRIDE: usize = 8;

/// Everything a walk carries across a page break.
#[derive(Clone)]
pub(super) struct WalkState {
    page_size: PageSizeState,
    fonts: BTreeMap<i32, NativeFont>,
    classic_fonts: BTreeMap<i32, ClassicFont>,
    pub(super) resources: BTreeMap<ResourceIdentity, Resource>,
    pub(super) ledger: CoverageLedger,
    color: Color,
    color_stack: Vec<(Color, Color)>,
    pdf_color_stacks: BTreeMap<i32, Vec<(Color, Color)>>,
    pdf_graphics: PdfGraphics,
}

/// Walker state entering a page, enough to walk on from its begin-page opcode.
#[derive(Clone)]
pub(super) struct WalkResume {
    pub(super) page_index: usize,
    state: WalkState,
}

#[derive(Clone, Copy)]
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

#[derive(Clone)]
struct ClassicFont {
    scale: u32,
    tfm: Option<Arc<Tfm>>,
    tfm_identity: Option<ResourceIdentity>,
    program_identity: Option<ResourceIdentity>,
    encoding_identity: Option<ResourceIdentity>,
}

pub(super) struct Walker<'a, P> {
    provider: &'a P,
    artifacts: &'a mut FontArtifacts,
    page: Option<PagePlan>,
    pub(super) pages: Vec<PagePlan>,
    /// Document page index of `pages[0]`, nonzero for a resumed walk.
    first_page_index: usize,
    pub(super) filetype: FileType,
    pub(super) resumes: Vec<WalkResume>,
    pub(super) state: WalkState,
    /// Open between `pdf:bann` and `pdf:eann`, never across a page boundary.
    open_link: Option<OpenLink>,
}

impl<'a, P: ResourceProvider> Walker<'a, P> {
    pub(super) fn new(
        provider: &'a P,
        artifacts: &'a mut FontArtifacts,
        magnification: f32,
    ) -> Self {
        let state = WalkState {
            page_size: PageSizeState::new(magnification),
            fonts: BTreeMap::new(),
            classic_fonts: BTreeMap::new(),
            resources: BTreeMap::new(),
            ledger: CoverageLedger::default(),
            color: Color::BLACK,
            color_stack: Vec::new(),
            pdf_color_stacks: BTreeMap::new(),
            pdf_graphics: PdfGraphics::default(),
        };
        Self::with_state(provider, artifacts, 0, FileType::Xdv, Vec::new(), state)
    }

    pub(super) fn resume(
        provider: &'a P,
        artifacts: &'a mut FontArtifacts,
        resume: WalkResume,
        filetype: FileType,
        mut earlier: Vec<WalkResume>,
    ) -> Self {
        let page_index = resume.page_index;
        let state = resume.state.clone();
        earlier.push(resume);
        Self::with_state(provider, artifacts, page_index, filetype, earlier, state)
    }

    fn with_state(
        provider: &'a P,
        artifacts: &'a mut FontArtifacts,
        first_page_index: usize,
        filetype: FileType,
        resumes: Vec<WalkResume>,
        state: WalkState,
    ) -> Self {
        Self {
            provider,
            artifacts,
            page: None,
            pages: Vec::new(),
            first_page_index,
            filetype,
            resumes,
            state,
            open_link: None,
        }
    }

    fn capture_resume(&mut self) {
        let page_index = self.first_page_index + self.pages.len();
        if !page_index.is_multiple_of(RESUME_STRIDE) {
            return;
        }
        if self
            .resumes
            .last()
            .is_some_and(|last| last.page_index >= page_index)
        {
            return;
        }
        self.resumes.push(WalkResume {
            page_index,
            state: self.state.clone(),
        });
    }

    fn font_map(&mut self) -> Result<Arc<FontMap>, RenderError> {
        let mut identities = [None; FONT_MAP_NAMES.len()];
        let mut resources = Vec::with_capacity(FONT_MAP_NAMES.len());
        for (slot, map_name) in identities.iter_mut().zip(FONT_MAP_NAMES) {
            let resource = self.provider.resolve(&ResourceRequest {
                kind: ResourceKind::FontMap,
                name: map_name.into(),
            })?;
            *slot = resource.as_ref().map(|resource| resource.identity);
            resources.extend(resource);
        }
        if let Some((cached, map)) = self.artifacts.font_map.as_ref() {
            if *cached == identities {
                return Ok(Arc::clone(map));
            }
        }
        let mut map = FontMap::default();
        for resource in resources {
            map.parse_into(&resource.bytes);
        }
        let map = Arc::new(map);
        self.artifacts.font_map = Some((identities, Arc::clone(&map)));
        Ok(map)
    }

    fn page_mut(&mut self) -> Result<&mut PagePlan, RenderError> {
        self.page
            .as_mut()
            .ok_or_else(|| RenderError::Xdv("drawing operation outside a page".into()))
    }

    fn unsupported(&mut self, name: &str) -> Result<(), RenderError> {
        self.state.ledger.record_unsupported(name);
        let page = self.page_mut()?;
        page.complete = false;
        *page.unsupported.entry(name.into()).or_default() += 1;
        Ok(())
    }

    pub(super) fn finish_page(&mut self) {
        self.close_link();

        if let Some(mut page) = self.page.take() {
            // The media size is final only once all of the page's specials are seen.
            let size = self.state.page_size.current();
            page.width_bp = size[0];
            page.height_bp = size[1];
            self.pages.push(page);
            self.state.page_size.finish_page();
        }
    }

    fn append_pdf_operations(&mut self, operations: Vec<DrawOperation>) -> Result<(), RenderError> {
        if !operations.is_empty() {
            self.page_mut()?.operations.extend(operations);
        }
        Ok(())
    }
}
