//! Typed views of fixed-size nodes, read and written as `mem[p].field`.
//!
//! A layout struct lies over a node's words in the flat `mem` arena. Indexing
//! `mem` with a [`NodePtr`] borrows the node as its layout, and a mutable
//! borrow first notes the node's words in the journal.

use crate::errors::Abort;
use crate::errors::OrConfusion;
use crate::fonts::Side;
use crate::math::MathStyle;
use bytemuck::{Pod, Zeroable};

use crate::codes::{LimitCode, MarkCode, ValueLevel};
use crate::consts::{EMPTY, NORMAL};
use crate::node::{
    AccentNoad, AdjustNode, AdjustPlacement, AlignRecord, AnyNoad, BoxLr, BoxNode, CharNode,
    ChoiceNode, DelimiterField, DiscNode, EdgeNode, FenceNoad, FenceSubtype, FixedNode,
    FractionNoad, FreeBlock, GlueKind, GlueNode, GlueSpecNode, GlyphNode, HeadNode, HyphPosNode,
    InsNode, KernNode, KernSubtype, LanguageNode, LeaderKind, LigBoundary, LigItem, LigatureNode,
    Link, LrNode, MarginKernNode, MarkNode, MathAccentKind, MathCode, MathField, MathNode,
    MovementNode, NativeWordNode, Noad, NodeKind, NodePtr, OpenNode, PageInsNode, ParShapeNode,
    PenaltiesNode, PenaltyNode, PicNode, PseudoFileNode, PseudoLineNode, RadicalNoad, RuleNode,
    SaChild, SaElement, SaIndexNode, SaLeaf, SaMarkClassNode, SaPointerNode, SaSaved,
    SaSavedPointer, SaSavedWord, SaWordNode, SizedWhatsit, SpanNode, Stored, StoredDimen,
    StyleNode, UnsetNode, WhatsitNode, WhatsitSubtype, WriteNode,
};
use crate::payload::PayloadId;
use crate::state::MemState;
use crate::token::TokenList;
use crate::units::{
    Dir, FontId, GlueOrder, GlueSign, MemPtr, NodeType, Orientation, Scaled, StrNumber,
};

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
pub(crate) const MAX_LAYOUT_WORDS: i32 = crate::consts::INDEX_NODE_SIZE;

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

/// A node kind with a fixed header `Layout` and equal entries, as many as the header says,
/// kept in its payload. Entries may be smaller than a word.
pub trait ArrayOverlay: Overlay {
    /// The struct laid over each entry.
    type Entry: Pod;
    /// The number of entries, read from the header.
    fn len(header: &Self::Layout) -> i32;
    fn payload(header: &Self::Layout) -> PayloadId;
    fn set_payload(header: &mut Self::Layout, id: PayloadId);
}

/// Declares the entry layout of an array node kind, whose header is its
/// `overlay!` layout and holds the payload handle in `$field`.
macro_rules! array_overlay {
    ($kind:ty => $entry:ty, $field:ident, |$header:ident| $len:expr) => {
        impl ArrayOverlay for $kind {
            type Entry = $entry;
            #[inline]
            fn len($header: &<$kind as Overlay>::Layout) -> i32 {
                $len
            }
            #[inline]
            fn payload(header: &<$kind as Overlay>::Layout) -> PayloadId {
                header.$field
            }
            #[inline]
            fn set_payload(header: &mut <$kind as Overlay>::Layout, id: PayloadId) {
                header.$field = id;
            }
        }
        const _: () = assert!(std::mem::align_of::<$entry>() <= std::mem::align_of::<u32>());
    };
}

/// The entries of one array node, writable, beside those of another.
pub type EntriesMutAnd<'a, K, L> = (
    &'a mut [<K as ArrayOverlay>::Entry],
    &'a [<L as ArrayOverlay>::Entry],
);

impl MemState {
    /// The byte length of `p`'s entries.
    #[inline]
    fn entry_bytes<K: ArrayOverlay>(&self, p: NodePtr<K>) -> usize {
        let len = usize::try_from(K::len(&self[p])).unwrap_or(0);
        len.saturating_mul(std::mem::size_of::<K::Entry>())
    }

    /// Gives the fresh array node `p` a zeroed payload for `entries` entries.
    pub fn alloc_payload<K: ArrayOverlay>(
        &mut self,
        p: NodePtr<K>,
        entries: usize,
    ) -> Result<(), crate::errors::Abort> {
        let bytes = entries.saturating_mul(std::mem::size_of::<K::Entry>());
        let id = self
            .payloads
            .insert(bytes.div_ceil(std::mem::size_of::<u32>()))?;
        K::set_payload(&mut self[p], id);
        Ok(())
    }

    /// Gives the copy `dst` of array node `src` its own handle on the payload.
    pub fn share_payload<K: ArrayOverlay>(
        &mut self,
        dst: NodePtr<K>,
        src: NodePtr<K>,
    ) -> Result<(), crate::errors::Abort> {
        let id = self.payloads.share(K::payload(&self[src]))?;
        K::set_payload(&mut self[dst], id);
        Ok(())
    }

    /// Drops the payload of array node `p`.
    pub fn release_payload<K: ArrayOverlay>(&mut self, p: NodePtr<K>) {
        let id = K::payload(&self[p]);
        self.payloads.release(id);
        K::set_payload(&mut self[p], PayloadId::NONE);
    }

    /// The entries of an array node.
    #[inline]
    pub fn entries<K: ArrayOverlay>(&self, p: NodePtr<K>) -> &[K::Entry] {
        let bytes = self.entry_bytes(p);
        let words: &[u8] = bytemuck::cast_slice(self.payloads.get(K::payload(&self[p])));
        bytemuck::cast_slice(words.get(..bytes).unwrap_or(&[]))
    }

    /// The entries of `dst`, writable, beside those of another node `src`. A confusion when
    /// both share one payload.
    #[inline]
    pub fn entries_mut_and<K: ArrayOverlay, L: ArrayOverlay>(
        &mut self,
        dst: NodePtr<K>,
        src: NodePtr<L>,
    ) -> Result<EntriesMutAnd<'_, K, L>, crate::errors::Abort> {
        let (dst_bytes, src_bytes) = (self.entry_bytes(dst), self.entry_bytes(src));
        let (dst_id, src_id) = (K::payload(&self[dst]), L::payload(&self[src]));
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
    pub fn entries_mut<K: ArrayOverlay>(&mut self, p: NodePtr<K>) -> &mut [K::Entry] {
        let bytes = self.entry_bytes(p);
        let id = K::payload(&self[p]);
        let words: &mut [u8] = bytemuck::cast_slice_mut(self.payloads.get_mut(id));
        let len = bytes.min(words.len());
        bytemuck::cast_slice_mut(&mut words[..len])
    }
}

/// The header of a `\parshape` array.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ParShapeHeader {
    pub lines: i32,
    pub payload: PayloadId,
    _unused: [i32; 2],
}

/// One line of a `\parshape` array.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ParShapeLine {
    pub indent: Scaled,
    pub width: Scaled,
}

overlay!(ParShapeNode => ParShapeHeader);
array_overlay!(ParShapeNode => ParShapeLine, payload, |h| h.lines);

/// The header of an e-TeX penalty array.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PenaltiesHeader {
    /// Half the entry count, rounded up, as e-TeX's node size was.
    pub half_size: i32,
    pub payload: PayloadId,
    _unused: i32,
    pub count: i32,
}

/// One entry of an e-TeX penalty array, or the zero padding after an even count.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PenaltyEntry {
    pub value: i32,
}

overlay!(PenaltiesNode => PenaltiesHeader);
array_overlay!(PenaltiesNode => PenaltyEntry, payload, |h| 2 * h.half_size - 1);

/// The header of a `\scantokens` pseudo-file line.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PseudoLineHeader {
    /// One more than the line's words of four characters, as e-TeX's node size was.
    pub size: i32,
    pub link: Link<PseudoLineNode>,
    pub payload: PayloadId,
    _unused: i32,
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

overlay!(PseudoLineNode => PseudoLineHeader);
array_overlay!(PseudoLineNode => PseudoWord, payload, |h| h.size - 1);

/// Any whatsit: its subtype and link.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct WhatsitLayout {
    pub subtype: Stored<WhatsitSubtype>,
    _type: u16,
    pub link: MemPtr,
}

overlay!(WhatsitNode => WhatsitLayout, 1);

/// `\openout`.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct OpenLayout {
    pub subtype: Stored<WhatsitSubtype>,
    _type: u16,
    pub link: MemPtr,
    pub stream: i32,
    pub name: StrNumber,
    pub area: StrNumber,
    pub ext: StrNumber,
}

overlay!(OpenNode => OpenLayout);

/// `\write`, `\closeout` or `\special`. A close has no tokens, a special
/// no stream.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct WriteLayout {
    pub subtype: Stored<WhatsitSubtype>,
    _type: u16,
    pub link: MemPtr,
    pub stream: i32,
    pub tokens: TokenList,
}

overlay!(WriteNode => WriteLayout);

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct LanguageLayout {
    pub subtype: Stored<WhatsitSubtype>,
    _type: u16,
    pub link: MemPtr,
    pub rhm: u16,
    pub lhm: u16,
    pub lang: i32,
}

overlay!(LanguageNode => LanguageLayout);

/// A glyph of a native font. The glyph id sits where a native word keeps
/// its length.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct GlyphLayout {
    pub subtype: Stored<WhatsitSubtype>,
    _type: u16,
    pub link: MemPtr,
    _unused1: i32,
    pub width: Scaled,
    _unused2: i32,
    pub depth: Scaled,
    _unused3: i32,
    pub height: Scaled,
    _glyph_count: u16,
    pub glyph: u16,
    pub font: FontId,
    _size: u16,
}

overlay!(GlyphNode => GlyphLayout);

/// A picture. Its path bytes follow the header.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PicLayout {
    pub subtype: Stored<WhatsitSubtype>,
    _type: u16,
    pub link: MemPtr,
    _unused1: i32,
    pub width: Scaled,
    _unused2: i32,
    pub depth: Scaled,
    _unused3: i32,
    pub height: Scaled,
    pub page: u16,
    pub path_len: u16,
    pub path: PayloadId,
    /// `[a, b, c, d, x, y]`.
    pub transform: [Scaled; 6],
    _unused5: u16,
    pub pagebox: u16,
    _unused6: i32,
}

overlay!(PicNode => PicLayout);
array_overlay!(PicNode => u8, path, |h| i32::from(h.path_len));

/// A native word. Its UTF-16 text follows the header, and its shaped glyphs
/// live in `mem.glyph_blobs` under `glyph_info`.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct NativeWordLayout {
    pub subtype: Stored<WhatsitSubtype>,
    _type: u16,
    pub link: MemPtr,
    _unused1: i32,
    pub width: Scaled,
    _unused2: i32,
    pub depth: Scaled,
    _unused3: i32,
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

impl NativeWordLayout {
    /// Forgets the shaped glyphs, without releasing them.
    #[inline]
    pub fn clear_glyphs(&mut self) {
        self.glyph_count = 0;
        self.glyph_info = 0;
    }
}

overlay!(NativeWordNode => NativeWordLayout);
array_overlay!(NativeWordNode => u16, text, |h| i32::from(h.length));

/// The dimensions a native word, glyph and picture share.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SizedWhatsitLayout {
    pub subtype: Stored<WhatsitSubtype>,
    _type: u16,
    pub link: MemPtr,
    _unused1: i32,
    pub width: Scaled,
    _unused2: i32,
    pub depth: Scaled,
    _unused3: i32,
    pub height: Scaled,
}

overlay!(SizedWhatsit => SizedWhatsitLayout, 4);

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
    subtype: u16,
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

impl BoxLayout {
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
pub struct KernLayout {
    pub subtype: Stored<KernSubtype>,
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
    pub subtype: Stored<MathCode>,
    _type: u16,
    pub link: MemPtr,
    _unused1: i32,
    pub width: Scaled,
    pub sync_tag: i32,
    pub sync_line: i32,
}

overlay!(MathNode => MathLayout);

impl MathLayout {
    /// The node's code. Every writer stores a [`MathCode`], so another value is a broken invariant.
    #[inline]
    pub fn code(&self) -> Result<MathCode, Abort> {
        self.subtype.get().or_confusion("math code")
    }
}

/// A margin kern on the side it protrudes on.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct MarginKernLayout {
    pub side: Stored<Side>,
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
    _subtype: u16,
    _type: u16,
    pub link: MemPtr,
    _unused1: i32,
    pub penalty: i32,
    _unused2: [i32; 2],
}

overlay!(PenaltyNode => PenaltyLayout);

/// A `\vadjust` list.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct AdjustLayout {
    pub placement: Stored<AdjustPlacement>,
    _type: u16,
    pub link: MemPtr,
    _unused1: i32,
    pub adjust_ptr: MemPtr,
}

overlay!(AdjustNode => AdjustLayout);

/// A discretionary.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct DiscLayout {
    pub replace_count: u16,
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
    _subtype: u16,
    _type: u16,
    pub link: MemPtr,
    pub display_mlist: MemPtr,
    pub text_mlist: MemPtr,
    pub script_mlist: MemPtr,
    pub script_script_mlist: MemPtr,
}

overlay!(ChoiceNode => ChoiceLayout);

/// A style change in an mlist.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct StyleLayout {
    pub style: MathStyle,
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

/// An LR edge in a shipped list.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct EdgeLayout {
    pub dir: Stored<Dir>,
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
    _subtype: u16,
    _type: u16,
    pub link: MemPtr,
    pub mark_class: i32,
    pub mark_ptr: TokenList,
}

overlay!(MarkNode => MarkLayout);

/// A noad's nucleus, script or accent word: its math type, then either a
/// character and family or a sub-list in the shared payload half.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct MathWord {
    character: u16,
    plane_and_fam: u16,
    pub math_type: i32,
}

overlay!(MathField => MathWord, 1);

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

overlay!(DelimiterField => DelimiterWord, 1);

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

/// The words every noad shares: the header, then a payload word whose upper
/// half holds the translated hlist once `mlist_to_hlist` has converted the noad.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct NoadHead {
    _subtype: u16,
    pub noad_type: Stored<NodeType>,
    pub link: MemPtr,
    _payload: i32,
    pub new_hlist: MemPtr,
}

overlay!(AnyNoad => NoadHead, 2);

/// A noad with a nucleus and two scripts.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct NoadLayout {
    subtype: u16,
    pub noad_type: Stored<NodeType>,
    pub link: MemPtr,
    pub nucleus: MathWord,
    pub supscr: MathWord,
    pub subscr: MathWord,
}

overlay!(Noad => NoadLayout);

impl NoadLayout {
    /// Empty fields and a normal subtype, as a new noad starts.
    #[inline]
    pub fn clear_fields(&mut self) {
        self.subtype = NORMAL;
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
pub struct RadicalLayout {
    pub base: NoadLayout,
    pub left_delimiter: DelimiterWord,
}

overlay!(RadicalNoad => RadicalLayout);

/// A math accent: a noad, then its accent character. The subtype marks
/// fixed and bottom accents.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct AccentLayout {
    pub base: NoadLayout,
    pub accent_chr: MathWord,
}

impl AccentLayout {
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

overlay!(AccentNoad => AccentLayout);

/// A generalized fraction.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct FractionLayout {
    _subtype: u16,
    pub noad_type: Stored<NodeType>,
    pub link: MemPtr,
    _unused1: i32,
    /// The rule thickness, `DEFAULT_CODE` for the font's.
    pub thickness: Scaled,
    pub numerator: MathWord,
    pub denominator: MathWord,
    pub left_delimiter: DelimiterWord,
    pub right_delimiter: DelimiterWord,
}

overlay!(FractionNoad => FractionLayout);

/// A `\left`, `\middle` or `\right` delimiter. A middle one is a right noad
/// with a middle subtype. WEB leaves the script words empty but reads them.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct FenceLayout {
    pub subtype: Stored<FenceSubtype>,
    pub noad_type: Stored<NodeType>,
    pub link: MemPtr,
    pub delimiter: DelimiterWord,
    pub supscr: MathWord,
    pub subscr: MathWord,
}

overlay!(FenceNoad => FenceLayout);

/// An eTeX sparse-array index node.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SaIndexLayout {
    /// `sa_used`, the count of non-null children.
    pub used: u16,
    /// The node's slot in its parent, or the tree's type at the root.
    pub sa_index: u16,
    pub link: Link<SaIndexNode>,
    /// Index nodes above the last level, elements on it.
    children: [MemPtr; 64],
}

overlay!(SaIndexNode => SaIndexLayout);

impl SaIndexLayout {
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
    /// whose elements all have the layout `t` gives.
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
pub struct SaElementHead {
    pub sa_lev: u16,
    /// The type times 64 plus the slot in the parent.
    pub sa_index: u16,
    pub link: NodePtr<SaIndexNode>,
}

overlay!(SaElement => SaElementHead, 1);

impl SaElementHead {
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
pub struct SaWordLayout {
    /// The grouping level of the value.
    pub sa_lev: u16,
    pub sa_index: u16,
    pub link: NodePtr<SaIndexNode>,
    /// References beyond the tree, `TEX_NULL` for none.
    pub sa_ref: i32,
    /// The register number.
    pub sa_num: i32,
    _unused1: i32,
    /// The count, or the dimension's raw value.
    pub sa_int: i32,
}

overlay!(SaWordNode => SaWordLayout);

/// A glue, muglue, box or token-list register.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SaPointerLayout {
    /// The grouping level of the value.
    pub sa_lev: u16,
    pub sa_index: u16,
    pub link: NodePtr<SaIndexNode>,
    /// References beyond the tree, `TEX_NULL` for none.
    pub sa_ref: i32,
    /// A glue spec, box or token list by the element's type.
    pub sa_ptr: MemPtr,
}

overlay!(SaPointerNode => SaPointerLayout);

impl SaPointerLayout {
    /// The value of a token-list register.
    #[inline]
    pub fn toks(&self) -> TokenList {
        TokenList::from_raw(self.sa_ptr.raw())
    }
}

/// The marks of one mark class.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SaMarkClassLayout {
    pub sa_lev: u16,
    pub sa_index: u16,
    pub link: NodePtr<SaIndexNode>,
    marks: [TokenList; 6],
}

overlay!(SaMarkClassNode => SaMarkClassLayout);

impl SaMarkClassLayout {
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
pub struct SaSavedHead {
    /// The saved grouping level.
    pub sa_lev: u16,
    pub sa_index: u16,
    pub link: Link<SaSaved>,
}

overlay!(SaSaved => SaSavedHead, 1);

/// A saved count or dimension.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SaSavedWordLayout {
    pub sa_lev: u16,
    pub sa_index: u16,
    pub link: Link<SaSaved>,
    /// The register the entry restores.
    pub sa_loc: NodePtr<SaWordNode>,
    _unused1: i32,
    _unused2: i32,
    pub sa_int: i32,
}

overlay!(SaSavedWord => SaSavedWordLayout);

/// A saved glue spec, box or token list.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SaSavedPointerLayout {
    pub sa_lev: u16,
    pub sa_index: u16,
    pub link: Link<SaSaved>,
    /// The register the entry restores.
    pub sa_loc: NodePtr<SaPointerNode>,
    pub sa_ptr: MemPtr,
}

overlay!(SaSavedPointer => SaSavedPointerLayout);

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
    _subtype: u16,
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
    pub subtype: Stored<GlueKind>,
    _type: u16,
    pub link: MemPtr,
    pub glue_ptr: NodePtr<GlueSpecNode>,
    /// The leader box or rule, null for plain glue.
    pub leader_ptr: MemPtr,
    pub sync_tag: i32,
    pub sync_line: i32,
}

overlay!(GlueNode => GlueLayout);

impl GlueLayout {
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
pub struct InsLayout {
    pub box_number: u16,
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
    pub box_number: u16,
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
    pub span_count: u16,
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

/// A character node: the character in the subtype, then its font.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct CharLayout {
    pub character: u16,
    _type: u16,
    pub link: MemPtr,
    pub font: FontId,
    _unused1: u16,
    _unused2: i32,
}

overlay!(CharNode => CharLayout);

/// The head of a list under construction. Only its link is used.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct HeadLayout {
    _subtype: u16,
    _type: u16,
    pub link: MemPtr,
    _unused: [i32; 2],
}

overlay!(HeadNode => HeadLayout);

/// An open `\scantokens` pseudo file: its unread lines and the next file.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PseudoFileLayout {
    pub lines: Link<PseudoLineNode>,
    pub link: MemPtr,
    _unused: [i32; 2],
}

overlay!(PseudoFileNode => PseudoFileLayout);

/// One hyphen position of a `\hyphenation` exception, in a list from the last position.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct HyphPosLayout {
    pub pos: i32,
    pub link: MemPtr,
    _unused: [i32; 2],
}

overlay!(HyphPosNode => HyphPosLayout);

/// An entry of the TeX--XeT LR stack.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct LrLayout {
    pub code: Stored<MathCode>,
    _unused1: u16,
    pub link: MemPtr,
    _unused2: [i32; 2],
}

overlay!(LrNode => LrLayout);

/// A ligature, with the character and font it shows.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct LigatureLayout {
    pub boundary: Stored<LigBoundary>,
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
    use crate::node::WhatsitVariant;
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
    fn a_negative_index_selects_no_words() {
        let mem = arena();
        assert!(mem.data.get(MemIdx::from_raw(-1).words(2)).is_none());
        assert_eq!(MemIdx::from_raw(3).words(2), 3..5);
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
        bx.set_lr(BoxLr::Dlist);
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
        mem[b].set_upwards(true);
        assert!(mem[b].upwards() && mem[b].lr() == BoxLr::Reversed);
        assert_eq!(mem.b16s0(at(4)), 1);
        mem[b].set_upwards(false);
        assert_eq!(mem.b16s0(at(4)), 0);
    }

    #[test]
    fn edge_and_margin_kern_fields_land_on_their_words() {
        let mut mem = arena();
        let at = MemIdx::from_raw;
        let e: NodePtr<EdgeNode> = NodePtr::from_idx_unchecked(at(4));
        let edge = &mut mem[e];
        edge.dir = Dir::Rtl.into();
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

        let m: NodePtr<MovementNode> = NodePtr::from_idx_unchecked(at(8));
        let mv = &mut mem[m];
        (mv.state, mv.link, mv.distance, mv.location) = (5, ptr(6), Scaled::from_raw(31), 32);
        assert_eq!((mem.b32s0(at(8)), mem.b32s1(at(8))), (5, 6));
        assert_eq!((mem.b32s1(at(9)), mem.b32s1(at(10))), (31, 32));
    }

    #[test]
    fn unset_and_page_ins_fields_land_on_their_words() {
        let mut mem = arena();
        let at = MemIdx::from_raw;
        let ptr = MemPtr::from_raw;
        let u: NodePtr<UnsetNode> = NodePtr::from_idx_unchecked(at(4));
        let unset = &mut mem[u];
        unset.span_count = 2;
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
        rec.box_number = 9;
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
        let c: NodePtr<CharNode> = NodePtr::from_idx_unchecked(at(14));
        let ch = &mut mem[c];
        (ch.character, ch.font, ch.link) = (65, FontId::new(3), MemPtr::from_raw(9));
        assert_eq!((mem.b16s0(at(14)), mem.b32s1(at(14))), (65, 9));
        assert_eq!(mem.b16s0(at(15)), 3);
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
        let delta = mem.journal.seal(&mem.data);
        delta.apply_to(&mut mem.data);
        let spec = mem[s];
        assert_eq!(
            (spec.ref_count, spec.width, spec.stretch, spec.shrink),
            (0, Scaled::ZERO, Scaled::ZERO, Scaled::from_raw(5))
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
        assert_eq!(mem.b32s0(MemIdx::from_raw(4)), 2);
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
        let at = MemIdx::from_raw;
        assert_eq!((mem.b32s0(at(4)), mem.b32s1(at(4))), (3, 9));
        assert_eq!(
            mem.payloads.get(mem[p].payload),
            [0, 0, 4 | 3 << 16, 2 | 1 << 16]
        );
        assert_eq!(mem.entries(p)[1].chars(), [1, 2, 3, 4]);
    }

    #[test]
    fn io_whatsit_fields_land_on_their_words() {
        let mut mem = arena();
        let at = MemIdx::from_raw;
        let o: NodePtr<OpenNode> = NodePtr::from_idx_unchecked(at(4));
        let open = &mut mem[o];
        open.subtype = WhatsitSubtype::Open.into();
        open.link = MemPtr::from_raw(3);
        open.stream = 5;
        open.name = StrNumber::from_raw(6);
        open.area = StrNumber::from_raw(7);
        open.ext = StrNumber::from_raw(8);
        assert_eq!((mem.b16s0(at(4)), mem.b32s1(at(4))), (0, 3));
        assert_eq!([mem.b32s0(at(5)), mem.b32s1(at(5))], [5, 6]);
        assert_eq!([mem.b32s0(at(6)), mem.b32s1(at(6))], [7, 8]);
        assert!(matches!(o.whatsit().variant(&mem), Some(WhatsitVariant::Open(p)) if p == o));

        let w: NodePtr<WriteNode> = NodePtr::from_idx_unchecked(at(8));
        let write = &mut mem[w];
        write.subtype = WhatsitSubtype::Special.into();
        write.stream = 9;
        write.tokens = TokenList::from_raw(10);
        assert_eq!(mem.b16s0(at(8)), 3);
        assert_eq!([mem.b32s0(at(9)), mem.b32s1(at(9))], [9, 10]);
        assert!(matches!(w.whatsit().variant(&mem), Some(WhatsitVariant::Special(p)) if p == w));

        let l: NodePtr<LanguageNode> = NodePtr::from_idx_unchecked(at(12));
        let language = &mut mem[l];
        language.subtype = WhatsitSubtype::Language.into();
        language.lang = 11;
        language.lhm = 2;
        language.rhm = 3;
        assert_eq!(mem.b16s0(at(12)), 5);
        assert_eq!(
            (mem.b16s1(at(13)), mem.b16s0(at(13)), mem.b32s1(at(13))),
            (2, 3, 11)
        );
        assert!(matches!(l.whatsit().variant(&mem), Some(WhatsitVariant::Language(p)) if p == l));
    }

    #[test]
    fn glyph_fields_land_on_their_words() {
        let mut mem = arena();
        let at = MemIdx::from_raw;
        let g: NodePtr<GlyphNode> = NodePtr::from_idx_unchecked(at(4));
        let glyph = &mut mem[g];
        glyph.subtype = WhatsitSubtype::Glyph.into();
        glyph.width = Scaled::from_raw(11);
        glyph.depth = Scaled::from_raw(12);
        glyph.height = Scaled::from_raw(13);
        glyph.glyph = 14;
        glyph.font = FontId::new(15);
        assert_eq!(mem.b16s0(at(4)), 42);
        assert_eq!([5, 6, 7].map(|i| mem.b32s1(at(i))), [11, 12, 13]);
        assert_eq!([5, 6, 7].map(|i| mem.b32s0(at(i))), [0; 3]);
        assert_eq!(
            [
                mem.b16s0(at(8)),
                mem.b16s1(at(8)),
                mem.b16s2(at(8)),
                mem.b16s3(at(8))
            ],
            [0, 14, 15, 0]
        );
        assert!(matches!(g.whatsit().variant(&mem), Some(WhatsitVariant::Glyph(p)) if p == g));
    }

    #[test]
    fn pic_fields_land_on_their_words_and_the_path_in_the_payload() {
        let mut mem = MemState {
            data: vec![Default::default(); 24],
            ..Default::default()
        };
        let at = MemIdx::from_raw;
        let p: NodePtr<PicNode> = NodePtr::from_idx_unchecked(at(4));
        let pic = &mut mem[p];
        pic.subtype = WhatsitSubtype::Pdf.into();
        pic.width = Scaled::from_raw(11);
        pic.depth = Scaled::from_raw(12);
        pic.height = Scaled::from_raw(13);
        pic.page = 2;
        pic.path_len = 10;
        pic.transform = [21, 22, 23, 24, 25, 26].map(Scaled::from_raw);
        pic.pagebox = 3;
        mem.alloc_payload(p, 10).unwrap();
        mem.entries_mut(p).copy_from_slice(b"0123456789");
        assert_eq!(mem.b16s0(at(4)), 44);
        assert_eq!([5, 6, 7].map(|i| mem.b32s1(at(i))), [11, 12, 13]);
        assert_eq!((mem.b16s0(at(8)), mem.b16s1(at(8))), (2, 10));
        assert_eq!(mem.b32s1(at(8)), 1);
        assert_eq!(
            [9, 10, 11].map(|i| [mem.b32s0(at(i)), mem.b32s1(at(i))]),
            [[21, 22], [23, 24], [25, 26]]
        );
        assert_eq!((mem.b16s0(at(12)), mem.b16s1(at(12))), (0, 3));
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
    fn native_word_fields_land_on_their_words_and_the_text_in_the_payload() {
        let mut mem = MemState {
            data: vec![Default::default(); 24],
            ..Default::default()
        };
        let at = MemIdx::from_raw;
        let w = native_word_at(&mut mem, 4, &[1, 2, 3, 4, 5]);
        let word = &mut mem[w];
        word.width = Scaled::from_raw(11);
        word.depth = Scaled::from_raw(12);
        word.height = Scaled::from_raw(13);
        word.glyph_count = 6;
        word.font = FontId::new(7);
        word.glyph_info = 9;
        assert_eq!(mem.b16s0(at(4)), 40);
        assert_eq!([5, 6, 7].map(|i| mem.b32s1(at(i))), [11, 12, 13]);
        assert_eq!(
            [mem.b16s0(at(8)), mem.b16s1(at(8)), mem.b16s2(at(8))],
            [6, 5, 7]
        );
        assert_eq!((mem.b32s0(at(9)), mem.b32s1(at(9))), (9, 1));
        assert_eq!(mem.entries(w), [1, 2, 3, 4, 5]);
        mem[w].clear_glyphs();
        assert_eq!((mem.b16s0(at(8)), mem.b32s0(at(9))), (0, 0));
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
        let dst = native_word_at(&mut mem, 10, &[0; 5]);
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
    fn sized_whatsits_share_their_dimension_words() {
        let mut mem = MemState {
            data: vec![Default::default(); 24],
            ..Default::default()
        };
        let at = MemIdx::from_raw;
        let w = native_word_at(&mut mem, 2, &[]);
        mem[w].depth = Scaled::from_raw(5);
        let g: NodePtr<GlyphNode> = NodePtr::from_idx_unchecked(at(10));
        mem[g].subtype = WhatsitSubtype::Glyph.into();
        mem[g].height = Scaled::from_raw(6);
        let sized = |mem: &MemState, p: NodePtr<WhatsitNode>| p.sized(mem).map(|s| mem[s]);
        assert_eq!(
            sized(&mem, w.whatsit()).map(|s| s.depth),
            Some(Scaled::from_raw(5))
        );
        assert_eq!(
            sized(&mem, g.whatsit()).map(|s| s.height),
            Some(Scaled::from_raw(6))
        );
        let l: NodePtr<LanguageNode> = NodePtr::from_idx_unchecked(at(16));
        mem[l].subtype = WhatsitSubtype::Language.into();
        assert!(l.whatsit().sized(&mem).is_none());
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
