//! Engine init, run and cleanup: the `tt_run_engine` entry point, the
//! initialize_* primers, primitive registration and teardown.

use std::ffi::CStr;

use crate::bridge::ttstub_output_open_stdout;
use crate::cmd::{
    BOX_REF, CAR_RET, COMMENT, DATA, DONT_EXPAND, ENDV, END_TEMPLATE, ESCAPE, GLUE_REF, IF_TEST,
    IGNORE, IGNORE_SPACES, INVALID_CHAR, LETTER, OTHER_CHAR, OUTER_CALL, SHAPE_REF, SPACER,
    UNDEFINED_CS,
};
use crate::consts::{
    ACTIVE_BASE, BIGGEST_CHAR, BIGGEST_LANG, BOX_BASE, CAT_CODE_BASE, CLOSED, CS_TOKEN_FLAG,
    CUR_FONT_LOC, DEL_CODE_BASE, DIMEN_BASE, EMPTY_STRING, END_WRITE, EQTB_SIZE, ETEX_PEN_BASE,
    FONT_BASE, FROZEN_CONTROL_SEQUENCE, FROZEN_DONT_EXPAND, FROZEN_ENDV, FROZEN_END_TEMPLATE,
    FROZEN_PRIMITIVE, FROZEN_PROTECTION, GLUE_BASE, GLUE_NODE, GLUE_SPEC_SIZE, HASH_BASE,
    HISTORY_FATAL_ERROR, HISTORY_SPOTLESS, IGNORE_DEPTH, INT_BASE, INT_PAR_DAY,
    INT_PAR_END_LINE_CHAR, INT_PAR_ESCAPE_CHAR, INT_PAR_HANG_AFTER, INT_PAR_MAG,
    INT_PAR_MAX_DEAD_CYCLES, INT_PAR_MONTH, INT_PAR_NEW_LINE_CHAR, INT_PAR_TIME, INT_PAR_TOLERANCE,
    INT_PAR_XETEX_GENERATE_ACTUAL_TEXT, INT_PAR_XETEX_HYPHENATABLE_LENGTH, INT_PAR_YEAR,
    LC_CODE_BASE, LEFT_TO_RIGHT, LEVEL_ONE, LOCAL_BASE, LOCAL_OUTPUT_ROUTINE, LOCAL_PAR_SHAPE,
    MATH_CODE_BASE, MATH_FONT_BASE, MAX_FONT_MAX, MAX_HALFWORD, MEM_TOP, MIN_TRIE_OP, NEW_LINE,
    NON_ADDRESS, NORMAL_SCANNER, NUMBER_MATH_FAMILIES, NUMBER_USVS, PRIM_EQTB_BASE, PRIM_SIZE,
    SELECTOR_NO_PRINT, SELECTOR_TERM_AND_LOG, SELECTOR_TERM_ONLY, SF_CODE_BASE, SINGLE_BASE,
    TEX_NULL, TOKEN_LIST, TOKS_BASE, TOO_BIG_CHAR, UC_CODE_BASE, UNDEFINED_CONTROL_SEQUENCE, VMODE,
};
use crate::errors::EngineAbort;
use crate::expand::IF_NODE_SIZE;
use crate::format_io::load_fmt_file;
use crate::hash::{HASH_OFFSET, HASH_PRIME, HASH_SIZE};
use crate::hyphenation::TRIE_OP_SIZE;
use crate::io::init_io;
use crate::main_control::{LAST_BOX_CODE, VSPLIT_CODE};
use crate::mem::{
    xmalloc, ACTIVE_LIST, CONTRIB_HEAD, END_SPAN, HI_MEM_STAT_MIN, HI_MEM_STAT_USAGE, NULL_LIST,
    OMIT_TEMPLATE, PAGE_HEAD, PAGE_INS_HEAD,
};
use crate::mem_types::{zeroed_vec, B16x4, B32x2, MemoryWord};
use crate::node::{GlueSpecNode, NodePtr};
use crate::scan::{INTER_CHAR_VAL, INT_VAL, MARK_VAL};
use crate::state::{EngineState, EqtbState, FontState, GroupCode, InteractionMode, MemState};
use crate::units::{MemIdx, MemPtr, Scaled};

/// A `get_node` request this large merges every free block into the rover list.
const GET_NODE_HEAD_SENTINEL: i32 = 0x4000_0000;

pub(crate) const POOL_FREE: i32 = 47_500;
pub(crate) const STRINGS_FREE: i32 = 100;
const HYPH_SIZE: i32 = 8_191;

const HISTORY_WARNING_ISSUED: i32 = 1;

const LEVEL_ZERO: u16 = 0;
const NORMAL: u8 = 0;
const FIL: u16 = 1;
const FILL: u16 = 2;
const HYPHENATED: u16 = 1;
const SPLIT_UP: u16 = 1;
const EMPTY: u8 = 0;

const TOP_MARK_CODE: usize = 0;
const SPLIT_BOT_MARK_CODE: usize = 4;

const NUM_ETEX_PENALTIES: i32 = 4;
const NUMBER_REGS: i32 = 256;
const NUMBER_MATH_FONTS: i32 = 3 * NUMBER_MATH_FAMILIES;
const TOO_BIG_LANG: i32 = 256;
pub(crate) const HYPH_PRIME: i32 = 607;
const VAR_FAM_CLASS: i32 = 7;

pub(crate) const REGISTER_HELP: &CStr = c"A register number must be between 0 and 32767.";

const _: () = assert!(
    MEM_TOP >= 1100
        && HASH_PRIME <= HASH_SIZE
        && TEX_NULL <= 0
        && MAX_FONT_MAX >= TEX_NULL
        && MAX_FONT_MAX <= MAX_HALFWORD
        && HASH_OFFSET >= 0
        && HASH_OFFSET <= HASH_BASE
        && 2 * MAX_HALFWORD >= MEM_TOP
);

pub(crate) static mut TEX_FORMAT_DEFAULT: *mut libc::c_char = std::ptr::null_mut();

extern "C" {
    fn _ttbc_set_error_message(message: *const libc::c_char);
    fn pdf_files_init();
    fn pdf_files_close_if_open();
    fn pdf_files_close();
    fn destroy_font_manager();
    fn xetex_layout_reset_font_caches();
}

pub(crate) fn reset_layout_font_caches() {
    // SAFETY: clears mutex-protected process-global maps and does not borrow
    // engine state.
    unsafe { xetex_layout_reset_font_caches() };
}

pub(crate) fn output_file_extension(st: &EngineState) -> *const libc::c_char {
    if st.config.semantic_pagination_enabled {
        c".spx".as_ptr()
    } else {
        c".xdv".as_ptr()
    }
}

pub(crate) unsafe fn get_strings_started_in(st: &mut EngineState) -> Result<(), EngineAbort> {
    st.strings.pool_ptr = 0;
    st.strings.str_start[0] = 0;
    st.strings.str_ptr = TOO_BIG_CHAR;

    let spare_size = st.strings.pool_size - st.strings.string_vacancies;
    let loaded = crate::string_pool::load_pool_strings(&mut st.strings, spare_size)
        .map_err(|e| e.into_engine_abort())?;
    if loaded == 0 {
        return Err(EngineAbort::TtAbort(c"must increase pool_size".as_ptr()));
    }
    Ok(())
}

pub(crate) fn sort_avail_in(mem: &mut MemState) {
    let _ = crate::mem::get_node(mem, GET_NODE_HEAD_SENTINEL);
    let mut p = mem.b32s1(mem.rover + 1);
    mem.set_b32s1(mem.rover + 1, MAX_HALFWORD);
    let old_rover = mem.rover;

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
    fn into_engine_abort(self) -> EngineAbort {
        match self {
            PrimitiveInitAbort::MakeTexString(e) => e.into_engine_abort(),
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

pub(crate) fn set_eq(eqtb: &mut EqtbState, p: i32, eq_type: u8, equiv: i32, level: u16) {
    eqtb.set_b16s1(p, eq_type.into());
    eqtb.set_b32s1(p, equiv);
    eqtb.set_b16s0(p, level);
}

unsafe fn primitive(
    st: &mut EngineState,
    ident: &CStr,
    c: u8,
    o: i32,
) -> Result<(), PrimitiveInitAbort> {
    let bytes = ident.to_bytes();
    let prim_val = if bytes.len() > 1 {
        let len = bytes.len() as i32;
        let s = crate::texmfmp::maketexstring_in(&mut st.strings, ident.as_ptr())?;

        if st.input.first + len > st.input.buf_size + 1 {
            return Err(PrimitiveInitAbort::BufferSize(st.input.buf_size));
        }
        let first = st.input.first as usize;
        for (slot, &b) in st.input.buffer[first..].iter_mut().zip(bytes) {
            *slot = b as i32;
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
        crate::hash::prim_lookup(&mut st.eqtb, &mut st.strings, s)?
    } else {
        let ch = bytes[0] as i32;
        st.scanner.cur_val = ch + SINGLE_BASE;
        crate::hash::prim_lookup(&mut st.eqtb, &mut st.strings, ch)?
    };

    set_eq(&mut st.eqtb, st.scanner.cur_val, c, o, LEVEL_ONE);
    set_eq(&mut st.eqtb, PRIM_EQTB_BASE + prim_val, c, o, LEVEL_ONE);
    Ok(())
}

pub(crate) unsafe fn final_cleanup_in(st: &mut EngineState) -> Result<(), EngineAbort> {
    let c = st.scanner.cur_chr;

    if c != 1 {
        st.eqtb.set_b32s1(INT_BASE + INT_PAR_NEW_LINE_CHAR, -1);
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

    while st.input.open_parens > 0 {
        crate::output::print_cstr_in(st, c" )".as_ptr());
        st.input.open_parens -= 1;
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

    while st.scanner.cond_ptr.0 != TEX_NULL {
        crate::output::print_nl_in(st, b'(' as i32);
        crate::output::print_esc_cstr_in(st, c"end occurred ".as_ptr());
        crate::output::print_cstr_in(st, c"when ".as_ptr());
        crate::output::print_cmd_chr_in(st, IF_TEST, st.scanner.cur_if as i32);
        let if_line = st.scanner.if_line;
        if if_line != 0 {
            crate::output::print_cstr_in(st, c" on line ".as_ptr());
            crate::output::print_int_in(st, if_line);
        }
        crate::output::print_cstr_in(st, c" was incomplete)".as_ptr());
        let cond_ptr = st.scanner.cond_ptr;
        st.scanner.if_line = st.mem.b32s1(cond_ptr + 1);
        st.scanner.cur_if = st.mem.b16s0(cond_ptr) as i16;
        st.mem.temp_ptr = cond_ptr;
        st.scanner.cond_ptr.0 = st.mem.b32s1(cond_ptr);
        crate::mem::free_node(&mut st.mem, cond_ptr, IF_NODE_SIZE);
    }

    if st.errors.history != HISTORY_SPOTLESS
        && (st.errors.history == HISTORY_WARNING_ISSUED
            || st.errors.interaction < InteractionMode::ErrorStop)
        && st.shipout.selector == SELECTOR_TERM_AND_LOG
    {
        st.shipout.selector = SELECTOR_TERM_ONLY;
        crate::output::print_nl_cstr_in(
            st,
            c"(see the transcript file for additional information)".as_ptr(),
        );
        st.shipout.selector = SELECTOR_TERM_AND_LOG;
    }

    if c != 1 {
        return Ok(());
    }
    if !st.config.in_initex_mode {
        crate::output::print_nl_cstr_in(st, c"(\\dump is performed only by INITEX)".as_ptr());
        return Ok(());
    }

    for mark_code in TOP_MARK_CODE..=SPLIT_BOT_MARK_CODE {
        let mark = st.scanner.cur_mark[mark_code];
        if !mark.is_null() {
            crate::mem::delete_token_ref(&mut st.mem, mark);
        }
    }

    if let Some(marks) = st.scanner.sa_root[MARK_VAL as usize].as_idx() {
        if crate::packaging::do_marks(&mut st.mem, &mut st.scanner, 3, 0, marks) {
            st.scanner.sa_root[MARK_VAL as usize] = MemPtr::NULL;
        }
    }

    for box_code in LAST_BOX_CODE as usize..=VSPLIT_CODE as usize {
        crate::packaging::flush_node_list(&mut st.mem, st.scanner.disc_ptr[box_code])
            .map_err(|msg| EngineAbort::Confusion(msg.as_ptr()))?;
    }

    if st.shipout.last_glue.0 != MAX_HALFWORD {
        crate::mem::delete_glue_ref(&mut st.mem, st.shipout.last_glue.0);
    }

    crate::format_io::store_fmt_file_in(st)
}

fn initialize_more_variables_in(st: &mut EngineState) {
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

    st.errors.spec_log[1..=13].copy_from_slice(&[
        93_032_640, 38_612_034, 17_922_280, 8_662_214, 4_261_238, 2_113_709, 1_052_693, 525_315,
        262_400, 131_136, 65_552, 32_772, 16_385,
    ]);
    for k in 14..=27usize {
        st.errors.spec_log[k] = st.errors.two_to_the[27 - k];
    }
    st.errors.spec_log[28] = 1;

    st.input.nest_ptr = 0;
    st.input.max_nest_stack = 0;
    st.input.cur_list.mode = VMODE;
    st.input.cur_list.head = CONTRIB_HEAD;
    st.input.cur_list.tail = CONTRIB_HEAD;
    st.input.cur_list.e_tex_aux = MemPtr::NULL;
    st.input.cur_list.set_aux_s1(IGNORE_DEPTH);
    st.input.cur_list.mode_line = 0;
    st.input.cur_list.prev_graf = 0;
    st.scanner.shown_mode = 0;
    st.shipout.page_contents = EMPTY;
    st.shipout.page_tail = PAGE_HEAD;
    st.shipout.last_glue.0 = MAX_HALFWORD;
    st.shipout.last_penalty = 0;
    st.shipout.last_kern = 0;
    st.shipout.last_node_type = -1;
    st.shipout.page_so_far[7] = 0;

    for k in INT_BASE..=EQTB_SIZE {
        st.eqtb.set_xeq_level_at(k, LEVEL_ONE);
    }

    st.eqtb.no_new_control_sequence = true;
    for k in 0..=PRIM_SIZE {
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
    st.scanner.cur_mark = [MemPtr::NULL; 5];
    st.scanner.cur_val = 0;
    st.scanner.cur_val_level = INT_VAL;
    st.scanner.radix = 0;
    st.scanner.cur_order = NORMAL;

    st.input.read_open = [CLOSED; 17];

    st.scanner.cond_ptr.0 = TEX_NULL;
    st.scanner.if_limit = NORMAL;
    st.scanner.cur_if = 0;
    st.scanner.if_line = 0;
    st.fonts.null_character = B16x4::default();
    st.shipout.total_pages = 0;
    st.shipout.max_v = 0;
    st.shipout.max_h = 0;
    st.shipout.max_push = 0;
    st.shipout.last_bop = -1;
    st.shipout.doing_leaders = false;
    st.shipout.dead_cycles = 0;
    st.scanner.adjust_tail.0 = TEX_NULL;
    st.shipout.last_badness = 0;
    st.scanner.pre_adjust_tail.0 = TEX_NULL;
    st.scanner.pack_begin_line = 0;
    st.scanner.empty.s1 = EMPTY as i32;
    st.scanner.empty.s0 = TEX_NULL;
    st.scanner.align_ptr = MemPtr::NULL;
    st.scanner.cur_align.0 = TEX_NULL;
    st.scanner.cur_span = MemPtr::NULL;
    st.scanner.cur_loop.0 = TEX_NULL;
    st.scanner.cur_head.0 = TEX_NULL;
    st.scanner.cur_tail.0 = TEX_NULL;
    st.scanner.cur_pre_head.0 = TEX_NULL;
    st.scanner.cur_pre_tail.0 = TEX_NULL;
    st.scanner.cur_f = 0;
    st.hyph.max_hyph_char = TOO_BIG_LANG;

    let hyph_len = st.hyph.hyph_size as usize + 1;
    st.hyph.hyph_word[..hyph_len].fill(0);
    st.hyph.hyph_list[..hyph_len].fill(TEX_NULL);
    st.hyph.hyph_link[..hyph_len].fill(0);

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

    st.shipout.write_open = [false; 18];

    st.scanner.lr_ptr.0 = TEX_NULL;
    st.scanner.lr_problems = 0;
    st.scanner.cur_dir = LEFT_TO_RIGHT;
    st.input.pseudo_files.0 = TEX_NULL;
    st.scanner.sa_root[MARK_VAL as usize] = MemPtr::NULL;
    st.scanner.sa_null = MemoryWord {
        b32: B32x2 {
            s0: TEX_NULL,
            s1: TEX_NULL,
        },
    };
    st.scanner.sa_chain.0 = TEX_NULL;
    st.scanner.sa_level = LEVEL_ZERO;
    st.scanner.disc_ptr[LAST_BOX_CODE as usize] = MemPtr::NULL;
    st.scanner.disc_ptr[VSPLIT_CODE as usize] = MemPtr::NULL;
    st.strings.edit_name_start = 0;
    st.scanner.stop_at_space = true;
}

fn static_glue_spec(p: i32) -> NodePtr<GlueSpecNode> {
    // SAFETY: mem[0..20] holds the five static glue specs of INITEX.
    unsafe { NodePtr::from_idx_unchecked(MemIdx::new(p)) }
}

fn initialize_more_initex_variables_in(st: &mut EngineState, format_ident: i32) {
    let set_family = |x: i32| (x & 0xFF) << 24;
    let set_class = |x: i32| (x & 0x07) << 21;

    let mem = &mut st.mem;
    for p in (0..=16).step_by(GLUE_SPEC_SIZE as usize) {
        let spec = static_glue_spec(p);
        spec.set_width(mem, Scaled(0));
        spec.set_stretch(mem, Scaled(0));
        spec.set_shrink(mem, Scaled(0));
        spec.set_ref_count(mem, TEX_NULL + 1);
        spec.set_stretch_order(mem, NORMAL.into());
        spec.set_shrink_order(mem, NORMAL.into());
    }

    let fil_glue = static_glue_spec(4);
    fil_glue.set_stretch(mem, Scaled(65536));
    fil_glue.set_stretch_order(mem, FIL);
    let fill_glue = static_glue_spec(8);
    fill_glue.set_stretch(mem, Scaled(65536));
    fill_glue.set_stretch_order(mem, FILL);
    let ss_glue = static_glue_spec(12);
    ss_glue.set_stretch(mem, Scaled(65536));
    ss_glue.set_stretch_order(mem, FIL);
    ss_glue.set_shrink(mem, Scaled(65536));
    ss_glue.set_shrink_order(mem, FIL);
    let fil_neg_glue = static_glue_spec(16);
    fil_neg_glue.set_stretch(mem, Scaled(-65536));
    fil_neg_glue.set_stretch_order(mem, FIL);

    mem.rover = 20;
    mem.set_b32s1(mem.rover, MAX_HALFWORD);
    mem.set_b32s0(mem.rover, 1000);
    mem.set_b32s0(mem.rover + 1, mem.rover);
    mem.set_b32s1(mem.rover + 1, mem.rover);
    mem.lo_mem_max = mem.rover + 1000;
    mem.set_b32s1(mem.lo_mem_max, TEX_NULL);
    mem.set_b32s0(mem.lo_mem_max, TEX_NULL);

    let template = mem.word(mem.lo_mem_max);
    for k in HI_MEM_STAT_MIN..=MEM_TOP {
        mem.set_word(k, template);
    }

    mem.set_b32s0(OMIT_TEMPLATE, CS_TOKEN_FLAG + FROZEN_END_TEMPLATE);
    mem.set_b32s1(END_SPAN, u16::MAX as i32 + 1);
    mem.set_b32s0(END_SPAN, TEX_NULL);
    mem.set_b16s1(ACTIVE_LIST, HYPHENATED);
    mem.set_b32s0(ACTIVE_LIST + 1, MAX_HALFWORD);
    mem.set_b16s0(ACTIVE_LIST, 0);
    mem.set_b16s0(PAGE_INS_HEAD, 255);
    mem.set_b16s1(PAGE_INS_HEAD, SPLIT_UP);
    mem.set_b32s1(PAGE_INS_HEAD, PAGE_INS_HEAD.0);
    mem.set_b16s1(PAGE_HEAD, GLUE_NODE);
    mem.set_b16s0(PAGE_HEAD, NORMAL.into());
    mem.avail = TEX_NULL;
    mem.mem_end = MEM_TOP;
    mem.hi_mem_min = HI_MEM_STAT_MIN;
    mem.var_used = 20;
    mem.dyn_used = HI_MEM_STAT_USAGE;

    let eqtb = &mut st.eqtb;
    set_eq(
        eqtb,
        UNDEFINED_CONTROL_SEQUENCE,
        UNDEFINED_CS,
        TEX_NULL,
        LEVEL_ZERO,
    );
    let undefined_entry = eqtb.word(UNDEFINED_CONTROL_SEQUENCE);
    eqtb.data[ACTIVE_BASE as usize..=eqtb.eqtb_top as usize].fill(undefined_entry);

    set_eq(eqtb, GLUE_BASE, GLUE_REF, 0, LEVEL_ONE);
    let glue_base_entry = eqtb.word(GLUE_BASE);
    eqtb.data[(GLUE_BASE + 1) as usize..LOCAL_BASE as usize].fill(glue_base_entry);

    let zero_glue = static_glue_spec(0);
    let zero_glue_refs = zero_glue.ref_count(&st.mem) + 531;
    zero_glue.set_ref_count(&mut st.mem, zero_glue_refs);

    set_eq(
        eqtb,
        LOCAL_BASE + LOCAL_PAR_SHAPE,
        SHAPE_REF,
        TEX_NULL,
        LEVEL_ONE,
    );
    let par_shape_entry = eqtb.word(LOCAL_BASE + LOCAL_PAR_SHAPE);
    eqtb.data[ETEX_PEN_BASE as usize..(ETEX_PEN_BASE + NUM_ETEX_PENALTIES) as usize]
        .fill(par_shape_entry);

    let toks_end = TOKS_BASE + NUMBER_REGS;
    eqtb.data[(LOCAL_BASE + LOCAL_OUTPUT_ROUTINE) as usize..toks_end as usize]
        .fill(undefined_entry);

    set_eq(eqtb, BOX_BASE, BOX_REF, TEX_NULL, LEVEL_ONE);
    let box_base_entry = eqtb.word(BOX_BASE);
    eqtb.data[(BOX_BASE + 1) as usize..(BOX_BASE + NUMBER_REGS) as usize].fill(box_base_entry);

    set_eq(eqtb, CUR_FONT_LOC, DATA, FONT_BASE, LEVEL_ONE);
    let cur_font_entry = eqtb.word(CUR_FONT_LOC);
    eqtb.data[MATH_FONT_BASE as usize..(MATH_FONT_BASE + NUMBER_MATH_FONTS) as usize]
        .fill(cur_font_entry);

    set_eq(eqtb, CAT_CODE_BASE, DATA, 0, LEVEL_ONE);
    let cat_code_entry = eqtb.word(CAT_CODE_BASE);
    eqtb.data[(CAT_CODE_BASE + 1) as usize..INT_BASE as usize].fill(cat_code_entry);

    for k in 0..NUMBER_USVS {
        eqtb.set_b32s1(CAT_CODE_BASE + k, OTHER_CHAR.into());
        eqtb.set_b32s1(MATH_CODE_BASE + k, k);
        eqtb.set_b32s1(SF_CODE_BASE + k, 1000);
    }

    eqtb.set_b32s1(CAT_CODE_BASE + 13, CAR_RET.into());
    eqtb.set_b32s1(CAT_CODE_BASE + 32, SPACER.into());
    eqtb.set_b32s1(CAT_CODE_BASE + 92, ESCAPE.into());
    eqtb.set_b32s1(CAT_CODE_BASE + 37, COMMENT.into());
    eqtb.set_b32s1(CAT_CODE_BASE + 127, INVALID_CHAR.into());
    eqtb.set_b32s1(CAT_CODE_BASE, IGNORE.into());

    for k in (b'0' as i32)..=(b'9' as i32) {
        eqtb.set_b32s1(MATH_CODE_BASE + k, k + set_class(VAR_FAM_CLASS));
    }

    for k in (b'A' as i32)..=(b'Z' as i32) {
        let lower = k + 32;
        eqtb.set_b32s1(CAT_CODE_BASE + k, LETTER.into());
        eqtb.set_b32s1(CAT_CODE_BASE + lower, LETTER.into());
        eqtb.set_b32s1(
            MATH_CODE_BASE + k,
            k + set_family(1) + set_class(VAR_FAM_CLASS),
        );
        eqtb.set_b32s1(
            MATH_CODE_BASE + lower,
            lower + set_family(1) + set_class(VAR_FAM_CLASS),
        );
        eqtb.set_b32s1(LC_CODE_BASE + k, lower);
        eqtb.set_b32s1(LC_CODE_BASE + lower, lower);
        eqtb.set_b32s1(UC_CODE_BASE + k, k);
        eqtb.set_b32s1(UC_CODE_BASE + lower, k);
        eqtb.set_b32s1(SF_CODE_BASE + k, 999);
    }

    for k in INT_BASE..DEL_CODE_BASE {
        eqtb.set_b32s1(k, 0);
    }

    eqtb.set_b32s1(INT_BASE + INT_PAR_MAG, 1000);
    eqtb.set_b32s1(INT_BASE + INT_PAR_TOLERANCE, 10000);
    eqtb.set_b32s1(INT_BASE + INT_PAR_HANG_AFTER, 1);
    eqtb.set_b32s1(INT_BASE + INT_PAR_MAX_DEAD_CYCLES, 25);
    eqtb.set_b32s1(INT_BASE + INT_PAR_ESCAPE_CHAR, b'\\' as i32);
    eqtb.set_b32s1(INT_BASE + INT_PAR_END_LINE_CHAR, b'\r' as i32);

    for k in 0..NUMBER_USVS {
        eqtb.set_b32s1(DEL_CODE_BASE + k, -1);
    }
    eqtb.set_b32s1(DEL_CODE_BASE + b'.' as i32, 0);

    for k in DIMEN_BASE..=EQTB_SIZE {
        eqtb.set_b32s1(k, 0);
    }

    eqtb.prim_used = PRIM_SIZE;
    eqtb.hash_used = FROZEN_CONTROL_SEQUENCE;
    eqtb.hash_high = 0;
    eqtb.cs_count = 0;

    let hyph = &mut st.hyph;
    hyph.trie_op_hash_array[..=(2 * TRIE_OP_SIZE) as usize].fill(0);
    hyph.trie_used[..=BIGGEST_LANG as usize].fill(MIN_TRIE_OP as u16);
    hyph.max_op_used = MIN_TRIE_OP as u16;
    hyph.trie_op_ptr = 0;
    hyph.trie_not_ready = true;

    st.strings.format_ident = format_ident;

    st.scanner.max_reg_num = 32767;
    st.scanner.max_reg_help_line = REGISTER_HELP.as_ptr();
    st.scanner.sa_root[INT_VAL as usize..=INTER_CHAR_VAL as usize].fill(MemPtr::NULL);

    st.eqtb
        .set_b32s1(INT_BASE + INT_PAR_XETEX_HYPHENATABLE_LENGTH, 63);
}

unsafe fn set_frozen_text(
    st: &mut EngineState,
    p: i32,
    text: &CStr,
) -> Result<(), PrimitiveInitAbort> {
    let s = crate::texmfmp::maketexstring_in(&mut st.strings, text.as_ptr())?;
    st.eqtb.set_hash_s1(p, s);
    Ok(())
}

unsafe fn initialize_primitives(st: &mut EngineState) -> Result<(), PrimitiveInitAbort> {
    use crate::primitives::PrimInit;

    st.eqtb.no_new_control_sequence = false;
    st.input.first = 0;

    for prim in crate::primitives::PRIMITIVES {
        primitive(st, prim.name, prim.cmd, prim.chr)?;
        match prim.init {
            PrimInit::None => {}
            PrimInit::Par => {
                st.input.par_loc = st.scanner.cur_val;
                st.input.par_token = CS_TOKEN_FLAG + st.input.par_loc;
            }
            PrimInit::Write => {
                st.shipout.write_loc = st.scanner.cur_val;
            }
            PrimInit::Frozen(frozen) => {
                set_frozen_text(st, frozen, prim.name)?;
                st.eqtb.data[frozen as usize] = st.eqtb.word(st.scanner.cur_val);
            }
        }
    }

    set_frozen_text(st, FROZEN_END_TEMPLATE, c"endtemplate")?;
    set_eq(
        &mut st.eqtb,
        FROZEN_END_TEMPLATE,
        END_TEMPLATE,
        NULL_LIST.0,
        LEVEL_ONE,
    );

    set_frozen_text(st, FROZEN_ENDV, c"endtemplate")?;
    set_eq(&mut st.eqtb, FROZEN_ENDV, ENDV, NULL_LIST.0, LEVEL_ONE);

    set_frozen_text(st, FROZEN_DONT_EXPAND, c"notexpanded:")?;
    st.eqtb.set_b16s1(FROZEN_DONT_EXPAND, DONT_EXPAND.into());

    set_frozen_text(st, FROZEN_PRIMITIVE, c"primitive")?;
    set_eq(&mut st.eqtb, FROZEN_PRIMITIVE, IGNORE_SPACES, 1, LEVEL_ONE);

    set_frozen_text(st, FROZEN_PROTECTION, c"inaccessible")?;

    set_frozen_text(st, END_WRITE, c"endwrite")?;
    set_eq(&mut st.eqtb, END_WRITE, OUTER_CALL, TEX_NULL, LEVEL_ONE);

    st.eqtb.no_new_control_sequence = true;
    Ok(())
}

/// Engine entry point (called from `tt_engine_xetex_main`).
///
/// A Rust panic is caught and turned into a `HISTORY_FATAL_ERROR` return with
/// a surfaced message instead of unwinding into the host.
pub(crate) unsafe fn tt_run_engine(
    api: *mut libc::c_void,
    dump_name: *const libc::c_char,
    input_file_name: *const libc::c_char,
    build_date: libc::time_t,
) -> i32 {
    reset_layout_font_caches();
    struct LayoutCacheGuard;
    impl Drop for LayoutCacheGuard {
        fn drop(&mut self) {
            reset_layout_font_caches();
        }
    }
    let _layout_cache_guard = LayoutCacheGuard;

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
    // An abort that skips terminal cleanup must not leak this process-global
    // registry into the next batch run.
    struct PdfFilesGuard;
    impl Drop for PdfFilesGuard {
        fn drop(&mut self) {
            unsafe { pdf_files_close_if_open() };
        }
    }
    let _pdf_files_guard = PdfFilesGuard;

    let mut st = Box::new(EngineState::default());
    crate::engine_interface::apply_config(&mut st.config);
    struct PtrGuard;
    impl Drop for PtrGuard {
        fn drop(&mut self) {
            crate::state::set_engine_state_ptr(std::ptr::null_mut());
        }
    }
    crate::state::set_engine_state_ptr(&mut *st as *mut EngineState);
    let _ptr_guard = PtrGuard;

    match initialize_engine(&mut st, dump_name, build_date) {
        InitOutcome::Ready => run_document_to_end(&mut st, input_file_name),
        InitOutcome::Terminate(history) => history,
    }
}

/// Result of initializing an engine before any primary document is opened.
pub(crate) enum InitOutcome {
    Ready,
    Terminate(i32),
}

pub(crate) fn alloc_font_arrays(fonts: &mut FontState) {
    let len = fonts.font_max as usize + 1;
    fonts.font_layout_engine = vec![None; len];
    fonts.font_mapping = vec![None; len];
    fonts.font_flags = vec![0; len];
    fonts.font_letter_space = vec![0; len];
    fonts.font_check = vec![B16x4::default(); len];
    fonts.font_size = vec![0; len];
    fonts.font_dsize = vec![0; len];
    fonts.font_params = vec![0; len];
    fonts.font_name = vec![0; len];
    fonts.font_area = vec![0; len];
    fonts.font_bc = vec![0; len];
    fonts.font_ec = vec![0; len];
    fonts.bchar_label = vec![0; len];
    fonts.font_bchar = vec![0; len];
    fonts.font_false_bchar = vec![0; len];
    fonts.font_glue = vec![0; len];
    fonts.hyphen_char = vec![0; len];
    fonts.skew_char = vec![0; len];
    fonts.char_base = vec![0; len];
    fonts.width_base = vec![0; len];
    fonts.height_base = vec![0; len];
    fonts.depth_base = vec![0; len];
    fonts.italic_base = vec![0; len];
    fonts.lig_kern_base = vec![0; len];
    fonts.kern_base = vec![0; len];
    fonts.exten_base = vec![0; len];
    fonts.param_base = vec![0; len];
}

/// Allocate engine storage, load the format, and initialize configuration.
///
/// On success the state is ready to open a primary input but owns no document
/// input stack, so batch and resident sessions share this path.
pub(crate) unsafe fn initialize_engine(
    st: &mut EngineState,
    dump_name: *const libc::c_char,
    build_date: libc::time_t,
) -> InitOutcome {
    TEX_FORMAT_DEFAULT = xmalloc(libc::strlen(dump_name) + 1) as *mut libc::c_char;
    libc::strcpy(TEX_FORMAT_DEFAULT, dump_name);
    st.shipout.rust_stdout = ttstub_output_open_stdout();

    if st.config.file_line_error_style_p < 0 {
        st.config.file_line_error_style_p = 0;
    }

    st.strings.pool_size = 6_250_000;
    st.strings.string_vacancies = 90_000;
    st.strings.max_strings = 565_536;
    st.fonts.font_mem_size = 8_000_000;
    st.fonts.font_max = 9_000;
    st.hyph.trie_size = 1_000_000;
    st.hyph.hyph_size = HYPH_SIZE;
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
    st.eqtb.hash_extra = 600_000;

    let in_open_words = st.input.max_in_open as usize + 1;
    let hyph_words = HYPH_SIZE as usize + 1;
    st.input.buffer = vec![0; st.input.buf_size as usize + 1];
    st.input.nest = vec![Default::default(); st.input.nest_size as usize + 1];
    st.save_stack.data = zeroed_vec(st.save_stack.save_size as usize + 1);
    st.input.input_file = std::iter::repeat_with(|| crate::state::InputSlot::Empty)
        .take(in_open_words)
        .collect();
    st.input.input_stack = vec![Default::default(); st.input.stack_size as usize + 1];
    st.input.line_stack = vec![0; in_open_words];
    st.input.grp_stack = vec![0; in_open_words];
    st.input.if_stack = vec![MemPtr::from_raw(0); in_open_words];
    st.input.source_filename_stack = vec![0; in_open_words];
    st.input.full_source_filename_stack = vec![0; in_open_words];
    st.input.eof_seen = vec![false; in_open_words];
    st.input.param_stack = vec![0; st.input.param_size as usize + 1];
    st.hyph.hyph_word = vec![0; hyph_words];
    st.hyph.hyph_list = vec![0; hyph_words];
    st.hyph.hyph_link = vec![0; hyph_words];

    if st.config.in_initex_mode {
        st.mem.data = zeroed_vec(MEM_TOP as usize + 2);

        st.eqtb.eqtb_top = EQTB_SIZE + st.eqtb.hash_extra;
        st.eqtb.hash_top = if st.eqtb.hash_extra == 0 {
            UNDEFINED_CONTROL_SEQUENCE
        } else {
            st.eqtb.eqtb_top
        };
        st.eqtb.hash = zeroed_vec((1 + st.eqtb.hash_top - HASH_OFFSET) as usize);
        st.eqtb.hash_used = st.eqtb.hash_top;

        st.eqtb.data = zeroed_vec(st.eqtb.eqtb_top as usize + 1);
        st.strings.str_start = vec![0; st.strings.max_strings as usize + 1];
        st.strings.str_pool = vec![0; st.strings.pool_size as usize + 1];
        st.fonts.font_info = zeroed_vec(st.fonts.font_mem_size as usize + 1);
    }

    st.errors.history = HISTORY_FATAL_ERROR;
    let mut bad = 0;
    if st.errors.half_error_line < 30 || st.errors.half_error_line > st.errors.error_line - 15 {
        bad = 1;
    }
    if st.config.max_print_line < 60 {
        bad = 2;
    }
    if st.input.max_in_open >= 128 {
        bad = 6;
    }
    if st.fonts.font_max > FONT_BASE + 9000 {
        bad = 16;
    }
    if st.save_stack.save_size > MAX_HALFWORD || st.strings.max_strings > MAX_HALFWORD {
        bad = 17;
    }
    if st.input.buf_size > MAX_HALFWORD {
        bad = 18;
    }
    if CS_TOKEN_FLAG + EQTB_SIZE + st.eqtb.hash_extra > MAX_HALFWORD {
        bad = 21;
    }
    if bad > 0 {
        let mut msg = [0 as libc::c_char; 80];
        crate::compat::snprintf(
            msg.as_mut_ptr(),
            msg.len(),
            c"failed internal consistency check #%d".as_ptr(),
            bad,
        );
        return InitOutcome::Terminate(EngineAbort::TtAbort(msg.as_ptr()).into_history(st));
    }

    initialize_more_variables_in(st);
    if st.config.in_initex_mode {
        if let Err(a) = get_strings_started_in(st) {
            return InitOutcome::Terminate(a.into_history(st));
        }
        let format_ident =
            match crate::texmfmp::maketexstring_in(&mut st.strings, c" (INITEX)".as_ptr()) {
                Ok(v) => v,
                Err(e) => return InitOutcome::Terminate(e.into_engine_abort().into_history(st)),
            };
        initialize_more_initex_variables_in(st, format_ident);
        if let Err(e) = initialize_primitives(st) {
            return InitOutcome::Terminate(e.into_engine_abort().into_history(st));
        }
        st.strings.init_str_ptr = st.strings.str_ptr;
        st.strings.init_pool_ptr = st.strings.pool_ptr;
    }

    crate::pagebuilder::initialize_pagebuilder_variables(st);
    crate::shipout::initialize_shipout_variables(st);
    if st.config.deterministic_mode {
        st.shipout.epochseconds = build_date as i32;
        st.shipout.microseconds = 0;
    } else {
        (st.shipout.epochseconds, st.shipout.microseconds) =
            crate::texmfmp::get_seconds_and_micros();
    }
    crate::texmfmp::init_start_time(st, build_date);

    st.shipout.selector = SELECTOR_TERM_ONLY;
    st.errors.tally = 0;
    st.shipout.term_offset = 0;
    st.shipout.file_offset = 0;
    st.strings.job_name = 0;
    st.scanner.name_in_progress = false;
    st.errors.log_opened = false;

    st.input.input_ptr = 0;
    st.input.max_in_stack = 0;
    st.input.source_filename_stack[0] = 0;
    st.input.full_source_filename_stack[0] = 0;
    st.input.in_open = 0;
    st.input.open_parens = 0;
    st.input.max_buf_stack = 0;
    st.input.grp_stack[0] = 0;
    st.input.if_stack[0] = MemPtr::NULL;
    st.input.param_ptr = 0;
    st.input.max_param_stack = 0;
    st.scanner.used_tectonic_coda_tokens = false;
    st.fonts.gave_char_warning_help = false;

    st.scanner.scanner_status = NORMAL_SCANNER;
    st.scanner.warning_index = TEX_NULL;
    st.input.first = 1;
    st.input.cur_input.state = NEW_LINE;
    st.input.cur_input.start = 1;
    st.input.cur_input.index = 0;
    st.mem.line = 0;
    st.input.cur_input.name = 0;
    st.input.force_eof = false;
    st.scanner.align_state = 1_000_000;

    init_io(st);

    if st.config.in_initex_mode {
        st.scanner.max_reg_num = 32767;
        st.scanner.max_reg_help_line = REGISTER_HELP.as_ptr();
    }
    st.eqtb.no_new_control_sequence = true;

    if !st.config.in_initex_mode {
        match load_fmt_file(st) {
            Ok(true) => {}
            Ok(false) => return InitOutcome::Terminate(st.errors.history),
            Err(a) => return InitOutcome::Terminate(a.into_history(st)),
        }
    }

    let end_line_char = st.eqtb.b32s1(INT_BASE + INT_PAR_END_LINE_CHAR);
    if (0..=BIGGEST_CHAR).contains(&end_line_char) {
        st.input.buffer[st.input.cur_input.limit as usize] = end_line_char;
    } else {
        st.input.cur_input.limit -= 1;
    }

    let (time, day, month, year) = if st.config.in_initex_mode {
        (0, 0, 0, 0)
    } else {
        crate::texmfmp::get_date_and_time(build_date, st.config.deterministic_mode)
    };
    st.eqtb.set_b32s1(INT_BASE + INT_PAR_TIME, time);
    st.eqtb.set_b32s1(INT_BASE + INT_PAR_DAY, day);
    st.eqtb.set_b32s1(INT_BASE + INT_PAR_MONTH, month);
    st.eqtb.set_b32s1(INT_BASE + INT_PAR_YEAR, year);

    if st.hyph.trie_not_ready {
        let hyph = &mut st.hyph;
        let trie_len = hyph.trie_size as usize + 1;
        hyph.trie_trl = vec![0; trie_len];
        hyph.trie_tro = vec![0; trie_len];
        hyph.trie_trc = vec![0; trie_len];
        hyph.trie_c = vec![0; trie_len];
        hyph.trie_o = vec![0; trie_len];
        hyph.trie_l = vec![0; trie_len];
        hyph.trie_r = vec![0; trie_len];
        hyph.trie_hash = vec![0; trie_len];
        hyph.trie_taken = vec![false; trie_len];
        hyph.trie_ptr = 0;
        hyph.hyph_start = 0;

        alloc_font_arrays(&mut st.fonts);

        let nullfont_name =
            match crate::texmfmp::maketexstring_in(&mut st.strings, c"nullfont".as_ptr()) {
                Ok(v) => v,
                Err(e) => return InitOutcome::Terminate(e.into_engine_abort().into_history(st)),
            };
        let fonts = &mut st.fonts;
        let f = FONT_BASE as usize;
        fonts.font_ptr = FONT_BASE;
        fonts.fmem_ptr = 7;
        fonts.font_name[f] = nullfont_name;
        fonts.font_area[f] = EMPTY_STRING;
        fonts.hyphen_char[f] = b'-' as i32;
        fonts.skew_char[f] = -1;
        fonts.bchar_label[f] = NON_ADDRESS;
        fonts.font_bchar[f] = TOO_BIG_CHAR;
        fonts.font_false_bchar[f] = TOO_BIG_CHAR;
        fonts.font_bc[f] = 1;
        fonts.font_ec[f] = 0;
        fonts.font_glue[f] = TEX_NULL;
        fonts.font_params[f] = 7;
        fonts.param_base[f] = -1;
        for word in &mut fonts.font_info[..7] {
            word.b32.s1 = 0;
        }
    }

    st.fonts.font_used = vec![false; st.fonts.font_max as usize + 1];

    st.errors.random_seed = initial_random_seed(
        st.config.deterministic_mode,
        build_date,
        st.shipout.epochseconds,
        st.shipout.microseconds,
    );
    let random_seed = st.errors.random_seed;
    crate::scaled_math::init_randoms(&mut st.errors, random_seed);

    st.shipout.selector = if st.errors.interaction == InteractionMode::Batch {
        SELECTOR_NO_PRINT
    } else {
        SELECTOR_TERM_ONLY
    };

    if st.config.semantic_pagination_enabled {
        st.eqtb
            .set_b32s1(INT_BASE + INT_PAR_XETEX_GENERATE_ACTUAL_TEXT, 1);
    }

    pdf_files_init();
    crate::synctex::synctex_init_command(st);
    InitOutcome::Ready
}

/// Open, execute, finalize, and release one batch document operation.
unsafe fn run_document_to_end(st: &mut EngineState, input_file_name: *const libc::c_char) -> i32 {
    let result = (|| {
        crate::io::start_input_in(st, input_file_name).map_err(|e| e.into_engine_abort())?;
        st.errors.history = HISTORY_SPOTLESS;
        crate::main_control::main_control_in(st).map_err(|e| e.into_engine_abort())?;
        final_cleanup_in(st)?;
        crate::io::close_files_and_terminate_in(st)
    })();
    if let Err(a) = result {
        return a.into_history(st);
    }
    tt_cleanup_in(st);
    st.errors.history
}

/// Run and finalize one document while retaining the initialized engine arenas.
/// Aborts are returned without terminal cleanup so a resident session can
/// close operation resources and rewind rung zero.
pub(crate) unsafe fn run_resident_document(
    st: &mut EngineState,
    input_file_name: *const libc::c_char,
) -> Result<i32, EngineAbort> {
    crate::io::start_input_in(st, input_file_name).map_err(|e| e.into_engine_abort())?;
    st.errors.history = HISTORY_SPOTLESS;
    crate::main_control::main_control_in(st).map_err(|e| e.into_engine_abort())?;
    final_cleanup_in(st)?;
    crate::io::close_files_and_terminate_in(st)?;
    Ok(st.errors.history)
}

/// Resume a resident document at a restored command boundary.
pub(crate) unsafe fn resume_resident_document(
    st: &mut EngineState,
    resume_pending_command: bool,
) -> Result<i32, EngineAbort> {
    crate::io::reopen_resident_outputs_in(st)?;
    crate::main_control::main_control_resume_in(st, resume_pending_command)
        .map_err(|e| e.into_engine_abort())?;
    final_cleanup_in(st)?;
    crate::io::close_files_and_terminate_in(st)?;
    Ok(st.errors.history)
}

/// Close input slots that may remain live after an interrupted resident op.
pub(crate) unsafe fn release_operation_inputs(st: &mut EngineState) {
    for slot in st
        .input
        .input_file
        .iter_mut()
        .chain(&mut st.input.read_file)
    {
        let _ = crate::io::u_close(std::mem::take(slot));
    }
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

pub(crate) unsafe fn tt_cleanup_in(st: &mut EngineState) {
    crate::journal::disarm(st);
    reset_layout_font_caches();
    pdf_files_close();
    libc::free(TEX_FORMAT_DEFAULT as *mut _);
    st.shipout.dvi_buf = Vec::new();

    destroy_font_manager();
    crate::ext::release_linebreak_state(&mut st.fonts);

    st.fonts.font_layout_engine.fill(None);
    st.fonts.font_mapping.fill(None);
    st.fonts.loaded_font_mapping = None;

    for i in 1..=st.input.in_open as usize {
        // Teardown has begun, so a serious close error is absorbed.
        let _ = crate::io::u_close(std::mem::take(&mut st.input.input_file[i]));
    }
    for slot in st.input.read_file.iter_mut() {
        *slot = crate::state::InputSlot::Empty;
    }

    st.save_stack.data = Vec::new();

    let input = &mut st.input;
    input.name_of_input_file = None;
    input.abspath_of_input_file.0 = [0; 1024];
    input.nest = Vec::new();
    input.input_file = Vec::new();
    input.read_file = Default::default();
    input.input_stack = Vec::new();
    input.param_stack = Vec::new();
    input.line_stack = Vec::new();
    input.grp_stack = Vec::new();
    input.if_stack = Vec::new();
    input.source_filename_stack = Vec::new();
    input.full_source_filename_stack = Vec::new();
    input.eof_seen = Vec::new();
    input.buffer = Vec::new();
    input.name_of_file = Vec::new();
    input.name_of_file16 = Vec::new();

    st.scanner.native_text = Vec::new();
    st.scanner.native_text_size = 0;
    st.scanner.native_len = 0;
    st.scanner.save_native_len = 0;

    st.eqtb.data = Vec::new();
    st.eqtb.hash = Vec::new();
    st.mem.data = Vec::new();
    st.mem.glyph_blobs.clear();

    st.strings.str_start = Vec::new();
    st.strings.str_pool = Vec::new();

    let fonts = &mut st.fonts;
    fonts.font_check = Vec::new();
    fonts.font_size = Vec::new();
    fonts.font_dsize = Vec::new();
    fonts.font_params = Vec::new();
    fonts.font_name = Vec::new();
    fonts.font_area = Vec::new();
    fonts.font_bc = Vec::new();
    fonts.font_ec = Vec::new();
    fonts.bchar_label = Vec::new();
    fonts.font_bchar = Vec::new();
    fonts.font_false_bchar = Vec::new();
    fonts.font_glue = Vec::new();
    fonts.font_used = Vec::new();
    fonts.font_info = Vec::new();
    fonts.font_layout_engine = Vec::new();
    fonts.font_mapping = Vec::new();
    fonts.font_flags = Vec::new();
    fonts.font_letter_space = Vec::new();
    fonts.hyphen_char = Vec::new();
    fonts.skew_char = Vec::new();
    fonts.char_base = Vec::new();
    fonts.width_base = Vec::new();
    fonts.height_base = Vec::new();
    fonts.depth_base = Vec::new();
    fonts.italic_base = Vec::new();
    fonts.lig_kern_base = Vec::new();
    fonts.kern_base = Vec::new();
    fonts.exten_base = Vec::new();
    fonts.param_base = Vec::new();
    fonts.xdv_buffer = Vec::new();
    fonts.mapped_text = Vec::new();

    let hyph = &mut st.hyph;
    hyph.hyph_word = Vec::new();
    hyph.hyph_list = Vec::new();
    hyph.hyph_link = Vec::new();
    hyph.trie_trl = Vec::new();
    hyph.trie_tro = Vec::new();
    hyph.trie_trc = Vec::new();
    hyph.trie_c = Vec::new();
    hyph.trie_o = Vec::new();
    hyph.trie_l = Vec::new();
    hyph.trie_r = Vec::new();
    hyph.trie_hash = Vec::new();
    hyph.trie_taken = Vec::new();
}
