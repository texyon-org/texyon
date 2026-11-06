//! Rust-owned engine state.

use crate::cmd::Cmd;
use crate::codes::{FiOrElseCode, ValueLevel};
use crate::params::{DimenPar, GluePar, IntPar, TokPar};
use crate::units::{EqtbPtr, FontId, Scaled, StrNumber};

#[derive(Default)]
pub(crate) struct EngineState {
    pub config: ConfigState,
    pub cancellation: crate::cancellation::CancellationState,
    pub errors: ErrorState,
    pub shipout: ShipoutState,
    pub scanner: ScannerState,
    pub save_stack: SaveStackState,
    pub eqtb: EqtbState,
    pub mem: MemState,
    pub input: InputClusterState,
    pub strings: StringsState,
    pub hyph: HyphState,
    pub fonts: FontState,
    pub linebreak: LinebreakState,
    pub texmf: TexmfState,
    pub synctex: SynctexCtxt,
    pub mainloop: MainLoopState,
    /// Not checkpointed.
    pub pgfmath: crate::pgfmath::Diagnostics,
}

/// Scratch registers of `main_control`'s character and ligature loop.
#[derive(Default, Clone)]
pub(crate) struct MainLoopState {
    pub main_p: crate::units::MemPtr,
    pub main_f: FontId,
    pub main_h: i32,
    pub main_pp: crate::units::MemPtr,
    pub main_ppp: crate::units::MemPtr,
    pub bchar: i32,
    pub false_bchar: i32,
    /// Inter-character space class of the current / previous char.
    pub space_class: i32,
    pub prev_class: i32,
    pub cur_l: i32,
    pub cur_r: i32,
    pub cur_q: crate::units::MemPtr,
    pub lig_stack: crate::units::MemPtr,
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
    pub node: crate::units::MemPtr,
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
            node: crate::units::MemPtr::default(),
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

/// One level of TeX's input stack (`in_state_record`).
#[derive(Copy, Clone, Default)]
pub(crate) struct InputState {
    pub state: u16,
    pub index: u16,
    pub start: i32,
    pub loc: i32,
    pub limit: i32,
    pub name: i32,
    pub synctex_tag: i32,
}

impl InputState {
    /// `name` of a macro level: the control sequence being expanded.
    #[inline]
    pub(crate) fn macro_name(&self) -> EqtbPtr {
        EqtbPtr::from_raw(self.name)
    }

    #[inline]
    pub(crate) fn set_macro_name(&mut self, cs: EqtbPtr) {
        self.name = cs.raw();
    }
}

/// `start` and `loc` of a token-list level, which point into `mem`.
impl InputState {
    #[inline]
    pub(crate) fn token_start(&self) -> crate::token::TokenList {
        crate::token::TokenList::from_ptr(crate::units::MemPtr::from_raw(self.start))
    }
    #[inline]
    pub(crate) fn token_loc(&self) -> crate::token::TokenList {
        crate::token::TokenList::from_ptr(crate::units::MemPtr::from_raw(self.loc))
    }
    #[inline]
    pub(crate) fn set_token_loc(&mut self, p: crate::token::TokenList) {
        self.loc = p.ptr().raw();
    }
    /// Starts reading the token list `p` from its first node.
    #[inline]
    pub(crate) fn set_token_list(&mut self, p: crate::token::TokenList) {
        self.start = p.ptr().raw();
        self.loc = p.ptr().raw();
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
    pub param_stack: Vec<crate::token::TokenList>,
    /// The last node of each `param_stack` list, so it can be freed without a walk.
    pub param_tail: Vec<crate::token::TokenList>,
    pub line_stack: Vec<i32>,
    pub grp_stack: Vec<i32>,
    pub if_stack: Vec<crate::units::MemPtr>,
    pub source_filename_stack: Vec<StrNumber>,
    pub full_source_filename_stack: Vec<StrNumber>,
    pub eof_seen: Vec<bool>,
    pub buffer: Vec<i32>,
    pub input_stack: Vec<InputState>,
    pub cur_input: InputState,
    pub input_file: Vec<InputSlot>,
    pub nest: Vec<crate::mem_types::ListStateRecord>,
    pub cur_list: crate::mem_types::ListStateRecord,
    pub open_parens: i32,
    pub in_open: i32,
    pub pseudo_files: crate::units::MemPtr,
    pub nest_ptr: i32,
    pub max_nest_stack: i32,
    pub input_ptr: i32,
    pub stack_size: i32,
    pub buf_size: i32,
    pub param_size: i32,
    pub max_in_open: i32,
    pub nest_size: i32,
    pub max_in_stack: i32,
    pub base_ptr: i32,
    pub par_loc: crate::units::EqtbPtr,
    pub par_token: crate::token::Token,
    pub param_ptr: i32,
    pub max_param_stack: i32,
    pub force_eof: bool,
    pub first: i32,
    pub last: i32,
    pub max_buf_stack: i32,
    pub read_open: [u8; 17],
    pub read_file: [InputSlot; 16],
    /// UTF-8, and cut short by a NUL between `check_for_tfm_font_mapping` and the open.
    pub name_of_file: Vec<u8>,
    /// Name of the most recently opened input file.
    pub name_of_input_file: Vec<u8>,
    pub abspath_of_input_file: Vec<u8>,
}

pub(crate) struct EqtbState {
    pub hash_used: EqtbPtr,
    pub hash_high: i32,
    pub no_new_control_sequence: bool,
    pub prim_used: i32,
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
            prim_used: 0,
            cs_count: 0,
            data: Vec::new(),
            // PRIM_SIZE + 1 = 2101.
            prim: vec![crate::mem_types::B32x2 { s0: 0, s1: 0 }; 2101],
            // EQTB_SIZE - INT_BASE + 1.
            xeq_level: vec![0u16; 1_114_730],
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

    #[inline]
    pub fn prim_s0(&self, p: i32) -> i32 {
        self.prim[p as usize].s0
    }

    #[inline]
    pub fn set_prim_s0(&mut self, p: i32, v: i32) {
        let index = p as usize;
        if self.journal.prim.armed() {
            self.journal.prim.note(index, self.prim[index]);
        }
        self.prim[index].s0 = v;
    }

    #[inline]
    pub fn prim_s1(&self, p: i32) -> i32 {
        self.prim[p as usize].s1
    }

    #[inline]
    pub fn set_prim_s1(&mut self, p: i32, v: i32) {
        let index = p as usize;
        if self.journal.prim.armed() {
            self.journal.prim.note(index, self.prim[index]);
        }
        self.prim[index].s1 = v;
    }

    #[inline]
    pub fn word(&self, p: EqtbPtr) -> crate::mem_types::MemoryWord {
        self.data[p.index()]
    }

    #[inline]
    fn b16s1(&self, p: EqtbPtr) -> u16 {
        self.data[p.index()].b16().s1
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
        crate::token::TokenList::from_ptr(self.equiv_ptr(p))
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
        FontId::from_raw(self.equiv(crate::consts::CUR_FONT_LOC))
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
    pub(crate) fn skip(&self, n: i32) -> crate::node::NodePtr<crate::node::GlueSpecNode> {
        crate::node::NodePtr::from_glue_ptr(self.equiv_ptr(crate::consts::SKIP_BASE + n))
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
        FontId::from_raw(self.equiv(crate::consts::MATH_FONT_BASE + n))
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
    pub fn glue_par(&self, n: GluePar) -> crate::node::NodePtr<crate::node::GlueSpecNode> {
        crate::node::NodePtr::from_glue_ptr(self.equiv_ptr(n.loc()))
    }

    #[inline]
    pub fn set_glue_par(
        &mut self,
        n: GluePar,
        spec: crate::node::NodePtr<crate::node::GlueSpecNode>,
    ) {
        self.set_b32s1(n.loc(), spec.ptr().raw());
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
    pub data: Vec<crate::mem_types::MemoryWord>,
    /// Owned shaped-glyph payloads referenced by handles in native nodes.
    pub glyph_blobs: crate::glyph_arena::GlyphBlobArena,
    pub journal: crate::journal::WordJournal,
    pub lo_mem_max: crate::units::MemPtr,
    pub hi_mem_min: crate::units::MemPtr,
    pub avail: crate::units::MemPtr,
    pub mem_end: crate::units::MemPtr,
    pub rover: crate::units::MemPtr,
    pub var_used: i32,
    pub dyn_used: i32,
    /// TeX's `line`, kept here because `get_node` stamps it into new nodes.
    pub line: i32,
    pub cur_synctex_tag: i32,
    pub temp_ptr: crate::units::MemPtr,
}

impl MemState {
    #[inline(always)]
    fn note(&mut self, index: usize) {
        if self.journal.armed() {
            self.journal.note(index, self.data[index]);
        }
    }

    #[inline]
    pub(crate) fn b32s0(&self, p: impl crate::units::MemAddr) -> i32 {
        self.data[p.addr() as usize].b32().s0
    }

    #[inline]
    pub(crate) fn b32s1(&self, p: impl crate::units::MemAddr) -> i32 {
        self.data[p.addr() as usize].b32().s1
    }

    #[inline]
    pub(crate) fn b16s0(&self, p: impl crate::units::MemAddr) -> u16 {
        self.data[p.addr() as usize].b16().s0
    }

    #[inline]
    pub(crate) fn b16s1(&self, p: impl crate::units::MemAddr) -> u16 {
        self.data[p.addr() as usize].b16().s1
    }

    #[inline]
    pub(crate) fn b16s2(&self, p: impl crate::units::MemAddr) -> u16 {
        self.data[p.addr() as usize].b16().s2
    }

    #[inline]
    pub(crate) fn b16s3(&self, p: impl crate::units::MemAddr) -> u16 {
        self.data[p.addr() as usize].b16().s3
    }

    #[inline]
    pub(crate) fn set_b32s0(&mut self, p: impl crate::units::MemAddr, v: i32) {
        let index = p.addr() as usize;
        self.note(index);
        self.data[index].b32.s0 = v;
    }

    #[inline]
    pub(crate) fn set_b32s1(&mut self, p: impl crate::units::MemAddr, v: i32) {
        let index = p.addr() as usize;
        self.note(index);
        self.data[index].b32.s1 = v;
    }

    #[inline]
    pub(crate) fn set_b16s0(&mut self, p: impl crate::units::MemAddr, v: u16) {
        let index = p.addr() as usize;
        self.note(index);
        self.data[index].b16.s0 = v;
    }

    #[inline]
    pub(crate) fn set_b16s1(&mut self, p: impl crate::units::MemAddr, v: u16) {
        let index = p.addr() as usize;
        self.note(index);
        self.data[index].b16.s1 = v;
    }

    #[inline]
    pub(crate) fn set_b16s2(&mut self, p: impl crate::units::MemAddr, v: u16) {
        let index = p.addr() as usize;
        self.note(index);
        self.data[index].b16.s2 = v;
    }

    #[inline]
    pub(crate) fn set_b16s3(&mut self, p: impl crate::units::MemAddr, v: u16) {
        let index = p.addr() as usize;
        self.note(index);
        self.data[index].b16.s3 = v;
    }

    #[inline]
    pub(crate) fn set_gr(&mut self, p: impl crate::units::MemAddr, v: f64) {
        let index = p.addr() as usize;
        self.note(index);
        self.data[index].gr = v;
    }

    /// `mem` as UTF-16 units, four per word, for text stored inline in nodes.
    #[inline]
    pub(crate) fn units(&self) -> &[u16] {
        // SAFETY: a `MemoryWord` is 8 bytes of plain data, 8-aligned, so its
        // storage reads as four valid `u16`s.
        unsafe { std::slice::from_raw_parts(self.data.as_ptr().cast(), self.data.len() * 4) }
    }

    /// `units`, writable. Journaling the words written is up to the caller.
    #[inline]
    pub(crate) fn units_mut(&mut self) -> &mut [u16] {
        // SAFETY: as in `units`, and any `u16`s written form valid words.
        unsafe {
            std::slice::from_raw_parts_mut(self.data.as_mut_ptr().cast(), self.data.len() * 4)
        }
    }

    /// `mem` as bytes, eight per word.
    #[inline]
    pub(crate) fn bytes(&self) -> &[u8] {
        // SAFETY: as in `units`.
        unsafe { std::slice::from_raw_parts(self.data.as_ptr().cast(), self.data.len() * 8) }
    }

    /// `bytes`, writable. Journaling the words written is up to the caller.
    #[inline]
    pub(crate) fn bytes_mut(&mut self) -> &mut [u8] {
        // SAFETY: as in `units_mut`.
        unsafe {
            std::slice::from_raw_parts_mut(self.data.as_mut_ptr().cast(), self.data.len() * 8)
        }
    }

    #[inline]
    pub(crate) fn word(&self, p: impl crate::units::MemAddr) -> crate::mem_types::MemoryWord {
        self.data[p.addr() as usize]
    }

    #[inline]
    pub(crate) fn set_word(
        &mut self,
        p: impl crate::units::MemAddr,
        w: crate::mem_types::MemoryWord,
    ) {
        let index = p.addr() as usize;
        self.note(index);
        self.data[index] = w;
    }

    /// Journals `words` words from `p` before a raw pointer writes them.
    pub(crate) fn journal_mark_range(&mut self, p: impl crate::units::MemAddr, words: usize) {
        if !self.journal.armed() {
            return;
        }
        let start = p.addr() as usize;
        for index in start..start + words {
            self.journal.note(index, self.data[index]);
        }
    }
}

pub(crate) fn native_text_words(units: i32) -> usize {
    (units.max(0) as usize * std::mem::size_of::<u16>())
        .div_ceil(std::mem::size_of::<crate::mem_types::MemoryWord>())
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

impl GroupCode {
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

#[derive(Default)]
pub(crate) struct SaveStackState {
    pub save_ptr: i32,
    pub max_save_stack: i32,
    pub save_size: i32,
    pub cur_level: u16,
    pub cur_group: GroupCode,
    pub cur_boundary: i32,
    pub data: Vec<crate::mem_types::MemoryWord>,
}

impl SaveStackState {
    #[inline]
    fn b32s1(&self, k: i32) -> i32 {
        self.data[k as usize].b32().s1
    }

    #[inline]
    fn set_b32s1(&mut self, k: i32, v: i32) {
        self.data[k as usize].b32.s1 = v;
    }

    #[inline]
    fn set_b16s1(&mut self, k: i32, v: u16) {
        self.data[k as usize].b16.s1 = v;
    }

    #[inline]
    fn set_b16s0(&mut self, k: i32, v: u16) {
        self.data[k as usize].b16.s0 = v;
    }

    #[inline]
    pub(crate) fn set_word(&mut self, k: i32, w: crate::mem_types::MemoryWord) {
        self.data[k as usize] = w;
    }

    #[inline]
    fn b16s1(&self, k: i32) -> u16 {
        self.data[k as usize].b16().s1
    }

    #[inline]
    fn b16s0(&self, k: i32) -> u16 {
        self.data[k as usize].b16().s0
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
    pub(crate) fn save_type(&self, k: i32) -> u16 {
        self.b16s1(k)
    }
    #[inline]
    pub(crate) fn save_level(&self, k: i32) -> u16 {
        self.b16s0(k)
    }
    #[inline]
    pub(crate) fn save_index(&self, k: i32) -> i32 {
        self.b32s1(k)
    }
    /// `save_index(k)` of an entry that saved an `eqtb` location.
    #[inline]
    pub(crate) fn save_loc(&self, k: i32) -> EqtbPtr {
        EqtbPtr::from_raw(self.b32s1(k))
    }
    /// `save_index(k)` of an entry that saved a pointer.
    #[inline]
    pub(crate) fn save_index_ptr(&self, k: i32) -> crate::units::MemPtr {
        crate::units::MemPtr::from_raw(self.save_index(k))
    }
    /// Writes a restore entry at `save_ptr`.
    #[inline]
    pub(crate) fn set_save_entry(&mut self, kind: u16, level: u16, index: i32) {
        let k = self.save_ptr;
        self.set_b16s1(k, kind);
        self.set_b16s0(k, level);
        self.set_b32s1(k, index);
    }

    /// The group and enclosing boundary saved by the level boundary at `k`.
    #[inline]
    pub(crate) fn boundary_at(&self, k: i32) -> (GroupCode, i32) {
        let group = GroupCode::from_raw(self.b16s0(k) as u8).expect("valid group code");
        (group, self.b32s1(k))
    }

    #[inline]
    pub(crate) fn word(&self, k: i32) -> crate::mem_types::MemoryWord {
        self.data[k as usize]
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
}

impl std::ops::Index<ValueLevel> for SaRoots {
    type Output = crate::units::MemPtr;
    #[inline]
    fn index(&self, t: ValueLevel) -> &crate::units::MemPtr {
        &self.0[t as usize]
    }
}

impl std::ops::IndexMut<ValueLevel> for SaRoots {
    #[inline]
    fn index_mut(&mut self, t: ValueLevel) -> &mut crate::units::MemPtr {
        &mut self.0[t as usize]
    }
}

#[derive(Default, Clone)]
pub(crate) struct ScannerState {
    /// Monotonic token-fetch clock used to order checkpoint fences.
    pub logical_clock: u64,
    pub cur_name: StrNumber,
    pub cur_area: StrNumber,
    pub cur_ext: StrNumber,
    pub area_delimiter: i32,
    pub ext_delimiter: i32,
    pub quoted_filename: bool,
    pub file_name_quote_char: u16,
    pub name_in_progress: bool,
    pub stop_at_space: bool,
    pub cur_align: crate::units::MemPtr,
    pub cur_span: crate::units::MemPtr,
    pub cur_loop: crate::units::MemPtr,
    pub align_ptr: crate::units::MemPtr,
    pub cur_head: crate::units::MemPtr,
    pub cur_tail: crate::units::MemPtr,
    pub cur_pre_head: crate::units::MemPtr,
    pub cur_pre_tail: crate::units::MemPtr,
    pub shown_mode: i16,
    pub mag_set: i32,
    pub radix: u8,
    pub expand_depth_count: i32,
    pub sa_chain: crate::units::MemPtr,
    pub sa_level: u16,
    pub max_reg_num: i32,
    pub cond_ptr: crate::units::MemPtr,
    pub if_limit: FiOrElseCode,
    pub cur_if: i16,
    pub if_line: i32,
    pub skip_line: i32,
    pub is_in_csname: bool,
    pub used_tectonic_coda_tokens: bool,
    pub cur_val_level: ValueLevel,
    pub cur_val1: i32,
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
    pub cur_val: i32,
    pub cur_ptr: crate::units::MemPtr,
    pub cur_mark: CurMarks,
    pub disc_ptr: [crate::units::MemPtr; 4],
    pub cur_mlist: crate::units::MemPtr,
    pub cur_style: i16,
    pub cur_size: i32,
    pub cur_mu: Scaled,
    pub mlist_penalties: bool,
    pub cur_box: Option<crate::node::BoxOrRule>,
    pub after_token: crate::token::Token,
    pub cur_order: u16,
    pub adjust_tail: crate::units::MemPtr,
    pub pre_adjust_tail: crate::units::MemPtr,
    pub total_stretch: [Scaled; 4],
    pub total_shrink: [Scaled; 4],
    pub cur_f: FontId,
    pub cur_c: i32,
    pub cur_i: crate::fonts::CharInfo,
    pub pack_begin_line: i32,
    pub set_box_allowed: bool,
    pub active_width: [Scaled; 7],
    pub best_height_plus_depth: Scaled,
    pub lr_ptr: crate::units::MemPtr,
    pub lr_problems: i32,
    pub cur_dir: i16,
    pub sa_root: SaRoots,
    pub sa_null: crate::mem_types::MemoryWord,
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
    /// TFM mapping name captured during filename parsing and consumed by the
    /// subsequent font load.
    pub saved_mapping_name: Option<Vec<u8>>,
    /// Per-engine ICU line-break cache and its string-pool locale key.
    pub brk_iter: Option<LinebreakHandle>,
    pub brk_locale_str_num: StrNumber,
    pub font_layout_engine: PerFont<Option<std::rc::Rc<FontEngineHandle>>>,
    pub font_mapping: PerFont<Option<std::sync::Arc<FontMappingHandle>>>,
    pub loaded_font_letter_space: Scaled,
    pub xtx_ligature_present: bool,
    pub gave_char_warning_help: bool,
    pub native_font_type_flag: StrNumber,
    pub font_mem_size: i32,
    pub font_max: FontId,
    pub fmem_ptr: i32,
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
    pub bchar_label: PerFont<i32>,
    pub font_bchar: PerFont<i32>,
    pub font_false_bchar: PerFont<i32>,
    pub font_glue: PerFont<crate::units::MemPtr>,
    pub font_used: PerFont<bool>,
    pub char_base: PerFont<i32>,
    pub width_base: PerFont<i32>,
    pub height_base: PerFont<i32>,
    pub depth_base: PerFont<i32>,
    pub italic_base: PerFont<i32>,
    pub lig_kern_base: PerFont<i32>,
    pub kern_base: PerFont<i32>,
    pub exten_base: PerFont<i32>,
    pub param_base: PerFont<i32>,
    pub hyphen_char: PerFont<i32>,
    pub skew_char: PerFont<i32>,
    pub font_info: Vec<crate::mem_types::MemoryWord>,
    pub font_info_journal: crate::journal::PlainTrail<crate::mem_types::MemoryWord>,
    /// Font-memory high-water mark at the start of the open journal epoch.
    pub fmem_epoch_base: i32,
    pub mapped_text: Vec<u16>,
}

impl Default for FontState {
    fn default() -> Self {
        Self {
            loaded_font_mapping: None,
            loaded_font_flags: 0,
            saved_mapping_name: None,
            brk_iter: None,
            brk_locale_str_num: StrNumber::from_raw(0),
            font_layout_engine: PerFont::default(),
            font_mapping: PerFont::default(),
            loaded_font_letter_space: Scaled::ZERO,
            xtx_ligature_present: false,
            gave_char_warning_help: false,
            native_font_type_flag: StrNumber::NONE,
            font_mem_size: 0,
            font_max: crate::consts::NULL_FONT,
            fmem_ptr: 0,
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
            fmem_epoch_base: 0,
            mapped_text: Vec::new(),
        }
    }
}

impl FontState {
    #[inline]
    pub(crate) fn font_info_b16(&self, k: i32) -> crate::mem_types::B16x4 {
        self.font_info[k as usize].b16()
    }

    /// `font_info[k].sc`.
    #[inline]
    pub(crate) fn font_info_sc(&self, k: i32) -> Scaled {
        Scaled::from_raw(self.font_info[k as usize].b32().s1)
    }

    /// Font parameter `n` of font `f`.
    #[inline]
    pub(crate) fn param(&self, f: FontId, n: i32) -> Scaled {
        self.font_info_sc(n + self.param_base[f])
    }

    #[inline]
    pub(crate) fn set_font_info_b16(&mut self, k: i32, v: crate::mem_types::B16x4) {
        let index = k as usize;
        if self.font_info_journal.armed() && k < self.fmem_epoch_base {
            self.font_info_journal.note(index, self.font_info[index]);
        }
        self.font_info[index].b16 = v;
    }

    #[inline]
    pub(crate) fn set_font_info_sc(&mut self, k: i32, v: Scaled) {
        let index = k as usize;
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
    pub last_glue: Option<crate::node::NodePtr<crate::node::GlueSpecNode>>,
    pub last_penalty: i32,
    pub last_kern: Scaled,
    pub last_node_type: i32,
    pub insert_penalties: i32,
    pub output_active: bool,
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
    pub current_diagnostic: Option<crate::bridge::Diagnostic>,
    pub best_page_break: crate::units::MemPtr,
    pub best_size: Scaled,
    pub least_page_cost: i32,
    pub page_max_depth: Scaled,
    pub total_pages: i32,
    pub max_v: Scaled,
    pub max_h: Scaled,
    pub doing_leaders: bool,
    pub rule_ht: Scaled,
    pub rule_dp: Scaled,
    pub rule_wd: Scaled,
    pub cur_h: Scaled,
    pub cur_v: Scaled,
    pub cur_page_width: Scaled,
    pub cur_page_height: Scaled,
    pub cur_h_offset: Scaled,
    pub cur_v_offset: Scaled,
    pub write_loc: crate::units::EqtbPtr,
    pub output_file_name: StrNumber,
    pub down_ptr: crate::units::MemPtr,
    pub right_ptr: crate::units::MemPtr,
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
    pub arith_error: bool,
    pub tex_remainder: i32,
    pub dig: [u8; 23],
    pub randoms: [i32; 55],
    pub j_random: u8,
    pub help_lines: [&'static str; 6],
    pub help_len: u8,
    pub two_to_the: [i32; 31],
    pub spec_log: [i32; 29],
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
    pub random_seed: i32,
    pub no_shrink_error_yet: bool,
    pub font_in_short_display: FontId,
    pub depth_threshold: i32,
    pub breadth_max: i32,
}

impl Default for ErrorState {
    fn default() -> Self {
        Self {
            arith_error: false,
            tex_remainder: 0,
            dig: [0; 23],
            randoms: [0; 55],
            j_random: 0,
            help_lines: [""; 6],
            help_len: 0,
            two_to_the: [0; 31],
            spec_log: [0; 29],
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
            random_seed: 0,
            no_shrink_error_yet: false,
            font_in_short_display: crate::consts::NULL_FONT,
            depth_threshold: 0,
            breadth_max: 0,
        }
    }
}

#[derive(Default)]
pub(crate) struct StringsState {
    pub str_ptr: i32,
    pub max_strings: i32,
    pub init_str_ptr: i32,
    pub init_pool_ptr: i32,
    pub job_name: StrNumber,
    pub texmf_log_name: StrNumber,
    pub string_vacancies: i32,
    pub pool_size: i32,
    pub pool_ptr: i32,
    pub format_ident: StrNumber,
    pub edit_name_start: i32,
    pub str_start: Vec<i32>,
    pub str_pool: Vec<u16>,
}

impl StringsState {
    /// The `str_start` slot of the string under construction.
    #[inline]
    pub(crate) fn cur_start_slot(&self) -> usize {
        (self.str_ptr - crate::consts::TOO_BIG_CHAR).max(0) as usize
    }

    /// The pool offset where the string under construction starts.
    #[inline]
    pub(crate) fn cur_string_start(&self) -> i32 {
        self.str_start[(self.str_ptr - crate::consts::TOO_BIG_CHAR) as usize]
    }

    /// TeX's `append_char`: adds UTF-16 unit `c` to the string under construction.
    #[inline]
    pub(crate) fn append_char(&mut self, c: u16) {
        self.str_pool[self.pool_ptr as usize] = c;
        self.pool_ptr += 1;
    }

    /// Appends UTF-16 `units` to the string under construction.
    #[inline]
    pub(crate) fn append_units(&mut self, units: &[u16]) {
        let start = self.pool_ptr as usize;
        self.str_pool[start..start + units.len()].copy_from_slice(units);
        self.pool_ptr += units.len() as i32;
    }

    /// The characters of the string under construction.
    #[inline]
    pub(crate) fn cur_string(&self) -> &[u16] {
        &self.str_pool[self.cur_string_start() as usize..self.pool_ptr as usize]
    }

    /// Drops the characters of the string under construction.
    #[inline]
    pub(crate) fn discard_cur_string(&mut self) {
        self.pool_ptr = self.cur_string_start();
    }
}

/// The line breaker's per-paragraph scratch.
#[derive(Clone)]
pub(crate) struct LinebreakState {
    pub last_leftmost_char: crate::units::MemPtr,
    pub last_rightmost_char: crate::units::MemPtr,
    pub hlist_stack: Vec<crate::units::MemPtr>,
    pub hlist_stack_level: i16,
    pub first_p: crate::units::MemPtr,
    pub global_prev_p: crate::units::MemPtr,
    pub cur_p: crate::units::MemPtr,
    pub last_special_line: i32,
    pub first_width: Scaled,
    pub second_width: Scaled,
    pub first_indent: Scaled,
    pub second_indent: Scaled,
    /// The active node of the chosen breakpoint, once there is one.
    pub best_line: i32,
    pub hn: u16,
    pub ha: crate::units::MemPtr,
    pub hb: crate::units::MemPtr,
    pub passive: crate::units::MemPtr,
    pub cur_active_width: [Scaled; 7],
    pub background: [Scaled; 7],
    pub break_width: [Scaled; 7],
    pub best_place: [crate::units::MemPtr; 4],
    pub best_pl_line: [i32; 4],
    pub disc_width: Scaled,
    pub final_pass: bool,
    pub threshold: i32,
    pub minimal_demerits: [i32; 4],
    pub minimum_demerits: i32,
    pub easy_line: i32,
    pub do_last_line_fit: bool,
    pub active_node_size: i16,
    pub fill_width: [Scaled; 3],
    pub best_pl_short: [Scaled; 4],
    pub best_pl_glue: [Scaled; 4],
}

impl Default for LinebreakState {
    fn default() -> Self {
        Self {
            last_leftmost_char: crate::units::MemPtr::default(),
            last_rightmost_char: crate::units::MemPtr::default(),
            hlist_stack: vec![crate::units::MemPtr::default(); 513],
            hlist_stack_level: 0,
            first_p: crate::units::MemPtr::default(),
            global_prev_p: crate::units::MemPtr::default(),
            cur_p: crate::units::MemPtr::default(),
            last_special_line: 0,
            first_width: Scaled::ZERO,
            second_width: Scaled::ZERO,
            first_indent: Scaled::ZERO,
            second_indent: Scaled::ZERO,
            best_line: 0,
            hn: 0,
            ha: crate::units::MemPtr::default(),
            hb: crate::units::MemPtr::default(),
            passive: crate::units::MemPtr::default(),
            cur_active_width: [Scaled::ZERO; 7],
            background: [Scaled::ZERO; 7],
            break_width: [Scaled::ZERO; 7],
            best_place: [crate::units::MemPtr::default(); 4],
            best_pl_line: [0; 4],
            disc_width: Scaled::ZERO,
            final_pass: false,
            threshold: 0,
            minimal_demerits: [0; 4],
            minimum_demerits: 0,
            easy_line: 0,
            do_last_line_fit: false,
            active_node_size: 0,
            fill_width: [Scaled::ZERO; 3],
            best_pl_short: [Scaled::ZERO; 4],
            best_pl_glue: [Scaled::ZERO; 4],
        }
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
    pub trie_max: i32,
    pub trie_size: i32,
    pub trie_ptr: i32,
    pub hyph_count: i32,
    pub hyph_size: i32,
    pub hyph_next: i32,
    pub trie_op_ptr: i32,
    pub max_op_used: u16,
    pub trie_not_ready: bool,
    pub hyph_start: i32,
    pub hyph_index: usize,
    pub cur_lang: u8,
    pub hyf_char: i32,
    pub l_hyf: i32,
    pub r_hyf: i32,
    pub init_l_hyf: i32,
    pub init_r_hyf: i32,
    pub hyf_bchar: i32,
    pub hc: Vec<i32>,
    pub hu: Vec<i32>,
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
            init_list: crate::units::MemPtr::default(),
            init_lig: false,
            init_lft: false,
            hyphen_passed: 0,
            trie_max: 0,
            trie_size: 0,
            trie_ptr: 0,
            hyph_count: 0,
            hyph_size: 0,
            hyph_next: 0,
            trie_op_ptr: 0,
            max_op_used: 0,
            trie_not_ready: false,
            hyph_start: 0,
            hyph_index: 0,
            cur_lang: 0,
            hyf_char: 0,
            l_hyf: 0,
            r_hyf: 0,
            init_l_hyf: 0,
            init_r_hyf: 0,
            hyf_bchar: 0,
            hc: vec![0; 4099],
            hu: vec![0; 4097],
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
    pub(crate) fn buffer_in_use(&self) -> i32 {
        self.max_buf_stack.max(self.last).max(self.first) + 1
    }

    /// `name_of_file` up to a font mapping cut.
    pub(crate) fn name_of_file_bytes(&self) -> &[u8] {
        let name = &self.name_of_file;
        &name[..name.iter().position(|&b| b == 0).unwrap_or(name.len())]
    }

    pub(crate) fn name_of_file16(&self) -> Vec<u16> {
        let mut s = &self.name_of_file[..];
        let mut units = Vec::with_capacity(s.len());
        while !s.is_empty() {
            let (c, n) = crate::utf8::utf16_units(crate::utf8::next_utf8_char(&mut s));
            units.extend_from_slice(&c[..n]);
        }
        units
    }
}

/// Cap on one run of native-font text, which upstream grows until memory runs out.
pub(crate) const NATIVE_TEXT_MAX: i32 = 1 << 20;

impl ScannerState {
    /// `cur_chr` of a command whose chr code is an `eqtb` location.
    #[inline(always)]
    pub(crate) fn chr_loc(&self) -> EqtbPtr {
        EqtbPtr::from_raw(self.cur_chr)
    }

    /// `cur_val` as the dimension that `scan_dimen` left there.
    #[inline]
    pub(crate) fn cur_dimen(&self) -> Scaled {
        Scaled::from_raw(self.cur_val)
    }

    #[inline]
    pub(crate) fn set_cur_dimen(&mut self, d: Scaled) {
        self.cur_val = d.raw();
    }

    /// `cur_val` as the glue spec that `scan_glue` left there.
    #[inline]
    pub(crate) fn cur_glue(&self) -> crate::node::NodePtr<crate::node::GlueSpecNode> {
        crate::node::NodePtr::from_glue_ptr(self.cur_val_ptr())
    }

    /// `cur_val` as a pointer, such as a token list.
    #[inline]
    pub(crate) fn cur_val_ptr(&self) -> crate::units::MemPtr {
        crate::units::MemPtr::from_raw(self.cur_val)
    }

    /// `cur_val` as a token list.
    #[inline]
    pub(crate) fn cur_val_tokens(&self) -> crate::token::TokenList {
        crate::token::TokenList::from_ptr(self.cur_val_ptr())
    }

    /// `cur_val` at level `ident`: the control sequence of a font identifier.
    #[inline]
    pub(crate) fn cur_val_loc(&self) -> EqtbPtr {
        EqtbPtr::from_raw(self.cur_val)
    }

    /// `cur_chr` as the macro a call command carries.
    #[inline]
    pub(crate) fn cur_chr_tokens(&self) -> crate::token::TokenList {
        crate::token::TokenList::from_ptr(self.cur_chr_ptr())
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
                what: "native text size",
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
