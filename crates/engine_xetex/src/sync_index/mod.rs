//! Checkpoint-safe in-memory source geometry for live preview.
//!
//! The index keeps the SyncTeX record stream of each page and answers queries
//! with the node selection rules of TeX Live's `synctex_parser.c`, so results
//! match the `synctex edit` and `synctex view` command line tools.

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

mod query;
mod text;
mod tree;

use tree::Page;

/// One page-absolute source rectangle in SyncTeX scaled-point coordinates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyncRect {
    /// Zero-based output page.
    pub page_index: usize,
    /// Engine-reported source name, shared across records and snapshots.
    pub path: Arc<str>,
    /// One-based source line.
    pub line: usize,
    /// Left edge in scaled points, including SyncTeX's one-inch origin.
    pub h: i32,
    /// Baseline in scaled points, including SyncTeX's one-inch origin.
    pub v: i32,
    /// Box width in scaled points, never negative.
    pub width: i32,
    /// Height above the baseline in scaled points.
    pub height: i32,
    /// Depth below the baseline in scaled points.
    pub depth: i32,
}

impl SyncRect {
    /// Left, top, right and bottom edges.
    fn extent(&self) -> [i64; 4] {
        let (h, v) = (i64::from(self.h), i64::from(self.v));
        [
            h,
            v - i64::from(self.height),
            h + i64::from(self.width),
            v + i64::from(self.depth),
        ]
    }

    fn has_area(&self) -> bool {
        self.width > 0 && self.height.saturating_add(self.depth) > 0
    }

    fn encloses(&self, other: &Self) -> bool {
        let [left, top, right, bottom] = self.extent();
        let [l, t, r, b] = other.extent();
        left <= l && top <= t && right >= r && bottom >= b
    }
}

/// What a viewer shows for a forward search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Highlight {
    /// The best `synctex view` box, where a viewer scrolls.
    pub best: SyncRect,
    /// The boxes to flash, all on the page of `best`, possibly none.
    pub boxes: Vec<SyncRect>,
}

/// Whether boxes stack like the lines of a paragraph, each starting below the
/// middle of the one above and at most a line below its bottom.
fn is_one_block(boxes: &[SyncRect]) -> bool {
    let mut spans = boxes
        .iter()
        .map(|rect| {
            let [_, top, _, bottom] = rect.extent();
            (top, bottom)
        })
        .collect::<Vec<_>>();
    spans.sort_unstable();
    !spans.is_empty()
        && spans.windows(2).all(|pair| {
            let ((top, bottom), (next_top, next_bottom)) = (pair[0], pair[1]);
            let line = (bottom - top).max(next_bottom - next_top);
            next_top >= bottom - (bottom - top) / 2 && next_top <= bottom + line
        })
}

/// The SyncTeX record types, named by their `.synctex` prefix.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RecordKind {
    /// `[`
    VBox,
    /// `]`
    VBoxEnd,
    /// `(`
    HBox,
    /// `)`
    HBoxEnd,
    /// `v`
    VoidVBox,
    /// `h`
    VoidHBox,
    /// `k`
    Kern,
    /// `g`
    Glue,
    /// `r`
    Rule,
    /// `$`
    Math,
    /// `x`
    Boundary,
}

impl RecordKind {
    pub(crate) fn from_prefix(prefix: char) -> Option<Self> {
        Some(match prefix {
            '[' => Self::VBox,
            ']' => Self::VBoxEnd,
            '(' => Self::HBox,
            ')' => Self::HBoxEnd,
            'v' => Self::VoidVBox,
            'h' => Self::VoidHBox,
            'k' => Self::Kern,
            'g' => Self::Glue,
            'r' => Self::Rule,
            '$' => Self::Math,
            'x' => Self::Boundary,
            _ => return None,
        })
    }

    fn is_end(self) -> bool {
        matches!(self, Self::VBoxEnd | Self::HBoxEnd)
    }
}

/// One SyncTeX record of a page, in shipout order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Record {
    page_index: usize,
    kind: RecordKind,
    tag: i32,
    line: i32,
    h: i32,
    v: i32,
    width: i32,
    height: i32,
    depth: i32,
}

impl Record {
    fn empty(page_index: usize, kind: RecordKind) -> Self {
        Self {
            page_index,
            kind,
            tag: 0,
            line: 0,
            h: 0,
            v: 0,
            width: 0,
            height: 0,
            depth: 0,
        }
    }
}

/// Immutable source-navigation index carried by one coherent engine frame.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SyncIndex {
    names: BTreeMap<i32, Arc<str>>,
    /// Records by page, shared with checkpoints and earlier snapshots.
    pages: Vec<Arc<[Record]>>,
}

impl SyncIndex {
    /// Build an index from the text of an uncompressed `.synctex` file.
    pub fn from_synctex(text: &str) -> Self {
        text::parse(text)
    }

    /// Source names referenced by indexed records.
    pub fn sources(&self) -> impl Iterator<Item = &str> {
        let mut tags = self.records().map(|record| record.tag).collect::<Vec<_>>();
        tags.sort_unstable();
        tags.dedup();
        tags.into_iter()
            .filter_map(|tag| self.names.get(&tag).map(|name| &**name))
    }

    /// Preserve records for pages beyond a partial foreground coverage.
    pub fn with_preserved_tail(mut self, previous: &Self, first_stale_page: usize) -> Self {
        // Tags are per compile, so previous records are renamed through their paths.
        let mut by_path = self
            .names
            .iter()
            .map(|(tag, name)| (Arc::clone(name), *tag))
            .collect::<HashMap<_, _>>();
        let mut next_tag = self.names.keys().next_back().map_or(1, |tag| tag + 1);
        let mut renamed = HashMap::new();
        let mut rename = |tag: i32| {
            *renamed.entry(tag).or_insert_with(|| {
                let Some(name) = previous.names.get(&tag) else {
                    return 0;
                };
                *by_path.entry(Arc::clone(name)).or_insert_with(|| {
                    let tag = next_tag;
                    next_tag += 1;
                    self.names.insert(tag, Arc::clone(name));
                    tag
                })
            })
        };
        let tail = previous
            .pages
            .iter()
            .enumerate()
            .skip(first_stale_page)
            .filter(|(_, records)| !records.is_empty())
            .map(|(page_index, records)| {
                let records = if records
                    .iter()
                    .all(|record| rename(record.tag) == record.tag)
                {
                    Arc::clone(records)
                } else {
                    records
                        .iter()
                        .map(|record| Record {
                            tag: rename(record.tag),
                            ..*record
                        })
                        .collect()
                };
                (page_index, records)
            })
            .collect::<Vec<_>>();
        for (page_index, records) in tail {
            self.push_page(page_index, records);
        }
        self
    }

    fn push_page(&mut self, page_index: usize, records: Arc<[Record]>) {
        if self.pages.len() <= page_index {
            self.pages.resize_with(page_index + 1, || Arc::from([]));
        }
        let page = &mut self.pages[page_index];
        *page = if page.is_empty() {
            records
        } else {
            page.iter().chain(records.iter()).copied().collect()
        };
    }

    fn records(&self) -> impl Iterator<Item = &Record> {
        self.pages.iter().flat_map(|records| records.iter())
    }

    /// The boxes showing a source line, best first, like `synctex view`.
    pub fn forward(&self, path: &str, line: usize) -> Vec<SyncRect> {
        self.display(path, line, query::display)
    }

    /// Where a viewer should scroll for a source line, and what to flash.
    ///
    /// Unlike [`Self::forward`], a line without boxes looks below itself first,
    /// as a beamer frame is typeset under the line of its `\end{frame}`. Only
    /// the innermost boxes with an area on the page of the best box are
    /// flashed, since the enclosing ones are frames, columns and page bodies.
    /// A line that fell back to another flashes nothing unless those boxes are
    /// one block of text, as a frame's are scattered from headline to footline.
    pub fn highlight(&self, path: &str, line: usize) -> Option<Highlight> {
        let found = self.display(path, line, query::display_ahead);
        let best = found.first()?.clone();
        let mut shown: Vec<SyncRect> = Vec::new();
        for rect in found {
            if rect.page_index == best.page_index
                && rect.has_area()
                && !shown.iter().any(|seen| seen.extent() == rect.extent())
            {
                shown.push(rect);
            }
        }
        let encloses_another = |outer: &SyncRect| {
            shown
                .iter()
                .any(|inner| inner.extent() != outer.extent() && outer.encloses(inner))
        };
        let innermost = shown
            .iter()
            .filter(|rect| !encloses_another(rect))
            .cloned()
            .collect::<Vec<_>>();
        let boxes = if best.line == line || is_one_block(&innermost) {
            innermost
        } else {
            Vec::new()
        };
        Some(Highlight { best, boxes })
    }

    fn display(
        &self,
        path: &str,
        line: usize,
        query: fn(&Self, i32, i32) -> Vec<query::DisplayHit>,
    ) -> Vec<SyncRect> {
        let Some(tag) = self.tag_of(path) else {
            return Vec::new();
        };
        let line = i32::try_from(line).unwrap_or(i32::MAX);
        query(self, tag, line)
            .into_iter()
            .filter_map(|hit| self.rect(hit.page_index, hit.tag, hit.line, hit.geometry))
            .collect()
    }

    /// The source position of a page point, like `synctex edit`.
    pub fn backward(&self, page_index: usize, h: i32, v: i32) -> Option<SyncRect> {
        let page = self.page(page_index)?;
        let node = query::edit(&page, h, v)?;
        let node = page.node(node);
        self.rect(page_index, node.tag, node.line, node.geometry())
    }

    fn rect(&self, page_index: usize, tag: i32, line: i32, geometry: [i32; 5]) -> Option<SyncRect> {
        let [h, v, width, height, depth] = geometry;
        let (h, width) = if width < 0 {
            (h.saturating_add(width), width.saturating_neg())
        } else {
            (h, width)
        };
        Some(SyncRect {
            page_index,
            path: Arc::clone(self.names.get(&tag)?),
            line: usize::try_from(line).ok()?,
            h,
            v,
            width,
            height,
            depth,
        })
    }

    fn page_records(&self, page_index: usize) -> impl Iterator<Item = &Record> {
        self.pages
            .get(page_index)
            .into_iter()
            .flat_map(|records| records.iter())
    }

    fn page(&self, page_index: usize) -> Option<Page> {
        let mut records = self.page_records(page_index).peekable();
        records.peek()?;
        Some(Page::build(records))
    }

    /// The tag `synctex view` resolves for a source name, the last input of it.
    fn tag_of(&self, path: &str) -> Option<i32> {
        let path = normalized_tex_path(path);
        self.names
            .iter()
            .rev()
            .find(|(_, name)| same_tex_path(&normalized_tex_path(name), &path))
            .map(|(tag, _)| *tag)
    }
}

fn normalized_tex_path(path: &str) -> String {
    let mut path = path.replace('\\', "/");
    if !path
        .rsplit('/')
        .next()
        .is_some_and(|name| name.contains('.'))
    {
        path.push_str(".tex");
    }
    path
}

fn same_tex_path(left: &str, right: &str) -> bool {
    let is_suffix = |long: &str, short: &str| {
        long.strip_suffix(short)
            .is_some_and(|dir| dir.ends_with('/'))
    };
    left == right || is_suffix(left, right) || is_suffix(right, left)
}

#[derive(Clone, Default)]
pub(crate) struct PreviewSyncIndex {
    index: SyncIndex,
    page_index: Option<usize>,
    open: Vec<Record>,
}

impl PreviewSyncIndex {
    pub(crate) fn register_name(&mut self, tag: i32, path: String) {
        if tag > 0 {
            self.index.names.insert(tag, Arc::from(path));
        }
    }

    pub(crate) fn begin_page(&mut self, page_index: usize) {
        self.flush();
        self.page_index = Some(page_index);
    }

    pub(crate) fn end_page(&mut self) {
        self.flush();
        self.page_index = None;
    }

    fn flush(&mut self) {
        if let (Some(page_index), false) = (self.page_index, self.open.is_empty()) {
            let records = Arc::from(std::mem::take(&mut self.open));
            self.index.push_page(page_index, records);
        }
    }

    /// Record a node as `[tag, line, h, v]` plus up to three dimensions.
    pub(crate) fn record(&mut self, kind: RecordKind, position: [i32; 4], dims: &[i32]) {
        let Some(page_index) = self.page_index else {
            return;
        };
        let [tag, line, h, v] = position;
        let dim = |i: usize| dims.get(i).copied().unwrap_or(0);
        self.open.push(Record {
            page_index,
            kind,
            tag,
            line,
            h,
            v,
            width: dim(0),
            height: dim(1),
            depth: dim(2),
        });
    }

    pub(crate) fn close(&mut self, kind: RecordKind) {
        if let Some(page_index) = self.page_index {
            self.open.push(Record::empty(page_index, kind));
        }
    }

    pub(crate) fn snapshot(&self) -> SyncIndex {
        let mut index = self.index.clone();
        if let (Some(page_index), false) = (self.page_index, self.open.is_empty()) {
            index.push_page(page_index, Arc::from(self.open.as_slice()));
        }
        index
    }

    /// Bytes a clone owns alone, since finished pages are shared.
    pub(crate) fn allocated_bytes(&self) -> usize {
        self.index.pages.capacity() * std::mem::size_of::<Arc<[Record]>>()
            + self.open.capacity() * std::mem::size_of::<Record>()
    }
}

#[cfg(test)]
mod tests;
