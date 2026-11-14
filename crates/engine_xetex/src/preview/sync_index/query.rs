//! Ports of `synctex_edit_query` and `synctex_display_query`.

use std::collections::BTreeMap;

use super::{
    SyncIndex,
    tree::{FRIEND_LISTS, Kind, NIL, Node, Page},
};

const FAR: i32 = i32::MAX;

/// A node and its distance to the hit point.
#[derive(Clone, Copy)]
struct Nd {
    node: u32,
    distance: i32,
}

const ND_0: Nd = Nd {
    node: NIL,
    distance: FAR,
};

#[derive(Clone, Copy)]
struct Hit {
    h: i32,
    v: i32,
}

/// Region distance to `min.h, max.h, min.v, max.v`, zero inside.
fn distance_to_box(hit: Hit, [min_h, max_h, min_v, max_v]: [i32; 4]) -> i32 {
    let dv = if hit.v < min_v {
        min_v.wrapping_sub(hit.v)
    } else if hit.v <= max_v {
        0
    } else {
        hit.v.wrapping_sub(max_v)
    };
    let dh = if hit.h < min_h {
        min_h.wrapping_sub(hit.h)
    } else if hit.h <= max_h {
        0
    } else {
        hit.h.wrapping_sub(max_h)
    };
    dv.wrapping_add(dh)
}

fn ordered(value: i32, min: i32, max: i32) -> i32 {
    if value < min {
        min.wrapping_sub(value)
    } else if value > max {
        max.wrapping_sub(value)
    } else {
        0
    }
}

impl Page {
    fn parent_of(&self, node: &Node) -> Option<&Node> {
        (node.parent != NIL).then(|| self.node(node.parent))
    }

    /// `_synctex_point_h_ordered_distance_v2`: positive when the node is right of the hit.
    fn h_ordered(&self, hit: Hit, id: u32) -> i32 {
        let node = self.node(id);
        match node.kind {
            Kind::VBox | Kind::VoidVBox | Kind::VoidHBox => {
                let min = node.h;
                ordered(hit.h, min, min.wrapping_add(node.width.wrapping_abs()))
            }
            Kind::HBox => {
                let min = node.visible.h;
                ordered(
                    hit.h,
                    min,
                    min.wrapping_add(node.visible.width.wrapping_abs()),
                )
            }
            Kind::Kern => {
                // The kern is recorded after the move, and loses ties to other nodes.
                let (min, max) = if node.width < 0 {
                    (node.h, node.h.wrapping_sub(node.width))
                } else {
                    (node.h.wrapping_sub(node.width), node.h)
                };
                let med = min.wrapping_add(max) / 2;
                if hit.h < min {
                    min.wrapping_sub(hit.h).wrapping_add(1)
                } else if hit.h > max {
                    max.wrapping_sub(hit.h).wrapping_sub(1)
                } else if hit.h > med {
                    max.wrapping_sub(hit.h).wrapping_add(1)
                } else {
                    min.wrapping_sub(hit.h).wrapping_sub(1)
                }
            }
            Kind::Rule | Kind::Glue | Kind::Math | Kind::Boundary | Kind::BoxBdry => {
                node.h.wrapping_sub(hit.h)
            }
            Kind::Sheet => FAR,
        }
    }

    /// `_synctex_point_v_ordered_distance_v2`: positive when the node is above the hit.
    fn v_ordered(&self, hit: Hit, id: u32) -> i32 {
        let node = self.node(id);
        match node.kind {
            Kind::VBox | Kind::VoidVBox | Kind::VoidHBox => ordered(
                hit.v,
                node.v.wrapping_sub(node.height.wrapping_abs()),
                node.v.wrapping_add(node.depth.wrapping_abs()),
            ),
            Kind::HBox => ordered(
                hit.v,
                node.visible
                    .v
                    .wrapping_sub(node.visible.height.wrapping_abs()),
                node.visible
                    .v
                    .wrapping_add(node.visible.depth.wrapping_abs()),
            ),
            Kind::Rule | Kind::Kern | Kind::Glue | Kind::Math => {
                let (height, depth) = self
                    .parent_of(node)
                    .map_or((0, 0), |parent| (parent.height, parent.depth));
                ordered(
                    hit.v,
                    node.v.wrapping_sub(height.wrapping_abs()),
                    node.v.wrapping_add(depth.wrapping_abs()),
                )
            }
            Kind::Boundary | Kind::BoxBdry | Kind::Sheet => FAR,
        }
    }

    fn contains(&self, hit: Hit, id: u32) -> bool {
        self.h_ordered(hit, id) == 0 && self.v_ordered(hit, id) == 0
    }

    /// `_synctex_point_node_distance_v2`
    fn distance(&self, hit: Hit, id: u32) -> i32 {
        let node = self.node(id);
        let parent_height = || {
            self.parent_of(node)
                .map_or(0, |parent| parent.height.wrapping_abs())
        };
        match node.kind {
            Kind::VBox => distance_to_box(
                hit,
                [
                    node.h,
                    node.h.wrapping_add(node.width.wrapping_abs()),
                    node.v.wrapping_sub(node.height.wrapping_abs()),
                    node.v.wrapping_add(node.depth.wrapping_abs()),
                ],
            ),
            Kind::HBox => {
                let visible = node.visible;
                distance_to_box(
                    hit,
                    [
                        visible.h,
                        visible.h.wrapping_add(visible.width.wrapping_abs()),
                        visible.v.wrapping_sub(visible.height.wrapping_abs()),
                        visible.v.wrapping_add(visible.depth.wrapping_abs()),
                    ],
                )
            }
            Kind::VoidVBox | Kind::VoidHBox => {
                let (min_v, max_v) = (
                    node.v.wrapping_sub(node.height.wrapping_abs()),
                    node.v.wrapping_add(node.depth.wrapping_abs()),
                );
                let left = node.h;
                let right = left.wrapping_add(node.width.wrapping_abs());
                distance_to_box(hit, [left, left, min_v, max_v])
                    .min(distance_to_box(hit, [right, right, min_v, max_v]))
            }
            Kind::Kern => {
                let min_v = node.v.wrapping_sub(parent_height());
                let other = node.h.wrapping_sub(node.width);
                distance_to_box(hit, [node.h, node.h, min_v, node.v])
                    .min(distance_to_box(hit, [other, other, min_v, node.v]))
            }
            Kind::Glue | Kind::Math | Kind::Boundary | Kind::BoxBdry => {
                let min_v = node.v.wrapping_sub(parent_height());
                distance_to_box(hit, [node.h, node.h, min_v, node.v])
            }
            Kind::Rule | Kind::Sheet => FAR,
        }
    }

    fn has_children(&self, id: u32) -> bool {
        self.node(id).child != NIL
    }

    /// `_synctex_eq_deepest_container_v2`
    fn deepest_container_v2(&self, hit: Hit, id: u32) -> Option<u32> {
        if !self.has_children(id) {
            return None;
        }
        for child in self.children(id) {
            if self.contains(hit, child)
                && let Some(deep) = self.deepest_container_v2(hit, child)
            {
                return Some(deep);
            }
        }
        if self.node(id).kind == Kind::VBox {
            let mut best = ND_0;
            for child in self.children(id) {
                if self.has_children(child) {
                    let d = self.distance(hit, child);
                    if d <= best.distance {
                        best = Nd {
                            node: child,
                            distance: d,
                        };
                    }
                }
            }
            if best.node != NIL {
                return Some(best.node);
            }
        }
        self.contains(hit, id).then_some(id)
    }

    /// `_synctex_eq_deepest_container_v3`
    fn deepest_container_v3(&self, hit: Hit, id: u32) -> Nd {
        if !self.has_children(id) {
            return ND_0;
        }
        for child in self.children(id) {
            let deep = self.deepest_container_v3(hit, child);
            if deep.node != NIL {
                return deep;
            }
        }
        if self.node(id).kind == Kind::VBox {
            let mut best = ND_0;
            for child in self.children(id) {
                if self.has_children(child) {
                    let d = self.distance(hit, child);
                    if d < best.distance {
                        best = Nd {
                            node: child,
                            distance: d,
                        };
                    }
                }
            }
            if best.node != NIL {
                return best;
            }
        }
        if self.contains(hit, id) {
            Nd {
                node: id,
                distance: 0,
            }
        } else {
            ND_0
        }
    }

    /// `__synctex_closest_deep_child_v2`
    fn closest_deep_child(&self, hit: Hit, id: u32) -> Nd {
        let mut best = ND_0;
        for child in self.children(id) {
            let nd = if self.node(child).kind.is_box() {
                self.closest_deep_child(hit, child)
            } else {
                Nd {
                    node: child,
                    distance: self.distance(hit, child),
                }
            };
            let is_kern = nd.node != NIL && self.node(nd.node).kind == Kind::Kern;
            if nd.distance < best.distance || (nd.distance == best.distance && !is_kern) {
                best = nd;
            }
        }
        best
    }

    /// Whether `candidate` beats `best` at an equal distance.
    fn earlier_line(&self, best: u32, candidate: u32) -> bool {
        let (best, candidate) = (self.node(best), self.node(candidate));
        best.tag == candidate.tag && best.line > candidate.line
    }

    /// `_synctex_eq_get_closest_children_in_box_v2`
    fn closest_children(&self, hit: Hit, id: u32) -> (Nd, Nd) {
        if !self.has_children(id) || self.node(id).kind != Kind::HBox {
            // The parser's vbox variant never finds anything.
            return (ND_0, ND_0);
        }
        let (mut left, mut right) = (ND_0, ND_0);
        for child in self.children(id) {
            let distance = self.h_ordered(hit, child);
            if distance > 0 {
                if right.distance > distance {
                    right = Nd {
                        node: child,
                        distance,
                    };
                } else if right.distance == distance
                    && right.node != NIL
                    && self.earlier_line(right.node, child)
                {
                    right.node = child;
                }
            } else if distance == 0 {
                if self.has_children(child) {
                    return self.closest_children(hit, child);
                }
                left = Nd {
                    node: child,
                    distance: 0,
                };
            } else {
                let distance = distance.wrapping_neg();
                if left.distance > distance {
                    left = Nd {
                        node: child,
                        distance,
                    };
                } else if left.distance == distance
                    && left.node != NIL
                    && self.earlier_line(left.node, child)
                {
                    left.node = child;
                }
            }
        }
        for side in [&mut left, &mut right] {
            if side.node != NIL {
                let deep = self.deepest_container_v3(hit, side.node);
                if deep.node != NIL {
                    *side = deep;
                }
                let deep = self.closest_deep_child(hit, side.node);
                if deep.node != NIL {
                    side.node = deep.node;
                }
            }
        }
        (left, right)
    }

    /// `_synctex_smallest_container_v2`
    fn smallest_container(&self, node: u32, other: u32) -> u32 {
        let (a, b) = (self.node(node), self.node(other));
        let total = |n: &Node| {
            i64::from(n.visible.depth.wrapping_abs()) + i64::from(n.visible.height.wrapping_abs())
        };
        let area = |n: &Node| i128::from(total(n)) * i128::from(n.visible.width.wrapping_abs());
        if area(a) != area(b) {
            return if area(a) < area(b) { node } else { other };
        }
        let (wa, wb) = (a.width.wrapping_abs(), b.width.wrapping_abs());
        if wa != wb {
            return if wa > wb { node } else { other };
        }
        if total(a) > total(b) { other } else { node }
    }
}

/// `synctex_iterator_new_edit`: the first result, the one Overleaf follows.
pub(super) fn edit(page: &Page, h: i32, v: i32) -> Option<u32> {
    let hit = Hit { h, v };
    let mut container = None;
    for (i, &hbox) in page.hboxes.iter().enumerate() {
        if page.contains(hit, hbox) {
            let mut node = hbox;
            for &next in &page.hboxes[i + 1..] {
                if page.contains(hit, next) {
                    node = page.smallest_container(next, node);
                }
            }
            container = Some(node);
            break;
        }
    }
    let (node, mut left, right) = match container {
        Some(node) => {
            let node = page.deepest_container_v2(hit, node).unwrap_or(node);
            let (left, right) = page.closest_children(hit, node);
            (node, left, right)
        }
        None => {
            let node = page.node(0).child;
            if node == NIL {
                return None;
            }
            (node, page.closest_deep_child(hit, node), ND_0)
        }
    };
    if left.node != NIL && right.node != NIL {
        let (l, r) = (page.node(left.node), page.node(right.node));
        if l.tag != r.tag || l.line != r.line {
            let right_first =
                r.line < l.line || (r.line == l.line && left.distance > right.distance);
            return Some(if right_first { right.node } else { left.node });
        }
        if left.distance > right.distance {
            left = right;
        }
    } else if right.node != NIL {
        left = right;
    } else if left.node == NIL {
        left.node = node;
    }
    Some(left.node)
}

/// One `synctex view` result: the box to highlight for a matching node.
pub(super) struct DisplayHit {
    pub page_index: usize,
    pub tag: i32,
    pub line: i32,
    pub geometry: [i32; 5],
}

/// The sheets a display query can match on, built on demand.
struct Sheets<'a> {
    index: &'a SyncIndex,
    built: BTreeMap<usize, Page>,
}

impl Sheets<'_> {
    fn page(&mut self, page_index: usize) -> &Page {
        let index = self.index;
        self.built
            .entry(page_index)
            .or_insert_with(|| super::tree::Page::build(index.page_records(page_index)))
    }
}

/// A matching friend node, identified by sheet and node.
#[derive(Clone, Copy)]
struct Match {
    page_index: usize,
    node: u32,
}

/// `synctex_iterator_new_display` with no page hint.
pub(super) fn display(index: &SyncIndex, tag: i32, line: i32) -> Vec<DisplayHit> {
    let max_line = index
        .records()
        .filter(|record| record.tag == tag && !record.kind.is_end())
        .map(|record| record.line)
        .max()
        .unwrap_or(0);
    let mut line = line.min(max_line);
    let mut offset = 1i32;
    let mut sheets = Sheets {
        index,
        built: BTreeMap::new(),
    };
    for _ in 0..100 {
        if line > max_line {
            break;
        }
        let hits = display_line(&mut sheets, tag, line);
        if !hits.is_empty() {
            return hits;
        }
        let step = |line: &mut i32, offset: &mut i32| {
            *line += *offset;
            *offset = if *offset < 0 {
                -(*offset - 1)
            } else {
                -(*offset + 1)
            };
        };
        step(&mut line, &mut offset);
        if line <= 0 {
            step(&mut line, &mut offset);
        }
    }
    Vec::new()
}

/// [`display`], except that a line without boxes tries the lines below it first.
///
/// Bodies read ahead as a macro argument, like beamer frames, are typeset
/// under the line that ends them.
pub(super) fn display_ahead(index: &SyncIndex, tag: i32, line: i32) -> Vec<DisplayHit> {
    // The window `display` searches on either side.
    const AHEAD: i32 = 50;
    let mut ahead = index
        .records()
        .filter(|record| {
            record.tag == tag
                && !record.kind.is_end()
                && (line..=line.saturating_add(AHEAD)).contains(&record.line)
        })
        .map(|record| record.line)
        .collect::<Vec<_>>();
    ahead.sort_unstable();
    ahead.dedup();
    let mut sheets = Sheets {
        index,
        built: BTreeMap::new(),
    };
    for next in ahead {
        let hits = display_line(&mut sheets, tag, next);
        if !hits.is_empty() {
            return hits;
        }
    }
    display(index, tag, line)
}

fn display_line(sheets: &mut Sheets, tag: i32, line: i32) -> Vec<DisplayHit> {
    let key = tag.wrapping_add(line);
    if key < 0 {
        return Vec::new();
    }
    let bucket = key % FRIEND_LISTS;
    let pages = {
        let mut pages = sheets
            .index
            .records()
            .filter(|record| record.tag == tag && record.line == line)
            .map(|record| record.page_index)
            .collect::<Vec<_>>();
        pages.dedup();
        pages
    };
    // Friends are walked newest first, across the whole document.
    let mut friends = Vec::new();
    for &page_index in &pages {
        let page = sheets.page(page_index);
        for (id, node) in page.nodes() {
            if let Some((list, order)) = node.friend
                && list == bucket
                && node.tag == tag
                && node.line == line
            {
                friends.push((page_index, order, id, node.kind.is_box()));
            }
        }
    }
    friends.sort_by_key(|friend| std::cmp::Reverse((friend.0, friend.1)));
    let mut matches = friends
        .iter()
        .filter(|friend| !friend.3)
        .map(|&(page_index, _, node, _)| Match { page_index, node })
        .collect::<Vec<_>>();
    if matches.is_empty() {
        matches = friends
            .iter()
            .map(|&(page_index, _, node, _)| Match { page_index, node })
            .collect();
    }
    let Some(groups) = group_by_page(&matches) else {
        return Vec::new();
    };
    let mut ordered: Vec<Vec<Match>> = Vec::new();
    let mut best = usize::MAX;
    for group in groups {
        let group = vertically_sorted(sheets, &group, tag, line);
        let distance = group[0].page_index + 1;
        if ordered.is_empty() {
            best = distance;
            ordered.push(group);
        } else if distance < best {
            best = distance;
            ordered.insert(0, group);
        } else {
            ordered.insert(1, group);
        }
    }
    ordered
        .into_iter()
        .flatten()
        .map(|found| {
            let page = sheets.page(found.page_index);
            let node = page.node(found.node);
            DisplayHit {
                page_index: found.page_index,
                tag: node.tag,
                line: node.line,
                geometry: page.visible_box(found.node),
            }
        })
        .collect()
}

/// `_synctex_display_query_v2`: result chains per page, in the parser's order.
fn group_by_page(matches: &[Match]) -> Option<Vec<Vec<Match>>> {
    let (first, rest) = matches.split_first()?;
    let mut groups = vec![vec![*first]];
    let mut rest = rest.iter();
    for found in rest.by_ref() {
        if found.page_index == first.page_index {
            groups[0].insert(0, *found);
        } else {
            groups.push(vec![*found]);
            break;
        }
    }
    for found in rest {
        match groups
            .iter_mut()
            .find(|group| group[0].page_index == found.page_index)
        {
            Some(group) => group.insert(1, *found),
            None => groups.insert(0, vec![*found]),
        }
    }
    Some(groups)
}

/// `_synctex_vertically_sorted_v2`: one result per hbox, weighted by its matches.
fn vertically_sorted(sheets: &mut Sheets, group: &[Match], tag: i32, line: i32) -> Vec<Match> {
    let page = sheets.page(group[0].page_index);
    let mut hbox_weights = BTreeMap::new();
    let weights = group
        .iter()
        .map(|found| {
            let parent = page.node(found.node).parent;
            let has_weight = page.node(parent).kind == Kind::HBox;
            if has_weight && hbox_weights.contains_key(&parent) {
                return 0;
            }
            let weight = page
                .children(parent)
                .filter(|&child| {
                    let child = page.node(child);
                    child.tag == tag && child.line == line
                })
                .count();
            if has_weight {
                hbox_weights.insert(parent, weight);
            }
            weight
        })
        .collect::<Vec<_>>();
    let mut chain = vec![(group[0], weights[0])];
    for (&found, &weight) in group.iter().zip(&weights).skip(1) {
        if weight > chain[0].1 {
            chain.insert(0, (found, weight));
        } else if weight != 0 {
            let mut at = 0;
            while at + 1 < chain.len() {
                at += 1;
                if weight >= chain[at].1 {
                    break;
                }
            }
            chain.insert(at + 1, (found, weight));
        }
    }
    chain.into_iter().map(|(found, _)| found).collect()
}

impl Page {
    /// `_synctex_node_box_visible` as `h, v, width, height, depth`.
    fn visible_box(&self, id: u32) -> [i32; 5] {
        const BOUND: i32 = 1_500_000;
        let mut node = id;
        if !self.node(node).kind.is_box() {
            node = self.node(node).parent;
        }
        let mean = self.mean_line(node);
        let mut parent = self.node(node).parent;
        while parent != NIL {
            let p = self.node(parent);
            if matches!(p.kind, Kind::HBox | Kind::VoidHBox) {
                if mean.wrapping_sub(self.mean_line(parent)).wrapping_abs() > 1 {
                    break;
                } else if p.width > BOUND || p.height.wrapping_add(p.depth) > BOUND {
                    node = parent;
                    break;
                }
                node = parent;
            }
            parent = p.parent;
        }
        let node = self.node(node);
        if node.kind == Kind::Sheet {
            [0; 5]
        } else {
            node.geometry()
        }
    }

    /// `synctex_node_mean_line`
    fn mean_line(&self, id: u32) -> i32 {
        let node = self.node(id);
        if node.kind == Kind::HBox {
            return node.mean_line;
        }
        match self.parent_of(node) {
            Some(parent) if parent.kind == Kind::HBox => parent.mean_line,
            _ => node.line,
        }
    }
}
