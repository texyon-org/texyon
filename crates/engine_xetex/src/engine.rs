//! Engine init / run / cleanup: the `tt_run_engine` entry point, the
//! initialize_* primers, primitive registration, and teardown — translated
//! from the driver part of xetex-ini.c.

use crate::mem::{xmalloc};
use crate::consts::{
    ACTIVE_BASE, BIGGEST_CHAR, BOX_BASE, CAT_CODE_BASE, CS_TOKEN_FLAG, CUR_FONT_LOC,
    DEL_CODE_BASE, DIMEN_BASE, EQTB_SIZE, ETEX_PEN_BASE, FONT_BASE, GLUE_BASE, GLUE_NODE,
    HISTORY_FATAL_ERROR, HISTORY_SPOTLESS, INT_BASE, INT_PAR_DAY, INT_PAR_END_LINE_CHAR,
    INT_PAR_MONTH, INT_PAR_XETEX_GENERATE_ACTUAL_TEXT, INT_PAR_YEAR, LC_CODE_BASE, LEVEL_ONE,
    LOCAL_BASE, MATH_CODE_BASE, MATH_FONT_BASE, MAX_FONT_MAX, MAX_HALFWORD, MEM_TOP, NEW_LINE,
    NON_ADDRESS, NUMBER_USVS, PC_INT_BASE, PRIM_EQTB_BASE, SELECTOR_NO_PRINT,
    SELECTOR_TERM_AND_LOG, SELECTOR_TERM_ONLY, SF_CODE_BASE, SINGLE_BASE, TOKS_BASE, TOO_BIG_CHAR,
    UC_CODE_BASE,
};
use crate::format_io::load_fmt_file;
use crate::io::init_io;
use crate::state::{EngineState, GroupCode, MemState};
use crate::state::InteractionMode;

const GET_NODE_HEAD_SENTINEL: i32 = 0x4000_0000;

const FORMAT_HASH_OFFSET: i32 = 514;
const HASH_PRIME: i32 = 557_111;
const HASH_SIZE: i32 = 2_242_711;
const HASH_BASE: i32 = 1_114_114;
const HASH_TOP_UNDEFINED_CONTROL_SEQUENCE: i32 = 2_254_339;
use crate::consts::EMPTY_STRING;
const INT_PAR_TIME: i32 = 20;
const NORMAL: i32 = 0;

// The `TRE_*` capacity statics are set by `tt_run_engine`; the two that
// `load_fmt_file` also reads are `pub(crate)`.
pub(crate) static mut TRE_POOL_SIZE: i32 = 0;
static mut TRE_STRING_VACANCIES: i32 = 0;
pub(crate) static mut TRE_MAX_STRINGS: i32 = 0;
static mut TRE_TRIE_SIZE: i32 = 0;
static mut TRE_HYPH_SIZE: i32 = 0;

extern "C" {
    // bridge_core: record a fatal message without longjmp-ing, so the host's
    // ttbc_get_error_message() surfaces it after we return HISTORY_FATAL_ERROR.
    fn _ttbc_set_error_message(message: *const libc::c_char);
}

// Raw lifecycle state used by tt_run_engine.
use crate::bridge::ttstub_output_open_stdout;
extern "C" {
    fn pdf_files_init();
}

// Engine-lifecycle globals formerly defined in xetex-ini.c. Set once per run in
// tt_run_engine; TEX_FORMAT_DEFAULT is freed in tt_cleanup_in.
pub(crate) static mut TEX_FORMAT_DEFAULT: *mut libc::c_char = std::ptr::null_mut();
pub(crate) static mut FORMAT_DEFAULT_LENGTH: i32 = 0;
pub(crate) static mut POOL_FREE: i32 = 0;
pub(crate) static mut STRINGS_FREE: i32 = 0;
pub(crate) static mut OUTPUT_FILE_EXTENSION: *const libc::c_char = std::ptr::null();
static mut BAD: i32 = 0;

// All tt_cleanup_ items need #[link_name] since the Rust names carry a prefix
// that doesn't exist in C, and some also clash with existing accessor functions.
extern "C" {
    #[link_name = "pdf_files_close"]
    fn tt_cleanup_pdf_files_close();
    #[link_name = "destroy_font_manager"]
    fn tt_cleanup_destroy_font_manager();
    fn xetex_layout_reset_font_caches();
}

pub(crate) fn reset_layout_font_caches() {
    // SAFETY: clears mutex-protected process-global maps and does not borrow
    // engine state.
    unsafe { xetex_layout_reset_font_caches() };
}

// translated from xetex-ini.c:3452 (get_strings_started)
pub(crate) unsafe fn get_strings_started_in(
    st: &mut crate::state::EngineState,
) -> Result<(), crate::errors::EngineAbort> {
    st.strings.pool_ptr = 0;
    st.strings.str_ptr = 0;
    st.strings.str_start[0] = 0;
    st.strings.str_ptr = TOO_BIG_CHAR;

    let spare_size = st.strings.pool_size - st.strings.string_vacancies;
    let loaded = match crate::string_pool::load_pool_strings(&mut st.strings, spare_size) {
        Ok(value) => value,
        Err(e) => return Err(e.into_engine_abort()),
    };
    if loaded == 0 {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"must increase pool_size".as_ptr(),
        ));
    }
    Ok(())
}
/// translated from xetex-ini.c:554 (sort_avail)
pub(crate) fn sort_avail_in(mem: &mut MemState) {
    let _ = crate::mem::get_node(mem, GET_NODE_HEAD_SENTINEL);
    let mut p = mem.b32s1(mem.rover + 1);
    mem.set_b32s1(mem.rover + 1, MAX_HALFWORD);
    let old_rover = mem.rover;

    /*136:*/
    while p != old_rover {
        if p < mem.rover {
            let q = p;
            p = mem.b32s1(q + 1);
            mem.set_b32s1(q + 1, mem.rover);
            mem.rover = q;
        } else {
            let mut q = mem.rover;
            while mem.b32s1(q + 1) < p {
                q = mem.b32s1(q + 1);
            }
            let r = mem.b32s1(p + 1);
            mem.set_b32s1(p + 1, mem.b32s1(q + 1));
            mem.set_b32s1(q + 1, p);
            p = r;
        }
    }

    p = mem.rover;
    while mem.b32s1(p + 1) != MAX_HALFWORD {
        mem.set_b32s0(mem.b32s1(p + 1) + 1, p);
        p = mem.b32s1(p + 1);
    }

    mem.set_b32s1(p + 1, mem.rover);
    mem.set_b32s0(mem.rover + 1, p);
}

enum PrimitiveInitAbort {
    MakeTexString(crate::texmfmp::MakeTexStringAbort),
    HashLookup(crate::hash::HashLookupError),
    BufferSize(i32),
}

impl From<crate::texmfmp::MakeTexStringAbort> for PrimitiveInitAbort {
    fn from(value: crate::texmfmp::MakeTexStringAbort) -> Self {
        Self::MakeTexString(value)
    }
}

impl From<crate::hash::HashLookupError> for PrimitiveInitAbort {
    fn from(value: crate::hash::HashLookupError) -> Self {
        Self::HashLookup(value)
    }
}

impl PrimitiveInitAbort {
    /// Fold this init-time sentinel into the unified terminal [`EngineAbort`].
    /// `tt_run_engine` catches it and returns `into_history()`.
    fn into_engine_abort(self) -> crate::errors::EngineAbort {
        use crate::errors::EngineAbort;
        match self {
            PrimitiveInitAbort::MakeTexString(e) => match e {
                crate::texmfmp::MakeTexStringAbort::PoolOverflow => {
                    EngineAbort::TtAbort(c"string pool overflow".as_ptr())
                }
                crate::texmfmp::MakeTexStringAbort::StringOverflow => EngineAbort::StringOverflow,
            },
            PrimitiveInitAbort::HashLookup(e) => {
                use crate::hash::HashLookupError::*;
                let (what, n) = match e {
                    HashSizeOverflow(n) => (c"hash size".as_ptr(), n),
                    PoolSizeOverflow(n) => (c"pool size".as_ptr(), n),
                    StringNumberOverflow(n) => (c"number of strings".as_ptr(), n),
                    PrimitiveSizeOverflow(n) => (c"primitive size".as_ptr(), n),
                };
                EngineAbort::Overflow { what, n }
            }
            PrimitiveInitAbort::BufferSize(limit) => EngineAbort::Overflow {
                what: c"buffer size".as_ptr(),
                n: limit,
            },
        }
    }
}

// translated from xetex-ini.c:597 (primitive)
unsafe fn primitive(
    st: &mut EngineState,
    ident: *const libc::c_char,
    c: u16,
    o: i32,
) -> Result<(), PrimitiveInitAbort> {
    let len = libc::strlen(ident) as i32;
    let prim_val: i32;

    if len > 1 {
        let s = crate::texmfmp::maketexstring_in(&mut st.strings, ident)?;

        if st.input.first + len > st.input.buf_size + 1 {
            return Err(PrimitiveInitAbort::BufferSize(st.input.buf_size));
        }

        for i in 0..len {
            let b = *ident.offset(i as isize) as u8 as i32;
            st.input.buffer[(st.input.first + i) as usize] = b;
        }

        st.scanner.cur_val = crate::hash::id_lookup(
            &mut st.eqtb,
            &mut st.strings,
            &st.input.buffer,
            st.input.first,
            len,
        )?;
        st.strings.str_ptr -= 1;
        st.strings.pool_ptr = st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
        st.eqtb.set_hash_s1(st.scanner.cur_val, s);
        prim_val = crate::hash::prim_lookup(&mut st.eqtb, &mut st.strings, s)?;
    } else {
        let ch = *ident as u8 as i32;
        st.scanner.cur_val = ch + SINGLE_BASE;
        prim_val = crate::hash::prim_lookup(&mut st.eqtb, &mut st.strings, ch)?;
    }

    st.eqtb.set_b16s0(st.scanner.cur_val, LEVEL_ONE);
    st.eqtb.set_b16s1(st.scanner.cur_val, c);
    st.eqtb.set_b32s1(st.scanner.cur_val, o);
    st.eqtb.set_b16s0(PRIM_EQTB_BASE + prim_val, LEVEL_ONE);
    st.eqtb.set_b16s1(PRIM_EQTB_BASE + prim_val, c);
    st.eqtb.set_b32s1(PRIM_EQTB_BASE + prim_val, o);
    Ok(())
}
// translated from xetex-ini.c:2201 (final_cleanup)
pub(crate) unsafe fn final_cleanup_in(
    st: &mut crate::state::EngineState,
) -> Result<(), crate::errors::EngineAbort> {
    const INT_PAR_NEW_LINE_CHAR: i32 = 49;
    const IF_NODE_SIZE: i32 = 2;
    const IF_TEST: u16 = 107;
    const TOKEN_LIST: u16 = 0;
    const TOP_MARK_CODE: usize = 0;
    const SPLIT_BOT_MARK_CODE: usize = 4;
    const LAST_BOX_CODE: usize = 2;
    const VSPLIT_CODE: usize = 3;
    const MARK_VAL: usize = 7;
    const HISTORY_SPOTLESS: i32 = 0;
    const HISTORY_WARNING_ISSUED: i32 = 1;

    let mut c = st.scanner.cur_chr;

    if c != 1 {
        st.eqtb.data[(INT_BASE + INT_PAR_NEW_LINE_CHAR) as usize].b32.s1 = -1;
    }

    if st.strings.job_name == 0 {
        crate::io::open_log_file_in(st).map_err(|e| e.into_engine_abort())?;
    }

    while st.input.input_ptr > 0 {
        if st.input.cur_input.state == TOKEN_LIST {
            crate::input::end_token_list_in(st).map_err(|e| e.into_engine_abort())?;
        } else {
            crate::input::end_file_reading_in(st).map_err(|e| e.into_engine_abort())?;
        }
    }

    loop {
        let n = st.input.open_parens;
        if n <= 0 {
            break;
        }
        crate::output::print_cstr_in(st, c" )".as_ptr());
        st.input.open_parens = n - 1;
    }

    let cur_level = st.save_stack.cur_level;
    if cur_level > LEVEL_ONE {
        crate::output::print_nl_in(st, b'(' as i32);
        crate::output::print_esc_cstr_in(st, c"end occurred ".as_ptr());
        crate::output::print_cstr_in(st, c"inside a group at level ".as_ptr());
        crate::output::print_int_in(st, cur_level as i32 - 1);
        crate::output::print_char_in(st, b')' as i32);
        crate::output::show_save_groups_in(st);
    }

    while st.scanner.cond_ptr.0 != crate::mem::TEX_NULL {
        crate::output::print_nl_in(st, b'(' as i32);
        crate::output::print_esc_cstr_in(st, c"end occurred ".as_ptr());
        crate::output::print_cstr_in(st, c"when ".as_ptr());
        let cur_if = st.scanner.cur_if as i32;
        crate::output::print_cmd_chr_in(st, IF_TEST, cur_if);
        let if_line = st.scanner.if_line;
        if if_line != 0 {
            crate::output::print_cstr_in(st, c" on line ".as_ptr());
            crate::output::print_int_in(st, if_line);
        }
        crate::output::print_cstr_in(st, c" was incomplete)".as_ptr());
        let cond_ptr = st.scanner.cond_ptr.0;
        // SAFETY: union projection; the `Vec` index is bounds-checked.
        let new_if_line = unsafe { st.mem.data[(cond_ptr + 1) as usize].b32.s1 };
        // SAFETY: union projection; the `Vec` index is bounds-checked.
        let new_cur_if = unsafe { st.mem.data[cond_ptr as usize].b16.s0 } as i16;
        // SAFETY: union projection; the `Vec` index is bounds-checked.
        let new_cond_ptr = unsafe { st.mem.data[cond_ptr as usize].b32.s1 };
        st.mem.temp_ptr.0 = cond_ptr;
        st.scanner.if_line = new_if_line;
        st.scanner.cur_if = new_cur_if;
        st.scanner.cond_ptr.0 = new_cond_ptr;
        let p = st.mem.temp_ptr.0;
        crate::mem::free_node(&mut st.mem, crate::units::MemPtr::from_raw(p), IF_NODE_SIZE);
    }

    if st.errors.history != HISTORY_SPOTLESS
        && (st.errors.history == HISTORY_WARNING_ISSUED || st.errors.interaction < InteractionMode::ErrorStop)
        && st.shipout.selector == SELECTOR_TERM_AND_LOG
    {
        st.shipout.selector = SELECTOR_TERM_ONLY;
        crate::output::print_nl_cstr_in(
            st,
            c"(see the transcript file for additional information)".as_ptr(),
        );
        st.shipout.selector = SELECTOR_TERM_AND_LOG;
    }

    if c == 1 {
        if st.config.in_initex_mode {
            c = TOP_MARK_CODE as i32;
            loop {
                let mark = st.scanner.cur_mark[c as usize];
                if !mark.is_null() {
                    crate::mem::delete_token_ref(&mut st.mem, mark);
                }
                if c as usize >= SPLIT_BOT_MARK_CODE {
                    break;
                }
                c += 1;
            }

            let marks = st.scanner.sa_root[MARK_VAL];
            if let Some(marks) = marks.as_idx() {
                if crate::packaging::do_marks(&mut st.mem, &mut st.scanner, 3, 0, marks) {
                    st.scanner.sa_root[MARK_VAL] = crate::units::MemPtr::NULL;
                }
            }

            c = LAST_BOX_CODE as i32;
            loop {
                crate::packaging::flush_node_list(&mut st.mem, st.scanner.disc_ptr[c as usize])
                    .map_err(|msg| crate::errors::EngineAbort::Confusion(msg.as_ptr()))?;
                if c as usize >= VSPLIT_CODE {
                    break;
                }
                c += 1;
            }

            if st.shipout.last_glue.0 != MAX_HALFWORD {
                crate::mem::delete_glue_ref(&mut st.mem, st.shipout.last_glue.0);
            }

            crate::format_io::store_fmt_file_in(st)?;
            return Ok(());
        }
        crate::output::print_nl_cstr_in(st, c"(\\dump is performed only by INITEX)".as_ptr());
    }
    Ok(())
}
// translated from xetex-ini.c:2210 (initialize_more_variables)
fn initialize_more_variables_in(st: &mut crate::state::EngineState) {
    const VMODE: i16 = 1;
    const MEM_TOP: i32 = 4_999_999;
    const CONTRIB_HEAD: i32 = MEM_TOP - 1;
    const PAGE_HEAD: i32 = MEM_TOP - 2;
    const IGNORE_DEPTH: i32 = -65_536_000;
    const EMPTY: u8 = 0;
    const PRIM_SIZE: i32 = 2100;
    const TOP_MARK_CODE: usize = 0;
    const FIRST_MARK_CODE: usize = 1;
    const BOT_MARK_CODE: usize = 2;
    const SPLIT_FIRST_MARK_CODE: usize = 3;
    const SPLIT_BOT_MARK_CODE: usize = 4;
    const INT_VAL: u8 = 0;
    const NORMAL: u8 = 0;
    const CLOSED: u8 = 2;
    const TOO_BIG_LANG: i32 = 256;
    const LEVEL_ZERO_U16: u16 = 0;
    const LEFT_TO_RIGHT: i16 = 0;
    const MARK_VAL: usize = 7;
    const LAST_BOX_CODE: usize = 2;
    const VSPLIT_CODE: usize = 3;
    const HYPH_PRIME: i32 = 607;

    st.shipout.doing_special = false;
    st.scanner.native_text_size = 128;
    st.scanner.native_text = vec![0; 128];

    st.errors.interaction = InteractionMode::ErrorStop;

    st.errors.deletions_allowed = true;
    st.scanner.set_box_allowed = true;
    st.errors.error_count = 0;
    st.errors.help_ptr = 0;
    st.errors.use_err_help = false;

    st.errors.two_to_the[0] = 1;
    for k in 1..=30usize {
        st.errors.two_to_the[k] = 2 * st.errors.two_to_the[k - 1];
    }

    st.errors.spec_log[1] = 93_032_640;
    st.errors.spec_log[2] = 38_612_034;
    st.errors.spec_log[3] = 17_922_280;
    st.errors.spec_log[4] = 8_662_214;
    st.errors.spec_log[5] = 4_261_238;
    st.errors.spec_log[6] = 2_113_709;
    st.errors.spec_log[7] = 1_052_693;
    st.errors.spec_log[8] = 525_315;
    st.errors.spec_log[9] = 262_400;
    st.errors.spec_log[10] = 131_136;
    st.errors.spec_log[11] = 65_552;
    st.errors.spec_log[12] = 32_772;
    st.errors.spec_log[13] = 16_385;
    for k in 14..=27usize {
        st.errors.spec_log[k] = st.errors.two_to_the[27 - k];
    }
    st.errors.spec_log[28] = 1;

    st.input.nest_ptr = 0;
    st.input.max_nest_stack = 0;
    st.input.cur_list.mode = VMODE;
    st.input.cur_list.head.0 = CONTRIB_HEAD;
    st.input.cur_list.tail.0 = CONTRIB_HEAD;
    st.input.cur_list.e_tex_aux = crate::units::MemPtr::NULL;
    st.input.cur_list.set_aux_s1(IGNORE_DEPTH);
    st.input.cur_list.mode_line = 0;
    st.input.cur_list.prev_graf = 0;
    st.scanner.shown_mode = 0;
    st.shipout.page_contents = EMPTY;
    st.shipout.page_tail.0 = PAGE_HEAD;
    st.shipout.last_glue.0 = MAX_HALFWORD;
    st.shipout.last_penalty = 0;
    st.shipout.last_kern = 0;
    st.shipout.last_node_type = -1;
    st.shipout.page_so_far[7] = 0;

    for k in INT_BASE..=EQTB_SIZE {
        st.eqtb.set_xeq_level_at(k, LEVEL_ONE);
    }

    st.eqtb.no_new_control_sequence = true;
    st.eqtb.set_prim_s0(0, 0);
    st.eqtb.set_prim_s1(0, 0);

    for k in 1..=PRIM_SIZE {
        st.eqtb.set_prim_s0(k, 0);
        st.eqtb.set_prim_s1(k, 0);
    }

    st.save_stack.save_ptr = 0;
    st.save_stack.cur_level = LEVEL_ONE;
    st.save_stack.cur_group = GroupCode::BottomLevel;
    st.save_stack.cur_boundary = 0;
    st.save_stack.max_save_stack = 0;
    st.scanner.mag_set = 0;
    st.scanner.expand_depth_count = 0;
    st.scanner.is_in_csname = false;
    st.scanner.cur_mark[TOP_MARK_CODE] = crate::units::MemPtr::NULL;
    st.scanner.cur_mark[FIRST_MARK_CODE] = crate::units::MemPtr::NULL;
    st.scanner.cur_mark[BOT_MARK_CODE] = crate::units::MemPtr::NULL;
    st.scanner.cur_mark[SPLIT_FIRST_MARK_CODE] = crate::units::MemPtr::NULL;
    st.scanner.cur_mark[SPLIT_BOT_MARK_CODE] = crate::units::MemPtr::NULL;
    st.scanner.cur_val = 0;
    st.scanner.cur_val_level = INT_VAL;
    st.scanner.radix = 0;
    st.scanner.cur_order = NORMAL;

    for k in 0..=16usize {
        st.input.read_open[k] = CLOSED;
    }

    st.scanner.cond_ptr.0 = crate::mem::TEX_NULL;
    st.scanner.if_limit = NORMAL;
    st.scanner.cur_if = 0;
    st.scanner.if_line = 0;
    st.fonts.null_character = crate::mem_types::B16x4 {
        s0: 0,
        s1: 0,
        s2: 0,
        s3: 0,
    };
    st.shipout.total_pages = 0;
    st.shipout.max_v = 0;
    st.shipout.max_h = 0;
    st.shipout.max_push = 0;
    st.shipout.last_bop = -1;
    st.shipout.doing_leaders = false;
    st.shipout.dead_cycles = 0;
    st.scanner.adjust_tail.0 = crate::mem::TEX_NULL;
    st.shipout.last_badness = 0;
    st.scanner.pre_adjust_tail.0 = crate::mem::TEX_NULL;
    st.scanner.pack_begin_line = 0;
    st.scanner.empty.s1 = EMPTY as i32;
    st.scanner.empty.s0 = crate::mem::TEX_NULL;
    st.scanner.align_ptr = crate::units::MemPtr::NULL;
    st.scanner.cur_align.0 = crate::mem::TEX_NULL;
    st.scanner.cur_span = crate::units::MemPtr::NULL;
    st.scanner.cur_loop.0 = crate::mem::TEX_NULL;
    st.scanner.cur_head.0 = crate::mem::TEX_NULL;
    st.scanner.cur_tail.0 = crate::mem::TEX_NULL;
    st.scanner.cur_pre_head.0 = crate::mem::TEX_NULL;
    st.scanner.cur_pre_tail.0 = crate::mem::TEX_NULL;
    st.scanner.cur_f = 0;
    st.hyph.max_hyph_char = TOO_BIG_LANG;

    for z in 0..=st.hyph.hyph_size {
        st.hyph.hyph_word[z as usize] = 0;
        st.hyph.hyph_list[z as usize] = crate::mem::TEX_NULL;
        st.hyph.hyph_link[z as usize] = 0;
    }

    st.hyph.hyph_count = 0;
    st.hyph.hyph_next = HYPH_PRIME + 1;
    if st.hyph.hyph_next > st.hyph.hyph_size {
        st.hyph.hyph_next = HYPH_PRIME;
    }

    st.shipout.output_active = false;
    st.shipout.insert_penalties = 0;
    st.mainloop.ligature_present = false;
    st.mainloop.cancel_boundary = false;
    st.mainloop.lft_hit = false;
    st.mainloop.rt_hit = false;
    st.mainloop.ins_disc = false;
    st.scanner.after_token = 0;
    st.errors.long_help_seen = false;
    st.strings.format_ident = 0;

    for k in 0..=17usize {
        st.shipout.write_open[k] = false;
    }

    st.scanner.lr_ptr.0 = crate::mem::TEX_NULL;
    st.scanner.lr_problems = 0;
    st.scanner.cur_dir = LEFT_TO_RIGHT;
    st.input.pseudo_files.0 = crate::mem::TEX_NULL;
    st.scanner.sa_root[MARK_VAL] = crate::units::MemPtr::NULL;
    st.scanner.sa_null = crate::mem_types::MemoryWord {
        b32: crate::mem_types::B32x2 {
            s0: crate::mem::TEX_NULL,
            s1: crate::mem::TEX_NULL,
        },
    };
    st.scanner.sa_chain.0 = crate::mem::TEX_NULL;
    st.scanner.sa_level = LEVEL_ZERO_U16;
    st.scanner.disc_ptr[LAST_BOX_CODE] = crate::units::MemPtr::NULL;
    st.scanner.disc_ptr[VSPLIT_CODE] = crate::units::MemPtr::NULL;
    st.strings.edit_name_start = 0;
    st.scanner.stop_at_space = true;
}
// translated from xetex-ini.c:2212 (initialize_more_initex_variables)
fn initialize_more_initex_variables_in(st: &mut crate::state::EngineState, format_ident: i32) {
    const MEM_TOP: i32 = 4_999_999;
    const PAGE_INS_HEAD: i32 = MEM_TOP;
    const PAGE_HEAD: i32 = MEM_TOP - 2;
    const ACTIVE_LIST: i32 = MEM_TOP - 7;
    const END_SPAN: i32 = MEM_TOP - 9;
    const OMIT_TEMPLATE: i32 = MEM_TOP - 10;
    const PRE_ADJUST_HEAD: i32 = MEM_TOP - 14;

    const NORMAL_U16: u16 = 0;
    const FIL: u16 = 1;
    const FILL: u16 = 2;
    const HYPHENATED: u16 = 1;
    const SPLIT_UP: u16 = 1;
    const HI_MEM_STAT_USAGE: i32 = 15;
    const FROZEN_CONTROL_SEQUENCE: i32 = 2_243_226;
    const FROZEN_END_TEMPLATE: i32 = FROZEN_CONTROL_SEQUENCE + 5;
    const UNDEFINED_CS: u16 = 103;
    const UNDEFINED_CONTROL_SEQUENCE: i32 = 2_254_339;
    const LEVEL_ZERO: u16 = 0;
    const LOCAL_PAR_SHAPE: i32 = 0;
    const LOCAL_OUTPUT_ROUTINE: i32 = 1;
    const NUM_ETEX_PENALTIES: i32 = 4;
    const NUMBER_REGS: i32 = 256;
    const FONT_BASE: i32 = 0;
    const NUMBER_MATH_FAMILIES: i32 = 256;
    const NUMBER_MATH_FONTS: i32 = 3 * NUMBER_MATH_FAMILIES;
    const GLUE_REF: u16 = 119;
    const SHAPE_REF: u16 = 120;
    const BOX_REF: u16 = 121;
    const DATA: u16 = 122;
    const OTHER_CHAR: i32 = 12;
    const LETTER: i32 = 11;
    const CAR_RET: i32 = 5;
    const SPACER: i32 = 10;
    const ESCAPE: i32 = 0;
    const COMMENT: i32 = 14;
    const INVALID_CHAR: i32 = 15;
    const IGNORE: i32 = 9;
    const VAR_FAM_CLASS: i32 = 7;
    const CARRIAGE_RETURN: i32 = 13;
    const INT_PAR_MAG: i32 = 17;
    const INT_PAR_TOLERANCE: i32 = 1;
    const INT_PAR_HANG_AFTER: i32 = 41;
    const INT_PAR_MAX_DEAD_CYCLES: i32 = 40;
    const INT_PAR_ESCAPE_CHAR: i32 = 45;
    const INT_PAR_END_LINE_CHAR: i32 = 48;
    const INT_PAR_XETEX_HYPHENATABLE_LENGTH: i32 = 80;
    const PRIM_SIZE: i32 = 2100;
    const TRIE_OP_SIZE_I32: i32 = 35111;
    const BIGGEST_LANG: usize = 255;
    const MIN_TRIE_OP_U16: u16 = 0;
    const INT_VAL: usize = 0;
    const INTER_CHAR_VAL: usize = 6;

    // set_family(x) = (x & 0xFF) << 24; set_class(x) = (x & 0x07) << 21
    fn set_family(x: i32) -> i32 {
        (x & 0xFF) << 24
    }
    fn set_class(x: i32) -> i32 {
        (x & 0x07) << 21
    }

    for k in 1..=19 {
        st.mem.set_b32s1(k, 0);
    }

    let mut k = 0;
    while k <= 19 {
        st.mem.set_b32s1(k, crate::mem::TEX_NULL + 1);
        st.mem.set_b16s1(k, NORMAL_U16);
        st.mem.set_b16s0(k, NORMAL_U16);
        k += 4;
    }

    st.mem.set_b32s1(6, 65536);
    st.mem.set_b16s1(4, FIL);
    st.mem.set_b32s1(10, 65536);
    st.mem.set_b16s1(8, FILL);
    st.mem.set_b32s1(14, 65536);
    st.mem.set_b16s1(12, FIL);
    st.mem.set_b32s1(15, 65536);
    st.mem.set_b16s0(12, FIL);
    st.mem.set_b32s1(18, -65536);
    st.mem.set_b16s1(16, FIL);
    st.mem.rover = 20;
    st.mem.set_b32s1(st.mem.rover, MAX_HALFWORD);
    st.mem.set_b32s0(st.mem.rover, 1000);
    st.mem.set_b32s0(st.mem.rover + 1, st.mem.rover);
    st.mem.set_b32s1(st.mem.rover + 1, st.mem.rover);
    st.mem.lo_mem_max = st.mem.rover + 1000;
    st.mem.set_b32s1(st.mem.lo_mem_max, crate::mem::TEX_NULL);
    st.mem.set_b32s0(st.mem.lo_mem_max, crate::mem::TEX_NULL);

    let template = st.mem.word(st.mem.lo_mem_max);
    for k in PRE_ADJUST_HEAD..=MEM_TOP {
        st.mem.set_word(k, template);
    }

    st.mem
        .set_b32s0(OMIT_TEMPLATE, CS_TOKEN_FLAG + FROZEN_END_TEMPLATE);
    st.mem.set_b32s1(END_SPAN, u16::MAX as i32 + 1);
    st.mem.set_b32s0(END_SPAN, crate::mem::TEX_NULL);
    st.mem.set_b16s1(ACTIVE_LIST, HYPHENATED);
    st.mem.set_b32s0(ACTIVE_LIST + 1, MAX_HALFWORD);
    st.mem.set_b16s0(ACTIVE_LIST, 0);
    st.mem.set_b16s0(PAGE_INS_HEAD, 255);
    st.mem.set_b16s1(PAGE_INS_HEAD, SPLIT_UP);
    st.mem.set_b32s1(PAGE_INS_HEAD, PAGE_INS_HEAD);
    st.mem.set_b16s1(PAGE_HEAD, GLUE_NODE);
    st.mem.set_b16s0(PAGE_HEAD, NORMAL_U16);
    st.mem.avail = crate::mem::TEX_NULL;
    st.mem.mem_end = MEM_TOP;
    st.mem.hi_mem_min = PRE_ADJUST_HEAD;
    st.mem.var_used = 20;
    st.mem.dyn_used = HI_MEM_STAT_USAGE;

    st.eqtb.set_b16s1(UNDEFINED_CONTROL_SEQUENCE, UNDEFINED_CS);
    st.eqtb
        .set_b32s1(UNDEFINED_CONTROL_SEQUENCE, crate::mem::TEX_NULL);
    st.eqtb.set_b16s0(UNDEFINED_CONTROL_SEQUENCE, LEVEL_ZERO);

    let undefined_entry = st.eqtb.word(UNDEFINED_CONTROL_SEQUENCE);
    for k in ACTIVE_BASE..=st.eqtb.eqtb_top {
        st.eqtb.data[k as usize] = undefined_entry;
    }

    st.eqtb.set_b32s1(GLUE_BASE, 0);
    st.eqtb.set_b16s0(GLUE_BASE, LEVEL_ONE);
    st.eqtb.set_b16s1(GLUE_BASE, GLUE_REF);

    let glue_base_entry = st.eqtb.word(GLUE_BASE);
    for k in (GLUE_BASE + 1)..=(LOCAL_BASE - 1) {
        st.eqtb.data[k as usize] = glue_base_entry;
    }

    st.mem.set_b32s1(0, st.mem.b32s1(0) + 531);
    st.eqtb
        .set_b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE, crate::mem::TEX_NULL);
    st.eqtb.set_b16s1(LOCAL_BASE + LOCAL_PAR_SHAPE, SHAPE_REF);
    st.eqtb.set_b16s0(LOCAL_BASE + LOCAL_PAR_SHAPE, LEVEL_ONE);

    let par_shape_entry = st.eqtb.word(LOCAL_BASE + LOCAL_PAR_SHAPE);
    for k in ETEX_PEN_BASE..=(NUM_ETEX_PENALTIES - 1) {
        st.eqtb.data[k as usize] = par_shape_entry;
    }

    for k in (LOCAL_BASE + LOCAL_OUTPUT_ROUTINE)..=(TOKS_BASE + NUMBER_REGS - 1) {
        st.eqtb.data[k as usize] = undefined_entry;
    }

    st.eqtb.set_b32s1(BOX_BASE, crate::mem::TEX_NULL);
    st.eqtb.set_b16s1(BOX_BASE, BOX_REF);
    st.eqtb.set_b16s0(BOX_BASE, LEVEL_ONE);

    let box_base_entry = st.eqtb.word(BOX_BASE);
    for k in (BOX_BASE + 1)..=(BOX_BASE + NUMBER_REGS - 1) {
        st.eqtb.data[k as usize] = box_base_entry;
    }

    st.eqtb.set_b32s1(CUR_FONT_LOC, FONT_BASE);
    st.eqtb.set_b16s1(CUR_FONT_LOC, DATA);
    st.eqtb.set_b16s0(CUR_FONT_LOC, LEVEL_ONE);

    let cur_font_entry = st.eqtb.word(CUR_FONT_LOC);
    for k in MATH_FONT_BASE..=(MATH_FONT_BASE + NUMBER_MATH_FONTS - 1) {
        st.eqtb.data[k as usize] = cur_font_entry;
    }

    st.eqtb.set_b32s1(CAT_CODE_BASE, 0);
    st.eqtb.set_b16s1(CAT_CODE_BASE, DATA);
    st.eqtb.set_b16s0(CAT_CODE_BASE, LEVEL_ONE);

    let cat_code_entry = st.eqtb.word(CAT_CODE_BASE);
    for k in (CAT_CODE_BASE + 1)..=(INT_BASE - 1) {
        st.eqtb.data[k as usize] = cat_code_entry;
    }

    for k in 0..NUMBER_USVS {
        st.eqtb.set_b32s1(CAT_CODE_BASE + k, OTHER_CHAR);
        st.eqtb.set_b32s1(MATH_CODE_BASE + k, k);
        st.eqtb.set_b32s1(SF_CODE_BASE + k, 1000);
    }

    st.eqtb.set_b32s1(CAT_CODE_BASE + 13, CAR_RET);
    st.eqtb.set_b32s1(CAT_CODE_BASE + 32, SPACER);
    st.eqtb.set_b32s1(CAT_CODE_BASE + 92, ESCAPE);
    st.eqtb.set_b32s1(CAT_CODE_BASE + 37, COMMENT);
    st.eqtb.set_b32s1(CAT_CODE_BASE + 127, INVALID_CHAR);
    st.eqtb.set_b32s1(CAT_CODE_BASE, IGNORE);

    for k in (b'0' as i32)..=(b'9' as i32) {
        st.eqtb
            .set_b32s1(MATH_CODE_BASE + k, k + set_class(VAR_FAM_CLASS));
    }

    for k in (b'A' as i32)..=(b'Z' as i32) {
        st.eqtb.set_b32s1(CAT_CODE_BASE + k, LETTER);
        st.eqtb.set_b32s1(CAT_CODE_BASE + k + 32, LETTER);
        st.eqtb.set_b32s1(
            MATH_CODE_BASE + k,
            k + set_family(1) + set_class(VAR_FAM_CLASS),
        );
        st.eqtb.set_b32s1(
            MATH_CODE_BASE + k + 32,
            k + 32 + set_family(1) + set_class(VAR_FAM_CLASS),
        );
        st.eqtb.set_b32s1(LC_CODE_BASE + k, k + 32);
        st.eqtb.set_b32s1(LC_CODE_BASE + k + 32, k + 32);
        st.eqtb.set_b32s1(UC_CODE_BASE + k, k);
        st.eqtb.set_b32s1(UC_CODE_BASE + k + 32, k);
        st.eqtb.set_b32s1(SF_CODE_BASE + k, 999);
    }

    for k in INT_BASE..=(DEL_CODE_BASE - 1) {
        st.eqtb.set_b32s1(k, 0);
    }

    st.eqtb.set_b32s1(INT_BASE + INT_PAR_MAG, 1000);
    st.eqtb.set_b32s1(INT_BASE + INT_PAR_TOLERANCE, 10000);
    st.eqtb.set_b32s1(INT_BASE + INT_PAR_HANG_AFTER, 1);
    st.eqtb.set_b32s1(INT_BASE + INT_PAR_MAX_DEAD_CYCLES, 25);
    st.eqtb
        .set_b32s1(INT_BASE + INT_PAR_ESCAPE_CHAR, b'\\' as i32);
    st.eqtb
        .set_b32s1(INT_BASE + INT_PAR_END_LINE_CHAR, CARRIAGE_RETURN);

    for k in 0..NUMBER_USVS {
        st.eqtb.set_b32s1(DEL_CODE_BASE + k, -1);
    }

    st.eqtb.set_b32s1(DEL_CODE_BASE + 46, 0);

    for k in DIMEN_BASE..=EQTB_SIZE {
        st.eqtb.set_b32s1(k, 0);
    }

    st.eqtb.prim_used = PRIM_SIZE;
    st.eqtb.hash_used = FROZEN_CONTROL_SEQUENCE;
    st.eqtb.hash_high = 0;
    st.eqtb.cs_count = 0;

    for k in (-TRIE_OP_SIZE_I32)..=TRIE_OP_SIZE_I32 {
        st.hyph.trie_op_hash_array[(k + TRIE_OP_SIZE_I32) as usize] = 0;
    }

    for k in 0..=BIGGEST_LANG {
        st.hyph.trie_used[k] = MIN_TRIE_OP_U16;
    }

    st.hyph.max_op_used = MIN_TRIE_OP_U16;
    st.hyph.trie_op_ptr = 0;
    st.hyph.trie_not_ready = true;

    st.strings.format_ident = format_ident;

    st.scanner.max_reg_num = 32767;
    st.scanner.max_reg_help_line = c"A register number must be between 0 and 32767.".as_ptr();

    for i in INT_VAL..=INTER_CHAR_VAL {
        st.scanner.sa_root[i] = crate::units::MemPtr::NULL;
    }

    st.eqtb
        .set_b32s1(INT_BASE + INT_PAR_XETEX_HYPHENATABLE_LENGTH, 63);
}
// translated from xetex-ini.c:2215 (initialize_primitives)
unsafe fn initialize_primitives(st: &mut EngineState) -> Result<(), PrimitiveInitAbort> {
    const FROZEN_CONTROL_SEQUENCE: i32 = 2_243_226;
    const FROZEN_PROTECTION: i32 = FROZEN_CONTROL_SEQUENCE;
    const FROZEN_END_TEMPLATE: i32 = FROZEN_CONTROL_SEQUENCE + 5;
    const FROZEN_ENDV: i32 = FROZEN_CONTROL_SEQUENCE + 6;
    const END_WRITE: i32 = FROZEN_CONTROL_SEQUENCE + 8;
    const FROZEN_DONT_EXPAND: i32 = FROZEN_CONTROL_SEQUENCE + 9;
    const FROZEN_PRIMITIVE: i32 = FROZEN_CONTROL_SEQUENCE + 11;

    const ENDV: u16 = 9;
    const IGNORE_SPACES: u16 = 39;
    const OUTER_CALL: u16 = 115;
    const END_TEMPLATE: u16 = 117;
    const DONT_EXPAND: u16 = 118;

    const MEM_TOP: i32 = 4_999_999;
    const NULL_LIST: i32 = MEM_TOP - 11;

    const XF_PRIM_INIT_NONE: i32 = 0;
    const XF_PRIM_INIT_PAR: i32 = 1;
    const XF_PRIM_INIT_WRITE: i32 = 2;

    st.eqtb.no_new_control_sequence = false;
    st.input.first = 0;

    for prim in crate::primitives::XETEX_PRIMITIVES.iter() {
        if let Some(name) = prim.name {
            primitive(st, name.as_ptr(), prim.cmd as u16, prim.chr)?;

            match prim.extra_init {
                XF_PRIM_INIT_NONE => {}
                XF_PRIM_INIT_PAR => {
                    st.input.par_loc = st.scanner.cur_val;
                    st.input.par_token = CS_TOKEN_FLAG + st.input.par_loc;
                }
                XF_PRIM_INIT_WRITE => {
                    st.shipout.write_loc = st.scanner.cur_val;
                }
                frozen => {
                    st.eqtb.set_hash_s1(
                        frozen,
                        crate::texmfmp::maketexstring_in(&mut st.strings, name.as_ptr())?,
                    );
                    st.eqtb.data[frozen as usize] = st.eqtb.word(st.scanner.cur_val);
                }
            }
        }
    }

    st.eqtb.set_hash_s1(
        FROZEN_END_TEMPLATE,
        crate::texmfmp::maketexstring_in(&mut st.strings, c"endtemplate".as_ptr())?,
    );
    st.eqtb.set_b16s1(FROZEN_END_TEMPLATE, END_TEMPLATE);
    st.eqtb.set_b32s1(FROZEN_END_TEMPLATE, NULL_LIST);
    st.eqtb.set_b16s0(FROZEN_END_TEMPLATE, LEVEL_ONE);

    st.eqtb.set_hash_s1(
        FROZEN_ENDV,
        crate::texmfmp::maketexstring_in(&mut st.strings, c"endtemplate".as_ptr())?,
    );
    st.eqtb.set_b16s1(FROZEN_ENDV, ENDV);
    st.eqtb.set_b32s1(FROZEN_ENDV, NULL_LIST);
    st.eqtb.set_b16s0(FROZEN_ENDV, LEVEL_ONE);

    st.eqtb.set_hash_s1(
        FROZEN_DONT_EXPAND,
        crate::texmfmp::maketexstring_in(&mut st.strings, c"notexpanded:".as_ptr())?,
    );
    st.eqtb.set_b16s1(FROZEN_DONT_EXPAND, DONT_EXPAND);

    st.eqtb.set_hash_s1(
        FROZEN_PRIMITIVE,
        crate::texmfmp::maketexstring_in(&mut st.strings, c"primitive".as_ptr())?,
    );
    st.eqtb.set_b16s1(FROZEN_PRIMITIVE, IGNORE_SPACES);
    st.eqtb.set_b32s1(FROZEN_PRIMITIVE, 1);
    st.eqtb.set_b16s0(FROZEN_PRIMITIVE, LEVEL_ONE);

    st.eqtb.set_hash_s1(
        FROZEN_PROTECTION,
        crate::texmfmp::maketexstring_in(&mut st.strings, c"inaccessible".as_ptr())?,
    );

    st.eqtb.set_hash_s1(
        END_WRITE,
        crate::texmfmp::maketexstring_in(&mut st.strings, c"endwrite".as_ptr())?,
    );
    st.eqtb.set_b16s0(END_WRITE, LEVEL_ONE);
    st.eqtb.set_b16s1(END_WRITE, OUTER_CALL);
    st.eqtb.set_b32s1(END_WRITE, crate::mem::TEX_NULL);

    st.eqtb.no_new_control_sequence = true;
    Ok(())
}
// translated from xetex-ini.c:1474 (tt_run_engine)
#[no_mangle]
/// Engine entry point (called from the C bridge `tt_engine_xetex_main`).
///
/// Wraps the run in [`catch_unwind`] so a Rust panic — out-of-memory from the
/// allocators, or an internal invariant violation — is turned into a
/// `HISTORY_FATAL_ERROR` return with a surfaced message, instead of unwinding
/// across the `extern "C"` boundary (which is UB). This is the Rust-native
/// replacement for the C bridge's setjmp/longjmp fatal bailout.
///
/// [`catch_unwind`]: std::panic::catch_unwind
pub unsafe extern "C" fn tt_run_engine(
    api: *mut libc::c_void,
    dump_name: *const libc::c_char,
    input_file_name: *const libc::c_char,
    build_date: libc::time_t,
) -> i32 {
    // Clear slot-keyed state before the operation and on every exit, including
    // a caught Rust panic.
    reset_layout_font_caches();
    struct LayoutCacheGuard;
    impl Drop for LayoutCacheGuard {
        fn drop(&mut self) {
            reset_layout_font_caches();
        }
    }
    let _layout_cache_guard = LayoutCacheGuard;

    // Install the bridge-state locator for the run; cleared on every exit
    // path (including panics) so it can't dangle between runs.
    crate::bridge::set_bridge_state(api);
    struct BridgeGuard;
    impl Drop for BridgeGuard {
        fn drop(&mut self) {
            unsafe { crate::bridge::set_bridge_state(std::ptr::null_mut()) };
        }
    }
    let _bridge_guard = BridgeGuard;

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tt_run_engine_inner(dump_name, input_file_name, build_date)
    }));
    match outcome {
        Ok(rv) => rv,
        Err(payload) => {
            let msg = payload
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                .unwrap_or("internal engine panic");
            if let Ok(cmsg) = std::ffi::CString::new(msg) {
                _ttbc_set_error_message(cmsg.as_ptr());
            }
            HISTORY_FATAL_ERROR
        }
    }
}

unsafe fn tt_run_engine_inner(
    dump_name: *const libc::c_char,
    input_file_name: *const libc::c_char,
    build_date: libc::time_t,
) -> i32 {
    // Own the engine state here, heap-boxed so the ~KB struct isn't a fresh
    // stack frame. The host set config (halt_on_error_p/in_initex_mode/…) via
    // `tt_xetex_set_int_variable` *before* this call; copy its config into the
    // owned state before installing the owner pointer.
    let mut st = Box::new(crate::state::EngineState::default());
    crate::engine_interface::apply_config(&mut st.config);
    // Clears the owner pointer on every exit path so a stale pointer to the
    // (dropped) `st` never outlives the run (e.g. a between-runs
    // `tt_xetex_set_int_variable` must hit the fallback, not freed memory).
    struct PtrGuard;
    impl Drop for PtrGuard {
        fn drop(&mut self) {
            crate::state::set_engine_state_ptr(std::ptr::null_mut());
        }
    }
    crate::state::set_engine_state_ptr(&mut *st as *mut crate::state::EngineState);
    let _ptr_guard = PtrGuard;

    match initialize_engine(&mut *st, dump_name, build_date) {
        InitOutcome::Ready => {}
        InitOutcome::Terminate(history) => return history,
    }
    run_document_to_end(&mut *st, input_file_name)
}

/// Result of initializing an engine before any primary document is opened.
enum InitOutcome {
    Ready,
    Terminate(i32),
}

/// Allocate engine storage, load the format, and initialize configuration.
///
/// On success the state is ready to open a primary input, but owns no document
/// input stack. Keeping this boundary explicit lets batch and future resident
/// sessions share the same initialization path.
unsafe fn initialize_engine(
    st: &mut crate::state::EngineState,
    dump_name: *const libc::c_char,
    build_date: libc::time_t,
) -> InitOutcome {

    TEX_FORMAT_DEFAULT = xmalloc(libc::strlen(dump_name) + 1) as *mut libc::c_char;
    libc::strcpy(TEX_FORMAT_DEFAULT, dump_name);
    FORMAT_DEFAULT_LENGTH = libc::strlen(dump_name) as i32;
    st.shipout.rust_stdout = ttstub_output_open_stdout();

    if st.config.file_line_error_style_p < 0 {
        st.config.file_line_error_style_p = 0;
    }

    TRE_POOL_SIZE = 6_250_000;
    TRE_STRING_VACANCIES = 90_000;
    POOL_FREE = 47_500;
    TRE_MAX_STRINGS = 565_536;
    STRINGS_FREE = 100;
    st.fonts.font_mem_size = 8_000_000;
    st.fonts.font_max = 9_000;
    TRE_TRIE_SIZE = 1_000_000;
    TRE_HYPH_SIZE = 8_191;
    st.input.buf_size = 200_000;
    st.input.nest_size = 500;
    st.input.max_in_open = 15;
    st.input.param_size = 10_000;
    st.save_stack.save_size = 80_000;
    st.input.stack_size = 5_000;
    st.errors.error_line = 79;
    st.errors.half_error_line = 50;
    st.config.max_print_line = 79;
    st.scanner.expand_depth = 10_000;

    // Allocate every Rust-owned per-run arena. All sizes (`*_size + 1`, matching
    // the former `xmalloc_array` shapes) are precomputed from the `tre_*` locals.
    // Allocated fresh each run; released by `tt_cleanup`; never reallocated
    // during a run (overflow errors instead of growing).
    let buf_words = (st.input.buf_size as usize) + 1;
    let nest_words = (st.input.nest_size as usize) + 1;
    let save_words = (st.save_stack.save_size as usize) + 1;
    let input_file_words = (st.input.max_in_open as usize) + 1;
    let stack_words = (st.input.stack_size as usize) + 1;
    let in_open_words = (st.input.max_in_open as usize) + 1;
    let param_words = (st.input.param_size as usize) + 1;
    let hyph_words = (TRE_HYPH_SIZE as usize) + 1;
    {
        let state = &mut *st;
        state.eqtb.hash_extra = 600_000;
        state.strings.pool_size = TRE_POOL_SIZE;
        state.strings.string_vacancies = TRE_STRING_VACANCIES;
        state.strings.max_strings = TRE_MAX_STRINGS;
        state.hyph.trie_size = TRE_TRIE_SIZE;
        state.hyph.hyph_size = TRE_HYPH_SIZE;

        state.input.buffer = vec![0i32; buf_words];
        state.input.nest = vec![
            crate::mem_types::ListStateRecord {
                mode: 0,
                head: crate::units::MemPtr::from_raw(0),
                tail: crate::units::MemPtr::from_raw(0),
                e_tex_aux: crate::units::MemPtr::from_raw(0),
                prev_graf: 0,
                mode_line: 0,
                aux: crate::mem_types::MemoryWord {
                    b32: crate::mem_types::B32x2 { s0: 0, s1: 0 },
                },
            };
            nest_words
        ];
        state.save_stack.data = crate::mem_types::zeroed_vec(save_words);
        state.input.input_file = std::iter::repeat_with(|| crate::state::InputSlot::Empty)
            .take(input_file_words)
            .collect();
        state.input.input_stack = vec![
            crate::globals::InputState {
                state: 0,
                index: 0,
                start: 0,
                loc: 0,
                limit: 0,
                name: 0,
                synctex_tag: 0,
            };
            stack_words
        ];
        state.input.line_stack = vec![0i32; in_open_words];
        state.input.grp_stack = vec![0i32; in_open_words];
        state.input.if_stack = vec![crate::units::MemPtr::from_raw(0); in_open_words];
        state.input.source_filename_stack = vec![0i32; in_open_words];
        state.input.full_source_filename_stack = vec![0i32; in_open_words];
        state.input.eof_seen = vec![false; in_open_words];
        state.input.param_stack = vec![0i32; param_words];
        state.hyph.hyph_word = vec![0i32; hyph_words];
        state.hyph.hyph_list = vec![0i32; hyph_words];
        state.hyph.hyph_link = vec![0u16; hyph_words];
    }

    if st.config.in_initex_mode {
        // `xmalloc_array(MEM_TOP + 1)` allocated `MEM_TOP + 2` words; match it.
        let mem_words_init = (MEM_TOP as usize) + 2;
        {
            let s = &mut *st;
            s.mem.data = crate::mem_types::zeroed_vec(mem_words_init);

            s.eqtb.eqtb_top = EQTB_SIZE + s.eqtb.hash_extra;
            s.eqtb.hash_top = if s.eqtb.hash_extra == 0 {
                HASH_TOP_UNDEFINED_CONTROL_SEQUENCE
            } else {
                s.eqtb.eqtb_top
            };

            let n = (1 + s.eqtb.hash_top - 514) as usize;
            s.eqtb.hash = crate::mem_types::zeroed_vec(n);
        }

        st.eqtb.hash[(HASH_BASE as usize) - 514].s0 = 0;
        st.eqtb.hash[(HASH_BASE as usize) - 514].s1 = 0;
        let hash_base = st.eqtb.hash[(HASH_BASE as usize) - 514].s0;
        let hash_base_s1 = st.eqtb.hash[(HASH_BASE as usize) - 514].s1;
        let mut h = HASH_BASE + 1;
        let h_top = st.eqtb.hash_top;
        while h <= h_top {
            st.eqtb.hash[(h as usize) - 514].s0 = hash_base;
            st.eqtb.hash[(h as usize) - 514].s1 = hash_base_s1;
            h += 1;
        }
        st.eqtb.hash_used = h_top;

        let fms_initex = st.fonts.font_mem_size;
        // `xcalloc_array(n)` allocated `n + 1` zeroed words; match that.
        let eqtb_words_initex = (st.eqtb.eqtb_top as usize) + 1;
        {
            let state = &mut *st;
            state.eqtb.data = crate::mem_types::zeroed_vec(eqtb_words_initex);
            state.strings.str_start = vec![0i32; (TRE_MAX_STRINGS as usize) + 1];
            state.strings.str_pool = vec![0u16; (TRE_POOL_SIZE as usize) + 1];
            state.fonts.font_info = crate::mem_types::zeroed_vec((fms_initex as usize) + 1);
        }
    }

    st.errors.history = HISTORY_FATAL_ERROR;
    BAD = 0;
    if st.errors.half_error_line < 30 || st.errors.half_error_line > st.errors.error_line - 15 {
        BAD = 1;
    }
    if st.config.max_print_line < 60 {
        BAD = 2;
    }
    if 1100 > MEM_TOP {
        BAD = 4;
    }
    if HASH_PRIME > HASH_SIZE {
        BAD = 5;
    }
    if st.input.max_in_open >= 128 {
        BAD = 6;
    }
    if MEM_TOP < 267 {
        BAD = 7;
    }
    if crate::mem::TEX_NULL > 0 {
        BAD = 12;
    }
    if MAX_FONT_MAX < crate::mem::TEX_NULL || MAX_FONT_MAX > MAX_HALFWORD {
        BAD = 15;
    }
    if st.fonts.font_max > FONT_BASE + 9000 {
        BAD = 16;
    }
    if st.save_stack.save_size > MAX_HALFWORD || TRE_MAX_STRINGS > MAX_HALFWORD {
        BAD = 17;
    }
    if st.input.buf_size > MAX_HALFWORD {
        BAD = 18;
    }
    if CS_TOKEN_FLAG + EQTB_SIZE + st.eqtb.hash_extra > MAX_HALFWORD {
        BAD = 21;
    }
    if FORMAT_HASH_OFFSET < 0 || FORMAT_HASH_OFFSET > HASH_BASE {
        BAD = 42;
    }
    if FORMAT_DEFAULT_LENGTH > i32::MAX {
        BAD = 31;
    }
    if 2 * MAX_HALFWORD < MEM_TOP {
        BAD = 41;
    }
    if BAD > 0 {
        let mut msg = [0 as libc::c_char; 80];
        libc::snprintf(
            msg.as_mut_ptr(),
            msg.len(),
            c"failed internal consistency check #%d".as_ptr(),
            BAD,
        );
        // Consistency check failed: terminate with HISTORY_FATAL_ERROR.
        return InitOutcome::Terminate(
            crate::errors::EngineAbort::TtAbort(msg.as_ptr()).into_history(&mut *st),
        );
    }

    initialize_more_variables_in(&mut *st);
    if st.config.in_initex_mode {
        if let Err(a) = get_strings_started_in(&mut *st) {
            return InitOutcome::Terminate(a.into_history(&mut *st));
        }
        let format_ident = match crate::texmfmp::maketexstring_in(&mut st.strings, c" (INITEX)".as_ptr())
            .map_err(|e| e.into_engine_abort())
        {
            Ok(v) => v,
            Err(a) => return InitOutcome::Terminate(a.into_history(&mut *st)),
        };
        initialize_more_initex_variables_in(&mut *st, format_ident);
        let result = initialize_primitives(&mut *st);
        if let Err(e) = result {
            return InitOutcome::Terminate(e.into_engine_abort().into_history(&mut *st));
        }
        {
            let state = &mut *st;
            state.strings.init_str_ptr = state.strings.str_ptr;
            state.strings.init_pool_ptr = state.strings.pool_ptr;
        }
    }

    crate::math::initialize_math_variables();
    crate::pagebuilder::initialize_pagebuilder_variables(&mut *st);
    crate::shipout::initialize_shipout_variables(&mut *st);
    if st.config.deterministic_mode {
        st.shipout.epochseconds = build_date as i32;
        st.shipout.microseconds = 0;
    } else {
        let mut epoch = 0;
        let mut micros = 0;
        crate::texmfmp::get_seconds_and_micros(&mut epoch, &mut micros);
        st.shipout.epochseconds = epoch;
        st.shipout.microseconds = micros;
    }
    crate::texmfmp::init_start_time(&mut *st, build_date);

    st.shipout.selector = SELECTOR_TERM_ONLY;
    st.errors.tally = 0;
    st.shipout.term_offset = 0;
    st.shipout.file_offset = 0;
    st.strings.job_name = 0;
    st.scanner.name_in_progress = false;
    st.errors.log_opened = false;
    OUTPUT_FILE_EXTENSION = if st.config.semantic_pagination_enabled {
        c".spx".as_ptr()
    } else {
        c".xdv".as_ptr()
    };

    st.input.input_ptr = 0;
    st.input.max_in_stack = 0;
    st.input.source_filename_stack[0] = 0;
    st.input.full_source_filename_stack[0] = 0;
    st.input.in_open = 0;
    st.input.open_parens = 0;
    st.input.max_buf_stack = 0;
    st.input.grp_stack[0] = 0;
    st.input.if_stack[0] = crate::units::MemPtr::NULL;
    st.input.param_ptr = 0;
    st.input.max_param_stack = 0;
    st.scanner.used_tectonic_coda_tokens = false;
    st.fonts.gave_char_warning_help = false;
    st.input.first = 0;

    st.scanner.scanner_status = NORMAL as u8;
    st.scanner.warning_index = crate::mem::TEX_NULL;
    st.input.first = 1;
    st.input.cur_input.state = NEW_LINE;
    st.input.cur_input.start = 1;
    st.input.cur_input.index = 0;
    st.mem.line = 0;
    st.input.cur_input.name = 0;
    st.input.force_eof = false;
    st.scanner.align_state = 1_000_000;

    init_io(&mut *st);

    if st.config.in_initex_mode {
        st.eqtb.no_new_control_sequence = false;
        st.scanner.max_reg_num = 32767;
        st.scanner.max_reg_help_line = c"A register number must be between 0 and 32767.".as_ptr();
    }
    st.eqtb.no_new_control_sequence = true;

    if !st.config.in_initex_mode {
        match load_fmt_file(&mut *st) {
            Ok(true) => {}
            // Graceful load failure: terminate with the current history.
            Ok(false) => return InitOutcome::Terminate(st.errors.history),
            // Fatal undump abort: HISTORY_FATAL_ERROR.
            Err(a) => return InitOutcome::Terminate(a.into_history(&mut *st)),
        }
    }

    // SAFETY: union projection; the eqtb index is a valid int-param slot.
    let end_line_char = unsafe { st.eqtb.data[(PC_INT_BASE + INT_PAR_END_LINE_CHAR) as usize].b32.s1 };
    if end_line_char < 0 || end_line_char > BIGGEST_CHAR {
        st.input.cur_input.limit = st.input.cur_input.limit - 1;
    } else {
        st.input.buffer[st.input.cur_input.limit as usize] = end_line_char;
    }

    if st.config.in_initex_mode {
        st.eqtb.data[(PC_INT_BASE + INT_PAR_TIME) as usize].b32.s1 = 0;
        st.eqtb.data[(PC_INT_BASE + INT_PAR_DAY) as usize].b32.s1 = 0;
        st.eqtb.data[(PC_INT_BASE + INT_PAR_MONTH) as usize].b32.s1 = 0;
        st.eqtb.data[(PC_INT_BASE + INT_PAR_YEAR) as usize].b32.s1 = 0;
    } else {
        let mut time = 0;
        let mut day = 0;
        let mut month = 0;
        let mut year = 0;
        crate::texmfmp::get_date_and_time(
            build_date,
            st.config.deterministic_mode,
            &mut time,
            &mut day,
            &mut month,
            &mut year,
        );
        st.eqtb.data[(PC_INT_BASE + INT_PAR_TIME) as usize].b32.s1 = time;
        st.eqtb.data[(PC_INT_BASE + INT_PAR_DAY) as usize].b32.s1 = day;
        st.eqtb.data[(PC_INT_BASE + INT_PAR_MONTH) as usize].b32.s1 = month;
        st.eqtb.data[(PC_INT_BASE + INT_PAR_YEAR) as usize].b32.s1 = year;
    }

    if st.hyph.trie_not_ready {
        {
            let state = &mut *st;
            let trie_len = (state.hyph.trie_size as usize) + 1;
            state.hyph.trie_trl = vec![0i32; trie_len];
            state.hyph.trie_tro = vec![0i32; trie_len];
            state.hyph.trie_trc = vec![0u16; trie_len];
            state.hyph.trie_c = vec![0u16; trie_len];
            state.hyph.trie_o = vec![0u16; trie_len];
            state.hyph.trie_l = vec![0i32; trie_len];
            state.hyph.trie_r = vec![0i32; trie_len];
            state.hyph.trie_hash = vec![0i32; trie_len];
            state.hyph.trie_taken = vec![false; trie_len];
            state.hyph.trie_l[0] = 0;
            state.hyph.trie_c[0] = 0;
            state.hyph.trie_ptr = 0;
            state.hyph.trie_r[0] = 0;
            state.hyph.hyph_start = 0;
        }

        let font_check_len = (st.fonts.font_max as usize) + 1;
        let base_fmt_len = (st.fonts.font_max as usize) + 1;
        {
            let state = &mut *st;
            state.fonts.font_layout_engine = vec![None; font_check_len];
            state.fonts.font_layout_engine_mirror = vec![std::ptr::null_mut(); font_check_len];
            state.fonts.font_mapping = vec![None; font_check_len];
            state.fonts.font_flags = vec![0i8; font_check_len];
            state.fonts.font_letter_space = vec![0i32; font_check_len];
            state.fonts.font_check = vec![
                crate::mem_types::B16x4 {
                    s0: 0,
                    s1: 0,
                    s2: 0,
                    s3: 0,
                };
                font_check_len
            ];
            state.fonts.font_size = vec![0i32; font_check_len];
            state.fonts.font_dsize = vec![0i32; font_check_len];
            state.fonts.font_params = vec![0i32; font_check_len];
            state.fonts.font_name = vec![0i32; font_check_len];
            state.fonts.font_area = vec![0i32; font_check_len];
            state.fonts.font_bc = vec![0u16; font_check_len];
            state.fonts.font_ec = vec![0u16; font_check_len];
            state.fonts.bchar_label = vec![0i32; font_check_len];
            state.fonts.font_bchar = vec![0i32; font_check_len];
            state.fonts.font_false_bchar = vec![0i32; font_check_len];
            state.fonts.font_glue = vec![0i32; font_check_len];

            state.fonts.hyphen_char = vec![0i32; base_fmt_len];
            state.fonts.skew_char = vec![0i32; base_fmt_len];
            state.fonts.char_base = vec![0i32; base_fmt_len];
            state.fonts.width_base = vec![0i32; base_fmt_len];
            state.fonts.height_base = vec![0i32; base_fmt_len];
            state.fonts.depth_base = vec![0i32; base_fmt_len];
            state.fonts.italic_base = vec![0i32; base_fmt_len];
            state.fonts.lig_kern_base = vec![0i32; base_fmt_len];
            state.fonts.kern_base = vec![0i32; base_fmt_len];
            state.fonts.exten_base = vec![0i32; base_fmt_len];
            state.fonts.param_base = vec![0i32; base_fmt_len];
            crate::globals::refresh_font_ffi_mirrors(&state.fonts);
        }

        // The nullfont (FONT_BASE) bootstrap is a contiguous run of font-table
        // writes.
        let nullfont_name = match crate::texmfmp::maketexstring_in(&mut st.strings, c"nullfont".as_ptr())
            .map_err(|e| e.into_engine_abort())
        {
            Ok(v) => v,
            Err(a) => return InitOutcome::Terminate(a.into_history(&mut *st)),
        };
        {
            let s = &mut *st;
            let f = FONT_BASE as usize;
            s.fonts.font_ptr = FONT_BASE;
            s.fonts.fmem_ptr = 7;
            s.fonts.font_name[f] = nullfont_name;
            s.fonts.font_area[f] = EMPTY_STRING;
            s.fonts.hyphen_char[f] = b'-' as i32;
            s.fonts.skew_char[f] = -1;
            s.fonts.bchar_label[f] = NON_ADDRESS;
            s.fonts.font_bchar[f] = TOO_BIG_CHAR;
            s.fonts.font_false_bchar[f] = TOO_BIG_CHAR;
            s.fonts.font_bc[f] = 1;
            s.fonts.font_ec[f] = 0;
            s.fonts.font_size[f] = 0;
            s.fonts.font_dsize[f] = 0;
            s.fonts.char_base[f] = 0;
            s.fonts.width_base[f] = 0;
            s.fonts.height_base[f] = 0;
            s.fonts.depth_base[f] = 0;
            s.fonts.italic_base[f] = 0;
            s.fonts.lig_kern_base[f] = 0;
            s.fonts.kern_base[f] = 0;
            s.fonts.exten_base[f] = 0;
            s.fonts.font_glue[f] = crate::mem::TEX_NULL;
            s.fonts.font_params[f] = 7;
            s.fonts.font_mapping[f] = None;
            s.fonts.param_base[f] = -1;
            for font_k in 0..=6usize {
                s.fonts.font_info[font_k].b32.s1 = 0;
            }
        }
    }

    let font_used_len = (st.fonts.font_max as usize) + 1;
    {
        let state = &mut *st;
        state.fonts.font_used = vec![false; font_used_len];
    }

    st.errors.random_seed = initial_random_seed(
        st.config.deterministic_mode,
        build_date,
        st.shipout.epochseconds,
        st.shipout.microseconds,
    );
    let random_seed = st.errors.random_seed;
    crate::scaled_math::init_randoms(&mut st.errors, random_seed);

    if st.errors.interaction == InteractionMode::Batch {
        st.shipout.selector = SELECTOR_NO_PRINT;
    } else {
        st.shipout.selector = SELECTOR_TERM_ONLY;
    }

    if st.config.semantic_pagination_enabled {
        st.eqtb.data[(PC_INT_BASE + INT_PAR_XETEX_GENERATE_ACTUAL_TEXT) as usize]
            .b32
            .s1 = 1;
    }

    pdf_files_init();
    crate::synctex::synctex_init_command(&mut *st);
    InitOutcome::Ready
}

/// Open, execute, finalize, and release one batch document operation.
///
/// Fatal paths reach the same terminal cleanup through `into_history`; the
/// ordinary path finalizes output and then tears down the engine state.
unsafe fn run_document_to_end(
    st: &mut crate::state::EngineState,
    input_file_name: *const libc::c_char,
) -> i32 {
    if let Err(a) =
        crate::io::start_input_in(&mut *st, input_file_name).map_err(|e| e.into_engine_abort())
    {
        // A fatal abort while opening the primary input. into_history runs the
        // print + cleanup and returns HISTORY_FATAL_ERROR, skipping the rest of
        // tt_run_engine.
        return a.into_history(&mut *st);
    }
    st.errors.history = HISTORY_SPOTLESS;
    if let Err(a) = crate::main_control::main_control_in(&mut *st).map_err(|e| e.into_engine_abort())
    {
        // A fatal main-control abort. into_history runs the print + cleanup and
        // returns HISTORY_FATAL_ERROR, skipping final_cleanup.
        return a.into_history(&mut *st);
    }
    if let Err(a) = final_cleanup_in(&mut *st) {
        // into_history runs the print + cleanup (incl. close_files_and_terminate
        // + tt_cleanup via post_error_message) and returns HISTORY_FATAL_ERROR.
        return a.into_history(&mut *st);
    }
    // Normal exit: a finalize_dvi_file failure here (e.g. XDV write error) aborts
    // via into_history, skipping tt_cleanup and returning HISTORY_FATAL_ERROR.
    if let Err(a) = crate::io::close_files_and_terminate_in(&mut *st) {
        return a.into_history(&mut *st);
    }
    tt_cleanup_in(&mut *st);

    st.errors.history
}

fn initial_random_seed(
    deterministic_mode: bool,
    build_date: libc::time_t,
    epochseconds: i32,
    microseconds: i32,
) -> i32 {
    if deterministic_mode {
        (build_date.wrapping_mul(1_000_003) & 0x7FFF_FFFF) as i32
    } else {
        microseconds * 1000 + (epochseconds % 1_000_000)
    }
}

#[cfg(test)]
mod deterministic_input_tests {
    use super::initial_random_seed;

    #[test]
    fn deterministic_seed_depends_only_on_build_date() {
        let a = initial_random_seed(true, 1_700_000_000, 1, 2);
        let b = initial_random_seed(true, 1_700_000_000, 999_999, 888_888);
        let c = initial_random_seed(true, 1_700_000_001, 1, 2);

        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a >= 0);
    }

    #[test]
    fn nondeterministic_seed_preserves_the_live_clock_formula() {
        assert_eq!(initial_random_seed(false, 0, 1_234_567, 42), 276_567);
        assert_eq!(
            initial_random_seed(false, 1_700_000_000, 1_234_567, 42),
            276_567
        );
    }
}
pub(crate) unsafe fn tt_cleanup_in(st: &mut crate::state::EngineState) {
    reset_layout_font_caches();
    tt_cleanup_pdf_files_close();
    libc::free(TEX_FORMAT_DEFAULT as *mut _);
    st.shipout.dvi_buf = Vec::new();

    tt_cleanup_destroy_font_manager();
    crate::ext::release_linebreak_state(&mut st.fonts);

    for slot in st.fonts.font_layout_engine.iter_mut() {
        *slot = None;
    }
    for raw in st.fonts.font_layout_engine_mirror.iter_mut() {
        *raw = std::ptr::null_mut();
    }
    for slot in st.fonts.font_mapping.iter_mut() {
        *slot = None;
    }
    st.fonts.loaded_font_mapping = None;

    for i in 1..=st.input.in_open {
        let slot = std::mem::take(&mut st.input.input_file[i as usize]);
        // Terminal cleanup absorbs a serious close error: teardown has begun.
        let _ = crate::io::u_close(slot);
    }
    for slot in st.input.read_file.iter_mut() {
        *slot = crate::state::InputSlot::Empty;
    }

    // Release all Rust-owned arenas (were C `malloc`'d / format-loaded arrays):
    // the save-stack, the input nest/file/macro/buffer
    // arrays, the hyph exception table, the scanner native_text scratch, the
    // mem/eqtb/strings/fonts format arenas, and the hyphenation trie arrays.
    {
        let s = &mut *st;
        s.save_stack.data = Vec::new();

        s.input.name_of_input_file = None;
        s.input.abspath_of_input_file.0 = [0; 1024];
        s.input.nest = Vec::new();
        s.input.input_file = Vec::new();
        s.input.read_file = Default::default();
        s.input.input_stack = Vec::new();
        s.input.param_stack = Vec::new();
        s.input.line_stack = Vec::new();
        s.input.grp_stack = Vec::new();
        s.input.if_stack = Vec::new();
        s.input.source_filename_stack = Vec::new();
        s.input.full_source_filename_stack = Vec::new();
        s.input.eof_seen = Vec::new();
        s.input.buffer = Vec::new();
        s.input.name_of_file = Vec::new();
        s.input.name_of_file16 = Vec::new();

        s.hyph.hyph_word = Vec::new();
        s.hyph.hyph_list = Vec::new();
        s.hyph.hyph_link = Vec::new();

        s.scanner.native_text = Vec::new();
        s.scanner.native_text_size = 0;
        s.scanner.native_len = 0;
        s.scanner.save_native_len = 0;

        s.eqtb.data = Vec::new();
        s.eqtb.hash = Vec::new();
        s.mem.data = Vec::new();
        s.mem.glyph_blobs.clear();

        s.strings.str_start = Vec::new();
        s.strings.str_pool = Vec::new();
        s.fonts.font_check = Vec::new();
        s.fonts.font_size = Vec::new();
        s.fonts.font_dsize = Vec::new();
        s.fonts.font_params = Vec::new();
        s.fonts.font_name = Vec::new();
        s.fonts.font_area = Vec::new();
        s.fonts.font_bc = Vec::new();
        s.fonts.font_ec = Vec::new();
        s.fonts.bchar_label = Vec::new();
        s.fonts.font_bchar = Vec::new();
        s.fonts.font_false_bchar = Vec::new();
        s.fonts.font_glue = Vec::new();
        s.fonts.font_used = Vec::new();

        s.fonts.font_info = Vec::new();
        s.fonts.font_layout_engine = Vec::new();
        s.fonts.font_layout_engine_mirror = Vec::new();
        s.fonts.font_mapping = Vec::new();
        s.fonts.font_flags = Vec::new();
        s.fonts.font_letter_space = Vec::new();
        s.fonts.hyphen_char = Vec::new();
        s.fonts.skew_char = Vec::new();
        s.fonts.char_base = Vec::new();
        s.fonts.width_base = Vec::new();
        s.fonts.height_base = Vec::new();
        s.fonts.depth_base = Vec::new();
        s.fonts.italic_base = Vec::new();
        s.fonts.lig_kern_base = Vec::new();
        s.fonts.kern_base = Vec::new();
        s.fonts.exten_base = Vec::new();
        s.fonts.param_base = Vec::new();
        s.fonts.xdv_buffer = Vec::new();
        s.fonts.mapped_text = Vec::new();
        crate::globals::refresh_font_ffi_mirrors(&s.fonts);

        s.hyph.trie_trl = Vec::new();
        s.hyph.trie_tro = Vec::new();
        s.hyph.trie_trc = Vec::new();
        s.hyph.trie_c = Vec::new();
        s.hyph.trie_o = Vec::new();
        s.hyph.trie_l = Vec::new();
        s.hyph.trie_r = Vec::new();
        s.hyph.trie_hash = Vec::new();
        s.hyph.trie_taken = Vec::new();
    }
}
