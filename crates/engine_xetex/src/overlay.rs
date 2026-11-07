//! Typed views of fixed-size nodes, read and written as `mem[p].field`.
//!
//! A layout struct lies over a node's words in the flat `mem` arena. Indexing
//! `mem` with a [`NodePtr`] borrows the node as its layout, and a mutable
//! borrow first notes the node's words in the journal.

use bytemuck::{Pod, Zeroable};

use crate::codes::{FiOrElseCode, ValueLevel};
use crate::node::{
    AdjustNode, AlignRecord, AlignStackNode, BoxNode, CharNode, ChoiceNode, CondNode, DeltaNode,
    DiscNode, EdgeNode, ExprNode, FixedNode, FreeBlock, GlueNode, GlueSpecNode, InsNode, KernNode,
    LigItem, LigatureNode, MarginKernNode, MarkNode, MathNode, MovementNode, NodeKind, NodePtr,
    PageInsNode, PassiveNode, PenaltyNode, RuleNode, SpanNode, Stored, StoredDimen, StyleNode,
    UnsetNode,
};
use crate::state::MemState;
use crate::token::TokenList;
use crate::units::{FontId, GlueOrder, GlueSign, MemPtr, Orientation, Scaled};

#[cfg(not(target_endian = "little"))]
compile_error!("node layouts assume little-endian memory words");

/// A node kind whose words read as one `#[repr(C)]` layout struct.
pub trait Overlay: NodeKind {
    /// The struct laid over the node's `WORDS` words.
    type Layout: Pod;
    const WORDS: i32;
}

impl<K: Overlay> std::ops::Index<NodePtr<K>> for MemState {
    type Output = K::Layout;

    #[inline]
    fn index(&self, p: NodePtr<K>) -> &K::Layout {
        bytemuck::from_bytes(bytemuck::cast_slice(&self.data[p.idx().words(K::WORDS)]))
    }
}

impl<K: Overlay> std::ops::IndexMut<NodePtr<K>> for MemState {
    #[inline]
    fn index_mut(&mut self, p: NodePtr<K>) -> &mut K::Layout {
        let words = p.idx().words(K::WORDS);
        let start = words.start;
        let node = &mut self.data[words];
        if self.journal.armed() {
            self.journal.note_run(start, node);
        }
        bytemuck::from_bytes_mut(bytemuck::cast_slice_mut(node))
    }
}

/// The most words a layout covers. `mem` keeps all but one of them as slack
/// past `MEM_TOP`, so a layout read at a list head near the top stays in bounds.
pub(crate) const MAX_LAYOUT_WORDS: i32 = 8;

/// Declares the layout of a node kind and checks it covers the node exactly.
/// The word count defaults to the kind's allocation size.
macro_rules! overlay {
    ($kind:ty => $layout:ty) => {
        overlay!($kind => $layout, <$kind as FixedNode>::SIZE);
    };
    ($kind:ty => $layout:ty, $words:expr) => {
        impl Overlay for $kind {
            type Layout = $layout;
            const WORDS: i32 = $words;
        }
        const _: () = assert!(<$kind as Overlay>::WORDS <= MAX_LAYOUT_WORDS);
        const _: () = assert!(
            std::mem::size_of::<$layout>()
                == std::mem::size_of::<crate::mem_types::MemoryWord>()
                    * <$kind as Overlay>::WORDS as usize
        );
    };
}

/// A glue specification. Its first word holds the fill orders and a reference
/// count in place of a type code and link.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct GlueSpec {
    pub shrink_order: Stored<GlueOrder>,
    pub stretch_order: Stored<GlueOrder>,
    /// The reference count, `TEX_NULL` for a single reference.
    pub ref_count: i32,
    _unused1: i32,
    pub width: Scaled,
    _unused2: i32,
    pub stretch: Scaled,
    _unused3: i32,
    pub shrink: Scaled,
}

overlay!(GlueSpecNode => GlueSpec);

/// An hlist or vlist box. Its orientation is the node's type code.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct BoxLayout {
    pub subtype: u16,
    pub orientation: Stored<Orientation>,
    pub link: MemPtr,
    _unused1: i32,
    pub width: Scaled,
    _unused2: i32,
    pub depth: Scaled,
    _unused3: i32,
    pub height: Scaled,
    _unused4: i32,
    pub shift_amount: Scaled,
    pub glue_order: Stored<GlueOrder>,
    pub glue_sign: Stored<GlueSign>,
    pub list_ptr: MemPtr,
    pub glue_set: f64,
    /// The SyncTeX tag and line that `get_node` stamps into the last word.
    pub sync_tag: i32,
    pub sync_line: i32,
}

overlay!(BoxNode => BoxLayout);

/// A kern. Its subtype tells normal, explicit, accent and math kerns apart.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct KernLayout {
    pub subtype: u16,
    _type: u16,
    pub link: MemPtr,
    _unused1: i32,
    pub width: Scaled,
    pub sync_tag: i32,
    pub sync_line: i32,
}

overlay!(KernNode => KernLayout);

/// A math-on/off node, or a TeX--XeT LR boundary, by its subtype.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct MathLayout {
    pub subtype: u16,
    _type: u16,
    pub link: MemPtr,
    _unused1: i32,
    pub width: Scaled,
    pub sync_tag: i32,
    pub sync_line: i32,
}

overlay!(MathNode => MathLayout);

/// A margin kern. Its subtype is the side it protrudes on.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct MarginKernLayout {
    pub subtype: u16,
    _type: u16,
    pub link: MemPtr,
    _unused1: i32,
    pub width: Scaled,
    _margin_char: i32,
    _unused2: i32,
}

overlay!(MarginKernNode => MarginKernLayout);

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PenaltyLayout {
    pub subtype: u16,
    _type: u16,
    pub link: MemPtr,
    _unused1: i32,
    pub penalty: i32,
    _unused2: [i32; 2],
}

overlay!(PenaltyNode => PenaltyLayout);

/// A `\vadjust` list, the subtype selects pre or post placement.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct AdjustLayout {
    pub subtype: u16,
    _type: u16,
    pub link: MemPtr,
    _unused1: i32,
    pub adjust_ptr: MemPtr,
}

overlay!(AdjustNode => AdjustLayout);

/// A discretionary. Its subtype is the replace count.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct DiscLayout {
    pub subtype: u16,
    _type: u16,
    pub link: MemPtr,
    pub pre_break: MemPtr,
    pub post_break: MemPtr,
}

overlay!(DiscNode => DiscLayout);

/// A `\mathchoice` node with one mlist per style.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ChoiceLayout {
    pub subtype: u16,
    _type: u16,
    pub link: MemPtr,
    pub display_mlist: MemPtr,
    pub text_mlist: MemPtr,
    pub script_mlist: MemPtr,
    pub script_script_mlist: MemPtr,
}

overlay!(ChoiceNode => ChoiceLayout);

/// A style change in an mlist. Its subtype is the style.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct StyleLayout {
    pub subtype: u16,
    _type: u16,
    pub link: MemPtr,
    _unused1: i32,
    payload1: i32,
    _unused2: i32,
    payload2: i32,
}

overlay!(StyleNode => StyleLayout);

impl StyleLayout {
    /// Zeroes the two payload words, as `new_style` does.
    #[inline]
    pub fn clear_payload(&mut self) {
        self.payload1 = 0;
        self.payload2 = 0;
    }
}

/// An LR edge in a shipped list. Its subtype is the direction.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct EdgeLayout {
    pub subtype: u16,
    _type: u16,
    pub link: MemPtr,
    _unused1: i32,
    pub width: Scaled,
    _unused2: i32,
    pub edge_dist: Scaled,
}

overlay!(EdgeNode => EdgeLayout);

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct MarkLayout {
    pub subtype: u16,
    _type: u16,
    pub link: MemPtr,
    pub mark_class: i32,
    pub mark_ptr: TokenList,
}

overlay!(MarkNode => MarkLayout);

/// A line-breaker delta node: the change in the totals between active nodes.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct DeltaLayout {
    pub subtype: u16,
    _type: u16,
    pub link: MemPtr,
    _unused1: i32,
    pub dwidth: Scaled,
    _unused2: i32,
    pub dstretch0: Scaled,
    _unused3: i32,
    pub dstretch1: Scaled,
    _unused4: i32,
    pub dstretch2: Scaled,
    _unused5: i32,
    pub dstretch3: Scaled,
    _unused6: i32,
    pub dshrink: Scaled,
}

overlay!(DeltaNode => DeltaLayout);

/// A line-breaker passive node, one feasible break.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PassiveLayout {
    /// A serial number, for tracing.
    pub serial: i32,
    pub link: MemPtr,
    /// WEB's `prev_break` until `post_line_break` reverses the chain.
    pub next_break: MemPtr,
    pub cur_break: MemPtr,
}

overlay!(PassiveNode => PassiveLayout);

/// An entry of the conditional stack, saving the enclosing `\if`.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct CondLayout {
    pub cur_if: u16,
    pub if_limit: Stored<FiOrElseCode>,
    pub link: MemPtr,
    _unused1: i32,
    pub if_line: i32,
}

overlay!(CondNode => CondLayout);

impl CondLayout {
    /// The saved `(cur_if, if_line, if_limit)`.
    #[inline]
    pub fn saved_state(&self) -> (i16, i32, Stored<FiOrElseCode>) {
        (self.cur_if as i16, self.if_line, self.if_limit)
    }
}

/// A shipout `down_ptr`/`right_ptr` movement record.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct MovementLayout {
    /// Whether the movement may be reused, and as which command.
    pub state: i32,
    pub link: MemPtr,
    _unused1: i32,
    pub distance: Scaled,
    _unused2: i32,
    /// The DVI offset where the movement was emitted.
    pub location: i32,
}

overlay!(MovementNode => MovementLayout);

/// An entry of a column's span list, ended by `END_SPAN`.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SpanLayout {
    pub next: NodePtr<SpanNode>,
    /// Columns spanned beyond the first. `END_SPAN` holds a larger sentinel.
    pub span_count: i32,
    _unused1: i32,
    pub width: Scaled,
}

overlay!(SpanNode => SpanLayout);

/// An alignment preamble column: a null box holding the column's templates,
/// natural width and span list.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct AlignRecordLayout {
    /// The head of the span list, read through `span_head`.
    _span_list: i32,
    pub link: MemPtr,
    _unused1: i32,
    pub width: Scaled,
    _unused2: i32,
    pub v_part: TokenList,
    _unused3: i32,
    pub u_part: TokenList,
    _unused4: [i32; 2],
    /// The command that ended the current entry of this column.
    pub extra_info: i32,
    _unused5: [i32; 5],
}

overlay!(AlignRecord => AlignRecordLayout);

/// A saved outer alignment on the `align_ptr` stack.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct AlignStackLayout {
    pub cur_align: MemPtr,
    pub link: MemPtr,
    pub preamble: MemPtr,
    pub cur_span: MemPtr,
    _unused1: i32,
    pub cur_loop: MemPtr,
    _unused2: i32,
    pub align_state: i32,
    pub cur_head: MemPtr,
    pub cur_tail: MemPtr,
    pub cur_pre_head: MemPtr,
    pub cur_pre_tail: MemPtr,
}

overlay!(AlignStackNode => AlignStackLayout);

/// A suspended level of `\numexpr`-style expression scanning.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ExprLayout {
    pub state: u16,
    pub level: Stored<ValueLevel>,
    pub link: MemPtr,
    _unused1: i32,
    pub e: i32,
    _unused2: i32,
    pub t: i32,
    _unused3: i32,
    pub n: i32,
}

overlay!(ExprNode => ExprLayout);

/// An entry of the ligature reconstitution stack.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct LigItemLayout {
    pub character: u16,
    _unused0: u16,
    pub link: MemPtr,
    _unused1: i32,
    pub component: MemPtr,
}

overlay!(LigItem => LigItemLayout);

/// A rule. A running dimension holds `NULL_FLAG`.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct RuleLayout {
    pub subtype: u16,
    _type: u16,
    pub link: MemPtr,
    _unused1: i32,
    pub width: StoredDimen,
    _unused2: i32,
    pub depth: StoredDimen,
    _unused3: i32,
    pub height: StoredDimen,
    pub sync_tag: i32,
    pub sync_line: i32,
}

overlay!(RuleNode => RuleLayout);

/// A glue node. Its subtype is a glue parameter, a leader kind or a math code.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct GlueLayout {
    pub subtype: u16,
    _type: u16,
    pub link: MemPtr,
    pub glue_ptr: NodePtr<GlueSpecNode>,
    /// The leader box or rule, null for plain glue.
    pub leader_ptr: MemPtr,
    pub sync_tag: i32,
    pub sync_line: i32,
}

overlay!(GlueNode => GlueLayout);

/// An insertion. Its subtype is the box number.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct InsLayout {
    pub subtype: u16,
    _type: u16,
    pub link: MemPtr,
    _unused1: i32,
    pub float_cost: i32,
    _unused2: i32,
    pub depth: Scaled,
    _unused3: i32,
    pub height: Scaled,
    pub ins_ptr: MemPtr,
    /// The `\splittopskip` glue spec.
    pub split_top_ptr: NodePtr<GlueSpecNode>,
}

overlay!(InsNode => InsLayout);

/// A record on the page builder's insertion ring. Its type code is
/// `inserting` or `split_up`, its subtype the insertion's box number.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PageInsLayout {
    pub subtype: u16,
    split_up: u16,
    pub link: MemPtr,
    pub broken_ins: MemPtr,
    pub broken_ptr: MemPtr,
    pub best_ins_ptr: MemPtr,
    pub last_ins_ptr: MemPtr,
    _unused3: i32,
    pub height: Scaled,
}

overlay!(PageInsNode => PageInsLayout);

impl PageInsLayout {
    #[inline]
    pub fn is_split_up(&self) -> bool {
        self.split_up == 1
    }

    #[inline]
    pub fn set_split_up(&mut self, split: bool) {
        self.split_up = u16::from(split);
    }
}

/// An alignment box before its column widths are known. It keeps the box
/// layout but holds shrink and stretch in place of shift and glue setting.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct UnsetLayout {
    /// The number of columns spanned, less one.
    pub subtype: u16,
    _type: u16,
    pub link: MemPtr,
    _unused1: i32,
    pub width: Scaled,
    _unused2: i32,
    pub depth: Scaled,
    _unused3: i32,
    pub height: Scaled,
    _unused4: i32,
    pub glue_shrink: Scaled,
    /// The stretch order.
    pub glue_order: Stored<GlueOrder>,
    /// The shrink order.
    pub glue_sign: Stored<GlueOrder>,
    pub list_ptr: MemPtr,
    _unused6: i32,
    pub glue_stretch: Scaled,
    pub sync_tag: i32,
    pub sync_line: i32,
}

overlay!(UnsetNode => UnsetLayout);

/// A one-word character node in the upper part of `mem`.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct CharLayout {
    pub character: u16,
    pub font: FontId,
    pub link: MemPtr,
}

overlay!(CharNode => CharLayout, 1);

/// A ligature. Its +1 word is laid out like a character node, see `char_word`.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct LigatureLayout {
    pub subtype: u16,
    _type: u16,
    pub link: MemPtr,
    pub character: u16,
    pub font: FontId,
    /// The original characters the ligature was formed from.
    pub lig_ptr: MemPtr,
}

overlay!(LigatureNode => LigatureLayout);

/// The first two words of a block on the variable-size free list.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct FreeBlockLayout {
    pub node_size: i32,
    /// `MAX_HALFWORD` marks the block as free.
    empty_flag: i32,
    pub llink: MemPtr,
    pub rlink: MemPtr,
}

overlay!(FreeBlock => FreeBlockLayout, 2);

impl FreeBlockLayout {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.empty_flag == crate::consts::MAX_HALFWORD
    }

    #[inline]
    pub fn mark_empty(&mut self) {
        self.empty_flag = crate::consts::MAX_HALFWORD;
    }
}

impl BoxLayout {
    #[inline]
    pub fn is_hlist(&self) -> bool {
        self.orientation == Orientation::Horizontal
    }

    #[inline]
    pub fn is_vlist(&self) -> bool {
        self.orientation == Orientation::Vertical
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::MemIdx;

    fn arena() -> MemState {
        MemState {
            data: vec![Default::default(); 16],
            ..Default::default()
        }
    }

    fn spec_at(idx: i32) -> NodePtr<GlueSpecNode> {
        NodePtr::from_idx_unchecked(MemIdx::from_raw(idx))
    }

    #[test]
    fn the_largest_layout_fits_at_the_top_of_mem() {
        let mut mem = MemState {
            data: crate::mem_types::zeroed_vec(crate::mem::MEM_WORDS),
            ..Default::default()
        };
        let top: NodePtr<BoxNode> =
            NodePtr::from_idx_unchecked(MemIdx::new(crate::mem::PAGE_INS_HEAD));
        mem[top].link = MemPtr::from_raw(5);
        assert_eq!(mem.b32s1(crate::mem::PAGE_INS_HEAD), 5);
    }

    #[test]
    fn glue_spec_fields_land_on_their_words() {
        let mut mem = arena();
        let s = spec_at(4);
        let spec = &mut mem[s];
        spec.shrink_order = GlueOrder::Fil.into();
        spec.stretch_order = GlueOrder::Fill.into();
        spec.ref_count = 7;
        spec.width = Scaled::from_raw(11);
        spec.stretch = Scaled::from_raw(12);
        spec.shrink = Scaled::from_raw(13);
        let at = MemIdx::from_raw;
        assert_eq!((mem.b16s0(at(4)), mem.b16s1(at(4))), (1, 2));
        assert_eq!([4, 5, 6, 7].map(|i| mem.b32s1(at(i))), [7, 11, 12, 13]);
        assert_eq!([3, 5, 6, 7, 8].map(|i| mem.b32s0(at(i))), [0; 5]);
        assert_eq!((mem.b32s1(at(3)), mem.b32s1(at(8))), (0, 0));
    }

    #[test]
    fn box_fields_land_on_their_words() {
        let mut mem = arena();
        let b: NodePtr<BoxNode> = NodePtr::from_idx_unchecked(MemIdx::from_raw(4));
        let bx = &mut mem[b];
        bx.subtype = 2;
        bx.orientation = Orientation::Vertical.into();
        bx.link = MemPtr::from_raw(3);
        bx.width = Scaled::from_raw(11);
        bx.depth = Scaled::from_raw(12);
        bx.height = Scaled::from_raw(13);
        bx.shift_amount = Scaled::from_raw(14);
        bx.glue_order = GlueOrder::Fil.into();
        bx.glue_sign = GlueSign::Shrinking.into();
        bx.list_ptr = MemPtr::from_raw(15);
        bx.glue_set = 0.5;
        bx.sync_tag = 16;
        bx.sync_line = 17;
        let at = MemIdx::from_raw;
        assert_eq!((mem.b16s0(at(4)), mem.b16s1(at(4))), (2, 1));
        assert_eq!((mem.b16s0(at(9)), mem.b16s1(at(9))), (1, 2));
        assert_eq!(
            [4, 5, 6, 7, 8, 9].map(|i| mem.b32s1(at(i))),
            [3, 11, 12, 13, 14, 15]
        );
        assert_eq!([5, 6, 7, 8].map(|i| mem.b32s0(at(i))), [0; 4]);
        assert_eq!((mem.b32s0(at(10)), mem.b32s1(at(10))), (0, 0x3FE0_0000));
        assert_eq!((mem.b32s0(at(11)), mem.b32s1(at(11))), (16, 17));
    }

    #[test]
    fn edge_and_margin_kern_fields_land_on_their_words() {
        let mut mem = arena();
        let at = MemIdx::from_raw;
        let e: NodePtr<EdgeNode> = NodePtr::from_idx_unchecked(at(4));
        let edge = &mut mem[e];
        edge.subtype = 1;
        edge.width = Scaled::from_raw(11);
        edge.edge_dist = Scaled::from_raw(12);
        assert_eq!(mem.b16s0(at(4)), 1);
        assert_eq!([5, 6].map(|i| mem.b32s1(at(i))), [11, 12]);
        let m: NodePtr<MarginKernNode> = NodePtr::from_idx_unchecked(at(8));
        mem[m].width = Scaled::from_raw(13);
        assert_eq!(mem.b32s1(at(9)), 13);
    }

    #[test]
    fn stack_record_fields_land_on_their_words() {
        let mut mem = arena();
        let at = MemIdx::from_raw;
        let ptr = MemPtr::from_raw;

        let c: NodePtr<CondNode> = NodePtr::from_idx_unchecked(at(2));
        let cond = &mut mem[c];
        cond.cur_if = 7;
        cond.if_limit = FiOrElseCode::Else.into();
        cond.link = ptr(9);
        cond.if_line = 11;
        assert_eq!((mem.b16s0(at(2)), mem.b16s1(at(2))), (7, 3));
        assert_eq!((mem.b32s1(at(2)), mem.b32s1(at(3))), (9, 11));

        let e: NodePtr<ExprNode> = NodePtr::from_idx_unchecked(at(4));
        let expr = &mut mem[e];
        expr.state = 2;
        expr.level = ValueLevel::Mu.into();
        (expr.e, expr.t, expr.n) = (21, 22, 23);
        assert_eq!((mem.b16s0(at(4)), mem.b16s1(at(4))), (2, 3));
        assert_eq!([5, 6, 7].map(|i| mem.b32s1(at(i))), [21, 22, 23]);

        let m: NodePtr<MovementNode> = NodePtr::from_idx_unchecked(at(8));
        let mv = &mut mem[m];
        (mv.state, mv.link, mv.distance, mv.location) = (5, ptr(6), Scaled::from_raw(31), 32);
        assert_eq!((mem.b32s0(at(8)), mem.b32s1(at(8))), (5, 6));
        assert_eq!((mem.b32s1(at(9)), mem.b32s1(at(10))), (31, 32));
    }

    #[test]
    fn align_stack_fields_land_on_their_words() {
        let mut mem = arena();
        let at = MemIdx::from_raw;
        let ptr = MemPtr::from_raw;
        let a: NodePtr<AlignStackNode> = NodePtr::from_idx_unchecked(at(4));
        let s = &mut mem[a];
        (s.cur_align, s.link, s.preamble, s.cur_span) = (ptr(1), ptr(2), ptr(3), ptr(4));
        (s.cur_loop, s.align_state) = (ptr(5), 6);
        (s.cur_head, s.cur_tail, s.cur_pre_head, s.cur_pre_tail) =
            (ptr(7), ptr(8), ptr(9), ptr(10));
        let words: Vec<_> = (4..10)
            .map(|i| (mem.b32s0(at(i)), mem.b32s1(at(i))))
            .collect();
        assert_eq!(words, [(1, 2), (3, 4), (0, 5), (0, 6), (7, 8), (9, 10)]);
    }

    #[test]
    fn unset_and_page_ins_fields_land_on_their_words() {
        let mut mem = arena();
        let at = MemIdx::from_raw;
        let ptr = MemPtr::from_raw;
        let u: NodePtr<UnsetNode> = NodePtr::from_idx_unchecked(at(4));
        let unset = &mut mem[u];
        unset.subtype = 2;
        (unset.width, unset.depth, unset.height) = (
            Scaled::from_raw(11),
            Scaled::from_raw(12),
            Scaled::from_raw(13),
        );
        unset.glue_shrink = Scaled::from_raw(14);
        unset.glue_order = GlueOrder::Fil.into();
        unset.glue_sign = GlueOrder::Filll.into();
        unset.list_ptr = ptr(15);
        unset.glue_stretch = Scaled::from_raw(16);
        assert_eq!(mem.b16s0(at(4)), 2);
        assert_eq!(
            [5, 6, 7, 8, 9, 10].map(|i| mem.b32s1(at(i))),
            [11, 12, 13, 14, 15, 16]
        );
        assert_eq!((mem.b16s0(at(9)), mem.b16s1(at(9))), (1, 3));

        let r: NodePtr<PageInsNode> = NodePtr::from_idx_unchecked(at(12));
        let rec = &mut mem[r];
        rec.subtype = 9;
        rec.set_split_up(true);
        (
            rec.broken_ins,
            rec.broken_ptr,
            rec.best_ins_ptr,
            rec.last_ins_ptr,
        ) = (ptr(1), ptr(2), ptr(3), ptr(4));
        rec.height = Scaled::from_raw(5);
        assert!(mem[r].is_split_up());
        assert_eq!((mem.b16s0(at(12)), mem.b16s1(at(12))), (9, 1));
        let words: Vec<_> = (13..15)
            .map(|i| (mem.b32s0(at(i)), mem.b32s1(at(i))))
            .collect();
        assert_eq!(words, [(1, 2), (3, 4)]);
        assert_eq!(mem.b32s1(at(15)), 5);
    }

    #[test]
    fn char_fields_land_on_their_word() {
        let mut mem = arena();
        let at = MemIdx::from_raw;
        let c: NodePtr<CharNode> = NodePtr::from_idx_unchecked(at(15));
        let ch = &mut mem[c];
        (ch.character, ch.font, ch.link) = (65, FontId::new(3), MemPtr::from_raw(9));
        assert_eq!((mem.b16s0(at(15)), mem.b16s1(at(15))), (65, 3));
        assert_eq!(mem.b32s1(at(15)), 9);
    }

    #[test]
    fn free_block_fields_land_on_their_words() {
        let mut mem = arena();
        let at = MemIdx::from_raw;
        let b: NodePtr<FreeBlock> = NodePtr::from_idx_unchecked(at(6));
        let block = &mut mem[b];
        block.node_size = 40;
        block.mark_empty();
        (block.llink, block.rlink) = (MemPtr::from_raw(2), MemPtr::from_raw(3));
        assert!(mem[b].is_empty());
        assert_eq!(
            (mem.b32s0(at(6)), mem.b32s1(at(6))),
            (40, crate::consts::MAX_HALFWORD)
        );
        assert_eq!((mem.b32s0(at(7)), mem.b32s1(at(7))), (2, 3));
    }

    #[test]
    fn a_mutable_borrow_journals_every_word_of_the_node() {
        let mut mem = arena();
        let s = spec_at(4);
        mem[s].shrink = Scaled::from_raw(5);
        mem.journal.arm(16);
        let spec = &mut mem[s];
        spec.ref_count = 1;
        spec.width = Scaled::from_raw(2);
        spec.stretch = Scaled::from_raw(3);
        spec.shrink = Scaled::from_raw(4);
        let delta = mem.journal.seal();
        delta.apply_to(&mut mem.data);
        let spec = mem[s];
        assert_eq!(
            (spec.ref_count, spec.width, spec.stretch, spec.shrink),
            (0, Scaled::ZERO, Scaled::ZERO, Scaled::from_raw(5))
        );
    }
}
