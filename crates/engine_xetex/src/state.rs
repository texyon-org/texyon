//! Rust-owned engine state.

use crate::cmd::Cmd;
use crate::codes::{FiOrElseCode, ValueLevel};
use crate::errors::OrConfusion;
use crate::glue_store::GlueSpecId;
use crate::params::{DimenPar, GluePar, IntPar, TokPar};
use crate::units::{
    BufPos, EqtbPtr, FontId, FontInfoIdx, PoolPos, PrimNum, SavePtr, Scaled, StrNumber,
};

#[derive(Default)]
pub(crate) struct EngineState {
    pub config: ConfigState,
    pub cancellation: crate::cancellation::CancellationState,
    pub errors: ErrorState,
    pub random: crate::random::Random,
    pub shipout: ShipoutState,
    pub scanner: ScannerState,
    pub save_stack: SaveStackState,
    pub eqtb: EqtbState,
    pub mem: MemState,
    pub input: InputClusterState,
    pub strings: StringsState,
    pub hyph: HyphState,
    pub fonts: FontState,
    pub texmf: TexmfState,
    pub synctex: SynctexCtxt,
    pub mainloop: MainLoopState,
    /// Not checkpointed.
    pub pgfmath: crate::pgfmath::Diagnostics,
    /// Not checkpointed: a restore keeps the operation's list.
    pub diagnostics: crate::diagnostic::DiagnosticsState,
}

/// Scratch registers of `main_control`'s character and ligature loop.
#[derive(Default, Clone)]
pub(crate) struct MainLoopState {
    pub main_f: FontId,
    pub main_h: i32,
    pub main_pp: crate::units::MemPtr,
    pub main_ppp: crate::units::MemPtr,
    pub bchar: crate::fonts::LigChar,
    pub false_bchar: crate::fonts::LigChar,
    /// Inter-character space class of the current / previous char.
    pub space_class: i32,
    pub prev_class: i32,
    pub cur_l: crate::fonts::LigChar,
    pub cur_r: crate::fonts::LigChar,
    pub cur_q: crate::units::MemPtr,
    pub lig_stack: Option<crate::main_control::LigStack>,
    pub ligature_present: bool,
    pub lft_hit: bool,
    pub rt_hit: bool,
    pub cancel_boundary: bool,
    pub ins_disc: bool,
}

#[derive(Default, Clone)]
pub(crate) struct SynctexFlags {
    pub content_ready: bool,
    pub off: bool,
    pub not_void: bool,
    pub warn: bool,
    pub output_p: bool,
}

/// The SyncTeX context.
#[derive(Clone)]
pub(crate) struct SynctexCtxt {
    pub file: usize,
    pub root_name: Option<Vec<u8>>,
    pub count: i32,
    /// The node of the current context, `None` before the first.
    pub node: Option<crate::units::MemPtr>,
    /// A kern whose record waits for the context to change. When set it equals
    /// `node`, so flush sites record it before reassigning `node`.
    pub deferred_kern: Option<crate::node::NodePtr<crate::node::KernNode>>,
    pub tag: i32,
    pub line: i32,
    pub curh: i32,
    pub curv: i32,
    pub magnification: i32,
    pub unit: i32,
    pub total_length: i32,
    pub lastv: i32,
    pub form_depth: i32,
    pub synctex_tag_counter: u32,
    pub flags: SynctexFlags,
    pub preview_index: Option<crate::sync_index::PreviewSyncIndex>,
}

impl Default for SynctexCtxt {
    fn default() -> Self {
        Self {
            file: 0, // INVALID_HANDLE
            root_name: None,
            count: 0,
            node: None,
            deferred_kern: None,
            tag: 0,
            line: 0,
            curh: 0,
            curv: 0,
            magnification: 0,
            unit: 0,
            total_length: 0,
            lastv: -1,
            form_depth: 0,
            synctex_tag_counter: 0,
            flags: SynctexFlags::default(),
            preview_index: None,
        }
    }
}

#[derive(Default, Clone)]
pub(crate) struct TexmfState {
    pub last_source_name: Option<Vec<u8>>,
    pub last_lineno: i32,
    /// `D:YYYYmmddHHMMSS+HH'MM'`.
    pub start_time_str: Vec<u8>,
}

/// TeX's input `buffer` of characters, indexed by position.
#[derive(Clone, Default, Debug)]
pub(crate) struct InputBuffer(pub Vec<i32>);

impl std::ops::Index<BufPos> for InputBuffer {
    type Output = i32;
    #[inline(always)]
    fn index(&self, p: BufPos) -> &i32 {
        &self.0[p.index()]
    }
}

impl std::ops::IndexMut<BufPos> for InputBuffer {
    #[inline(always)]
    fn index_mut(&mut self, p: BufPos) -> &mut i32 {
        &mut self.0[p.index()]
    }
}

impl InputBuffer {
    /// The characters from `start` up to but not including `end`.
    #[inline]
    pub(crate) fn span(&self, start: BufPos, end: BufPos) -> &[i32] {
        &self.0[start.index()..end.index()]
    }
}

/// What TeX keeps per open input file in `line_stack`, `grp_stack`,
/// `if_stack`, `eof_seen` and the source name stacks.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub(crate) struct OpenLevel {
    /// The enclosing file's line when this one was opened.
    pub outer_line: i32,
    /// `cur_boundary` when the file was opened.
    pub grp: crate::units::SavePtr,
    /// `cond_ptr` when the file was opened.
    pub cond: Option<crate::cond_stack::CondId>,
    pub source_name: StrNumber,
    pub full_source_name: StrNumber,
    /// Whether `\everyeof` has been inserted at its end.
    pub eof_seen: bool,
}

/// A macro argument on the parameter stack, with its last node so it can be
/// freed without a walk.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub(crate) struct MacroParam {
    pub list: crate::token::TokenList,
    pub tail: crate::token::TokenList,
}

/// One level of TeX's input stack (`in_state_record`).
#[derive(Copy, Clone, Default)]
pub(crate) struct InputState {
    pub state: u16,
    pub index: u16,
    /// On token-list levels `start` and `loc` hold `mem` pointers, read through
    /// [`token_start`](Self::token_start) and [`token_loc`](Self::token_loc), and
    /// a macro level's `limit` is its [`param_start`](Self::param_start).
    pub start: BufPos,
    pub loc: BufPos,
    pub limit: BufPos,
    pub name: InputName,
    pub synctex_tag: i32,
}

/// What an input level reads from, WEB's overloaded `name` field.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub(crate) enum InputName {
    /// The terminal, or an `\insert`ed line above the bottom level.
    #[default]
    Terminal,
    /// `\read` from stream 0 to 15.
    Read(u8),
    /// `\read` from the terminal.
    ReadTerminal,
    /// A `\scantokens` pseudo file, `traced` when `\tracingscantokens` showed it like a file.
    Pseudo { traced: bool },
    /// A file opened by `\input`, by the name it was found under.
    File(StrNumber),
    /// A macro's token list: the control sequence being expanded.
    Macro(EqtbPtr),
}

impl InputName {
    /// Whether lines come from a file or pseudo file, WEB's `name>17`.
    pub(crate) fn reads_lines(self) -> bool {
        matches!(self, Self::Pseudo { .. } | Self::File(_))
    }

    /// Whether the level prints parentheses around its input, WEB's `name>=19`.
    pub(crate) fn shows_parens(self) -> bool {
        matches!(self, Self::Pseudo { traced: true } | Self::File(_))
    }
}

impl InputClusterState {
    /// TeX's buffer size overflow error.
    pub(crate) fn buffer_overflow(&self) -> crate::errors::Abort {
        crate::errors::Abort::Overflow {
            capacity: crate::errors::Capacity::BufferSize,
            n: self.buf_size.raw(),
        }
    }

    /// The number of input levels below the current one, `input_ptr`.
    #[inline]
    pub(crate) fn input_depth(&self) -> i32 {
        i32::try_from(self.input_stack.len()).unwrap_or(i32::MAX)
    }
}

impl InputState {
    /// `limit` of a macro level: where its parameters start on the parameter stack.
    #[inline]
    pub(crate) fn param_start(&self) -> usize {
        usize::try_from(self.limit.raw()).unwrap_or(0)
    }

    #[inline]
    pub(crate) fn set_param_start(&mut self, n: usize) {
        self.limit = BufPos::from_raw(i32::try_from(n).unwrap_or(i32::MAX));
    }
}

/// `start` and `loc` of a token-list level, which point into the token store.
impl InputState {
    #[inline]
    pub(crate) fn token_start(&self) -> crate::token::TokenList {
        crate::token::TokenList::from_raw(self.start.raw())
    }
    #[inline]
    pub(crate) fn token_loc(&self) -> crate::token::TokenList {
        crate::token::TokenList::from_raw(self.loc.raw())
    }
    #[inline]
    pub(crate) fn set_token_loc(&mut self, p: crate::token::TokenList) {
        self.loc = BufPos::from_raw(p.raw());
    }
    #[inline]
    pub(crate) fn set_token_start(&mut self, p: crate::token::TokenList) {
        self.start = BufPos::from_raw(p.raw());
    }
    /// Starts reading the token list `p` from its first node.
    #[inline]
    pub(crate) fn set_token_list(&mut self, p: crate::token::TokenList) {
        self.start = BufPos::from_raw(p.raw());
        self.loc = BufPos::from_raw(p.raw());
    }
}

/// An owned main-input or `\read` stream slot.
#[derive(Default)]
pub(crate) enum InputSlot {
    #[default]
    Empty,
    /// The terminal, which has no file.
    Terminal,
    Open(Box<crate::io::UFile>),
}

#[derive(Default)]
pub(crate) struct InputClusterState {
    pub param_stack: crate::stack::TexStack<MacroParam>,
    /// Per open input file, indexed by `in_open`. Level 0 is the terminal.
    pub open_levels: Vec<OpenLevel>,
    pub buffer: InputBuffer,
    pub input_stack: crate::stack::TexStack<InputState>,
    pub cur_input: InputState,
    pub input_file: Vec<InputSlot>,
    pub nest: crate::stack::TexStack<crate::mem_types::ListStateRecord>,
    pub cur_list: crate::mem_types::ListStateRecord,
    pub open_parens: i32,
    pub in_open: usize,
    pub pseudo_files: crate::units::MemPtr,
    /// The last usable `buffer` position.
    pub buf_size: BufPos,
    pub max_in_open: usize,
    /// The level `show_context` and its relatives are looking at.
    pub base_ptr: usize,
    pub par_loc: crate::units::EqtbPtr,
    pub par_token: crate::token::Token,
    pub force_eof: bool,
    pub first: BufPos,
    pub last: BufPos,
    pub max_buf_stack: BufPos,
    pub read_open: [u8; 17],
    pub read_file: [InputSlot; 16],
}

pub(crate) struct EqtbState {
    pub hash_used: EqtbPtr,
    pub hash_high: i32,
    pub no_new_control_sequence: bool,
    pub prim_used: PrimNum,
    pub cs_count: i32,
    pub data: Vec<crate::mem_types::MemoryWord>,
    pub prim: Vec<crate::mem_types::B32x2>,
    pub xeq_level: Vec<u16>,
    pub hash: Vec<crate::mem_types::B32x2>,
    pub hash_extra: i32,
    pub eqtb_top: crate::units::EqtbPtr,
    pub hash_top: crate::units::EqtbPtr,
    pub journal: crate::journal::EqtbJournal,
    pub pgfmath: crate::pgfmath::KnownCs,
}

impl Default for EqtbState {
    fn default() -> Self {
        EqtbState {
            hash_used: EqtbPtr::NONE,
            hash_high: 0,
            no_new_control_sequence: false,
            prim_used: PrimNum::UNDEFINED,
            cs_count: 0,
            data: Vec::new(),
            // PRIM_SIZE + 1 = 2101.
            prim: vec![crate::mem_types::B32x2 { s0: 0, s1: 0 }; 2101],
            xeq_level: vec![0u16; crate::consts::XEQ_LEVEL_SIZE],
            hash: Vec::new(),
            hash_extra: 0,
            eqtb_top: EqtbPtr::NONE,
            hash_top: EqtbPtr::NONE,
            journal: crate::journal::EqtbJournal::default(),
            pgfmath: crate::pgfmath::KnownCs::default(),
        }
    }
}

impl EqtbState {
    #[inline(always)]
    fn note(&mut self, index: usize) {
        if self.journal.words.armed() {
            self.journal.words.note(index, self.data[index]);
        }
    }

    #[inline]
    fn b32s1(&self, p: EqtbPtr) -> i32 {
        self.data[p.index()].b32().s1
    }

    #[inline]
    fn set_b32s1(&mut self, p: EqtbPtr, v: i32) {
        let index = p.index();
        self.note(index);
        self.data[index].b32.s1 = v;
    }

    #[inline]
    pub fn set_word(&mut self, p: EqtbPtr, value: crate::mem_types::MemoryWord) {
        let index = p.index();
        self.note(index);
        self.data[index] = value;
    }

    #[inline]
    pub fn hash_s0(&self, p: EqtbPtr) -> EqtbPtr {
        EqtbPtr::from_raw(self.hash[crate::hash::hash_slot(p) as usize].s0)
    }

    #[inline]
    pub fn set_hash_s0(&mut self, p: EqtbPtr, v: EqtbPtr) {
        let index = crate::hash::hash_slot(p) as usize;
        if self.journal.hash.armed() {
            self.journal.hash.note(index, self.hash[index]);
        }
        self.hash[index].s0 = v.raw();
    }

    /// The name of control sequence `p`, `text(p)`.
    #[inline]
    pub fn text(&self, p: EqtbPtr) -> StrNumber {
        StrNumber::from_raw(self.hash[crate::hash::hash_slot(p) as usize].s1)
    }

    #[inline]
    pub fn set_text(&mut self, p: EqtbPtr, s: StrNumber) {
        let index = crate::hash::hash_slot(p) as usize;
        if self.journal.hash.armed() {
            self.journal.hash.note(index, self.hash[index]);
        }
        self.hash[index].s1 = s.raw();
    }

    /// The next slot in `p`'s hash chain.
    #[inline]
    pub fn prim_next(&self, p: PrimNum) -> PrimNum {
        PrimNum::from_raw(self.prim[p.index()].s0)
    }

    #[inline]
    pub fn set_prim_next(&mut self, p: PrimNum, next: PrimNum) {
        let index = p.index();
        if self.journal.prim.armed() {
            self.journal.prim.note(index, self.prim[index]);
        }
        self.prim[index].s0 = next.raw();
    }

    /// The name of the primitive in slot `p`, if the slot is used.
    #[inline]
    pub fn prim_text(&self, p: PrimNum) -> Option<StrNumber> {
        // Stored one past the string number so that 0 means unused.
        let raw = self.prim[p.index()].s1;
        (raw != 0).then(|| StrNumber::from_raw(raw - 1))
    }

    #[inline]
    pub fn set_prim_text(&mut self, p: PrimNum, s: Option<StrNumber>) {
        let index = p.index();
        if self.journal.prim.armed() {
            self.journal.prim.note(index, self.prim[index]);
        }
        self.prim[index].s1 = s.map_or(0, |s| s.raw() + 1);
    }

    #[inline]
    pub fn word(&self, p: EqtbPtr) -> crate::mem_types::MemoryWord {
        self.data[p.index()]
    }

    #[inline]
    fn b16s1(&self, p: EqtbPtr) -> u16 {
        self.data[p.index()].b16().s1
    }

    /// The low half of `equiv`.
    #[inline]
    fn b16s2(&self, p: EqtbPtr) -> u16 {
        self.data[p.index()].b16().s2
    }

    /// The command code of control-sequence entry `p`.
    #[inline]
    pub(crate) fn eq_type(&self, p: EqtbPtr) -> Cmd {
        Cmd::from_stored(self.b16s1(p))
    }
    #[inline]
    pub(crate) fn set_eq_type(&mut self, p: EqtbPtr, t: Cmd) {
        self.set_b16s1(p, t.into());
    }
    #[inline]
    pub(crate) fn eq_level(&self, p: EqtbPtr) -> u16 {
        self.b16s0(p)
    }
    #[inline]
    pub(crate) fn set_eq_level(&mut self, p: EqtbPtr, l: u16) {
        self.set_b16s0(p, l);
    }
    #[inline]
    pub(crate) fn equiv(&self, p: EqtbPtr) -> i32 {
        self.b32s1(p)
    }
    /// `equiv(p)` of an entry that holds a pointer.
    #[inline]
    pub(crate) fn equiv_ptr(&self, p: EqtbPtr) -> crate::units::MemPtr {
        crate::units::MemPtr::from_raw(self.equiv(p))
    }
    /// `equiv(p)` of an entry that holds an `eqtb` location, as a `\countdef` does.
    #[inline]
    pub(crate) fn equiv_loc(&self, p: EqtbPtr) -> EqtbPtr {
        EqtbPtr::from_raw(self.equiv(p))
    }
    /// `equiv(p)` of a macro or token-list entry.
    #[inline]
    pub(crate) fn equiv_tokens(&self, p: EqtbPtr) -> crate::token::TokenList {
        crate::token::TokenList::from_raw(self.equiv(p))
    }
    #[inline]
    pub(crate) fn set_equiv(&mut self, p: EqtbPtr, v: i32) {
        self.set_b32s1(p, v);
    }

    /// `eqtb[p].int`, a word in the integer or dimension regions.
    #[inline]
    pub(crate) fn int_at(&self, p: EqtbPtr) -> i32 {
        self.b32s1(p)
    }
    #[inline]
    pub(crate) fn set_int_at(&mut self, p: EqtbPtr, v: i32) {
        self.set_b32s1(p, v);
    }

    /// The control sequence text that identifies font `f`.
    #[inline]
    pub(crate) fn font_id_text(&self, f: FontId) -> StrNumber {
        self.text(f.id_loc())
    }

    #[inline]
    pub(crate) fn cur_font(&self) -> FontId {
        self.font_equiv(crate::consts::CUR_FONT_LOC)
    }

    /// The font an eqtb entry holds, read from the equiv's low half. Fonts are
    /// written as `FontId` and format load checks them, so the high half is zero.
    #[inline]
    pub(crate) fn font_equiv(&self, p: EqtbPtr) -> FontId {
        FontId::new(self.b16s2(p))
    }
    #[inline]
    pub(crate) fn tok_par(&self, n: TokPar) -> crate::token::TokenList {
        self.equiv_tokens(n.loc())
    }
    #[inline]
    pub(crate) fn par_shape(&self) -> Option<crate::node::NodePtr<crate::node::ParShapeNode>> {
        crate::node::NodePtr::from_ptr(self.equiv_ptr(crate::consts::PAR_SHAPE_LOC))
    }
    /// The e-TeX penalty array held in `eqtb[loc]`.
    #[inline]
    pub(crate) fn penalties(
        &self,
        loc: EqtbPtr,
    ) -> Option<crate::node::NodePtr<crate::node::PenaltiesNode>> {
        crate::node::NodePtr::from_ptr(self.equiv_ptr(loc))
    }
    #[inline]
    pub(crate) fn box_reg(&self, n: i32) -> crate::units::MemPtr {
        self.equiv_ptr(crate::consts::BOX_BASE + n)
    }
    #[inline]
    pub(crate) fn set_box_reg(&mut self, n: i32, p: crate::units::MemPtr) {
        self.set_equiv(crate::consts::BOX_BASE + n, p.raw());
    }
    #[inline]
    pub(crate) fn skip(&self, n: i32) -> crate::glue_store::GlueSpecId {
        GlueSpecId::from_raw(self.equiv_ptr(crate::consts::SKIP_BASE + n).raw())
    }
    #[inline]
    pub(crate) fn dimen(&self, n: i32) -> Scaled {
        Scaled::from_raw(self.int_at(crate::consts::SCALED_BASE + n))
    }
    #[inline]
    pub(crate) fn toks(&self, n: i32) -> crate::token::TokenList {
        self.equiv_tokens(crate::consts::TOKS_BASE + n)
    }
    #[inline]
    pub(crate) fn count(&self, n: i32) -> i32 {
        self.equiv(crate::consts::COUNT_BASE + n)
    }
    #[inline]
    pub(crate) fn math_font(&self, n: i32) -> FontId {
        self.font_equiv(crate::consts::MATH_FONT_BASE + n)
    }
    #[inline]
    pub(crate) fn cat_code(&self, c: i32) -> Cmd {
        Cmd::from_catcode_bits(self.equiv(crate::consts::CAT_CODE_BASE + c))
    }
    #[inline]
    pub(crate) fn lc_code(&self, c: i32) -> i32 {
        self.equiv(crate::consts::LC_CODE_BASE + c)
    }
    #[inline]
    pub(crate) fn sf_code(&self, c: i32) -> i32 {
        self.equiv(crate::consts::SF_CODE_BASE + c)
    }
    #[inline]
    pub(crate) fn math_code(&self, c: i32) -> i32 {
        self.equiv(crate::consts::MATH_CODE_BASE + c)
    }
    #[inline]
    pub(crate) fn del_code(&self, c: i32) -> i32 {
        self.equiv(crate::consts::DEL_CODE_BASE + c)
    }

    #[inline]
    fn set_b16s1(&mut self, p: EqtbPtr, v: u16) {
        let index = p.index();
        self.note(index);
        self.data[index].b16.s1 = v;
    }

    #[inline]
    fn b16s0(&self, p: EqtbPtr) -> u16 {
        self.data[p.index()].b16().s0
    }

    #[inline]
    fn set_b16s0(&mut self, p: EqtbPtr, v: u16) {
        let index = p.index();
        self.note(index);
        self.data[index].b16.s0 = v;
    }

    #[inline]
    pub fn int_par(&self, n: IntPar) -> i32 {
        self.b32s1(n.loc())
    }
    #[inline]
    pub fn set_int_par(&mut self, n: IntPar, v: i32) {
        self.set_b32s1(n.loc(), v);
    }
    #[inline]
    pub fn dimen_par(&self, n: DimenPar) -> Scaled {
        Scaled::from_raw(self.b32s1(n.loc()))
    }
    #[inline]
    pub fn set_dimen_par(&mut self, n: DimenPar, v: Scaled) {
        self.set_b32s1(n.loc(), v.raw());
    }
    /// The spec of glue parameter `n`, never null.
    #[inline]
    pub fn glue_par(&self, n: GluePar) -> crate::glue_store::GlueSpecId {
        crate::glue_store::GlueSpecId::from_raw(self.equiv(n.loc()))
    }

    #[inline]
    pub fn set_glue_par(&mut self, n: GluePar, spec: crate::glue_store::GlueSpecId) {
        self.set_b32s1(n.loc(), spec.raw());
    }

    #[inline]
    pub fn xeq_level_at(&self, p: EqtbPtr) -> u16 {
        self.xeq_level[(p - crate::consts::INT_BASE) as usize]
    }

    #[inline]
    pub fn set_xeq_level_at(&mut self, p: EqtbPtr, v: u16) {
        let index = (p - crate::consts::INT_BASE) as usize;
        if self.journal.xeq.armed() {
            self.journal.xeq.note(index, self.xeq_level[index]);
        }
        self.xeq_level[index] = v;
    }
}

/// The `mem` arena and its bookkeeping. `data` never reallocates during a run,
/// so the pointer-returning accessors stay valid.
#[derive(Default)]
pub(crate) struct MemState {
    /// The node arena, in 32-bit words.
    pub data: Vec<u32>,
    /// Owned shaped-glyph payloads referenced by handles in native nodes.
    pub glyph_blobs: crate::glyph_arena::GlyphBlobArena,
    /// The token lists, a store of their own.
    pub tokens: crate::token::TokenStore,
    /// The glue specifications.
    pub glue: crate::glue_store::GlueStore,
    /// The entries of array nodes: native text, picture paths, shapes and pseudo-file lines.
    pub payloads: crate::payload::PayloadStore,
    pub journal: crate::journal::WordJournal<u32>,
    /// The free list of each node size.
    pub free: crate::mem::FreeLists,
    /// TeX's `line`, kept here because `get_node` stamps it into new nodes.
    pub line: i32,
    pub cur_synctex_tag: i32,
    pub temp_ptr: crate::units::MemPtr,
}

impl MemState {
    /// The SyncTeX tag and line a node made now is stamped with.
    #[inline]
    pub(crate) fn sync_stamp(&self) -> crate::node::SyncStamp {
        crate::node::SyncStamp {
            tag: self.cur_synctex_tag,
            line: self.line,
        }
    }

    /// Sets every word of the fixed nodes to null, before the journal is armed.
    pub(crate) fn clear_fixed_nodes(&mut self) {
        debug_assert!(!self.journal.armed());
        let null = crate::units::MemPtr::NULL.raw().cast_unsigned();
        self.data[..crate::mem::RESERVED_WORDS].fill(null);
    }

    /// The raw word at `index`.
    #[cfg(test)]
    pub(crate) fn word(&self, index: usize) -> i32 {
        self.data[index].cast_signed()
    }

    /// Sets the raw word at `index`, journaled.
    #[cfg(test)]
    pub(crate) fn set_word(&mut self, index: usize, v: i32) {
        if self.journal.armed() {
            self.journal.note(index, self.data[index]);
        }
        self.data[index] = v.cast_unsigned();
    }
}

/// The kind of group on the save stack (`cur_group`).
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
#[repr(u8)]
pub(crate) enum GroupCode {
    #[default]
    BottomLevel = 0,
    Simple = 1,
    Hbox = 2,
    AdjustedHbox = 3,
    Vbox = 4,
    Vtop = 5,
    Align = 6,
    NoAlign = 7,
    Output = 8,
    Math = 9,
    Disc = 10,
    Insert = 11,
    Vcenter = 12,
    MathChoice = 13,
    SemiSimple = 14,
    MathShift = 15,
    MathLeft = 16,
}

/// The command that closes a group `}` cannot close, as TeX names it.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum Closer {
    EndGroup,
    MathShift,
    Right,
}

impl crate::output::TexPrint for Closer {
    fn print_to(&self, st: &mut EngineState) {
        use crate::output::{print, Esc};
        match self {
            Closer::EndGroup => print(st, Esc("endgroup")),
            Closer::MathShift => print(st, '$'),
            Closer::Right => print(st, Esc("right")),
        }
    }
}

/// What `off_save` inserts to close the current group, shown as the error names it.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) struct Insertion(pub Option<Closer>);

impl Insertion {
    /// The inserted tokens, in reading order.
    pub(crate) fn tokens(self) -> Vec<crate::token::Token> {
        use crate::consts::{FROZEN_END_GROUP, FROZEN_RIGHT};
        use crate::token::Token;
        match self.0 {
            Some(Closer::EndGroup) => vec![Token::from_cs(FROZEN_END_GROUP)],
            Some(Closer::MathShift) => vec![Token::math_shift(b'$')],
            Some(Closer::Right) => vec![Token::from_cs(FROZEN_RIGHT), Token::other(b'.')],
            None => vec![Token::right_brace(b'}')],
        }
    }
}

impl crate::output::TexPrint for Insertion {
    fn print_to(&self, st: &mut EngineState) {
        use crate::output::{print, Esc};
        match self.0 {
            Some(Closer::Right) => print(st, Esc("right.")),
            Some(closer) => print(st, closer),
            None => print(st, '}'),
        }
    }
}

impl GroupCode {
    /// What closes this group, `None` for groups that `}` closes.
    pub(crate) fn closer(self) -> Option<Closer> {
        match self {
            GroupCode::SemiSimple => Some(Closer::EndGroup),
            GroupCode::MathShift => Some(Closer::MathShift),
            GroupCode::MathLeft => Some(Closer::Right),
            _ => None,
        }
    }

    #[inline]
    pub(crate) fn from_raw(v: u8) -> Option<GroupCode> {
        Some(match v {
            0 => GroupCode::BottomLevel,
            1 => GroupCode::Simple,
            2 => GroupCode::Hbox,
            3 => GroupCode::AdjustedHbox,
            4 => GroupCode::Vbox,
            5 => GroupCode::Vtop,
            6 => GroupCode::Align,
            7 => GroupCode::NoAlign,
            8 => GroupCode::Output,
            9 => GroupCode::Math,
            10 => GroupCode::Disc,
            11 => GroupCode::Insert,
            12 => GroupCode::Vcenter,
            13 => GroupCode::MathChoice,
            14 => GroupCode::SemiSimple,
            15 => GroupCode::MathShift,
            16 => GroupCode::MathLeft,
            _ => return None,
        })
    }

    #[inline]
    pub(crate) fn as_raw(self) -> u8 {
        self as u8
    }
}

/// The kind of a save-stack entry.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u16)]
pub(crate) enum SaveType {
    RestoreOldValue = 0,
    RestoreZero = 1,
    InsertToken = 2,
    LevelBoundary = 3,
    RestoreSa = 4,
}

impl SaveType {
    #[inline]
    pub(crate) fn from_raw(raw: u16) -> Option<SaveType> {
        Some(match raw {
            0 => SaveType::RestoreOldValue,
            1 => SaveType::RestoreZero,
            2 => SaveType::InsertToken,
            3 => SaveType::LevelBoundary,
            4 => SaveType::RestoreSa,
            _ => return None,
        })
    }
}

#[derive(Default)]
pub(crate) struct SaveStackState {
    pub save_ptr: SavePtr,
    pub max_save_stack: SavePtr,
    pub save_size: usize,
    pub cur_level: u16,
    pub cur_group: GroupCode,
    pub cur_boundary: SavePtr,
    pub data: Vec<crate::mem_types::MemoryWord>,
}

impl SaveStackState {
    /// The save stack ran out.
    pub(crate) fn overflow(&self) -> crate::errors::Abort {
        crate::errors::Abort::overflow(crate::errors::Capacity::SaveSize, self.save_size)
    }

    #[inline]
    fn b32s1(&self, k: SavePtr) -> i32 {
        self.data[k.index()].b32().s1
    }

    #[inline]
    fn set_b32s1(&mut self, k: SavePtr, v: i32) {
        self.data[k.index()].b32.s1 = v;
    }

    #[inline]
    fn set_b16s1(&mut self, k: SavePtr, v: u16) {
        self.data[k.index()].b16.s1 = v;
    }

    #[inline]
    fn set_b16s0(&mut self, k: SavePtr, v: u16) {
        self.data[k.index()].b16.s0 = v;
    }

    #[inline]
    pub(crate) fn set_word(&mut self, k: SavePtr, w: crate::mem_types::MemoryWord) {
        self.data[k.index()] = w;
    }

    #[inline]
    fn b16s1(&self, k: SavePtr) -> u16 {
        self.data[k.index()].b16().s1
    }

    #[inline]
    fn b16s0(&self, k: SavePtr) -> u16 {
        self.data[k.index()].b16().s0
    }

    /// WEB's `saved(k)`, the word `k` entries above `save_ptr`.
    #[inline]
    pub(crate) fn saved(&self, k: i32) -> i32 {
        self.b32s1(self.save_ptr + k)
    }
    #[inline]
    pub(crate) fn set_saved(&mut self, k: i32, v: i32) {
        self.set_b32s1(self.save_ptr + k, v);
    }

    #[inline]
    pub(crate) fn save_type(&self, k: SavePtr) -> Option<SaveType> {
        SaveType::from_raw(self.b16s1(k))
    }
    #[inline]
    pub(crate) fn save_level(&self, k: SavePtr) -> u16 {
        self.b16s0(k)
    }
    #[inline]
    pub(crate) fn save_index(&self, k: SavePtr) -> i32 {
        self.b32s1(k)
    }
    /// Pushes a box's size spec, to be read back after its group by [`pop_spec`](Self::pop_spec).
    pub(crate) fn push_spec(&mut self, spec: crate::packaging::PackSpec) {
        let (code, amount) = spec.to_words();
        self.set_saved(0, code);
        self.set_saved(1, amount);
        self.save_ptr += 2;
    }

    /// Pops the size spec [`push_spec`](Self::push_spec) saved.
    pub(crate) fn pop_spec(&mut self) -> Result<crate::packaging::PackSpec, crate::errors::Abort> {
        self.save_ptr -= 2;
        self.spec_at(0).or_confusion("pop_spec")
    }

    /// The size spec saved at `saved(k)` and `saved(k + 1)`.
    pub(crate) fn spec_at(&self, k: i32) -> Option<crate::packaging::PackSpec> {
        crate::packaging::PackSpec::from_words(self.saved(k), self.saved(k + 1))
    }

    /// `save_index(k)` of an entry that saved an `eqtb` location.
    #[inline]
    pub(crate) fn save_loc(&self, k: SavePtr) -> EqtbPtr {
        EqtbPtr::from_raw(self.b32s1(k))
    }
    /// `save_index(k)` of an entry that saved a pointer.
    #[inline]
    pub(crate) fn save_index_ptr(&self, k: SavePtr) -> crate::units::MemPtr {
        crate::units::MemPtr::from_raw(self.save_index(k))
    }
    /// Writes a restore entry at `save_ptr`.
    #[inline]
    pub(crate) fn set_save_entry(&mut self, kind: SaveType, level: u16, index: i32) {
        let k = self.save_ptr;
        self.set_b16s1(k, kind as u16);
        self.set_b16s0(k, level);
        self.set_b32s1(k, index);
    }

    /// The group and enclosing boundary saved by the level boundary at `k`.
    #[inline]
    pub(crate) fn boundary_at(&self, k: SavePtr) -> (GroupCode, SavePtr) {
        let group = GroupCode::from_raw(self.b16s0(k) as u8).expect("valid group code");
        (group, SavePtr::from_raw(self.b32s1(k)))
    }

    #[inline]
    pub(crate) fn word(&self, k: SavePtr) -> crate::mem_types::MemoryWord {
        self.data[k.index()]
    }
}

/// `cur_mark`, the marks of the current page and split.
#[derive(Default, Clone, Copy)]
pub(crate) struct CurMarks([crate::token::TokenList; 5]);

impl CurMarks {
    pub(crate) const NULL: CurMarks = CurMarks([crate::token::TokenList::NULL; 5]);
}

impl std::ops::Index<crate::codes::MarkCode> for CurMarks {
    type Output = crate::token::TokenList;
    #[inline]
    fn index(&self, m: crate::codes::MarkCode) -> &crate::token::TokenList {
        &self.0[m as usize]
    }
}

impl std::ops::IndexMut<crate::codes::MarkCode> for CurMarks {
    #[inline]
    fn index_mut(&mut self, m: crate::codes::MarkCode) -> &mut crate::token::TokenList {
        &mut self.0[m as usize]
    }
}

/// `sa_root`, the sparse-array trees of registers and mark classes by type.
#[derive(Default, Clone, Copy)]
pub(crate) struct SaRoots([crate::units::MemPtr; 8]);

impl SaRoots {
    /// The register trees, `int_val..=inter_char_val`, which formats keep.
    pub(crate) fn registers_mut(&mut self) -> &mut [crate::units::MemPtr] {
        &mut self.0[ValueLevel::Int as usize..=ValueLevel::InterChar as usize]
    }

    /// The root index node of the tree for type `t`.
    #[inline]
    pub(crate) fn get(
        &self,
        t: ValueLevel,
    ) -> Option<crate::node::NodePtr<crate::node::SaIndexNode>> {
        self.0[t as usize]
            .as_idx()
            .map(crate::node::NodePtr::from_idx_unchecked)
    }

    #[inline]
    pub(crate) fn set(
        &mut self,
        t: ValueLevel,
        root: Option<crate::node::NodePtr<crate::node::SaIndexNode>>,
    ) {
        self.0[t as usize] = crate::node::link_of(root);
    }
}

#[derive(Default, Clone)]
pub(crate) struct ScannerState {
    /// Monotonic token-fetch clock used to order checkpoint fences.
    pub logical_clock: u64,
    pub name_in_progress: bool,
    pub cur_align: crate::units::MemPtr,
    pub cur_span: crate::units::MemPtr,
    pub cur_loop: crate::units::MemPtr,
    /// The enclosing alignments, TeX's `align_ptr` list.
    pub align_stack: Vec<crate::main_control::SavedAlign>,
    pub cur_head: crate::units::MemPtr,
    pub cur_tail: crate::units::MemPtr,
    pub cur_pre_head: crate::units::MemPtr,
    pub cur_pre_tail: crate::units::MemPtr,
    pub shown_mode: i16,
    pub mag_set: i32,
    pub expand_depth_count: i32,
    pub sa_chain: Option<crate::node::NodePtr<crate::node::SaSaved>>,
    pub sa_level: u16,
    pub max_reg_num: i32,
    /// The innermost incomplete conditional, `None` outside any.
    /// The conditional stack, TeX's `cond_ptr` list.
    pub conds: crate::cond_stack::CondStack,
    pub if_limit: crate::node::Stored<FiOrElseCode>,
    pub cur_if: i16,
    pub if_line: i32,
    pub skip_line: i32,
    pub is_in_csname: bool,
    pub used_tectonic_coda_tokens: bool,
    pub scanner_status: crate::codes::ScannerStatus,
    pub warning_index: EqtbPtr,
    pub def_ref: crate::token::TokenList,
    pub long_state: Cmd,
    pub align_state: i32,
    pub expand_depth: i32,
    pub cur_cmd: Cmd,
    pub cur_chr: i32,
    pub cur_cs: crate::units::EqtbPtr,
    pub cur_tok: crate::token::Token,
    pub cur_mark: CurMarks,
    pub disc_ptr: [crate::units::MemPtr; 4],
    pub after_token: crate::token::Token,
    pub cur_f: FontId,
    pub set_box_allowed: bool,
    pub sa_root: SaRoots,
    pub empty: crate::mem_types::B32x2,
    pub native_text: Vec<u16>,
    pub native_text_size: i32,
    pub native_len: i32,
    pub save_native_len: i32,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EngineExecutionMode {
    #[default]
    Batch,
    Resident,
}

#[derive(Default, Clone, Copy)]
pub(crate) struct ConfigState {
    pub file_line_error_style_p: i32,
    pub halt_on_error_p: i32,
    pub in_initex_mode: bool,
    pub synctex_enabled: i32,
    pub semantic_pagination_enabled: bool,
    pub shell_escape_enabled: bool,
    /// Remove live clock and timezone inputs from engine-visible values.
    pub deterministic_mode: bool,
    /// Selects operation-lifetime behavior that differs for a resident engine,
    /// such as guarded picture probes.
    pub execution_mode: EngineExecutionMode,
    pub max_print_line: i32,
    pub pgfmath: crate::pgfmath::Mode,
}

pub(crate) use crate::layout::{FontMappingHandle, LinebreakHandle};

/// A native font's layout engine.
pub(crate) type FontEngineHandle = texyon_xetex_layout::LayoutEngine<'static>;

pub(crate) struct FontState {
    /// Converter staged by a `mapping=` font feature until it is transferred
    /// into a font slot.
    pub loaded_font_mapping: Option<std::sync::Arc<FontMappingHandle>>,
    pub loaded_font_flags: i8,
    /// Per-engine ICU line-break cache and its string-pool locale key.
    pub brk_iter: Option<LinebreakHandle>,
    pub brk_locale_str_num: StrNumber,
    pub font_layout_engine: PerFont<Option<std::rc::Rc<FontEngineHandle>>>,
    pub font_mapping: PerFont<Option<std::sync::Arc<FontMappingHandle>>>,
    pub loaded_font_letter_space: Scaled,
    pub xtx_ligature_present: bool,
    pub gave_char_warning_help: bool,
    pub native_font_type_flag: StrNumber,
    /// The end of `font_info`.
    pub font_mem_size: FontInfoIdx,
    pub font_max: FontId,
    pub fmem_ptr: FontInfoIdx,
    pub font_ptr: FontId,
    pub font_flags: PerFont<i8>,
    pub font_letter_space: PerFont<Scaled>,
    pub font_check: PerFont<[u8; 4]>,
    pub font_size: PerFont<Scaled>,
    pub font_dsize: PerFont<Scaled>,
    pub font_params: PerFont<i32>,
    pub font_name: PerFont<StrNumber>,
    pub font_area: PerFont<StrNumber>,
    pub font_bc: PerFont<u16>,
    pub font_ec: PerFont<u16>,
    pub bchar_label: PerFont<FontInfoIdx>,
    pub font_bchar: PerFont<crate::fonts::LigChar>,
    pub font_false_bchar: PerFont<crate::fonts::LigChar>,
    /// The interword glue built from the font's parameters, once a space needs it.
    pub font_glue: PerFont<Option<crate::glue_store::GlueSpecId>>,
    pub font_used: PerFont<bool>,
    pub char_base: PerFont<FontInfoIdx>,
    pub width_base: PerFont<FontInfoIdx>,
    pub height_base: PerFont<FontInfoIdx>,
    pub depth_base: PerFont<FontInfoIdx>,
    pub italic_base: PerFont<FontInfoIdx>,
    pub lig_kern_base: PerFont<FontInfoIdx>,
    pub kern_base: PerFont<FontInfoIdx>,
    pub exten_base: PerFont<FontInfoIdx>,
    pub param_base: PerFont<FontInfoIdx>,
    pub hyphen_char: PerFont<i32>,
    pub skew_char: PerFont<i32>,
    pub font_info: Vec<crate::mem_types::MemoryWord>,
    pub font_info_journal: crate::journal::PlainTrail<crate::mem_types::MemoryWord>,
    /// Font-memory high-water mark at the start of the open journal epoch.
    pub fmem_epoch_base: FontInfoIdx,
    pub mapped_text: Vec<u16>,
    pub cp_codes: crate::fonts::ProtrusionCodes,
    /// Glyph bounds in TeX points, valid for the fonts through `font_ptr`.
    pub glyph_bboxes: std::cell::RefCell<
        std::collections::BTreeMap<(FontId, u16), texyon_xetex_layout::GlyphBBox>,
    >,
}

impl Default for FontState {
    fn default() -> Self {
        Self {
            loaded_font_mapping: None,
            loaded_font_flags: 0,
            brk_iter: None,
            brk_locale_str_num: StrNumber::from_raw(0),
            font_layout_engine: PerFont::default(),
            font_mapping: PerFont::default(),
            loaded_font_letter_space: Scaled::ZERO,
            xtx_ligature_present: false,
            gave_char_warning_help: false,
            native_font_type_flag: StrNumber::NONE,
            font_mem_size: FontInfoIdx::ZERO,
            font_max: crate::consts::NULL_FONT,
            fmem_ptr: FontInfoIdx::ZERO,
            font_ptr: crate::consts::NULL_FONT,
            font_flags: PerFont::default(),
            font_letter_space: PerFont::default(),
            font_check: PerFont::default(),
            font_size: PerFont::default(),
            font_dsize: PerFont::default(),
            font_params: PerFont::default(),
            font_name: PerFont::default(),
            font_area: PerFont::default(),
            font_bc: PerFont::default(),
            font_ec: PerFont::default(),
            bchar_label: PerFont::default(),
            font_bchar: PerFont::default(),
            font_false_bchar: PerFont::default(),
            font_glue: PerFont::default(),
            font_used: PerFont::default(),
            char_base: PerFont::default(),
            width_base: PerFont::default(),
            height_base: PerFont::default(),
            depth_base: PerFont::default(),
            italic_base: PerFont::default(),
            lig_kern_base: PerFont::default(),
            kern_base: PerFont::default(),
            exten_base: PerFont::default(),
            param_base: PerFont::default(),
            hyphen_char: PerFont::default(),
            skew_char: PerFont::default(),
            font_info: Vec::new(),
            font_info_journal: crate::journal::PlainTrail::default(),
            fmem_epoch_base: FontInfoIdx::ZERO,
            mapped_text: Vec::new(),
            cp_codes: crate::fonts::ProtrusionCodes::default(),
            glyph_bboxes: std::cell::RefCell::default(),
        }
    }
}

impl FontState {
    #[inline]
    pub(crate) fn font_info_b16(&self, k: FontInfoIdx) -> crate::mem_types::B16x4 {
        self.font_info[k.index()].b16()
    }

    /// `font_info[k].sc`.
    #[inline]
    pub(crate) fn font_info_sc(&self, k: FontInfoIdx) -> Scaled {
        Scaled::from_raw(self.font_info[k.index()].b32().s1)
    }

    /// Font parameter `n` of font `f`.
    #[inline]
    pub(crate) fn param(&self, f: FontId, n: i32) -> Scaled {
        self.font_info_sc(self.param_base[f] + n)
    }

    #[inline]
    pub(crate) fn set_font_info_b16(&mut self, k: FontInfoIdx, v: crate::mem_types::B16x4) {
        let index = k.index();
        if self.font_info_journal.armed() && k < self.fmem_epoch_base {
            self.font_info_journal.note(index, self.font_info[index]);
        }
        self.font_info[index].b16 = v;
    }

    #[inline]
    pub(crate) fn set_font_info_sc(&mut self, k: FontInfoIdx, v: Scaled) {
        let index = k.index();
        if self.font_info_journal.armed() && k < self.fmem_epoch_base {
            self.font_info_journal.note(index, self.font_info[index]);
        }
        self.font_info[index].b32.s1 = v.raw();
    }
}

/// A table with one entry per internal font.
#[derive(Clone, Default)]
pub(crate) struct PerFont<T>(Vec<T>);

impl<T: Clone> PerFont<T> {
    pub(crate) fn filled(len: usize, value: T) -> Self {
        PerFont(vec![value; len])
    }
}

impl<T> PerFont<T> {
    #[inline]
    pub(crate) fn as_slice(&self) -> &[T] {
        &self.0
    }

    #[inline]
    pub(crate) fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.0
    }
}

impl<T> std::ops::Index<FontId> for PerFont<T> {
    type Output = T;
    #[inline(always)]
    fn index(&self, f: FontId) -> &T {
        &self.0[f.index()]
    }
}

impl<T> std::ops::IndexMut<FontId> for PerFont<T> {
    #[inline(always)]
    fn index_mut(&mut self, f: FontId) -> &mut T {
        &mut self.0[f.index()]
    }
}

/// What the current page holds so far (`page_contents`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub(crate) enum PageContents {
    #[default]
    Empty,
    InsertsOnly,
    BoxThere,
}

#[derive(Default, Clone)]
pub(crate) struct ShipoutState {
    pub dvi: crate::dvi::DviWriter,
    pub insert_src_special_every_par: bool,
    pub insert_src_special_every_math: bool,
    pub insert_src_special_every_vbox: bool,
    pub insert_src_special_auto: bool,
    pub selector: Selector,
    pub doing_special: bool,
    pub term_offset: i32,
    pub file_offset: i32,
    pub old_setting: Selector,
    pub epochseconds: i32,
    pub microseconds: i32,
    pub dead_cycles: i32,
    pub page_tail: crate::units::MemPtr,
    pub last_badness: i32,
    pub page_contents: PageContents,
    pub page_so_far: [Scaled; 8],
    pub last_glue: Option<crate::glue_store::GlueSpecId>,
    pub last_penalty: i32,
    pub last_kern: Scaled,
    pub last_node_type: i32,
    pub insert_penalties: i32,
    pub output_active: bool,
    /// Set only while the finished `\output` level is popped.
    pub output_can_end: bool,
    pub pdf_last_x_pos: i32,
    pub pdf_last_y_pos: i32,
    pub write_file: [usize; 16],
    pub write_open: [bool; 18],
    /// Packed NUL-terminated names for bridge-local `\write` handles.
    pub write_name: [Option<Vec<u8>>; 16],
    /// Code units emitted before the current replay fence.
    pub write_prefix: [Vec<i32>; 16],
    pub rust_stdout: usize,
    pub log_file: usize,
    /// Transcript bytes written so far, captured with every checkpoint.
    pub log_bytes: u64,
    /// Where this operation's transcript continues the previous one, if it
    /// resumed. Operation-scoped, so checkpoint restores keep it.
    pub log_resumed_at: Option<u64>,
    /// Diagnostics reported so far, captured with every checkpoint.
    pub diagnostics_issued: usize,
    pub best_page_break: crate::units::MemPtr,
    pub best_size: Scaled,
    pub least_page_cost: i32,
    pub page_max_depth: Scaled,
    pub total_pages: i32,
    pub max_v: Scaled,
    pub max_h: Scaled,
    pub doing_leaders: bool,
    pub cur_h: Scaled,
    pub cur_v: Scaled,
    pub cur_page_width: Scaled,
    pub cur_page_height: Scaled,
    pub cur_h_offset: Scaled,
    pub cur_v_offset: Scaled,
    pub write_loc: crate::units::EqtbPtr,
    pub output_file_name: StrNumber,
    /// The vertical and horizontal DVI movements of the page so far, TeX's `down_ptr` and
    /// `right_ptr` lists, the latest last.
    pub down_moves: Vec<crate::shipout::Movement>,
    pub right_moves: Vec<crate::shipout::Movement>,
    pub dvi_h: Scaled,
    pub dvi_v: Scaled,
    pub dvi_f: FontId,
}

/// TeX's `interaction` level, ordered like its numeric codes.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub(crate) enum InteractionMode {
    Batch = 0,
    Nonstop = 1,
    Scroll = 2,
    ErrorStop = 3,
}

impl InteractionMode {
    /// `None` outside `0..=3`, which is the `\interactionmode` range check.
    pub(crate) fn from_raw(v: i32) -> Option<InteractionMode> {
        match v {
            0 => Some(InteractionMode::Batch),
            1 => Some(InteractionMode::Nonstop),
            2 => Some(InteractionMode::Scroll),
            3 => Some(InteractionMode::ErrorStop),
            _ => None,
        }
    }
}

/// Where printing goes (`selector`), ordered like TeX's numeric codes.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub(crate) enum Selector {
    /// `\write` stream `0..=15`.
    File(u8),
    NoPrint,
    #[default]
    TermOnly,
    LogOnly,
    TermAndLog,
    Pseudo,
    NewString,
}

impl Selector {
    pub(crate) const fn raw(self) -> u8 {
        match self {
            Selector::File(n) => n,
            Selector::NoPrint => 16,
            Selector::TermOnly => 17,
            Selector::LogOnly => 18,
            Selector::TermAndLog => 19,
            Selector::Pseudo => 20,
            Selector::NewString => 21,
        }
    }

    const fn from_raw(v: u8) -> Selector {
        match v {
            0..=15 => Selector::File(v),
            16 => Selector::NoPrint,
            17 => Selector::TermOnly,
            18 => Selector::LogOnly,
            19 => Selector::TermAndLog,
            20 => Selector::Pseudo,
            21 => Selector::NewString,
            _ => panic!("selector out of range"),
        }
    }

    /// TeX's `selector - 1`: `TermAndLog` to `LogOnly`, `TermOnly` to `NoPrint`.
    pub(crate) const fn without_terminal(self) -> Selector {
        Selector::from_raw(self.raw() - 1)
    }

    /// TeX's `selector + 1`, undoing [`Selector::without_terminal`].
    pub(crate) const fn with_terminal(self) -> Selector {
        Selector::from_raw(self.raw() + 1)
    }

    /// TeX's `selector + 2`: `NoPrint` to `LogOnly`, `TermOnly` to `TermAndLog`.
    pub(crate) const fn with_log(self) -> Selector {
        Selector::from_raw(self.raw() + 2)
    }

    /// TeX's `selector - 2`, undoing [`Selector::with_log`].
    pub(crate) const fn without_log(self) -> Selector {
        Selector::from_raw(self.raw() - 2)
    }
}

#[derive(Clone)]
pub(crate) struct ErrorState {
    pub help_lines: [&'static str; 6],
    pub help_len: u8,
    pub interaction: InteractionMode,
    pub history: i32,
    pub error_count: i8,
    pub use_err_help: bool,
    pub long_help_seen: bool,
    pub log_opened: bool,
    pub tally: i32,
    pub error_line: i32,
    pub half_error_line: i32,
    pub trick_buf: [i32; 256],
    pub trick_count: i32,
    pub first_count: i32,
    pub deletions_allowed: bool,
    /// Set once `--halt-on-error` has started to stop the run.
    pub halting_on_error: bool,
    pub font_in_short_display: FontId,
    pub depth_threshold: i32,
    pub breadth_max: i32,
}

impl Default for ErrorState {
    fn default() -> Self {
        Self {
            help_lines: [""; 6],
            help_len: 0,
            interaction: InteractionMode::Batch,
            history: 0,
            error_count: 0,
            use_err_help: false,
            long_help_seen: false,
            log_opened: false,
            tally: 0,
            error_line: 0,
            half_error_line: 0,
            trick_buf: [0; 256],
            trick_count: 0,
            first_count: 0,
            deletions_allowed: false,
            halting_on_error: false,
            font_in_short_display: crate::consts::NULL_FONT,
            depth_threshold: 0,
            breadth_max: 0,
        }
    }
}

#[derive(Default)]
pub(crate) struct StringsState {
    /// The string under construction, one past the last made string.
    pub str_ptr: StrNumber,
    /// The first string number past the `str_start` table.
    pub max_strings: StrNumber,
    pub init_str_ptr: StrNumber,
    pub init_pool_ptr: PoolPos,
    pub job_name: StrNumber,
    pub texmf_log_name: StrNumber,
    pub string_vacancies: i32,
    /// The end of `str_pool`.
    pub pool_size: PoolPos,
    pub pool_ptr: PoolPos,
    pub format_ident: StrNumber,
    pub str_start: Vec<PoolPos>,
    pub str_pool: Vec<u16>,
}

impl StringsState {
    /// The string pool ran out.
    pub(crate) fn pool_overflow(&self) -> crate::errors::Abort {
        crate::errors::Abort::Overflow {
            capacity: crate::errors::Capacity::PoolSize,
            n: self.pool_size - self.init_pool_ptr,
        }
    }

    /// The string table ran out.
    pub(crate) fn string_overflow(&self) -> crate::errors::Abort {
        crate::errors::Abort::Overflow {
            capacity: crate::errors::Capacity::NumberOfStrings,
            n: self.max_strings - self.init_str_ptr,
        }
    }

    /// The `str_start` slot of the string under construction.
    #[inline]
    pub(crate) fn cur_start_slot(&self) -> usize {
        if self.str_ptr.is_pool_string() {
            self.str_ptr.slot()
        } else {
            0
        }
    }

    /// The pool offset where the string under construction starts.
    #[inline]
    pub(crate) fn cur_string_start(&self) -> PoolPos {
        self.str_start[self.str_ptr.slot()]
    }

    /// TeX's `append_char`: adds UTF-16 unit `c` to the string under construction.
    #[inline]
    pub(crate) fn append_char(&mut self, c: u16) {
        self.str_pool[self.pool_ptr.index()] = c;
        self.pool_ptr += 1;
    }

    /// Appends UTF-16 `units` to the string under construction.
    #[inline]
    pub(crate) fn append_units(&mut self, units: &[u16]) {
        let start = self.pool_ptr.index();
        self.str_pool[start..start + units.len()].copy_from_slice(units);
        self.pool_ptr += units.len() as i32;
    }

    /// The characters of the string under construction.
    #[inline]
    pub(crate) fn cur_string(&self) -> &[u16] {
        &self.str_pool[self.cur_string_start().index()..self.pool_ptr.index()]
    }

    /// Drops the characters of the string under construction.
    #[inline]
    pub(crate) fn discard_cur_string(&mut self) {
        self.pool_ptr = self.cur_string_start();
    }
}

pub(crate) struct HyphState {
    /// Generation of the rarely-mutated trie and exception tables.
    pub tables_gen: u64,
    pub is_hyph: bool,
    pub hf: FontId,
    pub max_hyph_char: i32,
    pub init_list: crate::units::MemPtr,
    pub init_lig: bool,
    pub init_lft: bool,
    pub hyphen_passed: usize,
    pub trie_max: crate::hyphenation::TriePtr,
    pub trie_size: i32,
    pub trie_ptr: i32,
    pub hyph_count: i32,
    pub hyph_size: i32,
    pub hyph_next: i32,
    pub trie_op_ptr: i32,
    pub max_op_used: u16,
    pub trie_not_ready: bool,
    pub hyph_start: crate::hyphenation::TriePtr,
    /// The `\lccode` table saved with the current language's patterns.
    pub hyph_index: Option<crate::hyphenation::TriePtr>,
    pub cur_lang: u8,
    pub hyf_char: crate::fonts::LigChar,
    pub l_hyf: i32,
    pub r_hyf: i32,
    pub hyf_bchar: crate::fonts::LigChar,
    pub hc: Vec<i32>,
    /// The word's characters, with boundary sentinels at the ends.
    pub hu: Vec<crate::fonts::LigChar>,
    pub hyf: Vec<u8>,
    pub trie_min: Vec<u32>,
    pub hyf_distance: Vec<u16>,
    pub hyf_num: Vec<u16>,
    pub hyf_next: Vec<u16>,
    pub op_start: Vec<usize>,
    pub trie_used: Vec<u16>,
    pub trie_op_lang: Vec<u8>,
    pub trie_op_val: Vec<u16>,
    pub trie_trl: Vec<i32>,
    pub trie_tro: Vec<i32>,
    pub trie_trc: Vec<u16>,
    pub trie_c: Vec<u16>,
    pub trie_o: Vec<u16>,
    pub trie_l: Vec<u32>,
    pub trie_r: Vec<u32>,
    pub trie_hash: Vec<u32>,
    pub trie_taken: Vec<bool>,
    pub hyph_word: Vec<StrNumber>,
    pub hyph_list: Vec<crate::units::MemPtr>,
    pub hyph_link: Vec<u16>,
    pub trie_op_hash_array: Vec<i32>,
}

impl Default for HyphState {
    fn default() -> Self {
        Self {
            tables_gen: 0,
            is_hyph: false,
            hf: crate::consts::NULL_FONT,
            max_hyph_char: 0,
            init_list: crate::units::MemPtr::NULL,
            init_lig: false,
            init_lft: false,
            hyphen_passed: 0,
            trie_max: crate::hyphenation::TriePtr::NULL,
            trie_size: 0,
            trie_ptr: 0,
            hyph_count: 0,
            hyph_size: 0,
            hyph_next: 0,
            trie_op_ptr: 0,
            max_op_used: 0,
            trie_not_ready: false,
            hyph_start: crate::hyphenation::TriePtr::NULL,
            hyph_index: None,
            cur_lang: 0,
            hyf_char: crate::fonts::LigChar::NONE,
            l_hyf: 0,
            r_hyf: 0,
            hyf_bchar: crate::fonts::LigChar::NONE,
            hc: vec![0; 4099],
            hu: vec![crate::fonts::LigChar::NONE; 4097],
            hyf: vec![0; 4097],
            trie_min: vec![0; 65536],
            hyf_distance: vec![0; 35112],
            hyf_num: vec![0; 35112],
            hyf_next: vec![0; 35112],
            op_start: vec![0; 256],
            trie_used: vec![0; 256],
            trie_op_lang: vec![0; 35112],
            trie_op_val: vec![0; 35112],
            trie_trl: Vec::new(),
            trie_tro: Vec::new(),
            trie_trc: Vec::new(),
            trie_c: Vec::new(),
            trie_o: Vec::new(),
            trie_l: Vec::new(),
            trie_r: Vec::new(),
            trie_hash: Vec::new(),
            trie_taken: Vec::new(),
            hyph_word: Vec::new(),
            hyph_list: Vec::new(),
            hyph_link: Vec::new(),
            trie_op_hash_array: vec![0; 70223],
        }
    }
}

impl InputClusterState {
    /// How much of `buffer` has ever held input.
    #[inline]
    pub(crate) fn buffer_in_use(&self) -> usize {
        (self.max_buf_stack.max(self.last).max(self.first) + 1).index()
    }
}

/// Cap on one run of native-font text, which upstream grows until memory runs out.
pub(crate) const NATIVE_TEXT_MAX: i32 = 1 << 20;

impl ScannerState {
    /// `(cur_cmd, cur_chr)` decoded into one typed command.
    #[inline]
    pub(crate) fn command(&self) -> Result<crate::command::Command, crate::errors::Abort> {
        self.try_command().or_confusion("cur_chr")
    }

    /// `(cur_cmd, cur_chr)` decoded, `None` for a token that is no command.
    #[inline]
    pub(crate) fn try_command(&self) -> Option<crate::command::Command> {
        crate::command::Command::decode(self.cur_cmd, self.cur_chr)
    }

    /// `cur_chr` as the macro a call command carries.
    #[inline]
    pub(crate) fn cur_chr_tokens(&self) -> crate::token::TokenList {
        crate::token::TokenList::from_raw(self.cur_chr)
    }

    /// `cur_chr` as the pointer a macro or register command carries.
    #[inline]
    pub(crate) fn cur_chr_ptr(&self) -> crate::units::MemPtr {
        crate::units::MemPtr::from_raw(self.cur_chr)
    }

    /// Grows `native_text` until `extra` more units fit.
    #[inline(always)]
    pub(crate) fn reserve_native_text(&mut self, extra: i32) -> Result<(), crate::errors::Abort> {
        if extra >= NATIVE_TEXT_MAX - self.native_len {
            return Err(crate::errors::Abort::Overflow {
                capacity: crate::errors::Capacity::NativeTextSize,
                n: NATIVE_TEXT_MAX,
            });
        }
        while self.native_text_size <= self.native_len + extra {
            self.native_text_size += 128;
            self.native_text.resize(self.native_text_size as usize, 0);
        }
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn push_native_text(&mut self, c: u16) {
        self.native_text[self.native_len as usize] = c;
        self.native_len += 1;
    }
}
