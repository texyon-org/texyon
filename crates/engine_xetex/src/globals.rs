//! Centralized bindings to XeTeX's C globals.
//!
//! During Phase A of the C→Rust migration (see `plan/02-state.md`),
//! translated Rust functions read and write the engine's global state
//! through this module rather than declaring `extern "C"` statics at
//! their call sites. This keeps the `unsafe` blob small, ensures we
//! audit aliasing in one place, and gives us a trivial rename target
//! once state is threaded through `EngineState` in Phase B.
//!
//! ## Safety invariant for this entire module
//!
//! The tectonic engine holds a process-wide mutex (`bridge_core`'s
//! global engine lock) while a (Xe)TeX invocation is in flight, so
//! there are no concurrent readers or writers of any global declared
//! here. Every accessor therefore performs a single read-and-return
//! or write-and-commit: we **never** hand out a live `&` / `&mut`
//! reference that could overlap another call. In particular, do not
//! introduce a helper that returns `&'static mut` — two calls would
//! alias and that's insta-UB.
//!
//! ## Scope
//!
//! Only the globals that have actual Rust callers live here. Growing
//! this file on spec is busywork (there are ~150 globals in
//! `xetex-xetexd.h`). Add a declaration the first time a Rust function
//! needs it.

// ---------------------------------------------------------------------------
// extern bindings
// ---------------------------------------------------------------------------
//
// Raw extern statics are hidden in this private submodule so their names
// don't collide with the safe wrappers of the same name exported below.

/// Rust mirror of `input_state_t` (xetex-xetexd.h:344–352).
/// Must stay layout-compatible with the C struct (`#[repr(C)]`).
#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) struct InputState {
    pub state: u16,
    pub index: u16,
    pub start: i32,
    pub loc: i32,
    pub limit: i32,
    pub name: i32,
    pub synctex_tag: i32,
}

mod raw {
    use crate::mem_types::MemoryWord;

    extern "C" {
        // xetex-xetexd.h:452
        pub(super) static mut arith_error: bool;
        // xetex-xetexd.h:453 — `scaled_t` is `int32_t` (xetex-core.h:40).
        pub(super) static mut tex_remainder: i32;
        // xetex-xetexd.h:432 — digit scratch buffer for print_int / print_hex.
        pub(super) static mut dig: [u8; 23];

        // xetex-xetexd.h:454 — array of 55 pseudo-random fractions in
        // [0, 0x10000000).
        pub(super) static mut randoms: [i32; 55];
        // xetex-xetexd.h:455 — index into `randoms`; `unsigned char` maps to u8.
        pub(super) static mut j_random: u8;

        // xetex-xetexd.h:506 — current input state; layout mirrors InputState above.
        pub(super) static mut cur_input: super::InputState;
        // xetex-xetexd.h:508 — count of open pseudo-file parens.
        pub(super) static mut open_parens: i32;
        // xetex-xetexd.h:510 — current source line number.
        pub(super) static mut line: i32;

        // xetex-xetexd.h:386 — equivalents table (size eqtb_top+1 words).
        // Accessed by index like `mem`; valid indices depend on context.
        pub(super) static mut eqtb: *mut crate::mem_types::MemoryWord;

        // String pool (xetex-xetexd.h:424–426).
        // str_start[i] = pool offset of the i-th string (pool_pointer = i32).
        pub(super) static mut str_start: *mut i32;
        // str_ptr = number of strings in the pool (str_number = i32).
        pub(super) static mut str_ptr: i32;
        // xetex-xetexd.h:459 — temporary pointer register.
        pub(super) static mut temp_ptr: i32;

        // Memory arena globals (xetex-xetexd.h:460–466).
        // Allocated by C (`xmalloc_array`) during engine init; valid indices
        // are 0 .. =MEM_TOP (4999999). TEX_NULL (-268435455) is never used
        // as an index — it is a sentinel that callers check before indexing.
        pub(super) static mut mem: *mut MemoryWord;
        pub(super) static mut lo_mem_max: i32;
        pub(super) static mut hi_mem_min: i32;
        pub(super) static mut avail: i32;
        pub(super) static mut mem_end: i32;
        pub(super) static mut rover: i32;

        // Output selector (xetex-xetexd.h:431). C type `selector_t` is an
        // int-sized enum; values range 0–21 (see xetex-core.h).
        pub(super) static mut selector: i32;
        // True while processing a \special (xetex-xetexd.h:439).
        pub(super) static mut doing_special: bool;
        // Print position counters (xetex-xetexd.h:434–435).
        pub(super) static mut term_offset: i32;
        pub(super) static mut file_offset: i32;

        // Output file handles (xetex-xetexd.h:429–430, 691).
        // `rust_output_handle_t` is `uintptr_t` (tectonic_bridge_core_generated.h:9).
        pub(super) static mut rust_stdout: usize;
        pub(super) static mut log_file: usize;
        pub(super) static mut write_file: [usize; 16];

        // Current diagnostic capture handle (xetex-output.c, un-static'd session 8).
        // Null (0) means no active capture. Type is opaque `ttbc_diagnostic_t *`.
        pub(super) static mut current_diagnostic: *mut std::ffi::c_void;

        // Pseudo-file list head (xetex-xetexd.h:704; used by pseudo_input/close).
        pub(super) static mut pseudo_files: i32;
        // Input-level depth (xetex-xetexd.h:507).
        pub(super) static mut in_open: i32;
        // Pointer to the filename-stack array (xetex-xetexd.h:513, str_number* = *mut i32).
        pub(super) static mut full_source_filename_stack: *mut i32;
        // Pointer to the line-number-stack array (xetex-xetexd.h:511, *mut i32).
        pub(super) static mut line_stack: *mut i32;
        // Non-zero when file:line error format is active (xetex-xetexd.h:416).
        pub(super) static mut file_line_error_style_p: i32;

        // Hash table: `b32x2 *hash` (xetex-xetexd.h:482).
        pub(super) static mut hash: *mut crate::mem_types::B32x2;
        // Hash table management globals (xetex-xetexd.h:483–488).
        pub(super) static mut hash_used: i32;
        pub(super) static mut hash_extra: i32;
        pub(super) static mut hash_high: i32;
        pub(super) static mut no_new_control_sequence: bool;
        // Primitive hash table: `b32x2 prim[PRIM_SIZE + 1]` (xetex-xetexd.h:490).
        pub(super) static mut prim: [crate::mem_types::B32x2; 2101];
        // prim_used: next free slot in prim (xetex-xetexd.h:491).
        pub(super) static mut prim_used: i32;
        // Top of the eqtb array (xetex-xetexd.h:486).
        pub(super) static mut eqtb_top: i32;

        // String pool size limits (xetex-xetexd.h:400,427–428).
        pub(super) static mut max_strings: i32;
        pub(super) static mut init_str_ptr: i32;
        pub(super) static mut init_pool_ptr: i32;
        // Font memory capacity (xetex-xetexd.h:405).
        pub(super) static mut font_mem_size: i32;

        // Input buffer (xetex-xetexd.h:392); UnicodeScalar = int32_t.
        pub(super) static mut buffer: *mut i32;

        // Error-help machinery (xetex-xetexd.h:449–450).
        pub(super) static mut help_line: [*const libc::c_char; 6];
        pub(super) static mut help_ptr: u8;

        // Scaled-math tables (xetex-xetexd.h:457–458).
        pub(super) static mut two_to_the: [i32; 31];
        pub(super) static mut spec_log: [i32; 29];

        // Error / interaction state (xetex-xetexd.h:444–452, 551–552).
        // `interaction` is `unsigned char`; mode values: BATCH=0,SCROLL=2,ERROR_STOP=3.
        pub(super) static mut interaction: u8;
        // `history` is `tt_history_t` (int-sized enum, values 0–3).
        pub(super) static mut history: i32;
        pub(super) static mut error_count: i8;
        pub(super) static mut use_err_help: bool;
        // True after \errmessage when interaction < errorstop (xetex-xetexd.h:689).
        pub(super) static mut long_help_seen: bool;
        pub(super) static mut halt_on_error_p: i32;
        // `log_opened`, `job_name`, `texmf_log_name` (xetex-xetexd.h:551–554).
        pub(super) static mut log_opened: bool;
        pub(super) static mut job_name: i32;
        pub(super) static mut texmf_log_name: i32;

        // Engine config flags set before launch (xetex-xetexd.h:385,396,722,724).
        pub(super) static mut in_initex_mode: bool;
        pub(super) static mut synctex_enabled: i32;
        pub(super) static mut semantic_pagination_enabled: bool;
        pub(super) static mut shell_escape_enabled: bool;

        // Print line-length limit (xetex-xetexd.h:399).
        pub(super) static mut max_print_line: i32;
        // Tally of characters output (xetex-xetexd.h:433).
        pub(super) static mut tally: i32;
        // Error-line display helpers (xetex-xetexd.h:397, 436–437).
        pub(super) static mut error_line: i32;
        pub(super) static mut trick_buf: [u16; 256];
        pub(super) static mut trick_count: i32;
        // String pool (xetex-xetexd.h:403, 423, 425).
        // `packed_UTF16_code` = `unsigned short`; `pool_pointer` = `int32_t`.
        pub(super) static mut pool_size: i32;
        pub(super) static mut str_pool: *mut u16;
        pub(super) static mut pool_ptr: i32;

        // Name-of-file globals (xetex-xetexd.h:388–391).
        // name_of_file: current input filename (NUL-terminated C string).
        pub(super) static mut name_of_file: *mut libc::c_char;
        // name_length: byte length of name_of_file (excluding NUL).
        pub(super) static mut name_length: i32;
        // name_of_file16: UTF-16 encoding of name_of_file (xetex-xetexd.h:389).
        pub(super) static mut name_of_file16: *mut u16;
        // name_length16: number of UTF-16 code units in name_of_file16.
        pub(super) static mut name_length16: i32;

        // Saved selector value used by begin_diagnostic/end_diagnostic (xetex-xetexd.h:481).
        pub(super) static mut old_setting: u8;
        // SyncTeX timing base (xetex-xetexd.h:601–602).
        pub(super) static mut epochseconds: i32;
        pub(super) static mut microseconds: i32;
        // Font name table (xetex-xetexd.h:562; str_number = i32).
        pub(super) static mut font_name: *mut i32;
        // Font memory tables (xetex-xetexd.h:555–590).
        pub(super) static mut font_info: *mut MemoryWord;
        pub(super) static mut fmem_ptr: i32;
        pub(super) static mut font_ptr: i32;
        pub(super) static mut font_params: *mut i32;
        pub(super) static mut font_glue: *mut i32;
        pub(super) static mut param_base: *mut i32;
        // Current control-sequence index (xetex-xetexd.h:501; 0 = not a cs).
        pub(super) static mut cur_cs: i32;
        // Current token (xetex-xetexd.h:502; token encoded as int32).
        pub(super) static mut cur_tok: i32;
        // Alignment state (xetex-xetexd.h:520; tracks brace nesting in alignment).
        pub(super) static mut align_state: i32;
        // Current character code (xetex-xetexd.h:500).
        pub(super) static mut cur_chr: i32;
        // Current command code (`eight_bits` = u8, xetex-xetexd.h:499).
        pub(super) static mut cur_cmd: u8;
        // Mark registers: cur_mark[0..4] (xetex-xetexd.h:527; str_number = i32).
        pub(super) static mut cur_mark: [i32; 5];
        // Current parsed integer value (xetex-xetexd.h:530).
        pub(super) static mut cur_val: i32;
        // Parsed file-name components (xetex-xetexd.h:542–544; str_number = i32).
        pub(super) static mut cur_name: i32;
        pub(super) static mut cur_area: i32;
        pub(super) static mut cur_ext: i32;
        // True while a filename is being accumulated (xetex-xetexd.h:550).
        pub(super) static mut name_in_progress: bool;
        // File-name parsing scratch (xetex-xetexd.h:545–547).
        // area_delimiter: pool offset of last dir-sep in current name.
        pub(super) static mut area_delimiter: i32;
        // ext_delimiter: pool offset of last '.' in current name.
        pub(super) static mut ext_delimiter: i32;
        // quoted_filename: true if name was inside quotes.
        pub(super) static mut quoted_filename: bool;
        // file_name_quote_char: quote character currently active (0 = none).
        pub(super) static mut file_name_quote_char: u16;
        // Controls whether more_name stops at spaces (xetex-xetexd.h:718).
        pub(super) static mut stop_at_space: bool;
        // Magnification as actually set this job (xetex-xetexd.h:498; 0 = not set yet).
        pub(super) static mut mag_set: i32;

        // Maximum font id (xetex-xetexd.h:406).
        pub(super) static mut font_max: i32;

        // Input state stack (xetex-xetexd.h:503–504; layout mirrors InputState).
        pub(super) static mut input_stack: *mut super::InputState;
        pub(super) static mut input_ptr: i32;
        // Maximum input_ptr ever reached (xetex-xetexd.h:505).
        pub(super) static mut max_in_stack: i32;
        // Capacity of input_stack (xetex-xetexd.h:410).
        pub(super) static mut stack_size: i32;
        // Base pointer into input_stack (xetex-xetexd.h:521).
        pub(super) static mut base_ptr: i32;
        // Hash-table location of \par control sequence (xetex-xetexd.h:522).
        pub(super) static mut par_loc: i32;
        // Token form of \par (xetex-xetexd.h:523; cs_token = CS_TOKEN_FLAG + par_loc).
        pub(super) static mut par_token: i32;

        // Macro parameter stack (xetex-xetexd.h:517–519).
        pub(super) static mut param_stack: *mut i32;
        pub(super) static mut param_ptr: i32;
        pub(super) static mut max_param_stack: i32;

        // SA trie cursor — scratch pointer used during SA node traversal/allocation
        // (xetex-xetexd.h:710; xetex-ini.c:336).
        pub(super) static mut cur_ptr: i32;
        // SA null sentinel: a MemoryWord with both halves == TEX_NULL (xetex-ini.c:3203).
        pub(super) static mut sa_null: MemoryWord;

        // Sparse-array (SA) root table and saved-state chain (xetex-xetexd.h:709–713).
        // sa_root[i] is the root of the trie for register class i (0..=7).
        pub(super) static mut sa_root: [i32; 8];
        pub(super) static mut sa_chain: i32;
        pub(super) static mut sa_level: u16;

        // Register-size limits (xetex-xetexd.h:707–708).
        pub(super) static mut max_reg_num: i32;
        pub(super) static mut max_reg_help_line: *const libc::c_char;

        // Group-stack: grp_stack[i] = save-stack boundary for level i
        // (xetex-xetexd.h:705; save_pointer = i32).
        pub(super) static mut grp_stack: *mut i32;
        // If-stack: if_stack[i] = cond_ptr value at nesting level i
        // (xetex-xetexd.h:706).
        pub(super) static mut if_stack: *mut i32;

        // Conditional state (xetex-xetexd.h:537–540).
        pub(super) static mut cond_ptr: i32;
        // if_limit: unsigned char (xetex-xetexd.h:538).
        pub(super) static mut if_limit: u8;
        // cur_if: small_number = i16 (xetex-xetexd.h:539).
        pub(super) static mut cur_if: i16;
        pub(super) static mut if_line: i32;
        // xetex-xetexd.h:541 — line number saved when entering \if-skip block.
        pub(super) static mut skip_line: i32;

        // Current display mode for show_cur_cmd_chr (xetex-xetexd.h:480; short = i16).
        pub(super) static mut shown_mode: i16;

        // Semantic nest (mode stack, xetex-xetexd.h:476–479,413).
        pub(super) static mut nest: *mut crate::mem_types::ListStateRecord;
        pub(super) static mut nest_ptr: i32;
        pub(super) static mut max_nest_stack: i32;
        pub(super) static mut nest_size: i32;
        // Current list state (xetex-xetexd.h:479; direct struct, not a pointer).
        pub(super) static mut cur_list: crate::mem_types::ListStateRecord;
        // File-open states for \read streams (xetex-xetexd.h:536; CLOSED=2, JUST_OPEN=1).
        pub(super) static mut read_open: [u8; 17];
        // Array of UFILE* for \read streams (xetex-xetexd.h:535).
        // Declared as *mut c_void to avoid a cyclic type dependency with io.rs.
        pub(super) static mut read_file: [*mut libc::c_void; 16];

        // Save-stack (xetex-xetexd.h:492–494).
        pub(super) static mut save_stack: *mut crate::mem_types::MemoryWord;
        pub(super) static mut save_ptr: i32;
        pub(super) static mut max_save_stack: i32;
        pub(super) static mut save_size: i32;
        // Grouping level / group-code (xetex-xetexd.h:495–496).
        pub(super) static mut cur_level: u16;
        // group_code is unsigned char (xetex-xetexd.h:325).
        pub(super) static mut cur_group: u8;
        // Current group boundary in the save stack (xetex-xetexd.h:497).
        pub(super) static mut cur_boundary: i32;
        // XEQ_LEVEL array — level at which each eqtb entry was defined
        // (xetex-xetexd.h:733; size = EQTB_SIZE - INT_BASE + 1 = 1114730).
        pub(super) static mut _xeq_level_array: [u16; 1_114_730];

        // Expression-depth guard (xetex-xetexd.h:415, 525).
        pub(super) static mut expand_depth: i32;
        pub(super) static mut expand_depth_count: i32;
        // Value-type level for the current scanned quantity (xetex-xetexd.h:532).
        // 0=INT_VAL, 1=DIMEN_VAL, 2=GLUE_VAL, 3=MU_VAL (unsigned char).
        pub(super) static mut cur_val_level: u8;

        // Scanner state tracking (xetex-xetexd.h:514–516).
        // scanner_status: unsigned char (NORMAL=0 .. ABSORBING=5).
        pub(super) static mut scanner_status: u8;
        // warning_index: control-sequence index for error messages.
        pub(super) static mut warning_index: i32;
        // def_ref: head of the current macro-definition token list.
        pub(super) static mut def_ref: i32;

        // Input buffer: first = next free slot; buf_size = capacity
        // last = end-of-current-line; max_buf_stack = high-water mark
        // (xetex-xetexd.h:393–395, 409).
        pub(super) static mut first: i32;
        pub(super) static mut last: i32;
        pub(super) static mut max_buf_stack: i32;
        pub(super) static mut buf_size: i32;
        // Maximum allowed input-nesting depth (xetex-xetexd.h:411).
        pub(super) static mut max_in_open: i32;
        // Source-filename stacks (xetex-xetexd.h:512; str_number = i32).
        pub(super) static mut source_filename_stack: *mut i32;
        // eof_seen[k]: true when EOF was seen at level k (xetex-xetexd.h:700).
        pub(super) static mut eof_seen: *mut bool;
        // Array of UFILE* for the main input-file stack (xetex-xetexd.h:509).
        // Typed as *mut c_void to avoid a cyclic dependency on io.rs.
        pub(super) static mut input_file: *mut *mut libc::c_void;

        // is_in_csname: true while scanning between \csname...\endcsname (xetex-xetexd.h:526).
        pub(super) static mut is_in_csname: bool;

        // Second parsed value register used by scan_something_internal (xetex-xetexd.h:531).
        pub(super) static mut cur_val1: i32;
        // Scanner radix: 8, 10, or 16 (xetex-xetexd.h:533; small_number = u8).
        pub(super) static mut radix: u8;

        // Page builder state (xetex-xetexd.h:597, 604, 664–671).
        pub(super) static mut dead_cycles: i32;
        pub(super) static mut page_tail: i32;
        pub(super) static mut last_badness: i32;
        pub(super) static mut page_contents: u8;
        pub(super) static mut page_so_far: [i32; 8];
        pub(super) static mut last_glue: i32;
        pub(super) static mut last_penalty: i32;
        // disc_ptr[4]: saved discretionary break pointers (xetex-xetexd.h:716).
        pub(super) static mut disc_ptr: [i32; 4];
        pub(super) static mut last_kern: i32;
        pub(super) static mut last_node_type: i32;
        pub(super) static mut insert_penalties: i32;
        pub(super) static mut output_active: bool;

        // Random-number seed (xetex-xetexd.h:456; scaled_t = i32).
        pub(super) static mut random_seed: i32;

        // Positions of last PDF output points (xetex-xetexd.h:698–699).
        pub(super) static mut pdf_last_x_pos: i32;
        pub(super) static mut pdf_last_y_pos: i32;

        // Per-font encoding/layout arrays (xetex-xetexd.h:563–573).
        // font_area[f]: encoded font area (str_number = i32).
        pub(super) static mut font_area: *mut i32;
        // font_size[f], font_dsize[f]: current/default design size (scaled_t = i32).
        pub(super) static mut font_size: *mut i32;
        pub(super) static mut font_dsize: *mut i32;
        // font_bc[f], font_ec[f]: first/last valid char (UTF16_code = u16).
        pub(super) static mut font_bc: *mut u16;
        pub(super) static mut font_ec: *mut u16;
        // font_layout_engine[f]: pointer to AAT/OT layout engine (void*).
        pub(super) static mut font_layout_engine: *mut *mut libc::c_void;
        // font_mapping[f]: TECkit/TFM font mapping pointer (void*, xetex-xetexd.h:574).
        pub(super) static mut font_mapping: *mut *mut libc::c_void;
        // hyphen_char[f], skew_char[f] (xetex-xetexd.h:568–569).
        pub(super) static mut hyphen_char: *mut i32;
        pub(super) static mut skew_char: *mut i32;
        // xtx_ligature_present: set true when a XeTeX ligature was just applied (xetex-xetexd.h:720).
        pub(super) static mut xtx_ligature_present: bool;

        // Per-font index bases into font_info (xetex-xetexd.h:582–586).
        pub(super) static mut char_base: *mut i32;
        pub(super) static mut width_base: *mut i32;
        pub(super) static mut height_base: *mut i32;
        pub(super) static mut depth_base: *mut i32;
        pub(super) static mut italic_base: *mut i32;

        // True after the first char_warning fontspec hint has been issued (xetex-xetexd.h:725).
        pub(super) static mut gave_char_warning_help: bool;
        // main_control scratch glue-spec and font-info index (xetex-xetexd.h:675–676).
        pub(super) static mut main_p: i32;
        pub(super) static mut main_k: i32;
        // Current hyphenation language (xetex-xetexd.h:623; unsigned char = u8).
        pub(super) static mut cur_lang: u8;
        // SyncTeX "insert special on every paragraph" flag (xetex-xetexd.h:420).
        pub(super) static mut insert_src_special_every_par: bool;
        // Currently assembled box (xetex-xetexd.h:687; node index = i32).
        pub(super) static mut cur_box: i32;
        // Current glue order (normal=0..filll=3; xetex-xetexd.h:534; glue_ord = u8).
        pub(super) static mut cur_order: u8;
        // Alignment stack and current-alignment state (xetex-xetexd.h:612–617).
        pub(super) static mut cur_align: i32;
        pub(super) static mut cur_span: i32;
        pub(super) static mut cur_loop: i32;
        pub(super) static mut align_ptr: i32;
        pub(super) static mut cur_head: i32;
        pub(super) static mut cur_tail: i32;
        pub(super) static mut cur_pre_head: i32;
        pub(super) static mut cur_pre_tail: i32;
        // hpack/vpackage adjust-list pointers and glue accumulators (xetex-xetexd.h:603–606).
        pub(super) static mut adjust_tail: i32;
        pub(super) static mut pre_adjust_tail: i32;
        pub(super) static mut total_stretch: [i32; 4];
        pub(super) static mut total_shrink: [i32; 4];
        // Line number at which the current box-packing started (xetex-xetexd.h:607).
        pub(super) static mut pack_begin_line: i32;
        // Whether \setbox is allowed in the current context (xetex-xetexd.h:446).
        pub(super) static mut set_box_allowed: bool;
        // Glue/kern/height accumulator for vert_break and page-builder (xetex-xetexd.h:619).
        // 1-indexed in C; element 0 is unused.
        pub(super) static mut active_width: [i32; 7];
        // Height + depth of the best vertical break found so far (xetex-xetexd.h:662).
        pub(super) static mut best_height_plus_depth: i32;
    }
}

// ---------------------------------------------------------------------------
// arith_error
// ---------------------------------------------------------------------------

/// Returns the current value of the engine's `arith_error` flag.
#[inline]
#[allow(dead_code)] // used in unit tests and by future Rust callers of
                    // `mult_and_add` / `xn_over_d`.
pub(crate) fn arith_error() -> bool {
    // SAFETY: see module-level doc.
    unsafe { raw::arith_error }
}

/// Sets the engine's `arith_error` flag.
#[inline]
pub(crate) fn set_arith_error(v: bool) {
    // SAFETY: see module-level doc.
    unsafe {
        raw::arith_error = v;
    }
}

// ---------------------------------------------------------------------------
// tex_remainder
// ---------------------------------------------------------------------------

/// Returns the `tex_remainder` output register of `x_over_n` /
/// `xn_over_d`.
#[inline]
#[allow(dead_code)] // used in unit tests and by future Rust callers.
pub(crate) fn tex_remainder() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::tex_remainder }
}

/// Stores a value into the `tex_remainder` register.
#[inline]
pub(crate) fn set_tex_remainder(v: i32) {
    // SAFETY: see module-level doc.
    unsafe {
        raw::tex_remainder = v;
    }
}

// ---------------------------------------------------------------------------
// randoms / j_random
// ---------------------------------------------------------------------------

/// Returns `randoms[k]`, the k-th element of the pseudo-random table.
///
/// Panics in debug mode if `k >= 55`.
#[inline]
pub(crate) fn random_entry(k: usize) -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::randoms[k] }
}

/// Sets `randoms[k]` to `v`.
#[inline]
pub(crate) fn set_random_entry(k: usize, v: i32) {
    // SAFETY: see module-level doc.
    unsafe {
        raw::randoms[k] = v;
    }
}

/// Returns the current `j_random` index.
#[inline]
pub(crate) fn j_random() -> u8 {
    // SAFETY: see module-level doc.
    unsafe { raw::j_random }
}

/// Sets `j_random`.
#[inline]
pub(crate) fn set_j_random(v: u8) {
    // SAFETY: see module-level doc.
    unsafe {
        raw::j_random = v;
    }
}

// ---------------------------------------------------------------------------
// mem arena word accessors
// ---------------------------------------------------------------------------

/// Read `mem[p].b32.s0` (the first 32-bit word / node-size / type field).
#[inline]
pub(crate) fn mem_b32s0(p: i32) -> i32 {
    // SAFETY: see module-level doc. Caller ensures p is in-bounds.
    unsafe { (*raw::mem.offset(p as isize)).b32.s0 }
}

/// Write `v` into `mem[p].b32.s0`.
#[inline]
pub(crate) fn set_mem_b32s0(p: i32, v: i32) {
    // SAFETY: see module-level doc. Caller ensures p is in-bounds.
    unsafe {
        (*raw::mem.offset(p as isize)).b32.s0 = v;
    }
}

/// Read `mem[p].b32.s1` (the `LLIST_link` / successor-pointer field).
///
/// # Safety
/// `p` must be a valid arena index (`0 ..= MEM_TOP`). TEX_NULL is never
/// a valid index; callers must guard against it before calling this.
#[inline]
pub(crate) fn mem_b32s1(p: i32) -> i32 {
    // SAFETY: see module-level doc. Caller ensures p is in-bounds.
    unsafe { (*raw::mem.offset(p as isize)).b32.s1 }
}

/// Write `v` into `mem[p].b32.s1`.
#[inline]
pub(crate) fn set_mem_b32s1(p: i32, v: i32) {
    // SAFETY: see module-level doc. Caller ensures p is in-bounds.
    unsafe {
        (*raw::mem.offset(p as isize)).b32.s1 = v;
    }
}

// ---------------------------------------------------------------------------
// lo_mem_max / hi_mem_min / avail / mem_end / rover
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn lo_mem_max() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::lo_mem_max }
}

#[inline]
#[allow(dead_code)]
pub(crate) fn set_lo_mem_max(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::lo_mem_max = v; }
}

#[inline]
pub(crate) fn hi_mem_min() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::hi_mem_min }
}

#[inline]
pub(crate) fn set_hi_mem_min(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::hi_mem_min = v; }
}

#[inline]
pub(crate) fn avail() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::avail }
}

#[inline]
pub(crate) fn set_avail(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::avail = v; }
}

#[inline]
pub(crate) fn mem_end() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::mem_end }
}

#[inline]
pub(crate) fn set_mem_end(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::mem_end = v; }
}

#[inline]
pub(crate) fn rover() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::rover }
}

#[inline]
pub(crate) fn set_rover(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::rover = v; }
}

// ---------------------------------------------------------------------------
// cur_input / line
// ---------------------------------------------------------------------------

/// Returns `cur_input.synctex_tag`.
#[inline]
pub(crate) fn cur_input_synctex_tag() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::cur_input.synctex_tag }
}

/// Returns a copy of the entire `cur_input` input-state struct.
/// Used to save the current state onto `input_stack`.
#[inline]
pub(crate) fn cur_input() -> InputState {
    // SAFETY: see module-level doc.  InputState is Copy.
    unsafe { raw::cur_input }
}
/// Writes `cur_input.loc = v`.
#[inline]
pub(crate) fn set_cur_input_loc(v: i32) {
    unsafe { raw::cur_input.loc = v; }
}
/// Writes `cur_input.start = v`.
#[inline]
pub(crate) fn set_cur_input_start(v: i32) {
    unsafe { raw::cur_input.start = v; }
}
/// Writes `cur_input.state = v`.
#[inline]
pub(crate) fn set_cur_input_state(v: u16) {
    unsafe { raw::cur_input.state = v; }
}
/// Writes `cur_input.index = v`.
#[inline]
pub(crate) fn set_cur_input_index(v: u16) {
    unsafe { raw::cur_input.index = v; }
}
/// Writes `cur_input.name = v`.
#[inline]
pub(crate) fn set_cur_input_name(v: i32) {
    unsafe { raw::cur_input.name = v; }
}
/// Writes `cur_input.synctex_tag = v`.
#[inline]
pub(crate) fn set_cur_input_synctex_tag(v: i32) {
    unsafe { raw::cur_input.synctex_tag = v; }
}
/// Replaces the entire `cur_input` struct (e.g. when restoring from input_stack).
#[inline]
pub(crate) fn set_cur_input(v: InputState) {
    unsafe { raw::cur_input = v; }
}

/// Returns the current source line number.
#[inline]
pub(crate) fn line() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::line }
}

/// Returns `open_parens` (count of open pseudo-file parentheses in the transcript).
#[inline]
pub(crate) fn open_parens() -> i32 {
    unsafe { raw::open_parens }
}
/// Sets `open_parens = v`.
#[inline]
pub(crate) fn set_open_parens(v: i32) {
    unsafe { raw::open_parens = v; }
}

// ---------------------------------------------------------------------------
// mem word-level accessors (b16 and gr fields)
// ---------------------------------------------------------------------------

// Layout (little-endian): one MemoryWord = 8 bytes.
//   b32.s0 == bytes [0,4): b16.s0 (bytes 0-1) | b16.s1 (bytes 2-3)
//   b32.s1 == bytes [4,8): b16.s2 (bytes 4-5) | b16.s3 (bytes 6-7)
//   gr           == all 8 bytes as f64

/// Read `mem[p].b16.s0` (low 16 bits of the first 32-bit half-word).
#[inline]
pub(crate) fn mem_b16s0(p: i32) -> u16 {
    // SAFETY: see module-level doc.
    unsafe { (*raw::mem.offset(p as isize)).b16.s0 }
}
/// Write `v` into `mem[p].b16.s0`.
#[inline]
pub(crate) fn set_mem_b16s0(p: i32, v: u16) {
    // SAFETY: see module-level doc.
    unsafe { (*raw::mem.offset(p as isize)).b16.s0 = v; }
}

/// Read `mem[p].b16.s1` (high 16 bits of the first 32-bit half-word).
#[inline]
pub(crate) fn mem_b16s1(p: i32) -> u16 {
    // SAFETY: see module-level doc.
    unsafe { (*raw::mem.offset(p as isize)).b16.s1 }
}
/// Write `v` into `mem[p].b16.s1`.
#[inline]
pub(crate) fn set_mem_b16s1(p: i32, v: u16) {
    // SAFETY: see module-level doc.
    unsafe { (*raw::mem.offset(p as isize)).b16.s1 = v; }
}
/// Read `mem[p].b16.s2` (low 16 bits of the second 32-bit half-word).
#[inline]
pub(crate) fn mem_b16s2(p: i32) -> u16 {
    unsafe { (*raw::mem.offset(p as isize)).b16.s2 }
}
/// Read `mem[p].b16.s3` (high 16 bits of the second 32-bit half-word).
#[inline]
pub(crate) fn mem_b16s3(p: i32) -> u16 {
    unsafe { (*raw::mem.offset(p as isize)).b16.s3 }
}
/// Write `v` into `mem[p].b16.s2`.
#[inline]
pub(crate) fn set_mem_b16s2(p: i32, v: u16) {
    // SAFETY: see module-level doc.
    unsafe { (*raw::mem.offset(p as isize)).b16.s2 = v; }
}
/// Write `v` into `mem[p].b16.s3`.
#[inline]
pub(crate) fn set_mem_b16s3(p: i32, v: u16) {
    // SAFETY: see module-level doc.
    unsafe { (*raw::mem.offset(p as isize)).b16.s3 = v; }
}

/// Read the `gr` (f64) field of `mem[p]`.
#[inline]
#[allow(dead_code)]
pub(crate) fn mem_gr(p: i32) -> f64 {
    // SAFETY: see module-level doc.
    unsafe { (*raw::mem.offset(p as isize)).gr }
}
/// Write the `gr` (f64) field of `mem[p]`.
#[inline]
pub(crate) fn set_mem_gr(p: i32, v: f64) {
    // SAFETY: see module-level doc.
    unsafe { (*raw::mem.offset(p as isize)).gr = v; }
}

/// Read the `ptr` (void*) field of `mem[p]`.
#[inline]
pub(crate) fn mem_ptr(p: i32) -> *mut std::ffi::c_void {
    // SAFETY: see module-level doc.
    unsafe { (*raw::mem.offset(p as isize)).ptr }
}
/// Returns `&mem[p]` as a raw pointer — for C functions that expect a
/// `memory_word*` node pointer (e.g. `real_get_native_italic_correction`).
/// Equivalent to the C macro `get_native_italic_correction(p)` = `&mem[p]`.
#[inline]
pub(crate) fn mem_node_addr(p: i32) -> *mut std::ffi::c_void {
    unsafe { raw::mem.offset(p as isize) as *mut std::ffi::c_void }
}
/// Write the `ptr` (void*) field of `mem[p]`.
#[inline]
pub(crate) fn set_mem_ptr(p: i32, v: *mut std::ffi::c_void) {
    // SAFETY: see module-level doc.
    unsafe { (*raw::mem.offset(p as isize)).ptr = v; }
}

/// Return a `*const u16` pointing to `mem[p]` — used for NATIVE_NODE_text.
#[inline]
pub(crate) fn mem_as_u16_ptr(p: i32) -> *const u16 {
    // SAFETY: see module-level doc. Caller ensures p + NATIVE_NODE_SIZE is a valid mem index.
    unsafe { raw::mem.offset(p as isize) as *const u16 }
}

/// Copy the full MemoryWord at `mem[p]`.
#[inline]
pub(crate) fn mem_word(p: i32) -> crate::mem_types::MemoryWord {
    // SAFETY: see module-level doc.
    unsafe { *raw::mem.offset(p as isize) }
}
/// Write a full MemoryWord to `mem[p]`.
#[inline]
pub(crate) fn set_mem_word(p: i32, w: crate::mem_types::MemoryWord) {
    // SAFETY: see module-level doc.
    unsafe { *raw::mem.offset(p as isize) = w; }
}

// ---------------------------------------------------------------------------
// eqtb accessor
// ---------------------------------------------------------------------------

/// Returns true if the `eqtb` pointer is null (engine not yet initialized).
#[inline]
pub(crate) fn eqtb_is_null() -> bool {
    unsafe { raw::eqtb.is_null() }
}

/// Copy the full MemoryWord at `eqtb[p]`.
#[inline]
pub(crate) fn eqtb_word(p: i32) -> crate::mem_types::MemoryWord {
    unsafe { *raw::eqtb.offset(p as isize) }
}
/// Write a full MemoryWord to `eqtb[p]`.
#[inline]
pub(crate) fn set_eqtb_word(p: i32, w: crate::mem_types::MemoryWord) {
    unsafe { *raw::eqtb.offset(p as isize) = w; }
}
/// Read `eqtb[p].b32.s1` (the "value" half of an equivalents entry).
#[inline]
pub(crate) fn eqtb_b32s1(p: i32) -> i32 {
    // SAFETY: see module-level doc. Caller ensures p is a valid eqtb index.
    unsafe { (*raw::eqtb.offset(p as isize)).b32.s1 }
}
/// Write `v` into `eqtb[p].b32.s1`.
#[inline]
pub(crate) fn set_eqtb_b32s1(p: i32, v: i32) {
    // SAFETY: see module-level doc. Caller ensures p is a valid eqtb index.
    unsafe { (*raw::eqtb.offset(p as isize)).b32.s1 = v; }
}
/// Read `eqtb[p].b16.s1` (the "type" half of an equivalents entry).
#[inline]
pub(crate) fn eqtb_b16s1(p: i32) -> u16 {
    unsafe { (*raw::eqtb.offset(p as isize)).b16.s1 }
}
/// Write `v` into `eqtb[p].b16.s1`.
#[inline]
pub(crate) fn set_eqtb_b16s1(p: i32, v: u16) {
    unsafe { (*raw::eqtb.offset(p as isize)).b16.s1 = v; }
}
/// Read `eqtb[p].b16.s0` (the "level" half of an equivalents entry).
#[inline]
pub(crate) fn eqtb_b16s0(p: i32) -> u16 {
    unsafe { (*raw::eqtb.offset(p as isize)).b16.s0 }
}
/// Write `v` into `eqtb[p].b16.s0`.
#[inline]
pub(crate) fn set_eqtb_b16s0(p: i32, v: u16) {
    unsafe { (*raw::eqtb.offset(p as isize)).b16.s0 = v; }
}

// ---------------------------------------------------------------------------
// str_ptr / str_start / str_pool (read accessor)
// ---------------------------------------------------------------------------

/// Returns `name_of_file` as a raw pointer (NUL-terminated C string).
#[inline]
pub(crate) fn name_of_file() -> *mut libc::c_char {
    unsafe { raw::name_of_file }
}
/// Write `name_of_file = v`.
#[inline]
pub(crate) fn set_name_of_file(v: *mut libc::c_char) {
    unsafe { raw::name_of_file = v; }
}
/// Write `name_length = v`.
#[inline]
pub(crate) fn set_name_length(v: i32) {
    unsafe { raw::name_length = v; }
}
/// Returns `name_of_file16` (UTF-16 name buffer).
#[inline]
pub(crate) fn name_of_file16() -> *mut u16 {
    unsafe { raw::name_of_file16 }
}
/// Write `name_of_file16 = v`.
#[inline]
pub(crate) fn set_name_of_file16(v: *mut u16) {
    unsafe { raw::name_of_file16 = v; }
}
/// Returns `name_length16`.
#[inline]
pub(crate) fn name_length16() -> i32 {
    unsafe { raw::name_length16 }
}
/// Write `name_length16 = v`.
#[inline]
pub(crate) fn set_name_length16(v: i32) {
    unsafe { raw::name_length16 = v; }
}
/// Returns `name_length`.
#[inline]
pub(crate) fn name_length() -> i32 {
    unsafe { raw::name_length }
}

/// Returns the current number of strings in the string pool.
#[inline]
pub(crate) fn str_ptr() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::str_ptr }
}

/// Returns `str_start[k]` — the pool offset of the k-th string.
#[inline]
pub(crate) fn str_start(k: i32) -> i32 {
    // SAFETY: see module-level doc. Caller ensures k is a valid string index.
    unsafe { *raw::str_start.offset(k as isize) }
}

/// Read `str_pool[k]` (a packed UTF-16 code unit).
#[inline]
pub(crate) fn str_pool_entry(k: i32) -> u16 {
    // SAFETY: see module-level doc. Caller ensures k is a valid pool index.
    unsafe { *raw::str_pool.offset(k as isize) }
}

// ---------------------------------------------------------------------------
// current_diagnostic setter
// ---------------------------------------------------------------------------

/// Sets `current_diagnostic` to `v` (pass null to clear).
#[inline]
pub(crate) fn set_current_diagnostic(v: *mut std::ffi::c_void) {
    // SAFETY: see module-level doc.
    unsafe { raw::current_diagnostic = v; }
}

// ---------------------------------------------------------------------------
// in_open / full_source_filename_stack / line_stack / file_line_error_style_p
// ---------------------------------------------------------------------------

/// Returns the current input nesting depth.
#[inline]
pub(crate) fn in_open() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::in_open }
}
/// Returns `pseudo_files` — head of the pseudo-file linked list (xetex-xetexd.h:704).
#[inline]
pub(crate) fn pseudo_files() -> i32 {
    unsafe { raw::pseudo_files }
}
/// Sets `pseudo_files`.
#[inline]
pub(crate) fn set_pseudo_files(v: i32) {
    unsafe { raw::pseudo_files = v; }
}

/// Returns `full_source_filename_stack[k]` (the str_number of the source file
/// at nesting level k).
#[inline]
pub(crate) fn full_source_filename_stack(k: i32) -> i32 {
    // SAFETY: see module-level doc. Caller ensures k <= in_open.
    unsafe { *raw::full_source_filename_stack.offset(k as isize) }
}

/// Returns `line_stack[k]` (the saved line number at nesting level k).
#[inline]
pub(crate) fn line_stack(k: i32) -> i32 {
    // SAFETY: see module-level doc. Caller ensures k <= in_open.
    unsafe { *raw::line_stack.offset(k as isize) }
}

/// Returns non-zero when file:line-style error messages are active.
#[inline]
pub(crate) fn file_line_error_style_p() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::file_line_error_style_p }
}

// ---------------------------------------------------------------------------
// hash / prim / eqtb_top
// ---------------------------------------------------------------------------

/// Read `hash[p].s0` (the "link" half of a hash-table entry).
#[inline]
pub(crate) fn hash_s0(p: i32) -> i32 {
    unsafe { (*raw::hash.offset(p as isize)).s0 }
}
/// Write `hash[p].s0 = v`.
#[inline]
pub(crate) fn set_hash_s0(p: i32, v: i32) {
    unsafe { (*raw::hash.offset(p as isize)).s0 = v; }
}
/// Read `hash[p].s1` (the "text" half of a hash-table entry).
#[inline]
pub(crate) fn hash_s1(p: i32) -> i32 {
    // SAFETY: see module-level doc. Caller ensures p is a valid hash index.
    unsafe { (*raw::hash.offset(p as isize)).s1 }
}
/// Write `hash[p].s1 = v`.
#[inline]
pub(crate) fn set_hash_s1(p: i32, v: i32) {
    unsafe { (*raw::hash.offset(p as isize)).s1 = v; }
}

/// Read `prim[p].s0` (the link half of a primitive-table entry).
#[inline]
pub(crate) fn prim_s0(p: i32) -> i32 {
    unsafe { raw::prim[p as usize].s0 }
}
/// Write `prim[p].s0 = v`.
#[inline]
pub(crate) fn set_prim_s0(p: i32, v: i32) {
    unsafe { raw::prim[p as usize].s0 = v; }
}
/// Read `prim[p].s1` (the "text" half of a primitive-table entry).
#[inline]
pub(crate) fn prim_s1(p: i32) -> i32 {
    // SAFETY: see module-level doc. Caller ensures p < prim.len().
    unsafe { raw::prim[p as usize].s1 }
}
/// Write `prim[p].s1 = v`.
#[inline]
pub(crate) fn set_prim_s1(p: i32, v: i32) {
    unsafe { raw::prim[p as usize].s1 = v; }
}

/// Returns `eqtb_top`.
#[inline]
pub(crate) fn eqtb_top() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::eqtb_top }
}

// ---------------------------------------------------------------------------
// hash management globals (hash_used / hash_extra / hash_high /
//                          no_new_control_sequence / prim_used)
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn hash_used() -> i32 {
    unsafe { raw::hash_used }
}
#[inline]
pub(crate) fn set_hash_used(v: i32) {
    unsafe { raw::hash_used = v; }
}
#[inline]
pub(crate) fn hash_extra() -> i32 {
    unsafe { raw::hash_extra }
}
#[inline]
pub(crate) fn hash_high() -> i32 {
    unsafe { raw::hash_high }
}
#[inline]
pub(crate) fn set_hash_high(v: i32) {
    unsafe { raw::hash_high = v; }
}
#[inline]
pub(crate) fn no_new_control_sequence() -> bool {
    unsafe { raw::no_new_control_sequence }
}
#[inline]
pub(crate) fn set_no_new_control_sequence(v: bool) {
    unsafe { raw::no_new_control_sequence = v; }
}
#[inline]
pub(crate) fn prim_used() -> i32 {
    unsafe { raw::prim_used }
}
#[inline]
pub(crate) fn set_prim_used(v: i32) {
    unsafe { raw::prim_used = v; }
}

// ---------------------------------------------------------------------------
// temp_ptr
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn temp_ptr() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::temp_ptr }
}
#[inline]
pub(crate) fn set_temp_ptr(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::temp_ptr = v; }
}

// ---------------------------------------------------------------------------
// dig
// ---------------------------------------------------------------------------

/// Read `dig[k]`, the k-th digit scratch byte (xetex-xetexd.h:432).
#[inline]
pub(crate) fn dig_entry(k: usize) -> u8 {
    // SAFETY: see module-level doc.
    unsafe { raw::dig[k] }
}

/// Write `v` into `dig[k]`.
#[inline]
pub(crate) fn set_dig_entry(k: usize, v: u8) {
    // SAFETY: see module-level doc.
    unsafe { raw::dig[k] = v; }
}

// ---------------------------------------------------------------------------
// selector / term_offset / file_offset
// ---------------------------------------------------------------------------

/// Returns the current output selector (xetex-xetexd.h:431).
#[inline]
pub(crate) fn selector() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::selector }
}
/// Write `selector = v`.
#[inline]
pub(crate) fn set_selector(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::selector = v; }
}

/// Returns true while the engine is processing a `\special` (xetex-xetexd.h:439).
#[inline]
pub(crate) fn doing_special() -> bool {
    // SAFETY: see module-level doc.
    unsafe { raw::doing_special }
}

/// Returns `term_offset` (columns written to terminal since last newline).
#[inline]
pub(crate) fn term_offset() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::term_offset }
}
/// Sets `term_offset`.
#[inline]
pub(crate) fn set_term_offset(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::term_offset = v; }
}

/// Returns `file_offset` (columns written to log file since last newline).
#[inline]
pub(crate) fn file_offset() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::file_offset }
}
/// Sets `file_offset`.
#[inline]
pub(crate) fn set_file_offset(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::file_offset = v; }
}

// ---------------------------------------------------------------------------
// rust_stdout / log_file / write_file / current_diagnostic
// ---------------------------------------------------------------------------

/// Returns the `rust_stdout` output handle.
#[inline]
pub(crate) fn rust_stdout() -> usize {
    // SAFETY: see module-level doc.
    unsafe { raw::rust_stdout }
}

/// Returns the `log_file` output handle.
#[inline]
pub(crate) fn log_file() -> usize {
    // SAFETY: see module-level doc.
    unsafe { raw::log_file }
}
/// Sets the `log_file` output handle.
#[inline]
pub(crate) fn set_log_file(v: usize) {
    // SAFETY: see module-level doc.
    unsafe { raw::log_file = v; }
}

/// Returns `write_file[i]`, the output handle for write-stream `i` (0–15).
#[inline]
pub(crate) fn write_file(i: usize) -> usize {
    // SAFETY: see module-level doc.
    unsafe { raw::write_file[i] }
}

/// Returns the current diagnostic capture pointer (null = no active capture).
#[inline]
pub(crate) fn current_diagnostic() -> *mut std::ffi::c_void {
    // SAFETY: see module-level doc.
    unsafe { raw::current_diagnostic }
}

// ---------------------------------------------------------------------------
// print_raw_char globals
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn max_print_line() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::max_print_line }
}

#[inline]
pub(crate) fn tally() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::tally }
}
#[inline]
pub(crate) fn set_tally(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::tally = v; }
}

#[inline]
pub(crate) fn error_line() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::error_line }
}

/// Read `trick_buf[k]`.
#[inline]
pub(crate) fn trick_count() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::trick_count }
}
/// Write `trick_buf[k] = v`.
#[inline]
pub(crate) fn set_trick_buf(k: usize, v: u16) {
    // SAFETY: see module-level doc.
    unsafe { raw::trick_buf[k] = v; }
}

#[inline]
pub(crate) fn pool_size() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::pool_size }
}
#[inline]
pub(crate) fn pool_ptr() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::pool_ptr }
}
#[inline]
pub(crate) fn set_pool_ptr(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::pool_ptr = v; }
}
/// Write `str_pool[k] = v`.
#[inline]
pub(crate) fn set_str_pool(k: i32, v: u16) {
    // SAFETY: see module-level doc.
    unsafe { *raw::str_pool.offset(k as isize) = v; }
}

// ---------------------------------------------------------------------------
// str_start write / str_ptr write / string pool limits
// ---------------------------------------------------------------------------

/// Write `str_start[k] = v`.
#[inline]
pub(crate) fn set_str_start(k: i32, v: i32) {
    // SAFETY: see module-level doc. Caller ensures k is a valid string index.
    unsafe { *raw::str_start.offset(k as isize) = v; }
}
/// Write `str_ptr = v`.
#[inline]
pub(crate) fn set_str_ptr(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::str_ptr = v; }
}
#[inline]
pub(crate) fn max_strings() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::max_strings }
}
#[inline]
pub(crate) fn init_str_ptr() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::init_str_ptr }
}
#[inline]
pub(crate) fn init_pool_ptr() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::init_pool_ptr }
}
/// Read `buffer[k]` (UnicodeScalar = i32).
#[inline]
pub(crate) fn buffer_entry(k: i32) -> i32 {
    // SAFETY: see module-level doc. Caller ensures k is a valid buffer index.
    unsafe { *raw::buffer.offset(k as isize) }
}

// ---------------------------------------------------------------------------
// help_ptr / help_line
// ---------------------------------------------------------------------------

#[inline]
#[allow(dead_code)] // getter defined for completeness; only setter used so far
pub(crate) fn help_ptr() -> u8 {
    // SAFETY: see module-level doc.
    unsafe { raw::help_ptr }
}
#[inline]
pub(crate) fn set_help_ptr(v: u8) {
    // SAFETY: see module-level doc.
    unsafe { raw::help_ptr = v; }
}
/// Read `help_line[k]` as a C string pointer.
#[inline]
pub(crate) fn help_line(k: usize) -> *const libc::c_char {
    // SAFETY: see module-level doc. k must be < 6.
    unsafe { raw::help_line[k] }
}
/// Write `help_line[k] = v` (a C string pointer; caller owns the lifetime).
#[inline]
pub(crate) fn set_help_line(k: usize, v: *const libc::c_char) {
    // SAFETY: see module-level doc. k must be < 6.
    unsafe { raw::help_line[k] = v; }
}

// ---------------------------------------------------------------------------
// two_to_the / spec_log (read-only tables from xetex-ini.c)
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn two_to_the(k: usize) -> i32 {
    // SAFETY: see module-level doc. k must be < 31.
    unsafe { raw::two_to_the[k] }
}
#[inline]
pub(crate) fn spec_log(k: usize) -> i32 {
    // SAFETY: see module-level doc. k must be < 29.
    unsafe { raw::spec_log[k] }
}

// ---------------------------------------------------------------------------
// interaction / history / error_count / use_err_help / halt_on_error_p
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn interaction() -> u8 {
    // SAFETY: see module-level doc.
    unsafe { raw::interaction }
}
#[inline]
pub(crate) fn set_interaction(v: u8) {
    // SAFETY: see module-level doc.
    unsafe { raw::interaction = v; }
}
#[inline]
pub(crate) fn history() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::history }
}
#[inline]
pub(crate) fn set_history(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::history = v; }
}
#[inline]
pub(crate) fn error_count() -> i8 {
    // SAFETY: see module-level doc.
    unsafe { raw::error_count }
}
#[inline]
pub(crate) fn set_error_count(v: i8) {
    // SAFETY: see module-level doc.
    unsafe { raw::error_count = v; }
}
#[inline]
pub(crate) fn use_err_help() -> bool {
    // SAFETY: see module-level doc.
    unsafe { raw::use_err_help }
}
#[inline]
pub(crate) fn set_use_err_help(v: bool) {
    unsafe { raw::use_err_help = v; }
}
#[inline]
pub(crate) fn long_help_seen() -> bool {
    unsafe { raw::long_help_seen }
}
#[inline]
pub(crate) fn set_long_help_seen(v: bool) {
    unsafe { raw::long_help_seen = v; }
}
#[inline]
pub(crate) fn halt_on_error_p() -> bool {
    // SAFETY: see module-level doc.
    unsafe { raw::halt_on_error_p != 0 }
}
#[inline]
pub(crate) fn log_opened() -> bool {
    // SAFETY: see module-level doc.
    unsafe { raw::log_opened }
}
#[inline]
pub(crate) fn set_log_opened(v: bool) {
    // SAFETY: see module-level doc.
    unsafe { raw::log_opened = v; }
}
#[inline]
pub(crate) fn job_name() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::job_name }
}
#[inline]
pub(crate) fn set_job_name(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::job_name = v; }
}
#[inline]
pub(crate) fn texmf_log_name() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::texmf_log_name }
}
#[inline]
pub(crate) fn set_texmf_log_name(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::texmf_log_name = v; }
}
#[inline]
pub(crate) fn set_halt_on_error_p(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::halt_on_error_p = v; }
}

// ---------------------------------------------------------------------------
// engine launch config flags (set by tt_xetex_set_int_variable)
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn set_in_initex_mode(v: bool) {
    // SAFETY: see module-level doc.
    unsafe { raw::in_initex_mode = v; }
}
#[inline]
pub(crate) fn set_synctex_enabled(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::synctex_enabled = v; }
}
#[inline]
pub(crate) fn set_semantic_pagination_enabled(v: bool) {
    // SAFETY: see module-level doc.
    unsafe { raw::semantic_pagination_enabled = v; }
}
#[inline]
pub(crate) fn shell_escape_enabled() -> bool {
    // SAFETY: see module-level doc.
    unsafe { raw::shell_escape_enabled }
}
#[inline]
pub(crate) fn set_shell_escape_enabled(v: bool) {
    // SAFETY: see module-level doc.
    unsafe { raw::shell_escape_enabled = v; }
}

// ---------------------------------------------------------------------------
// old_setting
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn old_setting() -> u8 {
    unsafe { raw::old_setting }
}
#[inline]
pub(crate) fn set_old_setting(v: u8) {
    unsafe { raw::old_setting = v; }
}

// ---------------------------------------------------------------------------
// cur_chr / cur_val / cur_name / cur_area / cur_ext
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn cur_cs() -> i32 {
    unsafe { raw::cur_cs }
}
#[inline]
pub(crate) fn set_cur_cs(v: i32) {
    unsafe { raw::cur_cs = v; }
}
#[inline]
pub(crate) fn cur_tok() -> i32 {
    unsafe { raw::cur_tok }
}
#[inline]
pub(crate) fn set_cur_tok(v: i32) {
    unsafe { raw::cur_tok = v; }
}
#[inline]
pub(crate) fn align_state() -> i32 {
    unsafe { raw::align_state }
}
#[inline]
pub(crate) fn set_align_state(v: i32) {
    unsafe { raw::align_state = v; }
}
#[inline]
pub(crate) fn cur_chr() -> i32 {
    unsafe { raw::cur_chr }
}
#[inline]
pub(crate) fn cur_val() -> i32 {
    unsafe { raw::cur_val }
}
#[inline]
pub(crate) fn set_cur_val(v: i32) {
    unsafe { raw::cur_val = v; }
}
#[inline]
pub(crate) fn cur_name() -> i32 {
    unsafe { raw::cur_name }
}
#[inline]
pub(crate) fn set_cur_name(v: i32) {
    unsafe { raw::cur_name = v; }
}
#[inline]
pub(crate) fn cur_area() -> i32 {
    unsafe { raw::cur_area }
}
#[inline]
pub(crate) fn set_cur_area(v: i32) {
    unsafe { raw::cur_area = v; }
}
#[inline]
pub(crate) fn cur_ext() -> i32 {
    unsafe { raw::cur_ext }
}
#[inline]
pub(crate) fn set_cur_ext(v: i32) {
    unsafe { raw::cur_ext = v; }
}

/// Returns `area_delimiter` (pool offset of last dir-sep in current filename).
#[inline]
pub(crate) fn area_delimiter() -> i32 {
    unsafe { raw::area_delimiter }
}
#[inline]
pub(crate) fn set_area_delimiter(v: i32) {
    unsafe { raw::area_delimiter = v; }
}
/// Returns `ext_delimiter` (pool offset of last '.' in current filename).
#[inline]
pub(crate) fn ext_delimiter() -> i32 {
    unsafe { raw::ext_delimiter }
}
#[inline]
pub(crate) fn set_ext_delimiter(v: i32) {
    unsafe { raw::ext_delimiter = v; }
}
#[inline]
pub(crate) fn quoted_filename() -> bool {
    unsafe { raw::quoted_filename }
}
#[inline]
pub(crate) fn set_quoted_filename(v: bool) {
    unsafe { raw::quoted_filename = v; }
}
#[inline]
pub(crate) fn file_name_quote_char() -> u16 {
    unsafe { raw::file_name_quote_char }
}
#[inline]
pub(crate) fn set_file_name_quote_char(v: u16) {
    unsafe { raw::file_name_quote_char = v; }
}

// ---------------------------------------------------------------------------
// name_in_progress / stop_at_space
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn name_in_progress() -> bool {
    unsafe { raw::name_in_progress }
}
#[inline]
pub(crate) fn set_name_in_progress(v: bool) {
    unsafe { raw::name_in_progress = v; }
}
#[inline]
pub(crate) fn stop_at_space() -> bool {
    unsafe { raw::stop_at_space }
}
#[inline]
pub(crate) fn set_stop_at_space(v: bool) {
    unsafe { raw::stop_at_space = v; }
}

// ---------------------------------------------------------------------------
// save_stack / save_ptr / cur_level / cur_group
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn save_ptr() -> i32 {
    unsafe { raw::save_ptr }
}
#[inline]
pub(crate) fn set_save_ptr(v: i32) {
    unsafe { raw::save_ptr = v; }
}
#[inline]
pub(crate) fn max_save_stack() -> i32 {
    unsafe { raw::max_save_stack }
}
#[inline]
pub(crate) fn set_max_save_stack(v: i32) {
    unsafe { raw::max_save_stack = v; }
}
#[inline]
pub(crate) fn save_size() -> i32 {
    unsafe { raw::save_size }
}
/// Read `save_stack[k].b32.s1`.
#[inline]
pub(crate) fn save_stack_b32s1(k: i32) -> i32 {
    unsafe { (*raw::save_stack.offset(k as isize)).b32.s1 }
}
/// Read `save_stack[k].b32.s0`.
#[inline]
pub(crate) fn save_stack_b32s0(k: i32) -> i32 {
    unsafe { (*raw::save_stack.offset(k as isize)).b32.s0 }
}
/// Write `save_stack[k].b32.s1 = v`.
#[inline]
pub(crate) fn set_save_stack_b32s1(k: i32, v: i32) {
    unsafe { (*raw::save_stack.offset(k as isize)).b32.s1 = v; }
}
/// Write `save_stack[k].b32.s0 = v`.
#[inline]
pub(crate) fn set_save_stack_b32s0(k: i32, v: i32) {
    unsafe { (*raw::save_stack.offset(k as isize)).b32.s0 = v; }
}
/// Read `save_stack[k].b16.s1`.
#[inline]
pub(crate) fn save_stack_b16s1(k: i32) -> u16 {
    unsafe { (*raw::save_stack.offset(k as isize)).b16.s1 }
}
/// Read `save_stack[k].b16.s0`.
#[inline]
pub(crate) fn save_stack_b16s0(k: i32) -> u16 {
    unsafe { (*raw::save_stack.offset(k as isize)).b16.s0 }
}
/// Write `save_stack[k].b16.s1 = v`.
#[inline]
pub(crate) fn set_save_stack_b16s1(k: i32, v: u16) {
    unsafe { (*raw::save_stack.offset(k as isize)).b16.s1 = v; }
}
/// Write `save_stack[k].b16.s0 = v`.
#[inline]
pub(crate) fn set_save_stack_b16s0(k: i32, v: u16) {
    unsafe { (*raw::save_stack.offset(k as isize)).b16.s0 = v; }
}
/// Copy the full MemoryWord at `save_stack[k]`.
#[inline]
pub(crate) fn save_stack_word(k: i32) -> crate::mem_types::MemoryWord {
    unsafe { *raw::save_stack.offset(k as isize) }
}
/// Copy a full MemoryWord to `save_stack[k]`.
#[inline]
pub(crate) fn set_save_stack_word(k: i32, w: crate::mem_types::MemoryWord) {
    unsafe { *raw::save_stack.offset(k as isize) = w; }
}
#[inline]
pub(crate) fn cur_level() -> u16 {
    unsafe { raw::cur_level }
}
#[inline]
pub(crate) fn set_cur_level(v: u16) {
    unsafe { raw::cur_level = v; }
}
#[inline]
pub(crate) fn cur_group() -> u8 {
    unsafe { raw::cur_group }
}
#[inline]
pub(crate) fn set_cur_group(v: u8) {
    unsafe { raw::cur_group = v; }
}
#[inline]
pub(crate) fn cur_boundary() -> i32 {
    unsafe { raw::cur_boundary }
}
#[inline]
pub(crate) fn set_cur_boundary(v: i32) {
    unsafe { raw::cur_boundary = v; }
}
/// Read `XEQ_LEVEL(p)` = `_xeq_level_array[p - INT_BASE]`.
#[inline]
pub(crate) fn xeq_level(p: i32) -> u16 {
    // SAFETY: caller ensures p >= INT_BASE and p - INT_BASE < 1_114_730.
    unsafe { raw::_xeq_level_array[(p - 7_826_729) as usize] }
}
/// Write `XEQ_LEVEL(p) = v`.
#[inline]
pub(crate) fn set_xeq_level(p: i32, v: u16) {
    unsafe { raw::_xeq_level_array[(p - 7_826_729) as usize] = v; }
}

// ---------------------------------------------------------------------------
// read_open / read_file (for \read stream management)
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn read_open_entry(n: usize) -> u8 {
    unsafe { raw::read_open[n] }
}
#[inline]
pub(crate) fn set_read_open_entry(n: usize, v: u8) {
    unsafe { raw::read_open[n] = v; }
}
/// Returns `read_file[n]` (the UFILE* value at index n).
#[inline]
pub(crate) fn read_file_entry(n: usize) -> *mut crate::io::UFile {
    unsafe { raw::read_file[n] as *mut crate::io::UFile }
}
/// Returns a pointer to `read_file[n]`, for passing to `u_open_in`.
#[inline]
pub(crate) fn read_file_ptr(n: usize) -> *mut *mut crate::io::UFile {
    unsafe { raw::read_file.as_mut_ptr().add(n) as *mut *mut crate::io::UFile }
}

// ---------------------------------------------------------------------------
// name_of_file16_entry — indexed read of the UTF-16 name buffer
// ---------------------------------------------------------------------------

/// Returns `name_of_file16[k]`.
#[inline]
pub(crate) fn name_of_file16_entry(k: i32) -> u16 {
    unsafe { *raw::name_of_file16.add(k as usize) }
}

// ---------------------------------------------------------------------------
// font_name
// ---------------------------------------------------------------------------

/// Returns `font_name[f]` — the string-pool index for font f's name.
#[inline]
pub(crate) fn font_name_entry(f: i32) -> i32 {
    unsafe { *raw::font_name.offset(f as isize) }
}

/// Returns `font_info[k].b32.s1`.
#[inline]
pub(crate) fn font_info_b32s1(k: i32) -> i32 {
    unsafe { (*raw::font_info.offset(k as isize)).b32.s1 }
}

/// Writes `font_info[k].b32.s1 = v`.
#[inline]
pub(crate) fn set_font_info_b32s1(k: i32, v: i32) {
    unsafe { (*raw::font_info.offset(k as isize)).b32.s1 = v; }
}

/// Returns `fmem_ptr`.
#[inline]
pub(crate) fn fmem_ptr() -> i32 {
    unsafe { raw::fmem_ptr }
}

/// Sets `fmem_ptr`.
#[inline]
pub(crate) fn set_fmem_ptr(v: i32) {
    unsafe { raw::fmem_ptr = v; }
}

/// Returns `font_ptr`.
#[inline]
pub(crate) fn font_ptr() -> i32 {
    unsafe { raw::font_ptr }
}

/// Returns `font_mem_size`.
#[inline]
pub(crate) fn font_mem_size() -> i32 {
    unsafe { raw::font_mem_size }
}

/// Returns `font_params[f]`.
#[inline]
pub(crate) fn font_params(f: i32) -> i32 {
    unsafe { *raw::font_params.offset(f as isize) }
}

/// Sets `font_params[f] = v`.
#[inline]
pub(crate) fn set_font_params(f: i32, v: i32) {
    unsafe { *raw::font_params.offset(f as isize) = v; }
}

/// Returns `font_glue[f]`.
#[inline]
pub(crate) fn font_glue(f: i32) -> i32 {
    unsafe { *raw::font_glue.offset(f as isize) }
}

/// Sets `font_glue[f] = v`.
#[inline]
pub(crate) fn set_font_glue(f: i32, v: i32) {
    unsafe { *raw::font_glue.offset(f as isize) = v; }
}

/// Returns `param_base[f]`.
#[inline]
pub(crate) fn param_base(f: i32) -> i32 {
    unsafe { *raw::param_base.offset(f as isize) }
}

/// Returns `font_info[k].b16` (the B16x4 view of font_info word k).
#[inline]
pub(crate) fn font_info_b16(k: i32) -> crate::mem_types::B16x4 {
    unsafe { (*raw::font_info.offset(k as isize)).b16 }
}

/// Returns `font_area[f]` — the str_number encoding the font's area.
#[inline]
pub(crate) fn font_area_entry(f: i32) -> i32 {
    unsafe { *raw::font_area.offset(f as isize) }
}
/// Returns `font_size[f]` — the current at-size (scaled_t) for font f.
#[inline]
pub(crate) fn font_size_entry(f: i32) -> i32 {
    unsafe { *raw::font_size.offset(f as isize) }
}
/// Returns `font_dsize[f]` — the design-size (scaled_t) for font f.
#[inline]
pub(crate) fn font_dsize_entry(f: i32) -> i32 {
    unsafe { *raw::font_dsize.offset(f as isize) }
}
/// Returns `font_bc[f]` — first valid char code.
#[inline]
pub(crate) fn font_bc_entry(f: i32) -> u16 {
    unsafe { *raw::font_bc.offset(f as isize) }
}
/// Returns `font_ec[f]` — last valid char code.
#[inline]
pub(crate) fn font_ec_entry(f: i32) -> u16 {
    unsafe { *raw::font_ec.offset(f as isize) }
}
/// Returns `font_layout_engine[f]`.
#[inline]
pub(crate) fn font_layout_engine_ptr(f: i32) -> *mut libc::c_void {
    unsafe { *raw::font_layout_engine.offset(f as isize) }
}
/// Returns `font_mapping[f]`.
#[inline]
pub(crate) fn font_mapping_ptr(f: i32) -> *mut libc::c_void {
    unsafe { *raw::font_mapping.offset(f as isize) }
}
/// Returns `xtx_ligature_present`.
#[inline]
pub(crate) fn xtx_ligature_present() -> bool { unsafe { raw::xtx_ligature_present } }
/// Sets `xtx_ligature_present = v`.
#[inline]
pub(crate) fn set_xtx_ligature_present(v: bool) { unsafe { raw::xtx_ligature_present = v; } }
/// Returns `hyphen_char[f]`.
#[inline]
pub(crate) fn hyphen_char_entry(f: i32) -> i32 {
    unsafe { *raw::hyphen_char.offset(f as isize) }
}
/// Returns `skew_char[f]`.
#[inline]
pub(crate) fn skew_char_entry(f: i32) -> i32 {
    unsafe { *raw::skew_char.offset(f as isize) }
}
/// Returns `char_base[f]`.
#[inline]
pub(crate) fn char_base_entry(f: i32) -> i32 {
    unsafe { *raw::char_base.offset(f as isize) }
}
/// Returns `width_base[f]`.
#[inline]
pub(crate) fn width_base_entry(f: i32) -> i32 {
    unsafe { *raw::width_base.offset(f as isize) }
}
/// Returns `height_base[f]`.
#[inline]
pub(crate) fn height_base_entry(f: i32) -> i32 {
    unsafe { *raw::height_base.offset(f as isize) }
}
/// Returns `depth_base[f]`.
#[inline]
pub(crate) fn depth_base_entry(f: i32) -> i32 {
    unsafe { *raw::depth_base.offset(f as isize) }
}
/// Returns `italic_base[f]`.
#[inline]
pub(crate) fn italic_base_entry(f: i32) -> i32 {
    unsafe { *raw::italic_base.offset(f as isize) }
}

// ---------------------------------------------------------------------------
// is_in_csname
// ---------------------------------------------------------------------------
#[inline]
pub(crate) fn is_in_csname() -> bool { unsafe { raw::is_in_csname } }
#[inline]
pub(crate) fn set_is_in_csname(v: bool) { unsafe { raw::is_in_csname = v; } }

// ---------------------------------------------------------------------------
// cur_val1 / radix
// ---------------------------------------------------------------------------
#[inline]
pub(crate) fn cur_val1() -> i32 { unsafe { raw::cur_val1 } }
#[inline]
pub(crate) fn set_cur_val1(v: i32) { unsafe { raw::cur_val1 = v; } }
#[inline]
pub(crate) fn radix() -> u8 { unsafe { raw::radix } }
#[inline]
pub(crate) fn set_radix(v: u8) { unsafe { raw::radix = v; } }

// ---------------------------------------------------------------------------
// page builder / last-node / output_active / random_seed / pdf positions
// ---------------------------------------------------------------------------
#[inline]
pub(crate) fn dead_cycles() -> i32 { unsafe { raw::dead_cycles } }
#[inline]
pub(crate) fn set_dead_cycles(v: i32) { unsafe { raw::dead_cycles = v; } }
#[inline]
pub(crate) fn last_badness() -> i32 { unsafe { raw::last_badness } }
#[inline]
pub(crate) fn page_tail() -> i32 { unsafe { raw::page_tail } }
#[inline]
pub(crate) fn page_contents() -> u8 { unsafe { raw::page_contents } }
#[inline]
pub(crate) fn page_so_far(k: usize) -> i32 { unsafe { raw::page_so_far[k] } }
#[inline]
pub(crate) fn set_page_so_far(k: usize, v: i32) { unsafe { raw::page_so_far[k] = v; } }
#[inline]
pub(crate) fn last_glue() -> i32 { unsafe { raw::last_glue } }
#[inline]
pub(crate) fn disc_ptr(k: usize) -> i32 { unsafe { raw::disc_ptr[k] } }
#[inline]
pub(crate) fn set_disc_ptr(k: usize, v: i32) { unsafe { raw::disc_ptr[k] = v; } }
#[inline]
pub(crate) fn last_penalty() -> i32 { unsafe { raw::last_penalty } }
#[inline]
pub(crate) fn last_kern() -> i32 { unsafe { raw::last_kern } }
#[inline]
pub(crate) fn last_node_type() -> i32 { unsafe { raw::last_node_type } }
#[inline]
pub(crate) fn insert_penalties() -> i32 { unsafe { raw::insert_penalties } }
#[inline]
pub(crate) fn set_insert_penalties(v: i32) { unsafe { raw::insert_penalties = v; } }
#[inline]
pub(crate) fn output_active() -> bool { unsafe { raw::output_active } }
#[inline]
pub(crate) fn random_seed() -> i32 { unsafe { raw::random_seed } }
#[inline]
pub(crate) fn pdf_last_x_pos() -> i32 { unsafe { raw::pdf_last_x_pos } }
#[inline]
pub(crate) fn pdf_last_y_pos() -> i32 { unsafe { raw::pdf_last_y_pos } }

// ---------------------------------------------------------------------------
// epochseconds / microseconds
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn epochseconds() -> i32 {
    unsafe { raw::epochseconds }
}
#[inline]
pub(crate) fn microseconds() -> i32 {
    unsafe { raw::microseconds }
}

// ---------------------------------------------------------------------------
// mag_set
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn mag_set() -> i32 {
    unsafe { raw::mag_set }
}
#[inline]
pub(crate) fn set_mag_set(v: i32) {
    unsafe { raw::mag_set = v; }
}

// ---------------------------------------------------------------------------
// semantic nest (nest / nest_ptr / max_nest_stack / nest_size / cur_list)
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn nest_ptr() -> i32 {
    unsafe { raw::nest_ptr }
}
#[inline]
pub(crate) fn set_nest_ptr(v: i32) {
    unsafe { raw::nest_ptr = v; }
}
#[inline]
pub(crate) fn max_nest_stack() -> i32 {
    unsafe { raw::max_nest_stack }
}
#[inline]
pub(crate) fn set_max_nest_stack(v: i32) {
    unsafe { raw::max_nest_stack = v; }
}
#[inline]
pub(crate) fn nest_size() -> i32 {
    unsafe { raw::nest_size }
}
/// Copy `nest[k]` to `cur_list` (full struct assignment).
#[inline]
pub(crate) fn restore_cur_list_from_nest(k: i32) {
    unsafe { raw::cur_list = *raw::nest.offset(k as isize); }
}
/// Copy `cur_list` to `nest[k]` (full struct assignment).
#[inline]
pub(crate) fn save_cur_list_to_nest(k: i32) {
    unsafe { *raw::nest.offset(k as isize) = raw::cur_list; }
}
// cur_list field accessors
#[inline]
pub(crate) fn cur_list_mode() -> i16 {
    unsafe { raw::cur_list.mode }
}
#[inline]
pub(crate) fn set_cur_list_mode(v: i16) {
    unsafe { raw::cur_list.mode = v; }
}
#[inline]
pub(crate) fn cur_list_head() -> i32 {
    unsafe { raw::cur_list.head }
}
#[inline]
pub(crate) fn set_cur_list_head(v: i32) {
    unsafe { raw::cur_list.head = v; }
}
#[inline]
pub(crate) fn cur_list_tail() -> i32 {
    unsafe { raw::cur_list.tail }
}
#[inline]
pub(crate) fn set_cur_list_tail(v: i32) {
    unsafe { raw::cur_list.tail = v; }
}
#[inline]
pub(crate) fn cur_list_prev_graf() -> i32 {
    unsafe { raw::cur_list.prev_graf }
}
/// Returns `cur_list.aux.b32.s0` (the integer/spacefactor half of aux).
#[inline]
pub(crate) fn cur_list_aux_b32s0() -> i32 {
    unsafe { raw::cur_list.aux.b32.s0 }
}
/// Sets `cur_list.aux.b32.s0 = v`.
#[inline]
pub(crate) fn set_cur_list_aux_b32s0(v: i32) {
    unsafe { raw::cur_list.aux.b32.s0 = v; }
}
/// Returns `cur_list.aux.b32.s1` (the prevdepth half of aux).
#[inline]
pub(crate) fn cur_list_aux_b32s1() -> i32 {
    unsafe { raw::cur_list.aux.b32.s1 }
}
/// Sets `cur_list.aux.b32.s1 = v`.
#[inline]
pub(crate) fn set_cur_list_aux_b32s1(v: i32) {
    unsafe { raw::cur_list.aux.b32.s1 = v; }
}
/// Returns `nest[k].mode`.
#[inline]
pub(crate) fn nest_mode(k: i32) -> i16 {
    unsafe { (*raw::nest.offset(k as isize)).mode }
}
/// Returns `nest[k].eTeX_aux`.
#[inline]
pub(crate) fn nest_e_tex_aux(k: i32) -> i32 {
    unsafe { (*raw::nest.offset(k as isize)).e_tex_aux }
}
/// Returns `nest[k].aux.b32.s1` (prevdepth half of the aux word).
#[inline]
pub(crate) fn nest_aux_b32s1(k: i32) -> i32 {
    unsafe { (*raw::nest.offset(k as isize)).aux.b32.s1 }
}
/// Returns `nest[k].prev_graf`.
#[inline]
pub(crate) fn nest_prev_graf(k: i32) -> i32 {
    unsafe { (*raw::nest.offset(k as isize)).prev_graf }
}
#[inline]
pub(crate) fn set_nest_prev_graf(k: i32, v: i32) {
    unsafe { (*raw::nest.offset(k as isize)).prev_graf = v; }
}
#[inline]
pub(crate) fn set_cur_list_prev_graf(v: i32) {
    unsafe { raw::cur_list.prev_graf = v; }
}
#[inline]
pub(crate) fn set_cur_list_mode_line(v: i32) {
    unsafe { raw::cur_list.mode_line = v; }
}
#[inline]
pub(crate) fn cur_list_e_tex_aux() -> i32 {
    unsafe { raw::cur_list.e_tex_aux }
}
#[inline]
pub(crate) fn set_cur_list_e_tex_aux(v: i32) {
    unsafe { raw::cur_list.e_tex_aux = v; }
}

// ---------------------------------------------------------------------------
// font_max
// ---------------------------------------------------------------------------

/// Returns `font_max` — the largest valid internal font number
/// (xetex-xetexd.h:406).
#[inline]
pub(crate) fn font_max() -> i32 {
    unsafe { raw::font_max }
}

// ---------------------------------------------------------------------------
// cur_cmd / cur_mark
// ---------------------------------------------------------------------------

/// Returns the current command code (`cur_cmd`, xetex-xetexd.h:499).
#[inline]
pub(crate) fn cur_cmd() -> u8 {
    unsafe { raw::cur_cmd }
}
/// Sets `cur_cmd`.
#[inline]
pub(crate) fn set_cur_cmd(v: u8) {
    unsafe { raw::cur_cmd = v; }
}
/// Sets `cur_chr`.
#[inline]
pub(crate) fn set_cur_chr(v: i32) {
    unsafe { raw::cur_chr = v; }
}

/// Returns `cur_mark[k]` — the k-th mark register token list
/// (xetex-xetexd.h:527; k must be 0..=4).
#[inline]
pub(crate) fn cur_mark(k: usize) -> i32 {
    unsafe { raw::cur_mark[k] }
}

// ---------------------------------------------------------------------------
// input_stack / input_ptr / base_ptr
// ---------------------------------------------------------------------------

/// Copy `input_stack[k]` (full struct copy; xetex-xetexd.h:503).
#[inline]
pub(crate) fn input_stack(k: i32) -> InputState {
    // SAFETY: see module-level doc. Caller ensures k <= input_ptr.
    unsafe { *raw::input_stack.offset(k as isize) }
}
/// Write `input_stack[k] = v` (full struct assign).
#[inline]
pub(crate) fn set_input_stack(k: i32, v: InputState) {
    // SAFETY: see module-level doc.
    unsafe { *raw::input_stack.offset(k as isize) = v; }
}

/// Returns `input_ptr` — current top of the input-stack (xetex-xetexd.h:504).
#[inline]
pub(crate) fn input_ptr() -> i32 {
    unsafe { raw::input_ptr }
}
/// Sets `input_ptr`.
#[inline]
pub(crate) fn set_input_ptr(v: i32) {
    unsafe { raw::input_ptr = v; }
}
/// Returns `max_in_stack` (xetex-xetexd.h:505).
#[inline]
pub(crate) fn max_in_stack() -> i32 {
    unsafe { raw::max_in_stack }
}
/// Sets `max_in_stack`.
#[inline]
pub(crate) fn set_max_in_stack(v: i32) {
    unsafe { raw::max_in_stack = v; }
}
/// Returns `stack_size` — capacity of the input stack (xetex-xetexd.h:410).
#[inline]
pub(crate) fn stack_size() -> i32 {
    unsafe { raw::stack_size }
}

/// Returns `base_ptr` — search base in the input stack
/// (xetex-xetexd.h:521).
#[inline]
pub(crate) fn base_ptr() -> i32 {
    unsafe { raw::base_ptr }
}
/// Sets `base_ptr`.
#[inline]
pub(crate) fn set_base_ptr(v: i32) {
    unsafe { raw::base_ptr = v; }
}

/// Returns `par_loc` — hash-table location of \par (xetex-xetexd.h:522).
#[inline]
pub(crate) fn par_loc() -> i32 {
    unsafe { raw::par_loc }
}
/// Sets `par_loc`.
#[inline]
pub(crate) fn set_par_loc(v: i32) {
    unsafe { raw::par_loc = v; }
}
/// Returns `par_token` — CS_TOKEN_FLAG + par_loc (xetex-xetexd.h:523).
#[inline]
pub(crate) fn par_token() -> i32 {
    unsafe { raw::par_token }
}
/// Sets `par_token`.
#[inline]
pub(crate) fn set_par_token(v: i32) {
    unsafe { raw::par_token = v; }
}

// ---------------------------------------------------------------------------
// grp_stack / if_stack
// ---------------------------------------------------------------------------

/// Returns `grp_stack[k]` — the save-stack boundary saved at input level k
/// (xetex-xetexd.h:705; save_pointer = i32).
#[inline]
pub(crate) fn grp_stack(k: i32) -> i32 {
    unsafe { *raw::grp_stack.offset(k as isize) }
}
/// Sets `grp_stack[k]`.
#[inline]
pub(crate) fn set_grp_stack(k: i32, v: i32) {
    unsafe { *raw::grp_stack.offset(k as isize) = v; }
}

/// Returns `if_stack[k]` — the `cond_ptr` saved at input level k
/// (xetex-xetexd.h:706).
#[inline]
pub(crate) fn if_stack(k: i32) -> i32 {
    unsafe { *raw::if_stack.offset(k as isize) }
}
/// Sets `if_stack[k]`.
#[inline]
pub(crate) fn set_if_stack(k: i32, v: i32) {
    unsafe { *raw::if_stack.offset(k as isize) = v; }
}

// ---------------------------------------------------------------------------
// cond_ptr / if_limit / cur_if / if_line
// ---------------------------------------------------------------------------

/// Returns `cond_ptr` — head of the conditional-stack list
/// (xetex-xetexd.h:537).
#[inline]
pub(crate) fn cond_ptr() -> i32 {
    unsafe { raw::cond_ptr }
}
/// Sets `cond_ptr`.
#[inline]
pub(crate) fn set_cond_ptr(v: i32) {
    unsafe { raw::cond_ptr = v; }
}

/// Returns `if_limit` — current \if nesting limit code
/// (xetex-xetexd.h:538; unsigned char).
#[inline]
pub(crate) fn if_limit() -> u8 {
    unsafe { raw::if_limit }
}
/// Sets `if_limit`.
#[inline]
pub(crate) fn set_if_limit(v: u8) {
    unsafe { raw::if_limit = v; }
}

/// Returns `cur_if` — current \if type (xetex-xetexd.h:539; small_number = i16).
#[inline]
pub(crate) fn cur_if() -> i16 {
    unsafe { raw::cur_if }
}
/// Sets `cur_if`.
#[inline]
pub(crate) fn set_cur_if(v: i16) {
    unsafe { raw::cur_if = v; }
}

/// Returns `if_line` — line number where the current \if was entered
/// (xetex-xetexd.h:540).
#[inline]
pub(crate) fn if_line() -> i32 {
    unsafe { raw::if_line }
}
/// Sets `if_line`.
#[inline]
pub(crate) fn set_if_line(v: i32) {
    unsafe { raw::if_line = v; }
}
/// Returns `skip_line` (source line where the current \if-skip started).
#[inline]
pub(crate) fn skip_line() -> i32 {
    unsafe { raw::skip_line }
}
/// Sets `skip_line = v`.
#[inline]
pub(crate) fn set_skip_line(v: i32) {
    unsafe { raw::skip_line = v; }
}

// ---------------------------------------------------------------------------
// sa_root / sa_chain / sa_level
// ---------------------------------------------------------------------------

/// Read `sa_root[i]` (the root of sparse-array trie for class i).
#[inline]
pub(crate) fn sa_root(i: usize) -> i32 {
    unsafe { raw::sa_root[i] }
}
/// Write `sa_root[i] = v`.
#[inline]
pub(crate) fn set_sa_root(i: usize, v: i32) {
    unsafe { raw::sa_root[i] = v; }
}
#[inline]
pub(crate) fn sa_chain() -> i32 {
    unsafe { raw::sa_chain }
}
#[inline]
pub(crate) fn set_sa_chain(v: i32) {
    unsafe { raw::sa_chain = v; }
}
#[inline]
pub(crate) fn sa_level() -> u16 {
    unsafe { raw::sa_level }
}
#[inline]
pub(crate) fn set_sa_level(v: u16) {
    unsafe { raw::sa_level = v; }
}

// ---------------------------------------------------------------------------
// cur_ptr / sa_null
// ---------------------------------------------------------------------------

/// Returns `cur_ptr` — SA trie traversal cursor (xetex-xetexd.h:710).
#[inline]
pub(crate) fn cur_ptr() -> i32 {
    unsafe { raw::cur_ptr }
}
/// Sets `cur_ptr`.
#[inline]
pub(crate) fn set_cur_ptr(v: i32) {
    unsafe { raw::cur_ptr = v; }
}
/// Returns a copy of `sa_null` — the SA null memory-word sentinel.
#[inline]
pub(crate) fn sa_null() -> crate::mem_types::MemoryWord {
    unsafe { raw::sa_null }
}

// ---------------------------------------------------------------------------
// shown_mode
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn shown_mode() -> i16 {
    unsafe { raw::shown_mode }
}
#[inline]
pub(crate) fn set_shown_mode(v: i16) {
    unsafe { raw::shown_mode = v; }
}

// ---------------------------------------------------------------------------
// max_reg_num / max_reg_help_line
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn max_reg_num() -> i32 {
    unsafe { raw::max_reg_num }
}
#[inline]
pub(crate) fn max_reg_help_line() -> *const libc::c_char {
    unsafe { raw::max_reg_help_line }
}

// ---------------------------------------------------------------------------
// param_stack / param_ptr / max_param_stack
// ---------------------------------------------------------------------------

/// Returns `param_stack[k]` — the k-th macro parameter token list
/// (xetex-xetexd.h:517).
#[inline]
pub(crate) fn param_stack(k: i32) -> i32 {
    unsafe { *raw::param_stack.offset(k as isize) }
}
/// Writes `param_stack[k] = v`.
#[inline]
pub(crate) fn set_param_stack(k: i32, v: i32) {
    unsafe { *raw::param_stack.offset(k as isize) = v; }
}
/// Returns `param_ptr` — next free slot in param_stack (xetex-xetexd.h:518).
#[inline]
pub(crate) fn param_ptr() -> i32 {
    unsafe { raw::param_ptr }
}
/// Sets `param_ptr`.
#[inline]
pub(crate) fn set_param_ptr(v: i32) {
    unsafe { raw::param_ptr = v; }
}
/// Returns `max_param_stack` (xetex-xetexd.h:519).
#[inline]
pub(crate) fn max_param_stack() -> i32 {
    unsafe { raw::max_param_stack }
}
/// Sets `max_param_stack`.
#[inline]
pub(crate) fn set_max_param_stack(v: i32) {
    unsafe { raw::max_param_stack = v; }
}

// ---------------------------------------------------------------------------
// first / buf_size / max_in_open
// ---------------------------------------------------------------------------

/// Returns `first` — next free index in the input buffer (xetex-xetexd.h:393).
#[inline]
pub(crate) fn first() -> i32 {
    unsafe { raw::first }
}
/// Sets `first`.
#[inline]
pub(crate) fn set_first(v: i32) {
    unsafe { raw::first = v; }
}
/// Returns `buf_size` — capacity of the input buffer (xetex-xetexd.h:409).
#[inline]
pub(crate) fn buf_size() -> i32 {
    unsafe { raw::buf_size }
}
/// Returns `max_in_open` — maximum allowed input-nesting depth (xetex-xetexd.h:411).
#[inline]
pub(crate) fn max_in_open() -> i32 {
    unsafe { raw::max_in_open }
}

// ---------------------------------------------------------------------------
// set_in_open
// ---------------------------------------------------------------------------

/// Sets `in_open`.
#[inline]
pub(crate) fn set_in_open(v: i32) {
    unsafe { raw::in_open = v; }
}

// ---------------------------------------------------------------------------
// source_filename_stack / full_source_filename_stack / line_stack
// ---------------------------------------------------------------------------

/// Returns `source_filename_stack[k]` (str_number; xetex-xetexd.h:512).
#[inline]
pub(crate) fn source_filename_stack(k: i32) -> i32 {
    unsafe { *raw::source_filename_stack.offset(k as isize) }
}
/// Sets `source_filename_stack[k]`.
#[inline]
pub(crate) fn set_source_filename_stack(k: i32, v: i32) {
    unsafe { *raw::source_filename_stack.offset(k as isize) = v; }
}
/// Sets `full_source_filename_stack[k]`.
#[inline]
pub(crate) fn set_full_source_filename_stack(k: i32, v: i32) {
    unsafe { *raw::full_source_filename_stack.offset(k as isize) = v; }
}
/// Sets `line_stack[k]`.
#[inline]
pub(crate) fn set_line_stack(k: i32, v: i32) {
    unsafe { *raw::line_stack.offset(k as isize) = v; }
}
/// Sets `line`.
#[inline]
pub(crate) fn set_line(v: i32) {
    unsafe { raw::line = v; }
}

// ---------------------------------------------------------------------------
// eof_seen
// ---------------------------------------------------------------------------

/// Returns `eof_seen[k]` — whether EOF was seen at input level k.
#[inline]
pub(crate) fn eof_seen(k: i32) -> bool {
    unsafe { *raw::eof_seen.offset(k as isize) }
}
/// Sets `eof_seen[k]`.
#[inline]
pub(crate) fn set_eof_seen(k: i32, v: bool) {
    unsafe { *raw::eof_seen.offset(k as isize) = v; }
}

// ---------------------------------------------------------------------------
// input_file
// ---------------------------------------------------------------------------

/// Returns `input_file[k]` as a raw pointer (UFILE*; xetex-xetexd.h:509).
#[inline]
pub(crate) fn input_file(k: i32) -> *mut libc::c_void {
    unsafe { *raw::input_file.offset(k as isize) }
}
/// Sets `input_file[k]`.
#[inline]
pub(crate) fn set_input_file(k: i32, v: *mut libc::c_void) {
    unsafe { *raw::input_file.offset(k as isize) = v; }
}
/// Returns a pointer to `input_file[k]` (i.e. &input_file[k]), for passing to `u_open_in`.
#[inline]
pub(crate) fn input_file_ptr(k: i32) -> *mut *mut crate::io::UFile {
    unsafe { raw::input_file.offset(k as isize) as *mut *mut crate::io::UFile }
}

// ---------------------------------------------------------------------------
// last / max_buf_stack / set_buffer_entry / cur_input_limit
// ---------------------------------------------------------------------------

/// Returns `last` — index one past the last character on the current input line.
#[inline]
pub(crate) fn last() -> i32 {
    unsafe { raw::last }
}
/// Sets `last`.
#[inline]
pub(crate) fn set_last(v: i32) {
    unsafe { raw::last = v; }
}
/// Returns `max_buf_stack` — highest value `last` has ever reached.
#[inline]
pub(crate) fn max_buf_stack() -> i32 {
    unsafe { raw::max_buf_stack }
}
/// Sets `max_buf_stack`.
#[inline]
pub(crate) fn set_max_buf_stack(v: i32) {
    unsafe { raw::max_buf_stack = v; }
}
/// Writes `buffer[k] = v`.
#[inline]
pub(crate) fn set_buffer_entry(k: i32, v: i32) {
    unsafe { *raw::buffer.offset(k as isize) = v; }
}
/// Writes `cur_input.limit = v`.
#[inline]
pub(crate) fn set_cur_input_limit(v: i32) {
    unsafe { raw::cur_input.limit = v; }
}

// ---------------------------------------------------------------------------
// expand_depth / expand_depth_count / cur_val_level
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn expand_depth() -> i32 { unsafe { raw::expand_depth } }
#[inline]
pub(crate) fn expand_depth_count() -> i32 { unsafe { raw::expand_depth_count } }
#[inline]
pub(crate) fn set_expand_depth_count(v: i32) { unsafe { raw::expand_depth_count = v; } }
#[inline]
pub(crate) fn cur_val_level() -> u8 { unsafe { raw::cur_val_level } }
#[inline]
pub(crate) fn set_cur_val_level(v: u8) { unsafe { raw::cur_val_level = v; } }

// ---------------------------------------------------------------------------
// scanner_status / warning_index / def_ref
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn scanner_status() -> u8 {
    unsafe { raw::scanner_status }
}
#[inline]
pub(crate) fn set_scanner_status(v: u8) {
    unsafe { raw::scanner_status = v; }
}
#[inline]
pub(crate) fn warning_index() -> i32 {
    unsafe { raw::warning_index }
}
#[inline]
pub(crate) fn set_warning_index(v: i32) {
    unsafe { raw::warning_index = v; }
}
#[inline]
pub(crate) fn def_ref() -> i32 {
    unsafe { raw::def_ref }
}
#[inline]
pub(crate) fn set_def_ref(v: i32) {
    unsafe { raw::def_ref = v; }
}

// ---------------------------------------------------------------------------
// gave_char_warning_help / main_p / main_k
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn gave_char_warning_help() -> bool {
    unsafe { raw::gave_char_warning_help }
}
#[inline]
pub(crate) fn set_gave_char_warning_help(v: bool) {
    unsafe { raw::gave_char_warning_help = v; }
}
#[inline]
pub(crate) fn main_p() -> i32 {
    unsafe { raw::main_p }
}
#[inline]
pub(crate) fn set_main_p(v: i32) {
    unsafe { raw::main_p = v; }
}
#[inline]
pub(crate) fn main_k() -> i32 {
    unsafe { raw::main_k }
}
#[inline]
pub(crate) fn set_main_k(v: i32) {
    unsafe { raw::main_k = v; }
}

// ---------------------------------------------------------------------------
// cur_lang / insert_src_special_every_par
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn cur_lang() -> u8 {
    unsafe { raw::cur_lang }
}
#[inline]
pub(crate) fn set_cur_lang(v: u8) {
    unsafe { raw::cur_lang = v; }
}
#[inline]
pub(crate) fn insert_src_special_every_par() -> bool {
    unsafe { raw::insert_src_special_every_par }
}

// ---------------------------------------------------------------------------
// cur_box
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn cur_box() -> i32 {
    unsafe { raw::cur_box }
}
#[inline]
pub(crate) fn set_cur_box(v: i32) {
    unsafe { raw::cur_box = v; }
}

// ---------------------------------------------------------------------------
// cur_order
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn cur_order() -> u8 {
    unsafe { raw::cur_order }
}

// ---------------------------------------------------------------------------
// alignment state
// ---------------------------------------------------------------------------

#[inline] pub(crate) fn cur_align() -> i32 { unsafe { raw::cur_align } }
#[inline] pub(crate) fn set_cur_align(v: i32) { unsafe { raw::cur_align = v; } }
#[inline] pub(crate) fn cur_span() -> i32 { unsafe { raw::cur_span } }
#[inline] pub(crate) fn set_cur_span(v: i32) { unsafe { raw::cur_span = v; } }
#[inline] pub(crate) fn cur_loop() -> i32 { unsafe { raw::cur_loop } }
#[inline] pub(crate) fn set_cur_loop(v: i32) { unsafe { raw::cur_loop = v; } }
#[inline] pub(crate) fn align_ptr() -> i32 { unsafe { raw::align_ptr } }
#[inline] pub(crate) fn set_align_ptr(v: i32) { unsafe { raw::align_ptr = v; } }
#[inline] pub(crate) fn cur_head() -> i32 { unsafe { raw::cur_head } }
#[inline] pub(crate) fn set_cur_head(v: i32) { unsafe { raw::cur_head = v; } }
#[inline] pub(crate) fn cur_tail_a() -> i32 { unsafe { raw::cur_tail } }
#[inline] pub(crate) fn set_cur_tail_a(v: i32) { unsafe { raw::cur_tail = v; } }
#[inline] pub(crate) fn cur_pre_head() -> i32 { unsafe { raw::cur_pre_head } }
#[inline] pub(crate) fn set_cur_pre_head(v: i32) { unsafe { raw::cur_pre_head = v; } }
#[inline] pub(crate) fn cur_pre_tail() -> i32 { unsafe { raw::cur_pre_tail } }
#[inline] pub(crate) fn set_cur_pre_tail(v: i32) { unsafe { raw::cur_pre_tail = v; } }

// ---------------------------------------------------------------------------
// hpack adjust-tail / total_stretch / total_shrink
// ---------------------------------------------------------------------------

#[inline] pub(crate) fn adjust_tail() -> i32 { unsafe { raw::adjust_tail } }
#[inline] pub(crate) fn set_adjust_tail(v: i32) { unsafe { raw::adjust_tail = v; } }
#[inline] pub(crate) fn pre_adjust_tail() -> i32 { unsafe { raw::pre_adjust_tail } }
#[inline] pub(crate) fn set_pre_adjust_tail(v: i32) { unsafe { raw::pre_adjust_tail = v; } }
#[inline] pub(crate) fn total_stretch(o: usize) -> i32 { unsafe { raw::total_stretch[o] } }
#[inline] pub(crate) fn set_total_stretch(o: usize, v: i32) { unsafe { raw::total_stretch[o] = v; } }
#[inline] pub(crate) fn total_shrink(o: usize) -> i32 { unsafe { raw::total_shrink[o] } }
#[inline] pub(crate) fn set_total_shrink(o: usize, v: i32) { unsafe { raw::total_shrink[o] = v; } }
#[inline] pub(crate) fn pack_begin_line() -> i32 { unsafe { raw::pack_begin_line } }
#[inline] pub(crate) fn set_pack_begin_line(v: i32) { unsafe { raw::pack_begin_line = v; } }
#[inline] pub(crate) fn set_box_allowed() -> bool { unsafe { raw::set_box_allowed } }
#[inline] pub(crate) fn set_set_box_allowed(v: bool) { unsafe { raw::set_box_allowed = v; } }

/// Returns `cur_list.mode_line` (the source line where the current mode started).
#[inline]
pub(crate) fn cur_list_mode_line() -> i32 {
    unsafe { raw::cur_list.mode_line }
}

// ---------------------------------------------------------------------------
// active_width / best_height_plus_depth
// ---------------------------------------------------------------------------

/// Read `active_width[k]` (1-indexed, k in 1..=6).
#[inline]
pub(crate) fn active_width(k: usize) -> i32 {
    unsafe { raw::active_width[k] }
}

/// Write `active_width[k] = v` (1-indexed, k in 1..=6).
#[inline]
pub(crate) fn set_active_width(k: usize, v: i32) {
    unsafe { raw::active_width[k] = v; }
}

/// Read `best_height_plus_depth`.
#[inline]
pub(crate) fn best_height_plus_depth() -> i32 {
    unsafe { raw::best_height_plus_depth }
}

/// Write `best_height_plus_depth`.
#[inline]
pub(crate) fn set_best_height_plus_depth(v: i32) {
    unsafe { raw::best_height_plus_depth = v; }
}
