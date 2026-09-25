//! The main control loop, its command verbs, and the `\halign`/`\valign`
//! alignment family (= xetex0.c's main_control group).

use crate::consts::{
    ADJUST_HEAD, ASSIGN_GLUE, ASSIGN_INT, ASSIGN_MU_GLUE, A_LEADERS, BACKED_UP, BIGGEST_CHAR,
    BIGGEST_LANG, BOX_BASE, BOX_NODE_SIZE, CAT_CODE_BASE, CHAR_CLASS_LIMIT, CHAR_GIVEN,
    CONTRIB_HEAD, COUNT_BASE, CS_TOKEN_FLAG, CUR_FONT_LOC, DEL_CODE_BASE, DIMEN_BASE,
    DIMEN_PAR_HANG_INDENT, DIMEN_PAR_HSIZE, DISC_NODE, DLIST, ENDV, ETEX_PEN_BASE, EXPLICIT,
    FONT_BASE, GLUE_BASE, GLUE_NODE, GLUE_VAL, HLIST_NODE, HMODE, IGNORE_DEPTH, INSERTED, INT_BASE,
    INT_PAR_HANG_AFTER, INT_PAR_LANGUAGE, INT_PAR_LEFT_HYPHEN_MIN, INT_PAR_LOOSENESS,
    INT_PAR_RIGHT_HYPHEN_MIN, INT_PAR_TEXXET, KERN_NODE, LEFT_BRACE, LEFT_SIDE, LIGATURE_NODE,
    LIG_TAG, LOCAL_BASE, LOCAL_PAR_SHAPE, MARGIN_KERN_NODE, MARGIN_KERN_NODE_SIZE, MATH_CODE_BASE,
    MATH_NODE, MAX_COMMAND, MAX_HALFWORD, MMODE, MU_GLUE, MU_SKIP_BASE, MU_VAL_SCAN, NOAD_SIZE,
    NON_ADDRESS, NORMAL_SCANNER, NULL_CS, NULL_FLAG, OTHER_CHAR, OUTPUT_TEXT, PENALTY_NODE,
    PRIM_EQTB_BASE, REGISTER, RELAX, RIGHT_BRACE, RIGHT_SIDE, RULE_NODE, SCALED_BASE,
    SELECTOR_NEW_STRING, SELECTOR_NO_PRINT, SELECTOR_TERM_ONLY, SET_FONT, SF_CODE_BASE,
    SINGLE_BASE, SKIP_BASE, SMALL_NODE_SIZE, SPACER, SPACE_ADJUSTMENT, SUB_BOX, TEX_NULL,
    TOKEN_LIST, TOKS_BASE, TOO_BIG_CHAR, UNDEFINED_PRIMITIVE, VMODE,
};
use crate::ext::{append_src_special_in, apply_mapping, do_extension_in};
use crate::fonts::read_font_info_in;
use crate::hyphenation::{new_hyph_exceptions_in, new_patterns_in};
use crate::math::{after_math_in, init_math_in};
use crate::mem::is_char_node;
use crate::node::{AnyNode, GlueSpecNode, NodePtr, RuleDimen};
use crate::scan::{set_class_fn, set_family_fn};
use crate::state::{EngineState, GroupCode, MemState};
use crate::units::{GlueSign, MemIdx, MemPtr, NodeType, Scaled};

use crate::bridge::diag_append_str;

extern "C" {
    fn set_cp_code(font_num: i32, code: u32, side: i32, value: i32);
}

// Command codes (`cur_cmd`).
const MATH_SHIFT: u8 = 3;
const TAB_MARK: u8 = 4;
const CAR_RET: u8 = 5;
const MAC_PARAM: u8 = 6;
const SUP_MARK: u8 = 7;
const SUB_MARK: u8 = 8;
const LETTER: u8 = 11;
const PAR_END: u8 = 13;
const STOP: u8 = 14;
const DELIM_NUM: u8 = 15;
const CHAR_NUM: u8 = 16;
const MATH_CHAR_NUM: u8 = 17;
const MARK: u8 = 18;
const XRAY: u8 = 19;
const MAKE_BOX: u8 = 20;
const HMOVE: u8 = 21;
const VMOVE: u8 = 22;
const UN_HBOX: u8 = 23;
const UN_VBOX: u8 = 24;
const REMOVE_ITEM: u8 = 25;
const HSKIP: u8 = 26;
const VSKIP: u8 = 27;
const MSKIP: u8 = 28;
const KERN: u8 = 29;
const MKERN: u8 = 30;
const LEADER_SHIP: u8 = 31;
const HALIGN: u8 = 32;
const VALIGN: u8 = 33;
const NO_ALIGN: u8 = 34;
const VRULE: u8 = 35;
const HRULE: u8 = 36;
const INSERT: u8 = 37;
const VADJUST: u8 = 38;
const IGNORE_SPACES: u8 = 39;
const AFTER_ASSIGNMENT: u8 = 40;
const AFTER_GROUP: u8 = 41;
const BREAK_PENALTY: u8 = 42;
const START_PAR: u8 = 43;
const ITAL_CORR: u8 = 44;
const ACCENT: u8 = 45;
const MATH_ACCENT: u8 = 46;
const DISCRETIONARY: u8 = 47;
const EQ_NO: u8 = 48;
const LEFT_RIGHT: u8 = 49;
const MATH_COMP: u8 = 50;
const LIMIT_SWITCH: u8 = 51;
const ABOVE: u8 = 52;
const MATH_STYLE: u8 = 53;
const MATH_CHOICE: u8 = 54;
const NON_SCRIPT: u8 = 55;
const VCENTER: u8 = 56;
const CASE_SHIFT: u8 = 57;
const MESSAGE: u8 = 58;
const EXTENSION: u8 = 59;
const IN_STREAM: u8 = 60;
const BEGIN_GROUP: u8 = 61;
const END_GROUP: u8 = 62;
const OMIT: u8 = 63;
const EX_SPACE: u8 = 64;
const NO_BOUNDARY: u8 = 65;
const RADICAL: u8 = 66;
const END_CS_NAME: u8 = 67;
const MATH_GIVEN: u8 = 69;
const XETEX_MATH_GIVEN: u8 = 70;
const LAST_ITEM: u8 = 71;
const MAX_NON_PREFIXED_COMMAND: u8 = 71;
pub(crate) const TOKS_REGISTER: u8 = 72;
const ASSIGN_TOKS: u8 = 73;
const ASSIGN_DIMEN: u8 = 75;
const ASSIGN_FONT_DIMEN: u8 = 78;
const ASSIGN_FONT_INT: u8 = 79;
const SET_AUX: u8 = 80;
const SET_PREV_GRAF: u8 = 81;
const SET_PAGE_DIMEN: u8 = 82;
const SET_PAGE_INT: u8 = 83;
const SET_BOX_DIMEN: u8 = 84;
const SET_SHAPE: u8 = 85;
const DEF_CODE: u8 = 86;
const XETEX_DEF_CODE: u8 = 87;
const DEF_FAMILY: u8 = 88;
const DEF_FONT: u8 = 90;
const ADVANCE: u8 = 92;
const MULTIPLY: u8 = 93;
const DIVIDE: u8 = 94;
const PREFIX: u8 = 95;
const LET: u8 = 96;
const SHORTHAND_DEF: u8 = 97;
const READ_TO_CS: u8 = 98;
const DEF: u8 = 99;
const SET_BOX: u8 = 100;
const HYPH_DATA: u8 = 101;
const SET_INTERACTION: u8 = 102;

// Equivalent types beyond `MAX_COMMAND`.
const UNDEFINED_CS: u16 = 103;
pub(crate) const CALL: u16 = 113;
pub(crate) const LONG_OUTER_CALL: u16 = 116;
pub(crate) const GLUE_REF: u16 = 119;
pub(crate) const SHAPE_REF: u16 = 120;
pub(crate) const BOX_REF: u16 = 121;
const DATA: u16 = 122;

// Value levels (`cur_val_level`).
const INT_VAL: i16 = 0;
pub(crate) const DIMEN_VAL: i16 = 1;
pub(crate) const MU_VAL: i16 = 3;
const TOK_VAL: i16 = 5;
const INTER_CHAR_VAL: i16 = 6;

const INNER_VMODE: i16 = -VMODE;
const INNER_HMODE: i16 = -HMODE;

const ALIGNING: u8 = 4;

const PRE_ADJUST_HEAD: i32 = 4_999_985;
const END_SPAN: i32 = 4_999_990;
const ALIGN_HEAD: i32 = 4_999_991;
const HOLD_HEAD: i32 = 4_999_995;
const TEMP_HEAD: i32 = 4_999_996;
const PAGE_HEAD: i32 = 4_999_997;

const VLIST_NODE: u16 = 1;
const INS_NODE: u16 = 3;
const MARK_NODE: u16 = 4;
const ADJUST_NODE: u16 = 5;
const WHATSIT_NODE: u16 = 8;
const UNSET_NODE: u16 = 13;
const LANGUAGE_NODE_SUBTYPE: i16 = 4;

const ALIGN_STACK_NODE_SIZE: i32 = 6;
const INS_NODE_SIZE: i32 = 5;
const NATIVE_NODE_SIZE: i32 = 6;
const SPAN_NODE_SIZE: i32 = 2;

const NORMAL: u16 = 0;
const ADDITIONAL: i16 = 1;
const ACC_KERN: u16 = 2;
const TAB_SKIP_SUBTYPE: u16 = GLUE_PAR_TAB_SKIP as u16 + 1;
const COND_MATH_GLUE: u16 = 98;
const FIL: usize = 1;
const FILL: usize = 2;
const FILLL: usize = 3;
const BEGIN_M_CODE: u16 = 2;
const END_M_CODE: u16 = 3;

// Token-list types.
const U_TEMPLATE: u16 = 1;
const V_TEMPLATE: u16 = 2;
const BACKED_UP_CHAR: u16 = 4;
const EVERY_PAR_TEXT: u16 = 8;
const EVERY_HBOX_TEXT: u16 = 11;
const EVERY_VBOX_TEXT: u16 = 12;
const EVERY_JOB_TEXT: u16 = 13;
const EVERY_CR_TEXT: u16 = 14;
const INTER_CHAR_TEXT: u16 = 17;

const LEFT_BRACE_TOKEN: i32 = 0x20_0000;
const RIGHT_BRACE_TOKEN: i32 = 0x40_0000;
const MATH_SHIFT_TOKEN: i32 = 0x60_0000;
const TAB_TOKEN: i32 = 0x80_0000;
const OTHER_TOKEN: i32 = 0x180_0000;
const PROTECTED_TOKEN: i32 = 0x1C0_0000 + 1;

const HASH_BASE: i32 = 2_228_226;
const FROZEN_CR: i32 = 2_243_226 + 1;
const FROZEN_END_GROUP: i32 = 2_243_226 + 2;
const FROZEN_RIGHT: i32 = 2_243_226 + 3;
const FROZEN_END_TEMPLATE: i32 = 2_243_226 + 5;
const FONT_ID_BASE: i32 = 2_245_338;

const LOCAL_OUTPUT_ROUTINE: i32 = 1;
const LOCAL_EVERY_PAR: i32 = 2;
const LOCAL_EVERY_HBOX: i32 = 5;
const LOCAL_EVERY_VBOX: i32 = 6;
const LOCAL_EVERY_JOB: i32 = 7;
const LOCAL_EVERY_CR: i32 = 8;
const LOCAL_XETEX_INTER_CHAR_TOKS: i32 = 11;

const GLUE_PAR_PAR_SKIP: i16 = 2;
const GLUE_PAR_ABOVE_DISPLAY: i16 = 3;
const GLUE_PAR_BELOW_DISPLAY: i16 = 4;
const GLUE_PAR_SPLIT_TOP_SKIP: i32 = 10;
const GLUE_PAR_TAB_SKIP: i16 = 11;
const GLUE_PAR_SPACE_SKIP: i16 = 12;
const GLUE_PAR_XSPACE_SKIP: i32 = 13;
const GLUE_BASE_TAB_SKIP: i32 = GLUE_BASE + GLUE_PAR_TAB_SKIP as i32;

const INT_PAR_PRE_DISPLAY_PENALTY: i32 = 11;
const INT_PAR_POST_DISPLAY_PENALTY: i32 = 12;
const INT_PAR_TRACING_LOST_CHARS: i32 = 35;
const INT_PAR_TRACING_COMMANDS: i32 = 36;
const INT_PAR_FLOATING_PENALTY: i32 = 42;
const INT_PAR_GLOBAL_DEFS: i32 = 43;
const INT_PAR_XETEX_DASH_BREAK: i32 = 70;
const INT_PAR_XETEX_UPWARDS: i32 = 71;
const INT_PAR_XETEX_USE_GLYPH_METRICS: i32 = 72;
const INT_PAR_XETEX_INTER_CHAR_TOKENS: i32 = 73;
const INT_PAR_XETEX_INTERWORD_SPACE_SHAPING: i32 = 78;

const DIMEN_PAR_SPLIT_MAX_DEPTH: i32 = 6;
const DIMEN_PAR_BOX_MAX_DEPTH: i32 = 7;
const DIMEN_PAR_DISPLAY_INDENT: i32 = 15;
const DIMEN_PAR_OVERFULL_RULE: i32 = 16;

// `cur_chr` modifiers.
const BOX_CODE: i32 = 0;
const COPY_CODE: i32 = 1;
const LAST_BOX_CODE: i32 = 2;
const VSPLIT_CODE: i32 = 3;
const VTOP_CODE: i16 = 4;
const SKIP_CODE: i16 = 4;
const CHAR_DEF_CODE: i32 = 0;
const MATH_CHAR_DEF_CODE: i32 = 1;
const COUNT_DEF_CODE: i32 = 2;
const DIMEN_DEF_CODE: i32 = 3;
const SKIP_DEF_CODE: i32 = 4;
const MU_SKIP_DEF_CODE: i32 = 5;
const TOKS_DEF_CODE: i32 = 6;
const CHAR_SUB_DEF_CODE: i32 = 7;
const XETEX_MATH_CHAR_NUM_DEF_CODE: i32 = 8;
const XETEX_MATH_CHAR_DEF_CODE: i32 = 9;
const SLANT_CODE: i32 = 1;
const X_HEIGHT_CODE: i32 = 5;
const EXTRA_SPACE_CODE: i32 = 7;
const LP_CODE_BASE: i32 = 2;
const RP_CODE_BASE: i32 = 3;
const SPAN_CODE: i32 = 1_114_113;
const CR_CODE: i32 = 1_114_114;
const CR_CR_CODE: i32 = 1_114_115;

const BOX_FLAG: i32 = 0x4000_0000;
const GLOBAL_BOX_FLAG: i32 = 0x4000_8000;
const SHIP_OUT_FLAG: i32 = 0x4001_0000;
const LEADER_FLAG: i32 = 0x4001_0001;

const EMPTY: i32 = 0;
const SUB_MLIST: i32 = 3;
const MAX_CHAR_CODE: i32 = 15;
const MAX_CHAR_VAL: i32 = 0x20_0000;
const ACTIVE_MATH_CHAR: i32 = 0x1F_FFFF;
const BIGGEST_USV: i32 = 0x10_FFFF;
const TOO_BIG_USV: i32 = 1_114_112;
const TEX_INFINITY: i32 = 0x7FFF_FFFF;
const AAT_FONT_FLAG: i32 = 0xFFFF;
const OTGR_FONT_FLAG: i32 = 0xFFFE;

pub(crate) fn align_error_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    if st.scanner.align_state.abs() > 2 {
        crate::output::error_here_with_diagnostic_in(st, c"Misplaced ".as_ptr());
        crate::output::print_cmd_chr_in(st, st.scanner.cur_cmd as u16, st.scanner.cur_chr);
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        if st.scanner.cur_tok == TAB_TOKEN + 38 {
            help(
                &mut st.errors,
                &[
                    c"I can't figure out why you would want to use a tab mark",
                    c"here. If you just want an ampersand, the remedy is",
                    c"simple: Just type `I\\&' now. But if some right brace",
                    c"up above has ended a previous alignment prematurely,",
                    c"you're probably due for more error messages, and you",
                    c"might try typing `S' now just to see what is salvageable.",
                ],
            );
        } else {
            help(
                &mut st.errors,
                &[
                    c"I can't figure out why you would want to use a tab mark",
                    c"or \\cr or \\span just now. If something like a right brace",
                    c"up above has ended a previous alignment prematurely,",
                    c"you're probably due for more error messages, and you",
                    c"might try typing `S' now just to see what is salvageable.",
                ],
            );
        }
        mc_propagate_error(crate::errors::error_in(st))?;
    } else {
        crate::input::back_input_in(st)?;
        if st.scanner.align_state < 0 {
            crate::output::error_here_with_diagnostic_in(st, c"Missing { inserted".as_ptr());
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.scanner.align_state += 1;
            st.scanner.cur_tok = LEFT_BRACE_TOKEN + 123;
        } else {
            crate::output::error_here_with_diagnostic_in(st, c"Missing } inserted".as_ptr());
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.scanner.align_state -= 1;
            st.scanner.cur_tok = RIGHT_BRACE_TOKEN + 125;
        }
        help(
            &mut st.errors,
            &[
                c"I've put in what seems to be necessary to fix",
                c"the current column of the current alignment.",
                c"Try to go on, since this might almost work.",
            ],
        );
        mc_propagate_error(crate::input::ins_error_in(st)?)?;
    }
    Ok(())
}
pub(crate) fn no_align_error_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    crate::output::error_here_with_diagnostic_in(st, c"Misplaced ".as_ptr());
    crate::output::print_esc_cstr_in(st, c"noalign".as_ptr());
    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
    help(
        &mut st.errors,
        &[
            c"I expect to see \\noalign only after the \\cr of",
            c"an alignment. Proceed, and I'll ignore this case.",
        ],
    );
    mc_propagate_error(crate::errors::error_in(st))
}
pub(crate) fn omit_error_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    crate::output::error_here_with_diagnostic_in(st, c"Misplaced ".as_ptr());
    crate::output::print_esc_cstr_in(st, c"omit".as_ptr());
    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
    help(
        &mut st.errors,
        &[
            c"I expect to see \\omit only after tab marks or the \\cr of",
            c"an alignment. Proceed, and I'll ignore this case.",
        ],
    );
    mc_propagate_error(crate::errors::error_in(st))
}
pub(crate) fn fin_align_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    use crate::mem::{
        alloc_glue, alloc_null_box, alloc_param_glue, alloc_penalty, delete_glue_ref, flush_list,
        free_node, pop_nest_in,
    };
    use crate::packaging::flush_node_list;
    use crate::scan::NodeRecycleAbort;

    if st.save_stack.cur_group != GroupCode::Align {
        return Err(MainControlAbort::Confusion(c"align1"));
    }
    crate::save::unsave_in(st)?;
    if st.save_stack.cur_group != GroupCode::Align {
        return Err(MainControlAbort::Confusion(c"align0"));
    }
    crate::save::unsave_in(st)?;

    let in_display = st.input.nest[(st.input.nest_ptr - 1) as usize].mode == MMODE;
    let o = if in_display {
        st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_DISPLAY_INDENT)
    } else {
        0
    };

    let mut q = st.mem.b32s1(st.mem.b32s1(ALIGN_HEAD));
    loop {
        let u_part = st.mem.b32s1(q + 3);
        flush_list(&mut st.mem, MemPtr::from_raw(u_part));
        let v_part = st.mem.b32s1(q + 2);
        flush_list(&mut st.mem, MemPtr::from_raw(v_part));
        let p = st.mem.b32s1(st.mem.b32s1(q));
        if st.mem.b32s1(q + 1) == NULL_FLAG {
            st.mem.set_b32s1(q + 1, 0);
            let r = st.mem.b32s1(q);
            let s = st.mem.b32s0(r + 1);
            if s != 0 {
                // add_glue_ref(zero_glue)
                st.mem.set_b32s1(0, st.mem.b32s1(0) + 1);
                delete_glue_ref(&mut st.mem, s);
                st.mem.set_b32s0(r + 1, 0);
            }
        }
        if st.mem.b32s0(q) != END_SPAN {
            let t = st.mem.b32s1(q + 1) + st.mem.b32s1(st.mem.b32s0(st.mem.b32s1(q) + 1) + 1);
            let mut r = st.mem.b32s0(q);
            let mut s = END_SPAN;
            st.mem.set_b32s0(s, p);
            let mut n: i32 = 1;
            loop {
                st.mem.set_b32s1(r + 1, st.mem.b32s1(r + 1) - t);
                let u = st.mem.b32s0(r);
                while st.mem.b32s1(r) > n {
                    s = st.mem.b32s0(s);
                    n = st.mem.b32s1(st.mem.b32s0(s)) + 1;
                }
                if st.mem.b32s1(r) < n {
                    st.mem.set_b32s0(r, st.mem.b32s0(s));
                    st.mem.set_b32s0(s, r);
                    st.mem.set_b32s1(r, st.mem.b32s1(r) - 1);
                    s = r;
                } else {
                    if st.mem.b32s1(r + 1) > st.mem.b32s1(st.mem.b32s0(s) + 1) {
                        st.mem.set_b32s1(st.mem.b32s0(s) + 1, st.mem.b32s1(r + 1));
                    }
                    free_node(&mut st.mem, MemPtr::from_raw(r), SPAN_NODE_SIZE);
                }
                r = u;
                if r == END_SPAN {
                    break;
                }
            }
        }
        st.mem.set_b16s1(q, UNSET_NODE);
        st.mem.set_b16s0(q, 0);
        st.mem.set_b32s1(q + 3, 0);
        st.mem.set_b32s1(q + 2, 0);
        st.mem.set_b16s0(q + 5, NORMAL);
        st.mem.set_b16s1(q + 5, NORMAL);
        st.mem.set_b32s1(q + 6, 0);
        st.mem.set_b32s1(q + 4, 0);
        q = p;
        if q == TEX_NULL {
            break;
        }
    }

    st.save_stack.save_ptr -= 2;
    st.scanner.pack_begin_line = -st.input.cur_list.mode_line;
    let spec_size = st.save_stack.b32s1(st.save_stack.save_ptr + 1);
    let spec_mode = st.save_stack.b32s1(st.save_stack.save_ptr) as i16;
    let head = st.mem.link_ptr(ALIGN_HEAD);
    let p = if st.input.cur_list.mode == INNER_VMODE {
        let rule_save = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_OVERFULL_RULE);
        st.eqtb.set_b32s1(DIMEN_BASE + DIMEN_PAR_OVERFULL_RULE, 0);
        let packed = crate::packaging::hpack_emit_in(st, head, spec_size, spec_mode)?;
        st.eqtb
            .set_b32s1(DIMEN_BASE + DIMEN_PAR_OVERFULL_RULE, rule_save);
        packed
    } else {
        move_preamble_dimen(&mut st.mem, 1, 3);
        let packed =
            crate::packaging::vpackage_emit_in(st, head, spec_size, spec_mode, MAX_HALFWORD)?;
        move_preamble_dimen(&mut st.mem, 3, 1);
        packed
    };
    st.scanner.pack_begin_line = 0;

    let mut q = st.mem.b32s1(st.input.cur_list.head);
    let mut s = st.input.cur_list.head.0;
    while q != TEX_NULL {
        if !is_char_node(&st.mem, q) {
            match st.mem.classify(MemIdx(q)) {
                Ok(AnyNode::Unset(_)) => {
                    if st.input.cur_list.mode == INNER_VMODE {
                        st.mem.set_b16s1(q, HLIST_NODE);
                        st.mem.set_b32s1(q + 1, p.width(&st.mem).0);
                        if in_display {
                            st.mem.set_b16s0(q, DLIST);
                        }
                    } else {
                        st.mem.set_b16s1(q, VLIST_NODE);
                        st.mem.set_b32s1(q + 3, p.height(&st.mem).0);
                    }
                    st.mem.set_b16s0(q + 5, p.glue_order(&st.mem));
                    st.mem.set_b16s1(q + 5, p.glue_sign(&st.mem).as_raw());
                    st.mem.set_gr(q + 6, p.glue_set(&st.mem));
                    st.mem.set_b32s1(q + 4, o);

                    let mut r = st.mem.b32s1(st.mem.b32s1(q + 5));
                    let mut ss = p.list_ptr(&st.mem).link(&st.mem).0;
                    loop {
                        let n = st.mem.b16s0(r);
                        let w = st.mem.b32s1(ss + 1);
                        let mut t = w;
                        let mut u = HOLD_HEAD;
                        st.mem.set_b16s0(r, 0);
                        for _ in 0..n {
                            ss = st.mem.b32s1(ss);
                            let v = st.mem.b32s0(ss + 1);
                            let spec = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(v))
                                .expect("non-null glue spec");
                            let g = alloc_glue(&mut st.mem, v).ok_or(NodeRecycleAbort::Overflow)?;
                            MemPtr(u).set_link(&mut st.mem, g.ptr());
                            u = g.ptr().0;
                            g.set_subtype(&mut st.mem, TAB_SKIP_SUBTYPE);
                            t += spec.width(&st.mem).0;
                            match p.glue_sign(&st.mem) {
                                GlueSign::Stretching
                                    if spec.stretch_order(&st.mem) == p.glue_order(&st.mem) =>
                                {
                                    t += crate::scaled_math::tex_round(
                                        p.glue_set(&st.mem) * spec.stretch(&st.mem).0 as f64,
                                    );
                                }
                                GlueSign::Shrinking
                                    if spec.shrink_order(&st.mem) == p.glue_order(&st.mem) =>
                                {
                                    t -= crate::scaled_math::tex_round(
                                        p.glue_set(&st.mem) * spec.shrink(&st.mem).0 as f64,
                                    );
                                }
                                _ => {}
                            }
                            ss = st.mem.b32s1(ss);
                            let nb =
                                alloc_null_box(&mut st.mem).ok_or(NodeRecycleAbort::Overflow)?;
                            MemPtr(u).set_link(&mut st.mem, nb.ptr());
                            u = nb.ptr().0;
                            let span_w = st.mem.b32s1(ss + 1);
                            t += span_w;
                            if st.input.cur_list.mode == INNER_VMODE {
                                nb.set_width(&mut st.mem, Scaled(span_w));
                            } else {
                                st.mem.set_b16s1(u, VLIST_NODE);
                                st.mem.set_b32s1(u + 3, span_w);
                            }
                        }
                        if st.input.cur_list.mode == INNER_VMODE {
                            st.mem.set_b32s1(r + 3, st.mem.b32s1(q + 3));
                            st.mem.set_b32s1(r + 2, st.mem.b32s1(q + 2));
                            let natural = st.mem.b32s1(r + 1);
                            set_unset_glue(&mut st.mem, r, t, natural);
                            st.mem.set_b32s1(r + 1, w);
                            st.mem.set_b16s1(r, HLIST_NODE);
                        } else {
                            st.mem.set_b32s1(r + 1, st.mem.b32s1(q + 1));
                            let natural = st.mem.b32s1(r + 3);
                            set_unset_glue(&mut st.mem, r, t, natural);
                            st.mem.set_b32s1(r + 3, w);
                            st.mem.set_b16s1(r, VLIST_NODE);
                        }
                        st.mem.set_b32s1(r + 4, 0);
                        if u != HOLD_HEAD {
                            st.mem.set_b32s1(u, st.mem.b32s1(r));
                            st.mem.set_b32s1(r, st.mem.b32s1(HOLD_HEAD));
                            r = u;
                        }
                        r = st.mem.b32s1(st.mem.b32s1(r));
                        ss = st.mem.b32s1(st.mem.b32s1(ss));
                        if r == TEX_NULL {
                            break;
                        }
                    }
                }
                Ok(AnyNode::Rule(rule)) => {
                    let (w, h, d) = (p.width(&st.mem), p.height(&st.mem), p.depth(&st.mem));
                    if rule.width(&st.mem) == RuleDimen::Running {
                        rule.set_width(&mut st.mem, RuleDimen::Set(w));
                    }
                    if rule.height(&st.mem) == RuleDimen::Running {
                        rule.set_height(&mut st.mem, RuleDimen::Set(h));
                    }
                    if rule.depth(&st.mem) == RuleDimen::Running {
                        rule.set_depth(&mut st.mem, RuleDimen::Set(d));
                    }
                    if o != 0 {
                        let next = rule.link(&st.mem);
                        rule.set_link(&mut st.mem, MemPtr::NULL);
                        let boxed = crate::packaging::hpack_emit_in(st, rule.ptr(), 0, ADDITIONAL)?;
                        boxed.set_shift_amount(&mut st.mem, Scaled(o));
                        boxed.set_link(&mut st.mem, next);
                        MemPtr(s).set_link(&mut st.mem, boxed.ptr());
                        q = boxed.idx().0;
                    }
                }
                _ => {}
            }
        }
        s = q;
        q = st.mem.b32s1(q);
    }

    flush_node_list(&mut st.mem, p.idx().0).map_err(MainControlAbort::Confusion)?;
    pop_alignment_in(st);

    let aux_save_s0 = st.input.cur_list.aux_s0();
    let aux_save_s1 = st.input.cur_list.aux_s1();
    let rows = st.mem.link_ptr(st.input.cur_list.head);
    let rows_tail = st.input.cur_list.tail;
    pop_nest_in(&mut st.mem, &mut st.input);

    if st.input.cur_list.mode == MMODE {
        do_assignments_in(st)?;
        if st.scanner.cur_cmd != MATH_SHIFT {
            crate::output::error_here_with_diagnostic_in(st, c"Missing $$ inserted".as_ptr());
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            help(
                &mut st.errors,
                &[
                    c"Displays can use special alignments (like \\eqalignno)",
                    c"only if nothing but the alignment itself is between $$'s.",
                ],
            );
            mc_propagate_error(crate::input::back_error_in(st)?)?;
        } else {
            crate::scan::get_x_token_in(st)?;
            if st.scanner.cur_cmd != MATH_SHIFT {
                crate::output::error_here_with_diagnostic_in(
                    st,
                    c"Display math should end with $$".as_ptr(),
                );
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                help(
                    &mut st.errors,
                    &[
                        c"The `$' that I just saw supposedly matches a previous `$$'.",
                        c"So I shall assume that you typed `$$' both times.",
                    ],
                );
                mc_propagate_error(crate::input::back_error_in(st)?)?;
            }
        }
        let etex_aux = st.input.cur_list.e_tex_aux.0;
        flush_node_list(&mut st.mem, etex_aux).map_err(MainControlAbort::Confusion)?;
        pop_nest_in(&mut st.mem, &mut st.input);
        let pre_penalty = st.eqtb.b32s1(INT_BASE + INT_PAR_PRE_DISPLAY_PENALTY);
        let pen = alloc_penalty(&mut st.mem, pre_penalty).ok_or(NodeRecycleAbort::Overflow)?;
        tail_append(st, pen.ptr());
        let above = alloc_param_glue(&mut st.mem, &st.eqtb, GLUE_PAR_ABOVE_DISPLAY)
            .ok_or(NodeRecycleAbort::Overflow)?;
        tail_append(st, above.ptr());
        st.input.cur_list.tail.set_link(&mut st.mem, rows);
        if !rows.is_null() {
            st.input.cur_list.tail = rows_tail;
        }
        let post_penalty = st.eqtb.b32s1(INT_BASE + INT_PAR_POST_DISPLAY_PENALTY);
        let pen = alloc_penalty(&mut st.mem, post_penalty).ok_or(NodeRecycleAbort::Overflow)?;
        tail_append(st, pen.ptr());
        let below = alloc_param_glue(&mut st.mem, &st.eqtb, GLUE_PAR_BELOW_DISPLAY)
            .ok_or(NodeRecycleAbort::Overflow)?;
        tail_append(st, below.ptr());
        st.input.cur_list.set_aux_s1(aux_save_s1);
        crate::math::resume_after_display_in(st)?;
    } else {
        st.input.cur_list.set_aux_s0(aux_save_s0);
        st.input.cur_list.set_aux_s1(aux_save_s1);
        st.input.cur_list.tail.set_link(&mut st.mem, rows);
        if !rows.is_null() {
            st.input.cur_list.tail = rows_tail;
        }
        if st.input.cur_list.mode == VMODE {
            crate::pagebuilder::build_page_in(st)?;
        }
    }
    Ok(())
}

/// Moves word `from` of every preamble record into word `to`, zeroing `from`.
fn move_preamble_dimen(mem: &mut MemState, from: i32, to: i32) {
    let mut q = mem.b32s1(mem.b32s1(ALIGN_HEAD));
    loop {
        mem.set_b32s1(q + to, mem.b32s1(q + from));
        mem.set_b32s1(q + from, 0);
        q = mem.b32s1(mem.b32s1(q));
        if q == TEX_NULL {
            break;
        }
    }
}

/// Sets the glue of unset node `r`, whose natural size is `natural`, as if its size were `t`.
fn set_unset_glue(mem: &mut MemState, r: i32, t: i32, natural: i32) {
    if t == natural {
        mem.set_b16s1(r + 5, NORMAL);
        mem.set_b16s0(r + 5, NORMAL);
        mem.set_gr(r + 6, 0.0);
    } else if t > natural {
        mem.set_b16s1(r + 5, GlueSign::Stretching.as_raw());
        let stretch = mem.b32s1(r + 6);
        let ratio = if stretch == 0 {
            0.0
        } else {
            (t - natural) as f64 / stretch as f64
        };
        mem.set_gr(r + 6, ratio);
    } else {
        mem.set_b16s0(r + 5, mem.b16s1(r + 5));
        mem.set_b16s1(r + 5, GlueSign::Shrinking.as_raw());
        let shrink = mem.b32s1(r + 4);
        let ratio = if shrink == 0 {
            0.0
        } else if mem.b16s0(r + 5) == NORMAL && natural - t > shrink {
            1.0
        } else {
            (natural - t) as f64 / shrink as f64
        };
        mem.set_gr(r + 6, ratio);
    }
}

pub(crate) fn align_peek_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    loop {
        st.scanner.align_state = 1_000_000;
        get_x_non_blank_or_protected(st)?;
        match st.scanner.cur_cmd {
            NO_ALIGN => {
                crate::scan::scan_left_brace_in(st)?;
                crate::save::new_save_level(&mut st.save_stack, GroupCode::NoAlign, st.mem.line)?;
                if st.input.cur_list.mode == INNER_VMODE {
                    normal_paragraph(st)?;
                }
                return Ok(());
            }
            RIGHT_BRACE => return fin_align_in(st),
            CAR_RET if st.scanner.cur_chr == CR_CR_CODE => {}
            _ => {
                init_row_in(st)?;
                return init_col_in(st);
            }
        }
    }
}

fn get_x_non_blank_or_protected(st: &mut EngineState) -> Result<(), MainControlAbort> {
    loop {
        crate::scan::get_x_or_protected_in(st)?;
        if st.scanner.cur_cmd != SPACER {
            return Ok(());
        }
    }
}

pub(crate) fn get_preamble_token_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    loop {
        crate::scan::get_token_in(st)?;
        while st.scanner.cur_chr == SPAN_CODE && st.scanner.cur_cmd == TAB_MARK {
            crate::scan::get_token_in(st)?;
            if st.scanner.cur_cmd > MAX_COMMAND {
                crate::expand::expand_in(st)?;
                crate::scan::get_token_in(st)?;
            }
        }
        if st.scanner.cur_cmd == ENDV {
            return Err(MainControlAbort::Input(crate::input::InputAbort::Fatal(
                c"(interwoven alignment preambles are not allowed)".as_ptr(),
            )));
        }
        if st.scanner.cur_cmd != ASSIGN_GLUE || st.scanner.cur_chr != GLUE_BASE_TAB_SKIP {
            return Ok(());
        }
        crate::scan::scan_optional_equals_in(st)?;
        crate::scan::scan_glue_in(st, GLUE_VAL)?;
        let global = st.eqtb.b32s1(INT_BASE + INT_PAR_GLOBAL_DEFS) > 0;
        define(st, global, GLUE_BASE_TAB_SKIP, GLUE_REF, st.scanner.cur_val)?;
    }
}

pub(crate) fn init_span_in(st: &mut EngineState, p: MemPtr) -> Result<(), MainControlAbort> {
    crate::mem::push_nest_in(st)?;
    if st.input.cur_list.mode == INNER_HMODE {
        st.input.cur_list.set_aux_s0(1000);
    } else {
        st.input.cur_list.set_aux_s1(IGNORE_DEPTH);
        normal_paragraph(st)?;
    }
    st.scanner.cur_span = p;
    Ok(())
}

pub(crate) fn init_row_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    crate::mem::push_nest_in(st)?;
    st.input.cur_list.mode = -HMODE - VMODE - st.input.cur_list.mode;
    if st.input.cur_list.mode == INNER_HMODE {
        st.input.cur_list.set_aux_s0(0);
    } else {
        st.input.cur_list.set_aux_s1(0);
    }
    let tab_skip = st.mem.link_ptr(ALIGN_HEAD);
    let spec = st.mem.b32s0(tab_skip + 1);
    let glue =
        crate::mem::alloc_glue(&mut st.mem, spec).ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
    tail_append(st, glue.ptr());
    glue.set_subtype(&mut st.mem, TAB_SKIP_SUBTYPE);
    st.scanner.cur_align = tab_skip.link(&st.mem);
    st.scanner.cur_tail = st.scanner.cur_head;
    st.scanner.cur_pre_tail = st.scanner.cur_pre_head;
    init_span_in(st, st.scanner.cur_align)
}

pub(crate) fn init_col_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    st.mem
        .set_b32s0(st.scanner.cur_align + 5, st.scanner.cur_cmd as i32);
    if st.scanner.cur_cmd == OMIT {
        st.scanner.align_state = 0;
    } else {
        crate::input::back_input_in(st)?;
        crate::input::begin_token_list_in(st, st.mem.b32s1(st.scanner.cur_align + 3), U_TEMPLATE)?;
    }
    Ok(())
}

pub(crate) fn push_alignment_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    let p = crate::scan::get_node_scc(st, ALIGN_STACK_NODE_SIZE)?;
    st.mem.set_b32s1(p, st.scanner.align_ptr.0);
    st.mem.set_b32s0(p, st.scanner.cur_align.0);
    st.mem.set_b32s0(p + 1, st.mem.b32s1(ALIGN_HEAD));
    st.mem.set_b32s1(p + 1, st.scanner.cur_span.0);
    st.mem.set_b32s1(p + 2, st.scanner.cur_loop.0);
    st.mem.set_b32s1(p + 3, st.scanner.align_state);
    st.mem.set_b32s0(p + 4, st.scanner.cur_head.0);
    st.mem.set_b32s1(p + 4, st.scanner.cur_tail.0);
    st.mem.set_b32s0(p + 5, st.scanner.cur_pre_head.0);
    st.mem.set_b32s1(p + 5, st.scanner.cur_pre_tail.0);
    st.scanner.align_ptr = p;
    st.scanner.cur_head = crate::scan::get_avail_scc(st)?;
    st.scanner.cur_pre_head = crate::scan::get_avail_scc(st)?;
    Ok(())
}

pub(crate) fn pop_alignment_in(st: &mut EngineState) {
    crate::mem::free_avail(&mut st.mem, st.scanner.cur_head);
    crate::mem::free_avail(&mut st.mem, st.scanner.cur_pre_head);
    let p = st.scanner.align_ptr;
    st.scanner.cur_tail.0 = st.mem.b32s1(p + 4);
    st.scanner.cur_head.0 = st.mem.b32s0(p + 4);
    st.scanner.cur_pre_tail.0 = st.mem.b32s1(p + 5);
    st.scanner.cur_pre_head.0 = st.mem.b32s0(p + 5);
    st.scanner.align_state = st.mem.b32s1(p + 3);
    st.scanner.cur_loop.0 = st.mem.b32s1(p + 2);
    st.scanner.cur_span.0 = st.mem.b32s1(p + 1);
    st.mem.set_b32s1(ALIGN_HEAD, st.mem.b32s0(p + 1));
    st.scanner.cur_align.0 = st.mem.b32s0(p);
    st.scanner.align_ptr.0 = st.mem.b32s1(p);
    crate::mem::free_node(&mut st.mem, p, ALIGN_STACK_NODE_SIZE);
}

/// TeX's `store_new_token`: appends `tok` after `p` and returns the new tail.
fn store_new_token(
    mem: &mut MemState,
    p: i32,
    tok: i32,
) -> Result<i32, crate::scan::NodeRecycleAbort> {
    let q = crate::mem::get_avail(mem).ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
    mem.set_b32s1(p, q.0);
    q.set_info(mem, tok);
    Ok(q.0)
}

/// Copies token list `r` through `HOLD_HEAD` and returns the copy.
fn copy_token_list(mem: &mut MemState, mut r: i32) -> Result<i32, crate::scan::NodeRecycleAbort> {
    let mut q = HOLD_HEAD;
    while r != TEX_NULL {
        q = store_new_token(mem, q, mem.b32s0(r))?;
        r = mem.b32s1(r);
    }
    mem.set_b32s1(q, TEX_NULL);
    Ok(mem.b32s1(HOLD_HEAD))
}

fn highest_order(totals: &[i32]) -> usize {
    if totals[FILLL] != 0 {
        FILLL
    } else if totals[FILL] != 0 {
        FILL
    } else if totals[FIL] != 0 {
        FIL
    } else {
        0
    }
}

pub(crate) fn fin_col_in(st: &mut EngineState) -> Result<bool, MainControlAbort> {
    use crate::scan::NodeRecycleAbort;

    if st.scanner.cur_align.is_null() {
        return Err(MainControlAbort::Confusion(c"endv"));
    }
    let q = st.mem.b32s1(st.scanner.cur_align);
    if q == TEX_NULL {
        return Err(MainControlAbort::Confusion(c"endv"));
    }
    if st.scanner.align_state < 500_000 {
        return Err(MainControlAbort::Input(crate::input::InputAbort::Fatal(
            c"(interwoven alignment preambles are not allowed)".as_ptr(),
        )));
    }
    let mut p = st.mem.b32s1(q);
    if p == TEX_NULL && st.mem.b32s0(st.scanner.cur_align + 5) < CR_CODE {
        if !st.scanner.cur_loop.is_null() {
            let nb = crate::mem::alloc_null_box(&mut st.mem).ok_or(NodeRecycleAbort::Overflow)?;
            MemPtr(q).set_link(&mut st.mem, nb.ptr());
            p = nb.idx().0;
            st.mem.set_b32s0(p, END_SPAN);
            nb.set_width(&mut st.mem, Scaled(NULL_FLAG));
            st.scanner.cur_loop = st.scanner.cur_loop.link(&st.mem);
            let u_template = st.mem.b32s1(st.scanner.cur_loop + 3);
            let u_part = copy_token_list(&mut st.mem, u_template)?;
            st.mem.set_b32s1(p + 3, u_part);
            let v_template = st.mem.b32s1(st.scanner.cur_loop + 2);
            let v_part = copy_token_list(&mut st.mem, v_template)?;
            st.mem.set_b32s1(p + 2, v_part);
            st.scanner.cur_loop = st.scanner.cur_loop.link(&st.mem);
            let spec = st.mem.b32s0(st.scanner.cur_loop + 1);
            let g = crate::mem::alloc_glue(&mut st.mem, spec).ok_or(NodeRecycleAbort::Overflow)?;
            nb.set_link(&mut st.mem, g.ptr());
            g.set_subtype(&mut st.mem, TAB_SKIP_SUBTYPE);
        } else {
            crate::output::error_here_with_diagnostic_in(
                st,
                c"Extra alignment tab has been changed to ".as_ptr(),
            );
            crate::output::print_esc_cstr_in(st, c"cr".as_ptr());
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            help(
                &mut st.errors,
                &[
                    c"You have given more \\span or & marks than there were",
                    c"in the preamble to the \\halign or \\valign now in progress.",
                    c"So I'll assume that you meant to type \\cr instead.",
                ],
            );
            st.mem.set_b32s0(st.scanner.cur_align + 5, CR_CODE);
            mc_propagate_error(crate::errors::error_in(st))?;
        }
    }
    if st.mem.b32s0(st.scanner.cur_align + 5) != SPAN_CODE {
        crate::save::unsave_in(st)?;
        crate::save::new_save_level(&mut st.save_stack, GroupCode::Align, st.mem.line)?;
        let head = st.mem.link_ptr(st.input.cur_list.head);
        let (u, w) = if st.input.cur_list.mode == INNER_HMODE {
            st.scanner.adjust_tail = st.scanner.cur_tail;
            st.scanner.pre_adjust_tail = st.scanner.cur_pre_tail;
            let u = crate::packaging::hpack_emit_in(st, head, 0, ADDITIONAL)?;
            st.scanner.cur_tail = st.scanner.adjust_tail;
            st.scanner.adjust_tail = MemPtr::NULL;
            st.scanner.cur_pre_tail = st.scanner.pre_adjust_tail;
            st.scanner.pre_adjust_tail = MemPtr::NULL;
            (u, u.width(&st.mem).0)
        } else {
            let u = crate::packaging::vpackage_emit_in(st, head, 0, ADDITIONAL, 0)?;
            (u, u.height(&st.mem).0)
        };
        let mut n: i32 = 0;
        if st.scanner.cur_span != st.scanner.cur_align {
            let mut q = st.scanner.cur_span.0;
            loop {
                n += 1;
                q = st.mem.b32s1(st.mem.b32s1(q));
                if q == st.scanner.cur_align.0 {
                    break;
                }
            }
            if n > u16::MAX as i32 {
                return Err(MainControlAbort::Confusion(c"too many spans"));
            }
            q = st.scanner.cur_span.0;
            while st.mem.b32s1(st.mem.b32s0(q)) < n {
                q = st.mem.b32s0(q);
            }
            if st.mem.b32s1(st.mem.b32s0(q)) > n {
                let s = crate::mem::get_node(&mut st.mem, SPAN_NODE_SIZE)
                    .ok_or(NodeRecycleAbort::Overflow)?;
                let next = st.mem.b32s0(q);
                s.set_info(&mut st.mem, next);
                s.set_link(&mut st.mem, MemPtr(n));
                st.mem.set_b32s0(q, s.0);
                st.mem.set_b32s1(s + 1, w);
            } else if st.mem.b32s1(st.mem.b32s0(q) + 1) < w {
                st.mem.set_b32s1(st.mem.b32s0(q) + 1, w);
            }
        } else if w > st.mem.b32s1(st.scanner.cur_align + 1) {
            st.mem.set_b32s1(st.scanner.cur_align + 1, w);
        }
        let u = u.idx().0;
        st.mem.set_b16s1(u, UNSET_NODE);
        st.mem.set_b16s0(u, n as u16);
        let o = highest_order(&st.scanner.total_stretch);
        st.mem.set_b16s0(u + 5, o as u16);
        st.mem.set_b32s1(u + 6, st.scanner.total_stretch[o]);
        let o = highest_order(&st.scanner.total_shrink);
        st.mem.set_b16s1(u + 5, o as u16);
        st.mem.set_b32s1(u + 4, st.scanner.total_shrink[o]);
        crate::mem::pop_nest_in(&mut st.mem, &mut st.input);
        tail_append(st, MemPtr(u));
        let spec = st.mem.b32s0(st.mem.b32s1(st.scanner.cur_align) + 1);
        let g = crate::mem::alloc_glue(&mut st.mem, spec).ok_or(NodeRecycleAbort::Overflow)?;
        tail_append(st, g.ptr());
        g.set_subtype(&mut st.mem, TAB_SKIP_SUBTYPE);
        if st.mem.b32s0(st.scanner.cur_align + 5) >= CR_CODE {
            return Ok(true);
        }
        init_span_in(st, MemPtr(p))?;
    }
    st.scanner.align_state = 1_000_000;
    get_x_non_blank_or_protected(st)?;
    st.scanner.cur_align = MemPtr(p);
    init_col_in(st)?;
    Ok(false)
}

fn append_list(st: &mut EngineState, head: MemPtr, tail: MemPtr) {
    let first = head.link(&st.mem);
    st.input.cur_list.tail.set_link(&mut st.mem, first);
    st.input.cur_list.tail = tail;
}

/// Inserts the `\every...` token list at `LOCAL_BASE + local`, if set.
fn begin_every(st: &mut EngineState, local: i32, kind: u16) -> Result<(), MainControlAbort> {
    let list = st.eqtb.b32s1(LOCAL_BASE + local);
    if list != TEX_NULL {
        crate::input::begin_token_list_in(st, list, kind)?;
    }
    Ok(())
}

pub(crate) fn fin_row_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    use crate::mem::pop_nest_in;

    let head = st.mem.link_ptr(st.input.cur_list.head);
    let p = if st.input.cur_list.mode == INNER_HMODE {
        let row = crate::packaging::hpack_emit_in(st, head, 0, ADDITIONAL)?;
        pop_nest_in(&mut st.mem, &mut st.input);
        if st.scanner.cur_pre_head != st.scanner.cur_pre_tail {
            append_list(st, st.scanner.cur_pre_head, st.scanner.cur_pre_tail);
        }
        crate::packaging::append_to_vlist(&mut st.input.cur_list, &mut st.mem, &st.eqtb, row)?;
        if st.scanner.cur_head != st.scanner.cur_tail {
            append_list(st, st.scanner.cur_head, st.scanner.cur_tail);
        }
        row
    } else {
        let row = crate::packaging::vpackage_emit_in(st, head, 0, ADDITIONAL, MAX_HALFWORD)?;
        pop_nest_in(&mut st.mem, &mut st.input);
        tail_append(st, row.ptr());
        st.input.cur_list.set_aux_s0(1000);
        row
    };
    let p = p.idx().0;
    st.mem.set_b16s1(p, UNSET_NODE);
    st.mem.set_b32s1(p + 6, 0);
    begin_every(st, LOCAL_EVERY_CR, EVERY_CR_TEXT)?;
    align_peek_in(st)
}

pub(crate) fn init_align_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    use crate::mem::{alloc_null_box, alloc_param_glue};
    use crate::scan::NodeRecycleAbort;

    let save_cs_ptr = st.scanner.cur_cs;
    push_alignment_in(st)?;
    st.scanner.align_state = -1_000_000;

    if st.input.cur_list.mode == MMODE
        && (st.input.cur_list.tail != st.input.cur_list.head
            || st.input.cur_list.aux_s1() != TEX_NULL)
    {
        crate::output::error_here_with_diagnostic_in(st, c"Improper ".as_ptr());
        crate::output::print_esc_cstr_in(st, c"halign".as_ptr());
        crate::output::print_cstr_in(st, c" inside $$'s".as_ptr());
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        help(
            &mut st.errors,
            &[
                c"Displays can use special alignments (like \\eqalignno)",
                c"only if nothing but the alignment itself is between $$'s.",
                c"So I've deleted the formulas that preceded this alignment.",
            ],
        );
        mc_propagate_error(crate::errors::error_in(st))?;
        crate::math::flush_math(&mut st.input.cur_list, &mut st.mem)
            .map_err(MainControlAbort::Confusion)?;
    }

    crate::mem::push_nest_in(st)?;
    if st.input.cur_list.mode == MMODE {
        st.input.cur_list.mode = INNER_VMODE;
        let prev_depth = st.input.nest[(st.input.nest_ptr - 2) as usize].aux_s1();
        st.input.cur_list.set_aux_s1(prev_depth);
    } else if st.input.cur_list.mode > 0 {
        st.input.cur_list.mode = -st.input.cur_list.mode;
    }

    crate::scan::scan_spec_in(st, GroupCode::Align, false)?;
    st.mem.set_b32s1(ALIGN_HEAD, TEX_NULL);
    st.scanner.cur_align = MemPtr(ALIGN_HEAD);
    st.scanner.cur_loop = MemPtr::NULL;
    st.scanner.scanner_status = ALIGNING;
    st.scanner.warning_index = save_cs_ptr;
    st.scanner.align_state = -1_000_000;

    loop {
        let g = alloc_param_glue(&mut st.mem, &st.eqtb, GLUE_PAR_TAB_SKIP)
            .ok_or(NodeRecycleAbort::Overflow)?
            .ptr();
        st.scanner.cur_align.set_link(&mut st.mem, g);
        st.scanner.cur_align = g;
        if st.scanner.cur_cmd == CAR_RET {
            break;
        }

        let mut p = HOLD_HEAD;
        st.mem.set_b32s1(p, TEX_NULL);
        loop {
            get_preamble_token_in(st)?;
            if st.scanner.cur_cmd == MAC_PARAM {
                break;
            }
            if (TAB_MARK..=CAR_RET).contains(&st.scanner.cur_cmd)
                && st.scanner.align_state == -1_000_000
            {
                if p == HOLD_HEAD && st.scanner.cur_loop.is_null() && st.scanner.cur_cmd == TAB_MARK
                {
                    st.scanner.cur_loop = st.scanner.cur_align;
                } else {
                    crate::output::error_here_with_diagnostic_in(
                        st,
                        c"Missing # inserted in alignment preamble".as_ptr(),
                    );
                    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                    help(
                        &mut st.errors,
                        &[
                            c"There should be exactly one # between &'s, when an",
                            c"\\halign or \\valign is being set up. In this case you had",
                            c"none, so I've put one in; maybe that will work.",
                        ],
                    );
                    mc_propagate_error(crate::input::back_error_in(st)?)?;
                    break;
                }
            } else if st.scanner.cur_cmd != SPACER || p != HOLD_HEAD {
                p = store_new_token(&mut st.mem, p, st.scanner.cur_tok)?;
            }
        }

        let nb = alloc_null_box(&mut st.mem).ok_or(NodeRecycleAbort::Overflow)?;
        st.scanner.cur_align.set_link(&mut st.mem, nb.ptr());
        st.scanner.cur_align = nb.ptr();
        st.mem.set_b32s0(nb.idx(), END_SPAN);
        nb.set_width(&mut st.mem, Scaled(NULL_FLAG));
        st.mem
            .set_b32s1(nb.idx().offset(3), st.mem.b32s1(HOLD_HEAD));

        p = HOLD_HEAD;
        st.mem.set_b32s1(p, TEX_NULL);
        loop {
            get_preamble_token_in(st)?;
            if (TAB_MARK..=CAR_RET).contains(&st.scanner.cur_cmd)
                && st.scanner.align_state == -1_000_000
            {
                break;
            }
            if st.scanner.cur_cmd == MAC_PARAM {
                crate::output::error_here_with_diagnostic_in(
                    st,
                    c"Only one # is allowed per tab".as_ptr(),
                );
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                help(
                    &mut st.errors,
                    &[
                        c"There should be exactly one # between &'s, when an",
                        c"\\halign or \\valign is being set up. In this case you had",
                        c"more than one, so I'm ignoring all but the first.",
                    ],
                );
                mc_propagate_error(crate::errors::error_in(st))?;
                continue;
            }
            p = store_new_token(&mut st.mem, p, st.scanner.cur_tok)?;
        }
        store_new_token(&mut st.mem, p, CS_TOKEN_FLAG + FROZEN_END_TEMPLATE)?;
        st.mem
            .set_b32s1(st.scanner.cur_align + 2, st.mem.b32s1(HOLD_HEAD));
    }

    st.scanner.scanner_status = NORMAL_SCANNER;
    crate::save::new_save_level(&mut st.save_stack, GroupCode::Align, st.mem.line)?;
    begin_every(st, LOCAL_EVERY_CR, EVERY_CR_TEXT)?;
    align_peek_in(st)
}

/// Enters math mode.
pub(crate) fn push_math_in(st: &mut EngineState, c: GroupCode) -> Result<(), MainControlAbort> {
    crate::mem::push_nest_in(st)?;
    st.input.cur_list.mode = -207; // -MMODE (xetex_format.h:370)
    st.input.cur_list.set_aux_s1(TEX_NULL);
    crate::save::new_save_level(&mut st.save_stack, c, st.mem.line)?;
    Ok(())
}
pub(crate) fn end_graf_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    if st.input.cur_list.mode == HMODE {
        if st.input.cur_list.head == st.input.cur_list.tail {
            crate::mem::pop_nest_in(&mut st.mem, &mut st.input);
        } else {
            match crate::linebreak::line_break_in(st, false) {
                crate::linebreak::LineBreakOutcome::Done => {}
                o => return Err(MainControlAbort::LineBreak(o)),
            }
        }
        let aux = st.input.cur_list.e_tex_aux.0;
        if aux != TEX_NULL {
            crate::mem::flush_list(&mut st.mem, MemPtr::from_raw(aux));
            st.input.cur_list.e_tex_aux = MemPtr::NULL;
        }
        normal_paragraph(st)?;
        st.errors.error_count = 0;
    }
    Ok(())
}
pub(crate) fn append_penalty_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    crate::scan::scan_int_in(st)?;
    let p = crate::mem::alloc_penalty(&mut st.mem, st.scanner.cur_val)
        .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
    tail_append(st, p.ptr());
    if st.input.cur_list.mode == VMODE {
        crate::pagebuilder::build_page_in(st)?;
    }
    Ok(())
}
pub(crate) fn normal_paragraph(st: &mut EngineState) -> Result<(), crate::save::SaveAbort> {
    use crate::save::eq_word_define;

    if st.eqtb.b32s1(INT_BASE + INT_PAR_LOOSENESS) != 0 {
        eq_word_define(
            &mut st.eqtb,
            &mut st.save_stack,
            INT_BASE + INT_PAR_LOOSENESS,
            0,
        )?;
    }
    if st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_HANG_INDENT) != 0 {
        eq_word_define(
            &mut st.eqtb,
            &mut st.save_stack,
            DIMEN_BASE + DIMEN_PAR_HANG_INDENT,
            0,
        )?;
    }
    if st.eqtb.b32s1(INT_BASE + INT_PAR_HANG_AFTER) != 1 {
        eq_word_define(
            &mut st.eqtb,
            &mut st.save_stack,
            INT_BASE + INT_PAR_HANG_AFTER,
            1,
        )?;
    }
    for loc in [LOCAL_BASE + LOCAL_PAR_SHAPE, ETEX_PEN_BASE] {
        if st.eqtb.b32s1(loc) != TEX_NULL {
            define(st, false, loc, SHAPE_REF, TEX_NULL)?;
        }
    }
    Ok(())
}

pub(crate) fn its_all_over_in(st: &mut EngineState) -> Result<bool, MainControlAbort> {
    if crate::errors::privileged_in(st).map_err(MainControlAbort::Error)? {
        if st.shipout.page_tail.0 == PAGE_HEAD
            && st.input.cur_list.head == st.input.cur_list.tail
            && st.shipout.dead_cycles == 0
        {
            return Ok(true);
        }
        crate::input::back_input_in(st)?;
        let nb = crate::mem::alloc_null_box(&mut st.mem)
            .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
        tail_append(st, nb.ptr());
        st.mem.set_b32s1(
            st.input.cur_list.tail.0 + 1,
            st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_HSIZE),
        );
        let ng = crate::mem::alloc_glue(&mut st.mem, 8)
            .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
        tail_append(st, ng.ptr());
        let np = crate::mem::alloc_penalty(&mut st.mem, NULL_FLAG)
            .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
        tail_append(st, np.ptr());
        crate::pagebuilder::build_page_in(st)?;
    }
    Ok(false)
}
pub(crate) fn indent_in_hmode(st: &mut EngineState) -> Result<(), crate::scan::NodeRecycleAbort> {
    if st.scanner.cur_chr > 0 {
        let p = crate::mem::alloc_null_box(&mut st.mem)
            .ok_or(crate::scan::NodeRecycleAbort::Overflow)?
            .ptr()
            .0;
        let dimen = st.eqtb.b32s1(DIMEN_BASE);
        st.mem.set_b32s1(p + 1, dimen);
        let p = if st.input.cur_list.mode.abs() == HMODE {
            st.input.cur_list.set_aux_s0(1000);
            p
        } else {
            let q = crate::mem::alloc_noad(&mut st.mem)
                .ok_or(crate::scan::NodeRecycleAbort::Overflow)?
                .ptr()
                .0;
            st.mem.set_b32s1(q + 1, SUB_BOX);
            st.mem.set_b32s0(q + 1, p);
            q
        };
        let tail = st.input.cur_list.tail.0;
        st.mem.set_b32s1(tail, p);
        let v = st.mem.b32s1(tail);
        st.input.cur_list.tail.0 = v;
    }
    Ok(())
}

pub(crate) fn head_for_vmode_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    if st.input.cur_list.mode < 0 {
        if st.scanner.cur_cmd != HRULE {
            off_save_in(st)?;
        } else {
            crate::output::error_here_with_diagnostic_in(st, c"You can't use `".as_ptr());
            crate::output::print_esc_cstr_in(st, c"hrule".as_ptr());
            crate::output::print_cstr_in(st, c"' here except with leaders".as_ptr());
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            help(
                &mut st.errors,
                &[
                    c"To put a horizontal rule in an hbox or an alignment,",
                    c"you should use \\leaders or \\hrulefill (see The TeXbook).",
                ],
            );
            mc_propagate_error(crate::errors::error_in(st))?;
        }
    } else {
        crate::input::back_input_in(st)?;
        st.scanner.cur_tok = st.input.par_token;
        crate::input::back_input_in(st)?;
        st.input.cur_input.index = INSERTED;
    }
    Ok(())
}
pub(crate) fn begin_insert_or_adjust_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    if st.scanner.cur_cmd == VADJUST {
        st.scanner.cur_val = 255;
    } else {
        crate::scan::scan_eight_bit_int_in(st)?;
        if st.scanner.cur_val == 255 {
            crate::output::error_here_with_diagnostic_in(st, c"You can't ".as_ptr());
            crate::output::print_esc_cstr_in(st, c"insert".as_ptr());
            crate::output::print_int_in(st, st.scanner.cur_val);
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            help(
                &mut st.errors,
                &[c"I'm changing to \\insert0; box 255 is special."],
            );
            mc_propagate_error(crate::errors::error_in(st))?;
            st.scanner.cur_val = 0;
        }
    }
    let sp = st.save_stack.save_ptr;
    st.save_stack.set_b32s1(sp, st.scanner.cur_val);
    if st.scanner.cur_cmd == VADJUST && crate::scan::scan_keyword_in(st, c"pre".as_ptr())? {
        st.save_stack.set_b32s1(sp + 1, 1);
    } else {
        st.save_stack.set_b32s1(sp + 1, 0);
    }
    st.save_stack.save_ptr = sp + 2;
    crate::save::new_save_level(&mut st.save_stack, GroupCode::Insert, st.mem.line)?;
    crate::scan::scan_left_brace_in(st)?;
    normal_paragraph(st)?;
    crate::mem::push_nest_in(st)?;
    st.input.cur_list.mode = -1; // -VMODE
    st.input.cur_list.set_aux_s1(IGNORE_DEPTH);
    Ok(())
}
pub(crate) fn do_endv_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    st.input.base_ptr = st.input.input_ptr;
    st.input.input_stack[st.input.base_ptr as usize] = st.input.cur_input;
    while st.input.input_stack[st.input.base_ptr as usize].index != V_TEMPLATE
        && st.input.input_stack[st.input.base_ptr as usize].loc == TEX_NULL
        && st.input.input_stack[st.input.base_ptr as usize].state == TOKEN_LIST
    {
        st.input.base_ptr -= 1;
    }
    if st.input.input_stack[st.input.base_ptr as usize].index != V_TEMPLATE
        || st.input.input_stack[st.input.base_ptr as usize].loc != TEX_NULL
        || st.input.input_stack[st.input.base_ptr as usize].state != TOKEN_LIST
    {
        return Err(MainControlAbort::Input(crate::input::InputAbort::Fatal(
            c"(interwoven alignment preambles are not allowed)".as_ptr(),
        )));
    }
    if st.save_stack.cur_group == GroupCode::Align {
        end_graf_in(st)?;
        if fin_col_in(st)? {
            fin_row_in(st)?;
        }
    } else {
        off_save_in(st)?;
    }
    Ok(())
}
pub(crate) fn unpackage_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    if st.scanner.cur_chr > COPY_CODE {
        let tail = st.input.cur_list.tail.0;
        st.mem
            .set_b32s1(tail, st.scanner.disc_ptr[st.scanner.cur_chr as usize].0);
        st.scanner.disc_ptr[st.scanner.cur_chr as usize] = MemPtr::NULL;
        // goto done — jump to tail-walk loop
    } else {
        let c = st.scanner.cur_chr;
        crate::scan::scan_register_num_in(st)?;
        let p = if st.scanner.cur_val < 256 {
            st.eqtb.b32s1(BOX_BASE + st.scanner.cur_val)
        } else {
            let __cv18 = st.scanner.cur_val;
            crate::save::find_sa_element(&mut st.mem, &mut st.scanner, 4, __cv18, false);
            if st.scanner.cur_ptr.0 == TEX_NULL {
                TEX_NULL
            } else {
                st.mem.b32s1(st.scanner.cur_ptr + 1)
            }
        };

        if p == TEX_NULL {
            return Ok(());
        }

        let mode = st.input.cur_list.mode.abs();
        let ntype = st.mem.b16s1(p);
        if mode == MMODE
            || (mode == VMODE && ntype != VLIST_NODE)
            || (mode == HMODE && ntype != HLIST_NODE)
        {
            crate::output::error_here_with_diagnostic_in(
                st,
                c"Incompatible list can't be unboxed".as_ptr(),
            );
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            help(
                &mut st.errors,
                &[
                    c"Sorry, Pandora. (You sneaky devil.)",
                    c"I refuse to unbox an \\hbox in vertical mode or vice versa.",
                    c"And I can't open any boxes in math mode.",
                ],
            );
            return mc_propagate_error(crate::errors::error_in(st));
        }

        let list_head = st.mem.b32s1(p + 5);
        let tail = st.input.cur_list.tail.0;
        if c == COPY_CODE {
            let copy = crate::packaging::copy_node_list(&mut st.mem, list_head)?;
            st.mem.set_b32s1(tail, copy);
        } else {
            st.mem.set_b32s1(tail, list_head);
            if st.scanner.cur_val < 256 {
                st.eqtb.set_b32s1(BOX_BASE + st.scanner.cur_val, TEX_NULL);
            } else {
                let __cv17 = st.scanner.cur_val;
                crate::save::find_sa_element(&mut st.mem, &mut st.scanner, 4, __cv17, false);
                let cp = st.scanner.cur_ptr.0;
                if cp != TEX_NULL {
                    st.mem.set_b32s1(cp + 1, TEX_NULL);
                    let rc = st.mem.b32s0(cp + 1) + 1;
                    st.mem.set_b32s0(cp + 1, rc);
                    crate::save::delete_sa_ref(&mut st.mem, &mut st.scanner, cp);
                }
            }
            crate::mem::free_node(&mut st.mem, MemPtr::from_raw(p), BOX_NODE_SIZE);
        }
    }

    // done: advance cur_list.tail.0 past any MARGIN_KERN_NODEs
    while st.mem.b32s1(st.input.cur_list.tail) != TEX_NULL {
        let r = st.mem.b32s1(st.input.cur_list.tail);
        if r < st.mem.hi_mem_min && st.mem.b16s1(r) == MARGIN_KERN_NODE {
            let tail = st.input.cur_list.tail.0;
            st.mem.set_b32s1(tail, st.mem.b32s1(r));
            crate::mem::free_node(&mut st.mem, MemPtr::from_raw(r), MARGIN_KERN_NODE_SIZE);
        } else {
            st.input.cur_list.tail.0 = st.mem.b32s1(st.input.cur_list.tail);
        }
    }
    Ok(())
}
pub(crate) fn delete_last_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    if st.input.cur_list.mode == VMODE && st.input.cur_list.tail.0 == st.input.cur_list.head.0 {
        // On page: only allowed for \lastskip when last_glue != MAX_HALFWORD
        if st.scanner.cur_chr as u16 != GLUE_NODE || st.shipout.last_glue.0 != MAX_HALFWORD {
            crate::errors::you_cant_in(st);
            help(
                &mut st.errors,
                &[
                    c"Sorry...I usually can't take things from the current page.",
                    c"Try `I\\vskip-\\lastskip' instead.",
                ],
            );
            if st.scanner.cur_chr as u16 == KERN_NODE {
                st.errors.help_line[0] = c"Try `I\\kern-\\lastkern' instead.".as_ptr();
            } else if st.scanner.cur_chr as u16 != GLUE_NODE {
                st.errors.help_line[0] = c"Perhaps you can make the output routine do it.".as_ptr();
            }
            return mc_propagate_error(crate::errors::error_in(st));
        }
        return Ok(());
    }

    let mut tx = st.input.cur_list.tail.0;

    // If tail is MATH END, back up one to find the actual last "deletable" node
    if tx < st.mem.hi_mem_min && st.mem.b16s1(tx) == MATH_NODE && st.mem.b16s0(tx) == END_M_CODE {
        let mut r = st.input.cur_list.head.0;
        loop {
            let next_r = st.mem.b32s1(r);
            if next_r == tx {
                tx = r;
                break;
            }
            r = next_r;
        }
    }

    if tx >= st.mem.hi_mem_min || st.mem.b16s1(tx) as i32 != st.scanner.cur_chr {
        return Ok(());
    }

    // Walk the list to find the node before tx
    let mut q = st.input.cur_list.head.0;
    let mut p = TEX_NULL;
    let mut r_save;
    let mut fm;
    loop {
        r_save = p;
        p = q;
        fm = false;
        if q < st.mem.hi_mem_min {
            if st.mem.b16s1(q) == DISC_NODE {
                let count = st.mem.b16s0(q);
                for _ in 0..count {
                    p = st.mem.b32s1(p);
                }
                if p == tx {
                    return Ok(());
                }
            } else if st.mem.b16s1(q) == MATH_NODE && st.mem.b16s0(q) == BEGIN_M_CODE {
                fm = true;
            }
        }
        q = st.mem.b32s1(p);
        if q == tx {
            break;
        }
    }

    // q == tx; p is the node just before tx; r_save is the one before p
    let q_next = st.mem.b32s1(tx);
    st.mem.set_b32s1(p, q_next);
    st.mem.set_b32s1(tx, TEX_NULL);
    if q_next == TEX_NULL {
        if fm {
            return Err(MainControlAbort::Confusion(c"tail1"));
        } else {
            st.input.cur_list.tail.0 = p;
        }
    } else if fm {
        st.input.cur_list.tail.0 = r_save;
        st.mem.set_b32s1(r_save, TEX_NULL);
        crate::packaging::flush_node_list(&mut st.mem, p).map_err(MainControlAbort::Confusion)?;
    }
    crate::packaging::flush_node_list(&mut st.mem, tx).map_err(MainControlAbort::Confusion)?;
    Ok(())
}
// Returns the deferred `xn_over_d` side effects (`tex_remainder`, plus
// `arith_error` on overflow); `None` when the spacing branch taken did no
// scaling arithmetic.
pub(crate) fn app_space(
    st: &mut EngineState,
) -> Result<Option<(i32, bool)>, crate::scan::NodeRecycleAbort> {
    let space_factor = st.input.cur_list.aux_s0();
    let cf = st.eqtb.b32s1(CUR_FONT_LOC);
    let q;
    let mut side_effects = None;
    if space_factor >= 2000 && st.eqtb.b32s1(GLUE_BASE + GLUE_PAR_XSPACE_SKIP) != 0 {
        q = crate::mem::alloc_param_glue(&mut st.mem, &st.eqtb, GLUE_PAR_XSPACE_SKIP as i16)
            .ok_or(crate::scan::NodeRecycleAbort::Overflow)?
            .ptr()
            .0;
    } else {
        if st.eqtb.b32s1(GLUE_BASE + GLUE_PAR_SPACE_SKIP as i32) != 0 {
            st.mainloop.main_p.0 = st.eqtb.b32s1(GLUE_BASE + GLUE_PAR_SPACE_SKIP as i32);
        } else {
            st.mainloop.main_p.0 = st.fonts.font_glue(cf);
            if st.mainloop.main_p.0 == TEX_NULL {
                st.mainloop.main_p = crate::mem::alloc_spec(&mut st.mem, 0)
                    .ok_or(crate::scan::NodeRecycleAbort::Overflow)?
                    .ptr();
                st.mainloop.main_k = st.fonts.param_base(cf) + 2;
                // width = space, stretch = space_stretch, shrink = space_shrink
                st.mem.set_b32s1(
                    st.mainloop.main_p + 1,
                    st.fonts.font_info_b32s1(st.mainloop.main_k),
                );
                st.mem.set_b32s1(
                    st.mainloop.main_p + 2,
                    st.fonts.font_info_b32s1(st.mainloop.main_k + 1),
                );
                st.mem.set_b32s1(
                    st.mainloop.main_p + 3,
                    st.fonts.font_info_b32s1(st.mainloop.main_k + 2),
                );
                st.fonts.set_font_glue(cf, st.mainloop.main_p.0);
            }
        }
        st.mainloop.main_p = crate::mem::alloc_spec(&mut st.mem, st.mainloop.main_p.0)
            .ok_or(crate::scan::NodeRecycleAbort::Overflow)?
            .ptr();
        if space_factor >= 2000 {
            // add extra_space to width
            let extra = st
                .fonts
                .font_info_b32s1(EXTRA_SPACE_CODE + st.fonts.param_base(cf));
            st.mem.set_b32s1(
                st.mainloop.main_p + 1,
                st.mem.b32s1(st.mainloop.main_p + 1) + extra,
            );
        }
        // scale stretch by space_factor / 1000
        let (new_stretch, _r1, ov1) = crate::scaled_math::xn_over_d_parts(
            st.mem.b32s1(st.mainloop.main_p + 2),
            space_factor,
            1000,
        );
        st.mem.set_b32s1(st.mainloop.main_p + 2, new_stretch);
        // scale shrink by 1000 / space_factor
        let (new_shrink, r2, ov2) = crate::scaled_math::xn_over_d_parts(
            st.mem.b32s1(st.mainloop.main_p + 3),
            1000,
            space_factor,
        );
        st.mem.set_b32s1(st.mainloop.main_p + 3, new_shrink);
        // `xn_over_d` sets `tex_remainder` (last call wins) and `arith_error`
        // on any overflow.
        side_effects = Some((r2, ov1 || ov2));
        q = crate::mem::alloc_glue(&mut st.mem, st.mainloop.main_p.0)
            .ok_or(crate::scan::NodeRecycleAbort::Overflow)?
            .ptr()
            .0;
        st.mem.set_b32s1(st.mainloop.main_p, TEX_NULL);
    }
    tail_append(st, MemPtr::from_raw(q));
    Ok(side_effects)
}

pub(crate) fn app_space_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    if let Some((remainder, overflowed)) = app_space(st)? {
        if overflowed {
            st.errors.arith_error = true;
        }
        st.errors.tex_remainder = remainder;
    }
    Ok(())
}
pub(crate) fn scan_box_in(st: &mut EngineState, box_context: i32) -> Result<(), MainControlAbort> {
    loop {
        crate::scan::get_x_token_in(st)?;
        if st.scanner.cur_cmd != SPACER && st.scanner.cur_cmd != RELAX {
            break;
        }
    }
    if st.scanner.cur_cmd == MAKE_BOX {
        begin_box_in(st, box_context)?;
    } else if box_context >= LEADER_FLAG
        && (st.scanner.cur_cmd == HRULE || st.scanner.cur_cmd == VRULE)
    {
        st.scanner.cur_box.0 = crate::scan::scan_rule_spec_in(st)?;
        box_end_in(st, box_context)?;
    } else {
        crate::output::error_here_with_diagnostic_in(
            st,
            c"A <box> was supposed to be here".as_ptr(),
        );
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        help(
            &mut st.errors,
            &[
                c"I was expecting to see \\hbox or \\vbox or \\copy or \\box or",
                c"something like that. So you might find something missing in",
                c"your output. But keep trying; you can fix this later.",
            ],
        );
        let outcome = crate::input::back_error_in(st)?;
        if !matches!(outcome, crate::errors::ErrorOutcome::Continue) {
            return Err(MainControlAbort::Error(outcome));
        }
    }
    Ok(())
}
/// Closes a box group, packs the inner list, and dispatches via [`box_end_in`].
pub(crate) fn package_in(st: &mut EngineState, c: i16) -> Result<(), MainControlAbort> {
    let d = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_BOX_MAX_DEPTH);
    let u = st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_UPWARDS);
    crate::save::unsave_in(st)?;
    st.save_stack.save_ptr -= 3;
    let v = st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_UPWARDS);
    st.eqtb.set_b32s1(INT_BASE + INT_PAR_XETEX_UPWARDS, u);
    if st.input.cur_list.mode == INNER_HMODE {
        let sp = st.save_stack.save_ptr;
        let list = st.mem.link_ptr(st.input.cur_list.head);
        let w = st.save_stack.b32s1(sp + 2);
        let m = st.save_stack.b32s1(sp + 1) as i16;
        st.scanner.cur_box.0 = crate::packaging::hpack_emit_in(st, list, w, m)?.idx().0;
    } else {
        let sp = st.save_stack.save_ptr;
        let list = st.mem.link_ptr(st.input.cur_list.head);
        let h = st.save_stack.b32s1(sp + 2);
        let m = st.save_stack.b32s1(sp + 1) as i16;
        let cb = crate::packaging::vpackage_emit_in(st, list, h, m, d)?;
        st.scanner.cur_box.0 = cb.idx().0;
        if c == VTOP_CODE {
            let mut hh = 0i32;
            let p = cb.list_ptr(&st.mem).0;
            if p != TEX_NULL && st.mem.b16s1(p) <= 2 {
                hh = st.mem.b32s1(p + 3);
            }
            let new_depth = cb.depth(&st.mem).0 - hh + cb.height(&st.mem).0;
            cb.set_depth(&mut st.mem, Scaled(new_depth));
            cb.set_height(&mut st.mem, Scaled(hh));
        }
    }
    st.eqtb.set_b32s1(INT_BASE + INT_PAR_XETEX_UPWARDS, v);
    crate::mem::pop_nest_in(&mut st.mem, &mut st.input);
    let ctx = st.save_stack.b32s1(st.save_stack.save_ptr);
    box_end_in(st, ctx)?;
    Ok(())
}
/// Starts a new paragraph (par-skip glue, nest push, hyphenation/language
/// setup, optional indent box + src-special, `\everypar`, page build).
pub(crate) fn new_graf_in(st: &mut EngineState, indented: bool) -> Result<(), MainControlAbort> {
    st.input.cur_list.prev_graf = 0;

    if st.input.cur_list.mode == VMODE || st.input.cur_list.head != st.input.cur_list.tail {
        let pg = crate::mem::alloc_param_glue(&mut st.mem, &st.eqtb, GLUE_PAR_PAR_SKIP)
            .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
        tail_append(st, pg.ptr());
    }

    crate::mem::push_nest_in(st)?;
    st.input.cur_list.mode = HMODE;
    st.input.cur_list.set_aux_s0(1000);

    let lang_val = st.eqtb.b32s1(INT_BASE + INT_PAR_LANGUAGE);
    let l: u8 = if lang_val <= 0 || lang_val > BIGGEST_LANG {
        0
    } else {
        lang_val as u8
    };
    st.hyph.cur_lang = l;
    st.input.cur_list.set_aux_s1(l as i32);
    let lhm = crate::helpers::norm_min(st.eqtb.b32s1(INT_BASE + INT_PAR_LEFT_HYPHEN_MIN)) as i32;
    let rhm = crate::helpers::norm_min(st.eqtb.b32s1(INT_BASE + INT_PAR_RIGHT_HYPHEN_MIN)) as i32;
    st.input.cur_list.prev_graf = (lhm * 64 + rhm) * 65536 + l as i32;

    if indented {
        let nb = crate::mem::alloc_null_box(&mut st.mem)
            .ok_or(crate::scan::NodeRecycleAbort::Overflow)?
            .ptr();
        st.input.cur_list.tail = nb;
        st.input.cur_list.head.set_link(&mut st.mem, nb);
        st.mem.set_b32s1(nb.0 + 1, st.eqtb.b32s1(DIMEN_BASE));
        if st.shipout.insert_src_special_every_par {
            crate::ext::insert_src_special_in(st)?;
        }
    }

    begin_every(st, LOCAL_EVERY_PAR, EVERY_PAR_TEXT)?;

    if st.input.nest_ptr == 1 {
        crate::pagebuilder::build_page_in(st)?;
    }
    Ok(())
}
pub(crate) enum NewCharacterAbort {
    Native(crate::fonts::NativeCharAbort),
    Error(crate::errors::ErrorOutcome),
    MainMemoryOverflow,
}

impl NewCharacterAbort {
    /// Map to the unified [`EngineAbort`] terminal. `MainMemoryOverflow`
    /// runs `runaway` first (via `main_memory_overflow_abort`).
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            NewCharacterAbort::Native(a) => a.into_engine_abort(),
            NewCharacterAbort::Error(outcome) => outcome.into_engine_abort(),
            NewCharacterAbort::MainMemoryOverflow => crate::mem::main_memory_overflow_abort(),
        }
    }
}

pub(crate) enum MainControlAbort {
    /// Cooperative resident-operation cancellation.
    Cancelled,
    /// Viewport horizon reached at a resumable command boundary.
    Horizoned,
    Scanner(crate::scan::SccAbort),
    NodeRecycle(crate::scan::NodeRecycleAbort),
    PageBuild(crate::pagebuilder::PageBuildAbort),
    LineBreak(crate::linebreak::LineBreakOutcome),
    PushNest(crate::mem::PushNestAbort),
    Vsplit(crate::packaging::VsplitAbort),
    Input(crate::input::InputAbort),
    Shipout(crate::shipout::ShipoutAbort),
    Save(crate::save::SaveAbort),
    SaMutation(crate::save::SaMutationAbort),
    Unsave(crate::save::UnsaveAbort),
    InsertSrc(crate::ext::InsertSrcAbort),
    IssueMessage(crate::output::IssueMessageAbort),
    ShowWhatever(crate::output::ShowWhateverAbort),
    OpenLog(crate::io::OpenLogAbort),
    OpenOrClose(crate::io::OpenOrCloseAbort),
    MakeTexString(crate::texmfmp::MakeTexStringAbort),
    ReadFontInfo(crate::fonts::ReadFontInfoAbort),
    Error(crate::errors::ErrorOutcome),
    Confusion(&'static std::ffi::CStr),
    NewCharacter(NewCharacterAbort),
    Trie(crate::hyphenation::TrieAbort),
    /// Serious internal error closing an input file (the C `ttstub_input_close`
    /// wrapper `longjmp`'d here); reached from `load_picture_in`.
    InputClose,
    /// A native-font measurement cold abort.
    NativeFont(crate::ext::NativeFontAbort),
}

impl From<crate::ext::NativeFontAbort> for MainControlAbort {
    fn from(e: crate::ext::NativeFontAbort) -> Self {
        MainControlAbort::NativeFont(e)
    }
}

impl From<crate::hyphenation::TrieAbort> for MainControlAbort {
    fn from(e: crate::hyphenation::TrieAbort) -> Self {
        MainControlAbort::Trie(e)
    }
}

impl From<NewCharacterAbort> for MainControlAbort {
    fn from(e: NewCharacterAbort) -> Self {
        MainControlAbort::NewCharacter(e)
    }
}

impl From<crate::scan::SccAbort> for MainControlAbort {
    fn from(e: crate::scan::SccAbort) -> Self {
        MainControlAbort::Scanner(e)
    }
}

impl From<crate::scan::NodeRecycleAbort> for MainControlAbort {
    fn from(e: crate::scan::NodeRecycleAbort) -> Self {
        MainControlAbort::NodeRecycle(e)
    }
}

impl From<crate::pagebuilder::PageBuildAbort> for MainControlAbort {
    fn from(e: crate::pagebuilder::PageBuildAbort) -> Self {
        MainControlAbort::PageBuild(e)
    }
}

impl From<crate::mem::PushNestAbort> for MainControlAbort {
    fn from(e: crate::mem::PushNestAbort) -> Self {
        MainControlAbort::PushNest(e)
    }
}

impl From<crate::packaging::VsplitAbort> for MainControlAbort {
    fn from(e: crate::packaging::VsplitAbort) -> Self {
        MainControlAbort::Vsplit(e)
    }
}

impl From<crate::input::InputAbort> for MainControlAbort {
    fn from(e: crate::input::InputAbort) -> Self {
        MainControlAbort::Input(e)
    }
}

impl From<crate::shipout::ShipoutAbort> for MainControlAbort {
    fn from(e: crate::shipout::ShipoutAbort) -> Self {
        MainControlAbort::Shipout(e)
    }
}

impl From<crate::save::SaMutationAbort> for MainControlAbort {
    fn from(e: crate::save::SaMutationAbort) -> Self {
        MainControlAbort::SaMutation(e)
    }
}

impl From<crate::save::SaveAbort> for MainControlAbort {
    fn from(e: crate::save::SaveAbort) -> Self {
        MainControlAbort::Save(e)
    }
}

impl From<crate::save::UnsaveAbort> for MainControlAbort {
    fn from(e: crate::save::UnsaveAbort) -> Self {
        MainControlAbort::Unsave(e)
    }
}

impl From<crate::ext::InsertSrcAbort> for MainControlAbort {
    fn from(e: crate::ext::InsertSrcAbort) -> Self {
        MainControlAbort::InsertSrc(e)
    }
}

impl From<crate::output::IssueMessageAbort> for MainControlAbort {
    fn from(e: crate::output::IssueMessageAbort) -> Self {
        MainControlAbort::IssueMessage(e)
    }
}

impl From<crate::output::ShowWhateverAbort> for MainControlAbort {
    fn from(e: crate::output::ShowWhateverAbort) -> Self {
        MainControlAbort::ShowWhatever(e)
    }
}

impl From<crate::io::OpenLogAbort> for MainControlAbort {
    fn from(e: crate::io::OpenLogAbort) -> Self {
        MainControlAbort::OpenLog(e)
    }
}

impl From<crate::io::OpenOrCloseAbort> for MainControlAbort {
    fn from(e: crate::io::OpenOrCloseAbort) -> Self {
        MainControlAbort::OpenOrClose(e)
    }
}

impl From<crate::texmfmp::MakeTexStringAbort> for MainControlAbort {
    fn from(e: crate::texmfmp::MakeTexStringAbort) -> Self {
        MainControlAbort::MakeTexString(e)
    }
}

impl From<crate::fonts::ReadFontInfoAbort> for MainControlAbort {
    fn from(e: crate::fonts::ReadFontInfoAbort) -> Self {
        MainControlAbort::ReadFontInfo(e)
    }
}

impl MainControlAbort {
    /// Map the aggregated main-control cold abort to the unified [`EngineAbort`]
    /// terminal, delegating to each leaf's `into_engine_abort`.
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            MainControlAbort::Cancelled => crate::errors::EngineAbort::Cancelled,
            MainControlAbort::Horizoned => crate::errors::EngineAbort::Horizoned,
            MainControlAbort::Scanner(e) => e.into_engine_abort(),
            MainControlAbort::NodeRecycle(e) => e.into_engine_abort(),
            MainControlAbort::PageBuild(e) => e.into_engine_abort(),
            MainControlAbort::LineBreak(o) => o.into_engine_abort(),
            MainControlAbort::PushNest(e) => e.into_engine_abort(),
            MainControlAbort::Vsplit(a) => a.into_engine_abort(),
            MainControlAbort::Input(e) => e.into_engine_abort(),
            MainControlAbort::Shipout(e) => e.into_engine_abort(),
            MainControlAbort::Save(a) => a.into_engine_abort(),
            MainControlAbort::SaMutation(a) => a.into_engine_abort(),
            MainControlAbort::Unsave(a) => a.into_engine_abort(),
            MainControlAbort::InsertSrc(a) => a.into_engine_abort(),
            MainControlAbort::IssueMessage(a) => a.into_engine_abort(),
            MainControlAbort::ShowWhatever(a) => a.into_engine_abort(),
            MainControlAbort::OpenLog(a) => a.into_engine_abort(),
            MainControlAbort::OpenOrClose(a) => a.into_engine_abort(),
            MainControlAbort::MakeTexString(a) => a.into_engine_abort(),
            MainControlAbort::ReadFontInfo(a) => a.into_engine_abort(),
            MainControlAbort::Error(o) => o.into_engine_abort(),
            MainControlAbort::Confusion(m) => crate::errors::EngineAbort::Confusion(m.as_ptr()),
            MainControlAbort::NewCharacter(a) => a.into_engine_abort(),
            MainControlAbort::Trie(e) => e.into_engine_abort(),
            MainControlAbort::InputClose => crate::errors::EngineAbort::TtAbort(
                c"serious internal error closing an input file".as_ptr(),
            ),
            MainControlAbort::NativeFont(e) => e.into_engine_abort(),
        }
    }
}

/// Propagate a recoverable [`crate::errors::ErrorOutcome`] from a
/// main-control verb: `Continue` returns `Ok(())`, any cold tail rides
/// [`MainControlAbort::Error`].
#[inline]
pub(crate) fn mc_propagate_error(o: crate::errors::ErrorOutcome) -> Result<(), MainControlAbort> {
    if matches!(o, crate::errors::ErrorOutcome::Continue) {
        Ok(())
    } else {
        Err(MainControlAbort::Error(o))
    }
}

#[inline]
fn tail_append(st: &mut EngineState, p: MemPtr) {
    st.input.cur_list.tail.set_link(&mut st.mem, p);
    st.input.cur_list.tail = p;
}

/// TeX's `help0`..`help6`: `lines` in reading order.
pub(crate) fn help(errors: &mut crate::state::ErrorState, lines: &[&'static std::ffi::CStr]) {
    errors.help_ptr = lines.len() as u8;
    for (slot, line) in errors.help_line.iter_mut().zip(lines.iter().rev()) {
        *slot = line.as_ptr();
    }
}

pub(crate) fn new_character_in(
    st: &mut EngineState,
    f: i32,
    c: u16,
) -> Result<i32, NewCharacterAbort> {
    let fa = st.fonts.font_area_entry(f);
    if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
        return crate::fonts::new_native_character_in(st, f, c as u32)
            .map_err(NewCharacterAbort::Native);
    }
    let ec = crate::fonts::effective_char(&mut st.fonts, false, f, c) as u16;
    if st.fonts.font_bc_entry(f) <= ec && st.fonts.font_ec_entry(f) >= ec {
        let info = st
            .fonts
            .font_info_b16(st.fonts.char_base_entry(f) + ec as i32);
        if info.s3 > 0 {
            let p =
                crate::mem::get_avail(&mut st.mem).ok_or(NewCharacterAbort::MainMemoryOverflow)?;
            st.mem.set_b16s1(p, f as u16);
            st.mem.set_b16s0(p, c);
            return Ok(p.0);
        }
    }
    let outcome = crate::output::char_warning_in(st, f, c as i32);
    if !matches!(outcome, crate::errors::ErrorOutcome::Continue) {
        return Err(NewCharacterAbort::Error(outcome));
    }
    Ok(TEX_NULL)
}

pub(crate) fn box_end_in(st: &mut EngineState, box_context: i32) -> Result<(), MainControlAbort> {
    if box_context < BOX_FLAG {
        let cb = st.scanner.cur_box.0;
        if cb != TEX_NULL {
            st.mem.set_b32s1(cb + 4, box_context);

            if st.input.cur_list.mode.abs() == VMODE {
                if st.scanner.pre_adjust_tail.0 != TEX_NULL {
                    if PRE_ADJUST_HEAD != st.scanner.pre_adjust_tail.0 {
                        let tail = st.input.cur_list.tail.0;
                        st.mem.set_b32s1(tail, st.mem.b32s1(PRE_ADJUST_HEAD));
                        st.input.cur_list.tail.0 = st.scanner.pre_adjust_tail.0;
                    }
                    st.scanner.pre_adjust_tail.0 = TEX_NULL;
                }

                let b = st
                    .mem
                    .classify_box(MemIdx::new(cb))
                    .expect("valid node type for append_to_vlist")
                    .expect("append_to_vlist receives an hlist or vlist node");
                crate::packaging::append_to_vlist(
                    &mut st.input.cur_list,
                    &mut st.mem,
                    &st.eqtb,
                    b,
                )?;

                if st.scanner.adjust_tail.0 != TEX_NULL {
                    if ADJUST_HEAD != st.scanner.adjust_tail.0 {
                        let tail = st.input.cur_list.tail.0;
                        st.mem.set_b32s1(tail, st.mem.b32s1(ADJUST_HEAD));
                        st.input.cur_list.tail.0 = st.scanner.adjust_tail.0;
                    }
                    st.scanner.adjust_tail.0 = TEX_NULL;
                }

                if st.input.cur_list.mode > 0 {
                    crate::pagebuilder::build_page_in(st)?;
                }
            } else {
                if st.input.cur_list.mode.abs() == HMODE {
                    st.input.cur_list.set_aux_s0(1000); // space_factor
                } else {
                    let p = crate::mem::alloc_noad(&mut st.mem)
                        .ok_or(crate::scan::NodeRecycleAbort::Overflow)?
                        .ptr()
                        .0;
                    st.mem.set_b32s1(p + 1, SUB_BOX);
                    st.mem.set_b32s0(p + 1, cb);
                    st.scanner.cur_box.0 = p;
                }

                let cb2 = st.scanner.cur_box.0;
                let tail = st.input.cur_list.tail.0;
                st.mem.set_b32s1(tail, cb2);
                st.input.cur_list.tail.0 = cb2;
            }
        }
    } else if box_context < SHIP_OUT_FLAG {
        let (val, a) = if box_context < GLOBAL_BOX_FLAG {
            (box_context - BOX_FLAG, 0i16)
        } else {
            (box_context - GLOBAL_BOX_FLAG, 4i16)
        };
        st.scanner.cur_val = val;
        let cb = st.scanner.cur_box.0;

        if st.scanner.cur_val < 256 {
            let abort = if a >= 4 {
                let __cv16 = st.scanner.cur_val;
                crate::save::geq_define(
                    &mut st.eqtb,
                    &mut st.mem,
                    &mut st.scanner,
                    BOX_BASE + __cv16,
                    BOX_REF,
                    cb,
                )
            } else {
                let __cv15 = st.scanner.cur_val;
                crate::save::eq_define(
                    &mut st.eqtb,
                    &mut st.save_stack,
                    &mut st.mem,
                    &mut st.scanner,
                    BOX_BASE + __cv15,
                    BOX_REF,
                    cb,
                )
            };
            abort?;
        } else {
            let __cv14 = st.scanner.cur_val;
            crate::save::find_sa_element(&mut st.mem, &mut st.scanner, 4, __cv14, true)
                .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
            let p = st.scanner.cur_ptr.0;
            let result = if a >= 4 {
                crate::save::gsa_def(&mut st.mem, p, cb)
            } else {
                crate::save::sa_def(&mut st.mem, &mut st.scanner, &mut st.save_stack, p, cb)?
            };
            crate::save::finish_sa_mutation_in(st, p, result)?;
        }
    } else if st.scanner.cur_box.0 != TEX_NULL {
        if box_context > SHIP_OUT_FLAG {
            loop {
                crate::scan::get_x_token_in(st)?;
                if st.scanner.cur_cmd != SPACER && st.scanner.cur_cmd != RELAX {
                    break;
                }
            }

            if (st.scanner.cur_cmd == HSKIP && st.input.cur_list.mode.abs() != VMODE)
                || (st.scanner.cur_cmd == VSKIP && st.input.cur_list.mode.abs() == VMODE)
            {
                append_glue_in(st)?;
                let tail = st.input.cur_list.tail.0;
                st.mem.set_b16s0(
                    tail,
                    (box_context - (LEADER_FLAG - A_LEADERS as i32)) as u16,
                );
                let cb = st.scanner.cur_box.0;
                st.mem.set_b32s1(tail + 1, cb);
            } else {
                crate::output::error_here_with_diagnostic_in(
                    st,
                    c"Leaders not followed by proper glue".as_ptr(),
                );
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                help(
                    &mut st.errors,
                    &[
                        c"You should say `\\leaders <box or rule><hskip or vskip>'.",
                        c"I found the <box or rule>, but there's no suitable",
                        c"<hskip or vskip>, so I'm ignoring these leaders.",
                    ],
                );
                let outcome = crate::input::back_error_in(st)?;
                if !matches!(outcome, crate::errors::ErrorOutcome::Continue) {
                    return Err(MainControlAbort::Error(outcome));
                }
                let cb = st.scanner.cur_box.0;
                crate::packaging::flush_node_list(&mut st.mem, cb)
                    .map_err(MainControlAbort::Confusion)?;
            }
        } else {
            let cb = st.scanner.cur_box.0;
            unsafe {
                crate::shipout::ship_out_in(st, cb)?;
            }
        }
    }
    Ok(())
}
pub(crate) fn begin_box_in(st: &mut EngineState, box_context: i32) -> Result<(), MainControlAbort> {
    match st.scanner.cur_chr {
        BOX_CODE => {
            crate::scan::scan_register_num_in(st)?;
            let v = st.scanner.cur_val;
            if v < 256 {
                st.scanner.cur_box.0 = st.eqtb.b32s1(BOX_BASE + v);
            } else {
                crate::save::find_sa_element(&mut st.mem, &mut st.scanner, 4, v, false)
                    .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
                let cp = st.scanner.cur_ptr.0;
                st.scanner.cur_box.0 = if cp == TEX_NULL {
                    TEX_NULL
                } else {
                    st.mem.b32s1(cp + 1)
                };
            }
            if v < 256 {
                st.eqtb.set_b32s1(BOX_BASE + v, TEX_NULL);
            } else {
                crate::save::find_sa_element(&mut st.mem, &mut st.scanner, 4, v, false)
                    .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
                let cp = st.scanner.cur_ptr.0;
                if cp != TEX_NULL {
                    st.mem.set_b32s1(cp + 1, TEX_NULL);
                    st.mem.set_b32s0(cp + 1, st.mem.b32s0(cp + 1) + 1);
                    crate::save::delete_sa_ref(&mut st.mem, &mut st.scanner, cp);
                }
            }
        }

        COPY_CODE => {
            crate::scan::scan_register_num_in(st)?;
            let v = st.scanner.cur_val;
            let q = if v < 256 {
                st.eqtb.b32s1(BOX_BASE + v)
            } else {
                crate::save::find_sa_element(&mut st.mem, &mut st.scanner, 4, v, false)
                    .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
                let cp = st.scanner.cur_ptr.0;
                if cp == TEX_NULL {
                    TEX_NULL
                } else {
                    st.mem.b32s1(cp + 1)
                }
            };
            st.scanner.cur_box.0 = crate::packaging::copy_node_list(&mut st.mem, q)?;
        }

        LAST_BOX_CODE => {
            st.scanner.cur_box.0 = TEX_NULL;

            if st.input.cur_list.mode.abs() == MMODE {
                crate::errors::you_cant_in(st);
                help(&mut st.errors, &[c"Sorry; this \\lastbox will be void."]);
                let o = crate::errors::error_in(st);
                if !matches!(o, crate::errors::ErrorOutcome::Continue) {
                    return Err(MainControlAbort::Error(o));
                }
            } else if st.input.cur_list.mode == VMODE
                && st.input.cur_list.head == st.input.cur_list.tail
            {
                crate::errors::you_cant_in(st);
                help(
                    &mut st.errors,
                    &[
                        c"Sorry...I usually can't take things from the current page.",
                        c"This \\lastbox will therefore be void.",
                    ],
                );
                let o = crate::errors::error_in(st);
                if !matches!(o, crate::errors::ErrorOutcome::Continue) {
                    return Err(MainControlAbort::Error(o));
                }
            } else {
                let mut tx = st.input.cur_list.tail.0;
                if tx < st.mem.hi_mem_min
                    && st.mem.b16s1(tx) == MATH_NODE
                    && st.mem.b16s0(tx) == END_M_CODE
                {
                    let mut r = st.input.cur_list.head.0;
                    loop {
                        let prev = r;
                        r = st.mem.b32s1(prev);
                        if r == tx {
                            tx = prev;
                            break;
                        }
                    }
                }
                'last_box_done: {
                    if tx < st.mem.hi_mem_min {
                        let nt = st.mem.b16s1(tx);
                        if nt == HLIST_NODE || nt == VLIST_NODE {
                            let mut q = st.input.cur_list.head.0;
                            let mut p = TEX_NULL;
                            let mut r;
                            let mut fm;
                            loop {
                                r = p;
                                p = q;
                                fm = false;
                                if q < st.mem.hi_mem_min {
                                    if st.mem.b16s1(q) == DISC_NODE {
                                        for _ in 1..=st.mem.b16s0(q) {
                                            p = st.mem.b32s1(p);
                                        }
                                        if p == tx {
                                            break 'last_box_done;
                                        }
                                    } else if st.mem.b16s1(q) == MATH_NODE
                                        && st.mem.b16s0(q) == BEGIN_M_CODE
                                    {
                                        fm = true;
                                    }
                                }
                                q = st.mem.b32s1(p);
                                if q == tx {
                                    break;
                                }
                            }
                            let q2 = st.mem.b32s1(tx);
                            st.mem.set_b32s1(p, q2);
                            st.mem.set_b32s1(tx, TEX_NULL);
                            if q2 == TEX_NULL {
                                if fm {
                                    return Err(MainControlAbort::Confusion(c"tail1"));
                                } else {
                                    st.input.cur_list.tail.0 = p;
                                }
                            } else if fm {
                                st.input.cur_list.tail.0 = r;
                                st.mem.set_b32s1(r, TEX_NULL);
                                crate::packaging::flush_node_list(&mut st.mem, p)
                                    .map_err(MainControlAbort::Confusion)?;
                            }
                            st.scanner.cur_box.0 = tx;
                            st.mem.set_b32s1(tx + 4, 0);
                        }
                    }
                }
            }
        }

        VSPLIT_CODE => {
            crate::scan::scan_register_num_in(st)?;
            let n = st.scanner.cur_val;
            if !crate::scan::scan_keyword_in(st, c"to".as_ptr())? {
                crate::output::error_here_with_diagnostic_in(st, c"Missing `to' inserted".as_ptr());
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                help(
                    &mut st.errors,
                    &[
                        c"I'm working on `\\vsplit<box number> to <dimen>';",
                        c"will look for the <dimen> next.",
                    ],
                );
                let o = crate::errors::error_in(st);
                if !matches!(o, crate::errors::ErrorOutcome::Continue) {
                    return Err(MainControlAbort::Error(o));
                }
            }
            crate::scan::scan_dimen_in(st, false, false, false)?;
            st.scanner.cur_box.0 = crate::packaging::vsplit_in(st, n, st.scanner.cur_val)?;
        }

        _ => {
            let k = st.scanner.cur_chr - 4;
            let sp = st.save_stack.save_ptr;
            st.save_stack.set_b32s1(sp, box_context);
            if k == HMODE as i32 {
                if box_context < BOX_FLAG && st.input.cur_list.mode.abs() == VMODE {
                    crate::scan::scan_spec_in(st, GroupCode::AdjustedHbox, true)?;
                } else {
                    crate::scan::scan_spec_in(st, GroupCode::Hbox, true)?;
                }
            } else {
                if k == VMODE as i32 {
                    crate::scan::scan_spec_in(st, GroupCode::Vbox, true)?;
                } else {
                    crate::scan::scan_spec_in(st, GroupCode::Vtop, true)?;
                    // k = VMODE for the nest push below
                }
                normal_paragraph(st)?;
            }
            crate::mem::push_nest_in(st)?;
            // If k was adjusted to GroupCode::Vtop branch: k was neither HMODE nor VMODE,
            // so the else branch above ran and after scan_spec(GroupCode::Vtop), we want k=VMODE.
            let k_mode = if k != HMODE as i32 && k != VMODE as i32 {
                VMODE as i32
            } else {
                k
            };
            st.input.cur_list.mode = -(k_mode as i16);
            if k_mode == VMODE as i32 {
                st.input.cur_list.set_aux_s1(IGNORE_DEPTH);
                begin_every(st, LOCAL_EVERY_VBOX, EVERY_VBOX_TEXT)?;
            } else {
                st.input.cur_list.set_aux_s0(1000);
                begin_every(st, LOCAL_EVERY_HBOX, EVERY_HBOX_TEXT)?;
            }
            return Ok(()); // no box_end call for new-group cases
        }
    }

    box_end_in(st, box_context)
}
pub(crate) fn handle_right_brace_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    use crate::mem::{alloc_noad, delete_glue_ref, free_node, get_node, pop_nest_in, TEX_NULL};
    use crate::packaging::flush_node_list;
    use crate::scan::NodeRecycleAbort;

    match st.save_stack.cur_group {
        GroupCode::Simple => {
            crate::save::unsave_in(st)?;
        }

        GroupCode::BottomLevel => {
            crate::output::error_here_with_diagnostic_in(st, c"Too many }'s".as_ptr());
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            help(
                &mut st.errors,
                &[
                    c"You've closed more groups than you opened.",
                    c"Such booboos are generally harmless, so keep going.",
                ],
            );
            mc_propagate_error(crate::errors::error_in(st))?;
        }

        GroupCode::SemiSimple | GroupCode::MathShift | GroupCode::MathLeft => {
            mc_propagate_error(crate::errors::extra_right_brace_in(st))?;
        }

        GroupCode::Hbox => {
            package_in(st, 0)?;
        }

        GroupCode::AdjustedHbox => {
            st.scanner.adjust_tail.0 = ADJUST_HEAD;
            st.scanner.pre_adjust_tail.0 = PRE_ADJUST_HEAD;
            package_in(st, 0)?;
        }

        GroupCode::Vbox => {
            end_graf_in(st)?;
            package_in(st, 0)?;
        }

        GroupCode::Vtop => {
            end_graf_in(st)?;
            package_in(st, VTOP_CODE)?;
        }

        GroupCode::Insert => {
            end_graf_in(st)?;
            // GLUEPAR(split_top_skip)
            let q = st.eqtb.b32s1(GLUE_BASE + GLUE_PAR_SPLIT_TOP_SKIP);
            // GLUE_SPEC_ref_count(q)++
            st.mem.set_b32s1(q, st.mem.b32s1(q) + 1);
            let d = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_SPLIT_MAX_DEPTH);
            let f = st.eqtb.b32s1(INT_BASE + INT_PAR_FLOATING_PENALTY);
            crate::save::unsave_in(st)?;
            st.save_stack.save_ptr -= 2;
            let head = st.mem.link_ptr(st.input.cur_list.head);
            let p = crate::packaging::vpackage_emit_in(st, head, 0, ADDITIONAL, MAX_HALFWORD)?;
            pop_nest_in(&mut st.mem, &mut st.input);

            if st.save_stack.b32s1(st.save_stack.save_ptr) < 255 {
                let new_node =
                    get_node(&mut st.mem, INS_NODE_SIZE).ok_or(NodeRecycleAbort::Overflow)?;
                tail_append(st, new_node);
                let tail = st.input.cur_list.tail.0;
                st.mem.set_b16s1(tail, INS_NODE);
                let ins = match st.mem.classify(MemIdx::new(tail)) {
                    Ok(AnyNode::Ins(ins)) => ins,
                    _ => unreachable!("freshly stamped INS_NODE"),
                };
                let sp = st.save_stack.b32s1(st.save_stack.save_ptr) as u16;
                ins.set_subtype(&mut st.mem, sp);
                let hd = p.height(&st.mem).0 + p.depth(&st.mem).0;
                ins.set_height(&mut st.mem, Scaled(hd));
                let lp = p.list_ptr(&st.mem); // ins list ptr
                ins.set_ins_ptr(&mut st.mem, lp);
                ins.set_split_top_ptr(&mut st.mem, MemPtr::from_raw(q));
                ins.set_depth(&mut st.mem, Scaled(d));
                ins.set_float_cost(&mut st.mem, f);
            } else {
                let new_node =
                    get_node(&mut st.mem, SMALL_NODE_SIZE).ok_or(NodeRecycleAbort::Overflow)?;
                tail_append(st, new_node);
                let tail = st.input.cur_list.tail.0;
                st.mem.set_b16s1(tail, ADJUST_NODE);
                let adj = match st.mem.classify(MemIdx::new(tail)) {
                    Ok(AnyNode::Adjust(adj)) => adj,
                    _ => unreachable!("freshly stamped ADJUST_NODE"),
                };
                let sp = st.save_stack.b32s1(st.save_stack.save_ptr + 1) as u16;
                adj.set_subtype(&mut st.mem, sp);
                let lp = p.list_ptr(&st.mem);
                adj.set_adjust_ptr(&mut st.mem, lp);
                delete_glue_ref(&mut st.mem, q);
            }

            free_node(&mut st.mem, MemPtr::from_idx(p.idx()), BOX_NODE_SIZE);
            if st.input.nest_ptr == 0 {
                crate::pagebuilder::build_page_in(st)?;
            }
        }

        GroupCode::Output => {
            if st.input.cur_input.loc != TEX_NULL
                || (st.input.cur_input.index != OUTPUT_TEXT
                    && st.input.cur_input.index != BACKED_UP)
            {
                crate::output::error_here_with_diagnostic_in(
                    st,
                    c"Unbalanced output routine".as_ptr(),
                );
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                help(
                    &mut st.errors,
                    &[
                        c"Your sneaky output routine has problematic {'s and/or }'s.",
                        c"I can't handle that very well; good luck.",
                    ],
                );
                mc_propagate_error(crate::errors::error_in(st))?;
                loop {
                    crate::scan::get_token_in(st)?;
                    if st.input.cur_input.loc == TEX_NULL {
                        break;
                    }
                }
            }

            crate::input::end_token_list_in(st)?;
            end_graf_in(st)?;
            crate::save::unsave_in(st)?;
            st.shipout.output_active = false;
            st.shipout.insert_penalties = 0;

            if st.eqtb.b32s1(BOX_BASE + 255) != TEX_NULL {
                crate::output::error_here_with_diagnostic_in(
                    st,
                    c"Output routine didn't use all of ".as_ptr(),
                );
                crate::output::print_esc_cstr_in(st, c"box".as_ptr());
                crate::output::print_int_in(st, 255);
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                help(
                    &mut st.errors,
                    &[
                        c"Your \\output commands should empty \\box255,",
                        c"e.g., by saying `\\shipout\\box255'.",
                        c"Proceed; I'll discard its present contents.",
                    ],
                );
                mc_propagate_error(crate::errors::box_error_in(st, 255))?;
            }

            if st.input.cur_list.tail.0 != st.input.cur_list.head.0 {
                let head_link = st.mem.b32s1(st.input.cur_list.head);
                st.mem.set_b32s1(st.shipout.page_tail, head_link);
                st.shipout.page_tail.0 = st.input.cur_list.tail.0;
            }

            if st.mem.b32s1(PAGE_HEAD) != TEX_NULL {
                if st.mem.b32s1(CONTRIB_HEAD) == TEX_NULL {
                    st.input.nest[0].tail.0 = st.shipout.page_tail.0;
                }
                let contrib = st.mem.b32s1(CONTRIB_HEAD);
                st.mem.set_b32s1(st.shipout.page_tail, contrib);
                st.mem.set_b32s1(CONTRIB_HEAD, st.mem.b32s1(PAGE_HEAD));
                st.mem.set_b32s1(PAGE_HEAD, TEX_NULL);
                st.shipout.page_tail.0 = PAGE_HEAD;
            }

            let dp = st.scanner.disc_ptr[LAST_BOX_CODE as usize];
            flush_node_list(&mut st.mem, dp).map_err(MainControlAbort::Confusion)?;
            st.scanner.disc_ptr[LAST_BOX_CODE as usize] = MemPtr::NULL;
            pop_nest_in(&mut st.mem, &mut st.input);
            crate::pagebuilder::build_page_in(st)?;
        }

        GroupCode::Disc => {
            build_discretionary_in(st)?;
        }

        GroupCode::Align => {
            crate::input::back_input_in(st)?;
            st.scanner.cur_tok = CS_TOKEN_FLAG + FROZEN_CR;
            crate::output::error_here_with_diagnostic_in(st, c"Missing ".as_ptr());
            crate::output::print_esc_cstr_in(st, c"cr".as_ptr());
            crate::output::print_cstr_in(st, c" inserted".as_ptr());
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            help(
                &mut st.errors,
                &[c"I'm guessing that you meant to end an alignment here."],
            );
            mc_propagate_error(crate::input::ins_error_in(st)?)?;
        }

        GroupCode::NoAlign => {
            end_graf_in(st)?;
            crate::save::unsave_in(st)?;
            align_peek_in(st)?;
        }

        GroupCode::Vcenter => {
            end_graf_in(st)?;
            crate::save::unsave_in(st)?;
            st.save_stack.save_ptr -= 2;
            let head = st.mem.link_ptr(st.input.cur_list.head);
            let w = st.save_stack.b32s1(st.save_stack.save_ptr + 1);
            let m = st.save_stack.b32s1(st.save_stack.save_ptr) as i16;
            let p = crate::packaging::vpackage_emit_in(st, head, w, m, MAX_HALFWORD)?;
            pop_nest_in(&mut st.mem, &mut st.input);
            let noad = alloc_noad(&mut st.mem).ok_or(NodeRecycleAbort::Overflow)?;
            tail_append(st, noad.ptr());
            st.mem
                .set_b16s1(st.input.cur_list.tail, NodeType::VcenterNoad.as_raw());
            st.mem.set_b32s1(st.input.cur_list.tail + 1, SUB_BOX);
            st.mem.set_b32s0(st.input.cur_list.tail + 1, p.idx().0);
        }

        GroupCode::MathChoice => {
            crate::math::build_choices_in(st)?;
        }

        GroupCode::Math => {
            crate::save::unsave_in(st)?;
            st.save_stack.save_ptr -= 1;
            let save_loc = st.save_stack.b32s1(st.save_stack.save_ptr);
            st.mem.set_b32s1(save_loc, SUB_MLIST);
            let p = crate::math::fin_mlist_in(st, TEX_NULL)?;
            st.mem.set_b32s0(save_loc, p);

            if p != TEX_NULL && st.mem.b32s1(p) == TEX_NULL {
                if st.mem.b16s1(p) == NodeType::OrdNoad.as_raw()
                    && st.mem.b32s1(p + 3) == EMPTY
                    && st.mem.b32s1(p + 2) == EMPTY
                {
                    // collapse to inline field
                    st.mem.set_b32s0(save_loc, st.mem.b32s0(p + 1));
                    st.mem.set_b32s1(save_loc, st.mem.b32s1(p + 1));
                    free_node(&mut st.mem, MemPtr::from_raw(p), NOAD_SIZE);
                } else if st.mem.b16s1(p) == NodeType::AccentNoad.as_raw()
                    && save_loc == st.input.cur_list.tail.0 + 1
                    && st.mem.b16s1(st.input.cur_list.tail) == NodeType::OrdNoad.as_raw()
                {
                    // merge accent noad into ord
                    let mut q = st.input.cur_list.head;
                    while q.link(&st.mem) != st.input.cur_list.tail {
                        q = q.link(&st.mem);
                    }
                    st.mem.set_b32s1(q, p);
                    free_node(&mut st.mem, st.input.cur_list.tail, NOAD_SIZE);
                    st.input.cur_list.tail.0 = p;
                }
            }
        }
    }
    Ok(())
}

fn append_glue_node(
    mem: &mut crate::state::MemState,
    tail: i32,
    glue_spec: i32,
    subtype: u16,
    decrement_spec_ref: bool,
) -> Option<crate::node::NodePtr<crate::node::GlueNode>> {
    let p = crate::mem::alloc_glue(mem, glue_spec)?;
    mem.set_b32s1(tail, p.idx().0);
    if decrement_spec_ref {
        let spec = p.glue_spec(mem).expect("non-null glue spec");
        spec.set_ref_count(mem, spec.ref_count(mem) - 1);
        p.set_subtype(mem, subtype);
    }
    Some(p)
}

pub(crate) fn append_glue_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    let s = st.scanner.cur_chr as i16;
    match s {
        0 => st.scanner.cur_val = 4,
        1 => st.scanner.cur_val = 8,
        2 => st.scanner.cur_val = 12,
        3 => st.scanner.cur_val = 16,
        4 => crate::scan::scan_glue_in(st, GLUE_VAL)?, // SKIP_CODE
        5 => crate::scan::scan_glue_in(st, MU_VAL_SCAN)?, // MSKIP_CODE
        _ => {}
    }
    let subtype = if s > SKIP_CODE { MU_GLUE } else { 0 };
    let tail = st.input.cur_list.tail.0;
    let p = append_glue_node(
        &mut st.mem,
        tail,
        st.scanner.cur_val,
        subtype,
        s >= SKIP_CODE,
    )
    .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
    st.input.cur_list.tail = p.ptr();
    Ok(())
}
pub(crate) fn append_kern_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    let s = st.scanner.cur_chr as u16;
    crate::scan::scan_dimen_in(st, s == MU_GLUE, false, false)?;
    let p = crate::mem::alloc_kern(&mut st.mem, st.scanner.cur_val)
        .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
    p.set_subtype(&mut st.mem, s);
    tail_append(st, p.ptr());
    Ok(())
}
pub(crate) fn make_mark_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    let c = if st.scanner.cur_chr == 0 {
        0
    } else {
        crate::scan::scan_register_num_in(st)?;
        st.scanner.cur_val
    };
    crate::scan::scan_toks_in(st, false, true)?; // side effect: sets def_ref; return value discarded
    let p = crate::mem::get_node(&mut st.mem, SMALL_NODE_SIZE)
        .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
    st.mem.set_b16s1(p, MARK_NODE); // NODE_type(p)
    let mark = match st.mem.classify(MemIdx::new(p)) {
        Ok(AnyNode::Mark(mark)) => mark,
        _ => unreachable!("freshly stamped MARK_NODE"),
    };
    mark.set_mark_class(&mut st.mem, c);
    mark.set_subtype(&mut st.mem, 0);
    mark.set_mark_ptr(&mut st.mem, MemPtr::from_raw(st.scanner.def_ref.0));
    tail_append(st, p);
    Ok(())
}
pub(crate) fn new_interaction_in(st: &mut EngineState) {
    crate::output::print_ln_in(st);
    // The `\batchmode`/`\nonstopmode`/`\scrollmode`/`\errorstopmode` primitives
    // are registered with chr 0..=3, so this is the single classify boundary where
    // the mode enters the typed domain.
    st.errors.interaction = crate::state::InteractionMode::from_raw(st.scanner.cur_chr)
        .expect("interaction primitive chr is 0..=3");
    if st.errors.interaction == crate::state::InteractionMode::Batch {
        st.shipout.selector = SELECTOR_NO_PRINT;
    } else {
        st.shipout.selector = SELECTOR_TERM_ONLY;
    }
    if st.errors.log_opened {
        st.shipout.selector += 2;
    }
}
pub(crate) fn alter_aux_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    if st.scanner.cur_chr != st.input.cur_list.mode.abs() as i32 {
        return mc_propagate_error(crate::errors::report_illegal_case_in(st));
    }
    let c = st.scanner.cur_chr;
    crate::scan::scan_optional_equals_in(st)?;
    if c == VMODE as i32 {
        crate::scan::scan_dimen_in(st, false, false, false)?;
        st.input.cur_list.set_aux_s1(st.scanner.cur_val);
    } else {
        crate::scan::scan_int_in(st)?;
        if st.scanner.cur_val <= 0 || st.scanner.cur_val > 32767 {
            let errmsg =
                crate::output::error_here_with_diagnostic_in(st, c"Bad space factor".as_ptr());
            unsafe { diag_append_str(errmsg, &format!(" ({})", st.scanner.cur_val)) };
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            help(
                &mut st.errors,
                &[c"I allow only values in the range 1..32767 here."],
            );
            return mc_propagate_error(crate::errors::int_error_in(st, st.scanner.cur_val));
        }
        st.input.cur_list.set_aux_s0(st.scanner.cur_val);
    }
    Ok(())
}
pub(crate) fn alter_prev_graf_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    let np = st.input.nest_ptr;
    st.input.nest[np as usize] = st.input.cur_list;
    let mut p = np;
    while st.input.nest[p as usize].mode.abs() != VMODE {
        p -= 1;
    }
    crate::scan::scan_optional_equals_in(st)?;
    crate::scan::scan_int_in(st)?;
    if st.scanner.cur_val < 0 {
        let errmsg = crate::output::error_here_with_diagnostic_in(st, c"Bad ".as_ptr());
        unsafe { diag_append_str(errmsg, &format!(" ({})", st.scanner.cur_val)) };
        crate::output::print_esc_cstr_in(st, c"prevgraf".as_ptr());
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        help(&mut st.errors, &[c"I allow only nonnegative values here."]);
        return mc_propagate_error(crate::errors::int_error_in(st, st.scanner.cur_val));
    }
    st.input.nest[p as usize].prev_graf = st.scanner.cur_val;
    st.input.cur_list = st.input.nest[np as usize];
    Ok(())
}
pub(crate) fn alter_page_so_far_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    let c = st.scanner.cur_chr as usize;
    crate::scan::scan_optional_equals_in(st)?;
    crate::scan::scan_dimen_in(st, false, false, false)?;
    st.shipout.page_so_far[c] = st.scanner.cur_val;
    Ok(())
}
pub(crate) fn alter_integer_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    let c = st.scanner.cur_chr as u8;
    crate::scan::scan_optional_equals_in(st)?;
    crate::scan::scan_int_in(st)?;
    if c == 0 {
        st.shipout.dead_cycles = st.scanner.cur_val;
    } else if c == 2 {
        if crate::state::InteractionMode::from_raw(st.scanner.cur_val).is_none() {
            let errmsg =
                crate::output::error_here_with_diagnostic_in(st, c"Bad interaction mode".as_ptr());
            unsafe { diag_append_str(errmsg, &format!(" ({})", st.scanner.cur_val)) };
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            help(
                &mut st.errors,
                &[
                    c"Modes are 0=batch, 1=nonstop, 2=scroll, and",
                    c"3=errorstop. Proceed, and I'll ignore this case.",
                ],
            );
            return mc_propagate_error(crate::errors::int_error_in(st, st.scanner.cur_val));
        }
        st.scanner.cur_chr = st.scanner.cur_val;
        new_interaction_in(st);
    } else {
        st.shipout.insert_penalties = st.scanner.cur_val;
    }
    Ok(())
}
pub(crate) fn alter_box_dimen_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    let c = st.scanner.cur_chr;
    crate::scan::scan_register_num_in(st)?;
    let b = if st.scanner.cur_val < 256 {
        st.eqtb.b32s1(BOX_BASE + st.scanner.cur_val)
    } else {
        let __cv13 = st.scanner.cur_val;
        crate::save::find_sa_element(&mut st.mem, &mut st.scanner, 4, __cv13, false);
        if st.scanner.cur_ptr.0 == TEX_NULL {
            TEX_NULL
        } else {
            st.mem.b32s1(st.scanner.cur_ptr + 1)
        }
    };
    crate::scan::scan_optional_equals_in(st)?;
    crate::scan::scan_dimen_in(st, false, false, false)?;
    if b != TEX_NULL {
        st.mem.set_b32s1(b + c, st.scanner.cur_val);
    }
    Ok(())
}
pub(crate) fn trap_zero_glue(st: &mut EngineState) {
    let cv = st.scanner.cur_val;
    if st.mem.b32s1(cv + 1) == 0 && st.mem.b32s1(cv + 2) == 0 && st.mem.b32s1(cv + 3) == 0 {
        // GLUE_SPEC_ref_count(0)++ = mem[0].b32.s1++
        let r = st.mem.b32s1(0) + 1;
        st.mem.set_b32s1(0, r);
        crate::mem::delete_glue_ref(&mut st.mem, cv);
        st.scanner.cur_val = 0;
    }
}

pub(crate) fn fix_language(
    cl: &mut crate::mem_types::ListStateRecord,
    mem: &mut crate::state::MemState,
    eqtb: &crate::state::EqtbState,
) -> Result<(), crate::scan::NodeRecycleAbort> {
    let lang = eqtb.b32s1(INT_BASE + INT_PAR_LANGUAGE);
    let l = if lang <= 0 || lang > 255 { 0i32 } else { lang };
    if l != cl.aux_s1() {
        crate::mem::new_whatsit(cl, mem, LANGUAGE_NODE_SUBTYPE, SMALL_NODE_SIZE as i16)
            .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
        let tail = cl.tail.0;
        mem.set_b32s1(tail + 1, l);
        cl.set_aux_s1(l);
        mem.set_b16s1(
            tail + 1,
            crate::helpers::norm_min(eqtb.b32s1(INT_BASE + INT_PAR_LEFT_HYPHEN_MIN)) as u16,
        );
        mem.set_b16s0(
            tail + 1,
            crate::helpers::norm_min(eqtb.b32s1(INT_BASE + INT_PAR_RIGHT_HYPHEN_MIN)) as u16,
        );
    }
    Ok(())
}

pub(crate) fn off_save_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    if st.save_stack.cur_group == GroupCode::BottomLevel {
        crate::output::error_here_with_diagnostic_in(st, c"Extra ".as_ptr());
        crate::output::print_cmd_chr_in(st, st.scanner.cur_cmd as u16, st.scanner.cur_chr);
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        help(
            &mut st.errors,
            &[c"Things are pretty mixed up, but I think the worst is over."],
        );
        mc_propagate_error(crate::errors::error_in(st))?;
    } else {
        crate::input::back_input_in(st)?;
        let p = crate::scan::get_avail_scc(st)?;
        st.mem.set_b32s1(TEMP_HEAD, p.0);
        crate::output::error_here_with_diagnostic_in(st, c"Missing ".as_ptr());
        match st.save_stack.cur_group {
            GroupCode::SemiSimple => {
                st.mem.set_b32s0(p, CS_TOKEN_FLAG + FROZEN_END_GROUP);
                crate::output::print_esc_cstr_in(st, c"endgroup".as_ptr());
            }
            GroupCode::MathShift => {
                st.mem.set_b32s0(p, MATH_SHIFT_TOKEN + b'$' as i32);
                crate::output::print_char_in(st, b'$' as i32);
            }
            GroupCode::MathLeft => {
                p.set_info(&mut st.mem, CS_TOKEN_FLAG + FROZEN_RIGHT);
                let q = crate::scan::get_avail_scc(st)?;
                p.set_link(&mut st.mem, q);
                q.set_info(&mut st.mem, OTHER_TOKEN + b'.' as i32);
                crate::output::print_esc_cstr_in(st, c"right.".as_ptr());
            }
            _ => {
                st.mem.set_b32s0(p, RIGHT_BRACE_TOKEN + b'}' as i32);
                crate::output::print_char_in(st, b'}' as i32);
            }
        }
        crate::output::print_cstr_in(st, c" inserted".as_ptr());
        crate::input::begin_token_list_in(st, st.mem.b32s1(TEMP_HEAD), INSERTED)?;
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        help(
            &mut st.errors,
            &[
                c"I've inserted something that you may have forgotten.",
                c"(See the <inserted text> above.)",
                c"With luck, this will get me unwedged. But if you",
                c"really didn't forget anything, try typing `2' now; then",
                c"my insertion and my current dilemma will both disappear.",
            ],
        );
        mc_propagate_error(crate::errors::error_in(st))?;
    }
    Ok(())
}
pub(crate) fn append_discretionary_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    let d = crate::mem::alloc_disc(&mut st.mem).ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
    tail_append(st, d.ptr());

    if st.scanner.cur_chr == 1 {
        let f = st.eqtb.b32s1(CUR_FONT_LOC);
        let c = st.fonts.hyphen_char[f as usize];
        if (0..=BIGGEST_CHAR).contains(&c) {
            let ch = new_character_in(st, f, c as u16)?;
            let tail2 = st.input.cur_list.tail.0;
            st.mem.set_b32s0(tail2 + 1, ch);
        }
    } else {
        let sp = st.save_stack.save_ptr;
        st.save_stack.set_b32s1(sp, 0);
        st.save_stack.save_ptr = sp + 1;
        crate::save::new_save_level(&mut st.save_stack, GroupCode::Disc, st.mem.line)?;
        crate::scan::scan_left_brace_in(st)?;
        crate::mem::push_nest_in(st)?;
        st.input.cur_list.mode = -104; // -HMODE
        st.input.cur_list.set_aux_s0(1000);
    }
    Ok(())
}
pub(crate) fn shift_case_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    let b = st.scanner.cur_chr;
    crate::scan::scan_toks_in(st, false, false)?;
    let mut p = st.mem.b32s1(st.scanner.def_ref);
    while p != TEX_NULL {
        let t = st.mem.b32s0(p);
        if t < CS_TOKEN_FLAG + SINGLE_BASE {
            let c = t % MAX_CHAR_VAL;
            if st.eqtb.b32s1(b + c) != 0 {
                st.mem.set_b32s0(p, t - c + st.eqtb.b32s1(b + c));
            }
        }
        p = st.mem.b32s1(p);
    }
    let def_ref = st.scanner.def_ref.0;
    crate::input::begin_token_list_in(st, st.mem.b32s1(def_ref), BACKED_UP)?;
    crate::mem::free_avail(&mut st.mem, MemPtr::from_raw(def_ref));
    Ok(())
}
/// Validates and stitches the three discretionary sublists.
pub(crate) fn build_discretionary_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    crate::save::unsave_in(st)?;
    let q_head = st.input.cur_list.head.0;
    let mut q = q_head;
    let mut p = st.mem.b32s1(q);
    let mut n: i32 = 0;

    'walk: loop {
        if p == TEX_NULL {
            break;
        }
        if p < st.mem.hi_mem_min {
            // not a char node — check node type
            let ntype = st.mem.b16s1(p);
            if ntype > RULE_NODE {
                let ok = ntype == KERN_NODE   // 11
                    || ntype == LIGATURE_NODE  // 6
                    || ntype == WHATSIT_NODE && {
                        crate::node::WhatsitSubtype::from_raw(st.mem.b16s0(p)).is_some_and(|s| {
                            s.is_native_word() || s == crate::node::WhatsitSubtype::Glyph
                        })
                    };
                if !ok {
                    crate::output::error_here_with_diagnostic_in(
                        st,
                        c"Improper discretionary list".as_ptr(),
                    );
                    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                    help(
                        &mut st.errors,
                        &[c"Discretionary lists must contain only boxes and kerns."],
                    );
                    let o = crate::errors::error_in(st);
                    if !matches!(o, crate::errors::ErrorOutcome::Continue) {
                        return Err(MainControlAbort::Error(o));
                    }

                    crate::output::begin_diagnostic_in(st);
                    crate::output::diagnostic_begin_capture_warning_here_in(st);
                    crate::output::print_nl_cstr_in(
                        st,
                        c"The following discretionary sublist has been deleted:".as_ptr(),
                    );
                    crate::output::show_box_in(st, p);
                    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                    crate::output::end_diagnostic_in(st, true);

                    crate::packaging::flush_node_list(&mut st.mem, p)
                        .map_err(MainControlAbort::Confusion)?;
                    st.mem.set_b32s1(q, TEX_NULL);
                    break 'walk;
                }
            }
        }
        q = p;
        p = st.mem.b32s1(q);
        n += 1;
    }

    p = st.mem.b32s1(q_head);
    crate::mem::pop_nest_in(&mut st.mem, &mut st.input);

    let slot = st.save_stack.b32s1(st.save_stack.save_ptr - 1);
    match slot {
        0 => {
            st.mem.set_b32s0(st.input.cur_list.tail + 1, p);
        }
        1 => {
            st.mem.set_b32s1(st.input.cur_list.tail + 1, p);
        }
        2 => {
            if n > 0 && st.input.cur_list.mode.abs() == MMODE {
                crate::output::error_here_with_diagnostic_in(st, c"Illegal math ".as_ptr());
                crate::output::print_esc_cstr_in(st, c"discretionary".as_ptr());
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                help(
                    &mut st.errors,
                    &[
                        c"Sorry: The third part of a discretionary break must be",
                        c"empty, in math formulas. I had to delete your third part.",
                    ],
                );
                crate::packaging::flush_node_list(&mut st.mem, p)
                    .map_err(MainControlAbort::Confusion)?;
                n = 0;
                let o = crate::errors::error_in(st);
                if !matches!(o, crate::errors::ErrorOutcome::Continue) {
                    return Err(MainControlAbort::Error(o));
                }
            } else {
                st.mem.set_b32s1(st.input.cur_list.tail, p);
            }
            if n <= u16::MAX as i32 {
                st.mem.set_b16s0(st.input.cur_list.tail, n as u16);
            } else {
                crate::output::error_here_with_diagnostic_in(
                    st,
                    c"Discretionary list is too long".as_ptr(),
                );
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                help(
                    &mut st.errors,
                    &[
                        c"Wow---I never thought anybody would tweak me here.",
                        c"You can't seriously need such a huge discretionary list?",
                    ],
                );
                let o = crate::errors::error_in(st);
                if !matches!(o, crate::errors::ErrorOutcome::Continue) {
                    return Err(MainControlAbort::Error(o));
                }
            }
            if n > 0 {
                st.input.cur_list.tail.0 = q;
            }
            st.save_stack.save_ptr -= 1;
            return Ok(());
        }
        _ => {}
    }
    st.save_stack
        .set_b32s1(st.save_stack.save_ptr - 1, slot + 1);
    crate::save::new_save_level(&mut st.save_stack, GroupCode::Disc, st.mem.line)?;
    crate::scan::scan_left_brace_in(st)?;
    crate::mem::push_nest_in(st)?;
    st.input.cur_list.mode = -104; // -HMODE
    st.input.cur_list.set_aux_s0(1000);
    Ok(())
}
pub(crate) fn append_italic_correction_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    // get_native_italic_correction(p) = real_get_native_italic_correction(&mem[p])
    let native_kern =
        |st: &mut EngineState, tail: i32, glyph: bool| -> Result<(), MainControlAbort> {
            let ic = unsafe {
                if glyph {
                    crate::ext::real_get_native_glyph_italic_correction(&st.fonts, &st.mem, tail)
                } else {
                    crate::ext::real_get_native_italic_correction(&st.fonts, &st.mem, tail)
                }
            };
            let Some(kern) = crate::mem::alloc_kern(&mut st.mem, ic) else {
                return Err(crate::scan::NodeRecycleAbort::Overflow.into());
            };
            let kern_idx = kern.idx().0;
            st.mem.set_b32s1(tail, kern_idx);
            st.input.cur_list.tail.0 = kern_idx;
            kern.set_subtype(&mut st.mem, EXPLICIT);
            Ok(())
        };

    let tail = st.input.cur_list.tail.0;
    if tail == st.input.cur_list.head.0 {
        return Ok(());
    }

    let p = if tail >= st.mem.hi_mem_min {
        // is_char_node
        tail
    } else if st.mem.b16s1(tail) == LIGATURE_NODE {
        tail + 1
    } else if st.mem.b16s1(tail) == WHATSIT_NODE {
        let sub = crate::node::WhatsitSubtype::from_raw(st.mem.b16s0(tail));
        if sub.is_some_and(|s| s.is_native_word()) {
            native_kern(st, tail, false)?;
        } else if sub == Some(crate::node::WhatsitSubtype::Glyph) {
            native_kern(st, tail, true)?;
        }
        return Ok(());
    } else {
        return Ok(());
    };

    let f = st.mem.b16s1(p) as i32; // CHAR_NODE_font(p)
    let c = crate::fonts::effective_char(&mut st.fonts, true, f, st.mem.b16s0(p));
    let info = st.fonts.font_info_b16(st.fonts.char_base_entry(f) + c);
    let italic = st
        .fonts
        .font_info_b32s1(st.fonts.italic_base_entry(f) + (info.s1 / 4) as i32);
    let Some(kern) = crate::mem::alloc_kern(&mut st.mem, italic) else {
        return Err(crate::scan::NodeRecycleAbort::Overflow.into());
    };
    let kern_idx = kern.idx().0;
    let tail2 = st.input.cur_list.tail.0;
    st.mem.set_b32s1(tail2, kern_idx);
    st.input.cur_list.tail.0 = kern_idx;
    kern.set_subtype(&mut st.mem, EXPLICIT);
    Ok(())
}
pub(crate) fn make_accent_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    fn is_native(fonts: &crate::state::FontState, fnt: i32) -> bool {
        fonts.font_area_entry(fnt) == AAT_FONT_FLAG || fonts.font_area_entry(fnt) == OTGR_FONT_FLAG
    }

    crate::scan::scan_char_num_in(st)?;
    let mut f = st.eqtb.b32s1(CUR_FONT_LOC);
    let p = new_character_in(st, f, st.scanner.cur_val as u16)?;

    if p == TEX_NULL {
        return Ok(());
    }

    let x = st
        .fonts
        .font_info_b32s1(X_HEIGHT_CODE + st.fonts.param_base(f));
    let s = st
        .fonts
        .font_info_b32s1(SLANT_CODE + st.fonts.param_base(f)) as f64
        / 65536.0;
    let mut lsb: i32 = 0;
    let mut rsb: i32 = 0;
    let a = if is_native(&st.fonts, f) {
        let av = st.mem.b32s1(p + 1);
        if av == 0 {
            unsafe {
                crate::ext::get_native_char_sidebearings(
                    &st.fonts,
                    f,
                    st.scanner.cur_val,
                    &mut lsb,
                    &mut rsb,
                )?;
            }
        }
        av
    } else {
        let ch = st.mem.b16s0(p);
        let c = crate::fonts::effective_char(&mut st.fonts, true, f, ch);
        let info = st.fonts.font_info_b16(st.fonts.char_base_entry(f) + c);
        st.fonts
            .font_info_b32s1(st.fonts.width_base_entry(f) + info.s3 as i32)
    };

    do_assignments_in(st)?;
    let mut q = TEX_NULL;
    f = st.eqtb.b32s1(CUR_FONT_LOC);
    if st.scanner.cur_cmd == LETTER
        || st.scanner.cur_cmd == OTHER_CHAR
        || st.scanner.cur_cmd == CHAR_GIVEN
    {
        q = new_character_in(st, f, st.scanner.cur_chr as u16)?;
        st.scanner.cur_val = st.scanner.cur_chr;
    } else if st.scanner.cur_cmd == CHAR_NUM {
        crate::scan::scan_char_num_in(st)?;
        q = new_character_in(st, f, st.scanner.cur_val as u16)?;
    } else {
        crate::input::back_input_in(st)?;
    }

    if q != TEX_NULL {
        let t = st
            .fonts
            .font_info_b32s1(SLANT_CODE + st.fonts.param_base(f)) as f64
            / 65536.0;
        let (w, h) = if is_native(&st.fonts, f) {
            let wv = st.mem.b32s1(q + 1);
            let mut hv: i32 = 0;
            let mut dv: i32 = 0;
            unsafe {
                crate::ext::get_native_char_height_depth(
                    &st.fonts,
                    f,
                    st.scanner.cur_val,
                    &mut hv,
                    &mut dv,
                )?;
            }
            (wv, hv)
        } else {
            let ch = st.mem.b16s0(q);
            let c = crate::fonts::effective_char(&mut st.fonts, true, f, ch);
            let info = st.fonts.font_info_b16(st.fonts.char_base_entry(f) + c);
            let wv = st
                .fonts
                .font_info_b32s1(st.fonts.width_base_entry(f) + info.s3 as i32);
            let hv = st
                .fonts
                .font_info_b32s1(st.fonts.height_base_entry(f) + (info.s2 / 16) as i32);
            (wv, hv)
        };
        let mut pa = p;
        if h != x {
            pa = crate::packaging::hpack_emit_in(st, MemPtr::from_raw(p), 0, ADDITIONAL)?
                .idx()
                .0;
            st.mem.set_b32s1(pa + 4, x - h);
        }
        let delta = if is_native(&st.fonts, f) && a == 0 {
            crate::scaled_math::tex_round(
                (w - lsb + rsb) as f64 / 2.0 + h as f64 * t - x as f64 * s,
            )
        } else {
            crate::scaled_math::tex_round((w - a) as f64 / 2.0 + h as f64 * t - x as f64 * s)
        };
        let Some(r) = crate::mem::alloc_kern(&mut st.mem, delta) else {
            return Err(crate::scan::NodeRecycleAbort::Overflow.into());
        };
        r.set_subtype(&mut st.mem, ACC_KERN);
        st.mem.set_b32s1(st.input.cur_list.tail, r.idx().0);
        r.set_link(&mut st.mem, MemPtr::from_raw(pa));
        let Some(new_tail) = crate::mem::alloc_kern(&mut st.mem, -a - delta) else {
            return Err(crate::scan::NodeRecycleAbort::Overflow.into());
        };
        st.input.cur_list.tail.0 = new_tail.idx().0;
        new_tail.set_subtype(&mut st.mem, ACC_KERN);
        st.mem.set_b32s1(pa, new_tail.idx().0);
        // p = q (in C); we use q directly below
    }

    // Append p (q if base was found, else accent) to hlist.
    let p_final = if q != TEX_NULL { q } else { p };
    tail_append(st, MemPtr::from_raw(p_final));
    st.input.cur_list.set_aux_s0(1000);
    Ok(())
}
pub(crate) unsafe fn new_font_in(st: &mut EngineState, a: i16) -> Result<(), MainControlAbort> {
    if st.strings.job_name == 0 {
        crate::io::open_log_file_in(st)?;
    }

    crate::scan::get_r_token_in(st)?;
    let u = st.scanner.cur_cs;
    let t = if u >= HASH_BASE {
        st.eqtb.hash_s1(u)
    } else if u >= SINGLE_BASE {
        if u == NULL_CS {
            crate::texmfmp::maketexstring_in(&mut st.strings, c"FONT".as_ptr())?
        } else {
            u - SINGLE_BASE
        }
    } else {
        let old_setting = st.shipout.selector;
        st.shipout.selector = SELECTOR_NEW_STRING;
        crate::output::print_cstr_in(st, c"FONT".as_ptr());
        crate::output::print_in(st, u - 1);
        st.shipout.selector = old_setting;
        if st.strings.pool_ptr + 1 > st.strings.pool_size {
            return Err(MainControlAbort::Input(
                crate::input::InputAbort::Overflow {
                    what: c"pool size".as_ptr(),
                    n: st.strings.pool_size - st.strings.init_pool_ptr,
                },
            ));
        }
        match crate::string_pool::make_string(&mut st.strings) {
            Ok(s) => s,
            Err(crate::scan::NodeRecycleAbort::Overflow) => {
                return Err(MainControlAbort::Input(
                    crate::input::InputAbort::Overflow {
                        what: c"number of strings".as_ptr(),
                        n: st.strings.max_strings - st.strings.init_str_ptr,
                    },
                ));
            }
            Err(e) => return Err(MainControlAbort::NodeRecycle(e)),
        }
    };

    if a >= 4 {
        crate::save::geq_define(
            &mut st.eqtb,
            &mut st.mem,
            &mut st.scanner,
            u,
            SET_FONT as u16,
            FONT_BASE,
        )?;
    } else {
        crate::save::eq_define(
            &mut st.eqtb,
            &mut st.save_stack,
            &mut st.mem,
            &mut st.scanner,
            u,
            SET_FONT as u16,
            FONT_BASE,
        )?;
    }
    crate::scan::scan_optional_equals_in(st)?;
    crate::io::scan_file_name_in(st)?;
    st.scanner.name_in_progress = true;

    let s = if crate::scan::scan_keyword_in(st, c"at".as_ptr())? {
        crate::scan::scan_dimen_in(st, false, false, false)?;
        let sv = st.scanner.cur_val;
        if sv <= 0 || sv >= 0x800_0000 {
            crate::output::error_here_with_diagnostic_in(st, c"Improper `at' size (".as_ptr());
            crate::output::print_scaled_in(st, sv);
            crate::output::print_cstr_in(st, c"pt), replaced by 10pt".as_ptr());
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            help(
                &mut st.errors,
                &[
                    c"I can only handle fonts at positive sizes that are",
                    c"less than 2048pt, so I've changed what you said to 10pt.",
                ],
            );
            mc_propagate_error(crate::errors::error_in(st))?;
            10 * 65536_i32
        } else {
            sv
        }
    } else if crate::scan::scan_keyword_in(st, c"scaled".as_ptr())? {
        crate::scan::scan_int_in(st)?;
        let sv = st.scanner.cur_val;
        if sv <= 0 || sv > 32768 {
            let errmsg = crate::output::error_here_with_diagnostic_in(
                st,
                c"Illegal magnification has been changed to 1000".as_ptr(),
            );
            diag_append_str(errmsg, &format!(" ({})", sv));
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            help(
                &mut st.errors,
                &[c"The magnification ratio must be between 1 and 32768."],
            );
            mc_propagate_error(crate::errors::int_error_in(st, sv))?;
            -1000_i32
        } else {
            -sv
        }
    } else {
        -1000_i32
    };

    st.scanner.name_in_progress = false;

    let f = 'search: {
        let mut fi = FONT_BASE + 1;
        while fi <= st.fonts.font_ptr {
            if crate::string_pool::str_eq_str(
                &st.strings,
                st.fonts.font_name[fi as usize],
                st.scanner.cur_name,
            ) && ((crate::string_pool::length(&st.strings, st.scanner.cur_area) == 0
                && (st.fonts.font_area[fi as usize] == AAT_FONT_FLAG
                    || st.fonts.font_area[fi as usize] == OTGR_FONT_FLAG))
                || crate::string_pool::str_eq_str(
                    &st.strings,
                    st.fonts.font_area[fi as usize],
                    st.scanner.cur_area,
                ))
            {
                if s > 0 {
                    if s == st.fonts.font_size[fi as usize] {
                        break 'search fi;
                    }
                } else if st.fonts.font_size[fi as usize]
                    == crate::scaled_math::xn_over_d_in(
                        &mut st.errors,
                        st.fonts.font_dsize[fi as usize],
                        -s,
                        1000,
                    )
                {
                    break 'search fi;
                }
            }
            crate::string_pool::append_str(&mut st.strings, st.scanner.cur_area).map_err(|e| {
                match e {
                    crate::scan::NodeRecycleAbort::Overflow => {
                        MainControlAbort::Input(crate::input::InputAbort::Overflow {
                            what: c"pool size".as_ptr(),
                            n: st.strings.pool_size - st.strings.init_pool_ptr,
                        })
                    }
                    e => MainControlAbort::NodeRecycle(e),
                }
            })?;
            crate::string_pool::append_str(&mut st.strings, st.scanner.cur_name).map_err(|e| {
                match e {
                    crate::scan::NodeRecycleAbort::Overflow => {
                        MainControlAbort::Input(crate::input::InputAbort::Overflow {
                            what: c"pool size".as_ptr(),
                            n: st.strings.pool_size - st.strings.init_pool_ptr,
                        })
                    }
                    e => MainControlAbort::NodeRecycle(e),
                }
            })?;
            crate::string_pool::append_str(&mut st.strings, st.scanner.cur_ext).map_err(
                |e| match e {
                    crate::scan::NodeRecycleAbort::Overflow => {
                        MainControlAbort::Input(crate::input::InputAbort::Overflow {
                            what: c"pool size".as_ptr(),
                            n: st.strings.pool_size - st.strings.init_pool_ptr,
                        })
                    }
                    e => MainControlAbort::NodeRecycle(e),
                },
            )?;
            let ms = match crate::string_pool::make_string(&mut st.strings) {
                Ok(ms) => ms,
                Err(crate::scan::NodeRecycleAbort::Overflow) => {
                    return Err(MainControlAbort::Input(
                        crate::input::InputAbort::Overflow {
                            what: c"number of strings".as_ptr(),
                            n: st.strings.max_strings - st.strings.init_str_ptr,
                        },
                    ));
                }
                Err(e) => return Err(MainControlAbort::NodeRecycle(e)),
            };
            if crate::string_pool::str_eq_str(&st.strings, st.fonts.font_name[fi as usize], ms) {
                st.strings.str_ptr -= 1;
                st.strings.pool_ptr =
                    st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
                if st.fonts.font_area[fi as usize] == AAT_FONT_FLAG
                    || st.fonts.font_area[fi as usize] == OTGR_FONT_FLAG
                {
                    if s > 0 {
                        if s == st.fonts.font_size[fi as usize] {
                            break 'search fi;
                        }
                    } else if st.fonts.font_size[fi as usize]
                        == crate::scaled_math::xn_over_d_in(
                            &mut st.errors,
                            st.fonts.font_dsize[fi as usize],
                            -s,
                            1000,
                        )
                    {
                        break 'search fi;
                    }
                }
            } else {
                st.strings.str_ptr -= 1;
                st.strings.pool_ptr =
                    st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
            }
            fi += 1;
        }
        read_font_info_in(st, u, st.scanner.cur_name, st.scanner.cur_area, s)?
    };

    // common_ending:
    if a >= 4 {
        crate::save::geq_define(
            &mut st.eqtb,
            &mut st.mem,
            &mut st.scanner,
            u,
            SET_FONT as u16,
            f,
        )?;
    } else {
        crate::save::eq_define(
            &mut st.eqtb,
            &mut st.save_stack,
            &mut st.mem,
            &mut st.scanner,
            u,
            SET_FONT as u16,
            f,
        )?;
    }
    let word = st.eqtb.word(u);
    st.eqtb.set_word(FONT_ID_BASE + f, word);
    st.eqtb.set_hash_s1(FONT_ID_BASE + f, t);
    Ok(())
}
pub(crate) fn do_register_command_in(st: &mut EngineState, a: i16) -> Result<(), MainControlAbort> {
    let q = st.scanner.cur_cmd;
    let mut w: i32 = 0;
    let mut s: i32 = TEX_NULL;

    // Determine (l, p, e): register address, value type, is_extended_SA
    let (l, p, e) = 'determine: {
        if q != REGISTER {
            crate::scan::get_x_token_in(st)?;
            if st.scanner.cur_cmd >= ASSIGN_INT && st.scanner.cur_cmd <= ASSIGN_MU_GLUE {
                break 'determine (
                    st.scanner.cur_chr,
                    (st.scanner.cur_cmd - ASSIGN_INT) as i16,
                    false,
                );
            }
            if st.scanner.cur_cmd != REGISTER {
                crate::output::error_here_with_diagnostic_in(st, c"You can't use `".as_ptr());
                crate::output::print_cmd_chr_in(st, st.scanner.cur_cmd as u16, st.scanner.cur_chr);
                crate::output::print_cstr_in(st, c"' after ".as_ptr());
                crate::output::print_cmd_chr_in(st, q as u16, 0);
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                help(
                    &mut st.errors,
                    &[c"I'm forgetting what you said and not changing anything."],
                );
                return mc_propagate_error(crate::errors::error_in(st));
            }
        }
        if st.scanner.cur_chr < 0 || st.scanner.cur_chr > 19 {
            let l = st.scanner.cur_chr;
            let p = (st.mem.b16s1(l) / 64) as i16;
            (l, p, true)
        } else {
            let p = st.scanner.cur_chr as i16;
            crate::scan::scan_register_num_in(st)?;
            if st.scanner.cur_val > 255 {
                let __cv12 = st.scanner.cur_val;
                crate::save::find_sa_element(&mut st.mem, &mut st.scanner, p as u16, __cv12, true);
                (st.scanner.cur_ptr.0, p, true)
            } else {
                let l = match p {
                    INT_VAL => st.scanner.cur_val + COUNT_BASE,
                    DIMEN_VAL => st.scanner.cur_val + SCALED_BASE,
                    GLUE_VAL => st.scanner.cur_val + SKIP_BASE,
                    MU_VAL => st.scanner.cur_val + MU_SKIP_BASE,
                    _ => unreachable!(),
                };
                (l, p, false)
            }
        }
    };

    // found: read current value into w (int/dimen) or s (glue/mu)
    if p < GLUE_VAL {
        w = if e {
            st.mem.b32s1(l + 2)
        } else {
            unsafe { st.eqtb.data[l as usize].b32.s1 }
        };
    } else if e {
        s = st.mem.b32s1(l + 1);
    } else {
        s = unsafe { st.eqtb.data[l as usize].b32.s1 };
    }

    if q == REGISTER {
        crate::scan::scan_optional_equals_in(st)?;
    } else {
        crate::scan::scan_keyword_in(st, c"by".as_ptr())?;
    }

    st.errors.arith_error = false;

    if q < MULTIPLY {
        if p < GLUE_VAL {
            if p == INT_VAL {
                crate::scan::scan_int_in(st)?;
            } else {
                crate::scan::scan_dimen_in(st, false, false, false)?;
            }
            if q == ADVANCE {
                st.scanner.cur_val = st.scanner.cur_val + w;
            }
        } else {
            crate::scan::scan_glue_in(st, p)?;
            if q == ADVANCE {
                let qs_p = crate::mem::alloc_spec(&mut st.mem, st.scanner.cur_val)
                    .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
                let r = s;
                crate::mem::delete_glue_ref(&mut st.mem, st.scanner.cur_val);
                let r_p = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(r))
                    .expect("non-null glue spec");
                let w = qs_p.width(&st.mem).0 + r_p.width(&st.mem).0;
                qs_p.set_width(&mut st.mem, Scaled(w));
                if qs_p.stretch(&st.mem).0 == 0 {
                    qs_p.set_stretch_order(&mut st.mem, NORMAL);
                }
                if qs_p.stretch_order(&st.mem) == r_p.stretch_order(&st.mem) {
                    let v = qs_p.stretch(&st.mem).0 + r_p.stretch(&st.mem).0;
                    qs_p.set_stretch(&mut st.mem, Scaled(v));
                } else if qs_p.stretch_order(&st.mem) < r_p.stretch_order(&st.mem)
                    && r_p.stretch(&st.mem).0 != 0
                {
                    let (rs, ro) = (r_p.stretch(&st.mem), r_p.stretch_order(&st.mem));
                    qs_p.set_stretch(&mut st.mem, rs);
                    qs_p.set_stretch_order(&mut st.mem, ro);
                }
                if qs_p.shrink(&st.mem).0 == 0 {
                    qs_p.set_shrink_order(&mut st.mem, NORMAL);
                }
                if qs_p.shrink_order(&st.mem) == r_p.shrink_order(&st.mem) {
                    let v = qs_p.shrink(&st.mem).0 + r_p.shrink(&st.mem).0;
                    qs_p.set_shrink(&mut st.mem, Scaled(v));
                } else if qs_p.shrink_order(&st.mem) < r_p.shrink_order(&st.mem)
                    && r_p.shrink(&st.mem).0 != 0
                {
                    let (rs, ro) = (r_p.shrink(&st.mem), r_p.shrink_order(&st.mem));
                    qs_p.set_shrink(&mut st.mem, rs);
                    qs_p.set_shrink_order(&mut st.mem, ro);
                }
                st.scanner.cur_val = qs_p.ptr().0;
            }
        }
    } else {
        crate::scan::scan_int_in(st)?;
        if p < GLUE_VAL {
            if q == MULTIPLY {
                if p == INT_VAL {
                    st.scanner.cur_val = crate::scaled_math::mult_and_add_in(
                        &mut st.errors,
                        w,
                        st.scanner.cur_val,
                        0,
                        TEX_INFINITY,
                    );
                } else {
                    st.scanner.cur_val = crate::scaled_math::mult_and_add_in(
                        &mut st.errors,
                        w,
                        st.scanner.cur_val,
                        0,
                        MAX_HALFWORD,
                    );
                }
            } else {
                st.scanner.cur_val =
                    crate::scaled_math::x_over_n_in(&mut st.errors, w, st.scanner.cur_val);
            }
        } else {
            let r_p = crate::mem::alloc_spec(&mut st.mem, s)
                .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
            let s_p = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(s))
                .expect("non-null glue spec");
            let (sw, sst, ssh) = (
                s_p.width(&st.mem),
                s_p.stretch(&st.mem),
                s_p.shrink(&st.mem),
            );
            if q == MULTIPLY {
                let v = crate::scaled_math::mult_and_add_in(
                    &mut st.errors,
                    sw.0,
                    st.scanner.cur_val,
                    0,
                    MAX_HALFWORD,
                );
                r_p.set_width(&mut st.mem, Scaled(v));
                let v = crate::scaled_math::mult_and_add_in(
                    &mut st.errors,
                    sst.0,
                    st.scanner.cur_val,
                    0,
                    MAX_HALFWORD,
                );
                r_p.set_stretch(&mut st.mem, Scaled(v));
                let v = crate::scaled_math::mult_and_add_in(
                    &mut st.errors,
                    ssh.0,
                    st.scanner.cur_val,
                    0,
                    MAX_HALFWORD,
                );
                r_p.set_shrink(&mut st.mem, Scaled(v));
            } else {
                let v = crate::scaled_math::x_over_n_in(&mut st.errors, sw.0, st.scanner.cur_val);
                r_p.set_width(&mut st.mem, Scaled(v));
                let v = crate::scaled_math::x_over_n_in(&mut st.errors, sst.0, st.scanner.cur_val);
                r_p.set_stretch(&mut st.mem, Scaled(v));
                let v = crate::scaled_math::x_over_n_in(&mut st.errors, ssh.0, st.scanner.cur_val);
                r_p.set_shrink(&mut st.mem, Scaled(v));
            }
            st.scanner.cur_val = r_p.ptr().0;
        }
    }

    if st.errors.arith_error {
        crate::output::error_here_with_diagnostic_in(st, c"Arithmetic overflow".as_ptr());
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        help(
            &mut st.errors,
            &[
                c"I can't carry out that multiplication or division,",
                c"since the result is out of range.",
            ],
        );
        if p >= GLUE_VAL {
            crate::mem::delete_glue_ref(&mut st.mem, st.scanner.cur_val);
        }
        return mc_propagate_error(crate::errors::error_in(st));
    }

    let global = a >= 4;
    let value = st.scanner.cur_val;
    if p < GLUE_VAL {
        if e {
            sa_word_define(st, global, l, value)?;
        } else {
            word_define(st, global, l, value)?;
        }
    } else {
        trap_zero_glue(st);
        let value = st.scanner.cur_val;
        if e {
            sa_define(st, global, l, value)?;
        } else {
            define(st, global, l, GLUE_REF, value)?;
        }
    }
    Ok(())
}

pub(crate) fn do_assignments_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    loop {
        loop {
            crate::scan::get_x_token_in(st)?;
            if st.scanner.cur_cmd != SPACER && st.scanner.cur_cmd != RELAX {
                break;
            }
        }
        if st.scanner.cur_cmd <= MAX_NON_PREFIXED_COMMAND {
            return Ok(());
        }
        st.scanner.set_box_allowed = false;
        prefixed_command_in(st)?;
        st.scanner.set_box_allowed = true;
    }
}

pub(crate) fn main_control_append_normal_space_in(
    st: &mut EngineState,
) -> Result<(), MainControlAbort> {
    if inter_char_boundary_after_space_action(st, CHAR_NUM).execute_in(st)? {
        return Ok(());
    }

    let cur_font = st.eqtb.b32s1(CUR_FONT_LOC);
    if st.eqtb.b32s1(GLUE_BASE + GLUE_PAR_SPACE_SKIP as i32) == 0 {
        st.mainloop.main_p.0 = st.fonts.font_glue(cur_font);
        if st.mainloop.main_p.0 == TEX_NULL {
            st.mainloop.main_p = crate::mem::alloc_spec(&mut st.mem, 0)
                .ok_or(crate::scan::NodeRecycleAbort::Overflow)?
                .ptr();
            st.mainloop.main_k = st.fonts.param_base(cur_font) + 2;
            st.mem.set_b32s1(
                st.mainloop.main_p.0 + 1,
                st.fonts.font_info_b32s1(st.mainloop.main_k),
            );
            st.mem.set_b32s1(
                st.mainloop.main_p.0 + 2,
                st.fonts.font_info_b32s1(st.mainloop.main_k + 1),
            );
            st.mem.set_b32s1(
                st.mainloop.main_p.0 + 3,
                st.fonts.font_info_b32s1(st.mainloop.main_k + 2),
            );
            st.fonts.set_font_glue(cur_font, st.mainloop.main_p.0);
        }
        st.mem.temp_ptr = crate::mem::alloc_glue(&mut st.mem, st.mainloop.main_p.0)
            .ok_or(crate::scan::NodeRecycleAbort::Overflow)?
            .ptr();
    } else {
        st.mem.temp_ptr = crate::mem::alloc_param_glue(&mut st.mem, &st.eqtb, GLUE_PAR_SPACE_SKIP)
            .ok_or(crate::scan::NodeRecycleAbort::Overflow)?
            .ptr();
    }

    let tp = st.mem.temp_ptr;
    tail_append(st, tp);
    Ok(())
}

pub(crate) fn main_control_mode_independent_in(
    st: &mut EngineState,
) -> Result<(), MainControlAbort> {
    match st.scanner.cur_cmd {
        TOKS_REGISTER | ASSIGN_TOKS | ASSIGN_INT | ASSIGN_DIMEN | ASSIGN_GLUE | ASSIGN_MU_GLUE
        | ASSIGN_FONT_DIMEN | ASSIGN_FONT_INT | SET_AUX | SET_PREV_GRAF | SET_PAGE_DIMEN
        | SET_PAGE_INT | SET_BOX_DIMEN | SET_SHAPE | DEF_CODE | XETEX_DEF_CODE | DEF_FAMILY
        | SET_FONT | DEF_FONT | REGISTER | ADVANCE | MULTIPLY | DIVIDE | PREFIX | LET
        | SHORTHAND_DEF | READ_TO_CS | DEF | SET_BOX | HYPH_DATA | SET_INTERACTION => {
            prefixed_command_in(st)?
        }
        AFTER_ASSIGNMENT => {
            crate::scan::get_token_in(st)?;
            st.scanner.after_token = st.scanner.cur_tok;
        }
        AFTER_GROUP => {
            crate::scan::get_token_in(st)?;
            crate::save::save_for_after(&mut st.save_stack, st.scanner.cur_tok)?;
        }
        IN_STREAM => unsafe { crate::io::open_or_close_in(st)? },
        MESSAGE => crate::output::issue_message_in(st)?,
        CASE_SHIFT => shift_case_in(st)?,
        XRAY => crate::output::show_whatever_in(st)?,
        EXTENSION => do_extension_in(st)?,
        _ => {}
    }
    Ok(())
}

pub(crate) fn main_control_mode_error_cases_in(
    st: &mut EngineState,
) -> Result<(), MainControlAbort> {
    match (st.input.cur_list.mode.abs(), st.scanner.cur_cmd) {
        (VMODE, VMOVE)
        | (HMODE, HMOVE)
        | (MMODE, HMOVE)
        | (VMODE, LAST_ITEM)
        | (HMODE, LAST_ITEM)
        | (MMODE, LAST_ITEM)
        | (VMODE, VADJUST)
        | (VMODE, ITAL_CORR)
        | (VMODE, EQ_NO)
        | (HMODE, EQ_NO)
        | (VMODE, MAC_PARAM)
        | (HMODE, MAC_PARAM)
        | (MMODE, MAC_PARAM) => {
            mc_propagate_error(crate::errors::report_illegal_case_in(st))?;
        }
        (VMODE, SUP_MARK)
        | (HMODE, SUP_MARK)
        | (VMODE, SUB_MARK)
        | (HMODE, SUB_MARK)
        | (VMODE, MATH_CHAR_NUM)
        | (HMODE, MATH_CHAR_NUM)
        | (VMODE, MATH_GIVEN)
        | (HMODE, MATH_GIVEN)
        | (VMODE, XETEX_MATH_GIVEN)
        | (HMODE, XETEX_MATH_GIVEN)
        | (VMODE, MATH_COMP)
        | (HMODE, MATH_COMP)
        | (VMODE, DELIM_NUM)
        | (HMODE, DELIM_NUM)
        | (VMODE, LEFT_RIGHT)
        | (HMODE, LEFT_RIGHT)
        | (VMODE, ABOVE)
        | (HMODE, ABOVE)
        | (VMODE, RADICAL)
        | (HMODE, RADICAL)
        | (VMODE, MATH_STYLE)
        | (HMODE, MATH_STYLE)
        | (VMODE, MATH_CHOICE)
        | (HMODE, MATH_CHOICE)
        | (VMODE, VCENTER)
        | (HMODE, VCENTER)
        | (VMODE, NON_SCRIPT)
        | (HMODE, NON_SCRIPT)
        | (VMODE, MKERN)
        | (HMODE, MKERN)
        | (VMODE, LIMIT_SWITCH)
        | (HMODE, LIMIT_SWITCH)
        | (VMODE, MSKIP)
        | (HMODE, MSKIP)
        | (VMODE, MATH_ACCENT)
        | (HMODE, MATH_ACCENT)
        | (MMODE, ENDV)
        | (MMODE, PAR_END)
        | (MMODE, STOP)
        | (MMODE, VSKIP)
        | (MMODE, UN_VBOX)
        | (MMODE, VALIGN)
        | (MMODE, HRULE) => crate::scan::insert_dollar_sign_in(st)?,
        _ => {}
    }
    Ok(())
}

pub(crate) fn main_control_simple_list_group_cases_in(
    st: &mut EngineState,
) -> Result<(), MainControlAbort> {
    let mode = st.input.cur_list.mode.abs();
    match (mode, st.scanner.cur_cmd) {
        (VMODE, HRULE) | (HMODE, VRULE) | (MMODE, VRULE) => {
            let p = crate::scan::scan_rule_spec_in(st)?;
            tail_append(st, MemPtr::from_raw(p));
            if mode == VMODE {
                st.input.cur_list.set_aux_s1(IGNORE_DEPTH);
            } else if mode == HMODE {
                st.input.cur_list.set_aux_s0(1000);
            }
        }
        (VMODE, VSKIP) | (HMODE, HSKIP) | (MMODE, HSKIP) | (MMODE, MSKIP) => append_glue_in(st)?,
        (VMODE, KERN) | (HMODE, KERN) | (MMODE, KERN) | (MMODE, MKERN) => append_kern_in(st)?,
        (VMODE, LEFT_BRACE) | (HMODE, LEFT_BRACE) => {
            crate::save::new_save_level(&mut st.save_stack, GroupCode::Simple, st.mem.line)?;
        }
        (VMODE, BEGIN_GROUP) | (HMODE, BEGIN_GROUP) | (MMODE, BEGIN_GROUP) => {
            crate::save::new_save_level(&mut st.save_stack, GroupCode::SemiSimple, st.mem.line)?;
        }
        (VMODE, END_GROUP) | (HMODE, END_GROUP) | (MMODE, END_GROUP) => {
            if st.save_stack.cur_group == GroupCode::SemiSimple {
                crate::save::unsave_in(st)?;
            } else {
                off_save_in(st)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn main_control_box_list_cases_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    match (st.input.cur_list.mode.abs(), st.scanner.cur_cmd) {
        (VMODE, RIGHT_BRACE) | (HMODE, RIGHT_BRACE) | (MMODE, RIGHT_BRACE) => {
            handle_right_brace_in(st)?
        }
        (VMODE, HMOVE) | (HMODE, VMOVE) | (MMODE, VMOVE) => {
            let t = st.scanner.cur_chr;
            crate::scan::scan_dimen_in(st, false, false, false)?;
            if t == 0 {
                scan_box_in(st, st.scanner.cur_val)?;
            } else {
                scan_box_in(st, -st.scanner.cur_val)?;
            }
        }
        (VMODE, LEADER_SHIP) | (HMODE, LEADER_SHIP) | (MMODE, LEADER_SHIP) => {
            scan_box_in(st, LEADER_FLAG - A_LEADERS as i32 + st.scanner.cur_chr)?;
        }
        (VMODE, MAKE_BOX) | (HMODE, MAKE_BOX) | (MMODE, MAKE_BOX) => begin_box_in(st, 0)?,
        (VMODE, START_PAR) => new_graf_in(st, st.scanner.cur_chr > 0)?,
        (VMODE, LETTER)
        | (VMODE, OTHER_CHAR)
        | (VMODE, CHAR_NUM)
        | (VMODE, CHAR_GIVEN)
        | (VMODE, MATH_SHIFT)
        | (VMODE, UN_HBOX)
        | (VMODE, VRULE)
        | (VMODE, ACCENT)
        | (VMODE, DISCRETIONARY)
        | (VMODE, HSKIP)
        | (VMODE, VALIGN)
        | (VMODE, EX_SPACE)
        | (VMODE, NO_BOUNDARY) => {
            crate::input::back_input_in(st)?;
            new_graf_in(st, true)?;
        }
        (HMODE, START_PAR) | (MMODE, START_PAR) => {
            indent_in_hmode(st)?;
        }
        (VMODE, PAR_END) => {
            normal_paragraph(st)?;
            if st.input.cur_list.mode > 0 {
                crate::pagebuilder::build_page_in(st)?;
            }
        }
        (HMODE, PAR_END) => {
            if st.scanner.align_state < 0 {
                off_save_in(st)?;
            }
            end_graf_in(st)?;
            if st.input.cur_list.mode == VMODE as i16 {
                crate::pagebuilder::build_page_in(st)?;
            }
        }
        (HMODE, STOP) | (HMODE, VSKIP) | (HMODE, HRULE) | (HMODE, UN_VBOX) | (HMODE, HALIGN) => {
            head_for_vmode_in(st)?
        }
        (VMODE, INSERT)
        | (HMODE, INSERT)
        | (MMODE, INSERT)
        | (HMODE, VADJUST)
        | (MMODE, VADJUST) => begin_insert_or_adjust_in(st)?,
        (VMODE, MARK) | (HMODE, MARK) | (MMODE, MARK) => make_mark_in(st)?,
        (VMODE, BREAK_PENALTY) | (HMODE, BREAK_PENALTY) | (MMODE, BREAK_PENALTY) => {
            append_penalty_in(st)?
        }
        (VMODE, REMOVE_ITEM) | (HMODE, REMOVE_ITEM) | (MMODE, REMOVE_ITEM) => delete_last_in(st)?,
        (VMODE, UN_VBOX) | (HMODE, UN_HBOX) | (MMODE, UN_HBOX) => unpackage_in(st)?,
        (HMODE, ITAL_CORR) => append_italic_correction_in(st)?,
        (MMODE, ITAL_CORR) => {
            let p = crate::mem::alloc_kern(&mut st.mem, 0)
                .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
            tail_append(st, p.ptr());
        }
        (HMODE, DISCRETIONARY) | (MMODE, DISCRETIONARY) => append_discretionary_in(st)?,
        (HMODE, ACCENT) => make_accent_in(st)?,
        (VMODE, CAR_RET)
        | (HMODE, CAR_RET)
        | (MMODE, CAR_RET)
        | (VMODE, TAB_MARK)
        | (HMODE, TAB_MARK)
        | (MMODE, TAB_MARK) => align_error_in(st)?,
        _ => {}
    }
    Ok(())
}

pub(crate) fn main_control_alignment_cases_in(
    st: &mut EngineState,
) -> Result<(), MainControlAbort> {
    match (st.input.cur_list.mode.abs(), st.scanner.cur_cmd) {
        (VMODE, NO_ALIGN) | (HMODE, NO_ALIGN) | (MMODE, NO_ALIGN) => no_align_error_in(st)?,
        (VMODE, OMIT) | (HMODE, OMIT) | (MMODE, OMIT) => omit_error_in(st)?,
        (VMODE, HALIGN) => init_align_in(st)?,
        (HMODE, VALIGN) => {
            if st.scanner.cur_chr > 0 {
                let enabled = crate::scan::eTeX_enabled_in(
                    st,
                    st.eqtb.b32s1(INT_BASE + INT_PAR_TEXXET) > 0,
                    st.scanner.cur_cmd as u16,
                    st.scanner.cur_chr,
                )?;
                if enabled {
                    let p = crate::mem::alloc_math(&mut st.mem, 0, st.scanner.cur_chr as i16)
                        .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
                    tail_append(st, p.ptr());
                }
            } else {
                init_align_in(st)?;
            }
        }
        (MMODE, HALIGN) => {
            if crate::errors::privileged_in(st).map_err(MainControlAbort::Error)? {
                if st.save_stack.cur_group == GroupCode::MathShift {
                    init_align_in(st)?;
                } else {
                    off_save_in(st)?;
                }
            }
        }
        (VMODE, ENDV) | (HMODE, ENDV) => do_endv_in(st)?,
        (VMODE, END_CS_NAME) | (HMODE, END_CS_NAME) | (MMODE, END_CS_NAME) => {
            mc_propagate_error(crate::errors::cs_error_in(st))?
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn main_control_math_cases_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    extern "C" {}

    match (st.input.cur_list.mode.abs(), st.scanner.cur_cmd) {
        (HMODE, MATH_SHIFT) => init_math_in(st)?,

        (MMODE, EQ_NO) => {
            if crate::errors::privileged_in(st).map_err(MainControlAbort::Error)? {
                if st.save_stack.cur_group == GroupCode::MathShift {
                    crate::math::start_eq_no_in(st)?;
                } else {
                    off_save_in(st)?;
                }
            }
        }

        (MMODE, LEFT_BRACE) => {
            let n = crate::mem::alloc_noad(&mut st.mem)
                .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
            tail_append(st, n.ptr());
            crate::input::back_input_in(st)?;
            crate::scan::scan_math_in(st, st.input.cur_list.tail.0 + 1)?;
        }

        (MMODE, LETTER) | (MMODE, OTHER_CHAR) | (MMODE, CHAR_GIVEN) => {
            crate::scan::set_math_char_in(st, st.eqtb.b32s1(MATH_CODE_BASE + st.scanner.cur_chr))?;
        }

        (MMODE, CHAR_NUM) => {
            crate::scan::scan_char_num_in(st)?;
            st.scanner.cur_chr = st.scanner.cur_val;
            crate::scan::set_math_char_in(st, st.eqtb.b32s1(MATH_CODE_BASE + st.scanner.cur_chr))?;
        }

        (MMODE, MATH_CHAR_NUM) => {
            if st.scanner.cur_chr == 2 {
                // \Umathchar
                crate::scan::scan_math_class_int_in(st)?;
                let mut t = set_class_fn(st.scanner.cur_val);
                crate::scan::scan_math_fam_int_in(st)?;
                t += set_family_fn(st.scanner.cur_val);
                crate::scan::scan_usv_num_in(st)?;
                t += st.scanner.cur_val;
                crate::scan::set_math_char_in(st, t)?;
            } else if st.scanner.cur_chr == 1 {
                // \Umathcharnum
                crate::scan::scan_xetex_math_char_int_in(st)?;
                crate::scan::set_math_char_in(st, st.scanner.cur_val)?;
            } else {
                crate::scan::scan_fifteen_bit_int_in(st)?;
                let v = st.scanner.cur_val;
                crate::scan::set_math_char_in(
                    st,
                    set_class_fn(v / 4096) + set_family_fn((v % 4096) / 256) + (v % 256),
                )?;
            }
        }

        (MMODE, MATH_GIVEN) => {
            let c = st.scanner.cur_chr;
            crate::scan::set_math_char_in(
                st,
                set_class_fn(c / 4096) + set_family_fn((c % 4096) / 256) + (c % 256),
            )?;
        }

        (MMODE, XETEX_MATH_GIVEN) => {
            crate::scan::set_math_char_in(st, st.scanner.cur_chr)?;
        }

        (MMODE, DELIM_NUM) => {
            if st.scanner.cur_chr == 1 {
                // \Udelimiter
                crate::scan::scan_math_class_int_in(st)?;
                let mut t = set_class_fn(st.scanner.cur_val);
                crate::scan::scan_math_fam_int_in(st)?;
                t += set_family_fn(st.scanner.cur_val);
                crate::scan::scan_usv_num_in(st)?;
                t += st.scanner.cur_val;
                crate::scan::set_math_char_in(st, t)?;
            } else {
                crate::scan::scan_delimiter_int_in(st)?;
                let v = st.scanner.cur_val / 4096;
                crate::scan::set_math_char_in(
                    st,
                    set_class_fn(v / 4096) + set_family_fn((v % 4096) / 256) + (v % 256),
                )?;
            }
        }

        (MMODE, MATH_COMP) => {
            let n = crate::mem::alloc_noad(&mut st.mem)
                .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
            tail_append(st, n.ptr());
            st.mem
                .set_b16s1(st.input.cur_list.tail, st.scanner.cur_chr as u16);
            crate::scan::scan_math_in(st, st.input.cur_list.tail.0 + 1)?;
        }

        (MMODE, LIMIT_SWITCH) => crate::math::math_limit_switch_in(st)?,
        (MMODE, RADICAL) => crate::math::math_radical_in(st)?,
        (MMODE, ACCENT) | (MMODE, MATH_ACCENT) => crate::math::math_ac_in(st)?,

        (MMODE, VCENTER) => {
            crate::scan::scan_spec_in(st, GroupCode::Vcenter, false)?;
            normal_paragraph(st)?;
            crate::mem::push_nest_in(st)?;
            st.input.cur_list.mode = -1;
            st.input.cur_list.set_aux_s1(IGNORE_DEPTH);
            if st.shipout.insert_src_special_every_vbox {
                crate::ext::insert_src_special_in(st)?;
            }
            begin_every(st, LOCAL_EVERY_VBOX, EVERY_VBOX_TEXT)?;
        }

        (MMODE, MATH_STYLE) => {
            let s = crate::mem::alloc_style(&mut st.mem, st.scanner.cur_chr as i16)
                .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
            tail_append(st, s.ptr());
        }

        (MMODE, NON_SCRIPT) => {
            let g = crate::mem::alloc_glue(&mut st.mem, 0)
                .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
            tail_append(st, g.ptr());
            st.mem.set_b16s0(st.input.cur_list.tail, COND_MATH_GLUE);
        }

        (MMODE, MATH_CHOICE) => crate::math::append_choices_in(st)?,
        (MMODE, SUB_MARK) | (MMODE, SUP_MARK) => crate::math::sub_sup_in(st)?,
        (MMODE, ABOVE) => crate::math::math_fraction_in(st)?,
        (MMODE, LEFT_RIGHT) => crate::math::math_left_right_in(st)?,

        (MMODE, MATH_SHIFT) => {
            if st.save_stack.cur_group == GroupCode::MathShift {
                after_math_in(st)?;
            } else {
                off_save_in(st)?;
            }
        }

        _ => {}
    }
    Ok(())
}

pub(crate) fn main_control_ignore_spaces_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    if st.scanner.cur_chr == 0 {
        loop {
            crate::scan::get_x_token_in(st)?;
            if st.scanner.cur_cmd != SPACER {
                break;
            }
        }
    } else {
        let saved = st.scanner.scanner_status;
        st.scanner.scanner_status = NORMAL_SCANNER;
        let get_next_outcome = crate::scan::get_next_in(st);
        st.scanner.scanner_status = saved;
        let get_next_result: Result<(), crate::scan::SccAbort> = get_next_outcome.into();
        get_next_result?;

        if st.scanner.cur_cs < HASH_BASE {
            let v = crate::hash::prim_lookup(
                &mut st.eqtb,
                &mut st.strings,
                st.scanner.cur_cs - SINGLE_BASE,
            )
            .map_err(crate::scan::hash_err_abort)?;
            st.scanner.cur_cs = v;
        } else {
            let s = st.eqtb.hash_s1(st.scanner.cur_cs);
            let v = crate::hash::prim_lookup(&mut st.eqtb, &mut st.strings, s)
                .map_err(crate::scan::hash_err_abort)?;
            st.scanner.cur_cs = v;
        }

        if st.scanner.cur_cs != UNDEFINED_PRIMITIVE {
            st.scanner.cur_cmd = st.eqtb.b16s1(PRIM_EQTB_BASE + st.scanner.cur_cs) as u8;
            st.scanner.cur_chr = st.eqtb.b32s1(PRIM_EQTB_BASE + st.scanner.cur_cs);
            st.scanner.cur_tok = CS_TOKEN_FLAG + PRIM_EQTB_BASE + st.scanner.cur_cs;
        }
    }
    // Caller must do: goto reswitch
    Ok(())
}

enum InterCharInputAction {
    None,
    BackInputThenBeginTokenList {
        token_list: i32,
        force_cur_input_index: Option<u16>,
    },
}

impl InterCharInputAction {
    #[inline]
    fn execute_in(self, st: &mut EngineState) -> Result<bool, MainControlAbort> {
        match self {
            InterCharInputAction::None => Ok(false),
            InterCharInputAction::BackInputThenBeginTokenList {
                token_list,
                force_cur_input_index,
            } => {
                crate::input::back_input_in(st)?;
                if let Some(index) = force_cur_input_index {
                    st.input.cur_input.index = index;
                }
                crate::input::begin_token_list_in(st, token_list, INTER_CHAR_TEXT)?;
                Ok(true)
            }
        }
    }
}

fn set_current_token_for_inter_char_boundary(st: &mut EngineState, char_num_cmd: u8) {
    if st.scanner.cur_cs == 0 {
        if st.scanner.cur_cmd == char_num_cmd {
            st.scanner.cur_cmd = OTHER_CHAR;
        }
        st.scanner.cur_tok = st.scanner.cur_cmd as i32 * MAX_CHAR_VAL + st.scanner.cur_chr;
    } else {
        st.scanner.cur_tok = CS_TOKEN_FLAG + st.scanner.cur_cs;
    }
}

/// Probe `\XeTeXinterchartoks` for the current space-class followed by the
/// boundary class. The input-stack mutation is returned as an action so callers
/// can run `back_input`/`begin_token_list` after the state borrow drops.
fn inter_char_boundary_after_space_action(
    st: &mut EngineState,
    char_num_cmd: u8,
) -> InterCharInputAction {
    if st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_INTER_CHAR_TOKENS) <= 0 {
        return InterCharInputAction::None;
    }
    let sc = st.mainloop.space_class;
    if sc == CHAR_CLASS_LIMIT || st.mainloop.prev_class == CHAR_CLASS_LIMIT - 1 {
        return InterCharInputAction::None;
    }

    st.mainloop.prev_class = CHAR_CLASS_LIMIT - 1;
    crate::save::find_sa_element(
        &mut st.mem,
        &mut st.scanner,
        INTER_CHAR_VAL as u16,
        sc * CHAR_CLASS_LIMIT + CHAR_CLASS_LIMIT - 1,
        false,
    );
    let cur_ptr = st.scanner.cur_ptr.0;
    if cur_ptr == TEX_NULL {
        return InterCharInputAction::None;
    }
    let token_list = st.mem.b32s1(cur_ptr + 1);
    if token_list == TEX_NULL {
        return InterCharInputAction::None;
    }

    set_current_token_for_inter_char_boundary(st, char_num_cmd);
    InterCharInputAction::BackInputThenBeginTokenList {
        token_list,
        force_cur_input_index: None,
    }
}

fn inter_char_before_current_char_action(
    st: &mut EngineState,
    sc: i32,
    letter_cmd: u8,
    other_cmd: u8,
) -> InterCharInputAction {
    if st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_INTER_CHAR_TOKENS) <= 0 || sc == CHAR_CLASS_LIMIT {
        return InterCharInputAction::None;
    }

    let prev = st.mainloop.prev_class;
    if prev == CHAR_CLASS_LIMIT - 1 {
        let cur_input = st.input.cur_input;
        if cur_input.state == TOKEN_LIST && cur_input.index == BACKED_UP_CHAR {
            return InterCharInputAction::None;
        }
        crate::save::find_sa_element(
            &mut st.mem,
            &mut st.scanner,
            INTER_CHAR_VAL as u16,
            (CHAR_CLASS_LIMIT - 1) * CHAR_CLASS_LIMIT + sc,
            false,
        );
    } else {
        crate::save::find_sa_element(
            &mut st.mem,
            &mut st.scanner,
            INTER_CHAR_VAL as u16,
            prev * CHAR_CLASS_LIMIT + sc,
            false,
        );
    }

    let cur_ptr = st.scanner.cur_ptr.0;
    if cur_ptr == TEX_NULL {
        return InterCharInputAction::None;
    }
    let token_list = st.mem.b32s1(cur_ptr + 1);
    if token_list == TEX_NULL {
        return InterCharInputAction::None;
    }

    if st.scanner.cur_cmd != letter_cmd {
        st.scanner.cur_cmd = other_cmd;
    }
    st.scanner.cur_tok = st.scanner.cur_cmd as i32 * MAX_CHAR_VAL + st.scanner.cur_chr;
    InterCharInputAction::BackInputThenBeginTokenList {
        token_list,
        force_cur_input_index: Some(BACKED_UP_CHAR),
    }
}

// `is_char_node`: reads `hi_mem_min` directly (it can shrink on the get_avail
// hi-mem-exhaustion path, so it must be read fresh, not captured).
#[inline(always)]
fn mc_is_char_node(st: &EngineState, p: impl crate::units::MemAddr) -> bool {
    p.addr() >= st.mem.hi_mem_min
}

// Node-allocation helpers: allocate and lift the cold arena-exhaustion `None`
// into `MainControlAbort`.
#[inline]
fn mc_get_avail(st: &mut EngineState) -> Result<MemPtr, MainControlAbort> {
    crate::mem::get_avail(&mut st.mem).ok_or_else(|| crate::scan::NodeRecycleAbort::Overflow.into())
}
#[inline]
fn mc_new_disc(
    st: &mut EngineState,
) -> Result<crate::node::NodePtr<crate::node::DiscNode>, MainControlAbort> {
    Ok(crate::mem::alloc_disc(&mut st.mem).ok_or(crate::scan::NodeRecycleAbort::Overflow)?)
}
#[inline]
fn mc_new_kern(
    st: &mut EngineState,
    w: i32,
) -> Result<crate::node::NodePtr<crate::node::KernNode>, MainControlAbort> {
    Ok(crate::mem::alloc_kern(&mut st.mem, w).ok_or(crate::scan::NodeRecycleAbort::Overflow)?)
}
#[inline]
fn mc_new_ligature(
    st: &mut EngineState,
    f: i32,
    c: u16,
    q: i32,
) -> Result<crate::node::NodePtr<crate::node::LigatureNode>, MainControlAbort> {
    Ok(crate::mem::alloc_ligature(&mut st.mem, f, c, q)
        .ok_or(crate::scan::NodeRecycleAbort::Overflow)?)
}
#[inline]
fn mc_new_lig_item(
    st: &mut EngineState,
    c: u16,
) -> Result<crate::node::NodePtr<crate::node::LigItem>, MainControlAbort> {
    Ok(
        crate::mem::alloc_lig_item(&mut st.mem, c)
            .ok_or(crate::scan::NodeRecycleAbort::Overflow)?,
    )
}

// Returns true if the caller should goto big_switch (inter-char token was injected).
pub(crate) fn main_control_hmode_inter_char_check_in(
    st: &mut EngineState,
) -> Result<bool, MainControlAbort> {
    if st.input.cur_list.mode.abs() != HMODE {
        return Ok(false);
    }
    inter_char_boundary_after_space_action(st, CHAR_NUM).execute_in(st)
}

pub(crate) fn main_control_spacer_cases_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    match (st.input.cur_list.mode.abs(), st.scanner.cur_cmd) {
        (HMODE, SPACER) => {
            if st.input.cur_list.aux_s0() == 1000 {
                main_control_append_normal_space_in(st)?;
            } else {
                app_space_in(st)?;
            }
        }
        (HMODE, EX_SPACE) | (MMODE, EX_SPACE) => {
            main_control_append_normal_space_in(st)?;
        }
        _ => {}
    }
    Ok(())
}
// Returns true → caller must goto big_switch; false → caller must goto reswitch.
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
pub(crate) unsafe fn main_control_main_loop_in(
    st: &mut EngineState,
) -> Result<bool, MainControlAbort> {
    // === main_loop: entry point ===
    // SyncTeX: append special at start of new paragraph
    if st.input.cur_list.head == st.input.cur_list.tail
        && st.input.cur_list.mode > 0
        && st.shipout.insert_src_special_auto()
    {
        append_src_special_in(st)?;
    }
    st.mainloop.prev_class = CHAR_CLASS_LIMIT - 1;

    let mf = st.eqtb.b32s1(CUR_FONT_LOC);
    let fa = st.fonts.font_area_entry(mf);

    // =========================================================================
    // AAT / OTGR native font path (collect_native loop)
    // =========================================================================
    if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
        if st.input.cur_list.mode > 0
            && st.eqtb.b32s1(INT_BASE + INT_PAR_LANGUAGE) != st.input.cur_list.aux_s1()
        {
            fix_language(&mut st.input.cur_list, &mut st.mem, &st.eqtb)?;
        }
        st.mainloop.main_h = 0;
        st.mainloop.main_f = mf;
        st.scanner.set_native_len(0);

        // collect_native loop: accumulate consecutive chars into native_text buffer
        'collect: loop {
            // Update space factor from SF_CODE(cur_chr)
            let sf = st.eqtb.b32s1(SF_CODE_BASE + st.scanner.cur_chr);
            let ms = sf % 65536;
            st.mainloop.main_s = ms;
            if ms == 1000 {
                st.input.cur_list.set_aux_s0(1000);
            } else if ms < 1000 {
                if ms > 0 {
                    st.input.cur_list.set_aux_s0(ms);
                }
            } else if st.input.cur_list.aux_s0() < 1000 {
                st.input.cur_list.set_aux_s0(1000);
            } else {
                st.input.cur_list.set_aux_s0(ms);
            }

            st.scanner.cur_ptr.0 = TEX_NULL;
            st.mainloop.space_class = sf / 65536;
            let sc = st.mainloop.space_class;

            let was_boundary = st.mainloop.prev_class == CHAR_CLASS_LIMIT - 1;
            if inter_char_before_current_char_action(st, sc, LETTER, OTHER_CHAR).execute_in(st)? {
                if was_boundary {
                    return Ok(true); // goto big_switch
                }
                st.mainloop.prev_class = CHAR_CLASS_LIMIT - 1;
                break 'collect; // goto collected
            }
            if st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_INTER_CHAR_TOKENS) > 0
                && sc != CHAR_CLASS_LIMIT
            {
                st.mainloop.prev_class = sc;
            }

            // Append cur_chr to native_text buffer
            let ch = st.scanner.cur_chr;
            if ch > 65535 {
                while st.scanner.native_text_size() <= st.scanner.native_len() + 2 {
                    let size = st.scanner.native_text_size + 128;
                    st.scanner.native_text_size = size;
                    st.scanner.native_text.resize(size as usize, 0);
                }
                *st.scanner
                    .native_text
                    .as_mut_ptr()
                    .add(st.scanner.native_len() as usize) = ((ch - 65536) / 1024 + 0xD800) as u16;
                st.scanner.set_native_len(st.scanner.native_len() + 1);
                *st.scanner
                    .native_text
                    .as_mut_ptr()
                    .add(st.scanner.native_len() as usize) = ((ch - 65536) % 1024 + 0xDC00) as u16;
                st.scanner.set_native_len(st.scanner.native_len() + 1);
            } else {
                while st.scanner.native_text_size() <= st.scanner.native_len() + 1 {
                    let size = st.scanner.native_text_size + 128;
                    st.scanner.native_text_size = size;
                    st.scanner.native_text.resize(size as usize, 0);
                }
                *st.scanner
                    .native_text
                    .as_mut_ptr()
                    .add(st.scanner.native_len() as usize) = ch as u16;
                st.scanner.set_native_len(st.scanner.native_len() + 1);
            }

            // Check if this char is a hyphen/dash
            let is_h = ch == st.fonts.hyphen_char_entry(st.mainloop.main_f)
                || (st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_DASH_BREAK) > 0
                    && (ch == 8212 || ch == 8211));
            st.hyph.set_is_hyph(is_h);
            if st.mainloop.main_h == 0 && is_h {
                st.mainloop.main_h = st.scanner.native_len();
            }

            // Look ahead for more chars (same font)
            crate::scan::get_next_scc(st)?;
            if st.scanner.cur_cmd == LETTER
                || st.scanner.cur_cmd == OTHER_CHAR
                || st.scanner.cur_cmd == CHAR_GIVEN
            {
                continue 'collect;
            }
            crate::scan::x_token_in(st)?;
            if st.scanner.cur_cmd == LETTER
                || st.scanner.cur_cmd == OTHER_CHAR
                || st.scanner.cur_cmd == CHAR_GIVEN
            {
                continue 'collect;
            }
            if st.scanner.cur_cmd == CHAR_NUM {
                crate::scan::scan_usv_num_in(st)?;
                st.scanner.cur_chr = st.scanner.cur_val;
                continue 'collect;
            }

            // End of char run: inter-char suffix check
            if inter_char_boundary_after_space_action(st, CHAR_NUM).execute_in(st)? {
                break 'collect; // goto collected (then process mapping)
            }
            break 'collect; // natural end of collect_native
        } // end 'collect

        // collected:
        // Apply font mapping if present
        let fm = st.fonts.font_mapping_ptr(st.mainloop.main_f);
        if !fm.is_null() {
            let nt = st.scanner.native_text.as_mut_ptr();
            let nl = st.scanner.native_len();
            let mk = apply_mapping(&mut st.fonts, fm, nt, nl);
            st.scanner.set_native_len(0);
            while st.scanner.native_text_size() <= st.scanner.native_len() + mk {
                let size = st.scanner.native_text_size + 128;
                st.scanner.native_text_size = size;
                st.scanner.native_text.resize(size as usize, 0);
            }
            st.mainloop.main_h = 0;
            for mp in 0..mk {
                let c = st.fonts.mapped_text_entry(mp);
                *st.scanner
                    .native_text
                    .as_mut_ptr()
                    .add(st.scanner.native_len() as usize) = c;
                st.scanner.set_native_len(st.scanner.native_len() + 1);
                if st.mainloop.main_h == 0
                    && (c == st.fonts.hyphen_char_entry(st.mainloop.main_f) as u16
                        || (st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_DASH_BREAK) > 0
                            && (c == 8212 || c == 8211)))
                {
                    st.mainloop.main_h = st.scanner.native_len();
                }
            }
        }

        // Tracing lost chars
        if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_LOST_CHARS) > 0 {
            let mut tp = 0i32;
            while tp < st.scanner.native_len() {
                let mut mc = *st.scanner.native_text.as_mut_ptr().add(tp as usize) as i32;
                tp += 1;
                if (0xD800..0xDC00).contains(&mc) {
                    mc = 65536 + (mc - 0xD800) * 1024;
                    mc += *st.scanner.native_text.as_mut_ptr().add(tp as usize) as i32 - 0xDC00;
                    tp += 1;
                }
                if crate::ext::map_char_to_glyph(&st.fonts, st.mainloop.main_f, mc)? == 0 {
                    mc_propagate_error(crate::output::char_warning_in(st, st.mainloop.main_f, mc))?;
                }
            }
        }

        st.mainloop.main_p.0 = st.input.cur_list.tail.0;
        let mk = st.scanner.native_len();
        st.mainloop.main_pp.0 = st.input.cur_list.tail.0;

        if st.input.cur_list.mode == 104
        /* HMODE */
        {
            st.mainloop.main_ppp.0 = st.input.cur_list.head.0;
            // Advance main_ppp to find node just before main_pp
            while st.mainloop.main_ppp.0 != st.mainloop.main_pp.0
                && st.mem.b32s1(st.mainloop.main_ppp) != st.mainloop.main_pp.0
            {
                if !mc_is_char_node(st, st.mainloop.main_ppp.0)
                    && st.mem.b16s1(st.mainloop.main_ppp) == DISC_NODE
                {
                    let tp = st.mainloop.main_ppp.0;
                    for _ in 1..=st.mem.b16s0(tp) as i32 {
                        st.mainloop.main_ppp.0 = st.mem.b32s1(st.mainloop.main_ppp);
                    }
                }
                if st.mainloop.main_ppp.0 != st.mainloop.main_pp.0 {
                    st.mainloop.main_ppp.0 = st.mem.b32s1(st.mainloop.main_ppp);
                }
            }

            // C uses `main_k` as the running remaining-length and `temp_ptr` as
            // the offset into native_text of the current segment; both mutate
            // across the do-while. `main_h` (global) is the next hyphen offset.
            let mut temp_ptr = 0i32;
            let mut main_k = mk;
            loop {
                if st.mainloop.main_h == 0 {
                    st.mainloop.main_h = main_k;
                }

                let tail_is_native = st.mainloop.main_pp.0 != TEX_NULL
                    && !mc_is_char_node(st, st.mainloop.main_pp.0)
                    && st.mem.b16s1(st.mainloop.main_pp) == WHATSIT_NODE
                    && crate::node::WhatsitSubtype::from_raw(st.mem.b16s0(st.mainloop.main_pp))
                        .is_some_and(|s| s.is_native_word())
                    && st.mem.b16s2(st.mainloop.main_pp + 4) == st.mainloop.main_f as u16
                    && st.mainloop.main_ppp.0 != st.mainloop.main_pp.0
                    && !mc_is_char_node(st, st.mainloop.main_ppp.0)
                    && st.mem.b16s1(st.mainloop.main_ppp) != DISC_NODE;

                if tail_is_native {
                    let old_len = st.mem.b16s1(st.mainloop.main_pp + 4) as i32;
                    main_k = st.mainloop.main_h + old_len;
                    while st.scanner.native_text_size() <= st.scanner.native_len() + main_k {
                        let size = st.scanner.native_text_size + 128;
                        st.scanner.native_text_size = size;
                        st.scanner.native_text.resize(size as usize, 0);
                    }
                    st.scanner.set_save_native_len(st.scanner.native_len());
                    for i in 0..old_len {
                        *st.scanner
                            .native_text
                            .as_mut_ptr()
                            .add(st.scanner.native_len() as usize) = *st
                            .mem
                            .as_u16_ptr(st.mainloop.main_pp.0 + NATIVE_NODE_SIZE)
                            .add(i as usize);
                        st.scanner.set_native_len(st.scanner.native_len() + 1);
                    }
                    for i in 0..st.mainloop.main_h {
                        *st.scanner
                            .native_text
                            .as_mut_ptr()
                            .add(st.scanner.native_len() as usize) = *st
                            .scanner
                            .native_text
                            .as_mut_ptr()
                            .add((temp_ptr + i) as usize);
                        st.scanner.set_native_len(st.scanner.native_len() + 1);
                    }
                    crate::linebreak::do_locale_linebreaks_in(
                        st,
                        st.scanner.save_native_len(),
                        main_k,
                    )?;
                    st.scanner.set_native_len(st.scanner.save_native_len());
                    main_k = st.scanner.native_len() - st.mainloop.main_h - temp_ptr;
                    temp_ptr = st.mainloop.main_h;
                    st.mainloop.main_h = 0;
                    while st.mainloop.main_h < main_k
                        && (*st
                            .scanner
                            .native_text
                            .as_mut_ptr()
                            .add((temp_ptr + st.mainloop.main_h) as usize)
                            as i32)
                            != st.fonts.hyphen_char_entry(st.mainloop.main_f)
                        && (st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_DASH_BREAK) <= 0
                            || ((*st
                                .scanner
                                .native_text
                                .as_mut_ptr()
                                .add((temp_ptr + st.mainloop.main_h) as usize)
                                as i32)
                                != 8212
                                && (*st
                                    .scanner
                                    .native_text
                                    .as_mut_ptr()
                                    .add((temp_ptr + st.mainloop.main_h) as usize)
                                    as i32)
                                    != 8211))
                    {
                        st.mainloop.main_h = st.mainloop.main_h + 1;
                    }
                    if st.mainloop.main_h < main_k {
                        st.mainloop.main_h = st.mainloop.main_h + 1;
                    }
                    st.mem
                        .set_b32s1(st.mainloop.main_ppp, st.mem.b32s1(st.mainloop.main_pp));
                    st.mem.set_b32s1(st.mainloop.main_pp, TEX_NULL);
                    crate::packaging::flush_node_list(&mut st.mem, st.mainloop.main_pp.0)
                        .map_err(MainControlAbort::Confusion)?;
                    st.mainloop.main_pp.0 = st.input.cur_list.tail.0;
                    while st.mem.b32s1(st.mainloop.main_ppp) != st.mainloop.main_pp.0 {
                        st.mainloop.main_ppp.0 = st.mem.b32s1(st.mainloop.main_ppp);
                    }
                } else {
                    crate::linebreak::do_locale_linebreaks_in(st, temp_ptr, st.mainloop.main_h)?;
                    temp_ptr += st.mainloop.main_h;
                    main_k -= st.mainloop.main_h;
                    st.mainloop.main_h = 0;
                    while st.mainloop.main_h < main_k
                        && (*st
                            .scanner
                            .native_text
                            .as_mut_ptr()
                            .add((temp_ptr + st.mainloop.main_h) as usize)
                            as i32)
                            != st.fonts.hyphen_char_entry(st.mainloop.main_f)
                        && (st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_DASH_BREAK) <= 0
                            || ((*st
                                .scanner
                                .native_text
                                .as_mut_ptr()
                                .add((temp_ptr + st.mainloop.main_h) as usize)
                                as i32)
                                != 8212
                                && (*st
                                    .scanner
                                    .native_text
                                    .as_mut_ptr()
                                    .add((temp_ptr + st.mainloop.main_h) as usize)
                                    as i32)
                                    != 8211))
                    {
                        st.mainloop.main_h = st.mainloop.main_h + 1;
                    }
                    if st.mainloop.main_h < main_k {
                        st.mainloop.main_h = st.mainloop.main_h + 1;
                    }
                }

                if main_k > 0 || st.hyph.is_hyph() {
                    let nd = mc_new_disc(st)?;
                    tail_append(st, nd.ptr());
                    st.mainloop.main_pp.0 = st.input.cur_list.tail.0;
                }

                if main_k == 0 {
                    break;
                }
            }
        } else {
            // MMODE (or not HMODE): simple native word node creation
            st.mainloop.main_ppp.0 = st.input.cur_list.head.0;
            while st.mainloop.main_ppp.0 != st.mainloop.main_pp.0
                && st.mem.b32s1(st.mainloop.main_ppp) != st.mainloop.main_pp.0
            {
                if !mc_is_char_node(st, st.mainloop.main_ppp.0)
                    && st.mem.b16s1(st.mainloop.main_ppp) == DISC_NODE
                {
                    let tp = st.mainloop.main_ppp.0;
                    for _ in 1..=st.mem.b16s0(tp) as i32 {
                        st.mainloop.main_ppp.0 = st.mem.b32s1(st.mainloop.main_ppp);
                    }
                }
                if st.mainloop.main_ppp.0 != st.mainloop.main_pp.0 {
                    st.mainloop.main_ppp.0 = st.mem.b32s1(st.mainloop.main_ppp);
                }
            }

            let tail_is_native = st.mainloop.main_pp.0 != TEX_NULL
                && !mc_is_char_node(st, st.mainloop.main_pp.0)
                && st.mem.b16s1(st.mainloop.main_pp) == WHATSIT_NODE
                && crate::node::WhatsitSubtype::from_raw(st.mem.b16s0(st.mainloop.main_pp))
                    .is_some_and(|s| s.is_native_word())
                && st.mem.b16s2(st.mainloop.main_pp + 4) == mf as u16
                && st.mainloop.main_ppp.0 != st.mainloop.main_pp.0
                && !mc_is_char_node(st, st.mainloop.main_ppp.0)
                && st.mem.b16s1(st.mainloop.main_ppp) != DISC_NODE;

            if tail_is_native {
                let old_len = st.mem.b16s1(st.mainloop.main_pp + 4) as i32;
                let new_node =
                    crate::fonts::new_native_word_node(&mut st.mem, &st.eqtb, mf, mk + old_len)?;
                st.mem.set_b32s1(st.mainloop.main_pp, new_node);
                st.input.cur_list.tail.0 = st.mem.b32s1(st.mainloop.main_pp);
                st.mem.journal_mark_range(
                    st.input.cur_list.tail.0 + NATIVE_NODE_SIZE,
                    crate::state::native_text_words(mk + old_len),
                );
                for i in 0..old_len {
                    *st.mem
                        .as_u16_ptr_mut(st.input.cur_list.tail.0 + NATIVE_NODE_SIZE)
                        .add(i as usize) = *st
                        .mem
                        .as_u16_ptr(st.mainloop.main_pp.0 + NATIVE_NODE_SIZE)
                        .add(i as usize);
                }
                for i in 0..mk {
                    *st.mem
                        .as_u16_ptr_mut(st.input.cur_list.tail.0 + NATIVE_NODE_SIZE)
                        .add((i + old_len) as usize) =
                        *st.scanner.native_text.as_mut_ptr().add(i as usize);
                }
                crate::ext::measure_native_node(
                    &st.fonts,
                    &mut st.mem,
                    st.input.cur_list.tail.0,
                    (st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_USE_GLYPH_METRICS) > 0) as libc::c_int,
                )?;
                let mut prev = st.input.cur_list.head;
                if prev != st.mainloop.main_pp {
                    while prev.link(&st.mem) != st.mainloop.main_pp {
                        prev = prev.link(&st.mem);
                    }
                }
                let main_pp_link = st.mainloop.main_pp.link(&st.mem);
                prev.set_link(&mut st.mem, main_pp_link);
                st.mem.set_b32s1(st.mainloop.main_pp, TEX_NULL);
                crate::packaging::flush_node_list(&mut st.mem, st.mainloop.main_pp.0)
                    .map_err(MainControlAbort::Confusion)?;
            } else {
                let new_node = crate::fonts::new_native_word_node(&mut st.mem, &st.eqtb, mf, mk)?;
                st.mem.set_b32s1(st.mainloop.main_pp, new_node);
                st.input.cur_list.tail.0 = st.mem.b32s1(st.mainloop.main_pp);
                st.mem.journal_mark_range(
                    st.input.cur_list.tail.0 + NATIVE_NODE_SIZE,
                    crate::state::native_text_words(mk),
                );
                for i in 0..mk {
                    *st.mem
                        .as_u16_ptr_mut(st.input.cur_list.tail.0 + NATIVE_NODE_SIZE)
                        .add(i as usize) = *st.scanner.native_text.as_mut_ptr().add(i as usize);
                }
                crate::ext::measure_native_node(
                    &st.fonts,
                    &mut st.mem,
                    st.input.cur_list.tail.0,
                    (st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_USE_GLYPH_METRICS) > 0) as libc::c_int,
                )?;
            }
        }

        // Interword space shaping (both HMODE and MMODE)
        if st
            .eqtb
            .b32s1(INT_BASE + INT_PAR_XETEX_INTERWORD_SPACE_SHAPING)
            > 0
        {
            let mut p = st.input.cur_list.head;
            let mut pp = MemPtr::NULL;
            while p != st.input.cur_list.tail {
                if !p.is_null()
                    && !mc_is_char_node(st, p)
                    && st.mem.b16s1(p) == WHATSIT_NODE
                    && crate::node::WhatsitSubtype::from_raw(st.mem.b16s0(p))
                        .is_some_and(|s| s.is_native_word())
                {
                    pp = p;
                }
                p = p.link(&st.mem);
            }
            if !pp.is_null() && st.mem.b16s2(pp + 4) == mf as u16 {
                let mut p2 = pp.link(&st.mem);
                while !mc_is_char_node(st, p2)
                    && (st.mem.b16s1(p2) == PENALTY_NODE
                        || st.mem.b16s1(p2) == INS_NODE
                        || st.mem.b16s1(p2) == MARK_NODE
                        || st.mem.b16s1(p2) == ADJUST_NODE
                        || (st.mem.b16s1(p2) == WHATSIT_NODE && st.mem.b16s0(p2) <= 4))
                {
                    p2 = p2.link(&st.mem);
                }
                if !mc_is_char_node(st, p2) && st.mem.b16s1(p2) == GLUE_NODE {
                    let p3 = p2.link(&st.mem);
                    let mut p3m = p3;
                    while !mc_is_char_node(st, p3m)
                        && (st.mem.b16s1(p3m) == PENALTY_NODE
                            || st.mem.b16s1(p3m) == INS_NODE
                            || st.mem.b16s1(p3m) == MARK_NODE
                            || st.mem.b16s1(p3m) == ADJUST_NODE
                            || (st.mem.b16s1(p3m) == WHATSIT_NODE && st.mem.b16s0(p3m) <= 4))
                    {
                        p3m = p3m.link(&st.mem);
                    }
                    if p3m == st.input.cur_list.tail {
                        let pp_len = st.mem.b16s1(pp + 4) as i32;
                        let tail_len = st.mem.b16s1(st.input.cur_list.tail + 4) as i32;
                        let tmp = crate::fonts::new_native_word_node(
                            &mut st.mem,
                            &st.eqtb,
                            mf,
                            pp_len + 1 + tail_len,
                        )?;
                        st.mem.journal_mark_range(
                            tmp + NATIVE_NODE_SIZE,
                            crate::state::native_text_words(pp_len + 1 + tail_len),
                        );
                        let mut ki = 0i32;
                        for t in 0..pp_len {
                            *st.mem
                                .as_u16_ptr_mut(tmp + NATIVE_NODE_SIZE)
                                .add(ki as usize) =
                                *st.mem.as_u16_ptr(pp + NATIVE_NODE_SIZE).add(t as usize);
                            ki += 1;
                        }
                        *st.mem
                            .as_u16_ptr_mut(tmp + NATIVE_NODE_SIZE)
                            .add(ki as usize) = b' ' as u16;
                        ki += 1;
                        for t in 0..tail_len {
                            *st.mem
                                .as_u16_ptr_mut(tmp + NATIVE_NODE_SIZE)
                                .add(ki as usize) = *st
                                .mem
                                .as_u16_ptr(st.input.cur_list.tail.0 + NATIVE_NODE_SIZE)
                                .add(t as usize);
                            ki += 1;
                        }
                        crate::ext::measure_native_node(
                            &st.fonts,
                            &mut st.mem,
                            tmp,
                            (st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_USE_GLYPH_METRICS) > 0)
                                as libc::c_int,
                        )?;
                        let t_val = st.mem.b32s1(tmp + 1)
                            - st.mem.b32s1(pp + 1)
                            - st.mem.b32s1(st.input.cur_list.tail + 1);
                        let tmp_handle = st.mem.b32s0(tmp + 5);
                        if tmp_handle != 0 {
                            st.mem.glyph_blobs.release(tmp_handle);
                        }
                        let fnsz = st.mem.b16s3(tmp + 4) as i32;
                        crate::mem::free_node(&mut st.mem, MemPtr::from_raw(tmp), fnsz);
                        if t_val != st.mem.b32s1(st.fonts.font_glue(mf) + 1) {
                            let kw = t_val - st.mem.b32s1(st.fonts.font_glue(mf) + 1);
                            let kn = mc_new_kern(st, kw)?;
                            kn.set_subtype(&mut st.mem, SPACE_ADJUSTMENT);
                            let p2_link = st.mem.b32s1(p2);
                            kn.set_link(&mut st.mem, MemPtr::from_raw(p2_link));
                            st.mem.set_b32s1(p2, kn.ptr().0);
                        }
                    }
                }
            }
        }

        // Exit: cur_ptr set iff inter-char token injected
        return Ok(st.scanner.cur_ptr.0 != TEX_NULL);
    }

    // =========================================================================
    // Regular TFM font path
    // =========================================================================

    // Update space factor from SF_CODE(cur_chr)
    let sf = st.eqtb.b32s1(SF_CODE_BASE + st.scanner.cur_chr);
    let ms = sf % 65536;
    st.mainloop.main_s = ms;
    if ms == 1000 {
        st.input.cur_list.set_aux_s0(1000);
    } else if ms < 1000 {
        if ms > 0 {
            st.input.cur_list.set_aux_s0(ms);
        }
    } else if st.input.cur_list.aux_s0() < 1000 {
        st.input.cur_list.set_aux_s0(1000);
    } else {
        st.input.cur_list.set_aux_s0(ms);
    }

    st.scanner.cur_ptr.0 = TEX_NULL;
    st.mainloop.space_class = sf / 65536;
    let sc = st.mainloop.space_class;

    if inter_char_before_current_char_action(st, sc, LETTER, OTHER_CHAR).execute_in(st)? {
        return Ok(true); // goto big_switch
    }
    if st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_INTER_CHAR_TOKENS) > 0 && sc != CHAR_CLASS_LIMIT {
        st.mainloop.prev_class = sc;
    }

    // Set up font state
    st.mainloop.main_f = mf;
    st.mainloop.bchar = st.fonts.font_bchar_entry(mf);
    st.mainloop.false_bchar = st.fonts.font_false_bchar_entry(mf);

    if st.input.cur_list.mode > 0
        && st.eqtb.b32s1(INT_BASE + INT_PAR_LANGUAGE) != st.input.cur_list.aux_s1()
    {
        fix_language(&mut st.input.cur_list, &mut st.mem, &st.eqtb)?;
    }

    // Allocate lig_stack node from avail list
    let ls = mc_get_avail(st)?;
    st.mainloop.lig_stack = ls;
    st.mem.set_b16s1(ls, mf as u16);
    st.mainloop.cur_l = st.scanner.cur_chr;
    st.mem.set_b16s0(ls, st.mainloop.cur_l as u16);
    st.mainloop.cur_q.0 = st.input.cur_list.tail.0;

    // Determine entry point: NON_ADDRESS → move_2; else → lig_loop_1 (bchar)
    let bchar_lbl = if st.mainloop.cancel_boundary {
        st.mainloop.cancel_boundary = false;
        NON_ADDRESS
    } else {
        st.fonts.bchar_label_entry(mf)
    };

    // `enter_lig1_k`: if Some(k), enter lig loop at _1 with that k; else enter at move_2
    let mut enter_lig1_k: Option<i32> = if bchar_lbl != NON_ADDRESS {
        st.mainloop.cur_r = st.mainloop.cur_l;
        st.mainloop.cur_l = TOO_BIG_CHAR;
        Some(bchar_lbl)
    } else {
        None
    };

    // main_i: char info for cur_l (set at move_2 or when cur_l changes in lig loop)
    let mut main_i = crate::mem_types::B16x4 {
        s0: 0,
        s1: 0,
        s2: 0,
        s3: 0,
    };

    // -------------------------------------------------------------------------
    // Outer char-processing loop.
    // Each iteration: run the lig/kern table, then advance to the next char.
    // -------------------------------------------------------------------------
    'char_proc: loop {
        // Determine how to enter the lig/kern loop this iteration:
        // - enter_lig1_k = Some(k): start at lig_loop_1 (skips the precondition check)
        // - enter_lig1_k = None: start at lig_loop top (checks LIG_TAG and cur_r)

        // "lig_result": 0=wrapup, 1=goto_move (skip wrapup), 2=goto_move1 (skip wrapup+move-check)
        let lig_result: u8;

        if let Some(init_k) = enter_lig1_k.take() {
            // Enter at main_lig_loop_1 directly (bchar case or after lig advance)
            lig_result = lig_inner_loop(st, init_k, &mut main_i, &mut enter_lig1_k)?;
        } else {
            // Enter at main_lig_loop top: check preconditions
            if (main_i.s1 % 4) as u8 != LIG_TAG as u8 || st.mainloop.cur_r == TOO_BIG_CHAR {
                lig_result = 0; // goto main_loop_wrapup
            } else {
                let init_k = st.fonts.lig_kern_base_entry(st.mainloop.main_f) + main_i.s0 as i32;
                let init_j = st.fonts.font_info_b16(init_k);
                let real_k = if init_j.s3 > 128 {
                    st.fonts.lig_kern_base_entry(st.mainloop.main_f)
                        + 256 * init_j.s1 as i32
                        + init_j.s0 as i32
                        + 32768
                        - 256 * 128
                } else {
                    init_k
                };
                lig_result = lig_inner_loop(st, real_k, &mut main_i, &mut enter_lig1_k)?;
            }
        }

        // -----------------------------------------------------------------------
        // Handle lig_result: wrapup, move, or move1
        // -----------------------------------------------------------------------
        // 0xFF: lig type 1/2/3/7 advanced cur_l; re-enter lig_loop top (no wrapup/move)
        if lig_result == 0xFF {
            continue 'char_proc;
        }

        if lig_result == 0 {
            // main_loop_wrapup: only when lig_result==0 (result 1=goto_move, 2=goto_move1 both skip wrapup)
            if st.mainloop.cur_l < TOO_BIG_CHAR {
                if st.mem.b32s1(st.mainloop.cur_q) > TEX_NULL
                    && st.mem.b16s0(st.input.cur_list.tail)
                        == st.fonts.hyphen_char_entry(st.mainloop.main_f) as u16
                {
                    st.mainloop.ins_disc = true;
                }
                if st.mainloop.ligature_present {
                    let lq = st.mem.b32s1(st.mainloop.cur_q);
                    let mp = mc_new_ligature(st, st.mainloop.main_f, st.mainloop.cur_l as u16, lq)?;
                    if st.mainloop.lft_hit {
                        mp.set_subtype(&mut st.mem, 2);
                        st.mainloop.lft_hit = false;
                    }
                    if st.mainloop.rt_hit && st.mainloop.lig_stack.0 == TEX_NULL {
                        let s = mp.subtype(&st.mem);
                        mp.set_subtype(&mut st.mem, s + 1);
                        st.mainloop.rt_hit = false;
                    }
                    st.mem.set_b32s1(st.mainloop.cur_q, mp.ptr().0);
                    st.input.cur_list.tail = mp.ptr();
                    st.mainloop.ligature_present = false;
                }
                if st.mainloop.ins_disc {
                    st.mainloop.ins_disc = false;
                    if st.input.cur_list.mode > 0 {
                        let nd = mc_new_disc(st)?;
                        tail_append(st, nd.ptr());
                    }
                }
            }
        }
        // If lig_result == 1: skip wrapup, go directly to move

        // main_loop_move: check if lig_stack is exhausted
        // (for lig_result==2 / move1: skip this check, cur_l already set)
        if lig_result != 2 {
            if st.mainloop.lig_stack.0 == TEX_NULL {
                return Ok(false); // goto reswitch
            }
            st.mainloop.cur_q.0 = st.input.cur_list.tail.0;
            st.mainloop.cur_l = st.mem.b16s0(st.mainloop.lig_stack) as i32;
        }

        // main_loop_move_1: is lig_stack a char node or a lig item?
        if !mc_is_char_node(st, st.mainloop.lig_stack.0) {
            // main_loop_move_lig: handle lig item in lig_stack
            st.mainloop.main_p.0 = st.mem.b32s1(st.mainloop.lig_stack + 1);
            if st.mainloop.main_p.0 > TEX_NULL {
                tail_append(st, st.mainloop.main_p);
            }
            let temp = st.mainloop.lig_stack.0;
            st.mainloop.lig_stack.0 = st.mem.b32s1(temp);
            crate::mem::free_node(&mut st.mem, MemPtr::from_raw(temp), SMALL_NODE_SIZE);
            let ec = crate::fonts::effective_char(
                &mut st.fonts,
                true,
                st.mainloop.main_f,
                st.mainloop.cur_l as u16,
            );
            main_i = st
                .fonts
                .font_info_b16(st.fonts.char_base_entry(st.mainloop.main_f) + ec);
            st.mainloop.ligature_present = true;

            if st.mainloop.lig_stack.0 == TEX_NULL {
                if st.mainloop.main_p.0 > TEX_NULL {
                    // goto main_loop_lookahead (enter with main_i set, skip move_2)
                    // We do the lookahead inline here:
                    if !do_lookahead(st, &mut main_i, &mut enter_lig1_k, mf)? {
                        return Ok(true); // inter-char token injected → big_switch
                    }
                    continue 'char_proc;
                } else {
                    st.mainloop.cur_r = st.mainloop.bchar;
                }
            } else {
                st.mainloop.cur_r = st.mem.b16s0(st.mainloop.lig_stack) as i32;
            }
            // goto main_lig_loop (outer top) — continue 'char_proc
            continue 'char_proc;
        }

        // main_loop_move_2: validate and append the char node (lig_stack is char)
        let eff = crate::fonts::effective_char(
            &mut st.fonts,
            false,
            st.mainloop.main_f,
            st.scanner.cur_chr as u16,
        );
        if eff > st.fonts.font_ec_entry(st.mainloop.main_f) as i32
            || eff < st.fonts.font_bc_entry(st.mainloop.main_f) as i32
        {
            mc_propagate_error(crate::output::char_warning_in(
                st,
                st.mainloop.main_f,
                st.scanner.cur_chr,
            ))?;
            st.mem.set_b32s1(st.mainloop.lig_stack, st.mem.avail());
            st.mem.set_avail(st.mainloop.lig_stack.0);
            return Ok(true); // goto big_switch
        }
        main_i = crate::fonts::effective_char_info(
            &mut st.fonts,
            st.mainloop.main_f,
            st.mainloop.cur_l as u16,
        );
        if main_i.s3 == 0 {
            mc_propagate_error(crate::output::char_warning_in(
                st,
                st.mainloop.main_f,
                st.scanner.cur_chr,
            ))?;
            st.mem.set_b32s1(st.mainloop.lig_stack, st.mem.avail());
            st.mem.set_avail(st.mainloop.lig_stack.0);
            return Ok(true); // goto big_switch
        }
        tail_append(st, st.mainloop.lig_stack);

        // main_loop_lookahead: get next token, set up lig_stack for it
        if !do_lookahead(st, &mut main_i, &mut enter_lig1_k, mf)? {
            return Ok(true); // inter-char token injected → big_switch
        }
        // continue 'char_proc → back to top of lig loop
    } // end 'char_proc
}
// Inner lig/kern table scan (main_lig_loop_1 / main_lig_loop_2).
// Returns: 0=wrapup, 1=goto_move (skip wrapup), 2=goto_move1 (lig type 0/4 with stack)
// Also sets enter_lig1_k if we need to re-enter lig_loop_1 (bchar advance).
#[inline(always)]
unsafe fn lig_inner_loop(
    st: &mut EngineState,
    init_k: i32,
    main_i: &mut crate::mem_types::B16x4,
    _enter_lig1_k: &mut Option<i32>,
) -> Result<u8, MainControlAbort> {
    let mut mk = init_k;

    loop {
        // main_lig_loop_1 / main_lig_loop_2
        let mj = st.fonts.font_info_b16(mk);

        if mj.s2 as i32 == st.mainloop.cur_r {
            if mj.s3 <= 128 {
                if mj.s1 >= 128 {
                    // Kern case: partial wrapup + append kern
                    if st.mainloop.cur_l < TOO_BIG_CHAR {
                        if st.mem.b32s1(st.mainloop.cur_q) > TEX_NULL
                            && st.mem.b16s0(st.input.cur_list.tail)
                                == st.fonts.hyphen_char_entry(st.mainloop.main_f) as u16
                        {
                            st.mainloop.ins_disc = true;
                        }
                        if st.mainloop.ligature_present {
                            let lq = st.mem.b32s1(st.mainloop.cur_q);
                            let mp = mc_new_ligature(
                                st,
                                st.mainloop.main_f,
                                st.mainloop.cur_l as u16,
                                lq,
                            )?;
                            if st.mainloop.lft_hit {
                                mp.set_subtype(&mut st.mem, 2);
                                st.mainloop.lft_hit = false;
                            }
                            // rt_hit: only if lig_stack == NULL (not applicable here since we have kern)
                            st.mem.set_b32s1(st.mainloop.cur_q, mp.ptr().0);
                            st.input.cur_list.tail = mp.ptr();
                            st.mainloop.ligature_present = false;
                        }
                        if st.mainloop.ins_disc {
                            st.mainloop.ins_disc = false;
                            if st.input.cur_list.mode > 0 {
                                let nd = mc_new_disc(st)?;
                                tail_append(st, nd.ptr());
                            }
                        }
                    }
                    let kval = st.fonts.font_info_b32s1(
                        st.fonts.kern_base_entry(st.mainloop.main_f)
                            + 256 * mj.s1 as i32
                            + mj.s0 as i32,
                    );
                    let kn = mc_new_kern(st, kval)?;
                    tail_append(st, kn.ptr());
                    return Ok(1); // goto main_loop_move (skip wrapup)
                }

                // Ligature case
                if st.mainloop.cur_l == TOO_BIG_CHAR {
                    st.mainloop.lft_hit = true;
                } else if st.mainloop.lig_stack.0 == TEX_NULL {
                    st.mainloop.rt_hit = true;
                }

                match mj.s1 {
                    1 | 5 => {
                        st.mainloop.cur_l = mj.s0 as i32;
                        let ec = crate::fonts::effective_char(
                            &mut st.fonts,
                            true,
                            st.mainloop.main_f,
                            st.mainloop.cur_l as u16,
                        );
                        *main_i = st
                            .fonts
                            .font_info_b16(st.fonts.char_base_entry(st.mainloop.main_f) + ec);
                        st.mainloop.ligature_present = true;
                    }
                    2 | 6 => {
                        st.mainloop.cur_r = mj.s0 as i32;
                        if st.mainloop.lig_stack.0 == TEX_NULL {
                            st.mainloop.lig_stack =
                                mc_new_lig_item(st, st.mainloop.cur_r as u16)?.ptr();
                            st.mainloop.bchar = TOO_BIG_CHAR;
                        } else if mc_is_char_node(st, st.mainloop.lig_stack.0) {
                            let prev = st.mainloop.lig_stack.0;
                            st.mainloop.lig_stack =
                                mc_new_lig_item(st, st.mainloop.cur_r as u16)?.ptr();
                            st.mem.set_b32s1(st.mainloop.lig_stack + 1, prev);
                        } else {
                            st.mem
                                .set_b16s0(st.mainloop.lig_stack, st.mainloop.cur_r as u16);
                        }
                    }
                    3 => {
                        st.mainloop.cur_r = mj.s0 as i32;
                        let prev = st.mainloop.lig_stack.0;
                        st.mainloop.lig_stack =
                            mc_new_lig_item(st, st.mainloop.cur_r as u16)?.ptr();
                        st.mem.set_b32s1(st.mainloop.lig_stack, prev);
                    }
                    7 | 11 => {
                        // Partial wrapup
                        if st.mainloop.cur_l < TOO_BIG_CHAR {
                            if st.mem.b32s1(st.mainloop.cur_q) > TEX_NULL
                                && st.mem.b16s0(st.input.cur_list.tail)
                                    == st.fonts.hyphen_char_entry(st.mainloop.main_f) as u16
                            {
                                st.mainloop.ins_disc = true;
                            }
                            if st.mainloop.ligature_present {
                                let lq = st.mem.b32s1(st.mainloop.cur_q);
                                let mp = mc_new_ligature(
                                    st,
                                    st.mainloop.main_f,
                                    st.mainloop.cur_l as u16,
                                    lq,
                                )?;
                                if st.mainloop.lft_hit {
                                    mp.set_subtype(&mut st.mem, 2);
                                    st.mainloop.lft_hit = false;
                                }
                                // no rt_hit for type 7/11
                                st.mem.set_b32s1(st.mainloop.cur_q, mp.ptr().0);
                                st.input.cur_list.tail = mp.ptr();
                                st.mainloop.ligature_present = false;
                            }
                            if st.mainloop.ins_disc {
                                st.mainloop.ins_disc = false;
                                if st.input.cur_list.mode > 0 {
                                    let nd = mc_new_disc(st)?;
                                    tail_append(st, nd.ptr());
                                }
                            }
                        }
                        st.mainloop.cur_q.0 = st.input.cur_list.tail.0;
                        st.mainloop.cur_l = mj.s0 as i32;
                        let ec = crate::fonts::effective_char(
                            &mut st.fonts,
                            true,
                            st.mainloop.main_f,
                            st.mainloop.cur_l as u16,
                        );
                        *main_i = st
                            .fonts
                            .font_info_b16(st.fonts.char_base_entry(st.mainloop.main_f) + ec);
                        st.mainloop.ligature_present = true;
                    }
                    _ => {
                        // Type 0 or 4 (default): =: or |=:|
                        st.mainloop.cur_l = mj.s0 as i32;
                        st.mainloop.ligature_present = true;
                        if st.mainloop.lig_stack.0 == TEX_NULL {
                            return Ok(0); // goto main_loop_wrapup
                        } else {
                            return Ok(2); // goto main_loop_move_1
                        }
                    }
                }

                // Post-switch tail for types 1,2,3,5,6,7,11:
                if mj.s1 > 4 && mj.s1 != 7 {
                    return Ok(0); // types 5,6,11 → wrapup
                }
                // Types 1,2,3,7: re-enter lig loop
                if st.mainloop.cur_l < TOO_BIG_CHAR {
                    // re-enter main_lig_loop (outer): signal caller via enter_lig1_k = None
                    // The outer loop will re-enter at top (checking LIG_TAG)
                    // We need to break out and let caller restart 'char_proc at lig_loop top
                    return Ok(0xFF); // sentinel: re-enter lig_loop (not wrapup!)
                }
                // cur_l == TOO_BIG_CHAR: re-enter at lig_loop_1 with bchar_label
                mk = st.fonts.bchar_label_entry(st.mainloop.main_f);
                // continue inner loop (main_lig_loop_1)
            }
            // mj.s3 > 128: this entry's match byte > 128 means we skip (it's a stop/redirect)
            // but we already handled the redirect at the top. If we reach here with s3 > 128
            // in the inner loop, it means no match → go to wrapup.
            // Actually: if s3 > 128 when s2 == cur_r, that would be a redirect, but redirects
            // are only in the first entry and we've already computed the real index.
            // So s3 > 128 here when s2 == cur_r can't happen after the first entry redirect.
            // The s3 > 128 at line 7722 means "first entry has large byte → use redirect".
            // After redirect, s3 values in subsequent entries are normal (0-128).
        } else {
            // No match for this lig/kern table entry: advance
            if mj.s3 == 0 {
                mk += 1;
            } else if mj.s3 >= 128 {
                return Ok(0); // no more entries → wrapup
            } else {
                mk += mj.s3 as i32 + 1;
            }
            // continue inner loop (main_lig_loop_1)
        }
    }
}
// Lookahead: get next token and set up lig_stack for it.
// Returns false if an inter-char token was injected (caller should return true = big_switch).
// On normal return, cur_r and lig_stack are set for the next lig/kern iteration.
#[inline(always)]
unsafe fn do_lookahead(
    st: &mut EngineState,
    _main_i: &mut crate::mem_types::B16x4,
    _enter_lig1_k: &mut Option<i32>,
    _mf: i32,
) -> Result<bool, MainControlAbort> {
    crate::scan::get_next_scc(st)?;

    let is_char_cmd = |cmd: u8| cmd == LETTER || cmd == OTHER_CHAR || cmd == CHAR_GIVEN;

    if !is_char_cmd(st.scanner.cur_cmd) {
        crate::scan::x_token_in(st)?;
    }
    if !is_char_cmd(st.scanner.cur_cmd) {
        if st.scanner.cur_cmd == CHAR_NUM {
            crate::scan::scan_char_num_in(st)?;
            st.scanner.cur_chr = st.scanner.cur_val;
            // fall through to is_char_cmd path
        } else {
            // Not a char: set cur_r = bchar, lig_stack = NULL
            if st.scanner.cur_cmd == NO_BOUNDARY {
                st.mainloop.bchar = TOO_BIG_CHAR;
            }
            st.mainloop.cur_r = st.mainloop.bchar;
            st.mainloop.lig_stack.0 = TEX_NULL;
            return Ok(true); // normal: re-enter lig loop at top
        }
    }

    // main_loop_lookahead_1: we have a char in cur_chr
    let sf = st.eqtb.b32s1(SF_CODE_BASE + st.scanner.cur_chr);
    let ms = sf % 65536;
    if ms == 1000 {
        st.input.cur_list.set_aux_s0(1000);
    } else if ms < 1000 {
        if ms > 0 {
            st.input.cur_list.set_aux_s0(ms);
        }
    } else if st.input.cur_list.aux_s0() < 1000 {
        st.input.cur_list.set_aux_s0(1000);
    } else {
        st.input.cur_list.set_aux_s0(ms);
    }
    st.scanner.cur_ptr.0 = TEX_NULL;
    st.mainloop.space_class = sf / 65536;
    let sc = st.mainloop.space_class;

    // Inter-char check (lookahead_1 variant)
    let was_boundary = st.mainloop.prev_class == CHAR_CLASS_LIMIT - 1;
    if inter_char_before_current_char_action(st, sc, LETTER, OTHER_CHAR).execute_in(st)? {
        if !was_boundary {
            st.mainloop.prev_class = CHAR_CLASS_LIMIT - 1;
        }
        return Ok(false); // inter-char injected → caller returns true (big_switch)
    }
    if st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_INTER_CHAR_TOKENS) > 0 && sc != CHAR_CLASS_LIMIT {
        st.mainloop.prev_class = sc;
    }

    // Allocate lig_stack for the new char
    let ls = mc_get_avail(st)?;
    st.mainloop.lig_stack = ls;
    st.mem.set_b16s1(ls, st.mainloop.main_f as u16);
    let cr = st.scanner.cur_chr;
    st.mainloop.cur_r = cr;
    st.mem.set_b16s0(ls, cr as u16);
    if cr == st.mainloop.false_bchar {
        st.mainloop.cur_r = TOO_BIG_CHAR;
    }

    Ok(true) // normal lookahead complete, continue lig loop
}
// Top-level TeX command dispatcher: big_switch / reswitch loop.
unsafe fn main_control_from_in(
    st: &mut EngineState,
    run_every_job: bool,
    mut resume_pending_command: bool,
) -> Result<(), MainControlAbort> {
    if run_every_job {
        begin_every(st, LOCAL_EVERY_JOB, EVERY_JOB_TEXT)?;
    }

    // Cancellation and fence hooks are live only for resident sessions; the
    // batch path must not pay per-command bookkeeping for them.
    let resident = st.config.execution_mode == crate::state::EngineExecutionMode::Resident;
    loop {
        if resident && st.cancellation.at_command_boundary() {
            return Err(MainControlAbort::Cancelled);
        }
        if resume_pending_command {
            resume_pending_command = false;
        } else {
            if resident {
                crate::session::capture_prefetch_fence(st);
            }
            crate::scan::get_x_token_in(st)?;
        }
        if resident && crate::session::capture_command_fence(st) {
            return Err(MainControlAbort::Horizoned);
        }

        'reswitch: loop {
            if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_COMMANDS) > 0 {
                crate::output::show_cur_cmd_chr_in(st);
            }

            let mode = st.input.cur_list.mode.abs();
            let cmd = st.scanner.cur_cmd;

            // Fast char path and boundary check (first switch in C, before inter-char check)
            match (mode, cmd) {
                (HMODE, LETTER) | (HMODE, OTHER_CHAR) | (HMODE, CHAR_GIVEN) => {
                    if main_control_main_loop_in(st)? {
                        break 'reswitch;
                    }
                    continue 'reswitch;
                }
                (HMODE, CHAR_NUM) => {
                    crate::scan::scan_usv_num_in(st)?;
                    st.scanner.cur_chr = st.scanner.cur_val;
                    if main_control_main_loop_in(st)? {
                        break 'reswitch;
                    }
                    continue 'reswitch;
                }
                (HMODE, NO_BOUNDARY) => {
                    crate::scan::get_x_token_in(st)?;
                    if matches!(
                        st.scanner.cur_cmd,
                        LETTER | OTHER_CHAR | CHAR_GIVEN | CHAR_NUM
                    ) {
                        st.mainloop.cancel_boundary = true;
                    }
                    continue 'reswitch;
                }
                _ => {}
            }

            if main_control_hmode_inter_char_check_in(st)? {
                break 'reswitch; // → big_switch
            }

            // Main command dispatch (second switch in C)
            match (mode, cmd) {
                // Spacer/ex_space → spacer_cases
                (HMODE, SPACER) | (HMODE, EX_SPACE) | (MMODE, EX_SPACE) => {
                    main_control_spacer_cases_in(st)?;
                }
                // No-op: ANY_MODE(RELAX), VMODE+SPACER, MMODE+SPACER, MMODE+NO_BOUNDARY
                (HMODE, RELAX)
                | (VMODE, RELAX)
                | (MMODE, RELAX)
                | (VMODE, SPACER)
                | (MMODE, SPACER)
                | (MMODE, NO_BOUNDARY) => {}
                // ANY_MODE(IGNORE_SPACES) → call helper then reswitch
                (HMODE, IGNORE_SPACES) | (VMODE, IGNORE_SPACES) | (MMODE, IGNORE_SPACES) => {
                    main_control_ignore_spaces_in(st)?;
                    continue 'reswitch;
                }
                // VMODE+STOP → "the only way out"
                (VMODE, STOP) => {
                    if its_all_over_in(st)? {
                        return Ok(());
                    }
                }
                // Forbidden / math-mode-mismatch error cases
                (VMODE, VMOVE)
                | (HMODE, HMOVE)
                | (MMODE, HMOVE)
                | (VMODE, LAST_ITEM)
                | (HMODE, LAST_ITEM)
                | (MMODE, LAST_ITEM)
                | (VMODE, VADJUST)
                | (VMODE, ITAL_CORR)
                | (VMODE, EQ_NO)
                | (HMODE, EQ_NO)
                | (VMODE, MAC_PARAM)
                | (HMODE, MAC_PARAM)
                | (MMODE, MAC_PARAM)
                | (VMODE, SUP_MARK)
                | (HMODE, SUP_MARK)
                | (VMODE, SUB_MARK)
                | (HMODE, SUB_MARK)
                | (VMODE, MATH_CHAR_NUM)
                | (HMODE, MATH_CHAR_NUM)
                | (VMODE, MATH_GIVEN)
                | (HMODE, MATH_GIVEN)
                | (VMODE, XETEX_MATH_GIVEN)
                | (HMODE, XETEX_MATH_GIVEN)
                | (VMODE, MATH_COMP)
                | (HMODE, MATH_COMP)
                | (VMODE, DELIM_NUM)
                | (HMODE, DELIM_NUM)
                | (VMODE, LEFT_RIGHT)
                | (HMODE, LEFT_RIGHT)
                | (VMODE, ABOVE)
                | (HMODE, ABOVE)
                | (VMODE, RADICAL)
                | (HMODE, RADICAL)
                | (VMODE, MATH_STYLE)
                | (HMODE, MATH_STYLE)
                | (VMODE, MATH_CHOICE)
                | (HMODE, MATH_CHOICE)
                | (VMODE, VCENTER)
                | (HMODE, VCENTER)
                | (VMODE, NON_SCRIPT)
                | (HMODE, NON_SCRIPT)
                | (VMODE, MKERN)
                | (HMODE, MKERN)
                | (VMODE, LIMIT_SWITCH)
                | (HMODE, LIMIT_SWITCH)
                | (VMODE, MSKIP)
                | (HMODE, MSKIP)
                | (VMODE, MATH_ACCENT)
                | (HMODE, MATH_ACCENT)
                | (MMODE, ENDV)
                | (MMODE, PAR_END)
                | (MMODE, STOP)
                | (MMODE, VSKIP)
                | (MMODE, UN_VBOX)
                | (MMODE, VALIGN)
                | (MMODE, HRULE) => {
                    main_control_mode_error_cases_in(st)?;
                }
                // Rule/skip/kern/group cases
                (VMODE, HRULE)
                | (HMODE, VRULE)
                | (MMODE, VRULE)
                | (VMODE, VSKIP)
                | (HMODE, HSKIP)
                | (MMODE, HSKIP)
                | (MMODE, MSKIP)
                | (VMODE, KERN)
                | (HMODE, KERN)
                | (MMODE, KERN)
                | (MMODE, MKERN)
                | (VMODE, LEFT_BRACE)
                | (HMODE, LEFT_BRACE)
                | (VMODE, BEGIN_GROUP)
                | (HMODE, BEGIN_GROUP)
                | (MMODE, BEGIN_GROUP)
                | (VMODE, END_GROUP)
                | (HMODE, END_GROUP)
                | (MMODE, END_GROUP) => {
                    main_control_simple_list_group_cases_in(st)?;
                }
                // Box/list construction cases
                (VMODE, RIGHT_BRACE)
                | (HMODE, RIGHT_BRACE)
                | (MMODE, RIGHT_BRACE)
                | (VMODE, HMOVE)
                | (HMODE, VMOVE)
                | (MMODE, VMOVE)
                | (VMODE, LEADER_SHIP)
                | (HMODE, LEADER_SHIP)
                | (MMODE, LEADER_SHIP)
                | (VMODE, MAKE_BOX)
                | (HMODE, MAKE_BOX)
                | (MMODE, MAKE_BOX)
                | (VMODE, START_PAR)
                | (VMODE, LETTER)
                | (VMODE, OTHER_CHAR)
                | (VMODE, CHAR_NUM)
                | (VMODE, CHAR_GIVEN)
                | (VMODE, MATH_SHIFT)
                | (VMODE, UN_HBOX)
                | (VMODE, VRULE)
                | (VMODE, ACCENT)
                | (VMODE, DISCRETIONARY)
                | (VMODE, HSKIP)
                | (VMODE, VALIGN)
                | (VMODE, EX_SPACE)
                | (VMODE, NO_BOUNDARY)
                | (HMODE, START_PAR)
                | (MMODE, START_PAR)
                | (VMODE, PAR_END)
                | (HMODE, PAR_END)
                | (HMODE, STOP)
                | (HMODE, VSKIP)
                | (HMODE, HRULE)
                | (HMODE, UN_VBOX)
                | (HMODE, HALIGN)
                | (VMODE, INSERT)
                | (HMODE, INSERT)
                | (MMODE, INSERT)
                | (HMODE, VADJUST)
                | (MMODE, VADJUST)
                | (VMODE, MARK)
                | (HMODE, MARK)
                | (MMODE, MARK)
                | (VMODE, BREAK_PENALTY)
                | (HMODE, BREAK_PENALTY)
                | (MMODE, BREAK_PENALTY)
                | (VMODE, REMOVE_ITEM)
                | (HMODE, REMOVE_ITEM)
                | (MMODE, REMOVE_ITEM)
                | (VMODE, UN_VBOX)
                | (HMODE, UN_HBOX)
                | (MMODE, UN_HBOX)
                | (HMODE, ITAL_CORR)
                | (MMODE, ITAL_CORR)
                | (HMODE, DISCRETIONARY)
                | (MMODE, DISCRETIONARY)
                | (HMODE, ACCENT)
                | (VMODE, CAR_RET)
                | (HMODE, CAR_RET)
                | (MMODE, CAR_RET)
                | (VMODE, TAB_MARK)
                | (HMODE, TAB_MARK)
                | (MMODE, TAB_MARK) => {
                    main_control_box_list_cases_in(st)?;
                }
                // Alignment / endcsname cases
                (VMODE, NO_ALIGN)
                | (HMODE, NO_ALIGN)
                | (MMODE, NO_ALIGN)
                | (VMODE, OMIT)
                | (HMODE, OMIT)
                | (MMODE, OMIT)
                | (VMODE, HALIGN)
                | (HMODE, VALIGN)
                | (MMODE, HALIGN)
                | (VMODE, ENDV)
                | (HMODE, ENDV)
                | (VMODE, END_CS_NAME)
                | (HMODE, END_CS_NAME)
                | (MMODE, END_CS_NAME) => {
                    main_control_alignment_cases_in(st)?;
                }
                // Math entry/exit cases
                (HMODE, MATH_SHIFT)
                | (MMODE, EQ_NO)
                | (MMODE, LEFT_BRACE)
                | (MMODE, LETTER)
                | (MMODE, OTHER_CHAR)
                | (MMODE, CHAR_GIVEN)
                | (MMODE, CHAR_NUM)
                | (MMODE, MATH_CHAR_NUM)
                | (MMODE, MATH_GIVEN)
                | (MMODE, XETEX_MATH_GIVEN)
                | (MMODE, DELIM_NUM)
                | (MMODE, MATH_COMP)
                | (MMODE, LIMIT_SWITCH)
                | (MMODE, RADICAL)
                | (MMODE, ACCENT)
                | (MMODE, MATH_ACCENT)
                | (MMODE, VCENTER)
                | (MMODE, MATH_STYLE)
                | (MMODE, NON_SCRIPT)
                | (MMODE, MATH_CHOICE)
                | (MMODE, SUB_MARK)
                | (MMODE, SUP_MARK)
                | (MMODE, ABOVE)
                | (MMODE, LEFT_RIGHT)
                | (MMODE, MATH_SHIFT) => {
                    main_control_math_cases_in(st)?;
                }
                // Mode-independent cases (ANY_MODE): assignments, defs, etc.
                _ if matches!(
                    cmd,
                    TOKS_REGISTER
                        | ASSIGN_TOKS
                        | ASSIGN_INT
                        | ASSIGN_DIMEN
                        | ASSIGN_GLUE
                        | ASSIGN_MU_GLUE
                        | ASSIGN_FONT_DIMEN
                        | ASSIGN_FONT_INT
                        | SET_AUX
                        | SET_PREV_GRAF
                        | SET_PAGE_DIMEN
                        | SET_PAGE_INT
                        | SET_BOX_DIMEN
                        | SET_SHAPE
                        | DEF_CODE
                        | XETEX_DEF_CODE
                        | DEF_FAMILY
                        | SET_FONT
                        | DEF_FONT
                        | REGISTER
                        | ADVANCE
                        | MULTIPLY
                        | DIVIDE
                        | PREFIX
                        | LET
                        | SHORTHAND_DEF
                        | READ_TO_CS
                        | DEF
                        | SET_BOX
                        | HYPH_DATA
                        | SET_INTERACTION
                        | AFTER_ASSIGNMENT
                        | AFTER_GROUP
                        | IN_STREAM
                        | MESSAGE
                        | CASE_SHIFT
                        | XRAY
                        | EXTENSION
                ) =>
                {
                    main_control_mode_independent_in(st)?;
                }
                // Default: unrecognized / already handled → goto big_switch
                _ => {}
            }

            break 'reswitch; // → big_switch (get_x_token at top of loop)
        }
    }
}

/// Dispatch a fresh document, including `\everyjob` and the first token fetch.
pub(crate) unsafe fn main_control_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    main_control_from_in(st, true, false)
}

/// Resume at a restored command boundary, optionally with a command already fetched.
pub(crate) unsafe fn main_control_resume_in(
    st: &mut EngineState,
    resume_pending_command: bool,
) -> Result<(), MainControlAbort> {
    main_control_from_in(st, false, resume_pending_command)
}

fn define(
    st: &mut EngineState,
    global: bool,
    p: i32,
    t: u16,
    e: i32,
) -> Result<(), crate::save::SaveAbort> {
    if global {
        crate::save::geq_define(&mut st.eqtb, &mut st.mem, &mut st.scanner, p, t, e)
    } else {
        crate::save::eq_define(
            &mut st.eqtb,
            &mut st.save_stack,
            &mut st.mem,
            &mut st.scanner,
            p,
            t,
            e,
        )
    }
}

fn word_define(
    st: &mut EngineState,
    global: bool,
    p: i32,
    w: i32,
) -> Result<(), crate::save::SaveAbort> {
    if global {
        crate::save::geq_word_define(&mut st.eqtb, p, w);
        Ok(())
    } else {
        crate::save::eq_word_define(&mut st.eqtb, &mut st.save_stack, p, w)
    }
}

fn sa_define(st: &mut EngineState, global: bool, p: i32, e: i32) -> Result<(), MainControlAbort> {
    let result = if global {
        crate::save::gsa_def(&mut st.mem, p, e)
    } else {
        crate::save::sa_def(&mut st.mem, &mut st.scanner, &mut st.save_stack, p, e)?
    };
    crate::save::finish_sa_mutation_in(st, p, result)?;
    Ok(())
}

fn sa_word_define(
    st: &mut EngineState,
    global: bool,
    p: i32,
    w: i32,
) -> Result<(), MainControlAbort> {
    let result = if global {
        crate::save::gsa_w_def(&mut st.mem, p, w)
    } else {
        crate::save::sa_w_def(&mut st.mem, &mut st.scanner, &mut st.save_stack, p, w)?
    };
    crate::save::finish_sa_mutation_in(st, p, result)?;
    Ok(())
}

pub(crate) fn prefixed_command_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    fn set_class(x: i32) -> i32 {
        (((x as u32) & 0x07) << 21) as i32
    }
    fn set_family(x: i32) -> i32 {
        (((x as u32) & 0xFF) << 24) as i32
    }

    let mut a: i32 = 0;
    let f: i32;
    let mut j: i32;
    let k: i32;
    let mut p: i32;
    let mut q: i32;
    let mut n: i32;
    let mut e: bool;

    while st.scanner.cur_cmd == PREFIX {
        if (a / st.scanner.cur_chr) & 1 == 0 {
            a += st.scanner.cur_chr;
        }

        loop {
            crate::scan::get_x_token_in(st)?;
            if !(st.scanner.cur_cmd == SPACER || st.scanner.cur_cmd == RELAX) {
                break;
            }
        }

        if (st.scanner.cur_cmd) <= MAX_NON_PREFIXED_COMMAND {
            crate::output::error_here_with_diagnostic_in(
                st,
                c"You can't use a prefix with `".as_ptr(),
            );
            crate::output::print_cmd_chr_in(st, st.scanner.cur_cmd as u16, st.scanner.cur_chr);
            crate::output::print_char_in(st, '\'' as i32);
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            help(
                &mut st.errors,
                &[c"I'll pretend you didn't say \\long or \\outer or \\global or \\protected."],
            );
            let outcome = crate::input::back_error_in(st)?;
            mc_propagate_error(outcome)?;
            return Ok(());
        }

        if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_COMMANDS) > 2 {
            crate::output::show_cur_cmd_chr_in(st);
        }
    }

    if a >= 8 {
        j = PROTECTED_TOKEN;
        a -= 8;
    } else {
        j = 0;
    }

    if st.scanner.cur_cmd != DEF && (a % 4 != 0 || j != 0) {
        crate::output::error_here_with_diagnostic_in(st, c"You can't use `".as_ptr());
        crate::output::print_esc_cstr_in(st, c"long".as_ptr());
        crate::output::print_cstr_in(st, c"' or `".as_ptr());
        crate::output::print_esc_cstr_in(st, c"outer".as_ptr());
        crate::output::print_cstr_in(st, c"' or `".as_ptr());
        crate::output::print_esc_cstr_in(st, c"protected".as_ptr());
        crate::output::print_cstr_in(st, c"' with `".as_ptr());
        crate::output::print_cmd_chr_in(st, st.scanner.cur_cmd as u16, st.scanner.cur_chr);
        crate::output::print_char_in(st, '\'' as i32);
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        help(
            &mut st.errors,
            &[c"I'll pretend you didn't say \\long or \\outer or \\protected here."],
        );
        mc_propagate_error(crate::errors::error_in(st))?;
    }

    if st.eqtb.b32s1(INT_BASE + INT_PAR_GLOBAL_DEFS) != 0 {
        if st.eqtb.b32s1(INT_BASE + INT_PAR_GLOBAL_DEFS) < 0 {
            if a >= 4 {
                a -= 4;
            }
        } else if a < 4 {
            a += 4;
        }
    }

    'sw: {
        match st.scanner.cur_cmd {
            SET_FONT => {
                define(st, a >= 4, CUR_FONT_LOC, DATA as u16, st.scanner.cur_chr)?;
            }

            DEF => {
                if st.scanner.cur_chr & 1 != 0
                    && a < 4
                    && st.eqtb.b32s1(INT_BASE + INT_PAR_GLOBAL_DEFS) >= 0
                {
                    a += 4;
                }

                e = st.scanner.cur_chr >= 2;
                crate::scan::get_r_token_in(st)?;
                p = st.scanner.cur_cs;
                crate::scan::scan_toks_in(st, true, e)?;

                if j != 0 {
                    q = crate::mem::get_avail(&mut st.mem)
                        .ok_or(crate::scan::NodeRecycleAbort::Overflow)?
                        .0;
                    st.mem.set_b32s0(q, j);
                    st.mem.set_b32s1(q, st.mem.b32s1(st.scanner.def_ref));
                    st.mem.set_b32s1(st.scanner.def_ref, q);
                }

                define(st, a >= 4, p, CALL + (a % 4) as u16, st.scanner.def_ref.0)?;
            }

            LET => {
                n = st.scanner.cur_chr;
                crate::scan::get_r_token_in(st)?;
                p = st.scanner.cur_cs;

                if n == NORMAL as i32 {
                    loop {
                        crate::scan::get_token_in(st)?;
                        if st.scanner.cur_cmd != SPACER {
                            break;
                        }
                    }

                    if st.scanner.cur_tok == OTHER_TOKEN + '=' as i32 {
                        crate::scan::get_token_in(st)?;
                        if st.scanner.cur_cmd == SPACER {
                            crate::scan::get_token_in(st)?;
                        }
                    }
                } else {
                    crate::scan::get_token_in(st)?;
                    q = st.scanner.cur_tok;
                    crate::scan::get_token_in(st)?;
                    crate::input::back_input_in(st)?;
                    st.scanner.cur_tok = q;
                    crate::input::back_input_in(st)?;
                }

                if st.scanner.cur_cmd as u16 >= CALL {
                    st.mem
                        .set_b32s0(st.scanner.cur_chr, st.mem.b32s0(st.scanner.cur_chr) + 1);
                } else if (st.scanner.cur_cmd == REGISTER || st.scanner.cur_cmd == TOKS_REGISTER)
                    && (st.scanner.cur_chr < 0 || st.scanner.cur_chr > 19)
                {
                    st.mem.set_b32s0(
                        st.scanner.cur_chr + 1,
                        st.mem.b32s0(st.scanner.cur_chr + 1) + 1,
                    );
                }

                define(st, a >= 4, p, st.scanner.cur_cmd as u16, st.scanner.cur_chr)?;
            }

            SHORTHAND_DEF => {
                if st.scanner.cur_chr == CHAR_SUB_DEF_CODE {
                    crate::output::error_here_with_diagnostic_in(
                        st,
                        c"vestigial MLTeX shorthand encountered??".as_ptr(),
                    );
                    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                    help(
                        &mut st.errors,
                        &[c"This should never happen in Tectonic, where MLTeX has been excised."],
                    );
                    mc_propagate_error(crate::errors::error_in(st))?;
                } else {
                    n = st.scanner.cur_chr;
                    crate::scan::get_r_token_in(st)?;
                    p = st.scanner.cur_cs;

                    define(st, a >= 4, p, RELAX as u16, TOO_BIG_USV)?;

                    crate::scan::scan_optional_equals_in(st)?;

                    match n {
                        CHAR_DEF_CODE => {
                            crate::scan::scan_usv_num_in(st)?;
                            define(st, a >= 4, p, CHAR_GIVEN as u16, st.scanner.cur_val)?;
                        }
                        MATH_CHAR_DEF_CODE => {
                            crate::scan::scan_fifteen_bit_int_in(st)?;
                            define(st, a >= 4, p, MATH_GIVEN as u16, st.scanner.cur_val)?;
                        }
                        XETEX_MATH_CHAR_NUM_DEF_CODE => {
                            crate::scan::scan_xetex_math_char_int_in(st)?;
                            define(st, a >= 4, p, XETEX_MATH_GIVEN as u16, st.scanner.cur_val)?;
                        }
                        XETEX_MATH_CHAR_DEF_CODE => {
                            crate::scan::scan_math_class_int_in(st)?;
                            n = set_class(st.scanner.cur_val);
                            crate::scan::scan_math_fam_int_in(st)?;
                            n = n.wrapping_add(set_family(st.scanner.cur_val));
                            crate::scan::scan_usv_num_in(st)?;
                            n = n.wrapping_add(st.scanner.cur_val);
                            define(st, a >= 4, p, XETEX_MATH_GIVEN as u16, n)?;
                        }
                        _ => {
                            crate::scan::scan_register_num_in(st)?;

                            if st.scanner.cur_val > 255 {
                                j = n - 2;

                                if j > MU_VAL as i32 {
                                    j = TOK_VAL as i32;
                                }

                                let __cv7 = st.scanner.cur_val;
                                crate::save::find_sa_element(
                                    &mut st.mem,
                                    &mut st.scanner,
                                    j as u16,
                                    __cv7,
                                    true,
                                )
                                .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
                                st.mem.set_b32s0(
                                    st.scanner.cur_ptr.0 + 1,
                                    st.mem.b32s0(st.scanner.cur_ptr + 1) + 1,
                                );

                                if j == TOK_VAL as i32 {
                                    j = TOKS_REGISTER as i32;
                                } else {
                                    j = REGISTER as i32;
                                }

                                define(st, a >= 4, p, j as u16, st.scanner.cur_ptr.0)?;
                            } else {
                                match n {
                                    COUNT_DEF_CODE => {
                                        define(
                                            st,
                                            a >= 4,
                                            p,
                                            ASSIGN_INT as u16,
                                            COUNT_BASE + st.scanner.cur_val,
                                        )?;
                                    }
                                    DIMEN_DEF_CODE => {
                                        define(
                                            st,
                                            a >= 4,
                                            p,
                                            ASSIGN_DIMEN as u16,
                                            SCALED_BASE + st.scanner.cur_val,
                                        )?;
                                    }
                                    SKIP_DEF_CODE => {
                                        define(
                                            st,
                                            a >= 4,
                                            p,
                                            ASSIGN_GLUE as u16,
                                            SKIP_BASE + st.scanner.cur_val,
                                        )?;
                                    }
                                    MU_SKIP_DEF_CODE => {
                                        define(
                                            st,
                                            a >= 4,
                                            p,
                                            ASSIGN_MU_GLUE as u16,
                                            MU_SKIP_BASE + st.scanner.cur_val,
                                        )?;
                                    }
                                    TOKS_DEF_CODE => {
                                        define(
                                            st,
                                            a >= 4,
                                            p,
                                            ASSIGN_TOKS as u16,
                                            TOKS_BASE + st.scanner.cur_val,
                                        )?;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }

            READ_TO_CS => {
                j = st.scanner.cur_chr;
                crate::scan::scan_int_in(st)?;
                n = st.scanner.cur_val;
                if !crate::scan::scan_keyword_in(st, c"to".as_ptr())? {
                    crate::output::error_here_with_diagnostic_in(
                        st,
                        c"Missing `to' inserted".as_ptr(),
                    );
                    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                    help(
                        &mut st.errors,
                        &[
                            c"You should have said `\\read<number> to \\cs'.",
                            c"I'm going to look for the \\cs now.",
                        ],
                    );
                    mc_propagate_error(crate::errors::error_in(st))?;
                }

                crate::scan::get_r_token_in(st)?;
                p = st.scanner.cur_cs;
                crate::scan::read_toks_in(st, n, p, j)?;

                define(st, a >= 4, p, CALL as u16, st.scanner.cur_val)?;
            }

            TOKS_REGISTER | ASSIGN_TOKS => {
                q = st.scanner.cur_cs;
                e = false;

                if st.scanner.cur_cmd == TOKS_REGISTER {
                    if st.scanner.cur_chr == 0 {
                        crate::scan::scan_register_num_in(st)?;
                        if st.scanner.cur_val > 255 {
                            let __cv6 = st.scanner.cur_val;
                            crate::save::find_sa_element(
                                &mut st.mem,
                                &mut st.scanner,
                                TOK_VAL as u16,
                                __cv6,
                                true,
                            )
                            .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
                            st.scanner.cur_chr = st.scanner.cur_ptr.0;
                            e = true;
                        } else {
                            st.scanner.cur_chr = TOKS_BASE + st.scanner.cur_val;
                        }
                    } else {
                        e = true;
                    }
                } else if st.scanner.cur_chr == LOCAL_BASE + LOCAL_XETEX_INTER_CHAR_TOKS {
                    crate::scan::scan_char_class_not_ignored_in(st)?;
                    st.scanner.cur_ptr.0 = st.scanner.cur_val;
                    crate::scan::scan_char_class_not_ignored_in(st)?;
                    let __cv5 = st.scanner.cur_val;
                    let __cp5 = st.scanner.cur_ptr.0;
                    crate::save::find_sa_element(
                        &mut st.mem,
                        &mut st.scanner,
                        INTER_CHAR_VAL as u16,
                        __cp5 * CHAR_CLASS_LIMIT + __cv5,
                        true,
                    )
                    .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
                    st.scanner.cur_chr = st.scanner.cur_ptr.0;
                    e = true;
                }

                p = st.scanner.cur_chr;
                crate::scan::scan_optional_equals_in(st)?;

                loop {
                    crate::scan::get_x_token_in(st)?;
                    if !(st.scanner.cur_cmd == SPACER || st.scanner.cur_cmd == RELAX) {
                        break;
                    }
                }

                if st.scanner.cur_cmd != LEFT_BRACE
                    && (st.scanner.cur_cmd == TOKS_REGISTER || st.scanner.cur_cmd == ASSIGN_TOKS)
                {
                    if st.scanner.cur_cmd == TOKS_REGISTER {
                        if st.scanner.cur_chr == 0 {
                            crate::scan::scan_register_num_in(st)?;
                            if st.scanner.cur_val < 256 {
                                q = st.eqtb.b32s1(TOKS_BASE + st.scanner.cur_val);
                            } else {
                                let __cv4 = st.scanner.cur_val;
                                crate::save::find_sa_element(
                                    &mut st.mem,
                                    &mut st.scanner,
                                    TOK_VAL as u16,
                                    __cv4,
                                    false,
                                );
                                if st.scanner.cur_ptr.0 == TEX_NULL {
                                    q = TEX_NULL;
                                } else {
                                    q = st.mem.b32s1(st.scanner.cur_ptr + 1);
                                }
                            }
                        } else {
                            q = st.mem.b32s1(st.scanner.cur_chr + 1);
                        }
                    } else if st.scanner.cur_chr == LOCAL_BASE + LOCAL_XETEX_INTER_CHAR_TOKS {
                        crate::scan::scan_char_class_not_ignored_in(st)?;
                        st.scanner.cur_ptr.0 = st.scanner.cur_val;
                        crate::scan::scan_char_class_not_ignored_in(st)?;
                        let __cv3 = st.scanner.cur_val;
                        let __cp3 = st.scanner.cur_ptr.0;
                        crate::save::find_sa_element(
                            &mut st.mem,
                            &mut st.scanner,
                            INTER_CHAR_VAL as u16,
                            __cp3 * CHAR_CLASS_LIMIT + __cv3,
                            false,
                        );
                        if st.scanner.cur_ptr.0 == TEX_NULL {
                            q = TEX_NULL;
                        } else {
                            q = st.mem.b32s1(st.scanner.cur_ptr + 1);
                        }
                    } else {
                        q = st.eqtb.b32s1(st.scanner.cur_chr);
                    }

                    if q == TEX_NULL {
                        if e {
                            sa_define(st, a >= 4, p, TEX_NULL)?;
                        } else if a >= 4 {
                            define(st, true, p, UNDEFINED_CS as u16, TEX_NULL)?;
                        } else {
                            define(st, false, p, UNDEFINED_CS as u16, TEX_NULL)?;
                        }
                    } else {
                        st.mem.set_b32s0(q, st.mem.b32s0(q) + 1);
                        if e {
                            sa_define(st, a >= 4, p, q)?;
                        } else if a >= 4 {
                            define(st, true, p, CALL as u16, q)?;
                        } else {
                            define(st, false, p, CALL as u16, q)?;
                        }
                    }

                    break 'sw;
                }

                crate::input::back_input_in(st)?;
                st.scanner.cur_cs = q;
                q = crate::scan::scan_toks_in(st, false, false)?;

                if st.mem.b32s1(st.scanner.def_ref) == TEX_NULL {
                    if e {
                        sa_define(st, a >= 4, p, TEX_NULL)?;
                    } else if a >= 4 {
                        define(st, true, p, UNDEFINED_CS as u16, TEX_NULL)?;
                    } else {
                        define(st, false, p, UNDEFINED_CS as u16, TEX_NULL)?;
                    }

                    crate::mem::free_avail(&mut st.mem, st.scanner.def_ref);
                } else {
                    if p == LOCAL_BASE + LOCAL_OUTPUT_ROUTINE && !e {
                        let new_avail = crate::mem::get_avail(&mut st.mem)
                            .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
                        st.mem.set_b32s1(q, new_avail.0);
                        q = new_avail.0;
                        st.mem.set_b32s0(q, RIGHT_BRACE_TOKEN + 125);
                        q = crate::mem::get_avail(&mut st.mem)
                            .ok_or(crate::scan::NodeRecycleAbort::Overflow)?
                            .0;
                        st.mem.set_b32s0(q, LEFT_BRACE_TOKEN + 123);
                        st.mem.set_b32s1(q, st.mem.b32s1(st.scanner.def_ref));
                        st.mem.set_b32s1(st.scanner.def_ref, q);
                    }

                    if e {
                        sa_define(st, a >= 4, p, st.scanner.def_ref.0)?;
                    } else if a >= 4 {
                        define(st, true, p, CALL as u16, st.scanner.def_ref.0)?;
                    } else {
                        define(st, false, p, CALL as u16, st.scanner.def_ref.0)?;
                    }
                }
            }

            ASSIGN_INT => {
                p = st.scanner.cur_chr;
                crate::scan::scan_optional_equals_in(st)?;
                crate::scan::scan_int_in(st)?;
                word_define(st, a >= 4, p, st.scanner.cur_val)?;
            }

            ASSIGN_DIMEN => {
                p = st.scanner.cur_chr;
                crate::scan::scan_optional_equals_in(st)?;
                crate::scan::scan_dimen_in(st, false, false, false)?;
                word_define(st, a >= 4, p, st.scanner.cur_val)?;
            }

            ASSIGN_GLUE | ASSIGN_MU_GLUE => {
                p = st.scanner.cur_chr;
                let is_mu = st.scanner.cur_cmd == ASSIGN_MU_GLUE;
                crate::scan::scan_optional_equals_in(st)?;
                if is_mu {
                    crate::scan::scan_glue_in(st, MU_VAL as i16)?;
                } else {
                    crate::scan::scan_glue_in(st, GLUE_VAL as i16)?;
                }
                crate::main_control::trap_zero_glue(st);
                define(st, a >= 4, p, GLUE_REF as u16, st.scanner.cur_val)?;
            }

            XETEX_DEF_CODE => {
                if st.scanner.cur_chr == SF_CODE_BASE {
                    p = st.scanner.cur_chr;
                    crate::scan::scan_usv_num_in(st)?;
                    p += st.scanner.cur_val;
                    n = st.eqtb.b32s1(SF_CODE_BASE + st.scanner.cur_val) % 65536;
                    crate::scan::scan_optional_equals_in(st)?;
                    crate::scan::scan_char_class_in(st)?;
                    let v = st.scanner.cur_val.wrapping_mul(65536).wrapping_add(n);
                    define(st, a >= 4, p, DATA as u16, v)?;
                } else if st.scanner.cur_chr == MATH_CODE_BASE {
                    p = st.scanner.cur_chr;
                    crate::scan::scan_usv_num_in(st)?;
                    p += st.scanner.cur_val;
                    crate::scan::scan_optional_equals_in(st)?;
                    crate::scan::scan_xetex_math_char_int_in(st)?;
                    define(st, a >= 4, p, DATA as u16, st.scanner.cur_val)?;
                } else if st.scanner.cur_chr == MATH_CODE_BASE + 1 {
                    p = st.scanner.cur_chr - 1;
                    crate::scan::scan_usv_num_in(st)?;
                    p += st.scanner.cur_val;
                    crate::scan::scan_optional_equals_in(st)?;
                    crate::scan::scan_math_class_int_in(st)?;
                    n = set_class(st.scanner.cur_val);
                    crate::scan::scan_math_fam_int_in(st)?;
                    n = n.wrapping_add(set_family(st.scanner.cur_val));
                    crate::scan::scan_usv_num_in(st)?;
                    n = n.wrapping_add(st.scanner.cur_val);
                    define(st, a >= 4, p, DATA as u16, n)?;
                } else if st.scanner.cur_chr == DEL_CODE_BASE {
                    p = st.scanner.cur_chr;
                    crate::scan::scan_usv_num_in(st)?;
                    p += st.scanner.cur_val;
                    crate::scan::scan_optional_equals_in(st)?;
                    crate::scan::scan_int_in(st)?;
                    word_define(st, a >= 4, p, st.scanner.cur_val)?;
                } else {
                    p = st.scanner.cur_chr - 1;
                    crate::scan::scan_usv_num_in(st)?;
                    p += st.scanner.cur_val;
                    crate::scan::scan_optional_equals_in(st)?;
                    n = 0x4000_0000; // "extended delimiter code flag"
                    crate::scan::scan_math_fam_int_in(st)?;
                    n = n.wrapping_add(st.scanner.cur_val.wrapping_mul(0x20_0000)); // extended delimiter family
                    crate::scan::scan_usv_num_in(st)?;
                    n = n.wrapping_add(st.scanner.cur_val);
                    word_define(st, a >= 4, p, n)?;
                }
            }

            DEF_CODE => {
                if st.scanner.cur_chr == CAT_CODE_BASE {
                    n = MAX_CHAR_CODE;
                } else if st.scanner.cur_chr == MATH_CODE_BASE {
                    n = 0x8000;
                } else if st.scanner.cur_chr == SF_CODE_BASE {
                    n = 0x7FFF;
                } else if st.scanner.cur_chr == DEL_CODE_BASE {
                    n = 0xFFFFFF;
                } else {
                    n = BIGGEST_USV;
                }

                p = st.scanner.cur_chr;
                crate::scan::scan_usv_num_in(st)?;
                p += st.scanner.cur_val;
                crate::scan::scan_optional_equals_in(st)?;
                crate::scan::scan_int_in(st)?;

                if (st.scanner.cur_val < 0 && p < DEL_CODE_BASE) || st.scanner.cur_val > n {
                    crate::output::error_here_with_diagnostic_in(st, c"Invalid code (".as_ptr());
                    crate::output::print_int_in(st, st.scanner.cur_val);
                    if p < DEL_CODE_BASE {
                        crate::output::print_cstr_in(st, c"), should be in the range 0..".as_ptr());
                    } else {
                        crate::output::print_cstr_in(st, c"), should be at most ".as_ptr());
                    }
                    crate::output::print_int_in(st, n);
                    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                    help(
                        &mut st.errors,
                        &[c"I'm going to use 0 instead of that illegal code value."],
                    );
                    mc_propagate_error(crate::errors::error_in(st))?;
                    st.scanner.cur_val = 0;
                }

                if p < MATH_CODE_BASE {
                    if p >= SF_CODE_BASE {
                        n = st.eqtb.b32s1(p) / 65536;
                        let v = n.wrapping_mul(65536).wrapping_add(st.scanner.cur_val);
                        define(st, a >= 4, p, DATA as u16, v)?;
                    } else {
                        define(st, a >= 4, p, DATA as u16, st.scanner.cur_val)?;
                    }
                } else if p < DEL_CODE_BASE {
                    if st.scanner.cur_val == 32768 {
                        st.scanner.cur_val = ACTIVE_MATH_CHAR;
                    } else {
                        let v = set_class(st.scanner.cur_val / 4096)
                            .wrapping_add(set_family((st.scanner.cur_val % 4096) / 256))
                            .wrapping_add(st.scanner.cur_val % 256);
                        st.scanner.cur_val = v;
                    }

                    define(st, a >= 4, p, DATA as u16, st.scanner.cur_val)?;
                } else {
                    word_define(st, a >= 4, p, st.scanner.cur_val)?;
                }
            }

            DEF_FAMILY => {
                p = st.scanner.cur_chr;
                crate::scan::scan_math_fam_int_in(st)?;
                p += st.scanner.cur_val;
                crate::scan::scan_optional_equals_in(st)?;
                crate::scan::scan_font_ident_in(st)?;
                define(st, a >= 4, p, DATA as u16, st.scanner.cur_val)?;
            }

            REGISTER | ADVANCE | MULTIPLY | DIVIDE => {
                crate::main_control::do_register_command_in(st, a as i16)?;
            }

            SET_BOX => {
                crate::scan::scan_register_num_in(st)?;
                if a >= 4 {
                    n = GLOBAL_BOX_FLAG + st.scanner.cur_val;
                } else {
                    n = BOX_FLAG + st.scanner.cur_val;
                }

                crate::scan::scan_optional_equals_in(st)?;

                if st.scanner.set_box_allowed {
                    crate::main_control::scan_box_in(st, n)?;
                } else {
                    crate::output::error_here_with_diagnostic_in(st, c"Improper ".as_ptr());
                    crate::output::print_esc_cstr_in(st, c"setbox".as_ptr());
                    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                    help(
                        &mut st.errors,
                        &[
                            c"Sorry, \\setbox is not allowed after \\halign in a display,",
                            c"or between \\accent and an accented character.",
                        ],
                    );
                    mc_propagate_error(crate::errors::error_in(st))?;
                }
            }

            SET_AUX => {
                crate::main_control::alter_aux_in(st)?;
            }

            SET_PREV_GRAF => {
                crate::main_control::alter_prev_graf_in(st)?;
            }

            SET_PAGE_DIMEN => {
                crate::main_control::alter_page_so_far_in(st)?;
            }

            SET_PAGE_INT => {
                crate::main_control::alter_integer_in(st)?;
            }

            SET_BOX_DIMEN => {
                crate::main_control::alter_box_dimen_in(st)?;
            }

            SET_SHAPE => {
                q = st.scanner.cur_chr;
                crate::scan::scan_optional_equals_in(st)?;
                crate::scan::scan_int_in(st)?;
                n = st.scanner.cur_val;

                if n <= 0 {
                    p = TEX_NULL;
                } else if q > LOCAL_BASE + LOCAL_PAR_SHAPE {
                    n = (st.scanner.cur_val / 2) + 1;
                    p = crate::mem::get_node(&mut st.mem, 2 * n + 1)
                        .ok_or(crate::scan::NodeRecycleAbort::Overflow)?
                        .0;
                    st.mem.set_b32s0(p, n);
                    n = st.scanner.cur_val;
                    st.mem.set_b32s1(p + 1, n);

                    j = p + 2;
                    while j <= p + n + 1 {
                        crate::scan::scan_int_in(st)?;
                        st.mem.set_b32s1(j, st.scanner.cur_val);
                        j += 1;
                    }

                    if n & 1 == 0 {
                        st.mem.set_b32s1(p + n + 2, 0);
                    }
                } else {
                    p = crate::mem::get_node(&mut st.mem, 2 * n + 1)
                        .ok_or(crate::scan::NodeRecycleAbort::Overflow)?
                        .0;
                    st.mem.set_b32s0(p, n);

                    j = 1;
                    while j <= n {
                        crate::scan::scan_dimen_in(st, false, false, false)?;
                        st.mem.set_b32s1(p + 2 * j - 1, st.scanner.cur_val);
                        crate::scan::scan_dimen_in(st, false, false, false)?;
                        st.mem.set_b32s1(p + 2 * j, st.scanner.cur_val);
                        j += 1;
                    }
                }

                define(st, a >= 4, q, SHAPE_REF as u16, p)?;
            }

            HYPH_DATA => {
                if st.scanner.cur_chr == 1 {
                    if st.config.in_initex_mode {
                        new_patterns_in(st)?;
                        break 'sw;
                    }

                    crate::output::error_here_with_diagnostic_in(
                        st,
                        c"Patterns can be loaded only by INITEX".as_ptr(),
                    );
                    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                    help(&mut st.errors, &[]);
                    mc_propagate_error(crate::errors::error_in(st))?;

                    loop {
                        crate::scan::get_token_in(st)?;
                        if st.scanner.cur_cmd == RIGHT_BRACE {
                            break;
                        }
                    }

                    return Ok(());
                } else {
                    new_hyph_exceptions_in(st)?;
                    break 'sw;
                }
            }

            ASSIGN_FONT_DIMEN => {
                crate::scan::find_font_dimen_in(st, true)?;
                k = st.scanner.cur_val;
                crate::scan::scan_optional_equals_in(st)?;
                crate::scan::scan_dimen_in(st, false, false, false)?;
                st.fonts.set_font_info_b32s1(k, st.scanner.cur_val);
            }

            ASSIGN_FONT_INT => {
                n = st.scanner.cur_chr;
                crate::scan::scan_font_ident_in(st)?;
                f = st.scanner.cur_val;

                if n < 2 {
                    crate::scan::scan_optional_equals_in(st)?;
                    crate::scan::scan_int_in(st)?;
                    if n == 0 {
                        st.fonts.set_hyphen_char_entry(f, st.scanner.cur_val);
                    } else {
                        st.fonts.set_skew_char_entry(f, st.scanner.cur_val);
                    }
                } else {
                    if st.fonts.font_area_entry(f) == AAT_FONT_FLAG
                        || st.fonts.font_area_entry(f) == OTGR_FONT_FLAG
                    {
                        crate::scan::scan_glyph_number_in(st, f)?;
                    } else {
                        crate::scan::scan_char_num_in(st)?;
                    }

                    p = st.scanner.cur_val;
                    crate::scan::scan_optional_equals_in(st)?;
                    crate::scan::scan_int_in(st)?;
                    match n {
                        LP_CODE_BASE => unsafe {
                            set_cp_code(f, p as u32, LEFT_SIDE, st.scanner.cur_val)
                        },
                        RP_CODE_BASE => unsafe {
                            set_cp_code(f, p as u32, RIGHT_SIDE, st.scanner.cur_val)
                        },
                        _ => {}
                    }
                }
            }

            DEF_FONT => {
                unsafe { crate::main_control::new_font_in(st, a as i16)? };
            }

            SET_INTERACTION => {
                crate::main_control::new_interaction_in(st);
            }

            _ => {
                return Err(MainControlAbort::Confusion(c"prefix"));
            }
        }
    }

    // done:
    if st.scanner.after_token != 0 {
        st.scanner.cur_tok = st.scanner.after_token;
        crate::input::back_input_in(st)?;
        st.scanner.after_token = 0;
    }
    Ok(())
}
