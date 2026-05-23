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
