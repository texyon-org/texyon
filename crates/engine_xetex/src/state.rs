//! Rust-owned engine state.

use crate::base::cmd::Cmd;
use crate::base::codes::FiOrElseCode;
use crate::base::params::{DimenPar, GluePar, IntPar, TokPar};
use crate::base::units::{BufPos, FontId, PoolPos, PrimNum, SavePtr, Scaled, StrNumber};
use crate::memory::glue_store::GlueSpecId;
use crate::tables::cs::{CsId, Frozen};

#[derive(Default)]
pub(crate) struct EngineState {
    pub config: ConfigState,
    pub cancellation: crate::preview::cancellation::CancellationState,
    pub errors: ErrorState,
    pub random: crate::base::random::Random,
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
    pub interchar: InterCharState,
    /// Not checkpointed.
    pub pgfmath: crate::scanning::pgfmath::Diagnostics,
    /// Not checkpointed: a restore keeps the operation's list.
    pub diagnostics: crate::print::diagnostic::DiagnosticsState,
}

/// XeTeX's inter-character class registers. The big switch reads them
/// between runs of the main loop.
#[derive(Default, Clone)]
pub(crate) struct InterCharState {
    /// The class of the last character the main loop classed.
    pub space_class: i32,
    /// The class before it, the boundary class when tokens were just inserted.
    pub prev_class: i32,
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
    pub node: crate::base::units::MemPtr,
    /// A kern whose record waits for the context to change. When set it equals
    /// `node`, so flush sites record it before reassigning `node`.
    pub deferred_kern: Option<crate::memory::node::NodePtr<crate::memory::node::KernNode>>,
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
    pub preview_index: Option<crate::preview::sync_index::PreviewSyncIndex>,
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
    pub grp: Option<crate::base::units::SavePtr>,
    /// `cond_ptr` when the file was opened.
    pub cond: Option<crate::scanning::cond_stack::CondId>,
    pub source_name: StrNumber,
    pub full_source_name: StrNumber,
    /// Whether `\everyeof` has been inserted at its end.
    pub eof_seen: bool,
}

/// A macro argument on the parameter stack, with its last node so it can be
/// freed without a walk.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub(crate) struct MacroParam {
    pub list: crate::base::token::TokenList,
}

/// One level of TeX's input stack (`in_state_record`).
#[derive(Copy, Clone, Default)]
pub(crate) struct InputState {
    pub state: u16,
    pub index: u16,
    /// A text level's line in the buffer, unused on token-list levels.
    pub start: BufPos,
    pub loc: BufPos,
    pub limit: BufPos,
    pub name: InputName,
    pub synctex_tag: i32,
    /// A token-list level's list, which it frees or drops a reference to when it ends.
    tok_start: crate::base::token::TokenList,
    /// The next token a token-list level reads.
    tok_loc: crate::base::token::TokCursor,
    /// Where a macro level's arguments start on the parameter stack.
    param_start: u32,
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
    Macro(CsId),
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
    pub(crate) fn buffer_overflow(&self) -> crate::print::errors::Abort {
        crate::print::errors::Abort::Overflow {
            capacity: crate::print::errors::Capacity::BufferSize,
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
        usize::try_from(self.param_start).unwrap_or(usize::MAX)
    }

    #[inline]
    pub(crate) fn set_param_start(&mut self, n: usize) {
        self.param_start = u32::try_from(n).unwrap_or(u32::MAX);
    }
}

/// `start` and `loc` of a token-list level, which point into the token store.
impl InputState {
    #[inline]
    pub(crate) fn token_start(&self) -> crate::base::token::TokenList {
        self.tok_start
    }
    #[inline]
    pub(crate) fn token_loc(&self) -> crate::base::token::TokCursor {
        self.tok_loc
    }
    #[inline]
    pub(crate) fn set_token_loc(&mut self, c: crate::base::token::TokCursor) {
        self.tok_loc = c;
    }
    #[inline]
    pub(crate) fn set_token_start(&mut self, list: crate::base::token::TokenList) {
        self.tok_start = list;
    }
}

/// An owned main-input or `\read` stream slot.
#[derive(Default)]
pub(crate) enum InputSlot {
    #[default]
    Empty,
    /// The terminal, which has no file.
    Terminal,
    Open(Box<crate::files::io::UFile>),
}

#[derive(Default)]
pub(crate) struct InputClusterState {
    pub param_stack: crate::base::stack::TexStack<MacroParam>,
    /// Per open input file, indexed by `in_open`. Level 0 is the terminal.
    pub open_levels: Vec<OpenLevel>,
    pub buffer: InputBuffer,
    pub input_stack: crate::base::stack::TexStack<InputState>,
    pub cur_input: InputState,
    pub input_file: Vec<InputSlot>,
    pub nest: crate::base::stack::TexStack<crate::memory::mem_types::ListStateRecord>,
    pub cur_list: crate::memory::mem_types::ListStateRecord,
    pub open_parens: i32,
    pub in_open: usize,
    pub pseudo_files: crate::base::units::MemPtr,
    /// The last usable `buffer` position.
    pub buf_size: BufPos,
    pub max_in_open: usize,
    /// The level `show_context` and its relatives are looking at.
    pub base_ptr: usize,
    pub par_loc: Option<CsId>,
    pub par_token: crate::base::token::Token,
    pub force_eof: bool,
    pub first: BufPos,
    pub last: BufPos,
    pub max_buf_stack: BufPos,
    pub read_open: [u8; 17],
    pub read_file: [InputSlot; 16],
}

pub(crate) struct EqtbState {
    /// The lowest bucket handed out to a colliding name so far.
    pub hash_used: crate::tables::cs::NameSlot,
    /// The extra names in use.
    pub hash_high: u32,
    pub no_new_control_sequence: bool,
    pub prim_used: PrimNum,
    pub cs_count: i32,
    /// The meaning of each control sequence, and the level it was defined at.
    pub meanings: crate::tables::eqtb::Table<CsId, crate::base::command::Command>,
    pub ints: crate::tables::eqtb::Table<crate::tables::eqtb::IntLoc, i32>,
    pub dimens: crate::tables::eqtb::Table<crate::tables::eqtb::DimenLoc, Scaled>,
    pub glue: crate::tables::eqtb::Table<crate::tables::eqtb::GlueLoc, GlueSpecId>,
    pub toks:
        crate::tables::eqtb::Table<crate::tables::eqtb::ToksLoc, crate::base::token::TokenList>,
    pub shapes:
        crate::tables::eqtb::Table<crate::tables::eqtb::ShapeLoc, crate::tables::eqtb::ShapeRef>,
    pub fonts: crate::tables::eqtb::Table<crate::tables::eqtb::FontLoc, FontId>,
    pub cat_codes: crate::tables::eqtb::Table<crate::tables::eqtb::CatLoc, Cmd>,
    pub lc_codes: crate::tables::eqtb::Table<crate::tables::eqtb::LcLoc, i32>,
    pub uc_codes: crate::tables::eqtb::Table<crate::tables::eqtb::UcLoc, i32>,
    pub sf_codes: crate::tables::eqtb::Table<crate::tables::eqtb::SfLoc, i32>,
    pub math_codes: crate::tables::eqtb::Table<crate::tables::eqtb::MathLoc, i32>,
    pub del_codes: crate::tables::eqtb::Table<crate::tables::eqtb::DelLoc, i32>,
    /// `\XeTeXinterchartoks`, by class pair.
    pub inter_char: crate::tables::eqtb::SparseTable<
        crate::tables::eqtb::InterCharPair,
        crate::base::token::TokenList,
    >,
    pub boxes: crate::tables::eqtb::Table<
        crate::tables::eqtb::BoxLoc,
        Option<crate::memory::node::NodePtr<crate::memory::node::BoxNode>>,
    >,
    pub prim: Vec<crate::tables::hash::PrimEntry>,
    /// The `next` and `text` of each control sequence from the first hash bucket on.
    pub hash: Vec<crate::tables::hash::HashEntry>,
    pub hash_extra: u32,
    /// The last control sequence, past the extra hash entries.
    pub cs_top: CsId,
    pub journal: crate::preview::journal::EqtbJournal,
    pub pgfmath: crate::scanning::pgfmath::KnownCs,
}

impl Default for EqtbState {
    fn default() -> Self {
        EqtbState {
            hash_used: crate::tables::cs::NameSlot::BUCKETS_END,
            hash_high: 0,
            no_new_control_sequence: false,
            prim_used: PrimNum::LAST,
            cs_count: 0,
            meanings: Default::default(),
            inter_char: Default::default(),
            ints: Default::default(),
            dimens: Default::default(),
            glue: Default::default(),
            toks: Default::default(),
            shapes: Default::default(),
            boxes: Default::default(),
            fonts: Default::default(),
            cat_codes: Default::default(),
            lc_codes: Default::default(),
            uc_codes: Default::default(),
            sf_codes: Default::default(),
            math_codes: Default::default(),
            del_codes: Default::default(),
            // PRIM_SIZE + 1 = 2101.
            prim: vec![crate::tables::hash::PrimEntry::default(); 2101],
            hash: Vec::new(),
            hash_extra: 0,
            cs_top: CsId::UNDEFINED,
            journal: crate::preview::journal::EqtbJournal::default(),
            pgfmath: crate::scanning::pgfmath::KnownCs::default(),
        }
    }
}

impl EqtbState {
    /// Sizes the tables for `hash_extra` extra names, every control sequence undefined.
    pub(crate) fn alloc_tables(&mut self) {
        self.cs_top = CsId::top(self.hash_extra);
        let cs = self.cs_top.index() + 1;
        self.meanings = crate::tables::eqtb::Table::at_level(
            cs,
            crate::base::command::Command::UndefinedCs,
            crate::base::consts::LEVEL_ZERO,
        );
        self.hash =
            crate::memory::mem_types::zeroed_vec(self.cs_top.hash_index().map_or(0, |i| i + 1));
        self.ints = crate::tables::eqtb::Table::new(crate::tables::eqtb::IntLoc::LEN, 0);
        self.inter_char.clear();
        self.dimens =
            crate::tables::eqtb::Table::new(crate::tables::eqtb::DimenLoc::LEN, Scaled::ZERO);
        self.glue = crate::tables::eqtb::Table::new(
            crate::tables::eqtb::GlueLoc::LEN,
            crate::memory::mem::ZERO_GLUE,
        );
        self.toks = crate::tables::eqtb::Table::new(
            crate::tables::eqtb::ToksLoc::LEN,
            crate::base::token::TokenList::NULL,
        );
        self.shapes = crate::tables::eqtb::Table::new(
            crate::tables::eqtb::ShapeLoc::LEN,
            crate::tables::eqtb::ShapeRef::NONE,
        );
        self.boxes = crate::tables::eqtb::Table::new(crate::tables::eqtb::BoxLoc::LEN, None);
        self.fonts = crate::tables::eqtb::Table::new(
            crate::tables::eqtb::FontLoc::LEN,
            crate::base::consts::NULL_FONT,
        );
        let usvs = crate::base::consts::NUMBER_USVS as usize;
        self.cat_codes = crate::tables::eqtb::Table::new(usvs, Cmd::ESCAPE);
        self.lc_codes = crate::tables::eqtb::Table::new(usvs, 0);
        self.uc_codes = crate::tables::eqtb::Table::new(usvs, 0);
        self.sf_codes = crate::tables::eqtb::Table::new(usvs, 0);
        self.math_codes = crate::tables::eqtb::Table::new(usvs, 0);
        self.del_codes = crate::tables::eqtb::Table::new(usvs, -1);
    }
}

impl EqtbState {
    /// The meaning of control sequence `cs`.
    #[inline(always)]
    pub(crate) fn meaning(&self, cs: CsId) -> crate::base::command::Command {
        self.meanings.get(cs)
    }

    /// The level `cs`'s meaning was defined at.
    #[inline]
    pub(crate) fn cs_level(&self, cs: CsId) -> u16 {
        self.meanings.level(cs)
    }

    /// Gives `cs` the meaning `m` at level `level`.
    #[inline]
    pub(crate) fn set_meaning(&mut self, cs: CsId, m: crate::base::command::Command, level: u16) {
        self.meanings.set(cs, m, level);
    }

    #[inline]
    pub(crate) fn hash_next(&self, p: CsId) -> Option<CsId> {
        p.hash_index()
            .and_then(|i| self.hash.get(i))
            .and_then(|h| h.next)
    }

    #[inline]
    pub(crate) fn set_hash_next(&mut self, p: CsId, next: CsId) {
        let Some(index) = p.hash_index() else {
            return;
        };
        if self.journal.hash.armed() {
            self.journal.hash.note(index, self.hash[index]);
        }
        self.hash[index].next = Some(next);
    }

    /// The name of control sequence `p`, `text(p)`, none below the first hash bucket.
    #[inline]
    pub fn text(&self, p: CsId) -> StrNumber {
        p.hash_index()
            .and_then(|i| self.hash.get(i))
            .map_or(StrNumber::NONE, |h| h.text)
    }

    #[inline]
    pub fn set_text(&mut self, p: CsId, s: StrNumber) {
        let Some(index) = p.hash_index() else {
            return;
        };
        if self.journal.hash.armed() {
            self.journal.hash.note(index, self.hash[index]);
        }
        self.hash[index].text = s;
    }

    /// The next slot in `p`'s hash chain.
    #[inline]
    pub fn prim_next(&self, p: PrimNum) -> Option<PrimNum> {
        self.prim[p.index()].next
    }

    #[inline]
    pub fn set_prim_next(&mut self, p: PrimNum, next: Option<PrimNum>) {
        let index = p.index();
        if self.journal.prim.armed() {
            self.journal.prim.note(index, self.prim[index]);
        }
        self.prim[index].next = next;
    }

    /// The name of the primitive in slot `p`, if the slot is used.
    #[inline]
    pub fn prim_text(&self, p: PrimNum) -> Option<StrNumber> {
        self.prim[p.index()].text
    }

    #[inline]
    pub fn set_prim_text(&mut self, p: PrimNum, s: Option<StrNumber>) {
        let index = p.index();
        if self.journal.prim.armed() {
            self.journal.prim.note(index, self.prim[index]);
        }
        self.prim[index].text = s;
    }

    /// The control sequence text that identifies font `f`.
    #[inline]
    pub(crate) fn font_id_text(&self, f: FontId) -> StrNumber {
        self.text(CsId::font_id(f))
    }

    #[inline]
    pub(crate) fn cur_font(&self) -> FontId {
        self.fonts.get(crate::tables::eqtb::FontLoc::CUR)
    }

    #[inline]
    pub(crate) fn tok_par(&self, n: TokPar) -> crate::base::token::TokenList {
        self.toks.get(n.loc())
    }
    #[inline]
    pub(crate) fn par_shape(
        &self,
    ) -> Option<crate::memory::node::NodePtr<crate::memory::node::ParShapeNode>> {
        crate::memory::node::NodePtr::from_ptr(
            self.shapes
                .get(crate::tables::eqtb::ShapeLoc::ParShape)
                .ptr(),
        )
    }
    /// The e-TeX penalty array held at `loc`.
    #[inline]
    pub(crate) fn penalties(
        &self,
        loc: crate::tables::eqtb::ShapeLoc,
    ) -> Option<crate::memory::node::NodePtr<crate::memory::node::PenaltiesNode>> {
        crate::memory::node::NodePtr::from_ptr(self.shapes.get(loc).ptr())
    }
    #[inline]
    pub(crate) fn box_reg(
        &self,
        n: impl Into<crate::tables::eqtb::Register>,
    ) -> Option<crate::memory::node::NodePtr<crate::memory::node::BoxNode>> {
        self.boxes
            .get(crate::tables::eqtb::BoxLoc::register(n.into()))
    }
    #[inline]
    /// Sets box register `n` without saving, keeping its level.
    pub(crate) fn set_box_reg(
        &mut self,
        n: impl Into<crate::tables::eqtb::Register>,
        b: Option<crate::memory::node::NodePtr<crate::memory::node::BoxNode>>,
    ) {
        self.boxes
            .set_value(crate::tables::eqtb::BoxLoc::register(n.into()), b);
    }
    #[inline]
    pub(crate) fn skip(
        &self,
        n: impl Into<crate::tables::eqtb::Register>,
    ) -> crate::memory::glue_store::GlueSpecId {
        self.glue.get(crate::tables::eqtb::GlueLoc::skip(n.into()))
    }
    #[inline]
    pub(crate) fn dimen(&self, n: impl Into<crate::tables::eqtb::Register>) -> Scaled {
        self.dimens
            .get(crate::tables::eqtb::DimenLoc::dimen(n.into()))
    }
    #[inline]
    pub(crate) fn toks(
        &self,
        n: impl Into<crate::tables::eqtb::Register>,
    ) -> crate::base::token::TokenList {
        self.toks
            .get(crate::tables::eqtb::ToksLoc::register(n.into()))
    }
    #[inline]
    pub(crate) fn count(&self, n: impl Into<crate::tables::eqtb::Register>) -> i32 {
        self.ints.get(crate::tables::eqtb::IntLoc::count(n.into()))
    }
    #[inline]
    pub(crate) fn math_font(&self, n: i32) -> FontId {
        self.fonts.get(crate::tables::eqtb::FontLoc::math(n))
    }
    #[inline(always)]
    pub(crate) fn cat_code(&self, c: i32) -> Cmd {
        self.cat_codes.get(crate::tables::eqtb::CatLoc::of(c))
    }
    #[inline]
    pub(crate) fn lc_code(&self, c: i32) -> i32 {
        self.lc_codes.get(crate::tables::eqtb::LcLoc::of(c))
    }
    #[inline]
    pub(crate) fn uc_code(&self, c: i32) -> i32 {
        self.uc_codes.get(crate::tables::eqtb::UcLoc::of(c))
    }
    #[inline]
    pub(crate) fn sf_code(&self, c: i32) -> i32 {
        self.sf_codes.get(crate::tables::eqtb::SfLoc::of(c))
    }
    #[inline]
    pub(crate) fn math_code(&self, c: i32) -> i32 {
        self.math_codes.get(crate::tables::eqtb::MathLoc::of(c))
    }
    #[inline]
    pub(crate) fn del_code(&self, c: i32) -> i32 {
        self.del_codes.get(crate::tables::eqtb::DelLoc::of(c))
    }

    #[inline]
    pub fn int_par(&self, n: IntPar) -> i32 {
        self.ints.get(n.loc())
    }
    /// Sets `n`, keeping its level.
    #[inline]
    pub fn set_int_par(&mut self, n: IntPar, v: i32) {
        self.ints.set_value(n.loc(), v);
    }
    #[inline]
    pub fn dimen_par(&self, n: DimenPar) -> Scaled {
        self.dimens.get(n.loc())
    }
    /// Sets `n`, keeping its level.
    #[inline]
    pub fn set_dimen_par(&mut self, n: DimenPar, v: Scaled) {
        self.dimens.set_value(n.loc(), v);
    }
    /// The spec of glue parameter `n`, never null.
    #[inline]
    pub fn glue_par(&self, n: GluePar) -> crate::memory::glue_store::GlueSpecId {
        self.glue.get(n.loc())
    }

    #[inline]
    pub fn set_glue_par(&mut self, n: GluePar, spec: crate::memory::glue_store::GlueSpecId) {
        self.glue.set_value(n.loc(), spec);
    }
}

/// The `mem` arena and its bookkeeping. `data` never reallocates during a run,
/// so the pointer-returning accessors stay valid.
#[derive(Default)]
pub(crate) struct MemState {
    /// The node arena, in 32-bit words.
    pub data: Vec<u32>,
    /// Owned shaped-glyph payloads referenced by handles in native nodes.
    pub glyph_blobs: crate::memory::glyph_arena::GlyphBlobArena,
    /// The token lists, a store of their own.
    pub tokens: crate::base::token::TokenStore,
    /// The glue specifications.
    pub glue: crate::memory::glue_store::GlueStore,
    pub journal: crate::preview::journal::WordJournal<u32>,
    /// The free list of each node size.
    pub free: crate::memory::mem::FreeLists,
    /// TeX's `line`, kept here because `get_node` stamps it into new nodes.
    pub line: i32,
    pub cur_synctex_tag: i32,
    pub temp_ptr: crate::base::units::MemPtr,
}

impl MemState {
    /// The SyncTeX tag and line a node made now is stamped with.
    #[inline]
    pub(crate) fn sync_stamp(&self) -> crate::memory::node::SyncStamp {
        crate::memory::node::SyncStamp {
            tag: self.cur_synctex_tag,
            line: self.line,
        }
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

crate::base::codes::code_enum! {
    /// The kind of group on the save stack (`cur_group`).
    #[derive(Default)]
    pub(crate) enum GroupCode: u8 {
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
}

/// The command that closes a group `}` cannot close, as TeX names it.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum Closer {
    EndGroup,
    MathShift,
    Right,
}

impl crate::print::TexPrint for Closer {
    fn print_to(&self, st: &mut EngineState) {
        use crate::print::{Esc, print};
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
    pub(crate) fn tokens(self) -> Vec<crate::base::token::Token> {
        use crate::base::token::Token;
        match self.0 {
            Some(Closer::EndGroup) => vec![Token::from_cs(Frozen::EndGroup.cs())],
            Some(Closer::MathShift) => vec![Token::math_shift(b'$')],
            Some(Closer::Right) => vec![Token::from_cs(Frozen::Right.cs()), Token::other(b'.')],
            None => vec![Token::right_brace(b'}')],
        }
    }
}

impl crate::print::TexPrint for Insertion {
    fn print_to(&self, st: &mut EngineState) {
        use crate::print::{Esc, print};
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
}

/// One entry of the save stack.
#[derive(Copy, Clone)]
pub(crate) enum SaveEntry {
    /// Where a group began: the enclosing group's code and boundary, and the line this one began on.
    Boundary {
        group: GroupCode,
        below: Option<SavePtr>,
        line: i32,
    },
    /// A value to put back when the group ends.
    Restore(Restore),
    /// A token `\aftergroup` inserts when the group ends.
    InsertToken(crate::base::token::Token),
    /// Something a construction keeps until its group ends.
    Saved(Saved),
}

/// An old `eqtb` value, with the level it had.
#[derive(Copy, Clone)]
pub(crate) enum Restore {
    Cs(crate::tables::eqtb::Old<CsId>),
    Int(crate::tables::eqtb::Old<crate::tables::eqtb::IntLoc>),
    Dimen(crate::tables::eqtb::Old<crate::tables::eqtb::DimenLoc>),
    Glue(crate::tables::eqtb::Old<crate::tables::eqtb::GlueLoc>),
    Toks(crate::tables::eqtb::Old<crate::tables::eqtb::ToksLoc>),
    Shape(crate::tables::eqtb::Old<crate::tables::eqtb::ShapeLoc>),
    Box(crate::tables::eqtb::Old<crate::tables::eqtb::BoxLoc>),
    Font(crate::tables::eqtb::Old<crate::tables::eqtb::FontLoc>),
    Cat(crate::tables::eqtb::Old<crate::tables::eqtb::CatLoc>),
    Lc(crate::tables::eqtb::Old<crate::tables::eqtb::LcLoc>),
    Uc(crate::tables::eqtb::Old<crate::tables::eqtb::UcLoc>),
    Sf(crate::tables::eqtb::Old<crate::tables::eqtb::SfLoc>),
    Math(crate::tables::eqtb::Old<crate::tables::eqtb::MathLoc>),
    Del(crate::tables::eqtb::Old<crate::tables::eqtb::DelLoc>),
    InterChar(crate::tables::eqtb::Old<crate::tables::eqtb::InterCharPair>),
}

/// What a construction keeps on the save stack below its group.
#[derive(Copy, Clone, Debug, PartialEq, Hash)]
pub(crate) enum Saved {
    /// Where a box goes when its group ends, `box_context`.
    BoxContext(crate::build::main_control::BoxContext),
    Spec(crate::build::packaging::PackSpec),
    /// An `\insert` with its box number, or a `\vadjust` (number 255).
    Insert {
        n: i32,
        pre: bool,
    },
    /// The noad field a math group fills.
    MathGroup(crate::memory::node::MathSlot),
    /// An `\eqno` (false) or `\leqno` (true).
    EqNo {
        left: bool,
    },
    /// How many `\mathchoice` parts are done.
    ChoicePart(u8),
    /// How many `\discretionary` parts are done.
    DiscPart(u8),
}

#[derive(Default)]
pub(crate) struct SaveStackState {
    pub save_ptr: SavePtr,
    pub max_save_stack: SavePtr,
    pub save_size: usize,
    pub cur_level: u16,
    pub cur_group: GroupCode,
    /// The current group's boundary entry, none at the bottom level.
    pub cur_boundary: Option<SavePtr>,
    pub data: Vec<SaveEntry>,
}

impl SaveStackState {
    /// The save stack ran out.
    pub(crate) fn overflow(&self) -> crate::print::errors::Abort {
        crate::print::errors::Abort::overflow(
            crate::print::errors::Capacity::SaveSize,
            self.save_size,
        )
    }

    /// The stack of `size` free entries.
    pub(crate) fn alloc(&mut self, size: usize) {
        self.data = vec![SaveEntry::Saved(Saved::ChoicePart(0)); size];
    }

    #[inline]
    pub(crate) fn entry(&self, k: SavePtr) -> SaveEntry {
        self.data[k.index()]
    }

    /// Writes `e` at `save_ptr` and moves past it.
    #[inline]
    pub(crate) fn push(&mut self, e: SaveEntry) {
        self.data[self.save_ptr.index()] = e;
        self.save_ptr += 1;
    }

    /// Keeps `s` until the next group ends.
    #[inline]
    pub(crate) fn push_saved(&mut self, s: Saved) {
        self.push(SaveEntry::Saved(s));
    }

    /// Pops what [`push_saved`](Self::push_saved) kept, `None` if the top holds something else.
    pub(crate) fn pop_saved(&mut self) -> Option<Saved> {
        self.save_ptr -= 1;
        self.saved(0)
    }

    /// What a construction kept `k` entries above `save_ptr`.
    #[inline]
    pub(crate) fn saved(&self, k: i32) -> Option<Saved> {
        match self.data.get((self.save_ptr + k).index())? {
            SaveEntry::Saved(s) => Some(*s),
            _ => None,
        }
    }

    /// The entry just below `save_ptr`, to update a kept count in place.
    #[inline]
    pub(crate) fn top_saved_mut(&mut self) -> Option<&mut Saved> {
        match self.data.get_mut((self.save_ptr - 1).index())? {
            SaveEntry::Saved(s) => Some(s),
            _ => None,
        }
    }

    /// Pops the size spec a box group kept.
    pub(crate) fn pop_spec(
        &mut self,
    ) -> Result<crate::build::packaging::PackSpec, crate::print::errors::Abort> {
        match self.pop_saved() {
            Some(Saved::Spec(spec)) => Ok(spec),
            _ => Err(crate::print::errors::Abort::Confusion("pop_spec")),
        }
    }

    /// The enclosing group, its boundary, and the starting line kept by the boundary at `k`.
    #[inline]
    pub(crate) fn boundary_at(&self, k: SavePtr) -> Option<(GroupCode, Option<SavePtr>, i32)> {
        match self.entry(k) {
            SaveEntry::Boundary { group, below, line } => Some((group, below, line)),
            _ => None,
        }
    }

    /// What [`boundary_at`](Self::boundary_at) gives for the current group.
    #[inline]
    pub(crate) fn boundary(&self) -> Option<(GroupCode, Option<SavePtr>, i32)> {
        self.boundary_at(self.cur_boundary?)
    }

    /// What the current group's construction kept `k` entries below its boundary.
    #[inline]
    pub(crate) fn kept(&self, k: i32) -> Option<Saved> {
        match self.data.get((self.cur_boundary? - k).index())? {
            SaveEntry::Saved(s) => Some(*s),
            _ => None,
        }
    }
}

/// `cur_mark`, the marks of the current page and split.
#[derive(Default, Clone, Copy)]
pub(crate) struct CurMarks([crate::base::token::TokenList; 5]);

impl CurMarks {
    pub(crate) const NULL: CurMarks = CurMarks([crate::base::token::TokenList::NULL; 5]);
}

impl std::ops::Index<crate::base::codes::MarkCode> for CurMarks {
    type Output = crate::base::token::TokenList;
    #[inline]
    fn index(&self, m: crate::base::codes::MarkCode) -> &crate::base::token::TokenList {
        &self.0[m as usize]
    }
}

impl std::ops::IndexMut<crate::base::codes::MarkCode> for CurMarks {
    #[inline]
    fn index_mut(&mut self, m: crate::base::codes::MarkCode) -> &mut crate::base::token::TokenList {
        &mut self.0[m as usize]
    }
}

/// The marks of the `\marks` classes above 0, class 0 being `cur_mark`.
#[derive(Default, Clone)]
pub(crate) struct MarkClasses(std::collections::BTreeMap<crate::tables::eqtb::Register, CurMarks>);

impl MarkClasses {
    /// The marks of `class`, all null when it has none.
    pub(crate) fn get(&self, class: crate::tables::eqtb::Register) -> CurMarks {
        self.0.get(&class).copied().unwrap_or(CurMarks::NULL)
    }

    /// The marks of `class`, created null.
    pub(crate) fn entry(&mut self, class: crate::tables::eqtb::Register) -> &mut CurMarks {
        self.0.entry(class).or_insert(CurMarks::NULL)
    }

    /// Keeps the classes `keep` returns true for, in ascending order.
    pub(crate) fn retain(&mut self, mut keep: impl FnMut(&mut CurMarks) -> bool) {
        self.0.retain(|_, marks| keep(marks));
    }

    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (crate::tables::eqtb::Register, &CurMarks)> {
        self.0.iter().map(|(&class, marks)| (class, marks))
    }
}

#[derive(Default, Clone)]
pub(crate) struct ScannerState {
    /// Monotonic token-fetch clock used to order checkpoint fences.
    pub logical_clock: u64,
    pub name_in_progress: bool,
    pub cur_align: crate::base::units::MemPtr,
    pub cur_span: crate::base::units::MemPtr,
    pub cur_loop: crate::base::units::MemPtr,
    /// The enclosing alignments, TeX's `align_ptr` list.
    pub align_stack: Vec<crate::build::main_control::SavedAlign>,
    pub cur_head: crate::base::units::MemPtr,
    pub cur_tail: crate::base::units::MemPtr,
    pub cur_pre_head: crate::base::units::MemPtr,
    pub cur_pre_tail: crate::base::units::MemPtr,
    pub shown_mode: crate::base::mode::ModeName,
    pub mag_set: i32,
    pub expand_depth_count: i32,
    pub max_reg_num: i32,
    /// The innermost incomplete conditional, `None` outside any.
    /// The conditional stack, TeX's `cond_ptr` list.
    pub conds: crate::scanning::cond_stack::CondStack,
    pub if_limit: crate::memory::node::Stored<FiOrElseCode>,
    pub cur_if: crate::base::codes::IfTest,
    pub if_line: i32,
    pub skip_line: i32,
    pub is_in_csname: bool,
    pub used_tectonic_coda_tokens: bool,
    pub scanner_status: crate::base::codes::ScannerStatus,
    pub warning_index: Option<CsId>,
    pub def_ref: crate::base::token::TokenList,
    pub long_state: Cmd,
    pub align_state: i32,
    pub expand_depth: i32,
    /// The meaning of the token `get_next` read last.
    pub cur: crate::base::command::Command,
    pub cur_cs: Option<CsId>,
    pub cur_tok: crate::base::token::Token,
    pub cur_mark: CurMarks,
    pub mark_classes: MarkClasses,
    pub disc_ptr: [crate::base::units::MemPtr; 4],
    pub after_token: crate::base::token::Token,
    pub cur_f: FontId,
    pub set_box_allowed: bool,
    pub native_text: Vec<u16>,
    pub native_text_size: i32,
    pub native_len: i32,
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
    pub pgfmath: crate::scanning::pgfmath::Mode,
}

pub(crate) use crate::ffi::teckit_icu::{FontMappingHandle, LinebreakHandle};

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
    /// The limit on `fmem_ptr`.
    pub font_mem_size: i32,
    pub font_max: FontId,
    /// The `font_info` words TeX would use for the loaded fonts, counted for
    /// `font_mem_size` and the statistics.
    pub fmem_ptr: i32,
    pub font_ptr: FontId,
    pub font_flags: PerFont<i8>,
    pub font_letter_space: PerFont<Scaled>,
    pub font_check: PerFont<[u8; 4]>,
    pub font_size: PerFont<Scaled>,
    pub font_dsize: PerFont<Scaled>,
    pub font_name: PerFont<StrNumber>,
    pub font_area: PerFont<StrNumber>,
    pub font_bc: PerFont<u16>,
    pub font_ec: PerFont<u16>,
    pub font_bchar: PerFont<crate::fonts::LigChar>,
    pub font_false_bchar: PerFont<crate::fonts::LigChar>,
    /// The interword glue built from the font's parameters, once a space needs it.
    pub font_glue: PerFont<Option<crate::memory::glue_store::GlueSpecId>>,
    pub font_used: PerFont<bool>,
    pub hyphen_char: PerFont<i32>,
    pub skew_char: PerFont<i32>,
    pub metrics: PerFont<crate::fonts::tfm::FontMetrics>,
    pub params: PerFont<crate::fonts::tfm::FontParams>,
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
            font_mem_size: 0,
            font_max: crate::base::consts::NULL_FONT,
            fmem_ptr: 0,
            font_ptr: crate::base::consts::NULL_FONT,
            font_flags: PerFont::default(),
            font_letter_space: PerFont::default(),
            font_check: PerFont::default(),
            font_size: PerFont::default(),
            font_dsize: PerFont::default(),
            font_name: PerFont::default(),
            font_area: PerFont::default(),
            font_bc: PerFont::default(),
            font_ec: PerFont::default(),
            font_bchar: PerFont::default(),
            font_false_bchar: PerFont::default(),
            font_glue: PerFont::default(),
            font_used: PerFont::default(),
            hyphen_char: PerFont::default(),
            skew_char: PerFont::default(),
            metrics: PerFont::default(),
            params: PerFont::default(),
            mapped_text: Vec::new(),
            cp_codes: crate::fonts::ProtrusionCodes::default(),
            glyph_bboxes: std::cell::RefCell::default(),
        }
    }
}

impl FontState {
    /// Font parameter `n` of font `f`, zero when the font has no such parameter.
    #[inline]
    pub(crate) fn param(&self, f: FontId, n: i32) -> Scaled {
        self.params[f].get(n).unwrap_or(Scaled::ZERO)
    }

    /// TeX's `font_params`.
    #[inline]
    pub(crate) fn param_count(&self, f: FontId) -> i32 {
        self.params[f].count()
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
    pub dvi: crate::shipout::dvi::DviWriter,
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
    pub page_tail: crate::base::units::MemPtr,
    pub last_badness: i32,
    pub page_contents: PageContents,
    pub page_so_far: [Scaled; 8],
    pub last_glue: Option<crate::memory::glue_store::GlueSpecId>,
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
    pub best_page_break: crate::base::units::MemPtr,
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
    pub write_loc: Option<CsId>,
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

    /// TeX's `selector - 1`: `TermAndLog` to `LogOnly`, `TermOnly` to `NoPrint`.
    pub(crate) const fn without_terminal(self) -> Selector {
        match self {
            Selector::TermAndLog => Selector::LogOnly,
            Selector::TermOnly => Selector::NoPrint,
            other => other,
        }
    }

    /// TeX's `selector + 1`, undoing [`Selector::without_terminal`].
    pub(crate) const fn with_terminal(self) -> Selector {
        match self {
            Selector::LogOnly => Selector::TermAndLog,
            Selector::NoPrint => Selector::TermOnly,
            other => other,
        }
    }

    /// TeX's `selector + 2`: `NoPrint` to `LogOnly`, `TermOnly` to `TermAndLog`.
    pub(crate) const fn with_log(self) -> Selector {
        match self {
            Selector::NoPrint => Selector::LogOnly,
            Selector::TermOnly => Selector::TermAndLog,
            other => other,
        }
    }

    /// TeX's `selector - 2`, undoing [`Selector::with_log`].
    pub(crate) const fn without_log(self) -> Selector {
        match self {
            Selector::LogOnly => Selector::NoPrint,
            Selector::TermAndLog => Selector::TermOnly,
            other => other,
        }
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
            font_in_short_display: crate::base::consts::NULL_FONT,
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
    pub(crate) fn pool_overflow(&self) -> crate::print::errors::Abort {
        crate::print::errors::Abort::Overflow {
            capacity: crate::print::errors::Capacity::PoolSize,
            n: self.pool_size - self.init_pool_ptr,
        }
    }

    /// The string table ran out.
    pub(crate) fn string_overflow(&self) -> crate::print::errors::Abort {
        crate::print::errors::Abort::Overflow {
            capacity: crate::print::errors::Capacity::NumberOfStrings,
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
    pub hf: FontId,
    pub max_hyph_char: i32,
    pub init_list: crate::base::units::MemPtr,
    pub init_lig: bool,
    pub init_lft: bool,
    pub hyphen_passed: usize,
    pub trie_max: crate::build::hyphenation::TriePtr,
    pub trie_size: i32,
    pub trie_ptr: i32,
    pub hyph_count: i32,
    pub hyph_size: i32,
    pub hyph_next: i32,
    pub trie_op_ptr: i32,
    pub max_op_used: u16,
    pub trie_not_ready: bool,
    pub hyph_start: crate::build::hyphenation::TriePtr,
    /// The `\lccode` table saved with the current language's patterns.
    pub hyph_index: Option<crate::build::hyphenation::TriePtr>,
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
    pub hyph_list: Vec<crate::base::units::MemPtr>,
    pub hyph_link: Vec<u16>,
    pub trie_op_hash_array: Vec<i32>,
}

impl Default for HyphState {
    fn default() -> Self {
        Self {
            tables_gen: 0,
            hf: crate::base::consts::NULL_FONT,
            max_hyph_char: 0,
            init_list: None,
            init_lig: false,
            init_lft: false,
            hyphen_passed: 0,
            trie_max: crate::build::hyphenation::TriePtr::NULL,
            trie_size: 0,
            trie_ptr: 0,
            hyph_count: 0,
            hyph_size: 0,
            hyph_next: 0,
            trie_op_ptr: 0,
            max_op_used: 0,
            trie_not_ready: false,
            hyph_start: crate::build::hyphenation::TriePtr::NULL,
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
    /// Grows `native_text` until `extra` more units fit.
    #[inline(always)]
    pub(crate) fn reserve_native_text(
        &mut self,
        extra: i32,
    ) -> Result<(), crate::print::errors::Abort> {
        if extra >= NATIVE_TEXT_MAX - self.native_len {
            return Err(crate::print::errors::Abort::Overflow {
                capacity: crate::print::errors::Capacity::NativeTextSize,
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
