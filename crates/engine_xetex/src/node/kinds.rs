//! The node kinds, each a struct laid over its words in `mem`.
//!
//! Indexing `mem` with a [`NodePtr`] borrows the node as its struct, and a
//! mutable borrow first notes the node's words in the journal.

use crate::errors::Abort;
use crate::errors::OrConfusion;
use crate::fonts::Side;
use crate::glue_store::GlueSpecId;
use crate::math::MathStyle;
use bytemuck::{Pod, Zeroable};

use super::{
    AdjustPlacement, BoxLr, FenceSubtype, GlueKind, KernSubtype, LeaderKind, LigBoundary, Link,
    MathAccentKind, MathCode, Node, NodePtr, SaChild, SaLeaf, Stored, StoredDimen, View,
    WhatsitSubtype,
};
use crate::codes::{LimitCode, MarkCode, ValueLevel};
use crate::consts::{EMPTY, NORMAL};
use crate::payload::PayloadId;
use crate::state::MemState;
use crate::token::TokenList;
use crate::units::{
    Dir, FontId, GlueOrder, GlueSign, MemPtr, NodeType, Orientation, Scaled, StrNumber,
};

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

impl<K: View> std::ops::Index<NodePtr<K>> for MemState {
    type Output = K;

    #[inline]
    fn index(&self, p: NodePtr<K>) -> &K {
        bytemuck::from_bytes(bytemuck::cast_slice(&self.data[p.idx().words(K::SIZE)]))
    }
}

impl<K: View> std::ops::IndexMut<NodePtr<K>> for MemState {
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

/// A node whose header says how many entries of `Entry` its payload holds.
/// Entries may be smaller than a word.
pub trait ArrayNode: Node {
    type Entry: Pod;
    /// The number of entries.
    fn len(&self) -> i32;
    fn payload(&self) -> PayloadId;
    fn set_payload(&mut self, id: PayloadId);
}

pub type EntriesMutAnd<'a, K, L> = (
    &'a mut [<K as ArrayNode>::Entry],
    &'a [<L as ArrayNode>::Entry],
);

impl MemState {
    /// The byte length of `p`'s entries.
    #[inline]
    fn entry_bytes<K: ArrayNode>(&self, p: NodePtr<K>) -> usize {
        let len = usize::try_from(self[p].len()).unwrap_or(0);
        len.saturating_mul(std::mem::size_of::<K::Entry>())
    }

    /// Gives the fresh array node `p` a zeroed payload for `entries` entries.
    pub fn alloc_payload<K: ArrayNode>(
        &mut self,
        p: NodePtr<K>,
        entries: usize,
    ) -> Result<(), crate::errors::Abort> {
        let bytes = entries.saturating_mul(std::mem::size_of::<K::Entry>());
        let id = self
            .payloads
            .insert(bytes.div_ceil(std::mem::size_of::<u32>()))?;
        self[p].set_payload(id);
        Ok(())
    }

    /// Gives the copy `dst` of array node `src` its own handle on the payload.
    pub fn share_payload<K: ArrayNode>(
        &mut self,
        dst: NodePtr<K>,
        src: NodePtr<K>,
    ) -> Result<(), crate::errors::Abort> {
        let id = self.payloads.share(self[src].payload())?;
        self[dst].set_payload(id);
        Ok(())
    }

    /// Drops the payload of array node `p`.
    pub fn release_payload<K: ArrayNode>(&mut self, p: NodePtr<K>) {
        let id = self[p].payload();
        self.payloads.release(id);
        K::set_payload(&mut self[p], PayloadId::NONE);
    }

    /// The entries of an array node.
    #[inline]
    pub fn entries<K: ArrayNode>(&self, p: NodePtr<K>) -> &[K::Entry] {
        let bytes = self.entry_bytes(p);
        let words: &[u8] = bytemuck::cast_slice(self.payloads.get(self[p].payload()));
        bytemuck::cast_slice(words.get(..bytes).unwrap_or(&[]))
    }

    /// The entries of `dst`, writable, beside those of another node `src`. A confusion when
    /// both share one payload.
    #[inline]
    pub fn entries_mut_and<K: ArrayNode, L: ArrayNode>(
        &mut self,
        dst: NodePtr<K>,
        src: NodePtr<L>,
    ) -> Result<EntriesMutAnd<'_, K, L>, crate::errors::Abort> {
        let (dst_bytes, src_bytes) = (self.entry_bytes(dst), self.entry_bytes(src));
        let (dst_id, src_id) = (self[dst].payload(), self[src].payload());
        let (dst, src) = self
            .payloads
            .get_mut_and(dst_id, src_id)
            .or_confusion("entries")?;
        let dst: &mut [u8] = bytemuck::cast_slice_mut(dst);
        let src: &[u8] = bytemuck::cast_slice(src);
        Ok((
            bytemuck::cast_slice_mut(dst.get_mut(..dst_bytes).or_confusion("entries")?),
            bytemuck::cast_slice(src.get(..src_bytes).or_confusion("entries")?),
        ))
    }

    /// The entries of an array node, writable.
    #[inline]
    pub fn entries_mut<K: ArrayNode>(&mut self, p: NodePtr<K>) -> &mut [K::Entry] {
        let bytes = self.entry_bytes(p);
        let id = self[p].payload();
        let words: &mut [u8] = bytemuck::cast_slice_mut(self.payloads.get_mut(id));
        let len = bytes.min(words.len());
        bytemuck::cast_slice_mut(&mut words[..len])
    }
}

/// The header of a `\parshape` array.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ParShapeNode {
    pub lines: i32,
    pub payload: PayloadId,
    /// Pads the node to the size of a penalty array, which is freed through this view.
    _unused: i32,
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
}

impl ArrayNode for ParShapeNode {
    type Entry = ParShapeLine;

    #[inline]
    fn len(&self) -> i32 {
        self.lines
    }

    #[inline]
    fn payload(&self) -> PayloadId {
        self.payload
    }

    #[inline]
    fn set_payload(&mut self, id: PayloadId) {
        self.payload = id;
    }
}

/// The header of an e-TeX penalty array.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PenaltiesNode {
    /// Half the entry count, rounded up, as e-TeX's node size was.
    pub half_size: i32,
    pub payload: PayloadId,
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
}

impl ArrayNode for PenaltiesNode {
    type Entry = PenaltyEntry;

    #[inline]
    fn len(&self) -> i32 {
        2 * self.half_size - 1
    }

    #[inline]
    fn payload(&self) -> PayloadId {
        self.payload
    }

    #[inline]
    fn set_payload(&mut self, id: PayloadId) {
        self.payload = id;
    }
}

/// The header of a `\scantokens` pseudo-file line.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PseudoLineNode {
    /// One more than the line's words of four characters, as e-TeX's node size was.
    pub size: i32,
    pub link: Link<PseudoLineNode>,
    pub payload: PayloadId,
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
}

impl ArrayNode for PseudoLineNode {
    type Entry = PseudoWord;

    #[inline]
    fn len(&self) -> i32 {
        self.size - 1
    }

    #[inline]
    fn payload(&self) -> PayloadId {
        self.payload
    }

    #[inline]
    fn set_payload(&mut self, id: PayloadId) {
        self.payload = id;
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
    pub path: PayloadId,
    /// `[a, b, c, d, x, y]`.
    pub transform: [Scaled; 6],
    _unused5: u16,
    pub pagebox: u16,
}

impl View for PicNode {}

impl Node for PicNode {
    const TYPE: Option<NodeType> = Some(NodeType::Whatsit);
}

impl ArrayNode for PicNode {
    type Entry = u8;

    #[inline]
    fn len(&self) -> i32 {
        i32::from(self.path_len)
    }

    #[inline]
    fn payload(&self) -> PayloadId {
        self.path
    }

    #[inline]
    fn set_payload(&mut self, id: PayloadId) {
        self.path = id;
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
    _unused4: u16,
    /// One-based shaped-glyph arena handle, zero when unshaped.
    pub glyph_info: i32,
    pub text: PayloadId,
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
}

impl ArrayNode for NativeWordNode {
    type Entry = u16;

    #[inline]
    fn len(&self) -> i32 {
        i32::from(self.length)
    }

    #[inline]
    fn payload(&self) -> PayloadId {
        self.text
    }

    #[inline]
    fn set_payload(&mut self, id: PayloadId) {
        self.text = id;
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

impl std::fmt::Debug for GlueRatio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
        MemPtr::from_raw(bytemuck::cast([self.character, self.plane_and_fam]))
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
        self.set_info(MemPtr::NULL);
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
        self.new_hlist = MemPtr::NULL;
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

/// An eTeX sparse-array index node.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SaIndexNode {
    /// `sa_used`, the count of non-null children.
    pub used: u16,
    /// The node's slot in its parent, or the tree's type at the root.
    pub sa_index: u16,
    pub link: Link<SaIndexNode>,
    /// Index nodes above the last level, elements on it.
    children: [MemPtr; 64],
}

impl View for SaIndexNode {}

impl Node for SaIndexNode {
    const TYPE: Option<NodeType> = None;
}

impl SaIndexNode {
    /// Child `i` of a node above the last index level.
    #[inline]
    pub fn child_index(&self, i: usize) -> Option<NodePtr<SaIndexNode>> {
        self.children[i].as_idx().map(NodePtr::from_idx_unchecked)
    }
    /// Child `i` of a node on the last index level.
    #[inline]
    pub fn child_element(&self, i: usize) -> Option<NodePtr<SaElement>> {
        self.children[i].as_idx().map(NodePtr::from_idx_unchecked)
    }
    /// Child `i` of a node on the last index level of the tree for type `t`,
    /// whose elements all have the kind `t` gives.
    #[inline]
    pub fn child_leaf(&self, i: usize, t: ValueLevel) -> Option<SaLeaf> {
        self.child_element(i).map(|e| SaLeaf::of_type(t, e.idx()))
    }
    #[inline]
    pub fn set_child<K: SaChild>(&mut self, i: usize, p: Option<NodePtr<K>>) {
        self.children[i] = crate::node::link_of(p);
    }
    #[inline]
    pub fn clear_child(&mut self, i: usize) {
        self.children[i] = MemPtr::NULL;
    }
    #[inline]
    pub fn clear_children(&mut self) {
        self.children = [MemPtr::NULL; 64];
    }
}

/// The header every sparse-array element shares.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SaElement {
    pub sa_lev: u16,
    /// The type times 64 plus the slot in the parent.
    pub sa_index: u16,
    pub link: NodePtr<SaIndexNode>,
}

impl View for SaElement {}

impl SaElement {
    #[inline]
    pub fn sa_type(&self) -> Stored<ValueLevel> {
        Stored::from_raw(self.sa_index / 64)
    }
    /// The element's slot in its parent.
    #[inline]
    pub fn slot(&self) -> usize {
        usize::from(self.sa_index % 64)
    }
}

/// A count or dimension register.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SaWordNode {
    /// The grouping level of the value.
    pub sa_lev: u16,
    pub sa_index: u16,
    pub link: NodePtr<SaIndexNode>,
    /// References beyond the tree, `TEX_NULL` for none.
    pub sa_ref: i32,
    /// The register number.
    pub sa_num: i32,
    /// The count, or the dimension's raw value.
    pub sa_int: i32,
}

impl View for SaWordNode {}

impl Node for SaWordNode {
    const TYPE: Option<NodeType> = None;
}

/// A glue, muglue, box or token-list register.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SaPointerNode {
    /// The grouping level of the value.
    pub sa_lev: u16,
    pub sa_index: u16,
    pub link: NodePtr<SaIndexNode>,
    /// References beyond the tree, `TEX_NULL` for none.
    pub sa_ref: i32,
    /// A glue spec, box or token list by the element's type.
    pub sa_ptr: MemPtr,
}

impl View for SaPointerNode {}

impl Node for SaPointerNode {
    const TYPE: Option<NodeType> = None;
}

impl SaPointerNode {
    /// The value of a token-list register.
    #[inline]
    pub fn toks(&self) -> TokenList {
        TokenList::from_raw(self.sa_ptr.raw())
    }
}

/// The marks of one mark class.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SaMarkClassNode {
    pub sa_lev: u16,
    pub sa_index: u16,
    pub link: NodePtr<SaIndexNode>,
    marks: [TokenList; 6],
}

impl View for SaMarkClassNode {}

impl Node for SaMarkClassNode {
    const TYPE: Option<NodeType> = None;
}

impl SaMarkClassNode {
    #[inline]
    pub fn mark(&self, m: MarkCode) -> TokenList {
        self.marks[m as usize]
    }
    #[inline]
    pub fn set_mark(&mut self, m: MarkCode, v: TokenList) {
        self.marks[m as usize] = v;
    }
    #[inline]
    pub fn clear_marks(&mut self) {
        self.marks = [TokenList::NULL; 6];
    }
}

/// The header every save-chain entry shares.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SaSaved {
    /// The saved grouping level.
    pub sa_lev: u16,
    pub sa_index: u16,
    pub link: Link<SaSaved>,
}

impl View for SaSaved {}

/// A saved count or dimension.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SaSavedWord {
    pub sa_lev: u16,
    pub sa_index: u16,
    pub link: Link<SaSaved>,
    /// The register the entry restores.
    pub sa_loc: NodePtr<SaWordNode>,
    pub sa_int: i32,
}

impl View for SaSavedWord {}

impl Node for SaSavedWord {
    const TYPE: Option<NodeType> = None;
}

/// A saved glue spec, box or token list.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SaSavedPointer {
    pub sa_lev: u16,
    pub sa_index: u16,
    pub link: Link<SaSaved>,
    /// The register the entry restores.
    pub sa_loc: NodePtr<SaPointerNode>,
    pub sa_ptr: MemPtr,
}

impl View for SaSavedPointer {}

impl Node for SaSavedPointer {
    const TYPE: Option<NodeType> = None;
}

/// An entry of a column's span list, ended by `END_SPAN`.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SpanNode {
    pub next: NodePtr<SpanNode>,
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
    /// The head of the span list, read through `span_head`.
    _span_list: i32,
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
    pub box_number: u16,
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
    pub box_number: u16,
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
    pub lines: Link<PseudoLineNode>,
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
    AlignRecord._span_list == SpanNode.next,
    AlignRecord.width == SpanNode.width,
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
    SaWordNode.sa_index == SaElement.sa_index,
    SaPointerNode.sa_index == SaElement.sa_index,
    SaMarkClassNode.sa_index == SaElement.sa_index,
    SaWordNode.link == SaElement.link,
    SaPointerNode.link == SaElement.link,
    SaMarkClassNode.link == SaElement.link,
    SaSavedWord.sa_index == SaSaved.sa_index,
    SaSavedPointer.sa_index == SaSaved.sa_index,
    SaSavedWord.link == SaSaved.link,
    SaSavedPointer.link == SaSaved.link,
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
    use crate::node::View;
    use crate::node::WhatsitVariant;
    use crate::units::MemIdx;

    fn arena() -> MemState {
        MemState {
            data: vec![Default::default(); 32],
            ..Default::default()
        }
    }

    #[test]
    fn a_negative_index_selects_no_words() {
        let mem = arena();
        assert!(mem.data.get(MemIdx::from_raw(-1).words(2)).is_none());
        assert_eq!(MemIdx::from_raw(3).words(2), 3..5);
    }

    #[test]
    fn an_upwards_box_is_reversed() {
        let mut mem = arena();
        let b: NodePtr<BoxNode> = NodePtr::from_idx_unchecked(MemIdx::from_raw(4));
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
        let k: NodePtr<KernNode> = NodePtr::from_idx_unchecked(MemIdx::from_raw(4));
        mem[k].sync.line = 5;
        mem.journal.arm(16);
        let kern = &mut mem[k];
        kern.link = MemPtr::from_raw(1);
        kern.width = Scaled::from_raw(2);
        kern.sync.tag = 3;
        kern.sync.line = 4;
        let delta = mem.journal.seal(&mem.data);
        delta.apply_to(&mut mem.data);
        let kern = mem[k];
        assert_eq!(
            (kern.link, kern.width, kern.sync.tag, kern.sync.line),
            (MemPtr::from_raw(0), Scaled::ZERO, 0, 5)
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

    fn shape_at(idx: i32) -> NodePtr<ParShapeNode> {
        NodePtr::from_idx_unchecked(MemIdx::from_raw(idx))
    }

    #[test]
    fn par_shape_lines_live_in_the_payload() {
        let mut mem = arena();
        let s = shape_at(4);
        mem[s].lines = 2;
        mem.alloc_payload(s, 2).unwrap();
        let lines = mem.entries_mut(s);
        assert_eq!(lines.len(), 2);
        lines[0].indent = Scaled::from_raw(11);
        lines[0].width = Scaled::from_raw(12);
        lines[1].indent = Scaled::from_raw(13);
        lines[1].width = Scaled::from_raw(14);
        assert_eq!(mem.payloads.get(mem[s].payload), [11, 12, 13, 14]);
        assert_eq!(s.line(&mem, 2).width, Scaled::from_raw(14));
    }

    #[test]
    fn penalties_live_in_the_payload() {
        let mut mem = arena();
        let p: NodePtr<PenaltiesNode> = NodePtr::from_idx_unchecked(MemIdx::from_raw(4));
        let header = &mut mem[p];
        header.half_size = 2;
        header.count = 2;
        mem.alloc_payload(p, 3).unwrap();
        let entries = mem.entries_mut(p);
        assert_eq!(entries.len(), 3);
        entries[0].value = 11;
        entries[1].value = 12;
        assert_eq!(mem.payloads.get(mem[p].payload), [11, 12, 0]);
        assert_eq!([0, 1, 2].map(|k| p.penalty(&mem, k)), [2, 11, 12]);
    }

    #[test]
    fn pseudo_lines_live_in_the_payload() {
        let mut mem = arena();
        let p: NodePtr<PseudoLineNode> = NodePtr::from_idx_unchecked(MemIdx::from_raw(4));
        let header = &mut mem[p];
        header.size = 3;
        header.link = Some(NodePtr::from_idx_unchecked(MemIdx::from_raw(9))).into();
        mem.alloc_payload(p, 2).unwrap();
        let words = mem.entries_mut(p);
        assert_eq!(words.len(), 2);
        words[1] = PseudoWord::new([1, 2, 3, 4]);
        assert_eq!(
            mem.payloads.get(mem[p].payload),
            [0, 0, 4 | 3 << 16, 2 | 1 << 16]
        );
        assert_eq!(mem.entries(p)[1].chars(), [1, 2, 3, 4]);
    }

    #[test]
    fn a_whatsit_subtype_selects_its_variant() {
        let mut mem = arena();
        let at = MemIdx::from_raw;
        let o: NodePtr<OpenNode> = NodePtr::from_idx_unchecked(at(0));
        mem[o].subtype = WhatsitSubtype::Open.into();
        assert!(matches!(o.whatsit().variant(&mem), Some(WhatsitVariant::Open(p)) if p == o));
        let w: NodePtr<WriteNode> = NodePtr::from_idx_unchecked(at(4));
        mem[w].subtype = WhatsitSubtype::Special.into();
        assert!(matches!(w.whatsit().variant(&mem), Some(WhatsitVariant::Special(p)) if p == w));
        let l: NodePtr<LanguageNode> = NodePtr::from_idx_unchecked(at(8));
        mem[l].subtype = WhatsitSubtype::Language.into();
        assert!(matches!(l.whatsit().variant(&mem), Some(WhatsitVariant::Language(p)) if p == l));
        let g: NodePtr<GlyphNode> = NodePtr::from_idx_unchecked(at(12));
        mem[g].subtype = WhatsitSubtype::Glyph.into();
        assert!(matches!(g.whatsit().variant(&mem), Some(WhatsitVariant::Glyph(p)) if p == g));
    }

    #[test]
    fn a_pic_path_lives_in_the_payload() {
        let mut mem = MemState {
            data: vec![Default::default(); 24],
            ..Default::default()
        };
        let p: NodePtr<PicNode> = NodePtr::from_idx_unchecked(MemIdx::from_raw(4));
        mem[p].subtype = WhatsitSubtype::Pdf.into();
        mem[p].path_len = 10;
        mem.alloc_payload(p, 10).unwrap();
        mem.entries_mut(p).copy_from_slice(b"0123456789");
        assert_eq!(mem.entries(p), b"0123456789");
        assert!(matches!(p.whatsit().variant(&mem), Some(WhatsitVariant::Pic(q)) if q == p));
    }

    fn native_word_at(mem: &mut MemState, idx: i32, text: &[u16]) -> NodePtr<NativeWordNode> {
        let w: NodePtr<NativeWordNode> = NodePtr::from_idx_unchecked(MemIdx::from_raw(idx));
        let word = &mut mem[w];
        word.subtype = WhatsitSubtype::NativeWord.into();
        word.length = text.len() as u16;
        mem.alloc_payload(w, text.len()).unwrap();
        mem.entries_mut(w).copy_from_slice(text);
        w
    }

    #[test]
    fn clearing_glyphs_keeps_the_native_text() {
        let mut mem = MemState {
            data: vec![Default::default(); 24],
            ..Default::default()
        };
        let w = native_word_at(&mut mem, 4, &[1, 2, 3, 4, 5]);
        mem[w].glyph_count = 6;
        mem[w].glyph_info = 9;
        mem[w].clear_glyphs();
        assert_eq!((mem[w].glyph_count, mem[w].glyph_info), (0, 0));
        assert_eq!(mem.entries(w), [1, 2, 3, 4, 5]);
        assert!(matches!(w.whatsit().variant(&mem), Some(WhatsitVariant::NativeWord(p)) if p == w));
    }

    #[test]
    fn entries_of_two_nodes_copy_into_the_destination() {
        let mut mem = MemState {
            data: vec![Default::default(); 24],
            ..Default::default()
        };
        let src = native_word_at(&mut mem, 2, &[1, 2, 3]);
        let dst = native_word_at(&mut mem, 2 + NativeWordNode::SIZE, &[0; 5]);
        let Ok((to, from)) = mem.entries_mut_and(dst, src) else {
            panic!("two separate payloads overlap");
        };
        to[1..4].copy_from_slice(from);
        assert_eq!(mem.entries(dst), [0, 1, 2, 3, 0]);
        assert_eq!(mem.entries(src), [1, 2, 3]);
        assert!(mem.entries_mut_and(dst, dst).is_err());
    }

    #[test]
    fn a_copied_node_writes_its_own_payload() {
        let mut mem = arena();
        let (s, copy) = (shape_at(2), shape_at(4));
        mem[s].lines = 1;
        mem.alloc_payload(s, 1).unwrap();
        mem.entries_mut(s)[0].width = Scaled::from_raw(5);
        mem[copy] = mem[s];
        mem.share_payload(copy, s).unwrap();
        mem.entries_mut(copy)[0].width = Scaled::from_raw(6);
        assert_eq!(mem.entries(s)[0].width, Scaled::from_raw(5));
        assert_eq!(mem.entries(copy)[0].width, Scaled::from_raw(6));
        mem.release_payload(s);
        assert_eq!(mem.entries(copy)[0].width, Scaled::from_raw(6));
    }

    #[test]
    fn native_words_and_glyphs_have_dims_and_other_whatsits_none() {
        let mut mem = MemState {
            data: vec![Default::default(); 32],
            ..Default::default()
        };
        let at = MemIdx::from_raw;
        let w = native_word_at(&mut mem, 2, &[]);
        (mem[w].width, mem[w].depth, mem[w].height) = (
            Scaled::from_raw(1),
            Scaled::from_raw(2),
            Scaled::from_raw(3),
        );
        let g_at = 2 + NativeWordNode::SIZE;
        let g: NodePtr<GlyphNode> = NodePtr::from_idx_unchecked(at(g_at));
        mem[g].subtype = WhatsitSubtype::Glyph.into();
        (mem[g].width, mem[g].depth, mem[g].height) = (
            Scaled::from_raw(4),
            Scaled::from_raw(5),
            Scaled::from_raw(6),
        );
        let l: NodePtr<LanguageNode> = NodePtr::from_idx_unchecked(at(g_at + GlyphNode::SIZE));
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
        let s = shape_at(4);
        mem[s].lines = -3;
        assert!(mem.entries(s).is_empty());
    }

    #[test]
    fn a_checkpoint_keeps_the_payload_it_saw() {
        let mut mem = arena();
        let s = shape_at(4);
        mem[s].lines = 2;
        mem.alloc_payload(s, 2).unwrap();
        mem.entries_mut(s)[1].width = Scaled::from_raw(5);
        let saved = mem.payloads.clone();
        let lines = mem.entries_mut(s);
        lines[0].indent = Scaled::from_raw(1);
        lines[1].width = Scaled::from_raw(2);
        mem.payloads.restore_from(&saved);
        let lines = mem.entries(s);
        assert_eq!(
            (lines[0].indent, lines[1].width),
            (Scaled::ZERO, Scaled::from_raw(5))
        );
    }
}
