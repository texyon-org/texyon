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
use crate::pdfcode::{PdfGraphics, PdfOperation};
use crate::resource::{
    Resource, ResourceIdentity, ResourceKind, ResourceProvider, ResourceRequest,
};
use crate::tfm::Tfm;
use crate::RenderError;

use super::{CoverageLedger, DrawOperation, PagePlan};
use annotation::OpenLink;

/// Font artifacts derived from exact resource bytes, retained across analyses.
///
/// Every keystroke re-walks the whole document and therefore re-defines every
/// font it uses. Parsing the font map, the TFM tables, and the Type1 builtin
/// encodings each time dominated analysis, so the derived forms are memoized
/// under the SHA-256 identity of the bytes they came from: an entry can only be
/// reused when the resolved resource is byte-identical.
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

/// Page interval between retained walk-resume points.
///
/// Every retained point costs one clone of the walker's carried state, and a
/// coarser stride only ever costs a few extra re-walked pages, so this stays
/// small enough to be free and large enough to bound capture work.
const RESUME_STRIDE: usize = 8;

/// Complete walker state entering a page boundary.
///
/// Everything an XDV walk carries across a page break lives here, so restoring
/// one of these and parsing from the matching begin-page opcode produces
/// exactly what a walk from byte zero would have produced.
#[derive(Clone)]
pub(super) struct WalkResume {
    pub(super) page_index: usize,
    page_size: PageSizeState,
    fonts: BTreeMap<i32, NativeFont>,
    classic_fonts: BTreeMap<i32, ClassicFont>,
    resources: BTreeMap<ResourceIdentity, Resource>,
    ledger: CoverageLedger,
    color: Color,
    color_stack: Vec<(Color, Color)>,
    pdf_color_stacks: BTreeMap<i32, Vec<(Color, Color)>>,
    pdf_graphics: PdfGraphics,
}

#[derive(Clone)]
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
    /// Stream type, from the preamble or carried across a resume.
    pub(super) filetype: FileType,
    pub(super) resumes: Vec<WalkResume>,
    page_size: PageSizeState,
    fonts: BTreeMap<i32, NativeFont>,
    classic_fonts: BTreeMap<i32, ClassicFont>,
    pub(super) resources: BTreeMap<ResourceIdentity, Resource>,
    pub(super) ledger: CoverageLedger,
    color: Color,
    color_stack: Vec<(Color, Color)>,
    pdf_color_stacks: BTreeMap<i32, Vec<(Color, Color)>>,
    pdf_graphics: PdfGraphics,
    /// The annotation between a `pdf:bann` and its `pdf:eann`, if inside one.
    open_link: Option<OpenLink>,
}

impl<'a, P: ResourceProvider> Walker<'a, P> {
    pub(super) fn new(
        provider: &'a P,
        artifacts: &'a mut FontArtifacts,
        magnification: f32,
    ) -> Self {
        Self {
            provider,
            artifacts,
            page: None,
            pages: Vec::new(),
            first_page_index: 0,
            filetype: FileType::Xdv,
            resumes: Vec::new(),
            page_size: PageSizeState::new(magnification),
            fonts: BTreeMap::new(),
            classic_fonts: BTreeMap::new(),
            resources: BTreeMap::new(),
            ledger: CoverageLedger::default(),
            color: Color::BLACK,
            color_stack: Vec::new(),
            pdf_color_stacks: BTreeMap::new(),
            pdf_graphics: PdfGraphics::default(),
            open_link: None,
        }
    }

    /// Restore the carried state of a previous walk at a page boundary.
    pub(super) fn resume(
        provider: &'a P,
        artifacts: &'a mut FontArtifacts,
        resume: WalkResume,
        filetype: FileType,
        mut earlier: Vec<WalkResume>,
    ) -> Self {
        earlier.push(resume.clone());
        Self {
            provider,
            artifacts,
            page: None,
            pages: Vec::new(),
            first_page_index: resume.page_index,
            filetype,
            resumes: earlier,
            page_size: resume.page_size,
            fonts: resume.fonts,
            classic_fonts: resume.classic_fonts,
            resources: resume.resources,
            ledger: resume.ledger,
            color: resume.color,
            color_stack: resume.color_stack,
            pdf_color_stacks: resume.pdf_color_stacks,
            pdf_graphics: resume.pdf_graphics,
            // A resume begins at a page boundary, and an annotation cannot
            // cross one, so there is never one to restore.
            open_link: None,
        }
    }

    /// Retain the state entering the page that is about to start, on the
    /// stride that bounds how much a later walk must redo.
    fn capture_resume(&mut self) {
        let page_index = self.first_page_index + self.pages.len();
        if page_index % RESUME_STRIDE != 0 {
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
            page_size: self.page_size.clone(),
            fonts: self.fonts.clone(),
            classic_fonts: self.classic_fonts.clone(),
            resources: self.resources.clone(),
            ledger: self.ledger.clone(),
            color: self.color,
            color_stack: self.color_stack.clone(),
            pdf_color_stacks: self.pdf_color_stacks.clone(),
            pdf_graphics: self.pdf_graphics.clone(),
        });
    }

    /// Font map parsed from the exact current map bytes, reused across analyses
    /// while every contributing resource keeps the same identity.
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
        self.ledger.record_unsupported(name);
        let page = self.page_mut()?;
        page.complete = false;
        *page.unsupported.entry(name.into()).or_default() += 1;
        Ok(())
    }

    pub(super) fn finish_page(&mut self) {
        // An annotation cannot cross a page boundary, so one still open when
        // the page ends is closed against what it has covered so far.
        self.close_link();

        if let Some(mut page) = self.page.take() {
            // The media size is only final once the page's specials have all
            // been seen, so commit it here rather than at the page's start.
            let size = self.page_size.current();
            page.width_bp = size[0];
            page.height_bp = size[1];
            self.pages.push(page);
            self.page_size.finish_page();
        }
    }

    fn append_pdf_operations(&mut self, operations: Vec<PdfOperation>) -> Result<(), RenderError> {
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
}
