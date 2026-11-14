//! The node tree `synctex_parser.c` builds for one sheet.
//!
//! This mirrors the parser's post-processing: box boundary nodes, the visible
//! extent of hboxes, mean lines, and the line fixes of leading boundaries and
//! of a box's trailing kern and glue.

use super::{Record, RecordKind};

pub(super) const NIL: u32 = u32::MAX;

/// The number of friend lists of the parser's display index.
pub(super) const FRIEND_LISTS: i32 = 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Kind {
    Sheet,
    VBox,
    HBox,
    VoidVBox,
    VoidHBox,
    Kern,
    Glue,
    Rule,
    Math,
    Boundary,
    BoxBdry,
}

impl Kind {
    pub(super) fn is_box(self) -> bool {
        matches!(
            self,
            Self::VBox | Self::HBox | Self::VoidVBox | Self::VoidHBox
        )
    }
}

#[derive(Clone, Debug)]
pub(super) struct Node {
    pub kind: Kind,
    pub tag: i32,
    pub line: i32,
    pub h: i32,
    pub v: i32,
    pub width: i32,
    pub height: i32,
    pub depth: i32,
    /// Visible extent of an hbox, grown to its contents.
    pub visible: Visible,
    pub mean_line: i32,
    pub weight: i32,
    pub parent: u32,
    pub child: u32,
    pub sibling: u32,
    last: u32,
    /// Friend list and registration order, when registered for display queries.
    pub friend: Option<(i32, u32)>,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Visible {
    pub h: i32,
    pub v: i32,
    pub width: i32,
    pub height: i32,
    pub depth: i32,
}

impl Node {
    fn new(kind: Kind, record: &Record, parent: u32) -> Self {
        let has_dims = !matches!(
            kind,
            Kind::Glue | Kind::Math | Kind::Boundary | Kind::BoxBdry
        );
        let has_box = has_dims && kind != Kind::Kern;
        Self {
            kind,
            tag: record.tag,
            line: record.line,
            h: record.h,
            v: record.v,
            width: if has_dims { record.width } else { 0 },
            height: if has_box { record.height } else { 0 },
            depth: if has_box { record.depth } else { 0 },
            visible: Visible {
                h: record.h,
                v: record.v,
                width: record.width,
                height: record.height,
                depth: record.depth,
            },
            mean_line: 0,
            weight: 0,
            parent,
            child: NIL,
            sibling: NIL,
            last: NIL,
            friend: None,
        }
    }

    /// `h, v, width, height, depth`, visible for an hbox.
    pub(super) fn geometry(&self) -> [i32; 5] {
        if self.kind == Kind::HBox {
            let visible = self.visible;
            [
                visible.h,
                visible.v,
                visible.width,
                visible.height,
                visible.depth,
            ]
        } else {
            [self.h, self.v, self.width, self.height, self.depth]
        }
    }
}

/// `min.h, max.h, min.v, max.v`
type Extent = [i32; 4];

pub(super) struct Page {
    nodes: Vec<Node>,
    /// Closed hboxes, most recent first, like the parser's `next_hbox` chain.
    pub hboxes: Vec<u32>,
}

struct Builder {
    page: Page,
    parent: u32,
    /// Leading boundaries waiting for a line, one list per open box.
    pending: Vec<Vec<u32>>,
    last_k: u32,
    last_g: u32,
    friends: u32,
}

impl Page {
    pub(super) fn node(&self, id: u32) -> &Node {
        &self.nodes[id as usize]
    }

    pub(super) fn nodes(&self) -> impl Iterator<Item = (u32, &Node)> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(id, node)| (id as u32, node))
    }

    pub(super) fn children(&self, id: u32) -> impl Iterator<Item = u32> + '_ {
        std::iter::successors(Some(self.node(id).child).filter(|&c| c != NIL), |&c| {
            Some(self.node(c).sibling).filter(|&s| s != NIL)
        })
    }

    pub(super) fn build<'a>(records: impl Iterator<Item = &'a Record>) -> Self {
        let sheet = Record::empty(0, RecordKind::VBox);
        let mut builder = Builder {
            page: Page {
                nodes: vec![Node::new(Kind::Sheet, &sheet, NIL)],
                hboxes: Vec::new(),
            },
            parent: 0,
            pending: vec![Vec::new()],
            last_k: NIL,
            last_g: NIL,
            friends: 0,
        };
        for record in records {
            builder.push(record);
        }
        builder.page.hboxes.reverse();
        builder.page
    }
}

impl Builder {
    fn node(&mut self, id: u32) -> &mut Node {
        &mut self.page.nodes[id as usize]
    }

    fn append(&mut self, kind: Kind, record: &Record) -> u32 {
        let id = self.page.nodes.len() as u32;
        let parent = self.parent;
        self.page.nodes.push(Node::new(kind, record, parent));
        let last = self.node(parent).last;
        if last == NIL {
            self.node(parent).child = id;
        } else {
            self.node(last).sibling = id;
        }
        self.node(parent).last = id;
        id
    }

    fn make_friend(&mut self, id: u32) {
        let node = self.node(id);
        let key = node.tag.wrapping_add(node.line);
        if key >= 0 {
            let order = self.friends;
            self.node(id).friend = Some((key % FRIEND_LISTS, order));
            self.friends += 1;
        }
    }

    fn copy_tlc(&mut self, to: u32, from: u32) {
        let (tag, line) = (self.node(from).tag, self.node(from).line);
        let node = self.node(to);
        node.tag = tag;
        node.line = line;
    }

    /// `_synctex_handle_set_tlc`: pending boundaries take the line of `child`.
    fn settle_pending(&mut self, child: u32) {
        let pending = std::mem::take(self.pending.last_mut().expect("open level"));
        for &x in pending.iter().rev() {
            self.copy_tlc(x, child);
            self.make_friend(x);
        }
    }

    fn parent_is_hbox(&self) -> bool {
        self.page.nodes[self.parent as usize].kind == Kind::HBox
    }

    fn push(&mut self, record: &Record) {
        match record.kind {
            RecordKind::VBox | RecordKind::HBox => self.open(record),
            RecordKind::VBoxEnd => {
                if self.node(self.parent).kind == Kind::VBox {
                    let vbox = self.parent;
                    if self.node(vbox).child == NIL {
                        self.make_friend(vbox);
                    }
                    self.close();
                }
            }
            RecordKind::HBoxEnd => {
                if self.node(self.parent).kind == Kind::HBox {
                    self.close_hbox();
                }
            }
            RecordKind::VoidVBox => {
                let id = self.append(Kind::VoidVBox, record);
                self.settle_pending(id);
                self.reset_trailing();
            }
            RecordKind::VoidHBox => {
                let id = self.append(Kind::VoidHBox, record);
                self.settle_pending(id);
                let extent = self.data_extent(id);
                self.contain(extent);
                self.reset_trailing();
            }
            RecordKind::Kern => {
                let id = self.append(Kind::Kern, record);
                self.make_friend(id);
                self.settle_pending(id);
                let node = self.node(id);
                let (h, v, width) = (node.h, node.v, node.width);
                let (min, max) = if width > 0 {
                    (h.wrapping_sub(width), h)
                } else {
                    (h, h.wrapping_sub(width))
                };
                self.contain([min, max, v, v]);
                self.last_k = id;
                self.last_g = NIL;
            }
            RecordKind::Glue => {
                let id = self.append(Kind::Glue, record);
                self.make_friend(id);
                self.settle_pending(id);
                self.contain_point(id);
                if self.last_k != NIL {
                    self.last_g = id;
                } else {
                    self.reset_trailing();
                }
            }
            RecordKind::Rule => {
                let id = self.append(Kind::Rule, record);
                self.make_friend(id);
                self.settle_pending(id);
                self.reset_trailing();
            }
            RecordKind::Math => {
                let id = self.append(Kind::Math, record);
                self.make_friend(id);
                self.settle_pending(id);
                self.contain_point(id);
                self.reset_trailing();
            }
            RecordKind::Boundary => {
                let previous = self.node(self.parent).last;
                let id = self.append(Kind::Boundary, record);
                let pending = self.pending.last_mut().expect("open level");
                let leading =
                    previous != NIL && self.page.nodes[previous as usize].kind == Kind::BoxBdry;
                if leading || !pending.is_empty() {
                    pending.push(id);
                } else {
                    self.make_friend(id);
                }
                self.contain_point(id);
                self.reset_trailing();
            }
        }
    }

    fn reset_trailing(&mut self) {
        self.last_k = NIL;
        self.last_g = NIL;
    }

    fn open(&mut self, record: &Record) {
        let kind = if record.kind == RecordKind::HBox {
            Kind::HBox
        } else {
            Kind::VBox
        };
        let id = self.append(kind, record);
        self.pending.push(Vec::new());
        self.parent = id;
        if kind == Kind::HBox {
            let bdry = self.append(Kind::BoxBdry, record);
            self.make_friend(bdry);
        }
        self.reset_trailing();
    }

    fn close(&mut self) {
        let closed = self.parent;
        let inner = self.pending.pop().unwrap_or_default();
        for x in inner.into_iter().rev() {
            self.make_friend(x);
        }
        if self.pending.is_empty() {
            self.pending.push(Vec::new());
        }
        self.settle_pending(closed);
        self.parent = self.node(closed).parent;
        self.reset_trailing();
    }

    fn close_hbox(&mut self) {
        let hbox = self.parent;
        self.page.hboxes.push(hbox);
        let first = self.node(hbox).child;
        let second = self.node(first).sibling;
        if second != NIL {
            self.copy_tlc_line(first, second);
            let (mut weight, mut lines) = (0i64, 0i64);
            let mut id = second;
            while id != NIL {
                let node = &self.page.nodes[id as usize];
                if node.kind == Kind::HBox && node.weight != 0 {
                    weight += i64::from(node.weight);
                    lines += i64::from(node.mean_line) * i64::from(node.weight);
                } else {
                    weight += 1;
                    lines += i64::from(if node.kind == Kind::HBox {
                        node.mean_line
                    } else {
                        node.line
                    });
                }
                id = node.sibling;
            }
            let node = self.node(hbox);
            node.mean_line = ((lines + weight / 2) / weight) as i32;
            node.weight = weight as i32;
        } else {
            let node = self.node(hbox);
            node.mean_line = node.line;
            node.weight = 1;
        }
        let last = self.node(hbox).last;
        let visible = self.node(hbox).visible;
        let end = self.append(
            Kind::BoxBdry,
            &Record {
                h: visible.h.wrapping_add(visible.width),
                v: visible.v,
                ..Record::empty(0, RecordKind::Boundary)
            },
        );
        self.copy_tlc(end, last);
        let first = self.node(first);
        first.h = visible.h;
        first.v = visible.v;
        if self.last_k != NIL && self.last_g != NIL {
            let mut id = self.node(hbox).child;
            loop {
                let next = self.node(id).sibling;
                if next == NIL {
                    break;
                }
                if next == self.last_k {
                    let (k, g) = (self.last_k, self.last_g);
                    self.copy_tlc(k, id);
                    self.copy_tlc(g, id);
                    break;
                }
                id = next;
            }
        }
        self.close();
        if self.parent_is_hbox() {
            let extent = visible_extent(&self.page.nodes[hbox as usize].visible);
            self.contain(extent);
        }
    }

    fn copy_tlc_line(&mut self, to: u32, from: u32) {
        let line = self.node(from).line;
        self.node(to).line = line;
    }

    fn data_extent(&mut self, id: u32) -> Extent {
        let node = self.node(id);
        extent(node.h, node.v, node.width, node.height, node.depth)
    }

    fn contain_point(&mut self, id: u32) {
        let node = self.node(id);
        let (h, v) = (node.h, node.v);
        self.contain([h, h, v, v]);
    }

    /// `_synctex_make_hbox_contain_box`, a no-op outside hboxes.
    fn contain(&mut self, extent: Extent) {
        if !self.parent_is_hbox() {
            return;
        }
        let [min_h, max_h, min_v, max_v] = extent;
        let visible = &mut self.page.nodes[self.parent as usize].visible;
        let n = visible.width;
        if n < 0 {
            let max = visible.h;
            let min = max.wrapping_add(n);
            if min_h < min {
                visible.width = min_h.wrapping_sub(max);
            } else if max_h > max {
                visible.h = max_h;
                visible.width = min.wrapping_sub(max_h);
            }
        } else {
            let min = visible.h;
            let max = min.wrapping_add(n);
            if min_h < min {
                visible.h = min_h;
                visible.width = max.wrapping_sub(min_h);
            } else if max_h > max {
                visible.width = max_h.wrapping_sub(min);
            }
        }
        let n = visible.v;
        if min_v < n.wrapping_sub(visible.height) {
            visible.height = n.wrapping_sub(min_v);
        } else if max_v > n.wrapping_add(visible.depth) {
            visible.depth = max_v.wrapping_sub(n);
        }
    }
}

fn extent(h: i32, v: i32, width: i32, height: i32, depth: i32) -> Extent {
    let (min, max) = if width < 0 {
        (h.wrapping_add(width), h)
    } else {
        (h, h.wrapping_add(width))
    };
    [min, max, v.wrapping_sub(height), v.wrapping_add(depth)]
}

fn visible_extent(visible: &Visible) -> Extent {
    extent(
        visible.h,
        visible.v,
        visible.width,
        visible.height,
        visible.depth,
    )
}
