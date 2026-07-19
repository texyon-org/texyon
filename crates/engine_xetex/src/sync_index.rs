//! Checkpoint-safe in-memory source geometry for live preview.

use std::collections::BTreeMap;

/// One page-absolute source rectangle in SyncTeX scaled-point coordinates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyncRect {
    /// Zero-based output page.
    pub page_index: usize,
    /// Engine-reported source name.
    pub path: String,
    /// One-based source line.
    pub line: usize,
    /// Left edge in scaled points, including SyncTeX's one-inch origin.
    pub h: i32,
    /// Baseline in scaled points, including SyncTeX's one-inch origin.
    pub v: i32,
    /// Box width in scaled points.
    pub width: i32,
    /// Height above the baseline in scaled points.
    pub height: i32,
    /// Depth below the baseline in scaled points.
    pub depth: i32,
}

impl SyncRect {
    fn contains(&self, h: i32, v: i32) -> bool {
        let left = self.h.min(self.h.saturating_add(self.width));
        let right = self.h.max(self.h.saturating_add(self.width));
        let top = self.v.saturating_sub(self.height.max(0));
        let bottom = self.v.saturating_add(self.depth.max(0));
        (left..=right).contains(&h) && (top..=bottom).contains(&v)
    }

    fn area(&self) -> u64 {
        u64::from(self.width.unsigned_abs().max(1)).saturating_mul(u64::from(
            self.height.saturating_add(self.depth).unsigned_abs().max(1),
        ))
    }

    fn distance_squared(&self, h: i32, v: i32) -> u128 {
        let dh = i128::from(self.h) - i128::from(h);
        let dv = i128::from(self.v) - i128::from(v);
        (dh * dh + dv * dv) as u128
    }
}

/// Immutable source-navigation index carried by one coherent engine frame.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SyncIndex {
    records: Vec<SyncRect>,
}

impl SyncIndex {
    /// All indexed source rectangles in shipout order.
    pub fn records(&self) -> &[SyncRect] {
        &self.records
    }

    /// Preserve records for pages beyond a partial foreground coverage.
    pub fn with_preserved_tail(mut self, previous: &Self, first_stale_page: usize) -> Self {
        self.records.extend(
            previous
                .records
                .iter()
                .filter(|record| record.page_index >= first_stale_page)
                .cloned(),
        );
        self
    }

    /// Find the closest indexed source line, preferring exact line matches.
    pub fn forward(&self, path: &str, line: usize) -> Option<&SyncRect> {
        self.records
            .iter()
            .filter(|record| same_tex_path(&record.path, path))
            .min_by_key(|record| (record.line.abs_diff(line), record.page_index))
    }

    /// Find the innermost box containing a page point, then the nearest anchor.
    pub fn backward(&self, page_index: usize, h: i32, v: i32) -> Option<&SyncRect> {
        let page = self
            .records
            .iter()
            .filter(|record| record.page_index == page_index)
            .collect::<Vec<_>>();
        page.iter()
            .copied()
            .filter(|record| record.contains(h, v))
            .min_by_key(|record| record.area())
            .or_else(|| {
                page.into_iter()
                    .min_by_key(|record| record.distance_squared(h, v))
            })
    }
}

fn same_tex_path(left: &str, right: &str) -> bool {
    fn normalized(path: &str) -> String {
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
    let left = normalized(left);
    let right = normalized(right);
    left == right || left.ends_with(&format!("/{right}")) || right.ends_with(&format!("/{left}"))
}

#[derive(Clone, Default)]
pub(crate) struct PreviewSyncIndex {
    names: BTreeMap<i32, String>,
    page_index: Option<usize>,
    records: Vec<TaggedSyncRect>,
}

#[derive(Clone)]
struct TaggedSyncRect {
    page_index: usize,
    tag: i32,
    line: usize,
    h: i32,
    v: i32,
    width: i32,
    height: i32,
    depth: i32,
}

impl PreviewSyncIndex {
    pub(crate) fn register_name(&mut self, tag: i32, path: String) {
        if tag > 0 {
            self.names.insert(tag, path);
        }
    }

    pub(crate) fn begin_page(&mut self, page_index: usize) {
        self.page_index = Some(page_index);
    }

    pub(crate) fn end_page(&mut self) {
        self.page_index = None;
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record(
        &mut self,
        tag: i32,
        line: i32,
        h: i32,
        v: i32,
        width: i32,
        height: i32,
        depth: i32,
    ) {
        let (Some(page_index), Ok(line)) = (self.page_index, usize::try_from(line)) else {
            return;
        };
        if tag <= 0 || line == 0 {
            return;
        }
        self.records.push(TaggedSyncRect {
            page_index,
            tag,
            line,
            h,
            v,
            width,
            height,
            depth,
        });
    }

    pub(crate) fn snapshot(&self) -> SyncIndex {
        SyncIndex {
            records: self
                .records
                .iter()
                .filter_map(|record| {
                    Some(SyncRect {
                        page_index: record.page_index,
                        path: self.names.get(&record.tag)?.clone(),
                        line: record.line,
                        h: record.h,
                        v: record.v,
                        width: record.width,
                        height: record.height,
                        depth: record.depth,
                    })
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index() -> SyncIndex {
        SyncIndex {
            records: vec![
                SyncRect {
                    page_index: 0,
                    path: "chapters/one.tex".into(),
                    line: 4,
                    h: 100,
                    v: 200,
                    width: 80,
                    height: 20,
                    depth: 5,
                },
                SyncRect {
                    page_index: 0,
                    path: "chapters/one.tex".into(),
                    line: 5,
                    h: 120,
                    v: 200,
                    width: 20,
                    height: 10,
                    depth: 2,
                },
            ],
        }
    }

    #[test]
    fn forward_normalizes_implicit_tex_name() {
        assert_eq!(index().forward("chapters/one", 5).unwrap().line, 5);
    }

    #[test]
    fn backward_prefers_innermost_containing_box() {
        assert_eq!(index().backward(0, 125, 195).unwrap().line, 5);
    }
}
