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
    records: Vec<Record>,
}

impl SyncIndex {
    /// Build an index from the text of an uncompressed `.synctex` file.
    pub fn from_synctex(text: &str) -> Self {
        text::parse(text)
    }

    /// Source names referenced by indexed records.
    pub fn sources(&self) -> impl Iterator<Item = &str> {
        let mut tags = self
            .records
            .iter()
            .map(|record| record.tag)
            .collect::<Vec<_>>();
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
        for record in previous
            .records
            .iter()
            .filter(|record| record.page_index >= first_stale_page)
        {
            let tag = *renamed.entry(record.tag).or_insert_with(|| {
                let Some(name) = previous.names.get(&record.tag) else {
                    return 0;
                };
                *by_path.entry(Arc::clone(name)).or_insert_with(|| {
                    let tag = next_tag;
                    next_tag += 1;
                    self.names.insert(tag, Arc::clone(name));
                    tag
                })
            });
            self.records.push(Record { tag, ..*record });
        }
        self
    }

    /// The boxes showing a source line, best first, like `synctex view`.
    pub fn forward(&self, path: &str, line: usize) -> Vec<SyncRect> {
        let Some(tag) = self.tag_of(path) else {
            return Vec::new();
        };
        let line = i32::try_from(line).unwrap_or(i32::MAX);
        query::display(self, tag, line)
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
        self.records
            .iter()
            .filter(move |record| record.page_index == page_index)
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
    names: BTreeMap<i32, Arc<str>>,
    page_index: Option<usize>,
    records: Vec<Record>,
}

impl PreviewSyncIndex {
    pub(crate) fn register_name(&mut self, tag: i32, path: String) {
        if tag > 0 {
            self.names.insert(tag, Arc::from(path));
        }
    }

    pub(crate) fn begin_page(&mut self, page_index: usize) {
        self.page_index = Some(page_index);
    }

    pub(crate) fn end_page(&mut self) {
        self.page_index = None;
    }

    /// Record a node as `[tag, line, h, v]` plus up to three dimensions.
    pub(crate) fn record(&mut self, kind: RecordKind, position: [i32; 4], dims: &[i32]) {
        let Some(page_index) = self.page_index else {
            return;
        };
        let [tag, line, h, v] = position;
        let dim = |i: usize| dims.get(i).copied().unwrap_or(0);
        self.records.push(Record {
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
            self.records.push(Record::empty(page_index, kind));
        }
    }

    pub(crate) fn snapshot(&self) -> SyncIndex {
        SyncIndex {
            names: self.names.clone(),
            records: self.records.clone(),
        }
    }
}

#[cfg(test)]
mod tests;
