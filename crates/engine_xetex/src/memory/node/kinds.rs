//! The node kinds, each a struct laid over its words in `mem`.
//!
//! Indexing `mem` with a [`NodePtr`] borrows the node as its struct, and a
//! mutable borrow first notes the node's words in the journal.

use std::fmt::{Debug, Formatter};
use std::mem;
use std::ops::{Index, IndexMut, Range};

use bytemuck::{Pod, Zeroable};

use super::{
    AdjustPlacement, BoxLr, FenceSubtype, GlueKind, KernSubtype, LeaderKind, LigBoundary,
    MathAccentKind, MathCode, Node, NodePtr, Stored, StoredDimen, View, WhatsitSubtype,
};
use crate::base::codes::LimitCode;
use crate::base::consts::{EMPTY, NORMAL};
use crate::base::token::TokenList;
use crate::base::units::{
    Dir, FontId, GlueOrder, GlueSign, MemIdx, MemPtr, NodeType, Orientation, RawPtr, Scaled,
    StrNumber,
};
use crate::build::math::MathStyle;
use crate::fonts::Side;
use crate::memory::glue_store::GlueSpecId;
use crate::memory::mem::MemState;
use crate::print::errors::{Abort, OrConfusion};

/// Where SyncTeX places a node: the input tag and line it was made at.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, PartialEq, Eq, Debug)]
pub struct SyncStamp {
    pub tag: i32,
    pub line: i32,
}

#[cfg(not(target_endian = "little"))]
compile_error!("node structs assume little-endian memory words");

/// The words in `bytes`, for sizes and field offsets of node structs.
pub(crate) const fn word_offset(bytes: usize) -> i32 {
    assert!(bytes.is_multiple_of(4) && bytes / 4 < 1 << 16);
    (bytes / 4) as i32
}

impl<K: View> Index<NodePtr<K>> for MemState {
    type Output = K;

    #[inline]
    fn index(&self, p: NodePtr<K>) -> &K {
        bytemuck::from_bytes(bytemuck::cast_slice(&self.data[p.idx().words(K::SIZE)]))
    }
}

impl<K: View> IndexMut<NodePtr<K>> for MemState {
    #[inline]
    fn index_mut(&mut self, p: NodePtr<K>) -> &mut K {
        let words = p.idx().words(K::SIZE);
        let start = words.start;
        let node = &mut self.data[words];
        if self.journal.armed() {
            self.journal.note_run(start, node);
        }
        bytemuck::from_bytes_mut(bytemuck::cast_slice_mut(node))
    }
}

/// A node whose entries follow its struct in `mem`. Its header says how many.
/// Entries may be smaller than a word.
pub trait ArrayNode: Node {
    type Entry: Pod;
    /// The number of entries.
    fn len(&self) -> i32;
    /// Records the entry count of a fresh node.
    fn set_len(&mut self, entries: usize) -> Result<(), Abort>;
    /// The words kept for the entries, which stay the node's even when `len` shrinks.
    #[inline]
    fn entry_words(&self) -> usize {
        entry_words::<Self>(usize::try_from(self.len()).unwrap_or(0))
    }
}

/// The words `entries` entries of `K` take.
#[inline]
fn entry_words<K: ArrayNode>(entries: usize) -> usize {
    entries
        .saturating_mul(mem::size_of::<K::Entry>())
        .div_ceil(mem::size_of::<u32>())
}

pub type EntriesMutAnd<'a, K, L> = (
    &'a mut [<K as ArrayNode>::Entry],
    &'a [<L as ArrayNode>::Entry],
);

impl MemState {
    /// The words of `p`'s entries and how many bytes of them are in use.
    #[inline]
    fn entry_span<K: ArrayNode>(&self, p: NodePtr<K>) -> (Range<usize>, usize) {
        let len = usize::try_from(self[p].len()).unwrap_or(0);
        let start = p.idx().words(K::SIZE).end;
        (
            start..start + entry_words::<K>(len),
            len.saturating_mul(mem::size_of::<K::Entry>()),
        )
    }

    /// A fresh array node with room for `entries` zeroed entries, its other fields unset.
    pub fn alloc_array<K: ArrayNode>(&mut self, entries: usize) -> Result<NodePtr<K>, Abort> {
        const { assert!(K::ARRAY) };
        let head = usize::try_from(K::SIZE).or_confusion("node size")?;
        let p = self.get_array_node(head.saturating_add(entry_words::<K>(entries)))?;
        if let Some(ty) = K::TYPE {
            self.set_header(p, ty, 0);
        }
        let p = NodePtr::from_idx_unchecked(p);
        K::init(self, p);
        self[p].set_len(entries)?;
        self.entries_mut(p).fill(Zeroable::zeroed());
        Ok(p)
    }

    /// A new node holding every field and entry of array node `src`.
    pub fn copy_array<K: ArrayNode>(&mut self, src: NodePtr<K>) -> Result<NodePtr<K>, Abort> {
        let entries = usize::try_from(self[src].len()).unwrap_or(0);
        let copy = self.alloc_array::<K>(entries)?;
        self[copy] = self[src];
        self[copy].set_len(entries)?;
        let (to, from) = self.entries_mut_and(copy, src)?;
        to.copy_from_slice(from);
        Ok(copy)
    }

    /// Returns array node `p` to the free list of its size.
    pub fn free_array<K: ArrayNode>(&mut self, p: NodePtr<K>) {
        let words = usize::try_from(K::SIZE).unwrap_or(0) + self[p].entry_words();
        self.free_array_node(p.idx(), words);
    }

    /// The entries of an array node.
    #[inline]
    pub fn entries<K: ArrayNode>(&self, p: NodePtr<K>) -> &[K::Entry] {
        let (words, bytes) = self.entry_span(p);
        let words: &[u8] = bytemuck::cast_slice(&self.data[words]);
        bytemuck::cast_slice(&words[..bytes])
    }

    /// The entries of `dst`, writable, beside those of another node `src`.
    #[inline]
    pub fn entries_mut_and<K: ArrayNode, L: ArrayNode>(
        &mut self,
        dst: NodePtr<K>,
        src: NodePtr<L>,
    ) -> Result<EntriesMutAnd<'_, K, L>, Abort> {
        let (dst_words, dst_bytes) = self.entry_span(dst);
        let (src_words, src_bytes) = self.entry_span(src);
        if self.journal.armed() {
            self.journal
                .note_run(dst_words.start, &self.data[dst_words.clone()]);
        }
        let [dst, src] = self
            .data
            .get_disjoint_mut([dst_words, src_words])
            .or_confusion("entries")?;
        let dst: &mut [u8] = bytemuck::cast_slice_mut(dst);
        let src: &[u8] = bytemuck::cast_slice(src);
        Ok((
            bytemuck::cast_slice_mut(&mut dst[..dst_bytes]),
            bytemuck::cast_slice(&src[..src_bytes]),
        ))
    }

    /// The entries of an array node, writable.
    #[inline]
    pub fn entries_mut<K: ArrayNode>(&mut self, p: NodePtr<K>) -> &mut [K::Entry] {
        let (words, bytes) = self.entry_span(p);
        let start = words.start;
        let words = &mut self.data[words];
        if self.journal.armed() {
            self.journal.note_run(start, words);
        }
        let words: &mut [u8] = bytemuck::cast_slice_mut(words);
        bytemuck::cast_slice_mut(&mut words[..bytes])
    }
}

/// The header of a `\parshape` array.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ParShapeNode {
    pub lines: i32,
}

/// One line of a `\parshape` array.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ParShapeLine {
    pub indent: Scaled,
    pub width: Scaled,
}

impl View for ParShapeNode {}

impl Node for ParShapeNode {
    const TYPE: Option<NodeType> = None;
    const ARRAY: bool = true;
}

impl ArrayNode for ParShapeNode {
    type Entry = ParShapeLine;

    #[inline]
    fn len(&self) -> i32 {
        self.lines
    }

    #[inline]
    fn set_len(&mut self, entries: usize) -> Result<(), Abort> {
        self.lines = i32::try_from(entries).or_confusion("parshape")?;
        Ok(())
    }
}

/// The header of an e-TeX penalty array.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PenaltiesNode {
    /// Half the entry count, rounded up, as e-TeX's node size was.
    pub half_size: i32,
    pub count: i32,
}

/// One entry of an e-TeX penalty array, or the zero padding after an even count.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PenaltyEntry {
    pub value: i32,
}

impl View for PenaltiesNode {}

impl Node for PenaltiesNode {
    const TYPE: Option<NodeType> = None;
    const ARRAY: bool = true;
}

impl ArrayNode for PenaltiesNode {
    type Entry = PenaltyEntry;

    #[inline]
    fn len(&self) -> i32 {
        2 * self.half_size - 1
    }

    #[inline]
    fn set_len(&mut self, entries: usize) -> Result<(), Abort> {
        self.half_size = i32::try_from(entries.div_ceil(2)).or_confusion("penalties")?;
        Ok(())
    }
}

/// The header of a `\scantokens` pseudo-file line.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PseudoLineNode {
    /// One more than the line's words of four characters, as e-TeX's node size was.
    pub size: i32,
    pub link: Option<NodePtr<PseudoLineNode>>,
}

/// Four characters of a pseudo-file line, stored last first.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PseudoWord {
    reversed: [u16; 4],
}

impl PseudoWord {
    #[inline]
    pub fn new([a, b, c, d]: [u16; 4]) -> Self {
        PseudoWord {
            reversed: [d, c, b, a],
        }
    }

    #[inline]
    pub fn chars(self) -> [u16; 4] {
        let [d, c, b, a] = self.reversed;
        [a, b, c, d]
    }
}

impl View for PseudoLineNode {}

impl Node for PseudoLineNode {
    const TYPE: Option<NodeType> = None;
    const ARRAY: bool = true;
}

impl ArrayNode for PseudoLineNode {
    type Entry = PseudoWord;

    #[inline]
    fn len(&self) -> i32 {
        self.size - 1
    }

    #[inline]
    fn set_len(&mut self, entries: usize) -> Result<(), Abort> {
        self.size = i32::try_from(entries + 1).or_confusion("pseudo line")?;
        Ok(())
    }
}

/// The words every list node starts with, read before its kind is known.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct NodeHeader {
    pub subtype: u16,
    pub ty: u16,
    pub link: MemPtr,
}

impl View for NodeHeader {}

/// Any whatsit: its subtype and link.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct WhatsitNode {
    pub subtype: Stored<WhatsitSubtype>,
    _type: u16,
    pub link: MemPtr,
}

impl View for WhatsitNode {}

/// A `\pdfsavepos` mark, nothing but its header.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SavePosNode {
    pub subtype: Stored<WhatsitSubtype>,
    _type: u16,
    pub link: MemPtr,
}

impl View for SavePosNode {}

impl Node for SavePosNode {
    const TYPE: Option<NodeType> = Some(NodeType::Whatsit);
}

/// `\openout`.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct OpenNode {
    pub subtype: Stored<WhatsitSubtype>,
    _type: u16,
    pub link: MemPtr,
    pub stream: i32,
    pub name: StrNumber,
    pub area: StrNumber,
    pub ext: StrNumber,
}

impl View for OpenNode {}

impl Node for OpenNode {
    const TYPE: Option<NodeType> = Some(NodeType::Whatsit);
}

/// `\write`, `\closeout` or `\special`. A close has no tokens, a special
/// no stream.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct WriteNode {
    pub subtype: Stored<WhatsitSubtype>,
    _type: u16,
    pub link: MemPtr,
    pub stream: i32,
    pub tokens: TokenList,
}

impl View for WriteNode {}

impl Node for WriteNode {
    const TYPE: Option<NodeType> = Some(NodeType::Whatsit);
}

/// A language change: the language and its hyphenation minima.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct LanguageNode {
    pub subtype: Stored<WhatsitSubtype>,
    _type: u16,
    pub link: MemPtr,
    pub rhm: u16,
    pub lhm: u16,
    pub lang: i32,
}

impl View for LanguageNode {}

impl Node for LanguageNode {
    const TYPE: Option<NodeType> = Some(NodeType::Whatsit);
}

/// A glyph of a native font. The glyph id sits where a native word keeps
/// its length.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct GlyphNode {
    pub subtype: Stored<WhatsitSubtype>,
    _type: u16,
    pub link: MemPtr,
    pub width: Scaled,
    pub depth: Scaled,
    pub height: Scaled,
    _glyph_count: u16,
    pub glyph: u16,
    pub font: FontId,
    _size: u16,
}

impl View for GlyphNode {}

impl Node for GlyphNode {
    const TYPE: Option<NodeType> = Some(NodeType::Whatsit);
}

/// A picture. Its path bytes follow the header.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PicNode {
    pub subtype: Stored<WhatsitSubtype>,
    _type: u16,
    pub link: MemPtr,
    pub width: Scaled,
    pub depth: Scaled,
    pub height: Scaled,
    pub page: u16,
    pub path_len: u16,
    /// `[a, b, c, d, x, y]`.
    pub transform: [Scaled; 6],
    _unused5: u16,
    pub pagebox: u16,
}

impl View for PicNode {}

impl Node for PicNode {
    const TYPE: Option<NodeType> = Some(NodeType::Whatsit);
    const ARRAY: bool = true;
}

impl ArrayNode for PicNode {
    type Entry = u8;

    #[inline]
    fn len(&self) -> i32 {
        i32::from(self.path_len)
    }

    #[inline]
    fn set_len(&mut self, entries: usize) -> Result<(), Abort> {
        self.path_len = u16::try_from(entries).or_confusion("picture path")?;
        Ok(())
    }
}

/// A native word. Its UTF-16 text follows the header, and its shaped glyphs
/// live in `mem.glyph_blobs` under `glyph_info`.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct NativeWordNode {
    pub subtype: Stored<WhatsitSubtype>,
    _type: u16,
    pub link: MemPtr,
    pub width: Scaled,
    pub depth: Scaled,
    pub height: Scaled,
    pub glyph_count: u16,
    /// The text length in UTF-16 units.
    pub length: u16,
    pub font: FontId,
    /// The words kept for the text, which outlive a shortened `length`.
    text_words: u16,
    /// One-based shaped-glyph arena handle, zero when unshaped.
    pub glyph_info: i32,
}

impl NativeWordNode {
    /// Forgets the shaped glyphs, without releasing them.
    #[inline]
    pub fn clear_glyphs(&mut self) {
        self.glyph_count = 0;
        self.glyph_info = 0;
    }
}

impl View for NativeWordNode {}

impl Node for NativeWordNode {
    const TYPE: Option<NodeType> = Some(NodeType::Whatsit);
    const ARRAY: bool = true;
}

impl ArrayNode for NativeWordNode {
    type Entry = u16;

    #[inline]
    fn len(&self) -> i32 {
        i32::from(self.length)
    }

    #[inline]
    fn set_len(&mut self, entries: usize) -> Result<(), Abort> {
        self.length = u16::try_from(entries).or_confusion("native word length")?;
        self.text_words =
            u16::try_from(entry_words::<Self>(entries)).or_confusion("native text")?;
        Ok(())
    }

    #[inline]
    fn entry_words(&self) -> usize {
        usize::from(self.text_words)
    }
}

/// The box dimensions of a native word, glyph or picture.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Dims {
    pub width: Scaled,
    pub depth: Scaled,
    pub height: Scaled,
}

/// A box's glue ratio, an `f64` kept as two words so node structs stay 4-byte aligned.
#[repr(transparent)]
#[derive(Copy, Clone, Pod, Zeroable, Default)]
pub struct GlueRatio([u32; 2]);

impl GlueRatio {
    #[inline]
    pub fn get(self) -> f64 {
        f64::from_bits(bytemuck::cast(self.0))
    }

    #[inline]
    pub fn new(r: f64) -> Self {
        GlueRatio(bytemuck::cast(r.to_bits()))
    }
}

impl Debug for GlueRatio {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.get().fmt(f)
    }
}

/// An hlist or vlist box. Its orientation is the node's type code.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct BoxNode {
    subtype: u16,
    pub orientation: Stored<Orientation>,
    pub link: MemPtr,
    pub width: Scaled,
    pub depth: Scaled,
    pub height: Scaled,
    pub shift_amount: Scaled,
    pub glue_order: Stored<GlueOrder>,
    pub glue_sign: Stored<GlueSign>,
    pub list_ptr: MemPtr,
    pub glue_set: GlueRatio,
    pub sync: SyncStamp,
}

impl View for BoxNode {}

impl Node for BoxNode {
    const TYPE: Option<NodeType> = Some(NodeType::HList);

    #[inline]
    fn init(mem: &mut MemState, p: NodePtr<Self>) {
        mem[p].sync = mem.sync_stamp();
    }
}

impl BoxNode {
    /// TeX--XeT's direction state of an hlist.
    #[inline]
    pub fn lr(&self) -> Stored<BoxLr> {
        Stored::from_raw(self.subtype)
    }

    #[inline]
    pub fn set_lr(&mut self, lr: BoxLr) {
        self.subtype = Stored::from(lr).raw();
    }

    /// A vlist that XeTeX's `vpack` built bottom to top, kept in the same half-word.
    #[inline]
    pub fn upwards(&self) -> bool {
        self.subtype == 1
    }

    #[inline]
    pub fn set_upwards(&mut self, upwards: bool) {
        self.subtype = u16::from(upwards);
    }
}

/// A kern. Its subtype tells normal, explicit, accent and math kerns apart.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct KernNode {
    pub subtype: Stored<KernSubtype>,
    _type: u16,
    pub link: MemPtr,
    _unused1: i32,
    pub width: Scaled,
    pub sync: SyncStamp,
}

impl View for KernNode {}

impl Node for KernNode {
    const TYPE: Option<NodeType> = Some(NodeType::Kern);

    #[inline]
    fn init(mem: &mut MemState, p: NodePtr<Self>) {
        mem[p].sync = mem.sync_stamp();
    }
}

/// A math-on/off node, or a TeX--XeT LR boundary, by its subtype.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct MathNode {
    pub subtype: Stored<MathCode>,
    _type: u16,
    pub link: MemPtr,
    _unused1: i32,
    pub width: Scaled,
    pub sync: SyncStamp,
}

impl View for MathNode {}

impl Node for MathNode {
    const TYPE: Option<NodeType> = Some(NodeType::Math);

    #[inline]
    fn init(mem: &mut MemState, p: NodePtr<Self>) {
        mem[p].sync = mem.sync_stamp();
    }
}

impl MathNode {
    /// The node's code. Every writer stores a [`MathCode`], so another value is a broken invariant.
    #[inline]
    pub fn code(&self) -> Result<MathCode, Abort> {
        self.subtype.get().or_confusion("math code")
    }
}

/// A margin kern on the side it protrudes on.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct MarginKernNode {
    pub side: Stored<Side>,
    _type: u16,
    pub link: MemPtr,
    pub width: Scaled,
}

impl View for MarginKernNode {}

impl Node for MarginKernNode {
    const TYPE: Option<NodeType> = Some(NodeType::MarginKern);
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PenaltyNode {
    _subtype: u16,
    _type: u16,
    pub link: MemPtr,
    _unused1: i32,
    pub penalty: i32,
    _unused2: [i32; 2],
}

impl View for PenaltyNode {}

impl Node for PenaltyNode {
    const TYPE: Option<NodeType> = Some(NodeType::Penalty);
}

/// A `\vadjust` list.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct AdjustNode {
    pub placement: Stored<AdjustPlacement>,
    _type: u16,
    pub link: MemPtr,
    pub adjust_ptr: MemPtr,
}

impl View for AdjustNode {}

impl Node for AdjustNode {
    const TYPE: Option<NodeType> = Some(NodeType::Adjust);
}

/// A discretionary.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct DiscNode {
    pub replace_count: u16,
    _type: u16,
    pub link: MemPtr,
    pub pre_break: MemPtr,
    pub post_break: MemPtr,
}

impl View for DiscNode {}

impl Node for DiscNode {
    const TYPE: Option<NodeType> = Some(NodeType::Disc);
}

/// A `\mathchoice` node with one mlist per style.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ChoiceNode {
    _subtype: u16,
    _type: u16,
    pub link: MemPtr,
    pub display_mlist: MemPtr,
    pub text_mlist: MemPtr,
    pub script_mlist: MemPtr,
    pub script_script_mlist: MemPtr,
}

impl View for ChoiceNode {}

impl Node for ChoiceNode {
    const TYPE: Option<NodeType> = Some(NodeType::Choice);
}

/// A style change in an mlist.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct StyleNode {
    pub style: MathStyle,
    _type: u16,
    pub link: MemPtr,
    _unused1: i32,
    payload1: i32,
    _unused2: i32,
    payload2: i32,
}

impl View for StyleNode {}

impl Node for StyleNode {
    const TYPE: Option<NodeType> = Some(NodeType::Style);
}

impl StyleNode {
    /// Zeroes the two payload words, as `new_style` does.
    #[inline]
    pub fn clear_payload(&mut self) {
        self.payload1 = 0;
        self.payload2 = 0;
    }
}

/// An LR edge in a shipped list.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct EdgeNode {
    pub dir: Stored<Dir>,
    _type: u16,
    pub link: MemPtr,
    _unused1: i32,
    pub width: Scaled,
    _unused2: i32,
    pub edge_dist: Scaled,
}

impl View for EdgeNode {}

impl Node for EdgeNode {
    const TYPE: Option<NodeType> = Some(NodeType::Style);
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct MarkNode {
    _subtype: u16,
    _type: u16,
    pub link: MemPtr,
    pub mark_class: i32,
    pub mark_ptr: TokenList,
}

impl View for MarkNode {}

impl Node for MarkNode {
    const TYPE: Option<NodeType> = Some(NodeType::Mark);
}

/// A noad's nucleus, script or accent word: its math type, then either a
/// character and family or a sub-list in the shared payload half.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct MathWord {
    character: u16,
    plane_and_fam: u16,
    pub math_type: i32,
}

impl MathWord {
    /// The list of a `SUB_BOX` or `SUB_MLIST` field.
    #[inline]
    pub fn info(&self) -> MemPtr {
        MemIdx::from_raw(bytemuck::cast([self.character, self.plane_and_fam]))
    }
    #[inline]
    pub fn set_info(&mut self, p: MemPtr) {
        [self.character, self.plane_and_fam] = bytemuck::cast(p.raw());
    }
    #[inline]
    pub fn character(&self) -> u16 {
        self.character
    }
    #[inline]
    pub fn set_character(&mut self, c: u16) {
        self.character = c;
    }
    /// The family in the low byte and the character's plane above it.
    #[inline]
    pub fn plane_and_fam(&self) -> u16 {
        self.plane_and_fam
    }
    #[inline]
    pub fn set_plane_and_fam(&mut self, v: u16) {
        self.plane_and_fam = v;
    }
    #[inline]
    pub fn fam(&self) -> i32 {
        i32::from(self.plane_and_fam) % 256
    }
    /// The character code with its plane.
    #[inline]
    pub fn full_char(&self) -> i32 {
        i32::from(self.character) + (i32::from(self.plane_and_fam) / 256) * 65_536
    }
    #[inline]
    pub fn clear(&mut self) {
        self.math_type = EMPTY;
        self.set_info(None);
    }
}

/// A delimiter word: a large and a small variant, each a character and a
/// family with the character's plane above it.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct DelimiterWord {
    large_char: u16,
    large_plane_and_fam: u16,
    small_char: u16,
    small_plane_and_fam: u16,
}

impl DelimiterWord {
    #[inline]
    fn variant(plane_and_fam: u16, char: u16) -> (i32, i32) {
        let plane_and_fam = i32::from(plane_and_fam);
        (
            plane_and_fam % 256,
            i32::from(char) + (plane_and_fam / 256) * 65_536,
        )
    }
    /// The small variant's `(family, character)`.
    #[inline]
    pub fn small(&self) -> (i32, i32) {
        Self::variant(self.small_plane_and_fam, self.small_char)
    }
    /// The large variant's `(family, character)`.
    #[inline]
    pub fn large(&self) -> (i32, i32) {
        Self::variant(self.large_plane_and_fam, self.large_char)
    }
    #[inline]
    pub fn set_small(&mut self, plane_and_fam: u16, char: u16) {
        self.small_plane_and_fam = plane_and_fam;
        self.small_char = char;
    }
    #[inline]
    pub fn set_large(&mut self, plane_and_fam: u16, char: u16) {
        self.large_plane_and_fam = plane_and_fam;
        self.large_char = char;
    }
    #[inline]
    pub fn clear(&mut self) {
        *self = Self::zeroed();
    }
    #[inline]
    pub fn is_null(&self) -> bool {
        self.small() == (0, 0) && self.large() == (0, 0)
    }
}

/// The words every noad starts with: the header, then the hlist `mlist_to_hlist` translates it to.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct AnyNoad {
    _subtype: u16,
    pub noad_type: Stored<NodeType>,
    pub link: MemPtr,
    pub new_hlist: MemPtr,
}

impl View for AnyNoad {}

/// A noad with a nucleus and two scripts.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Noad {
    subtype: u16,
    pub noad_type: Stored<NodeType>,
    pub link: MemPtr,
    pub new_hlist: MemPtr,
    pub nucleus: MathWord,
    pub supscr: MathWord,
    pub subscr: MathWord,
}

impl View for Noad {}

impl Node for Noad {
    const TYPE: Option<NodeType> = Some(NodeType::OrdNoad);
}

impl Noad {
    /// Empty fields and a normal subtype, as a new noad starts.
    #[inline]
    pub fn clear_fields(&mut self) {
        self.subtype = NORMAL;
        self.new_hlist = None;
        self.nucleus.clear();
        self.supscr.clear();
        self.subscr.clear();
    }

    /// `\limits` or `\nolimits` on an operator. Accents keep their kind in the same half-word.
    #[inline]
    pub fn limits(&self) -> Stored<LimitCode> {
        Stored::from_raw(self.subtype)
    }

    #[inline]
    pub fn set_limits(&mut self, limits: LimitCode) {
        self.subtype = Stored::from(limits).raw();
    }
}

/// A radical: a noad, then its delimiter.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct RadicalNoad {
    pub base: Noad,
    pub left_delimiter: DelimiterWord,
}

impl View for RadicalNoad {}

impl Node for RadicalNoad {
    const TYPE: Option<NodeType> = Some(NodeType::RadicalNoad);
}

/// A math accent: a noad, then its accent character. The subtype marks
/// fixed and bottom accents.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct AccentNoad {
    pub base: Noad,
    pub accent_chr: MathWord,
}

impl AccentNoad {
    /// Where the accent goes and whether it stretches, kept in the noad's subtype.
    #[inline]
    pub fn kind(&self) -> Stored<MathAccentKind> {
        Stored::from_raw(self.base.subtype)
    }

    #[inline]
    pub fn set_kind(&mut self, kind: MathAccentKind) {
        self.base.subtype = Stored::from(kind).raw();
    }
}

impl View for AccentNoad {}

impl Node for AccentNoad {
    const TYPE: Option<NodeType> = Some(NodeType::AccentNoad);
}

/// A generalized fraction.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct FractionNoad {
    _subtype: u16,
    pub noad_type: Stored<NodeType>,
    pub link: MemPtr,
    pub new_hlist: MemPtr,
    /// The rule thickness, `DEFAULT_CODE` for the font's.
    pub thickness: Scaled,
    pub numerator: MathWord,
    pub denominator: MathWord,
    pub left_delimiter: DelimiterWord,
    pub right_delimiter: DelimiterWord,
}

impl View for FractionNoad {}

impl Node for FractionNoad {
    const TYPE: Option<NodeType> = Some(NodeType::FractionNoad);
}

/// A `\left`, `\middle` or `\right` delimiter. A middle one is a right noad
/// with a middle subtype. WEB leaves the script words empty but reads them.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct FenceNoad {
    pub subtype: Stored<FenceSubtype>,
    pub noad_type: Stored<NodeType>,
    pub link: MemPtr,
    pub new_hlist: MemPtr,
    pub delimiter: DelimiterWord,
    pub supscr: MathWord,
    pub subscr: MathWord,
}

impl View for FenceNoad {}

impl Node for FenceNoad {
    const TYPE: Option<NodeType> = Some(NodeType::LeftNoad);
}

/// An entry of a column's span list, ended by `END_SPAN`.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SpanNode {
    pub next: Option<NodePtr<SpanNode>>,
    /// Columns spanned beyond the first. `END_SPAN` holds a larger sentinel.
    pub span_count: i32,
    pub width: Scaled,
}

impl View for SpanNode {}

impl Node for SpanNode {
    const TYPE: Option<NodeType> = None;
}

/// An alignment preamble column: a null box holding the column's templates,
/// natural width and span list.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct AlignRecord {
    /// The column's span list, sorted by span count and ended by `END_SPAN`.
    pub span_list: Option<NodePtr<SpanNode>>,
    pub link: MemPtr,
    pub width: Scaled,
    pub v_part: TokenList,
    pub u_part: TokenList,
    _unused1: i32,
    /// The command that ended the current entry of this column.
    pub extra_info: i32,
    _unused2: [i32; 3],
    pub sync: SyncStamp,
}

impl View for AlignRecord {}

impl Node for AlignRecord {
    const TYPE: Option<NodeType> = Some(NodeType::HList);

    #[inline]
    fn init(mem: &mut MemState, p: NodePtr<Self>) {
        mem[p].sync = mem.sync_stamp();
    }
}

/// An entry of the ligature reconstitution stack.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct LigItem {
    pub character: u16,
    _unused0: u16,
    pub link: MemPtr,
    pub component: MemPtr,
}

impl View for LigItem {}

impl Node for LigItem {
    const TYPE: Option<NodeType> = None;
}

/// A rule. A running dimension holds `NULL_FLAG`.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct RuleNode {
    _subtype: u16,
    _type: u16,
    pub link: MemPtr,
    pub width: StoredDimen,
    pub depth: StoredDimen,
    pub height: StoredDimen,
    pub sync: SyncStamp,
}

impl View for RuleNode {}

impl Node for RuleNode {
    const TYPE: Option<NodeType> = Some(NodeType::Rule);

    #[inline]
    fn init(mem: &mut MemState, p: NodePtr<Self>) {
        mem[p].sync = mem.sync_stamp();
    }
}

/// A glue node. Its subtype is a glue parameter, a leader kind or a math code.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct GlueNode {
    pub subtype: Stored<GlueKind>,
    _type: u16,
    pub link: MemPtr,
    pub glue_ptr: GlueSpecId,
    /// The leader box or rule, null for plain glue.
    pub leader_ptr: MemPtr,
    pub sync: SyncStamp,
}

impl View for GlueNode {}

impl Node for GlueNode {
    const TYPE: Option<NodeType> = Some(NodeType::Glue);

    #[inline]
    fn init(mem: &mut MemState, p: NodePtr<Self>) {
        mem[p].sync = mem.sync_stamp();
    }
}

impl GlueNode {
    /// The leader kind, `None` for any other glue.
    #[inline]
    pub fn leaders(&self) -> Option<LeaderKind> {
        match self.subtype.get() {
            Some(GlueKind::Leaders(kind)) => Some(kind),
            _ => None,
        }
    }
}

/// An insertion into the class of its box number.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct InsNode {
    pub box_number: u8,
    _pad: u8,
    _type: u16,
    pub link: MemPtr,
    pub float_cost: i32,
    pub depth: Scaled,
    pub height: Scaled,
    pub ins_ptr: MemPtr,
    /// The `\splittopskip` glue spec.
    pub split_top_ptr: GlueSpecId,
}

impl View for InsNode {}

impl Node for InsNode {
    const TYPE: Option<NodeType> = Some(NodeType::Ins);
}

/// A record on the page builder's insertion ring. Its type code is
/// `inserting` or `split_up`, its subtype the insertion's box number.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PageInsNode {
    pub box_number: u8,
    _pad: u8,
    split_up: u16,
    pub link: MemPtr,
    pub broken_ins: MemPtr,
    pub broken_ptr: MemPtr,
    pub best_ins_ptr: MemPtr,
    pub last_ins_ptr: MemPtr,
    pub height: Scaled,
}

impl View for PageInsNode {}

impl Node for PageInsNode {
    const TYPE: Option<NodeType> = None;
}

impl PageInsNode {
    #[inline]
    pub fn is_split_up(&self) -> bool {
        self.split_up == 1
    }

    #[inline]
    pub fn set_split_up(&mut self, split: bool) {
        self.split_up = u16::from(split);
    }
}

/// An alignment box before its column widths are known. It keeps the box's
/// fields but holds shrink and stretch in place of shift and glue setting.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct UnsetNode {
    /// The number of columns spanned, less one.
    pub span_count: u16,
    _type: u16,
    pub link: MemPtr,
    pub width: Scaled,
    pub depth: Scaled,
    pub height: Scaled,
    pub glue_shrink: Scaled,
    /// The stretch order.
    pub glue_order: Stored<GlueOrder>,
    /// The shrink order.
    pub glue_sign: Stored<GlueOrder>,
    pub list_ptr: MemPtr,
    _unused6: i32,
    pub glue_stretch: Scaled,
    pub sync: SyncStamp,
}

impl View for UnsetNode {}

impl Node for UnsetNode {
    const TYPE: Option<NodeType> = Some(NodeType::Unset);

    #[inline]
    fn init(mem: &mut MemState, p: NodePtr<Self>) {
        mem[p].sync = mem.sync_stamp();
    }
}

/// A character node: the character in the subtype, then its font.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct CharNode {
    pub character: u16,
    _type: u16,
    pub link: MemPtr,
    pub font: FontId,
    _unused1: u16,
}

impl View for CharNode {}

impl Node for CharNode {
    const TYPE: Option<NodeType> = Some(NodeType::Char);
}

/// The head of a list under construction. Only its link is used.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct HeadNode {
    _subtype: u16,
    _type: u16,
    pub link: MemPtr,
}

impl View for HeadNode {}

impl Node for HeadNode {
    const TYPE: Option<NodeType> = Some(NodeType::Head);
}

/// An open `\scantokens` pseudo file: its unread lines and the next file.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PseudoFileNode {
    pub lines: Option<NodePtr<PseudoLineNode>>,
    pub link: MemPtr,
}

impl View for PseudoFileNode {}

impl Node for PseudoFileNode {
    const TYPE: Option<NodeType> = None;
}

/// One hyphen position of a `\hyphenation` exception, in a list from the last position.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct HyphPosNode {
    pub pos: i32,
    pub link: MemPtr,
}

impl View for HyphPosNode {}

impl Node for HyphPosNode {
    const TYPE: Option<NodeType> = None;
}

/// An entry of the TeX--XeT LR stack.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct LrNode {
    pub code: Stored<MathCode>,
    _unused1: u16,
    pub link: MemPtr,
}

impl View for LrNode {}

impl Node for LrNode {
    const TYPE: Option<NodeType> = None;
}

/// A ligature, with the character and font it shows.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct LigatureNode {
    pub boundary: Stored<LigBoundary>,
    _type: u16,
    pub link: MemPtr,
    pub character: u16,
    pub font: FontId,
    /// The original characters the ligature was formed from.
    pub lig_ptr: MemPtr,
}

impl View for LigatureNode {}

impl Node for LigatureNode {
    const TYPE: Option<NodeType> = Some(NodeType::Ligature);
}

/// Asserts that each `Kind.field` pair sits at one offset, for fields read through another kind.
macro_rules! same_offset {
    ($($a:ident.$f:ident == $b:ident.$g:ident),+ $(,)?) => {
        $(const _: () = assert!(
            std::mem::offset_of!($a, $f) == std::mem::offset_of!($b, $g)
        );)+
    };
}

same_offset!(
    // The header every list node is read through before its kind is known.
    WhatsitNode._type == NodeHeader.ty,
    WhatsitNode.link == NodeHeader.link,
    OpenNode._type == NodeHeader.ty,
    OpenNode.link == NodeHeader.link,
    WriteNode._type == NodeHeader.ty,
    WriteNode.link == NodeHeader.link,
    LanguageNode._type == NodeHeader.ty,
    LanguageNode.link == NodeHeader.link,
    GlyphNode._type == NodeHeader.ty,
    GlyphNode.link == NodeHeader.link,
    PicNode._type == NodeHeader.ty,
    PicNode.link == NodeHeader.link,
    NativeWordNode._type == NodeHeader.ty,
    NativeWordNode.link == NodeHeader.link,
    KernNode._type == NodeHeader.ty,
    KernNode.link == NodeHeader.link,
    MathNode._type == NodeHeader.ty,
    MathNode.link == NodeHeader.link,
    MarginKernNode._type == NodeHeader.ty,
    MarginKernNode.link == NodeHeader.link,
    PenaltyNode._type == NodeHeader.ty,
    PenaltyNode.link == NodeHeader.link,
    AdjustNode._type == NodeHeader.ty,
    AdjustNode.link == NodeHeader.link,
    DiscNode._type == NodeHeader.ty,
    DiscNode.link == NodeHeader.link,
    ChoiceNode._type == NodeHeader.ty,
    ChoiceNode.link == NodeHeader.link,
    StyleNode._type == NodeHeader.ty,
    StyleNode.link == NodeHeader.link,
    EdgeNode._type == NodeHeader.ty,
    EdgeNode.link == NodeHeader.link,
    MarkNode._type == NodeHeader.ty,
    MarkNode.link == NodeHeader.link,
    RuleNode._type == NodeHeader.ty,
    RuleNode.link == NodeHeader.link,
    GlueNode._type == NodeHeader.ty,
    GlueNode.link == NodeHeader.link,
    InsNode._type == NodeHeader.ty,
    InsNode.link == NodeHeader.link,
    UnsetNode._type == NodeHeader.ty,
    UnsetNode.link == NodeHeader.link,
    CharNode._type == NodeHeader.ty,
    CharNode.link == NodeHeader.link,
    HeadNode._type == NodeHeader.ty,
    HeadNode.link == NodeHeader.link,
    LigatureNode._type == NodeHeader.ty,
    LigatureNode.link == NodeHeader.link,
    BoxNode.orientation == NodeHeader.ty,
    BoxNode.link == NodeHeader.link,
    AnyNoad.noad_type == NodeHeader.ty,
    AnyNoad.link == NodeHeader.link,
    PageInsNode.split_up == NodeHeader.ty,
    PageInsNode.link == NodeHeader.link,
);

same_offset!(
    // Retyped in place: unset and box, an align record into an unset, math into kern.
    UnsetNode.width == BoxNode.width,
    UnsetNode.depth == BoxNode.depth,
    UnsetNode.height == BoxNode.height,
    UnsetNode.list_ptr == BoxNode.list_ptr,
    AlignRecord.width == UnsetNode.width,
    // `into_unset` clears the unset fields an align record keeps its parts in.
    AlignRecord.v_part == UnsetNode.depth,
    AlignRecord.u_part == UnsetNode.height,
    AlignRecord.extra_info == UnsetNode.glue_order,
    // An unset's glue totals sit where the box's shift and glue ratio do.
    UnsetNode.glue_shrink == BoxNode.shift_amount,
    UnsetNode.glue_order == BoxNode.glue_order,
    UnsetNode.glue_sign == BoxNode.glue_sign,
    MathNode.width == KernNode.width,
    // A retyped node keeps the SyncTeX stamp it was made with.
    UnsetNode.sync == BoxNode.sync,
    AlignRecord.sync == UnsetNode.sync,
    MathNode.sync == KernNode.sync,
    GlueNode.sync == KernNode.sync,
    // Shared heads read before the kind is known.
    Noad.link == AnyNoad.link,
    Noad.new_hlist == AnyNoad.new_hlist,
    FractionNoad.new_hlist == AnyNoad.new_hlist,
    FenceNoad.new_hlist == AnyNoad.new_hlist,
    FractionNoad.link == AnyNoad.link,
    FenceNoad.link == AnyNoad.link,
    Noad.noad_type == AnyNoad.noad_type,
    FractionNoad.noad_type == AnyNoad.noad_type,
    FenceNoad.noad_type == AnyNoad.noad_type,
);

impl BoxNode {
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
    use crate::base::units::MemIdx;
    use crate::memory::node::{View, WhatsitVariant};

    fn arena() -> MemState {
        MemState {
            data: vec![Default::default(); 32],
            ..Default::default()
        }
    }

    #[test]
    fn an_upwards_box_is_reversed() {
        let mut mem = arena();
        let b: NodePtr<BoxNode> = NodePtr::from_idx_unchecked(MemIdx::from_raw(4).unwrap());
        mem[b].set_lr(BoxLr::Dlist);
        assert!(!mem[b].upwards());
        mem[b].set_upwards(true);
        assert!(mem[b].upwards() && mem[b].lr() == BoxLr::Reversed);
        mem[b].set_upwards(false);
        assert!(!mem[b].upwards() && mem[b].lr() == BoxLr::Normal);
    }

    #[test]
    fn a_mutable_borrow_journals_every_word_of_the_node() {
        let mut mem = arena();
        let k: NodePtr<KernNode> = NodePtr::from_idx_unchecked(MemIdx::from_raw(4).unwrap());
        mem[k].sync.line = 5;
        mem.journal.arm(16);
        let kern = &mut mem[k];
        kern.link = MemIdx::from_raw(1);
        kern.width = Scaled::from_raw(2);
        kern.sync.tag = 3;
        kern.sync.line = 4;
        let delta = mem.journal.seal(&mem.data);
        delta.apply_to(&mut mem.data);
        let kern = mem[k];
        assert_eq!(
            (kern.link, kern.width, kern.sync.tag, kern.sync.line),
            (MemIdx::from_raw(0), Scaled::ZERO, 0, 5)
        );
    }

    #[test]
    fn glue_specs_are_journaled_in_their_store() {
        let mut mem = arena();
        let s = mem.glue.alloc().unwrap();
        mem[s].shrink = Scaled::from_raw(5);
        mem.glue.journal.arm(mem.glue.specs().len());
        mem[s].width = Scaled::from_raw(2);
        mem[s].shrink = Scaled::from_raw(4);
        let delta = mem.glue.seal_journal();
        delta.apply_to(mem.glue.specs_mut());
        assert_eq!(
            (mem[s].width, mem[s].shrink),
            (Scaled::ZERO, Scaled::from_raw(5))
        );
    }

    /// The `n` words after the struct of `p`.
    fn tail<K: View>(mem: &MemState, p: NodePtr<K>, n: usize) -> &[u32] {
        let start = p.idx().words(K::SIZE).end;
        &mem.data[start..start + n]
    }

    #[test]
    fn par_shape_lines_follow_the_header() {
        let mut mem = arena();
        let s = mem.alloc_array::<ParShapeNode>(2).unwrap();
        assert_eq!(mem[s].lines, 2);
        let lines = mem.entries_mut(s);
        assert_eq!(lines.len(), 2);
        lines[0].indent = Scaled::from_raw(11);
        lines[0].width = Scaled::from_raw(12);
        lines[1].indent = Scaled::from_raw(13);
        lines[1].width = Scaled::from_raw(14);
        assert_eq!(tail(&mem, s, 4), [11, 12, 13, 14]);
        assert_eq!(s.line(&mem, 2).width, Scaled::from_raw(14));
    }

    #[test]
    fn penalties_follow_the_header() {
        let mut mem = arena();
        let p = mem.alloc_array::<PenaltiesNode>(3).unwrap();
        assert_eq!(mem[p].half_size, 2);
        mem[p].count = 2;
        let entries = mem.entries_mut(p);
        assert_eq!(entries.len(), 3);
        entries[0].value = 11;
        entries[1].value = 12;
        assert_eq!(tail(&mem, p, 3), [11, 12, 0]);
        assert_eq!([0, 1, 2].map(|k| p.penalty(&mem, k)), [2, 11, 12]);
    }

    #[test]
    fn pseudo_lines_follow_the_header() {
        let mut mem = arena();
        let p = mem.alloc_array::<PseudoLineNode>(2).unwrap();
        assert_eq!(mem[p].size, 3);
        let words = mem.entries_mut(p);
        assert_eq!(words.len(), 2);
        words[1] = PseudoWord::new([1, 2, 3, 4]);
        assert_eq!(tail(&mem, p, 4), [0, 0, 4 | 3 << 16, 2 | 1 << 16]);
        assert_eq!(mem.entries(p)[1].chars(), [1, 2, 3, 4]);
    }

    #[test]
    fn a_shape_view_frees_a_penalty_array() {
        let mut mem = arena();
        let p = mem.alloc_array::<PenaltiesNode>(5).unwrap();
        mem.free_shape_ref(Some(p.idx()));
        let again = mem.alloc_array::<PenaltiesNode>(5).unwrap();
        assert_eq!(again, p);
        let len = mem.data.len();
        mem.free_array(again);
        let shape = mem.alloc_array::<ParShapeNode>(3).unwrap();
        assert_eq!((shape.idx(), mem.data.len()), (p.idx(), len));
    }

    #[test]
    fn a_reused_array_starts_zeroed() {
        let mut mem = arena();
        let s = mem.alloc_array::<ParShapeNode>(2).unwrap();
        mem.entries_mut(s)[1].width = Scaled::from_raw(9);
        mem.free_array(s);
        let again = mem.alloc_array::<ParShapeNode>(2).unwrap();
        assert_eq!(again, s);
        assert_eq!(tail(&mem, again, 4), [0; 4]);
    }

    #[test]
    fn large_arrays_share_the_free_list_of_their_rounded_size() {
        let mut mem = arena();
        let before = mem.data.len();
        let s = mem.alloc_array::<ParShapeNode>(100).unwrap();
        assert_eq!(mem.data.len() - before, 256);
        mem.free_array(s);
        let again = mem.alloc_array::<ParShapeNode>(120).unwrap();
        assert_eq!((again, mem.data.len() - before), (s, 256));
        let larger = mem.alloc_array::<ParShapeNode>(128).unwrap();
        assert_eq!(mem.data.len() - before, 256 + 512);
        assert_ne!(larger, again);
    }

    #[test]
    fn a_whatsit_subtype_selects_its_variant() {
        let mut mem = arena();
        let at = MemIdx::from_raw;
        let o: NodePtr<OpenNode> = NodePtr::from_idx_unchecked(at(1).unwrap());
        mem[o].subtype = WhatsitSubtype::Open.into();
        assert!(matches!(o.whatsit().variant(&mem), Some(WhatsitVariant::Open(p)) if p == o));
        let w: NodePtr<WriteNode> = NodePtr::from_idx_unchecked(at(5).unwrap());
        mem[w].subtype = WhatsitSubtype::Special.into();
        assert!(matches!(w.whatsit().variant(&mem), Some(WhatsitVariant::Special(p)) if p == w));
        let l: NodePtr<LanguageNode> = NodePtr::from_idx_unchecked(at(9).unwrap());
        mem[l].subtype = WhatsitSubtype::Language.into();
        assert!(matches!(l.whatsit().variant(&mem), Some(WhatsitVariant::Language(p)) if p == l));
        let g: NodePtr<GlyphNode> = NodePtr::from_idx_unchecked(at(13).unwrap());
        mem[g].subtype = WhatsitSubtype::Glyph.into();
        assert!(matches!(g.whatsit().variant(&mem), Some(WhatsitVariant::Glyph(p)) if p == g));
    }

    #[test]
    fn a_pic_path_follows_the_header() {
        let mut mem = arena();
        let p = mem.alloc_array::<PicNode>(10).unwrap();
        mem[p].subtype = WhatsitSubtype::Pdf.into();
        assert_eq!(mem[p].path_len, 10);
        mem.entries_mut(p).copy_from_slice(b"0123456789");
        assert_eq!(mem.entries(p), b"0123456789");
        assert!(matches!(p.whatsit().variant(&mem), Some(WhatsitVariant::Pic(q)) if q == p));
    }

    fn native_word(mem: &mut MemState, text: &[u16]) -> NodePtr<NativeWordNode> {
        let w = mem.alloc_array::<NativeWordNode>(text.len()).unwrap();
        mem[w].subtype = WhatsitSubtype::NativeWord.into();
        mem.entries_mut(w).copy_from_slice(text);
        w
    }

    #[test]
    fn clearing_glyphs_keeps_the_native_text() {
        let mut mem = arena();
        let w = native_word(&mut mem, &[1, 2, 3, 4, 5]);
        mem[w].glyph_count = 6;
        mem[w].glyph_info = 9;
        mem[w].clear_glyphs();
        assert_eq!((mem[w].glyph_count, mem[w].glyph_info), (0, 0));
        assert_eq!(mem.entries(w), [1, 2, 3, 4, 5]);
        assert!(matches!(w.whatsit().variant(&mem), Some(WhatsitVariant::NativeWord(p)) if p == w));
    }

    #[test]
    fn entries_of_two_nodes_copy_into_the_destination() {
        let mut mem = arena();
        let src = native_word(&mut mem, &[1, 2, 3]);
        let dst = native_word(&mut mem, &[0; 5]);
        let Ok((to, from)) = mem.entries_mut_and(dst, src) else {
            panic!("two separate nodes overlap");
        };
        to[1..4].copy_from_slice(from);
        assert_eq!(mem.entries(dst), [0, 1, 2, 3, 0]);
        assert_eq!(mem.entries(src), [1, 2, 3]);
        assert!(mem.entries_mut_and(dst, dst).is_err());
    }

    #[test]
    fn a_copied_array_writes_its_own_entries() {
        let mut mem = arena();
        let s = mem.alloc_array::<ParShapeNode>(1).unwrap();
        mem.entries_mut(s)[0].width = Scaled::from_raw(5);
        let copy = mem.copy_array(s).unwrap();
        assert_eq!(mem.entries(copy)[0].width, Scaled::from_raw(5));
        mem.entries_mut(copy)[0].width = Scaled::from_raw(6);
        assert_eq!(mem.entries(s)[0].width, Scaled::from_raw(5));
        mem.free_array(s);
        assert_eq!(mem.entries(copy)[0].width, Scaled::from_raw(6));
    }

    #[test]
    fn a_shortened_native_word_is_freed_and_copied_by_its_own_size() {
        let mut mem = arena();
        let w = native_word(&mut mem, &[1, 2, 3, 4, 5, 6, 7, 8, 9]);
        mem[w].length = 1;
        let copy = mem.copy_array(w).unwrap();
        assert_eq!(mem.entries(copy), [1]);
        mem.free_array(copy);
        assert_eq!(native_word(&mut mem, &[7, 7]), copy);
        mem.free_array(w);
        assert_eq!(native_word(&mut mem, &[0; 9]), w);
    }

    #[test]
    fn native_words_and_glyphs_have_dims_and_other_whatsits_none() {
        let mut mem = arena();
        let at = MemIdx::from_raw;
        let w = native_word(&mut mem, &[]);
        (mem[w].width, mem[w].depth, mem[w].height) = (
            Scaled::from_raw(1),
            Scaled::from_raw(2),
            Scaled::from_raw(3),
        );
        let g_at = 2 + NativeWordNode::SIZE;
        let g: NodePtr<GlyphNode> = NodePtr::from_idx_unchecked(at(g_at).unwrap());
        mem[g].subtype = WhatsitSubtype::Glyph.into();
        (mem[g].width, mem[g].depth, mem[g].height) = (
            Scaled::from_raw(4),
            Scaled::from_raw(5),
            Scaled::from_raw(6),
        );
        let l: NodePtr<LanguageNode> =
            NodePtr::from_idx_unchecked(at(g_at + GlyphNode::SIZE).unwrap());
        mem[l].subtype = WhatsitSubtype::Language.into();
        let dims = |width, depth, height| Dims {
            width: Scaled::from_raw(width),
            depth: Scaled::from_raw(depth),
            height: Scaled::from_raw(height),
        };
        assert_eq!(w.whatsit().dims(&mem), Some(dims(1, 2, 3)));
        assert_eq!(g.whatsit().dims(&mem), Some(dims(4, 5, 6)));
        assert_eq!(l.whatsit().dims(&mem), None);
    }

    #[test]
    fn a_negative_size_leaves_no_entries() {
        let mut mem = arena();
        let s = mem.alloc_array::<ParShapeNode>(1).unwrap();
        mem[s].lines = -3;
        assert!(mem.entries(s).is_empty());
    }

    #[test]
    fn a_checkpoint_restores_the_entries_it_saw() {
        let mut mem = arena();
        let s = mem.alloc_array::<ParShapeNode>(2).unwrap();
        let t = mem.alloc_array::<ParShapeNode>(1).unwrap();
        mem.entries_mut(s)[1].width = Scaled::from_raw(5);
        mem.journal.arm(mem.data.len());
        let lines = mem.entries_mut(s);
        lines[0].indent = Scaled::from_raw(1);
        lines[1].width = Scaled::from_raw(2);
        let (to, from) = mem.entries_mut_and(t, s).unwrap();
        to[0].width = from[1].width;
        assert_eq!(mem.entries(t)[0].width, Scaled::from_raw(2));
        let delta = mem.journal.seal(&mem.data);
        delta.apply_to(&mut mem.data);
        let lines = mem.entries(s);
        assert_eq!(
            (lines[0].indent, lines[1].width, mem.entries(t)[0].width),
            (Scaled::ZERO, Scaled::from_raw(5), Scaled::ZERO)
        );
    }
}
