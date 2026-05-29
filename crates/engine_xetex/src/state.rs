//! Rust-owned engine state during the Stage 2 idiomatization.

use std::cell::RefCell;

#[derive(Default)]
pub(crate) struct EngineState {
    pub config: ConfigState,
    pub errors: ErrorState,
    pub shipout: ShipoutState,
    pub scanner: ScannerState,
    pub save_stack: SaveStackState,
    pub eqtb: EqtbState,
    pub input: InputClusterState,
    pub strings: StringsState,
    pub hyph: HyphState,
    pub fonts: FontState,
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
    /// are `str_number` (newtype debts tracked in 01a).
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
    /// Rust-owned main input-file stack (`max_in_open + 1` handles, matching
    /// the former `xmalloc_array` shape). Allocated at engine init and released
    /// by `tt_cleanup`; never reallocated during a run (the pointer-escaping
    /// `input_file_ptr` accessor relies on this). Replaces the former
    /// C-`malloc`'d `input_file` pointer. Not serialized in format dump/undump.
    /// Element type is a `UFILE*` handle, stored as `*mut c_void` to avoid a
    /// cyclic dependency on `io.rs` — a plain pointer, no newtype debt.
    pub input_file: Vec<*mut libc::c_void>,
    /// Rust-owned semantic-nest (mode) stack (`nest_size + 1` records, matching
    /// the former `xmalloc_array` shape). Allocated at engine init and released
    /// by `tt_cleanup`; never reallocated during a run. Replaces the former
    /// C-`malloc`'d `nest` pointer. Not serialized in format dump/undump.
    /// Element type is the `ListStateRecord` struct (mirrors C
    /// `list_state_record`) — a compound struct, not a scalar typedef, no
    /// newtype debt.
    pub nest: Vec<crate::mem_types::ListStateRecord>,
    /// Count of open pseudo-file parentheses emitted to the transcript.
    /// Plain `int32_t`, no newtype debt.
    pub open_parens: i32,
    /// Current input-nesting depth (number of open input files). Plain `int32_t`.
    pub in_open: i32,
    /// Head of the pseudo-file linked list (xetex-xetexd.h:704). Plain `int32_t`.
    pub pseudo_files: i32,
    /// Current top of the semantic-nest stack. Plain `int32_t`.
    pub nest_ptr: i32,
    /// Peak value ever reached by `nest_ptr`. Plain `int32_t`.
    pub max_nest_stack: i32,
    /// Current top of the input-state stack. Plain `int32_t`.
    pub input_ptr: i32,
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
}

#[derive(Default)]
pub(crate) struct EqtbState {
    pub hash_used: i32,
    pub hash_high: i32,
    pub no_new_control_sequence: bool,
    pub prim_used: i32,
    pub cs_count: i32,
}

#[derive(Default)]
pub(crate) struct SaveStackState {
    pub save_ptr: i32,
    pub max_save_stack: i32,
    pub cur_level: u16,
    pub cur_group: u8,
    pub cur_boundary: i32,
    /// Rust-owned save-stack arena (`save_size + 1` words). Allocated at
    /// engine init and released by `tt_cleanup`; never reallocated during a
    /// run. Replaces the former C-`malloc`'d `save_stack` pointer.
    pub data: Vec<crate::mem_types::MemoryWord>,
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
    pub cur_align: i32,
    pub cur_span: i32,
    pub cur_loop: i32,
    pub align_ptr: i32,
    pub cur_head: i32,
    pub cur_tail: i32,
    pub cur_pre_head: i32,
    pub cur_pre_tail: i32,
    pub shown_mode: i16,
    pub mag_set: i32,
    pub radix: u8,
    pub expand_depth_count: i32,
    pub sa_chain: i32,
    pub sa_level: u16,
    pub max_reg_num: i32,
    /// Head of the conditional-stack list (xetex-xetexd.h:537). Plain `int32_t`.
    pub cond_ptr: i32,
    /// Current `\if` nesting limit code (xetex-xetexd.h:538). Stored as `u8`
    /// (`unsigned char` in C); plain, no semantic typedef debt.
    pub if_limit: u8,
    /// Current `\if` type (xetex-xetexd.h:539; C type `small_number` = `short`).
    /// Newtype debt: `SmallNumber` (tracked in 01a, same as `radix`).
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
    pub def_ref: i32,
    /// Macro call type (xetex-xetexd.h:528; `unsigned char`;
    /// CALL/LONG_CALL/OUTER_CALL/LONG_OUTER_CALL). Plain, no newtype debt.
    pub long_state: u8,
    /// Alignment brace-nesting counter (xetex-xetexd.h:520). Positive = extra
    /// open braces; negative = unmatched `&` or `\cr`. Plain `int32_t`.
    pub align_state: i32,
    /// Mark registers: `cur_mark[0..=4]` (xetex-xetexd.h:527). Each entry is
    /// a token-list pointer (`int32_t`). Inline fixed array; no alloc/cleanup
    /// edge. Plain `int32_t` elements, no newtype debt.
    pub cur_mark: [i32; 5],
    /// Saved discretionary break pointers (xetex-xetexd.h:716). Inline fixed
    /// array of 4 `int32_t` half-word pointers. No alloc/cleanup edge. Plain
    /// `int32_t` elements, no newtype debt.
    pub disc_ptr: [i32; 4],
}

#[derive(Default)]
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
    /// C type `font_index` (semantic typedef for `int32_t`; newtype debt tracked
    /// in 01a).
    pub fmem_ptr: i32,
    /// Highest font number loaded so far. C type `internal_font_number`
    /// (semantic typedef; newtype debt tracked in 01a).
    pub font_ptr: i32,
    /// Per-font renderer flags (`char *`, one byte per font slot). Not
    /// format-serialized; reallocated fresh at init / format-load.
    pub font_flags: Vec<i8>,
    /// Per-font letter-spacing value (`scaled_t *`). Not format-serialized;
    /// reallocated fresh at init / format-load. Newtype debt tracked in 01a.
    pub font_letter_space: Vec<i32>,
    pub null_character: crate::mem_types::B16x4,
    /// Rust-owned font checksum table (`font_max + 1` entries, matching the
    /// former `xmalloc_array` shape). Allocated at engine init / format load
    /// and released by `tt_cleanup`; serialized in the format dump.
    pub font_check: Vec<crate::mem_types::B16x4>,
    /// Rust-owned font size tables (`font_max + 1` scaled values each).
    /// Allocated at engine init / format load and released by `tt_cleanup`;
    /// serialized in the format dump. Element type is `scaled_t` (newtype debt
    /// tracked in 01a).
    pub font_size: Vec<i32>,
    pub font_dsize: Vec<i32>,
    /// Rust-owned font-parameter count table (`font_max + 1` entries).
    /// Serialized in the format dump. Element type is `font_index` (newtype
    /// debt tracked in 01a).
    pub font_params: Vec<i32>,
    /// Rust-owned font name/area tables (`font_max + 1` string-pool indexes).
    /// Serialized in the format dump. Element type is `str_number` (newtype
    /// debt tracked in 01a).
    pub font_name: Vec<i32>,
    pub font_area: Vec<i32>,
    /// Rust-owned first/last valid character-code tables (`font_max + 1`
    /// entries each). Serialized in the format dump. Element type is
    /// `UTF16_code` (newtype debt tracked in 01a).
    pub font_bc: Vec<u16>,
    pub font_ec: Vec<u16>,
    /// Rust-owned boundary-character tables (`font_max + 1` entries each).
    /// Serialized in the format dump. Element types are `font_index` /
    /// `nine_bits` (newtype debt tracked in 01a).
    pub bchar_label: Vec<i32>,
    pub font_bchar: Vec<i32>,
    pub font_false_bchar: Vec<i32>,
    /// Rust-owned per-font glue-spec pointer table (`font_max + 1` entries).
    /// Serialized in the format dump (checked undump against `lo_mem_max`).
    /// Element is a nullable memory/halfword pointer (newtype debt tracked in
    /// 01a).
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
        }
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
    pub page_tail: i32,
    pub last_badness: i32,
    pub page_contents: u8,
    pub page_so_far: [i32; 8],
    pub last_glue: i32,
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
    pub interaction: u8,
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
            interaction: 0,
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

thread_local! {
    static ENGINE_STATE: RefCell<EngineState> = RefCell::new(EngineState::default());
}

#[inline]
pub(crate) fn with_state<R>(f: impl FnOnce(&EngineState) -> R) -> R {
    ENGINE_STATE.with(|state| f(&state.borrow()))
}

#[inline]
pub(crate) fn with_state_mut<R>(f: impl FnOnce(&mut EngineState) -> R) -> R {
    ENGINE_STATE.with(|state| f(&mut state.borrow_mut()))
}

pub(crate) struct HyphState {
    pub is_hyph: bool,
    pub hf: i32,
    pub max_hyph_char: i32,
    pub init_list: i32,
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
}

impl Default for HyphState {
    fn default() -> Self {
        Self {
            is_hyph: false,
            hf: 0,
            max_hyph_char: 0,
            init_list: 0,
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
        }
    }
}
