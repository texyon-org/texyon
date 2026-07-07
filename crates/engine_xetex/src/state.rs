//! Rust-owned engine state.


#[derive(Default)]
pub(crate) struct EngineState {
    pub config: ConfigState,
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
}

/// Scratch registers of `main_control`'s character/ligature main loop
/// (xetex-xetexd.h:672–686). Pure per-iteration scratch (set-before-read
/// within the loop).
#[derive(Default)]
pub(crate) struct MainLoopState {
    /// Scratch glue-spec pointer (xetex-xetexd.h:675).
    pub main_p: crate::units::MemPtr,
    /// Scratch font-info index (xetex-xetexd.h:676).
    pub main_k: i32,
    /// Current font of the main loop — `internal_font_number` (xetex-xetexd.h:672).
    pub main_f: i32,
    pub main_h: i32,
    pub main_pp: crate::units::MemPtr,
    pub main_ppp: crate::units::MemPtr,
    pub main_s: i32,
    /// Boundary char of the current font (xetex-xetexd.h:684).
    pub bchar: i32,
    /// Non-existent boundary-char sentinel (xetex-xetexd.h:685).
    pub false_bchar: i32,
    /// Inter-character space class of the current / previous char.
    pub space_class: i32,
    pub prev_class: i32,
    /// Ligature/kern reconstitution scratch (xetex-xetexd.h:630–634, 683–686);
    /// shared between `main_control`'s main loop and `linebreak::reconstitute`.
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


/// SyncTeX boolean flag block (formerly the `synctex_flags_t` bitfield struct).
#[derive(Default)]
pub(crate) struct SynctexFlags {
    pub content_ready: bool,
    pub off: bool,
    pub not_void: bool,
    pub warn: bool,
    #[allow(dead_code)]
    pub output_p: bool,
}

/// The SyncTeX context. `root_name` is an owned C string managed via
/// `synctexabort`/the synctex open path (its lifecycle stays in synctex.rs, no
/// `tt_cleanup` edge).
pub(crate) struct SynctexCtxt {
    pub file: usize,
    pub root_name: *mut libc::c_char,
    pub count: i32,
    pub node: crate::units::MemPtr,
    /// A kern node whose SyncTeX record is deferred until the context changes
    /// (the C `synctex_recorder` fn pointer — kern is the only node kind this
    /// engine ever defers, so the typed handle replaces the callback; when
    /// `Some`, it always equals `node`). Flush sites must record it *before*
    /// reassigning `node`.
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
}

impl Default for SynctexCtxt {
    fn default() -> Self {
        // Every field zero/null/None/false except `lastv`, which starts at -1.
        Self {
            file: 0, // INVALID_HANDLE
            root_name: std::ptr::null_mut(),
            count: 0,
            node: crate::units::MemPtr::from_raw(0),
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
        }
    }
}

/// The `texmf` cluster: small io/job-context state formerly held as
/// `xetex-texmfmp.c` file-statics (`last_source_name`, `last_lineno`,
/// `start_time_str`).
///
/// `last_source_name` is an **owned C string** (`gettexstring`/`libc::free`):
/// its free/reassign lifecycle stays entirely in `remember_source_info`, with
/// no `tt_cleanup` edge (the last allocation leaks at teardown exactly as
/// before). Eventual newtype target: `Option<CString>`.
#[derive(Default)]
pub(crate) struct TexmfState {
    pub last_source_name: *mut libc::c_char,
    pub last_lineno: i32,
    /// TIME_STR_SIZE = 30 ("D:YYYYmmddHHMMSS+HH'MM'" plus NUL and headroom).
    pub start_time_str: [u8; 30],
}

pub(crate) struct AbsPathBuf(pub [libc::c_char; 1024]);
impl Default for AbsPathBuf {
    fn default() -> Self {
        AbsPathBuf([0; 1024])
    }
}

/// The `input` cluster's Rust-owned state. Distinct from
/// `crate::globals::InputState`, which is a single per-input-record struct;
/// this holds the cluster's owned arenas (input/param/file stacks).
#[derive(Default)]
pub(crate) struct InputClusterState {
    /// Rust-owned macro-parameter stack (`param_size + 1` words, matching the
    /// former `xmalloc_array` shape). Allocated at engine init and released by
    /// `tt_cleanup`; never reallocated during a run. Replaces the former
    /// C-`malloc`'d `param_stack` pointer. Not serialized in format dump/undump.
    pub param_stack: Vec<i32>,
    /// Rust-owned input line/group stacks, each sized `max_in_open` (matching
    /// the former `xmalloc_array` shape). Allocated at engine init and released
    /// by `tt_cleanup`; never reallocated during a run. Not serialized in
    /// format dump/undump. `grp_stack` is a `save_pointer`, `*filename_stack`
    /// are `str_number` (newtype debts).
    pub line_stack: Vec<i32>,
    pub grp_stack: Vec<i32>,
    pub if_stack: Vec<i32>,
    pub source_filename_stack: Vec<i32>,
    pub full_source_filename_stack: Vec<i32>,
    /// `eof_seen[k]`: true when EOF was seen at input level k. Sized
    /// `max_in_open + 1`; allocated at init and released by `tt_cleanup`.
    /// Not serialized in format dump/undump. Plain `bool`, no newtype debt.
    pub eof_seen: Vec<bool>,
    /// Rust-owned input buffer (`buf_size + 1` words, matching the former
    /// `xmalloc_array` shape). Allocated at engine init and released by
    /// `tt_cleanup`; never reallocated during a run (overflow errors instead
    /// of growing). Replaces the former C-`malloc`'d `buffer` pointer. Not
    /// serialized in format dump/undump. Element type is `UnicodeScalar`
    /// (`i32`) — plain int, no newtype debt.
    pub buffer: Vec<i32>,
    /// Rust-owned input stack (`stack_size + 1` records, matching the former
    /// `xmalloc_array` shape). Allocated at engine init and released by
    /// `tt_cleanup`; never reallocated during a run. Replaces the former
    /// C-`malloc`'d `input_stack` pointer. Not serialized in format
    /// dump/undump. Element type is the `InputState` struct (mirrors C
    /// `input_state_t`) — a compound struct, not a scalar typedef, no
    /// newtype debt.
    pub input_stack: Vec<crate::globals::InputState>,
    /// The live top-of-input-stack record — TeX's `cur_input` (xetex-xetexd.h:506).
    /// `push_input` saves it into `input_stack[input_ptr]`,
    /// `pop_input` restores it. `synctex_tag` is mirrored into `st.mem.cur_synctex_tag`
    /// for `get_node`.
    pub cur_input: crate::globals::InputState,
    /// Rust-owned main input-file stack (`max_in_open + 1` handles, matching
    /// the former `xmalloc_array` shape). Allocated at engine init and released
    /// by `tt_cleanup`; never reallocated during a run (the pointer-escaping
    /// `input_file_ptr` accessor relies on this). Replaces the former
    /// C-`malloc`'d `input_file` pointer. Not serialized in format dump/undump.
    /// Element type is a `UFILE*` handle, stored as `*mut c_void` to avoid a
    /// cyclic dependency on `io.rs` — a plain pointer, no newtype debt.
    pub input_file: Vec<*mut crate::io::UFile>,
    /// Rust-owned semantic-nest (mode) stack (`nest_size + 1` records, matching
    /// the former `xmalloc_array` shape). Allocated at engine init and released
    /// by `tt_cleanup`; never reallocated during a run. Replaces the former
    /// C-`malloc`'d `nest` pointer. Not serialized in format dump/undump.
    /// Element type is the `ListStateRecord` struct (mirrors C
    /// `list_state_record`) — a compound struct, not a scalar typedef, no
    /// newtype debt.
    pub nest: Vec<crate::mem_types::ListStateRecord>,
    /// The live top-of-nest semantic list — TeX's `cur_list`; sibling to `nest`.
    pub cur_list: crate::mem_types::ListStateRecord,
    /// Count of open pseudo-file parentheses emitted to the transcript.
    /// Plain `int32_t`, no newtype debt.
    pub open_parens: i32,
    /// Current input-nesting depth (number of open input files). Plain `int32_t`.
    pub in_open: i32,
    /// Head of the pseudo-file linked list (xetex-xetexd.h:704). Plain `int32_t`.
    pub pseudo_files: crate::units::MemPtr,
    /// Current top of the semantic-nest stack. Plain `int32_t`.
    pub nest_ptr: i32,
    /// Peak value ever reached by `nest_ptr`. Plain `int32_t`.
    pub max_nest_stack: i32,
    /// Current top of the input-state stack. Plain `int32_t`.
    pub input_ptr: i32,
    /// Input-related capacity constants, set once at engine init.
    pub stack_size: i32,
    pub buf_size: i32,
    pub param_size: i32,
    pub max_in_open: i32,
    /// Capacity of the semantic-nest stack (`st.input.nest`); TeX's `nest_size`.
    pub nest_size: i32,
    /// Peak value ever reached by `input_ptr`. Plain `int32_t`.
    pub max_in_stack: i32,
    /// Search base in the input stack (xetex-xetexd.h:521). Plain `int32_t`.
    pub base_ptr: i32,
    /// Hash-table location of `\par` (xetex-xetexd.h:522). Plain `int32_t`.
    pub par_loc: i32,
    /// Token form of `\par` (CS_TOKEN_FLAG + par_loc, xetex-xetexd.h:523). Plain `int32_t`.
    pub par_token: i32,
    /// Next free slot in the macro-parameter stack. Plain `int32_t`.
    pub param_ptr: i32,
    /// Peak value ever reached by `param_ptr`. Plain `int32_t`.
    pub max_param_stack: i32,
    /// Set true by `\endinput` to force end-of-file on the current input level
    /// (xetex-xetexd.h:524). Plain `bool`.
    pub force_eof: bool,
    /// Start of the current input line in the buffer — TeX's `first`
    /// (xetex-xetexd.h:395). Sibling to `last`.
    pub first: i32,
    /// End of the current input line in the buffer (xetex-xetexd.h:395;
    /// next position after the last character). Plain `int32_t`.
    pub last: i32,
    /// High-water mark of buffer usage (xetex-xetexd.h:395). Plain `int32_t`.
    pub max_buf_stack: i32,
    /// Byte length of `name_of_file` (excluding NUL; xetex-xetexd.h:390). Plain `int32_t`.
    pub name_length: i32,
    /// Number of UTF-16 code units in `name_of_file16` (xetex-xetexd.h:391). Plain `int32_t`.
    pub name_length16: i32,
    /// File-open states for `\read` streams (xetex-xetexd.h:536; CLOSED=2, JUST_OPEN=1,
    /// NORMAL=0). BSS array of 17 bytes — default all-zero = NORMAL. Plain `u8`, no newtype debt.
    pub read_open: [u8; 17],
    /// `UFILE*` handles for `\read` streams (xetex-xetexd.h:535). BSS array of 16 pointers —
    /// default all-null. Stored as `*mut c_void` to avoid cyclic dependency with `io.rs` (same
    /// convention as `input_file`). The pointer-escaping accessor `read_file_ptr` is valid
    /// because the array lives in the `'static` thread-local and is never moved. Plain opaque
    /// pointer, no newtype debt.
    pub read_file: [*mut crate::io::UFile; 16],
    /// Rust-owned name buffer (NUL-terminated C string).
    pub name_of_file: Vec<libc::c_char>,
    /// Rust-owned UTF-16 name buffer.
    pub name_of_file16: Vec<u16>,
    /// Rust-owned name of current input file.
    pub name_of_input_file: *mut libc::c_char,
    /// Rust-owned absolute path of current input file.
    pub abspath_of_input_file: AbsPathBuf,
}

pub(crate) struct EqtbState {
    pub hash_used: i32,
    pub hash_high: i32,
    pub no_new_control_sequence: bool,
    pub prim_used: i32,
    pub cs_count: i32,
    /// equivalents table (former C-`malloc`'d `memory_word *eqtb`, size
    /// `eqtb_top + 1`). Rust-owned `Vec`, allocated in `ini.rs` at init and
    /// format-load and dropped by `tt_cleanup`; indexed by the `eqtb_*`
    /// accessors. In the format dump (sparse region `dump_slice`/`undump_slice`);
    /// those paths take a state-derived base pointer. Element
    /// `memory_word` (faithful `MemoryWord`), no newtype debt.
    pub data: Vec<crate::mem_types::MemoryWord>,
    /// Primitive-lookup table (former inline C `prim[PRIM_SIZE + 1]`). Heap-
    /// backed `Vec` sized `PRIM_SIZE + 1 = 2101`; not `malloc`'d in C (inline
    /// BSS), so it is sized here in `Default`, not allocated in `ini.rs`.
    /// Format dump/undump goes through the `prim_s0`/`prim_s1` accessors.
    /// Element `B32x2`, no newtype debt.
    pub prim: Vec<crate::mem_types::B32x2>,
    /// Per-int-parameter save-level array (former inline C
    /// `_xeq_level_array[EQTB_SIZE - INT_BASE + 1]`). Heap-backed `Vec` sized
    /// `1_114_730`; inline BSS in C, so sized here in `Default`. Not in the
    /// format dump. Element `u16` save level, no newtype debt.
    pub xeq_level: Vec<u16>,
    /// Hash table (former C-`malloc`'d `b32x2 *hash`, size `1 + hash_top - hash_offset`).
    /// Rust-owned `Vec`, allocated in `ini.rs` at init and format-load and dropped by `tt_cleanup`;
    /// indexed by `hash_s0`/`hash_s1` accessors with `- 514` (hash_offset) indexing offset.
    /// Element `B32x2`, no newtype debt.
    pub hash: Vec<crate::mem_types::B32x2>,
    pub hash_extra: i32,
    pub eqtb_top: i32,
    pub hash_top: i32,
}

impl Default for EqtbState {
    fn default() -> Self {
        EqtbState {
            hash_used: 0,
            hash_high: 0,
            no_new_control_sequence: false,
            prim_used: 0,
            cs_count: 0,
            // Allocated in `ini.rs` (initex / format-load) like the other
            // Rust-owned arena arrays; empty until then.
            data: Vec::new(),
            // PRIM_SIZE + 1 = 2101.
            prim: vec![crate::mem_types::B32x2 { s0: 0, s1: 0 }; 2101],
            // EQTB_SIZE - INT_BASE + 1.
            xeq_level: vec![0u16; 1_114_730],
            hash: Vec::new(),
            hash_extra: 0,
            eqtb_top: 0,
            hash_top: 0,
        }
    }
}

impl EqtbState {
    /// Read `eqtb[p].b32.s1`.
    #[inline]
    pub fn b32s1(&self, p: i32) -> i32 {
        // SAFETY: `MemoryWord` union projection; caller ensures p is a valid eqtb index.
        unsafe { self.data[p as usize].b32.s1 }
    }

    /// Write `eqtb[p].b32.s1`.
    #[inline]
    pub fn set_b32s1(&mut self, p: i32, v: i32) {
        self.data[p as usize].b32.s1 = v;
    }

    /// Read `hash[p].s0` (the link half of a hash-table entry).
    #[inline]
    pub fn hash_s0(&self, p: i32) -> i32 {
        self.hash[(p as usize) - 514].s0
    }

    /// Write `hash[p].s0 = v`.
    #[inline]
    pub fn set_hash_s0(&mut self, p: i32, v: i32) {
        self.hash[(p as usize) - 514].s0 = v;
    }

    /// Read `hash[p].s1` (the "text" half of a hash-table entry).
    #[inline]
    pub fn hash_s1(&self, p: i32) -> i32 {
        self.hash[(p as usize) - 514].s1
    }

    /// Write `hash[p].s1 = v`.
    #[inline]
    pub fn set_hash_s1(&mut self, p: i32, v: i32) {
        self.hash[(p as usize) - 514].s1 = v;
    }

    /// Read `prim[p].s0` (the link half of a primitive-table entry).
    #[inline]
    pub fn prim_s0(&self, p: i32) -> i32 {
        self.prim[p as usize].s0
    }

    /// Write `prim[p].s0 = v`.
    #[inline]
    pub fn set_prim_s0(&mut self, p: i32, v: i32) {
        self.prim[p as usize].s0 = v;
    }

    /// Read `prim[p].s1` (the "text" half of a primitive-table entry).
    #[inline]
    pub fn prim_s1(&self, p: i32) -> i32 {
        self.prim[p as usize].s1
    }

    /// Write `prim[p].s1 = v`.
    #[inline]
    pub fn set_prim_s1(&mut self, p: i32, v: i32) {
        self.prim[p as usize].s1 = v;
    }

    /// Copy the full `MemoryWord` at `eqtb[p]`.
    #[inline]
    pub fn word(&self, p: i32) -> crate::mem_types::MemoryWord {
        self.data[p as usize]
    }

    /// Read `eqtb[p].b16.s1` (the "type" half).
    #[inline]
    pub fn b16s1(&self, p: i32) -> u16 {
        // SAFETY: `MemoryWord` union projection; caller ensures p is a valid eqtb index.
        unsafe { self.data[p as usize].b16.s1 }
    }

    /// Write `eqtb[p].b16.s1`.
    #[inline]
    pub fn set_b16s1(&mut self, p: i32, v: u16) {
        self.data[p as usize].b16.s1 = v;
    }

    /// Read `eqtb[p].b16.s0` (the "level" half).
    #[inline]
    pub fn b16s0(&self, p: i32) -> u16 {
        // SAFETY: `MemoryWord` union projection; caller ensures p is a valid eqtb index.
        unsafe { self.data[p as usize].b16.s0 }
    }

    /// Write `eqtb[p].b16.s0`.
    #[inline]
    pub fn set_b16s0(&mut self, p: i32, v: u16) {
        self.data[p as usize].b16.s0 = v;
    }

    /// Read `XEQ_LEVEL(p)` = `xeq_level[p - INT_BASE]`.
    #[inline]
    pub fn xeq_level_at(&self, p: i32) -> u16 {
        self.xeq_level[(p - crate::consts::INT_BASE) as usize]
    }

    /// Write `XEQ_LEVEL(p) = v`.
    #[inline]
    pub fn set_xeq_level_at(&mut self, p: i32, v: u16) {
        self.xeq_level[(p - crate::consts::INT_BASE) as usize] = v;
    }
}

/// The dynamic-memory arena (former C-`malloc`'d `memory_word *mem` plus its
/// bookkeeping scalars). The `mem` array is the engine's hottest storage; its
/// bookkeeping scalars (`lo_mem_max`, `hi_mem_min`, `avail`, `mem_end`, `rover`,
/// `var_used`, `dyn_used`) move *with* the array. `data` is a Rust-owned `Vec`,
/// allocated in `ini.rs` (initex / format-load) and dropped by `tt_cleanup`; it
/// must **never** reallocate during a run so the pointer-escaping accessors stay
/// valid. In the format dump (the free-list `dump_slice`/`undump_slice`
/// regions); those paths take a state-derived base pointer. Element
/// `memory_word` (faithful `MemoryWord`) and the scalars are plain `int32_t`
/// mem-pointers/counters — no newtype debt (the mem-pointer scalars are
/// pointer-taxonomy candidates only).
#[derive(Default)]
pub(crate) struct MemState {
    pub data: Vec<crate::mem_types::MemoryWord>,
    pub lo_mem_max: i32,
    pub hi_mem_min: i32,
    pub avail: i32,
    pub mem_end: i32,
    pub rover: i32,
    pub var_used: i32,
    pub dyn_used: i32,
    /// Current source line number — TeX's `line`.
    /// Lives in MemState because `get_node` (the sole mem-only reader, called
    /// from hundreds of `&mut MemState`-only sites) stamps it into the synctex
    /// trailer of MEDIUM+ nodes; it is the allocation-time source line.
    pub line: i32,
    /// Mirror of `st.input.cur_input.synctex_tag` for `get_node`'s synctex trailer.
    /// `synctex_tag` proper is a stack-saved field of the `cur_input`
    /// struct and can't leave it; `get_node` holds only `&mut MemState`, so it reads
    /// this mirror, kept in sync at the 7 sites that write `cur_input.synctex_tag`.
    pub cur_synctex_tag: i32,
    /// General-purpose scratch node pointer — TeX's `temp_ptr`. Lives here
    /// (not ScannerState) because every user holds `&mut MemState`; two-phase
    /// borrows keep `mem.f(mem.temp_ptr.0,…)` legal. Function-local scratch
    /// reused across many routines.
    pub temp_ptr: crate::units::MemPtr,
}

impl MemState {
    /// Read `mem[p].b32.s0`.
    #[inline]
    pub(crate) fn b32s0(&self, p: impl crate::units::MemAddr) -> i32 {
        // SAFETY: union projection; the `Vec` index is bounds-checked.
        unsafe { self.data[p.addr() as usize].b32.s0 }
    }

    /// Read `mem[p].b32.s1` (the `LLIST_link` / successor-pointer field).
    #[inline]
    pub(crate) fn b32s1(&self, p: impl crate::units::MemAddr) -> i32 {
        // SAFETY: union projection; the `Vec` index is bounds-checked.
        unsafe { self.data[p.addr() as usize].b32.s1 }
    }

    /// Read `mem[p].b32.s1` as a nullable link — the typed sibling of
    /// [`b32s1`](Self::b32s1) for reading a list head off a pseudo-node
    /// (`ALIGN_HEAD`/`TEMP_HEAD`/`PAGE_HEAD`/`cur_list.head`) where no
    /// `NodePtr` family applies.
    #[inline]
    pub(crate) fn link_ptr(&self, p: impl crate::units::MemAddr) -> crate::units::MemPtr {
        crate::units::MemPtr::from_raw(self.b32s1(p))
    }

    /// Read `mem[p].b16.s0`.
    #[inline]
    pub(crate) fn b16s0(&self, p: impl crate::units::MemAddr) -> u16 {
        // SAFETY: union projection; the `Vec` index is bounds-checked.
        unsafe { self.data[p.addr() as usize].b16.s0 }
    }

    /// Read `mem[p].b16.s1`.
    #[inline]
    pub(crate) fn b16s1(&self, p: impl crate::units::MemAddr) -> u16 {
        // SAFETY: union projection; the `Vec` index is bounds-checked.
        unsafe { self.data[p.addr() as usize].b16.s1 }
    }

    /// Read `mem[p].b16.s2`.
    #[inline]
    pub(crate) fn b16s2(&self, p: impl crate::units::MemAddr) -> u16 {
        // SAFETY: union projection; the `Vec` index is bounds-checked.
        unsafe { self.data[p.addr() as usize].b16.s2 }
    }

    /// Read `mem[p].b16.s3`.
    #[inline]
    pub(crate) fn b16s3(&self, p: impl crate::units::MemAddr) -> u16 {
        // SAFETY: union projection; the `Vec` index is bounds-checked.
        unsafe { self.data[p.addr() as usize].b16.s3 }
    }

    /// Read `mem[p].ptr` (the native-glyph pointer field).
    #[inline]
    pub(crate) fn ptr(&self, p: impl crate::units::MemAddr) -> *mut std::ffi::c_void {
        // SAFETY: union projection; the `Vec` index is bounds-checked.
        unsafe { self.data[p.addr() as usize].ptr }
    }

    /// Write `mem[p].ptr`. Union writes through a `&mut` index are safe.
    #[inline]
    pub(crate) fn set_ptr(&mut self, p: impl crate::units::MemAddr, v: *mut std::ffi::c_void) {
        self.data[p.addr() as usize].ptr = v;
    }

    /// Write `mem[p].b32.s0`. Union writes through a `&mut` index are safe.
    #[inline]
    pub(crate) fn set_b32s0(&mut self, p: impl crate::units::MemAddr, v: i32) {
        self.data[p.addr() as usize].b32.s0 = v;
    }

    /// Write `mem[p].b32.s1` (the `LLIST_link` field).
    #[inline]
    pub(crate) fn set_b32s1(&mut self, p: impl crate::units::MemAddr, v: i32) {
        self.data[p.addr() as usize].b32.s1 = v;
    }

    /// Write `mem[p].b16.s0` (low 16 bits of the first half-word). Union writes
    /// through a `&mut` index are safe.
    #[inline]
    pub(crate) fn set_b16s0(&mut self, p: impl crate::units::MemAddr, v: u16) {
        self.data[p.addr() as usize].b16.s0 = v;
    }

    /// Write `mem[p].b16.s1` (high 16 bits of the first half-word).
    #[inline]
    pub(crate) fn set_b16s1(&mut self, p: impl crate::units::MemAddr, v: u16) {
        self.data[p.addr() as usize].b16.s1 = v;
    }

    /// Write `mem[p].b16.s2`.
    #[inline]
    pub(crate) fn set_b16s2(&mut self, p: impl crate::units::MemAddr, v: u16) {
        self.data[p.addr() as usize].b16.s2 = v;
    }

    /// Write `mem[p].b16.s3`.
    #[inline]
    pub(crate) fn set_b16s3(&mut self, p: impl crate::units::MemAddr, v: u16) {
        self.data[p.addr() as usize].b16.s3 = v;
    }

    /// Read the `gr` (f64) field of `mem[p]`.
    #[inline]
    pub(crate) fn gr(&self, p: impl crate::units::MemAddr) -> f64 {
        // SAFETY: `MemoryWord` union projection; the `Vec` index is bounds-checked.
        unsafe { self.data[p.addr() as usize].gr }
    }

    /// Write the `gr` (f64) field of `mem[p]`.
    #[inline]
    pub(crate) fn set_gr(&mut self, p: impl crate::units::MemAddr, v: f64) {
        self.data[p.addr() as usize].gr = v;
    }

    /// Reinterpret the arena from word `p` onward as a `*mut u16`. Valid past
    /// the borrow because the thread-local owner never reallocates.
    #[inline]
    pub(crate) fn as_u16_ptr_mut(&mut self, p: impl crate::units::MemAddr) -> *mut u16 {
        self.data.as_mut_ptr().wrapping_add(p.addr() as usize) as *mut u16
    }

    /// Reinterpret the arena from word `p` onward as a `*const u16`. Valid past
    /// the borrow because the thread-local owner never reallocates.
    #[inline]
    pub(crate) fn as_u16_ptr(&self, p: impl crate::units::MemAddr) -> *const u16 {
        self.data.as_ptr().wrapping_add(p.addr() as usize) as *const u16
    }

    /// Reinterpret the arena from word `p` onward as a `*mut std::ffi::c_void`. Valid
    /// past the borrow because the thread-local owner never reallocates.
    #[inline]
    pub(crate) fn node_addr(&self, p: impl crate::units::MemAddr) -> *mut std::ffi::c_void {
        self.data.as_ptr().wrapping_add(p.addr() as usize) as *mut std::ffi::c_void
    }

    /// Copy the full `MemoryWord` at `mem[p]`.
    #[inline]
    pub(crate) fn word(&self, p: impl crate::units::MemAddr) -> crate::mem_types::MemoryWord {
        self.data[p.addr() as usize]
    }

    /// Write a full `MemoryWord` to `mem[p]`.
    #[inline]
    pub(crate) fn set_word(&mut self, p: impl crate::units::MemAddr, w: crate::mem_types::MemoryWord) {
        self.data[p.addr() as usize] = w;
    }
}

/// The kind of grouping level on the save stack (`cur_group`,
/// xetex-constants.h:152–169). A pure classification field — compared for
/// equality and dispatched in `handle_right_brace`/`off_save`/`show_save_groups`,
/// never used in arithmetic — so it is typed directly rather than kept as a raw
/// `u8`. Replaces the ~70 duplicated `const SIMPLE_GROUP`/`HBOX_GROUP`/… code
/// definitions that were scattered across main_control/output/math/save/errors.
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
    /// Decode a raw `cur_group` byte; `None` for an unknown code.
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

    /// The raw `cur_group` byte for this group (`self as u8`); used only at the
    /// save-stack (de)serialization boundary.
    #[inline]
    pub(crate) fn as_raw(self) -> u8 {
        self as u8
    }
}

#[derive(Default)]
pub(crate) struct SaveStackState {
    pub save_ptr: i32,
    pub max_save_stack: i32,
    /// Capacity of the save stack (`save_size`); set once at engine init.
    pub save_size: i32,
    pub cur_level: u16,
    pub cur_group: GroupCode,
    pub cur_boundary: i32,
    /// Rust-owned save-stack arena (`save_size + 1` words). Allocated at
    /// engine init and released by `tt_cleanup`; never reallocated during a
    /// run. Replaces the former C-`malloc`'d `save_stack` pointer.
    pub data: Vec<crate::mem_types::MemoryWord>,
}

impl SaveStackState {
    /// Read `save_stack[k].b32.s1`.
    #[inline]
    pub(crate) fn b32s1(&self, k: i32) -> i32 {
        // SAFETY: union projection; the entry was last written via the b32 view.
        unsafe { self.data[k as usize].b32.s1 }
    }

    /// Write `save_stack[k].b32.s1`. Union writes through a `&mut` index are safe.
    #[inline]
    pub(crate) fn set_b32s1(&mut self, k: i32, v: i32) {
        self.data[k as usize].b32.s1 = v;
    }

    /// Write `save_stack[k].b16.s1`. Union writes through a `&mut` index are safe.
    #[inline]
    pub(crate) fn set_b16s1(&mut self, k: i32, v: u16) {
        self.data[k as usize].b16.s1 = v;
    }

    /// Write `save_stack[k].b16.s0`. Union writes through a `&mut` index are safe.
    #[inline]
    pub(crate) fn set_b16s0(&mut self, k: i32, v: u16) {
        self.data[k as usize].b16.s0 = v;
    }

    /// Write a full `MemoryWord` to `save_stack[k]`.
    #[inline]
    pub(crate) fn set_word(&mut self, k: i32, w: crate::mem_types::MemoryWord) {
        self.data[k as usize] = w;
    }

    /// Read `save_stack[k].b16.s1`.
    #[inline]
    pub(crate) fn b16s1(&self, k: i32) -> u16 {
        // SAFETY: union projection; the entry was last written via the b16 view.
        unsafe { self.data[k as usize].b16.s1 }
    }

    /// Read `save_stack[k].b16.s0`.
    #[inline]
    pub(crate) fn b16s0(&self, k: i32) -> u16 {
        // SAFETY: union projection; the entry was last written via the b16 view.
        unsafe { self.data[k as usize].b16.s0 }
    }

    /// Read a full `MemoryWord` from `save_stack[k]`.
    #[inline]
    pub(crate) fn word(&self, k: i32) -> crate::mem_types::MemoryWord {
        self.data[k as usize]
    }
}

#[derive(Default)]
pub(crate) struct ScannerState {
    pub cur_name: i32,
    pub cur_area: i32,
    pub cur_ext: i32,
    pub area_delimiter: i32,
    pub ext_delimiter: i32,
    pub quoted_filename: bool,
    pub file_name_quote_char: u16,
    pub name_in_progress: bool,
    pub stop_at_space: bool,
    pub cur_align: crate::units::MemPtr,
    pub cur_span: i32,
    pub cur_loop: crate::units::MemPtr,
    pub align_ptr: i32,
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
    /// Head of the conditional-stack list (xetex-xetexd.h:537). Plain `int32_t`.
    pub cond_ptr: crate::units::MemPtr,
    /// Current `\if` nesting limit code (xetex-xetexd.h:538). Stored as `u8`
    /// (`unsigned char` in C); plain, no semantic typedef debt.
    pub if_limit: u8,
    /// Current `\if` type (xetex-xetexd.h:539; C type `small_number` = `short`).
    /// Newtype debt: `SmallNumber` (same as `radix`).
    pub cur_if: i16,
    /// Line number where the current `\if` was entered (xetex-xetexd.h:540).
    /// Plain `int32_t`.
    pub if_line: i32,
    /// Line number where the current if-skip started (xetex-xetexd.h:541).
    /// Plain `int32_t`.
    pub skip_line: i32,
    /// True while scanning between `\csname...\endcsname` (xetex-xetexd.h:526).
    /// Plain `bool`.
    pub is_in_csname: bool,
    /// True after `\TectonicCodaTokens` was inserted once (xetex-xetexd.h:723).
    /// Plain `bool`.
    pub used_tectonic_coda_tokens: bool,
    /// Value-type level for the current scanned quantity (xetex-xetexd.h:532;
    /// `unsigned char`; 0=INT_VAL, 1=DIMEN_VAL, 2=GLUE_VAL, 3=MU_VAL). Plain
    /// `unsigned char`, no semantic typedef, no newtype debt.
    pub cur_val_level: u8,
    /// Second parsed value register for `scan_something_internal`
    /// (xetex-xetexd.h:531). Plain `int32_t`.
    pub cur_val1: i32,
    /// Scanner state (xetex-xetexd.h:514; `unsigned char`; NORMAL=0..ABSORBING=5).
    /// Plain `unsigned char`, no semantic typedef, no newtype debt.
    pub scanner_status: u8,
    /// Control-sequence index used in scanner-error messages (xetex-xetexd.h:515).
    /// Plain `int32_t`.
    pub warning_index: i32,
    /// Head of the current macro-definition token list (xetex-xetexd.h:516).
    /// Plain `int32_t`.
    pub def_ref: crate::units::MemPtr,
    /// Macro call type (xetex-xetexd.h:528; `unsigned char`;
    /// CALL/LONG_CALL/OUTER_CALL/LONG_OUTER_CALL). Plain, no newtype debt.
    pub long_state: u8,
    /// Alignment brace-nesting counter (xetex-xetexd.h:520). Positive = extra
    /// open braces; negative = unmatched `&` or `\cr`. Plain `int32_t`.
    pub align_state: i32,
    /// Expansion-depth limit (`expand_depth`); set once at init.
    pub expand_depth: i32,
    /// Current command code — the scanner's `cur_cmd` (`eight_bits`).
    pub cur_cmd: u8,
    /// Current character code — the scanner's `cur_chr`.
    pub cur_chr: i32,
    /// Current control-sequence index — the scanner's `cur_cs`.
    pub cur_cs: i32,
    /// Current token — the scanner's `cur_tok`.
    pub cur_tok: i32,
    /// Current parsed integer value — the scanner's `cur_val`.
    pub cur_val: i32,
    /// Sparse-array cursor — TeX's `cur_ptr`. Sibling to `sa_root`, set/read by
    /// the sa-element machinery.
    pub cur_ptr: crate::units::MemPtr,
    /// Mark registers: `cur_mark[0..=4]` (xetex-xetexd.h:527). Each entry is
    /// a token-list pointer (`int32_t`). Inline fixed array; no alloc/cleanup
    /// edge. Plain `int32_t` elements, no newtype debt.
    pub cur_mark: [i32; 5],
    /// Saved discretionary break pointers (xetex-xetexd.h:716). Inline fixed
    /// array of 4 `int32_t` half-word pointers. No alloc/cleanup edge. Plain
    /// `int32_t` elements, no newtype debt.
    pub disc_ptr: [i32; 4],
    // Math-list-to-hlist conversion scratch (former xetex-math.c module
    // globals).
    /// Mlist being converted by `mlist_to_hlist` (xetex-math.c). Mem pointer
    /// stored as `int32_t`; pointer-taxonomy-sweep candidate, no typedef debt.
    pub cur_mlist: crate::units::MemPtr,
    /// Current math style (C type `small_number` = `short`). Newtype debt:
    /// `SmallNumber`.
    pub cur_style: i16,
    /// Current math size code (`int32_t`; TEXT_SIZE/SCRIPT_SIZE/...). Plain.
    pub cur_size: i32,
    /// Current `math unit` (`mu`) value (C type `scaled_t`). Newtype debt:
    /// `Scaled`.
    pub cur_mu: i32,
    /// Whether `mlist_to_hlist` inserts penalties (`bool`). Plain.
    pub mlist_penalties: bool,
    pub cur_box: crate::units::MemPtr,
    pub after_token: i32,
    pub cur_order: u8,
    pub adjust_tail: crate::units::MemPtr,
    pub pre_adjust_tail: crate::units::MemPtr,
    pub total_stretch: [i32; 4],
    pub total_shrink: [i32; 4],
    pub just_box: crate::units::MemPtr,
    pub cur_f: i32,
    pub cur_c: i32,
    pub cur_i: crate::mem_types::B16x4,
    pub pack_begin_line: i32,
    pub set_box_allowed: bool,
    pub active_width: [i32; 7],
    pub best_height_plus_depth: i32,
    // TeXXeT left-right directionality state.
    pub lr_ptr: crate::units::MemPtr,
    pub lr_problems: i32,
    pub cur_dir: i16,
    /// Sparse-array trie roots: `sa_root[0..=7]` (xetex-xetexd.h:709).
    pub sa_root: [i32; 8],
    /// Sparse-array null sentinel (xetex-ini.c:3203).
    pub sa_null: crate::mem_types::MemoryWord,
    /// The all-zero B32x2 template (xetex-xetexd.h:608).
    pub empty: crate::mem_types::B32x2,
    /// Register-size limit help message pointer (xetex-xetexd.h:708).
    pub max_reg_help_line: *const libc::c_char,
    /// Rust-owned dynamic native text buffer.
    pub native_text: Vec<u16>,
    pub native_text_size: i32,
    pub native_len: i32,
    pub save_native_len: i32,
}

#[derive(Default, Clone, Copy)]
pub(crate) struct ConfigState {
    pub file_line_error_style_p: i32,
    pub halt_on_error_p: i32,
    pub in_initex_mode: bool,
    pub synctex_enabled: i32,
    pub semantic_pagination_enabled: bool,
    pub shell_escape_enabled: bool,
    pub max_print_line: i32,
}

pub(crate) struct FontState {
    pub loaded_font_mapping: *mut libc::c_void,
    pub loaded_font_flags: libc::c_char,
    /// Rust-owned per-font layout engine handle table (`font_max + 1` entries).
    /// Stores `XeTeXLayoutEngine` (opaque void *) pointers; not format-
    /// serialized; handles are released explicitly in `tt_cleanup` before the
    /// Vec is dropped. Plain `*mut c_void`, no newtype debt.
    pub font_layout_engine: Vec<*mut libc::c_void>,
    /// Rust-owned per-font TECkit mapping handle table (`font_max + 1` entries).
    /// Stores `TECkit_Converter` (opaque void *) pointers; not format-serialized;
    /// converters are disposed explicitly in `tt_cleanup`. Plain `*mut c_void`.
    pub font_mapping: Vec<*mut libc::c_void>,
    pub loaded_font_letter_space: i32,
    pub xtx_ligature_present: bool,
    pub gave_char_warning_help: bool,
    pub native_font_type_flag: i32,
    pub font_mem_size: i32,
    pub font_max: i32,
    /// Current font-memory write pointer; advances as font metrics are loaded.
    /// C type `font_index` (semantic typedef for `int32_t`; newtype debt).
    pub fmem_ptr: i32,
    /// Highest font number loaded so far. C type `internal_font_number`
    /// (semantic typedef; newtype debt).
    pub font_ptr: i32,
    /// Per-font renderer flags (`char *`, one byte per font slot). Not
    /// format-serialized; reallocated fresh at init / format-load.
    pub font_flags: Vec<i8>,
    /// Per-font letter-spacing value (`scaled_t *`). Not format-serialized;
    /// reallocated fresh at init / format-load. Newtype debt.
    pub font_letter_space: Vec<i32>,
    pub null_character: crate::mem_types::B16x4,
    /// Rust-owned font checksum table (`font_max + 1` entries, matching the
    /// former `xmalloc_array` shape). Allocated at engine init / format load
    /// and released by `tt_cleanup`; serialized in the format dump.
    pub font_check: Vec<crate::mem_types::B16x4>,
    /// Rust-owned font size tables (`font_max + 1` scaled values each).
    /// Allocated at engine init / format load and released by `tt_cleanup`;
    /// serialized in the format dump. Element type is `scaled_t` (newtype debt).
    pub font_size: Vec<i32>,
    pub font_dsize: Vec<i32>,
    /// Rust-owned font-parameter count table (`font_max + 1` entries).
    /// Serialized in the format dump. Element type is `font_index` (newtype debt).
    pub font_params: Vec<i32>,
    /// Rust-owned font name/area tables (`font_max + 1` string-pool indexes).
    /// Serialized in the format dump. Element type is `str_number` (newtype debt).
    pub font_name: Vec<i32>,
    pub font_area: Vec<i32>,
    /// Rust-owned first/last valid character-code tables (`font_max + 1`
    /// entries each). Serialized in the format dump. Element type is
    /// `UTF16_code` (newtype debt).
    pub font_bc: Vec<u16>,
    pub font_ec: Vec<u16>,
    /// Rust-owned boundary-character tables (`font_max + 1` entries each).
    /// Serialized in the format dump. Element types are `font_index` /
    /// `nine_bits` (newtype debt).
    pub bchar_label: Vec<i32>,
    pub font_bchar: Vec<i32>,
    pub font_false_bchar: Vec<i32>,
    /// Rust-owned per-font glue-spec pointer table (`font_max + 1` entries).
    /// Serialized in the format dump (checked undump against `lo_mem_max`).
    /// Element is a nullable memory/halfword pointer (newtype debt).
    pub font_glue: Vec<i32>,
    /// Rust-owned per-font "emitted in output" flag table (`font_max + 1`
    /// entries). Not serialized in the format dump; plain `bool`.
    pub font_used: Vec<bool>,
    /// Rust-owned font metric offset index tables (`font_max + 1` entries
    /// each). These hold offsets into the `font_info` arena for each per-font
    /// metric table (char info, widths, heights, depths, italic corrections,
    /// lig/kern, kern, extensible, parameters). All serialized in the format
    /// dump; element type is plain `int32_t` (no semantic typedef in the C
    /// header).
    pub char_base: Vec<i32>,
    pub width_base: Vec<i32>,
    pub height_base: Vec<i32>,
    pub depth_base: Vec<i32>,
    pub italic_base: Vec<i32>,
    pub lig_kern_base: Vec<i32>,
    pub kern_base: Vec<i32>,
    pub exten_base: Vec<i32>,
    pub param_base: Vec<i32>,
    /// Rust-owned per-font hyphen-character tables (`font_max + 1` entries).
    /// Serialized in the format dump. Plain `int32_t *`, no newtype debt.
    pub hyphen_char: Vec<i32>,
    /// Rust-owned per-font skew-character tables (`font_max + 1` entries).
    /// Serialized in the format dump. Plain `int32_t *`, no newtype debt.
    pub skew_char: Vec<i32>,
    /// Rust-owned font metric arena (`font_mem_size + 1` words). Holds all
    /// per-font metric tables (widths, heights, depths, italic corrections,
    /// lig/kern programs, kern values, extensible recipes, font parameters).
    /// `fmem_ptr` is the next-free write pointer into this arena. Serialized
    /// in the format dump. Plain `MemoryWord *`, no newtype debt.
    pub font_info: Vec<crate::mem_types::MemoryWord>,
    /// Rust-owned dynamic XDV buffer.
    pub xdv_buffer: Vec<u8>,
    /// Rust-owned dynamic mapped text buffer.
    pub mapped_text: Vec<u16>,
}

impl Default for FontState {
    fn default() -> Self {
        Self {
            loaded_font_mapping: std::ptr::null_mut(),
            loaded_font_flags: 0,
            font_layout_engine: Vec::new(),
            font_mapping: Vec::new(),
            loaded_font_letter_space: 0,
            xtx_ligature_present: false,
            gave_char_warning_help: false,
            native_font_type_flag: 0,
            font_mem_size: 0,
            font_max: 0,
            fmem_ptr: 0,
            font_ptr: 0,
            font_flags: Vec::new(),
            font_letter_space: Vec::new(),
            null_character: crate::mem_types::B16x4 {
                s0: 0,
                s1: 0,
                s2: 0,
                s3: 0,
            },
            font_check: Vec::new(),
            font_size: Vec::new(),
            font_dsize: Vec::new(),
            font_params: Vec::new(),
            font_name: Vec::new(),
            font_area: Vec::new(),
            font_bc: Vec::new(),
            font_ec: Vec::new(),
            bchar_label: Vec::new(),
            font_bchar: Vec::new(),
            font_false_bchar: Vec::new(),
            font_glue: Vec::new(),
            font_used: Vec::new(),
            char_base: Vec::new(),
            width_base: Vec::new(),
            height_base: Vec::new(),
            depth_base: Vec::new(),
            italic_base: Vec::new(),
            lig_kern_base: Vec::new(),
            kern_base: Vec::new(),
            exten_base: Vec::new(),
            param_base: Vec::new(),
            hyphen_char: Vec::new(),
            skew_char: Vec::new(),
            font_info: Vec::new(),
            xdv_buffer: Vec::new(),
            mapped_text: Vec::new(),
        }
    }
}

impl FontState {
    #[inline]
    pub(crate) fn font_area_entry(&self, f: i32) -> i32 {
        self.font_area[f as usize]
    }

    #[inline]
    pub(crate) fn font_layout_engine_ptr(&self, f: i32) -> *mut libc::c_void {
        self.font_layout_engine[f as usize]
    }

    #[inline]
    pub(crate) fn font_letter_space_entry(&self, f: i32) -> i32 {
        self.font_letter_space[f as usize]
    }

    /// Read `font_flags[f]`.
    #[inline]
    pub(crate) fn font_flags_entry(&self, f: i32) -> i32 {
        self.font_flags[f as usize] as i32
    }

    /// Read `font_bc[f]`.
    #[inline]
    pub(crate) fn font_bc_entry(&self, f: i32) -> u16 {
        self.font_bc[f as usize]
    }

    /// Read `font_ec[f]`.
    #[inline]
    pub(crate) fn font_ec_entry(&self, f: i32) -> u16 {
        self.font_ec[f as usize]
    }

    #[inline]
    pub(crate) fn font_mapping_ptr(&self, f: i32) -> *mut libc::c_void {
        self.font_mapping[f as usize]
    }

    #[inline]
    pub(crate) fn xtx_ligature_present(&self) -> bool {
        self.xtx_ligature_present
    }

    #[inline]
    pub(crate) fn set_xtx_ligature_present(&mut self, v: bool) {
        self.xtx_ligature_present = v;
    }

    #[inline]
    pub(crate) fn font_info_b16(&self, k: i32) -> crate::mem_types::B16x4 {
        // SAFETY: union projection; callers preserve the original font-info indexing contract.
        unsafe { self.font_info[k as usize].b16 }
    }

    #[inline]
    pub(crate) fn font_info_b32s1(&self, k: i32) -> i32 {
        // SAFETY: union projection; callers preserve the original font-info indexing contract.
        unsafe { self.font_info[k as usize].b32.s1 }
    }

    #[inline]
    pub(crate) fn font_params(&self, f: i32) -> i32 {
        self.font_params[f as usize]
    }

    #[inline]
    pub(crate) fn param_base(&self, f: i32) -> i32 {
        self.param_base[f as usize]
    }

    #[inline]
    pub(crate) fn bchar_label_entry(&self, f: i32) -> i32 {
        self.bchar_label[f as usize]
    }

    #[inline]
    pub(crate) fn font_bchar_entry(&self, f: i32) -> i32 {
        self.font_bchar[f as usize]
    }

    #[inline]
    pub(crate) fn char_base_entry(&self, f: i32) -> i32 {
        self.char_base[f as usize]
    }

    #[inline]
    pub(crate) fn width_base_entry(&self, f: i32) -> i32 {
        self.width_base[f as usize]
    }

    #[inline]
    pub(crate) fn italic_base_entry(&self, f: i32) -> i32 {
        self.italic_base[f as usize]
    }

    #[inline]
    pub(crate) fn height_base_entry(&self, f: i32) -> i32 {
        self.height_base[f as usize]
    }

    #[inline]
    pub(crate) fn depth_base_entry(&self, f: i32) -> i32 {
        self.depth_base[f as usize]
    }

    #[inline]
    pub(crate) fn exten_base_entry(&self, f: i32) -> i32 {
        self.exten_base[f as usize]
    }

    #[inline]
    pub(crate) fn lig_kern_base_entry(&self, f: i32) -> i32 {
        self.lig_kern_base[f as usize]
    }

    #[inline]
    pub(crate) fn kern_base_entry(&self, f: i32) -> i32 {
        self.kern_base[f as usize]
    }

    #[inline]
    pub(crate) fn skew_char_entry(&self, f: i32) -> i32 {
        self.skew_char[f as usize]
    }

    #[inline]
    pub(crate) fn set_char_base_entry(&mut self, f: i32, v: i32) {
        self.char_base[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_width_base_entry(&mut self, f: i32, v: i32) {
        self.width_base[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_height_base_entry(&mut self, f: i32, v: i32) {
        self.height_base[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_depth_base_entry(&mut self, f: i32, v: i32) {
        self.depth_base[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_italic_base_entry(&mut self, f: i32, v: i32) {
        self.italic_base[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_lig_kern_base_entry(&mut self, f: i32, v: i32) {
        self.lig_kern_base[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_kern_base_entry(&mut self, f: i32, v: i32) {
        self.kern_base[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_exten_base_entry(&mut self, f: i32, v: i32) {
        self.exten_base[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_param_base(&mut self, f: i32, v: i32) {
        self.param_base[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_font_check(&mut self, f: i32, v: crate::mem_types::B16x4) {
        self.font_check[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_font_dsize_entry(&mut self, f: i32, v: i32) {
        self.font_dsize[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_font_size_entry(&mut self, f: i32, v: i32) {
        self.font_size[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_font_info_b16(&mut self, k: i32, v: crate::mem_types::B16x4) {
        self.font_info[k as usize].b16 = v;
    }
    #[inline]
    pub(crate) fn set_font_info_b32s1(&mut self, k: i32, v: i32) {
        self.font_info[k as usize].b32.s1 = v;
    }
    #[inline]
    pub(crate) fn set_font_params(&mut self, f: i32, v: i32) {
        self.font_params[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_hyphen_char_entry(&mut self, f: i32, v: i32) {
        self.hyphen_char[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_skew_char_entry(&mut self, f: i32, v: i32) {
        self.skew_char[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_bchar_label_entry(&mut self, f: i32, v: i32) {
        self.bchar_label[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_font_bchar_entry(&mut self, f: i32, v: i32) {
        self.font_bchar[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_font_false_bchar_entry(&mut self, f: i32, v: i32) {
        self.font_false_bchar[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_font_name_entry(&mut self, f: i32, v: i32) {
        self.font_name[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_font_area_entry(&mut self, f: i32, v: i32) {
        self.font_area[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_font_bc_entry(&mut self, f: i32, v: u16) {
        self.font_bc[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_font_ec_entry(&mut self, f: i32, v: u16) {
        self.font_ec[f as usize] = v;
    }
    #[inline]
    pub(crate) fn font_glue(&self, f: i32) -> i32 {
        self.font_glue[f as usize]
    }
    #[inline]
    pub(crate) fn hyphen_char_entry(&self, f: i32) -> i32 {
        self.hyphen_char[f as usize]
    }
    #[inline]
    pub(crate) fn font_false_bchar_entry(&self, f: i32) -> i32 {
        self.font_false_bchar[f as usize]
    }
    #[inline]
    pub(crate) fn set_font_glue(&mut self, f: i32, v: i32) {
        self.font_glue[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_font_used_entry(&mut self, f: i32, v: bool) {
        self.font_used[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_font_layout_engine_ptr(&mut self, f: i32, v: *mut libc::c_void) {
        self.font_layout_engine[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_font_mapping_ptr(&mut self, f: i32, v: *mut libc::c_void) {
        self.font_mapping[f as usize] = v;
    }
    #[inline]
    pub(crate) fn set_font_letter_space_entry(&mut self, f: i32, v: i32) {
        self.font_letter_space[f as usize] = v;
    }
}

#[derive(Default)]
pub(crate) struct ShipoutState {
    pub insert_src_special_every_par: bool,
    pub insert_src_special_every_math: bool,
    pub insert_src_special_every_vbox: bool,
    pub insert_src_special_auto: bool,
    pub selector: i32,
    pub doing_special: bool,
    pub term_offset: i32,
    pub file_offset: i32,
    pub old_setting: u8,
    pub epochseconds: i32,
    pub microseconds: i32,
    pub dead_cycles: i32,
    pub page_tail: crate::units::MemPtr,
    pub last_badness: i32,
    pub page_contents: u8,
    pub page_so_far: [i32; 8],
    pub last_glue: crate::units::MemPtr,
    pub last_penalty: i32,
    pub last_kern: i32,
    pub last_node_type: i32,
    pub insert_penalties: i32,
    pub output_active: bool,
    pub pdf_last_x_pos: i32,
    pub pdf_last_y_pos: i32,
    pub write_file: [usize; 16],
    pub write_open: [bool; 18],
    pub rust_stdout: usize,
    pub log_file: usize,
    pub current_diagnostic: *mut std::ffi::c_void,
    // Page-builder scratch.
    pub best_page_break: crate::units::MemPtr,
    pub best_size: i32,
    pub least_page_cost: i32,
    pub page_max_depth: i32,
    // DVI-output scratch (page/position tracking).
    pub total_pages: i32,
    pub max_v: i32,
    pub max_h: i32,
    pub max_push: i32,
    pub last_bop: i32,
    pub doing_leaders: bool,
    pub rule_ht: i32,
    pub rule_dp: i32,
    pub rule_wd: i32,
    pub cur_h: i32,
    pub cur_v: i32,
    pub cur_page_width: i32,
    pub cur_page_height: i32,
    pub cur_h_offset: i32,
    pub cur_v_offset: i32,
    pub write_loc: i32,
    // DVI (dvi_* positioning and output variables).
    pub dvi_file: usize,
    pub output_file_name: i32,
    pub dvi_buf: Vec<u8>,
    pub dvi_limit: i32,
    pub dvi_ptr: i32,
    pub dvi_offset: i32,
    pub dvi_gone: i32,
    pub down_ptr: crate::units::MemPtr,
    pub right_ptr: crate::units::MemPtr,
    pub cur_s: i32,
    pub dvi_h: i32,
    pub dvi_v: i32,
    pub dvi_f: i32,
}

impl ShipoutState {
    /// Set `page_contents`.
    #[inline]
    pub(crate) fn set_page_contents(&mut self, v: u8) {
        self.page_contents = v;
    }

    /// Set `page_so_far[k]`.
    #[inline]
    pub(crate) fn set_page_so_far(&mut self, k: usize, v: i32) {
        self.page_so_far[k] = v;
    }

    /// Set `page_max_depth`.
    #[inline]
    pub(crate) fn set_page_max_depth(&mut self, v: i32) {
        self.page_max_depth = v;
    }

    /// Set `least_page_cost`.
    #[inline]
    pub(crate) fn set_least_page_cost(&mut self, v: i32) {
        self.least_page_cost = v;
    }
}

/// TeX's error-interaction mode (`\batchmode`/`\nonstopmode`/`\scrollmode`/
/// `\errorstopmode`, xetex-constants.h). Ordered Batch < Nonstop < Scroll <
/// ErrorStop (the numeric encoding), matching the `interaction < ERROR_STOP_MODE`
/// / `> BATCH_MODE` tests that gate terminal prompting and log verbosity.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub(crate) enum InteractionMode {
    Batch = 0,
    Nonstop = 1,
    Scroll = 2,
    ErrorStop = 3,
}

impl InteractionMode {
    /// Decode a scanned mode value; `None` for out-of-range (this IS the
    /// `\interactionmode=N` bounds check — 0..=3).
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

pub(crate) struct ErrorState {
    pub arith_error: bool,
    pub tex_remainder: i32,
    pub dig: [u8; 23],
    pub randoms: [i32; 55],
    pub j_random: u8,
    pub help_line: [*const libc::c_char; 6],
    pub help_ptr: u8,
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
    pub trick_buf: [u16; 256],
    pub trick_count: i32,
    pub first_count: i32,
    pub deletions_allowed: bool,
    pub random_seed: i32,
    pub no_shrink_error_yet: bool,
    pub font_in_short_display: i32,
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
            help_line: [std::ptr::null(); 6],
            help_ptr: 0,
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
            font_in_short_display: 0,
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
    pub job_name: i32,
    pub texmf_log_name: i32,
    pub string_vacancies: i32,
    pub pool_size: i32,
    pub pool_ptr: i32,
    pub format_ident: i32,
    pub edit_name_start: i32,
    pub str_start: Vec<i32>,
    pub str_pool: Vec<u16>,
}

/// The line-breaker's working state (the line-breaking / LR-reverse /
/// hyphenation-attempt scratch globals, defined in C across `xetex-ini.c` and
/// `xetex-linebreak.c`). These are per-paragraph scratch — cold relative to the
/// tokenizer. None are in the format dump and none had a `#[link_name]` cleanup
/// alias (no `ini.rs` alloc, no `tt_cleanup` free).
///
/// `hlist_stack` is heap-backed as a `Vec<i32>` (sized `MAX_HLIST_STACK + 1`
/// = 513 in `Default`, matching the C BSS array `int32_t
/// hlist_stack[MAX_HLIST_STACK + 1]`) rather than an inline `[i32; 513]`:
/// `EngineState` lives inline in the `ENGINE_STATE` thread-local, so a 2 KB
/// inline array risks the thread-stack overflow seen when the hyph fixed
/// arrays were first moved inline. The small fixed arrays stay inline.
///
/// Newtype debts: the `scaled_t` dimension fields and the `small_number`
/// fields. The `int32_t` mem-pointer fields (`cur_p`, `passive`, …) are
/// pointer-taxonomy sweep candidates, not typedef-loss rows.
pub(crate) struct LinebreakState {
    pub last_leftmost_char: crate::units::MemPtr,
    pub last_rightmost_char: crate::units::MemPtr,
    /// Heap-backed; sized `MAX_HLIST_STACK + 1` (= 513) in `Default`.
    pub hlist_stack: Vec<crate::units::MemPtr>,
    pub hlist_stack_level: i16,
    pub first_p: crate::units::MemPtr,
    pub global_prev_p: crate::units::MemPtr,
    pub cur_p: crate::units::MemPtr,
    pub last_special_line: i32,
    pub first_width: i32,
    pub second_width: i32,
    pub first_indent: i32,
    pub second_indent: i32,
    /// The chosen best breakpoint's active node (set by `choose_best_break`;
    /// `None` until then). Typed once here so the readers skip the
    /// `active(..)` `from_idx_unchecked` re-cast.
    pub best_bet: Option<crate::node::NodePtr<crate::node::ActiveNode>>,
    pub best_line: i32,
    pub hn: i16,
    pub ha: crate::units::MemPtr,
    pub hb: crate::units::MemPtr,
    pub passive: crate::units::MemPtr,
    pub cur_active_width: [i32; 7],
    pub background: [i32; 7],
    pub break_width: [i32; 7],
    pub best_place: [crate::units::MemPtr; 4],
    pub best_pl_line: [i32; 4],
    pub disc_width: i32,
    pub final_pass: bool,
    pub threshold: i32,
    pub minimal_demerits: [i32; 4],
    pub minimum_demerits: i32,
    pub easy_line: i32,
    pub do_last_line_fit: bool,
    pub active_node_size: i16,
    pub fill_width: [i32; 3],
    pub best_pl_short: [i32; 4],
    pub best_pl_glue: [i32; 4],
}

impl Default for LinebreakState {
    fn default() -> Self {
        Self {
            last_leftmost_char: crate::units::MemPtr::from_raw(0),
            last_rightmost_char: crate::units::MemPtr::from_raw(0),
            hlist_stack: vec![crate::units::MemPtr::from_raw(0); 513],
            hlist_stack_level: 0,
            first_p: crate::units::MemPtr::from_raw(0),
            global_prev_p: crate::units::MemPtr::from_raw(0),
            cur_p: crate::units::MemPtr::from_raw(0),
            last_special_line: 0,
            first_width: 0,
            second_width: 0,
            first_indent: 0,
            second_indent: 0,
            best_bet: None,
            best_line: 0,
            hn: 0,
            ha: crate::units::MemPtr::from_raw(0),
            hb: crate::units::MemPtr::from_raw(0),
            passive: crate::units::MemPtr::from_raw(0),
            cur_active_width: [0; 7],
            background: [0; 7],
            break_width: [0; 7],
            best_place: [crate::units::MemPtr::from_raw(0); 4],
            best_pl_line: [0; 4],
            disc_width: 0,
            final_pass: false,
            threshold: 0,
            minimal_demerits: [0; 4],
            minimum_demerits: 0,
            easy_line: 0,
            do_last_line_fit: false,
            active_node_size: 0,
            fill_width: [0; 3],
            best_pl_short: [0; 4],
            best_pl_glue: [0; 4],
        }
    }
}

// --- Owned-state locator for the one irreducible C++ callback ----------------
// The engine state is OWNED by `tt_run_engine` (a `Box<EngineState>`). Every
// access goes through `&mut EngineState` — with ONE exception that cannot
// be: `font_feature_warning`, the macOS AAT `#[no_mangle]` callback that fires
// mid-`loadAATfont` deep inside Core Text (xetex-macos.c, built only on macOS)
// with no Rust frame in between to carry a borrow. For it alone, `tt_run_engine`
// installs a raw pointer to its owned state here; the callback reads it via
// `current_engine_state()`. This is a LOCATOR (re-established at each run entry,
// like the `FONT_*` FFI mirrors), NOT engine state — a clone captures the state,
// not this pointer. Never used on the Linux build.
thread_local! {
    static ENGINE_STATE_PTR: std::cell::Cell<*mut EngineState> =
        const { std::cell::Cell::new(std::ptr::null_mut()) };
}

/// Install (or clear, with a null pointer) the owner pointer for the current
/// thread. Called by `tt_run_engine` at entry (set) and on every exit path (clear
/// via a drop guard).
///
/// SAFETY: while non-null, `p` must point to an `EngineState` that outlives every
/// `current_engine_state()` read on this thread until it is cleared.
pub(crate) fn set_engine_state_ptr(p: *mut EngineState) {
    ENGINE_STATE_PTR.with(|c| c.set(p));
}

/// The current run's owned `EngineState`, for the macOS `font_feature_warning`
/// callback only (see the module comment). Null outside a run.
///
/// SAFETY: the caller must not create an aliasing `&mut` while another borrow of
/// the same `EngineState` is live; `font_feature_warning` runs inside a C font
/// load with no other live Rust borrow.
pub(crate) unsafe fn current_engine_state() -> *mut EngineState {
    ENGINE_STATE_PTR.with(|c| c.get())
}

pub(crate) struct HyphState {
    pub is_hyph: bool,
    pub hf: i32,
    pub max_hyph_char: i32,
    pub init_list: crate::units::MemPtr,
    pub init_lig: bool,
    pub init_lft: bool,
    pub hyphen_passed: i16,
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
    pub hyph_index: i32,
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
    pub trie_min: Vec<i32>,
    pub hyf_distance: Vec<i16>,
    pub hyf_num: Vec<i16>,
    pub hyf_next: Vec<u16>,
    pub op_start: Vec<i32>,
    pub trie_used: Vec<u16>,
    pub trie_op_lang: Vec<u8>,
    pub trie_op_val: Vec<u16>,
    pub trie_trl: Vec<i32>,
    pub trie_tro: Vec<i32>,
    pub trie_trc: Vec<u16>,
    pub trie_c: Vec<u16>,
    pub trie_o: Vec<u16>,
    pub trie_l: Vec<i32>,
    pub trie_r: Vec<i32>,
    pub trie_hash: Vec<i32>,
    pub trie_taken: Vec<bool>,
    pub hyph_word: Vec<i32>,
    pub hyph_list: Vec<i32>,
    pub hyph_link: Vec<u16>,
    pub trie_op_hash_array: Vec<i32>,
}

impl Default for HyphState {
    fn default() -> Self {
        Self {
            is_hyph: false,
            hf: 0,
            max_hyph_char: 0,
            init_list: crate::units::MemPtr::from_raw(0),
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
    pub(crate) fn get_current_name(&self) -> &std::ffi::CStr {
        // SAFETY: self.abspath_of_input_file.0 is a NUL-terminated buffer of length 1024.
        unsafe { std::ffi::CStr::from_ptr(self.abspath_of_input_file.0.as_ptr()) }
    }
}

impl MemState {
    #[inline]
    pub(crate) fn avail(&self) -> i32 {
        self.avail
    }
    #[inline]
    pub(crate) fn set_avail(&mut self, v: i32) {
        self.avail = v;
    }
}

impl ScannerState {
    #[inline]
    pub(crate) fn native_len(&self) -> i32 {
        self.native_len
    }
    #[inline]
    pub(crate) fn set_native_len(&mut self, v: i32) {
        self.native_len = v;
    }
    #[inline]
    pub(crate) fn native_text_size(&self) -> i32 {
        self.native_text_size
    }
    #[inline]
    pub(crate) fn save_native_len(&self) -> i32 {
        self.save_native_len
    }
    #[inline]
    pub(crate) fn set_save_native_len(&mut self, v: i32) {
        self.save_native_len = v;
    }
}

impl FontState {
    #[inline]
    pub(crate) fn mapped_text_entry(&self, i: i32) -> u16 {
        self.mapped_text[i as usize]
    }
}

impl ShipoutState {
    #[inline]
    pub(crate) fn insert_src_special_auto(&self) -> bool {
        self.insert_src_special_auto
    }
}

impl HyphState {
    #[inline]
    pub(crate) fn is_hyph(&self) -> bool {
        self.is_hyph
    }
    #[inline]
    pub(crate) fn set_is_hyph(&mut self, v: bool) {
        self.is_hyph = v;
    }
}
