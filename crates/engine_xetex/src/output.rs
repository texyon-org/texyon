use std::ffi::CStr;

use crate::bridge::{
    diag_append_cstr, diag_append_str, issue_warning_cstr, issue_warning_str,
    ttbc_diag_begin_error_ptr, ttbc_diag_begin_warning_ptr, ttstub_diag_finish,
    ttstub_output_flush, ttstub_output_putc,
};
use crate::cmd::{
    ABOVE, ACCENT, ADVANCE, AFTER_ASSIGNMENT, AFTER_GROUP, ASSIGN_DIMEN, ASSIGN_FONT_DIMEN,
    ASSIGN_FONT_INT, ASSIGN_GLUE, ASSIGN_INT, ASSIGN_MU_GLUE, ASSIGN_TOKS, BEGIN_GROUP,
    BREAK_PENALTY, CALL, CAR_RET, CASE_SHIFT, CHAR_GIVEN, CHAR_NUM, CONVERT, CS_NAME, DEF,
    DEF_CODE, DEF_FAMILY, DEF_FONT, DELIM_NUM, DISCRETIONARY, DIVIDE, ENDV, END_CS_NAME, END_GROUP,
    END_MATCH, END_TEMPLATE, EQ_NO, EXPAND_AFTER, EXTENSION, EX_SPACE, FI_OR_ELSE, HALIGN, HMOVE,
    HRULE, HSKIP, HYPH_DATA, IF_TEST, IGNORE_SPACES, INPUT, INSERT, IN_STREAM, ITAL_CORR, KERN,
    LAST_ITEM, LEADER_SHIP, LEFT_BRACE, LEFT_RIGHT, LET, LETTER, LIMIT_SWITCH, LONG_CALL,
    LONG_OUTER_CALL, MAC_PARAM, MAKE_BOX, MARK, MATCH, MATH_ACCENT, MATH_CHAR_NUM, MATH_CHOICE,
    MATH_COMP, MATH_GIVEN, MATH_SHIFT, MATH_STYLE, MAX_COMMAND, MESSAGE, MKERN, MSKIP, MULTIPLY,
    NON_SCRIPT, NO_ALIGN, NO_BOUNDARY, NO_EXPAND, OMIT, OTHER_CHAR, OUTER_CALL, OUT_PARAM, PAR_END,
    PREFIX, RADICAL, READ_TO_CS, REGISTER, RELAX, REMOVE_ITEM, RIGHT_BRACE, SET_AUX, SET_BOX,
    SET_BOX_DIMEN, SET_FONT, SET_INTERACTION, SET_PAGE_DIMEN, SET_PAGE_INT, SET_PREV_GRAF,
    SET_SHAPE, SHORTHAND_DEF, SPACER, START_PAR, STOP, SUB_MARK, SUP_MARK, TAB_MARK, THE,
    TOKS_REGISTER, TOP_BOT_MARK, UNDEFINED_CS, UN_HBOX, UN_VBOX, VADJUST, VALIGN, VCENTER, VMOVE,
    VRULE, VSKIP, XETEX_DEF_CODE, XETEX_MATH_GIVEN, XRAY,
};
use crate::consts::{
    ACC_KERN, ACTIVE_BASE, AFTER, A_LEADERS, BEGIN_L_CODE, BEGIN_R_CODE, BIGGEST_CHAR, BIGGEST_USV,
    CAT_CODE_BASE, COND_MATH_GLUE, COUNT_BASE, CS_TOKEN_FLAG, C_LEADERS, DEL_CODE_BASE, DIMEN_BASE,
    DLIST, EMPTY, END_L_CODE, EQTB_SIZE, ETEX_PEN_BASE, FILLL, FONT_ID_BASE, FROZEN_NULL_FONT,
    GLUE_BASE, HASH_BASE, HISTORY_SPOTLESS, HISTORY_WARNING_ISSUED, HMODE, IGNORE_DEPTH, INT_BASE,
    INT_PAR_END_LINE_CHAR, INT_PAR_ERROR_CONTEXT_LINES, INT_PAR_ESCAPE_CHAR, INT_PAR_NEW_LINE_CHAR,
    INT_PAR_SHOW_BOX_BREADTH, INT_PAR_SHOW_BOX_DEPTH, INT_PAR_TRACING_IFS,
    INT_PAR_TRACING_LOST_CHARS, INT_PAR_TRACING_NESTING, INT_PAR_TRACING_ONLINE, LC_CODE_BASE,
    LOCAL_BASE, LOCAL_PAR_SHAPE, L_CODE, MATH_CHAR, MATH_CODE_BASE, MATH_FONT_BASE, MMODE, MU_GLUE,
    MU_SKIP_BASE, NORMAL, NULL_CS, NULL_FLAG, PRIM_EQTB_BASE, R_CODE, SCALED_BASE, SCRIPT_SIZE,
    SELECTOR_LOG_ONLY, SELECTOR_NEW_STRING, SELECTOR_NO_PRINT, SELECTOR_TERM_AND_LOG,
    SELECTOR_TERM_ONLY, SF_CODE_BASE, SINGLE_BASE, SKIP_BASE, SPACE_ADJUSTMENT, SUB_BOX, SUB_MLIST,
    TEXT_SIZE, TEX_NULL, TOKS_BASE, TOO_BIG_CHAR, UC_CODE_BASE, UNDEFINED_CONTROL_SEQUENCE,
    UNLESS_CODE, VMODE, X_LEADERS,
};
use crate::errors::{error_in, ErrorOutcome, ERR_HELP_LOC};
use crate::expand::{
    ETEX_REVISION_CODE, EXPANDED_CODE, FI_CODE, FONT_NAME_CODE, IFX_CODE, IF_CASE_CODE,
    IF_CAT_CODE, IF_CS_CODE, IF_DEF_CODE, IF_DIM_CODE, IF_EOF_CODE, IF_FALSE_CODE,
    IF_FONT_CHAR_CODE, IF_HBOX_CODE, IF_HMODE_CODE, IF_INNER_CODE, IF_INT_CODE, IF_IN_CSNAME_CODE,
    IF_MMODE_CODE, IF_ODD_CODE, IF_PRIMITIVE_CODE, IF_TRUE_CODE, IF_VBOX_CODE, IF_VMODE_CODE,
    IF_VOID_CODE, LEFT_MARGIN_KERN_CODE, MARKS_CODE, MEANING_CODE, NORMAL_DEVIATE_CODE,
    NUMBER_CODE, OR_CODE, PDF_CREATION_DATE_CODE, PDF_FILE_DUMP_CODE, PDF_FILE_MOD_DATE_CODE,
    PDF_FILE_SIZE_CODE, PDF_MDFIVE_SUM_CODE, PDF_STRCMP_CODE, RIGHT_MARGIN_KERN_CODE,
    ROMAN_NUMERAL_CODE, STRING_CODE, UNIFORM_DEVIATE_CODE, XETEX_FEATURE_NAME_CODE,
    XETEX_GLYPH_NAME_CODE, XETEX_REVISION_CODE, XETEX_SELECTOR_NAME_CODE, XETEX_UCHARCAT_CODE,
    XETEX_UCHAR_CODE, XETEX_VARIATION_NAME_CODE,
};
use crate::ext::{
    CLOSE_NODE, GLYPH_CODE, IMMEDIATE_CODE, OPEN_NODE, PDF_FILE_CODE, PDF_SAVE_POS_NODE,
    PIC_FILE_CODE, RESET_TIMER_CODE, SET_LANGUAGE_CODE, SET_RANDOM_SEED_CODE, SPECIAL_NODE,
    WRITE_NODE, XETEX_DEFAULT_ENCODING_EXTENSION_CODE, XETEX_INPUT_ENCODING_EXTENSION_CODE,
    XETEX_LINEBREAK_LOCALE_EXTENSION_CODE,
};
use crate::fonts::{AAT_FONT_FLAG, OTGR_FONT_FLAG};
use crate::input::{
    BACKED_UP, BACKED_UP_CHAR, EVERY_CR_TEXT, EVERY_DISPLAY_TEXT, EVERY_EOF_TEXT, EVERY_HBOX_TEXT,
    EVERY_JOB_TEXT, EVERY_MATH_TEXT, EVERY_PAR_TEXT, EVERY_VBOX_TEXT, INSERTED, INTER_CHAR_TEXT,
    MACRO, MARK_TEXT, OUTPUT_TEXT, PARAMETER, TECTONIC_CODA_TEXT, TOKEN_LIST, U_TEMPLATE,
    V_TEMPLATE, WRITE_TEXT,
};
use crate::main_control::{
    BOX_CODE, BOX_FLAG, CHAR_DEF_CODE, CHAR_SUB_DEF_CODE, COPY_CODE, COUNT_DEF_CODE, CR_CODE,
    DIMEN_DEF_CODE, FILL_CODE, FIL_CODE, GLOBAL_BOX_FLAG, LAST_BOX_CODE, LEADER_FLAG, LP_CODE_BASE,
    MATH_CHAR_DEF_CODE, MU_SKIP_DEF_CODE, RP_CODE_BASE, SHIP_OUT_FLAG, SKIP_CODE, SKIP_DEF_CODE,
    SPAN_CODE, SS_CODE, VSPLIT_CODE, VTOP_CODE, XETEX_MATH_CHAR_DEF_CODE,
    XETEX_MATH_CHAR_NUM_DEF_CODE,
};
use crate::math::{
    ABOVE_CODE, ATOP_CODE, DEFAULT_CODE, DELIMITED_CODE, LIMITS, MIDDLE_NOAD, NO_LIMITS, OVER_CODE,
};
use crate::mem::{CONTRIB_HEAD, GARBAGE, PAGE_HEAD, PAGE_INS_HEAD, TEMP_HEAD, ZERO_GLUE};
use crate::node::{
    AnyBox, AnyNode, GlueSpecNode, NodePtr, PageInsNode, SaNode, WhatsitNode, WhatsitSubtype,
};
use crate::packaging::EXACTLY;
use crate::save::DIMEN_VAL_LIMIT;
use crate::scan::{
    CURRENT_GROUP_LEVEL_CODE, CURRENT_GROUP_TYPE_CODE, CURRENT_IF_BRANCH_CODE,
    CURRENT_IF_LEVEL_CODE, CURRENT_IF_TYPE_CODE, DIMEN_VAL, ELAPSED_TIME_CODE, ETEX_EXPR,
    ETEX_VERSION_CODE, FONT_CHAR_DP_CODE, FONT_CHAR_HT_CODE, FONT_CHAR_IC_CODE, FONT_CHAR_WD_CODE,
    GLUE_SHRINK_CODE, GLUE_SHRINK_ORDER_CODE, GLUE_STRETCH_CODE, GLUE_STRETCH_ORDER_CODE,
    GLUE_TO_MU_CODE, GLUE_VAL, HEIGHT_OFFSET, INPUT_LINE_NO_CODE, INT_VAL, LAST_NODE_TYPE_CODE,
    MAX_CHAR_VAL, MU_TO_GLUE_CODE, MU_VAL, PAR_SHAPE_DIMEN_CODE, PAR_SHAPE_INDENT_CODE,
    PAR_SHAPE_LENGTH_CODE, PDF_LAST_X_POS_CODE, PDF_LAST_Y_POS_CODE, PDF_SHELL_ESCAPE_CODE,
    PROTECTED_TOKEN, RANDOM_SEED_CODE, WIDTH_OFFSET, XETEX_COUNT_FEATURES_CODE,
    XETEX_COUNT_GLYPHS_CODE, XETEX_COUNT_SELECTORS_CODE, XETEX_COUNT_VARIATIONS_CODE,
    XETEX_FEATURE_CODE_CODE, XETEX_FIND_FEATURE_BY_NAME_CODE, XETEX_FIND_SELECTOR_BY_NAME_CODE,
    XETEX_FIND_VARIATION_BY_NAME_CODE, XETEX_FIRST_CHAR_CODE, XETEX_FONT_TYPE_CODE,
    XETEX_GLYPH_BOUNDS_CODE, XETEX_GLYPH_INDEX_CODE, XETEX_IS_DEFAULT_SELECTOR_CODE,
    XETEX_IS_EXCLUSIVE_FEATURE_CODE, XETEX_LAST_CHAR_CODE, XETEX_MAP_CHAR_TO_GLYPH_CODE,
    XETEX_OT_COUNT_FEATURES_CODE, XETEX_OT_COUNT_LANGUAGES_CODE, XETEX_OT_COUNT_SCRIPTS_CODE,
    XETEX_OT_FEATURE_CODE, XETEX_OT_LANGUAGE_CODE, XETEX_OT_SCRIPT_CODE, XETEX_PDF_PAGE_COUNT_CODE,
    XETEX_SELECTOR_CODE_CODE, XETEX_VARIATION_CODE, XETEX_VARIATION_DEFAULT_CODE,
    XETEX_VARIATION_MAX_CODE, XETEX_VARIATION_MIN_CODE, XETEX_VERSION_CODE,
};
use crate::state::{EngineState, GroupCode, InteractionMode};
use crate::units::{GlueSign, MemIdx, MemPtr, NodeType};

const SELECTOR_PSEUDO: i32 = 20;

fn print_the_digs_in(st: &mut EngineState, mut k: u8) {
    while k > 0 {
        k -= 1;
        let d = st.errors.dig[k as usize] as i32;
        if d < 10 {
            print_char_in(st, b'0' as i32 + d);
        } else {
            print_char_in(st, b'A' as i32 - 10 + d);
        }
    }
}

/// Stores the base-`radix` digits of nonnegative `n` into `dig[k..]`, returning the new length.
fn store_digits(st: &mut EngineState, mut n: i32, radix: i32, mut k: u8) -> u8 {
    loop {
        st.errors.dig[k as usize] = (n % radix) as u8;
        n /= radix;
        k += 1;
        if n == 0 {
            return k;
        }
    }
}

pub(crate) fn print_int_in(st: &mut EngineState, mut n: i32) {
    let mut k: u8 = 0;
    if n < 0 {
        print_char_in(st, b'-' as i32);
        if n > -100_000_000 {
            n = -n;
        } else {
            // Peel off the last digit so negating cannot overflow.
            let m_val = -1 - n;
            n = m_val / 10;
            let m = (m_val % 10) + 1;
            k = 1;
            if m < 10 {
                st.errors.dig[0] = m as u8;
            } else {
                st.errors.dig[0] = 0;
                n += 1;
            }
        }
    }
    let k = store_digits(st, n, 10, k);
    print_the_digs_in(st, k);
}

pub(crate) fn print_hex_in(st: &mut EngineState, n: i32) {
    print_char_in(st, b'"' as i32);
    let k = store_digits(st, n, 16, 0);
    print_the_digs_in(st, k);
}

pub(crate) fn print_ucs_code_in(st: &mut EngineState, n: i32) {
    print_cstr_in(st, c"U+".as_ptr());
    let mut k = store_digits(st, n, 16, 0);
    while k < 4 {
        st.errors.dig[k as usize] = 0;
        k += 1;
    }
    print_the_digs_in(st, k);
}

pub(crate) fn print_cstr_in(st: &mut EngineState, str_ptr: *const libc::c_char) {
    let s = unsafe { CStr::from_ptr(str_ptr) };
    print_bytes_in(st, s.to_bytes());
}

fn print_bytes_in(st: &mut EngineState, bytes: &[u8]) {
    for &b in bytes {
        print_char_in(st, b as i32);
    }
}

fn at_line_start(st: &EngineState) -> bool {
    let sel = st.shipout.selector;
    !((st.shipout.term_offset > 0 && sel & 1 != 0)
        || (st.shipout.file_offset > 0 && sel >= SELECTOR_LOG_ONLY))
}

pub(crate) fn print_nl_cstr_in(st: &mut EngineState, str_ptr: *const libc::c_char) {
    if !at_line_start(st) {
        print_ln_in(st);
    }
    print_cstr_in(st, str_ptr);
}

pub(crate) fn print_nl_str_in(st: &mut EngineState, s: &str) {
    if !at_line_start(st) {
        print_ln_in(st);
    }
    print_bytes_in(st, s.as_bytes());
}

pub(crate) fn print_nl_in(st: &mut EngineState, s: i32) {
    if !at_line_start(st) {
        print_ln_in(st);
    }
    print_in(st, s);
}

fn print_escape_char_in(st: &mut EngineState) {
    let c = st.eqtb.int_par(INT_PAR_ESCAPE_CHAR);
    if (0..=BIGGEST_USV).contains(&c) {
        print_char_in(st, c);
    }
}

pub(crate) fn print_esc_cstr_in(st: &mut EngineState, s: *const libc::c_char) {
    print_escape_char_in(st);
    print_cstr_in(st, s);
}

pub(crate) fn print_esc_in(st: &mut EngineState, s: i32) {
    print_escape_char_in(st);
    print_in(st, s);
}

fn esc(st: &mut EngineState, s: &CStr) {
    print_esc_cstr_in(st, s.as_ptr());
}

pub(crate) fn print_roman_int_in(st: &mut EngineState, mut n: i32) {
    // Letter/divisor pairs: each letter is the previous value divided by the digit after it.
    const ROMAN_DATA: &[u8] = b"m2d5c2l5x2v5i";
    let mut j: usize = 0;
    let mut v: i32 = 1000;
    loop {
        while n >= v {
            print_char_in(st, ROMAN_DATA[j] as i32);
            n -= v;
        }
        if n <= 0 {
            return;
        }
        let mut k = j + 2;
        let mut u = v / (ROMAN_DATA[k - 1] as i32 - b'0' as i32);
        if ROMAN_DATA[k - 1] == b'2' {
            k += 2;
            u /= ROMAN_DATA[k - 1] as i32 - b'0' as i32;
        }
        if n + u >= v {
            print_char_in(st, ROMAN_DATA[k] as i32);
            n += u;
        } else {
            j += 2;
            v /= ROMAN_DATA[j - 1] as i32 - b'0' as i32;
        }
    }
}

pub(crate) fn print_scaled_in(st: &mut EngineState, mut s: i32) {
    if s < 0 {
        print_char_in(st, b'-' as i32);
        s = -s;
    }
    print_int_in(st, s / 0x10000);
    print_char_in(st, b'.' as i32);
    s = 10 * (s % 0x10000) + 5;
    let mut delta: i32 = 10;
    loop {
        if delta > 0x10000 {
            s = s + 0x8000 - 50000;
        }
        print_char_in(st, b'0' as i32 + (s / 0x10000));
        s = 10 * (s % 0x10000);
        delta *= 10;
        if s <= delta {
            break;
        }
    }
}

fn warn_char_in(st: &mut EngineState, c: i32) {
    let diag = st.shipout.current_diagnostic;
    if !diag.is_null() {
        let bytes: [u8; 2] = [c as u8, 0];
        unsafe { diag_append_cstr(diag, bytes.as_ptr() as *const libc::c_char) };
    }
}

pub(crate) fn print_ln_in(st: &mut EngineState) {
    match st.shipout.selector {
        sel @ (SELECTOR_TERM_ONLY | SELECTOR_LOG_ONLY | SELECTOR_TERM_AND_LOG) => {
            warn_char_in(st, b'\n' as i32);
            if sel != SELECTOR_LOG_ONLY {
                unsafe { ttstub_output_putc(st.shipout.rust_stdout, b'\n' as i32) };
            }
            if sel != SELECTOR_TERM_ONLY {
                unsafe { ttstub_output_putc(st.shipout.log_file, b'\n' as i32) };
            }
            if sel != SELECTOR_LOG_ONLY {
                st.shipout.term_offset = 0;
            }
            if sel != SELECTOR_TERM_ONLY {
                st.shipout.file_offset = 0;
            }
        }
        SELECTOR_NO_PRINT | SELECTOR_PSEUDO | SELECTOR_NEW_STRING => {}
        sel => {
            unsafe { ttstub_output_putc(st.shipout.write_file[sel as usize], b'\n' as i32) };
            st.shipout.write_prefix[sel as usize].push(b'\n' as i32);
        }
    }
}

/// Writes one UTF-16 code unit to the current selector.
pub(crate) fn print_raw_char_in(st: &mut EngineState, s: u16, incr_offset: bool) {
    match st.shipout.selector {
        sel @ (SELECTOR_TERM_ONLY | SELECTOR_LOG_ONLY | SELECTOR_TERM_AND_LOG) => {
            let to_term = sel != SELECTOR_LOG_ONLY;
            let to_log = sel != SELECTOR_TERM_ONLY;
            warn_char_in(st, s as i32);
            if to_term {
                unsafe { ttstub_output_putc(st.shipout.rust_stdout, s as i32) };
            }
            if to_log {
                unsafe { ttstub_output_putc(st.shipout.log_file, s as i32) };
            }
            if incr_offset {
                if to_term {
                    st.shipout.term_offset += 1;
                }
                if to_log {
                    st.shipout.file_offset += 1;
                }
            }
            if to_term && st.shipout.term_offset == st.config.max_print_line {
                unsafe { ttstub_output_putc(st.shipout.rust_stdout, b'\n' as i32) };
                st.shipout.term_offset = 0;
            }
            if to_log && st.shipout.file_offset == st.config.max_print_line {
                unsafe { ttstub_output_putc(st.shipout.log_file, b'\n' as i32) };
                st.shipout.file_offset = 0;
            }
        }
        SELECTOR_NO_PRINT => {}
        SELECTOR_PSEUDO => {
            let t = st.errors.tally;
            if t < st.errors.trick_count {
                st.errors.trick_buf[(t % st.errors.error_line) as usize] = s;
            }
        }
        SELECTOR_NEW_STRING => {
            let pp = st.strings.pool_ptr;
            if pp < st.strings.pool_size {
                st.strings.str_pool[pp as usize] = s;
                st.strings.pool_ptr = pp + 1;
            }
        }
        sel => {
            unsafe { ttstub_output_putc(st.shipout.write_file[sel as usize], s as i32) };
            st.shipout.write_prefix[sel as usize].push(s as i32);
        }
    }
    st.errors.tally += 1;
}

pub(crate) fn print_char_in(st: &mut EngineState, s: i32) {
    if st.shipout.selector > SELECTOR_PSEUDO && !st.shipout.doing_special {
        if s >= 0x10000 {
            print_raw_char_in(st, (0xD800 + (s - 0x10000) / 1024) as u16, true);
            print_raw_char_in(st, (0xDC00 + (s - 0x10000) % 1024) as u16, true);
        } else {
            print_raw_char_in(st, s as u16, true);
        }
        return;
    }

    if s == st.eqtb.int_par(INT_PAR_NEW_LINE_CHAR) && st.shipout.selector < SELECTOR_PSEUDO {
        print_ln_in(st);
        return;
    }

    if s < 32 && !st.shipout.doing_special {
        print_raw_char_in(st, b'^' as u16, true);
        print_raw_char_in(st, b'^' as u16, true);
        print_raw_char_in(st, (s + 64) as u16, true);
    } else if s < 127 {
        print_raw_char_in(st, s as u16, true);
    } else if s == 127 {
        if !st.shipout.doing_special {
            print_raw_char_in(st, b'^' as u16, true);
            print_raw_char_in(st, b'^' as u16, true);
            print_raw_char_in(st, b'?' as u16, true);
        } else {
            print_raw_char_in(st, s as u16, true);
        }
    } else if s < 160 && !st.shipout.doing_special {
        let hex_digit = |d: i32| {
            (if d < 10 {
                b'0' as i32 + d
            } else {
                b'a' as i32 + d - 10
            }) as u16
        };
        print_raw_char_in(st, b'^' as u16, true);
        print_raw_char_in(st, b'^' as u16, true);
        print_raw_char_in(st, hex_digit((s % 256) / 16), true);
        print_raw_char_in(st, hex_digit(s % 16), true);
    } else if st.shipout.selector == SELECTOR_PSEUDO {
        print_raw_char_in(st, s as u16, true);
    } else {
        // UTF-8, counting one column per character.
        if s < 2048 {
            print_raw_char_in(st, (192 + s / 64) as u16, false);
            print_raw_char_in(st, (128 + s % 64) as u16, true);
        } else if s < 0x10000 {
            print_raw_char_in(st, (224 + s / 4096) as u16, false);
            print_raw_char_in(st, (128 + (s % 4096) / 64) as u16, false);
            print_raw_char_in(st, (128 + s % 64) as u16, true);
        } else {
            print_raw_char_in(st, (240 + s / 0x40000) as u16, false);
            print_raw_char_in(st, (128 + (s % 0x40000) / 4096) as u16, false);
            print_raw_char_in(st, (128 + (s % 4096) / 64) as u16, false);
            print_raw_char_in(st, (128 + s % 64) as u16, true);
        }
    }
}

pub(crate) fn print_in(st: &mut EngineState, s: i32) {
    if s >= st.strings.str_ptr {
        return print_cstr_in(st, c"???".as_ptr());
    }
    if s <= BIGGEST_CHAR {
        if s < 0 {
            return print_cstr_in(st, c"???".as_ptr());
        }
        if st.shipout.selector > SELECTOR_PSEUDO {
            print_char_in(st, s);
            return;
        }
        if s == st.eqtb.int_par(INT_PAR_NEW_LINE_CHAR) && st.shipout.selector < SELECTOR_PSEUDO {
            print_ln_in(st);
            return;
        }
        let nl = st.eqtb.int_par(INT_PAR_NEW_LINE_CHAR);
        st.eqtb.set_int_par(INT_PAR_NEW_LINE_CHAR, -1);
        print_char_in(st, s);
        st.eqtb.set_int_par(INT_PAR_NEW_LINE_CHAR, nl);
        return;
    }

    let pool_idx = (s - TOO_BIG_CHAR) as usize;
    let mut i = st.strings.str_start[pool_idx] as usize;
    let end = st.strings.str_start[pool_idx + 1] as usize;
    while i < end {
        let c = st.strings.str_pool[i];
        if i + 1 < end {
            if let Some(pair) = surrogate_pair(c, st.strings.str_pool[i + 1]) {
                print_char_in(st, pair);
                i += 2;
                continue;
            }
        }
        print_char_in(st, c as i32);
        i += 1;
    }
}

fn is_high_surrogate(c: u16) -> bool {
    (0xD800..0xDC00).contains(&c)
}

fn surrogate_pair(hi: u16, lo: u16) -> Option<i32> {
    (is_high_surrogate(hi) && (0xDC00..0xE000).contains(&lo))
        .then(|| 0x10000 + (hi as i32 - 0xD800) * 1024 + (lo as i32 - 0xDC00))
}

fn print_current_string_in(st: &mut EngineState) {
    let start = st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
    for j in start..st.strings.pool_ptr {
        let c = st.strings.str_pool[j as usize];
        print_char_in(st, c as i32);
    }
}

pub(crate) fn capture_to_diagnostic_in(st: &mut EngineState, diagnostic: *mut std::ffi::c_void) {
    let cur = st.shipout.current_diagnostic;
    if !cur.is_null() {
        unsafe { ttstub_diag_finish(cur) };
    }
    st.shipout.current_diagnostic = diagnostic;
}

/// The innermost input level that is a real file, with its current line.
fn current_file_line(st: &EngineState) -> Option<(i32, i32)> {
    let mut level = st.input.in_open;
    while level > 0 && st.input.full_source_filename_stack[level as usize] == 0 {
        level -= 1;
    }
    if level == 0 {
        return None;
    }
    let line = if level == st.input.in_open {
        st.mem.line
    } else {
        st.input.line_stack[(level + 1) as usize]
    };
    Some((st.input.full_source_filename_stack[level as usize], line))
}

fn diagnostic_print_file_line_in(st: &mut EngineState, diagnostic: *mut std::ffi::c_void) {
    let Some((name, line)) = current_file_line(st) else {
        unsafe { diag_append_cstr(diagnostic, c"!".as_ptr()) };
        return;
    };
    let filename = unsafe { crate::texmfmp::gettexstring_in(&st.strings, name) };
    unsafe {
        let name = CStr::from_ptr(filename).to_string_lossy();
        diag_append_str(diagnostic, &format!("{name}:{line}: "));
        libc::free(filename as *mut std::ffi::c_void);
    }
}

pub(crate) fn diagnostic_begin_capture_warning_here_in(
    st: &mut EngineState,
) -> *mut std::ffi::c_void {
    let warning = ttbc_diag_begin_warning_ptr();
    diagnostic_print_file_line_in(st, warning);
    capture_to_diagnostic_in(st, warning);
    warning
}

pub(crate) fn error_here_with_diagnostic_in(
    st: &mut EngineState,
    message: *const libc::c_char,
) -> *mut std::ffi::c_void {
    let error = ttbc_diag_begin_error_ptr();
    diagnostic_print_file_line_in(st, error);
    unsafe { diag_append_cstr(error, message) };
    print_error_prefix_in(st);
    print_cstr_in(st, message);
    capture_to_diagnostic_in(st, error);
    error
}

/// Starts an error line with `! ` or, in file:line:error style, the source position.
pub(crate) fn print_error_prefix_in(st: &mut EngineState) {
    if st.config.file_line_error_style_p != 0 {
        print_file_line_in(st);
    } else {
        print_nl_cstr_in(st, c"! ".as_ptr());
    }
}

pub(crate) fn print_file_line_in(st: &mut EngineState) {
    let Some((name, line)) = current_file_line(st) else {
        print_nl_cstr_in(st, c"! ".as_ptr());
        return;
    };
    print_nl_cstr_in(st, c"".as_ptr());
    print_in(st, name);
    print_char_in(st, b':' as i32);
    print_int_in(st, line);
    print_cstr_in(st, c": ".as_ptr());
}

pub(crate) fn print_size_in(st: &mut EngineState, s: i32) {
    esc(
        st,
        match s {
            TEXT_SIZE => c"textfont",
            SCRIPT_SIZE => c"scriptfont",
            _ => c"scriptscriptfont",
        },
    );
}

fn print_write_whatsit_in(st: &mut EngineState, s: &CStr, p: NodePtr<WhatsitNode>) {
    esc(st, s);
    let v = p.write_stream(&st.mem);
    if v < 16 {
        print_int_in(st, v);
    } else if v == 16 {
        print_char_in(st, b'*' as i32);
    } else {
        print_char_in(st, b'-' as i32);
    }
}

fn str_range(st: &EngineState, s: i32) -> std::ops::Range<usize> {
    let start = st.strings.str_start[(s - TOO_BIG_CHAR) as usize];
    let end = st.strings.str_start[(s + 1 - TOO_BIG_CHAR) as usize];
    start as usize..end as usize
}

pub(crate) fn print_file_name_in(st: &mut EngineState, n: i32, a: i32, e: i32) {
    // Swaps one quote character for the other.
    const QUOTE_SUM: i32 = b'"' as i32 + b'\'' as i32;

    let mut must_quote = false;
    let mut quote_char: i32 = 0;
    for s in [a, n, e] {
        if s == 0 {
            continue;
        }
        for j in str_range(st, s) {
            if must_quote && quote_char != 0 {
                break;
            }
            let c = st.strings.str_pool[j] as i32;
            if c == b' ' as i32 {
                must_quote = true;
            } else if c == b'"' as i32 || c == b'\'' as i32 {
                must_quote = true;
                quote_char = QUOTE_SUM - c;
            }
        }
    }

    if must_quote {
        if quote_char == 0 {
            quote_char = b'"' as i32;
        }
        print_char_in(st, quote_char);
    }

    for s in [a, n, e] {
        if s == 0 {
            continue;
        }
        for j in str_range(st, s) {
            let c = st.strings.str_pool[j] as i32;
            if c == quote_char {
                print_in(st, quote_char);
                quote_char = QUOTE_SUM - quote_char;
                print_in(st, quote_char);
            }
            print_in(st, c);
        }
    }

    if quote_char != 0 {
        print_char_in(st, quote_char);
    }
}

/// The name string of a multi-letter control sequence at eqtb location `p`.
fn cs_text(st: &EngineState, p: i32) -> i32 {
    if (PRIM_EQTB_BASE..FROZEN_NULL_FONT).contains(&p) {
        st.eqtb.prim_s1(p - PRIM_EQTB_BASE) - 1
    } else {
        st.eqtb.hash_s1(p)
    }
}

pub(crate) fn print_cs_in(st: &mut EngineState, p: i32) {
    if p < HASH_BASE {
        if p >= SINGLE_BASE {
            if p == NULL_CS {
                esc(st, c"csname");
                esc(st, c"endcsname");
                print_char_in(st, b' ' as i32);
            } else {
                print_esc_in(st, p - SINGLE_BASE);
                if st.eqtb.b32s1(CAT_CODE_BASE + p - SINGLE_BASE) == LETTER.into() {
                    print_char_in(st, b' ' as i32);
                }
            }
        } else if p < ACTIVE_BASE {
            esc(st, c"IMPOSSIBLE.");
        } else {
            print_char_in(st, p - 1);
        }
    } else if (UNDEFINED_CONTROL_SEQUENCE..=EQTB_SIZE).contains(&p) || p > st.eqtb.eqtb_top {
        esc(st, c"IMPOSSIBLE.");
    } else if st.eqtb.hash_s1(p) >= st.strings.str_ptr {
        esc(st, c"NONEXISTENT.");
    } else {
        print_esc_in(st, cs_text(st, p));
        print_char_in(st, b' ' as i32);
    }
}

/// Like [`print_cs_in`] but without the trailing space.
pub(crate) fn sprint_cs_in(st: &mut EngineState, p: i32) {
    if p >= HASH_BASE {
        print_esc_in(st, cs_text(st, p));
    } else if p < SINGLE_BASE {
        print_char_in(st, p - 1);
    } else if p < NULL_CS {
        print_esc_in(st, p - SINGLE_BASE);
    } else {
        esc(st, c"csname");
        esc(st, c"endcsname");
    }
}

fn print_native_word_in(st: &mut EngineState, p: NodePtr<WhatsitNode>) {
    let len = p.native_length(&st.mem) as usize;
    let mut i = 0;
    while i < len {
        let c = p.native_text(&st.mem)[i];
        if i + 1 < len {
            if let Some(pair) = surrogate_pair(c, p.native_text(&st.mem)[i + 1]) {
                print_char_in(st, pair);
                i += 2;
                continue;
            }
        }
        if is_high_surrogate(c) {
            print_in(st, b'.' as i32);
        } else {
            print_char_in(st, c as i32);
        }
        i += 1;
    }
}

fn print_sa_num_in(st: &mut EngineState, q: NodePtr<SaNode>) {
    let mem = &st.mem;
    let n = if q.sa_index(mem) < DIMEN_VAL_LIMIT {
        q.sa_ptr(mem).0
    } else {
        let q1 = q.parent(mem);
        let q2 = q1.parent(mem);
        let q3 = q2.parent(mem);
        let digit = |q: NodePtr<SaNode>| q.sa_index(mem) as i32;
        digit(q) % 64 + 64 * digit(q1) + 64 * 64 * (digit(q2) + 64 * digit(q3))
    };
    print_int_in(st, n);
}

pub(crate) fn begin_diagnostic_in(st: &mut EngineState) {
    st.shipout.old_setting = st.shipout.selector as u8;
    if st.eqtb.int_par(INT_PAR_TRACING_ONLINE) <= 0 && st.shipout.selector == SELECTOR_TERM_AND_LOG
    {
        st.shipout.selector -= 1;
        note_warning_issued(st);
    }
}

pub(crate) fn end_diagnostic_in(st: &mut EngineState, blank_line: bool) {
    print_nl_cstr_in(st, c"".as_ptr());
    if blank_line {
        print_ln_in(st);
    }
    st.shipout.selector = st.shipout.old_setting as i32;
}

fn note_warning_issued(st: &mut EngineState) {
    if st.errors.history == HISTORY_SPOTLESS {
        st.errors.history = HISTORY_WARNING_ISSUED;
    }
}

fn mode_name(m: i32) -> Option<&'static CStr> {
    let group = MAX_COMMAND as i32 + 1;
    match m.signum() {
        0 => Some(c"no mode"),
        1 => [c"vertical mode", c"horizontal mode", c"display math mode"]
            .get((m / group) as usize)
            .copied(),
        _ => [
            c"internal vertical mode",
            c"restricted horizontal mode",
            c"math mode",
        ]
        .get((-m / group) as usize)
        .copied(),
    }
}

pub(crate) fn print_mode_in(st: &mut EngineState, m: i32) {
    if let Some(name) = mode_name(m) {
        print_cstr_in(st, name.as_ptr());
    }
}

pub(crate) fn print_in_mode_in(st: &mut EngineState, m: i32) {
    if let Some(name) = mode_name(m) {
        print_cstr_in(st, c"' in ".as_ptr());
        print_cstr_in(st, name.as_ptr());
    }
}

pub(crate) fn print_group_in(st: &mut EngineState, e: bool) {
    let name = match st.save_stack.cur_group {
        GroupCode::BottomLevel => {
            print_cstr_in(st, c"bottom level".as_ptr());
            return;
        }
        GroupCode::Simple => c"simple",
        GroupCode::SemiSimple => c"semi simple",
        GroupCode::Hbox => c"hbox",
        GroupCode::AdjustedHbox => c"adjusted hbox",
        GroupCode::Vbox => c"vbox",
        GroupCode::Vtop => c"vtop",
        GroupCode::Align => c"align",
        GroupCode::NoAlign => c"no align",
        GroupCode::Output => c"output",
        GroupCode::Disc => c"disc",
        GroupCode::Insert => c"insert",
        GroupCode::Vcenter => c"vcenter",
        GroupCode::Math => c"math",
        GroupCode::MathChoice => c"math choice",
        GroupCode::MathShift => c"math shift",
        GroupCode::MathLeft => c"math left",
    };
    print_cstr_in(st, name.as_ptr());
    print_cstr_in(st, c" group (level ".as_ptr());
    print_int_in(st, st.save_stack.cur_level as i32);
    print_char_in(st, b')' as i32);
    let line_at = st.save_stack.b32s1(st.save_stack.save_ptr - 1);
    if line_at != 0 {
        if e {
            print_cstr_in(st, c" entered at line ".as_ptr());
        } else {
            print_cstr_in(st, c" at line ".as_ptr());
        }
        print_int_in(st, line_at);
    }
}

pub(crate) fn token_show_in(st: &mut EngineState, p: impl crate::units::MemAddr) {
    let p = p.addr();
    if p != TEX_NULL {
        let head = st.mem.link_ptr(p);
        show_token_list_in(st, head, TEX_NULL, 10_000_000);
    }
}

pub(crate) fn print_spec_in(
    st: &mut EngineState,
    p: impl crate::units::MemAddr,
    s: *const libc::c_char,
) {
    let p = p.addr();
    let spec = (0..st.mem.lo_mem_max)
        .contains(&p)
        .then(|| NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr(p)));
    let Some(gs) = spec else {
        print_char_in(st, b'*' as i32);
        return;
    };
    print_scaled_in(st, gs.width(&st.mem).0);
    if !s.is_null() {
        print_cstr_in(st, s);
    }
    if gs.stretch(&st.mem).0 != 0 {
        print_cstr_in(st, c" plus ".as_ptr());
        print_glue_in(st, gs.stretch(&st.mem).0, gs.stretch_order(&st.mem), s);
    }
    if gs.shrink(&st.mem).0 != 0 {
        print_cstr_in(st, c" minus ".as_ptr());
        print_glue_in(st, gs.shrink(&st.mem).0, gs.shrink_order(&st.mem), s);
    }
}

pub(crate) fn print_skip_param_in(st: &mut EngineState, n: i32) {
    let name = match n {
        0 => c"lineskip",
        1 => c"baselineskip",
        2 => c"parskip",
        3 => c"abovedisplayskip",
        4 => c"belowdisplayskip",
        5 => c"abovedisplayshortskip",
        6 => c"belowdisplayshortskip",
        7 => c"leftskip",
        8 => c"rightskip",
        9 => c"topskip",
        10 => c"splittopskip",
        11 => c"tabskip",
        12 => c"spaceskip",
        13 => c"xspaceskip",
        14 => c"parfillskip",
        15 => c"XeTeXlinebreakskip",
        16 => c"thinmuskip",
        17 => c"medmuskip",
        18 => c"thickmuskip",
        _ => return print_cstr_in(st, c"[unknown glue parameter!]".as_ptr()),
    };
    esc(st, name);
}

pub(crate) fn print_param_in(st: &mut EngineState, n: i32) {
    let name = match n {
        0 => c"pretolerance",
        1 => c"tolerance",
        2 => c"linepenalty",
        3 => c"hyphenpenalty",
        4 => c"exhyphenpenalty",
        5 => c"clubpenalty",
        6 => c"widowpenalty",
        7 => c"displaywidowpenalty",
        8 => c"brokenpenalty",
        9 => c"binoppenalty",
        10 => c"relpenalty",
        11 => c"predisplaypenalty",
        12 => c"postdisplaypenalty",
        13 => c"interlinepenalty",
        14 => c"doublehyphendemerits",
        15 => c"finalhyphendemerits",
        16 => c"adjdemerits",
        17 => c"mag",
        18 => c"delimiterfactor",
        19 => c"looseness",
        20 => c"time",
        21 => c"day",
        22 => c"month",
        23 => c"year",
        24 => c"showboxbreadth",
        25 => c"showboxdepth",
        26 => c"hbadness",
        27 => c"vbadness",
        28 => c"pausing",
        29 => c"tracingonline",
        30 => c"tracingmacros",
        31 => c"tracingstats",
        32 => c"tracingparagraphs",
        33 => c"tracingpages",
        34 => c"tracingoutput",
        35 => c"tracinglostchars",
        36 => c"tracingcommands",
        37 => c"tracingrestores",
        38 => c"uchyph",
        39 => c"outputpenalty",
        40 => c"maxdeadcycles",
        41 => c"hangafter",
        42 => c"floatingpenalty",
        43 => c"globaldefs",
        44 => c"fam",
        45 => c"escapechar",
        46 => c"defaulthyphenchar",
        47 => c"defaultskewchar",
        48 => c"endlinechar",
        49 => c"newlinechar",
        50 => c"language",
        51 => c"lefthyphenmin",
        52 => c"righthyphenmin",
        53 => c"holdinginserts",
        54 => c"errorcontextlines",
        55 => c"tracingstacklevels",
        56 => c"tracingassigns",
        57 => c"tracinggroups",
        58 => c"tracingifs",
        59 => c"tracingscantokens",
        60 => c"tracingnesting",
        61 => c"predisplaydirection",
        62 => c"lastlinefit",
        63 => c"savingvdiscards",
        64 => c"savinghyphcodes",
        65 => c"suppressfontnotfounderror",
        67 => c"XeTeXlinebreakpenalty",
        68 => c"XeTeXprotrudechars",
        69 => c"TeXXeTstate",
        70 => c"XeTeXdashbreakstate",
        71 => c"XeTeXupwardsmode",
        72 => c"XeTeXuseglyphmetrics",
        73 => c"XeTeXinterchartokenstate",
        74 => c"XeTeXinputnormalization",
        77 => c"XeTeXtracingfonts",
        78 => c"XeTeXinterwordspaceshaping",
        79 => c"XeTeXgenerateactualtext",
        80 => c"XeTeXhyphenatablelength",
        81 => c"synctex",
        82 => c"pdfoutput",
        _ => return print_cstr_in(st, c"[unknown int32_t parameter!]".as_ptr()),
    };
    esc(st, name);
}

pub(crate) fn print_length_param_in(st: &mut EngineState, n: i32) {
    let name = match n {
        0 => c"parindent",
        1 => c"mathsurround",
        2 => c"lineskiplimit",
        3 => c"hsize",
        4 => c"vsize",
        5 => c"maxdepth",
        6 => c"splitmaxdepth",
        7 => c"boxmaxdepth",
        8 => c"hfuzz",
        9 => c"vfuzz",
        10 => c"delimitershortfall",
        11 => c"nulldelimiterspace",
        12 => c"scriptspace",
        13 => c"predisplaysize",
        14 => c"displaywidth",
        15 => c"displayindent",
        16 => c"overfullrule",
        17 => c"hangindent",
        18 => c"hoffset",
        19 => c"voffset",
        20 => c"emergencystretch",
        21 => c"pdfpagewidth",
        22 => c"pdfpageheight",
        _ => return print_cstr_in(st, c"[unknown dimen parameter!]".as_ptr()),
    };
    esc(st, name);
}

pub(crate) fn print_style_in(st: &mut EngineState, c: i32) {
    let name = match c / 2 {
        0 => c"displaystyle",
        1 => c"textstyle",
        2 => c"scriptstyle",
        3 => c"scriptscriptstyle",
        _ => return print_cstr_in(st, c"Unknown style!".as_ptr()),
    };
    esc(st, name);
}

/// Prints a character code, as a string number when it is one.
fn print_code_in(st: &mut EngineState, c: i32) {
    if c < TOO_BIG_CHAR {
        print_in(st, c);
    } else {
        print_char_in(st, c);
    }
}

/// Prints the math family and character packed in the noad field at `p`.
fn print_fam_and_char_in(st: &mut EngineState, p: i32) {
    let fam_word = st.mem.b16s1(p) as i32;
    esc(st, c"fam");
    print_int_in(st, fam_word % 256);
    print_char_in(st, b' ' as i32);
    print_code_in(st, st.mem.b16s0(p) as i32 + (fam_word / 256) * 65536);
}

/// The small and large character of the delimiter field at `p`, each with its family.
fn delimiter_parts(st: &EngineState, p: i32) -> [(i32, i32); 2] {
    let part = |fam_word: u16, char_word: u16| {
        let fam_word = fam_word as i32;
        (fam_word % 256, char_word as i32 + (fam_word / 256) * 65536)
    };
    let mem = &st.mem;
    [
        part(mem.b16s3(p), mem.b16s2(p)),
        part(mem.b16s1(p), mem.b16s0(p)),
    ]
}

fn delimiter_is_null(st: &EngineState, p: i32) -> bool {
    delimiter_parts(st, p) == [(0, 0); 2]
}

fn print_delimiter_in(st: &mut EngineState, p: i32) {
    let [(small_fam, small_char), (large_fam, large_char)] = delimiter_parts(st, p);
    let a = ((small_fam as i64 * 256 + small_char as i64) * 4096
        + large_fam as i64 * 256
        + large_char as i64) as i32;
    if a < 0 {
        print_int_in(st, a);
    } else {
        print_hex_in(st, a);
    }
}

pub(crate) fn print_rule_dimen_in(st: &mut EngineState, d: i32) {
    if d == NULL_FLAG {
        print_char_in(st, b'*' as i32);
    } else {
        print_scaled_in(st, d);
    }
}

/// Prints `d` with its fil/fill/filll order, or the unit `s` for finite glue.
pub(crate) fn print_glue_in(st: &mut EngineState, d: i32, order: u16, s: *const libc::c_char) {
    print_scaled_in(st, d);
    if !(NORMAL..=FILLL).contains(&order) {
        print_cstr_in(st, c"foul".as_ptr());
    } else if order > NORMAL {
        print_cstr_in(st, c"fil".as_ptr());
        for _ in 1..order {
            print_char_in(st, b'l' as i32);
        }
    } else if !s.is_null() {
        print_cstr_in(st, s);
    }
}

fn print_font_char_in(st: &mut EngineState, font: i32, c: u16) {
    if font > st.fonts.font_max {
        print_char_in(st, b'*' as i32);
    } else {
        print_esc_in(st, st.eqtb.hash_s1(FONT_ID_BASE + font));
    }
    print_char_in(st, b' ' as i32);
    print_in(st, c as i32);
}

fn print_font_and_char_in(st: &mut EngineState, p: i32) {
    if p > st.mem.mem_end {
        esc(st, c"CLOBBERED.");
    } else {
        print_font_char_in(st, st.mem.b16s1(p) as i32, st.mem.b16s0(p));
    }
}

fn print_mark_in(st: &mut EngineState, p: MemPtr) {
    print_char_in(st, b'{' as i32);
    if p.0 < st.mem.hi_mem_min || p.0 > st.mem.mem_end {
        esc(st, c"CLOBBERED.");
    } else {
        let head = st.mem.link_ptr(p);
        let l = st.config.max_print_line - 10;
        show_token_list_in(st, head, TEX_NULL, l);
    }
    print_char_in(st, b'}' as i32);
}

// Sub-codes of \lastitem.

// Sub-codes of \convert.

// Sub-codes of \if tests.

// Sub-codes of \extension.

const SHOW_BOX_CODE: i32 = 1;
const SHOW_THE_CODE: i32 = 2;
const SHOW_LISTS: i32 = 3;
const SHOW_GROUPS: i32 = 4;
const SHOW_TOKENS: i32 = 5;
const SHOW_IFS: i32 = 6;

pub(crate) fn print_cmd_chr_in(st: &mut EngineState, cmd: u8, chr_code: i32) {
    let char_cmd = |st: &mut EngineState, s: &CStr| {
        print_cstr_in(st, s.as_ptr());
        print_code_in(st, chr_code);
    };

    match cmd {
        LEFT_BRACE => char_cmd(st, c"begin-group character "),
        RIGHT_BRACE => char_cmd(st, c"end-group character "),
        MATH_SHIFT => char_cmd(st, c"math shift character "),
        MAC_PARAM => char_cmd(st, c"macro parameter character "),
        SUP_MARK => char_cmd(st, c"superscript character "),
        SUB_MARK => char_cmd(st, c"subscript character "),
        ENDV => print_cstr_in(st, c"end of alignment template".as_ptr()),
        SPACER => char_cmd(st, c"blank space "),
        LETTER => char_cmd(st, c"the letter "),
        OTHER_CHAR => char_cmd(st, c"the character "),
        ASSIGN_GLUE | ASSIGN_MU_GLUE => {
            if chr_code < SKIP_BASE {
                print_skip_param_in(st, chr_code - GLUE_BASE);
            } else if chr_code < MU_SKIP_BASE {
                esc(st, c"skip");
                print_int_in(st, chr_code - SKIP_BASE);
            } else {
                esc(st, c"muskip");
                print_int_in(st, chr_code - MU_SKIP_BASE);
            }
        }
        ASSIGN_TOKS => {
            if chr_code >= TOKS_BASE {
                esc(st, c"toks");
                print_int_in(st, chr_code - TOKS_BASE);
            } else {
                let name = match chr_code - LOCAL_BASE {
                    1 => c"output",
                    2 => c"everypar",
                    3 => c"everymath",
                    4 => c"everydisplay",
                    5 => c"everyhbox",
                    6 => c"everyvbox",
                    7 => c"everyjob",
                    8 => c"everycr",
                    10 => c"everyeof",
                    11 => c"XeTeXinterchartoks",
                    12 => c"TectonicCodaTokens",
                    _ => c"errhelp",
                };
                esc(st, name);
            }
        }
        ASSIGN_INT => {
            if chr_code < COUNT_BASE {
                print_param_in(st, chr_code - INT_BASE);
            } else {
                esc(st, c"count");
                print_int_in(st, chr_code - COUNT_BASE);
            }
        }
        ASSIGN_DIMEN => {
            if chr_code < SCALED_BASE {
                print_length_param_in(st, chr_code - DIMEN_BASE);
            } else {
                esc(st, c"dimen");
                print_int_in(st, chr_code - SCALED_BASE);
            }
        }
        ACCENT => esc(st, c"accent"),
        ADVANCE => esc(st, c"advance"),
        AFTER_ASSIGNMENT => esc(st, c"afterassignment"),
        AFTER_GROUP => esc(st, c"aftergroup"),
        ASSIGN_FONT_DIMEN => esc(st, c"fontdimen"),
        BEGIN_GROUP => esc(st, c"begingroup"),
        BREAK_PENALTY => esc(st, c"penalty"),
        CHAR_NUM => esc(st, c"char"),
        CS_NAME => esc(st, c"csname"),
        DEF_FONT => esc(st, c"font"),
        DELIM_NUM => esc(
            st,
            if chr_code == 1 {
                c"Udelimiter"
            } else {
                c"delimiter"
            },
        ),
        DIVIDE => esc(st, c"divide"),
        END_CS_NAME => esc(st, c"endcsname"),
        END_GROUP => esc(st, c"endgroup"),
        EX_SPACE => print_esc_in(st, b' ' as i32),
        EXPAND_AFTER => esc(
            st,
            if chr_code == 0 {
                c"expandafter"
            } else {
                c"unless"
            },
        ),
        HALIGN => esc(st, c"halign"),
        HRULE => esc(st, c"hrule"),
        IGNORE_SPACES => esc(
            st,
            if chr_code == 0 {
                c"ignorespaces"
            } else {
                c"primitive"
            },
        ),
        INSERT => esc(st, c"insert"),
        ITAL_CORR => print_esc_in(st, b'/' as i32),
        MARK => {
            esc(st, c"mark");
            if chr_code > 0 {
                print_char_in(st, b's' as i32);
            }
        }
        MATH_ACCENT => esc(
            st,
            if chr_code == 1 {
                c"Umathaccent"
            } else {
                c"mathaccent"
            },
        ),
        MATH_CHAR_NUM => esc(
            st,
            match chr_code {
                2 => c"Umathchar",
                1 => c"Umathcharnum",
                _ => c"mathchar",
            },
        ),
        MATH_CHOICE => esc(st, c"mathchoice"),
        MULTIPLY => esc(st, c"multiply"),
        NO_ALIGN => esc(st, c"noalign"),
        NO_BOUNDARY => esc(st, c"noboundary"),
        NO_EXPAND => esc(
            st,
            if chr_code == 0 {
                c"noexpand"
            } else {
                c"primitive"
            },
        ),
        NON_SCRIPT => esc(st, c"nonscript"),
        OMIT => esc(st, c"omit"),
        RADICAL => esc(
            st,
            if chr_code == 1 {
                c"Uradical"
            } else {
                c"radical"
            },
        ),
        READ_TO_CS => esc(st, if chr_code == 0 { c"read" } else { c"readline" }),
        RELAX => esc(st, c"relax"),
        SET_BOX => esc(st, c"setbox"),
        SET_PREV_GRAF => esc(st, c"prevgraf"),
        SET_SHAPE => {
            let name = if chr_code == LOCAL_BASE + LOCAL_PAR_SHAPE {
                c"parshape"
            } else {
                match chr_code - ETEX_PEN_BASE {
                    0 => c"interlinepenalties",
                    1 => c"clubpenalties",
                    2 => c"widowpenalties",
                    3 => c"displaywidowpenalties",
                    _ => return,
                }
            };
            esc(st, name);
        }
        THE => esc(
            st,
            match chr_code {
                0 => c"the",
                1 => c"unexpanded",
                _ => c"detokenize",
            },
        ),
        TOKS_REGISTER => {
            esc(st, c"toks");
            if chr_code != 0 {
                if let Some(q) = NodePtr::<SaNode>::from_ptr(MemPtr(chr_code)) {
                    print_sa_num_in(st, q);
                }
            }
        }
        VADJUST => esc(st, c"vadjust"),
        VALIGN => esc(
            st,
            match chr_code {
                0 => c"valign",
                x if x == BEGIN_L_CODE as i32 => c"beginL",
                x if x == END_L_CODE as i32 => c"endL",
                x if x == BEGIN_R_CODE as i32 => c"beginR",
                _ => c"endR",
            },
        ),
        VCENTER => esc(st, c"vcenter"),
        VRULE => esc(st, c"vrule"),
        PAR_END => esc(st, c"par"),
        INPUT => esc(
            st,
            match chr_code {
                0 => c"input",
                2 => c"scantokens",
                _ => c"endinput",
            },
        ),
        TOP_BOT_MARK => {
            let name = match chr_code % MARKS_CODE {
                1 => c"firstmark",
                2 => c"botmark",
                3 => c"splitfirstmark",
                4 => c"splitbotmark",
                _ => c"topmark",
            };
            esc(st, name);
            if chr_code >= MARKS_CODE {
                print_char_in(st, b's' as i32);
            }
        }
        REGISTER => {
            let sa = if (0..=19).contains(&chr_code) {
                None
            } else {
                NodePtr::<SaNode>::from_ptr(MemPtr(chr_code))
            };
            let the_cmd = sa.map_or(chr_code as u8, |q| q.sa_type(&st.mem) as u8);
            let name = match the_cmd {
                INT_VAL => c"count",
                DIMEN_VAL => c"dimen",
                GLUE_VAL => c"skip",
                _ => c"muskip",
            };
            esc(st, name);
            if let Some(q) = sa {
                print_sa_num_in(st, q);
            }
        }
        SET_AUX => esc(
            st,
            if chr_code == VMODE as i32 {
                c"prevdepth"
            } else {
                c"spacefactor"
            },
        ),
        SET_PAGE_INT => esc(
            st,
            match chr_code {
                0 => c"deadcycles",
                2 => c"interactionmode",
                _ => c"insertpenalties",
            },
        ),
        SET_BOX_DIMEN => esc(
            st,
            match chr_code {
                WIDTH_OFFSET => c"wd",
                HEIGHT_OFFSET => c"ht",
                _ => c"dp",
            },
        ),
        LAST_ITEM => {
            let name = match chr_code {
                x if x == INT_VAL as i32 => c"lastpenalty",
                x if x == DIMEN_VAL as i32 => c"lastkern",
                x if x == GLUE_VAL as i32 => c"lastskip",
                INPUT_LINE_NO_CODE => c"inputlineno",
                LAST_NODE_TYPE_CODE => c"lastnodetype",
                ETEX_VERSION_CODE => c"eTeXversion",
                XETEX_VERSION_CODE => c"XeTeXversion",
                XETEX_COUNT_GLYPHS_CODE => c"XeTeXcountglyphs",
                XETEX_COUNT_VARIATIONS_CODE => c"XeTeXcountvariations",
                XETEX_VARIATION_CODE => c"XeTeXvariation",
                XETEX_FIND_VARIATION_BY_NAME_CODE => c"XeTeXfindvariationbyname",
                XETEX_VARIATION_MIN_CODE => c"XeTeXvariationmin",
                XETEX_VARIATION_MAX_CODE => c"XeTeXvariationmax",
                XETEX_VARIATION_DEFAULT_CODE => c"XeTeXvariationdefault",
                XETEX_COUNT_FEATURES_CODE => c"XeTeXcountfeatures",
                XETEX_FEATURE_CODE_CODE => c"XeTeXfeaturecode",
                XETEX_FIND_FEATURE_BY_NAME_CODE => c"XeTeXfindfeaturebyname",
                XETEX_IS_EXCLUSIVE_FEATURE_CODE => c"XeTeXisexclusivefeature",
                XETEX_COUNT_SELECTORS_CODE => c"XeTeXcountselectors",
                XETEX_SELECTOR_CODE_CODE => c"XeTeXselectorcode",
                XETEX_FIND_SELECTOR_BY_NAME_CODE => c"XeTeXfindselectorbyname",
                XETEX_IS_DEFAULT_SELECTOR_CODE => c"XeTeXisdefaultselector",
                XETEX_OT_COUNT_SCRIPTS_CODE => c"XeTeXOTcountscripts",
                XETEX_OT_COUNT_LANGUAGES_CODE => c"XeTeXOTcountlanguages",
                XETEX_OT_COUNT_FEATURES_CODE => c"XeTeXOTcountfeatures",
                XETEX_OT_SCRIPT_CODE => c"XeTeXOTscripttag",
                XETEX_OT_LANGUAGE_CODE => c"XeTeXOTlanguagetag",
                XETEX_OT_FEATURE_CODE => c"XeTeXOTfeaturetag",
                XETEX_MAP_CHAR_TO_GLYPH_CODE => c"XeTeXcharglyph",
                XETEX_GLYPH_INDEX_CODE => c"XeTeXglyphindex",
                XETEX_GLYPH_BOUNDS_CODE => c"XeTeXglyphbounds",
                XETEX_FONT_TYPE_CODE => c"XeTeXfonttype",
                XETEX_FIRST_CHAR_CODE => c"XeTeXfirstfontchar",
                XETEX_LAST_CHAR_CODE => c"XeTeXlastfontchar",
                XETEX_PDF_PAGE_COUNT_CODE => c"XeTeXpdfpagecount",
                CURRENT_GROUP_LEVEL_CODE => c"currentgrouplevel",
                CURRENT_GROUP_TYPE_CODE => c"currentgrouptype",
                CURRENT_IF_LEVEL_CODE => c"currentiflevel",
                CURRENT_IF_TYPE_CODE => c"currentiftype",
                CURRENT_IF_BRANCH_CODE => c"currentifbranch",
                FONT_CHAR_WD_CODE => c"fontcharwd",
                FONT_CHAR_HT_CODE => c"fontcharht",
                FONT_CHAR_DP_CODE => c"fontchardp",
                FONT_CHAR_IC_CODE => c"fontcharic",
                PAR_SHAPE_LENGTH_CODE => c"parshapelength",
                PAR_SHAPE_INDENT_CODE => c"parshapeindent",
                PAR_SHAPE_DIMEN_CODE => c"parshapedimen",
                x if x == ETEX_EXPR + INT_VAL as i32 => c"numexpr",
                x if x == ETEX_EXPR + DIMEN_VAL as i32 => c"dimexpr",
                x if x == ETEX_EXPR + GLUE_VAL as i32 => c"glueexpr",
                x if x == ETEX_EXPR + MU_VAL as i32 => c"muexpr",
                GLUE_STRETCH_ORDER_CODE => c"gluestretchorder",
                GLUE_SHRINK_ORDER_CODE => c"glueshrinkorder",
                GLUE_STRETCH_CODE => c"gluestretch",
                GLUE_SHRINK_CODE => c"glueshrink",
                MU_TO_GLUE_CODE => c"mutoglue",
                GLUE_TO_MU_CODE => c"gluetomu",
                PDF_LAST_X_POS_CODE => c"pdflastxpos",
                PDF_LAST_Y_POS_CODE => c"pdflastypos",
                ELAPSED_TIME_CODE => c"elapsedtime",
                PDF_SHELL_ESCAPE_CODE => c"shellescape",
                RANDOM_SEED_CODE => c"randomseed",
                _ => c"badness",
            };
            esc(st, name);
        }
        CONVERT => {
            let name = match chr_code {
                NUMBER_CODE => c"number",
                ROMAN_NUMERAL_CODE => c"romannumeral",
                STRING_CODE => c"string",
                MEANING_CODE => c"meaning",
                FONT_NAME_CODE => c"fontname",
                ETEX_REVISION_CODE => c"eTeXrevision",
                EXPANDED_CODE => c"expanded",
                LEFT_MARGIN_KERN_CODE => c"leftmarginkern",
                RIGHT_MARGIN_KERN_CODE => c"rightmarginkern",
                PDF_CREATION_DATE_CODE => c"creationdate",
                PDF_FILE_MOD_DATE_CODE => c"filemoddate",
                PDF_FILE_SIZE_CODE => c"filesize",
                PDF_MDFIVE_SUM_CODE => c"mdfivesum",
                PDF_FILE_DUMP_CODE => c"filedump",
                PDF_STRCMP_CODE => c"strcmp",
                UNIFORM_DEVIATE_CODE => c"uniformdeviate",
                NORMAL_DEVIATE_CODE => c"normaldeviate",
                XETEX_REVISION_CODE => c"XeTeXrevision",
                XETEX_VARIATION_NAME_CODE => c"XeTeXvariationname",
                XETEX_FEATURE_NAME_CODE => c"XeTeXfeaturename",
                XETEX_SELECTOR_NAME_CODE => c"XeTeXselectorname",
                XETEX_GLYPH_NAME_CODE => c"XeTeXglyphname",
                XETEX_UCHAR_CODE => c"Uchar",
                XETEX_UCHARCAT_CODE => c"Ucharcat",
                _ => c"jobname",
            };
            esc(st, name);
        }
        IF_TEST => {
            if chr_code >= UNLESS_CODE {
                esc(st, c"unless");
            }
            let name = match (chr_code % UNLESS_CODE) as i16 {
                IF_CAT_CODE => c"ifcat",
                IF_INT_CODE => c"ifnum",
                IF_DIM_CODE => c"ifdim",
                IF_ODD_CODE => c"ifodd",
                IF_VMODE_CODE => c"ifvmode",
                IF_HMODE_CODE => c"ifhmode",
                IF_MMODE_CODE => c"ifmmode",
                IF_INNER_CODE => c"ifinner",
                IF_VOID_CODE => c"ifvoid",
                IF_HBOX_CODE => c"ifhbox",
                IF_VBOX_CODE => c"ifvbox",
                IFX_CODE => c"ifx",
                IF_EOF_CODE => c"ifeof",
                IF_TRUE_CODE => c"iftrue",
                IF_FALSE_CODE => c"iffalse",
                IF_CASE_CODE => c"ifcase",
                IF_PRIMITIVE_CODE => c"ifprimitive",
                IF_DEF_CODE => c"ifdefined",
                IF_CS_CODE => c"ifcsname",
                IF_FONT_CHAR_CODE => c"iffontchar",
                IF_IN_CSNAME_CODE => c"ifincsname",
                _ => c"if",
            };
            esc(st, name);
        }
        FI_OR_ELSE => {
            let name = if chr_code == FI_CODE as i32 {
                c"fi"
            } else if chr_code == OR_CODE as i32 {
                c"or"
            } else {
                c"else"
            };
            esc(st, name);
        }
        TAB_MARK => {
            if chr_code == SPAN_CODE {
                esc(st, c"span");
            } else {
                char_cmd(st, c"alignment tab character ");
            }
        }
        CAR_RET => esc(st, if chr_code == CR_CODE { c"cr" } else { c"crcr" }),
        SET_PAGE_DIMEN => {
            let name = match chr_code {
                0 => c"pagegoal",
                1 => c"pagetotal",
                2 => c"pagestretch",
                3 => c"pagefilstretch",
                4 => c"pagefillstretch",
                5 => c"pagefilllstretch",
                6 => c"pageshrink",
                _ => c"pagedepth",
            };
            esc(st, name);
        }
        STOP => esc(st, if chr_code == 1 { c"dump" } else { c"end" }),
        HSKIP => esc(
            st,
            match chr_code {
                SKIP_CODE => c"hskip",
                FIL_CODE => c"hfil",
                FILL_CODE => c"hfill",
                SS_CODE => c"hss",
                _ => c"hfilneg",
            },
        ),
        VSKIP => esc(
            st,
            match chr_code {
                SKIP_CODE => c"vskip",
                FIL_CODE => c"vfil",
                FILL_CODE => c"vfill",
                SS_CODE => c"vss",
                _ => c"vfilneg",
            },
        ),
        MSKIP => esc(st, c"mskip"),
        KERN => esc(st, c"kern"),
        MKERN => esc(st, c"mkern"),
        HMOVE => esc(
            st,
            if chr_code == 1 {
                c"moveleft"
            } else {
                c"moveright"
            },
        ),
        VMOVE => esc(st, if chr_code == 1 { c"raise" } else { c"lower" }),
        MAKE_BOX => {
            let name = match chr_code {
                BOX_CODE => c"box",
                COPY_CODE => c"copy",
                LAST_BOX_CODE => c"lastbox",
                VSPLIT_CODE => c"vsplit",
                VTOP_CODE => c"vtop",
                x if x == VTOP_CODE + VMODE as i32 => c"vbox",
                _ => c"hbox",
            };
            esc(st, name);
        }
        LEADER_SHIP => {
            let name = match chr_code - A_LEADERS as i32 {
                0 => c"leaders",
                1 => c"cleaders",
                2 => c"xleaders",
                _ => c"shipout",
            };
            esc(st, name);
        }
        START_PAR => esc(
            st,
            if chr_code == 0 {
                c"noindent"
            } else {
                c"indent"
            },
        ),
        REMOVE_ITEM => {
            let name = if chr_code == NodeType::Glue as i32 {
                c"unskip"
            } else if chr_code == NodeType::Kern as i32 {
                c"unkern"
            } else {
                c"unpenalty"
            };
            esc(st, name);
        }
        UN_HBOX => esc(
            st,
            if chr_code == COPY_CODE {
                c"unhcopy"
            } else {
                c"unhbox"
            },
        ),
        UN_VBOX => esc(
            st,
            match chr_code {
                COPY_CODE => c"unvcopy",
                LAST_BOX_CODE => c"pagediscards",
                VSPLIT_CODE => c"splitdiscards",
                _ => c"unvbox",
            },
        ),
        DISCRETIONARY => {
            if chr_code == 1 {
                print_esc_in(st, b'-' as i32);
            } else {
                esc(st, c"discretionary");
            }
        }
        EQ_NO => esc(st, if chr_code == 1 { c"leqno" } else { c"eqno" }),
        MATH_COMP => {
            const ORD_NOAD: i32 = NodeType::OrdNoad as i32;
            const OP_NOAD: i32 = NodeType::OpNoad as i32;
            const BIN_NOAD: i32 = NodeType::BinNoad as i32;
            const REL_NOAD: i32 = NodeType::RelNoad as i32;
            const OPEN_NOAD: i32 = NodeType::OpenNoad as i32;
            const CLOSE_NOAD: i32 = NodeType::CloseNoad as i32;
            const PUNCT_NOAD: i32 = NodeType::PunctNoad as i32;
            const INNER_NOAD: i32 = NodeType::InnerNoad as i32;
            const UNDER_NOAD: i32 = NodeType::UnderNoad as i32;
            let name = match chr_code {
                ORD_NOAD => c"mathord",
                OP_NOAD => c"mathop",
                BIN_NOAD => c"mathbin",
                REL_NOAD => c"mathrel",
                OPEN_NOAD => c"mathopen",
                CLOSE_NOAD => c"mathclose",
                PUNCT_NOAD => c"mathpunct",
                INNER_NOAD => c"mathinner",
                UNDER_NOAD => c"underline",
                _ => c"overline",
            };
            esc(st, name);
        }
        LIMIT_SWITCH => esc(
            st,
            match chr_code as u16 {
                LIMITS => c"limits",
                NO_LIMITS => c"nolimits",
                _ => c"displaylimits",
            },
        ),
        MATH_STYLE => print_style_in(st, chr_code),
        ABOVE => {
            let name = match chr_code {
                OVER_CODE => c"over",
                ATOP_CODE => c"atop",
                x if x == DELIMITED_CODE + ABOVE_CODE => c"abovewithdelims",
                x if x == DELIMITED_CODE + OVER_CODE => c"overwithdelims",
                x if x == DELIMITED_CODE + ATOP_CODE => c"atopwithdelims",
                _ => c"above",
            };
            esc(st, name);
        }
        LEFT_RIGHT => {
            let name = if chr_code == NodeType::LeftNoad as i32 {
                c"left"
            } else if chr_code == MIDDLE_NOAD as i32 {
                c"middle"
            } else {
                c"right"
            };
            esc(st, name);
        }
        PREFIX => esc(
            st,
            match chr_code {
                1 => c"long",
                2 => c"outer",
                8 => c"protected",
                _ => c"global",
            },
        ),
        DEF => esc(
            st,
            match chr_code {
                0 => c"def",
                1 => c"gdef",
                2 => c"edef",
                _ => c"xdef",
            },
        ),
        LET => esc(st, if chr_code != 0 { c"futurelet" } else { c"let" }),
        SHORTHAND_DEF => {
            let name = match chr_code {
                CHAR_DEF_CODE => c"chardef",
                MATH_CHAR_DEF_CODE => c"mathchardef",
                XETEX_MATH_CHAR_DEF_CODE => c"Umathchardef",
                XETEX_MATH_CHAR_NUM_DEF_CODE => c"Umathcharnumdef",
                COUNT_DEF_CODE => c"countdef",
                DIMEN_DEF_CODE => c"dimendef",
                SKIP_DEF_CODE => c"skipdef",
                MU_SKIP_DEF_CODE => c"muskipdef",
                CHAR_SUB_DEF_CODE => c"charsubdef",
                _ => c"toksdef",
            };
            esc(st, name);
        }
        CHAR_GIVEN => {
            esc(st, c"char");
            print_hex_in(st, chr_code);
        }
        MATH_GIVEN => {
            esc(st, c"mathchar");
            print_hex_in(st, chr_code);
        }
        XETEX_MATH_GIVEN => {
            let code = chr_code as u32;
            esc(st, c"Umathchar");
            print_hex_in(st, ((code >> 21) & 0x07) as i32);
            print_hex_in(st, ((code >> 24) & 0xFF) as i32);
            print_hex_in(st, (code & 0x1F_FFFF) as i32);
        }
        DEF_CODE => {
            let name = match chr_code {
                LC_CODE_BASE => c"lccode",
                UC_CODE_BASE => c"uccode",
                SF_CODE_BASE => c"sfcode",
                MATH_CODE_BASE => c"mathcode",
                DEL_CODE_BASE => c"delcode",
                _ => c"catcode",
            };
            esc(st, name);
        }
        XETEX_DEF_CODE => {
            let name = match chr_code {
                SF_CODE_BASE => c"XeTeXcharclass",
                MATH_CODE_BASE => c"Umathcodenum",
                x if x == MATH_CODE_BASE + 1 => c"Umathcode",
                DEL_CODE_BASE => c"Udelcodenum",
                _ => c"Udelcode",
            };
            esc(st, name);
        }
        DEF_FAMILY => print_size_in(st, chr_code - MATH_FONT_BASE),
        HYPH_DATA => esc(
            st,
            if chr_code == 1 {
                c"patterns"
            } else {
                c"hyphenation"
            },
        ),
        ASSIGN_FONT_INT => {
            let name = match chr_code {
                0 => c"hyphenchar",
                1 => c"skewchar",
                LP_CODE_BASE => c"lpcode",
                RP_CODE_BASE => c"rpcode",
                _ => return,
            };
            esc(st, name);
        }
        SET_FONT => print_font_select_in(st, chr_code as usize),
        SET_INTERACTION => {
            const BATCH: i32 = InteractionMode::Batch as i32;
            const NONSTOP: i32 = InteractionMode::Nonstop as i32;
            const SCROLL: i32 = InteractionMode::Scroll as i32;
            let name = match chr_code {
                BATCH => c"batchmode",
                NONSTOP => c"nonstopmode",
                SCROLL => c"scrollmode",
                _ => c"errorstopmode",
            };
            esc(st, name);
        }
        IN_STREAM => esc(st, if chr_code == 0 { c"closein" } else { c"openin" }),
        MESSAGE => esc(
            st,
            if chr_code == 0 {
                c"message"
            } else {
                c"errmessage"
            },
        ),
        CASE_SHIFT => esc(
            st,
            if chr_code == LC_CODE_BASE {
                c"lowercase"
            } else {
                c"uppercase"
            },
        ),
        XRAY => {
            let name = match chr_code {
                SHOW_BOX_CODE => c"showbox",
                SHOW_THE_CODE => c"showthe",
                SHOW_LISTS => c"showlists",
                SHOW_GROUPS => c"showgroups",
                SHOW_TOKENS => c"showtokens",
                SHOW_IFS => c"showifs",
                _ => c"show",
            };
            esc(st, name);
        }
        UNDEFINED_CS => print_cstr_in(st, c"undefined".as_ptr()),
        CALL | LONG_CALL | OUTER_CALL | LONG_OUTER_CALL => {
            // Bit 0 is long, bit 1 outer, bit 2 protected.
            let mut n = (cmd - CALL) as i32;
            if st.mem.b32s0(st.mem.b32s1(chr_code)) == PROTECTED_TOKEN {
                n += 4;
            }
            if n / 4 % 2 != 0 {
                esc(st, c"protected");
            }
            if n % 2 != 0 {
                esc(st, c"long");
            }
            if n / 2 % 2 != 0 {
                esc(st, c"outer");
            }
            if n > 0 {
                print_char_in(st, b' ' as i32);
            }
            print_cstr_in(st, c"macro".as_ptr());
        }
        END_TEMPLATE => esc(st, c"outer endtemplate"),
        EXTENSION => {
            let name = match chr_code {
                OPEN_NODE => c"openout",
                WRITE_NODE => c"write",
                CLOSE_NODE => c"closeout",
                SPECIAL_NODE => c"special",
                IMMEDIATE_CODE => c"immediate",
                SET_LANGUAGE_CODE => c"setlanguage",
                PDF_SAVE_POS_NODE => c"pdfsavepos",
                RESET_TIMER_CODE => c"resettimer",
                SET_RANDOM_SEED_CODE => c"setrandomseed",
                PIC_FILE_CODE => c"XeTeXpicfile",
                PDF_FILE_CODE => c"XeTeXpdffile",
                GLYPH_CODE => c"XeTeXglyph",
                XETEX_LINEBREAK_LOCALE_EXTENSION_CODE => c"XeTeXlinebreaklocale",
                XETEX_INPUT_ENCODING_EXTENSION_CODE => c"XeTeXinputencoding",
                XETEX_DEFAULT_ENCODING_EXTENSION_CODE => c"XeTeXdefaultencoding",
                _ => return print_cstr_in(st, c"[unknown extension!]".as_ptr()),
            };
            esc(st, name);
        }
        _ => print_cstr_in(st, c"[unknown command code!]".as_ptr()),
    }
}

/// `select font <name>`, quoting native font names, plus ` at <size>pt` if scaled.
fn print_font_select_in(st: &mut EngineState, f: usize) {
    print_cstr_in(st, c"select font ".as_ptr());
    let name = st.fonts.font_name[f];
    let area = st.fonts.font_area[f];
    if area == AAT_FONT_FLAG || area == OTGR_FONT_FLAG {
        let has_double_quote = st.strings.str_pool[str_range(st, name)].contains(&(b'"' as u16));
        let quote_char = if has_double_quote { b'\'' } else { b'"' } as i32;
        print_char_in(st, quote_char);
        print_in(st, name);
        print_char_in(st, quote_char);
    } else {
        print_in(st, name);
    }
    if st.fonts.font_size[f] != st.fonts.font_dsize[f] {
        print_cstr_in(st, c" at ".as_ptr());
        print_scaled_in(st, st.fonts.font_size[f]);
        print_cstr_in(st, c"pt".as_ptr());
    }
}

pub(crate) fn print_meaning_in(st: &mut EngineState) {
    let cmd = st.scanner.cur_cmd;
    let chr = st.scanner.cur_chr;
    print_cmd_chr_in(st, cmd, chr);
    if cmd >= CALL {
        print_char_in(st, b':' as i32);
        print_ln_in(st);
        token_show_in(st, chr);
    } else if cmd == TOP_BOT_MARK && (chr as usize) < 5 {
        print_char_in(st, b':' as i32);
        print_ln_in(st);
        token_show_in(st, st.scanner.cur_mark[chr as usize]);
    }
}

/// Walks `base_ptr` down to the file at input level `level`, reporting whether it is a real file.
fn level_is_file(st: &mut EngineState, level: i32) -> bool {
    while st.input.input_stack[st.input.base_ptr as usize].state == TOKEN_LIST
        || st.input.input_stack[st.input.base_ptr as usize].index as i32 > level
    {
        st.input.base_ptr -= 1;
    }
    st.input.input_stack[st.input.base_ptr as usize].name > 17
}

pub(crate) fn group_warning_in(st: &mut EngineState) {
    st.input.base_ptr = st.input.input_ptr;
    st.input.input_stack[st.input.base_ptr as usize] = st.input.cur_input;
    let mut i = st.input.in_open;
    let mut w = false;
    while st.input.grp_stack[i as usize] == st.save_stack.cur_boundary && i > 0 {
        if st.eqtb.int_par(INT_PAR_TRACING_NESTING) > 0 && level_is_file(st, i) {
            w = true;
        }
        st.input.grp_stack[i as usize] = st.save_stack.b32s1(st.save_stack.save_ptr);
        i -= 1;
    }
    if w {
        print_nl_cstr_in(st, c"Warning: ".as_ptr());
        diagnostic_begin_capture_warning_here_in(st);
        print_cstr_in(st, c"end of ".as_ptr());
        print_group_in(st, true);
        print_cstr_in(st, c" of a different file".as_ptr());
        print_ln_in(st);
        if st.eqtb.int_par(INT_PAR_TRACING_NESTING) > 1 {
            show_context_in(st);
        }
        note_warning_issued(st);
        capture_to_diagnostic_in(st, std::ptr::null_mut());
    }
}

pub(crate) fn if_warning_in(st: &mut EngineState) {
    st.input.base_ptr = st.input.input_ptr;
    st.input.input_stack[st.input.base_ptr as usize] = st.input.cur_input;
    let mut i = st.input.in_open;
    let mut w = false;
    while st.input.if_stack[i as usize] == st.scanner.cond_ptr {
        if st.eqtb.int_par(INT_PAR_TRACING_NESTING) > 0 && level_is_file(st, i) {
            w = true;
        }
        st.input.if_stack[i as usize] = st.scanner.cond_ptr.link(&st.mem);
        i -= 1;
    }
    if w {
        print_nl_cstr_in(st, c"Warning: ".as_ptr());
        diagnostic_begin_capture_warning_here_in(st);
        print_cstr_in(st, c"end of ".as_ptr());
        print_cmd_chr_in(st, IF_TEST, st.scanner.cur_if as i32);
        print_if_line_in(st, st.scanner.if_line);
        print_cstr_in(st, c" of a different file".as_ptr());
        print_ln_in(st);
        if st.eqtb.int_par(INT_PAR_TRACING_NESTING) > 1 {
            show_context_in(st);
        }
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        note_warning_issued(st);
    }
}

fn print_if_line_in(st: &mut EngineState, line: i32) {
    if line != 0 {
        print_cstr_in(st, c" entered on line ".as_ptr());
        print_int_in(st, line);
    }
}

/// Pops the save-stack group boundary at `save_ptr` into `cur_group`.
fn leave_group_boundary(st: &mut EngineState) {
    let sp = st.save_stack.save_ptr;
    st.save_stack.cur_group =
        GroupCode::from_raw(st.save_stack.b16s0(sp) as u8).expect("valid group code");
    st.save_stack.save_ptr = st.save_stack.b32s1(sp);
}

/// The saved `(cur_if, if_line, if_limit)` of the conditional below cond node `p`.
fn saved_cond_state(st: &EngineState, p: MemPtr) -> (i16, i32, u8) {
    (
        st.mem.b16s0(p) as i16,
        st.mem.b32s1(p + 1),
        st.mem.b16s1(p) as u8,
    )
}

pub(crate) fn file_warning_in(st: &mut EngineState) {
    let saved_ptr = st.save_stack.save_ptr;
    let saved_level = st.save_stack.cur_level;
    let saved_group = st.save_stack.cur_group;
    st.save_stack.save_ptr = st.save_stack.cur_boundary;
    while st.input.grp_stack[st.input.in_open as usize] != st.save_stack.save_ptr {
        st.save_stack.cur_level -= 1;
        print_nl_cstr_in(st, c"Warning: ".as_ptr());
        diagnostic_begin_capture_warning_here_in(st);
        print_cstr_in(st, c"end of file when ".as_ptr());
        print_group_in(st, true);
        print_cstr_in(st, c" is incomplete".as_ptr());
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        leave_group_boundary(st);
    }
    st.save_stack.save_ptr = saved_ptr;
    st.save_stack.cur_level = saved_level;
    st.save_stack.cur_group = saved_group;

    let saved_cond = st.scanner.cond_ptr;
    let saved_limit = st.scanner.if_limit;
    let saved_if = st.scanner.cur_if;
    let saved_line = st.scanner.if_line;
    while st.input.if_stack[st.input.in_open as usize] != st.scanner.cond_ptr {
        print_nl_cstr_in(st, c"Warning: ".as_ptr());
        diagnostic_begin_capture_warning_here_in(st);
        print_cstr_in(st, c"end of file when ".as_ptr());
        print_cmd_chr_in(st, IF_TEST, st.scanner.cur_if as i32);
        if st.scanner.if_limit == FI_CODE {
            esc(st, c"else");
        }
        print_if_line_in(st, st.scanner.if_line);
        print_cstr_in(st, c" is incomplete".as_ptr());
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        let cond = st.scanner.cond_ptr;
        (st.scanner.cur_if, st.scanner.if_line, st.scanner.if_limit) = saved_cond_state(st, cond);
        st.scanner.cond_ptr = cond.link(&st.mem);
    }
    st.scanner.cond_ptr = saved_cond;
    st.scanner.if_limit = saved_limit;
    st.scanner.cur_if = saved_if;
    st.scanner.if_line = saved_line;

    print_ln_in(st);
    if st.eqtb.int_par(INT_PAR_TRACING_NESTING) > 1 {
        diagnostic_begin_capture_warning_here_in(st);
        show_context_in(st);
        capture_to_diagnostic_in(st, std::ptr::null_mut());
    }
    note_warning_issued(st);
}

pub(crate) fn show_cur_cmd_chr_in(st: &mut EngineState) {
    begin_diagnostic_in(st);
    diagnostic_begin_capture_warning_here_in(st);
    print_nl_in(st, b'{' as i32);
    if st.input.cur_list.mode != st.scanner.shown_mode {
        print_mode_in(st, st.input.cur_list.mode as i32);
        print_cstr_in(st, c": ".as_ptr());
        st.scanner.shown_mode = st.input.cur_list.mode;
    }
    print_cmd_chr_in(st, st.scanner.cur_cmd, st.scanner.cur_chr);
    if st.eqtb.int_par(INT_PAR_TRACING_IFS) > 0
        && (IF_TEST as u8..=FI_OR_ELSE).contains(&st.scanner.cur_cmd)
    {
        print_cstr_in(st, c": ".as_ptr());
        let (mut n, l) = if st.scanner.cur_cmd == FI_OR_ELSE {
            print_cmd_chr_in(st, IF_TEST, st.scanner.cur_if as i32);
            print_char_in(st, b' ' as i32);
            (0, st.scanner.if_line)
        } else {
            (1, st.mem.line)
        };
        let mut p = st.scanner.cond_ptr;
        while !p.is_null() {
            n += 1;
            p = p.link(&st.mem);
        }
        print_cstr_in(st, c"(level ".as_ptr());
        print_int_in(st, n);
        print_char_in(st, b')' as i32);
        print_if_line_in(st, l);
    }
    print_char_in(st, b'}' as i32);
    capture_to_diagnostic_in(st, std::ptr::null_mut());
    end_diagnostic_in(st, false);
}

pub(crate) fn change_if_limit_in(
    st: &mut EngineState,
    l: i16,
    p: impl crate::units::MemAddr,
) -> ErrorOutcome {
    let p = p.addr();
    if p == st.scanner.cond_ptr.0 {
        st.scanner.if_limit = l as u8;
        return ErrorOutcome::Continue;
    }
    let mut q = st.scanner.cond_ptr;
    while !q.is_null() {
        let next = q.link(&st.mem);
        if next.0 == p {
            st.mem.set_b16s1(q, l as u16);
            return ErrorOutcome::Continue;
        }
        q = next;
    }
    ErrorOutcome::Confusion(c"if")
}

/// C++ font-layer callback, reaching the run's state through the locator pointer.
#[no_mangle]
pub unsafe extern "C" fn font_feature_warning(
    feature_name_p: *const libc::c_void,
    feat_len: i32,
    setting_name_p: *const libc::c_void,
    set_len: i32,
) {
    let p = crate::state::current_engine_state();
    if !p.is_null() {
        font_feature_warning_in(&mut *p, feature_name_p, feat_len, setting_name_p, set_len);
    }
}

pub(crate) fn font_feature_warning_in(
    st: &mut EngineState,
    feature_name_p: *const libc::c_void,
    feat_len: i32,
    setting_name_p: *const libc::c_void,
    set_len: i32,
) {
    begin_diagnostic_in(st);
    diagnostic_begin_capture_warning_here_in(st);
    print_nl_cstr_in(st, c"Unknown ".as_ptr());
    if set_len > 0 {
        print_cstr_in(st, c"selector `".as_ptr());
        crate::ext::print_utf8_str_in(st, setting_name_p as *const u8, set_len);
        print_cstr_in(st, c"' for ".as_ptr());
    }
    print_cstr_in(st, c"feature `".as_ptr());
    crate::ext::print_utf8_str_in(st, feature_name_p as *const u8, feat_len);
    print_cstr_in(st, c"' in font `".as_ptr());
    print_name_of_file_in(st);
    print_cstr_in(st, c"'.".as_ptr());
    capture_to_diagnostic_in(st, std::ptr::null_mut());
    end_diagnostic_in(st, false);
}

pub(crate) fn font_mapping_warning_in(
    st: &mut EngineState,
    mapping_name_p: *const libc::c_void,
    mapping_name_len: i32,
    warning_type: i32,
) {
    begin_diagnostic_in(st);
    diagnostic_begin_capture_warning_here_in(st);
    if warning_type == 0 {
        print_nl_cstr_in(st, c"Loaded mapping `".as_ptr());
    } else {
        print_nl_cstr_in(st, c"Font mapping `".as_ptr());
    }
    crate::ext::print_utf8_str_in(st, mapping_name_p as *const u8, mapping_name_len);
    print_cstr_in(st, c"' for font `".as_ptr());
    print_name_of_file_in(st);
    match warning_type {
        1 => print_cstr_in(st, c"' not found.".as_ptr()),
        2 => {
            print_cstr_in(st, c"' not usable;".as_ptr());
            print_nl_cstr_in(st, c"bad mapping file or incorrect mapping type.".as_ptr());
        }
        _ => print_cstr_in(st, c"'.".as_ptr()),
    }
    capture_to_diagnostic_in(st, std::ptr::null_mut());
    end_diagnostic_in(st, false);
}

fn print_name_of_file_in(st: &mut EngineState) {
    let name = &st.input.name_of_file;
    let len = name.iter().position(|&b| b == 0).unwrap_or(name.len());
    for i in 0..len {
        let b = st.input.name_of_file[i] as u16;
        print_raw_char_in(st, b, true);
    }
}

pub(crate) fn bad_utf8_warning_in(st: &mut EngineState) {
    begin_diagnostic_in(st);
    diagnostic_begin_capture_warning_here_in(st);
    print_nl_cstr_in(st, c"Invalid UTF-8 byte or sequence".as_ptr());
    if st.input.cur_input.name == 0 {
        print_cstr_in(st, c" in terminal input".as_ptr());
    } else {
        print_cstr_in(st, c" at line ".as_ptr());
        print_int_in(st, st.mem.line);
    }
    print_cstr_in(st, c" replaced by U+FFFD.".as_ptr());
    capture_to_diagnostic_in(st, std::ptr::null_mut());
    end_diagnostic_in(st, false);
}

/// Records where the error context splits, when printing reaches the current position.
fn set_trick_count(st: &mut EngineState) {
    st.errors.first_count = st.errors.tally;
    let tc = st.errors.tally + 1 + st.errors.error_line - st.errors.half_error_line;
    st.errors.trick_count = tc.max(st.errors.error_line);
}

pub(crate) fn show_token_list_in(
    st: &mut EngineState,
    p: impl crate::units::MemAddr,
    q: i32,
    l: i32,
) {
    let mut p = MemPtr(p.addr());
    let mut match_chr = b'#' as i32;
    let mut n = b'0' as i32;
    st.errors.tally = 0;
    while !p.is_null() && st.errors.tally < l {
        if p.0 == q {
            set_trick_count(st);
        }
        if p.0 < st.mem.hi_mem_min || p.0 > st.mem.mem_end {
            esc(st, c"CLOBBERED.");
            return;
        }
        let w = p.info(&st.mem);
        if w >= CS_TOKEN_FLAG {
            print_cs_in(st, w - CS_TOKEN_FLAG);
        } else if w < 0 {
            esc(st, c"BAD.");
        } else {
            let c = w % MAX_CHAR_VAL;
            match (w / MAX_CHAR_VAL) as u8 {
                LEFT_BRACE | RIGHT_BRACE | MATH_SHIFT | TAB_MARK | SUP_MARK | SUB_MARK | SPACER
                | LETTER | OTHER_CHAR => print_char_in(st, c),
                MAC_PARAM => {
                    print_char_in(st, c);
                    print_char_in(st, c);
                }
                OUT_PARAM => {
                    print_char_in(st, match_chr);
                    if c > 9 {
                        print_char_in(st, b'!' as i32);
                        return;
                    }
                    print_char_in(st, b'0' as i32 + c);
                }
                MATCH => {
                    match_chr = c;
                    print_char_in(st, c);
                    n += 1;
                    print_char_in(st, n);
                    if n > b'9' as i32 {
                        return;
                    }
                }
                END_MATCH => {
                    if c == 0 {
                        print_cstr_in(st, c"->".as_ptr());
                    }
                }
                _ => esc(st, c"BAD."),
            }
        }
        p = p.link(&st.mem);
    }
    if !p.is_null() {
        esc(st, c"ETC.");
    }
}

fn print_totals_in(st: &mut EngineState) {
    print_scaled_in(st, st.shipout.page_so_far[1]);
    for (i, order) in [(2, c""), (3, c"fil"), (4, c"fill"), (5, c"filll")] {
        if st.shipout.page_so_far[i] != 0 {
            print_cstr_in(st, c" plus ".as_ptr());
            print_scaled_in(st, st.shipout.page_so_far[i]);
            print_cstr_in(st, order.as_ptr());
        }
    }
    if st.shipout.page_so_far[6] != 0 {
        print_cstr_in(st, c" minus ".as_ptr());
        print_scaled_in(st, st.shipout.page_so_far[6]);
    }
}

fn show_info_in(st: &mut EngineState) {
    let p = st.mem.b32s0(st.mem.temp_ptr);
    show_node_list_in(st, p);
}

/// Drops the most recently made string.
fn flush_last_string(st: &mut EngineState) {
    st.strings.str_ptr -= 1;
    st.strings.pool_ptr = st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
}

pub(crate) enum IssueMessageAbort {
    Scanner(crate::scan::SccAbort),
    NodeRecycle(crate::scan::NodeRecycleAbort),
    Error(ErrorOutcome),
    PoolSize,
}

impl IssueMessageAbort {
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            IssueMessageAbort::Scanner(e) => e.into_engine_abort(),
            IssueMessageAbort::NodeRecycle(e) => e.into_engine_abort(),
            IssueMessageAbort::Error(o) => o.into_engine_abort(),
            IssueMessageAbort::PoolSize => crate::errors::EngineAbort::PoolOverflow,
        }
    }
}

pub(crate) fn issue_message_in(st: &mut EngineState) -> Result<(), IssueMessageAbort> {
    let c = st.scanner.cur_chr as u8;
    let toks = crate::scan::scan_toks_in(st, false, true).map_err(IssueMessageAbort::Scanner)?;
    st.mem.set_b32s1(GARBAGE, toks);
    let old_setting = st.shipout.selector;
    st.shipout.selector = SELECTOR_NEW_STRING;
    token_show_in(st, st.scanner.def_ref);
    st.shipout.selector = old_setting;
    crate::mem::flush_list(&mut st.mem, st.scanner.def_ref);
    if st.strings.pool_ptr + 1 > st.strings.pool_size {
        return Err(IssueMessageAbort::PoolSize);
    }
    let s =
        crate::string_pool::make_string(&mut st.strings).map_err(IssueMessageAbort::NodeRecycle)?;
    if c == 0 {
        let len = crate::string_pool::length(&st.strings, s);
        if st.shipout.term_offset + len > st.config.max_print_line - 2 {
            print_ln_in(st);
        } else if st.shipout.term_offset > 0 || st.shipout.file_offset > 0 {
            print_char_in(st, b' ' as i32);
        }
        print_in(st, s);
        unsafe { ttstub_output_flush(st.shipout.rust_stdout) };
    } else {
        error_here_with_diagnostic_in(st, c"".as_ptr());
        print_in(st, s);
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        if st.eqtb.b32s1(ERR_HELP_LOC) != TEX_NULL {
            st.errors.use_err_help = true;
        } else if st.errors.long_help_seen {
            st.errors.help(&[r"(That was another \errmessage.)"]);
        } else {
            if st.errors.interaction < InteractionMode::ErrorStop {
                st.errors.long_help_seen = true;
            }
            st.errors.help(&[
                r"This error message was generated by an \errmessage",
                "command, so I can't give any explicit help.",
                "Pretend that you're Hercule Poirot: Examine all clues,",
                "and deduce the truth by order and method.",
            ]);
        }
        let outcome = error_in(st);
        st.errors.use_err_help = false;
        if !matches!(outcome, ErrorOutcome::Continue) {
            return Err(IssueMessageAbort::Error(outcome));
        }
    }
    flush_last_string(st);
    Ok(())
}

pub(crate) fn show_save_groups_in(st: &mut EngineState) {
    // Where the tex.web `found1`/`found2`/`found` label chain is entered.
    enum Entry {
        BoxContext,
        Found1,
        Found2,
        Found,
    }

    let mut p = st.input.nest_ptr;
    st.input.nest[p as usize] = st.input.cur_list;
    let saved_ptr = st.save_stack.save_ptr;
    let saved_level = st.save_stack.cur_level;
    let saved_group = st.save_stack.cur_group;
    st.save_stack.save_ptr = st.save_stack.cur_boundary;
    st.save_stack.cur_level -= 1;
    let mut a: i8 = 1;

    print_nl_cstr_in(st, c"".as_ptr());
    print_ln_in(st);

    loop {
        print_nl_cstr_in(st, c"### ".as_ptr());
        print_group_in(st, true);
        if st.save_stack.cur_group == GroupCode::BottomLevel {
            break;
        }

        // Find the enclosing non-paragraph mode, leaving `p` just below it.
        let m = loop {
            let cur_m = st.input.nest[p as usize].mode;
            if p == 0 {
                break VMODE;
            }
            p -= 1;
            if cur_m != HMODE {
                break cur_m;
            }
        };

        print_cstr_in(st, c" (".as_ptr());

        let saved = |st: &EngineState, k: i32| st.save_stack.b32s1(st.save_stack.save_ptr - k);
        let mut s: &CStr = c"";
        let entry = match st.save_stack.cur_group {
            GroupCode::Simple => {
                p += 1;
                Entry::Found2
            }
            GroupCode::Hbox | GroupCode::AdjustedHbox => {
                s = c"hbox";
                Entry::BoxContext
            }
            GroupCode::Vbox => {
                s = c"vbox";
                Entry::BoxContext
            }
            GroupCode::Vtop => {
                s = c"vtop";
                Entry::BoxContext
            }
            GroupCode::Align if a == 0 => {
                s = if m == -VMODE { c"halign" } else { c"valign" };
                a = 1;
                Entry::Found1
            }
            GroupCode::Align => {
                if a == 1 {
                    print_cstr_in(st, c"align entry".as_ptr());
                } else {
                    esc(st, c"cr");
                }
                if p >= a as i32 {
                    p -= a as i32;
                }
                a = 0;
                Entry::Found
            }
            GroupCode::NoAlign => {
                p += 1;
                a = -1;
                esc(st, c"noalign");
                Entry::Found2
            }
            GroupCode::Output => {
                esc(st, c"output");
                Entry::Found
            }
            GroupCode::Math => Entry::Found2,
            GroupCode::Disc | GroupCode::MathChoice => {
                if st.save_stack.cur_group == GroupCode::Disc {
                    esc(st, c"discretionary");
                } else {
                    esc(st, c"mathchoice");
                }
                let braces = saved(st, 2);
                for i in 1..=3 {
                    if i <= braces {
                        print_cstr_in(st, c"{}".as_ptr());
                    }
                }
                Entry::Found2
            }
            GroupCode::Insert => {
                let n = saved(st, 2);
                if n == 255 {
                    esc(st, c"vadjust");
                } else {
                    esc(st, c"insert");
                    print_int_in(st, n);
                }
                Entry::Found2
            }
            GroupCode::Vcenter => {
                s = c"vcenter";
                Entry::Found1
            }
            GroupCode::SemiSimple => {
                p += 1;
                esc(st, c"begingroup");
                Entry::Found
            }
            GroupCode::MathShift => {
                if m == MMODE {
                    print_char_in(st, b'$' as i32);
                } else if st.input.nest[p as usize].mode == MMODE {
                    print_cmd_chr_in(st, EQ_NO, saved(st, 2));
                    print_char_in(st, b')' as i32);
                    st.save_stack.cur_level -= 1;
                    leave_group_boundary(st);
                    continue;
                }
                print_char_in(st, b'$' as i32);
                Entry::Found
            }
            GroupCode::MathLeft => {
                let delim = MemIdx::new(st.input.nest[(p + 1) as usize].e_tex_aux);
                if st.mem.node_type(delim) == Ok(NodeType::LeftNoad) {
                    esc(st, c"left");
                } else {
                    esc(st, c"middle");
                }
                Entry::Found
            }
            GroupCode::BottomLevel => Entry::Found,
        };

        if matches!(entry, Entry::BoxContext) {
            let i = saved(st, 4);
            if i != 0 {
                if i < BOX_FLAG {
                    let cmd = if st.input.nest[p as usize].mode.abs() == VMODE {
                        HMOVE
                    } else {
                        VMOVE
                    };
                    print_cmd_chr_in(st, cmd, if i > 0 { 0 } else { 1 });
                    print_scaled_in(st, i.abs());
                    print_cstr_in(st, c"pt".as_ptr());
                } else if i < SHIP_OUT_FLAG {
                    let mut box_num = i;
                    if box_num >= GLOBAL_BOX_FLAG {
                        esc(st, c"global");
                        box_num -= GLOBAL_BOX_FLAG - BOX_FLAG;
                    }
                    esc(st, c"setbox");
                    print_int_in(st, box_num - BOX_FLAG);
                    print_char_in(st, b'=' as i32);
                } else {
                    print_cmd_chr_in(st, LEADER_SHIP, i - (LEADER_FLAG - A_LEADERS as i32));
                }
            }
        }

        if matches!(entry, Entry::BoxContext | Entry::Found1) {
            esc(st, s);
            let spec = saved(st, 2);
            if spec != 0 {
                print_char_in(st, b' ' as i32);
                if saved(st, 3) == EXACTLY as i32 {
                    print_cstr_in(st, c"to".as_ptr());
                } else {
                    print_cstr_in(st, c"spread".as_ptr());
                }
                print_scaled_in(st, spec);
                print_cstr_in(st, c"pt".as_ptr());
            }
        }

        if !matches!(entry, Entry::Found) {
            print_char_in(st, b'{' as i32);
        }

        print_char_in(st, b')' as i32);
        st.save_stack.cur_level -= 1;
        leave_group_boundary(st);
    }

    st.save_stack.save_ptr = saved_ptr;
    st.save_stack.cur_level = saved_level;
    st.save_stack.cur_group = saved_group;
}

pub(crate) enum ShowWhateverAbort {
    Scanner(crate::scan::SccAbort),
    Error(ErrorOutcome),
}

impl ShowWhateverAbort {
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            ShowWhateverAbort::Scanner(e) => e.into_engine_abort(),
            ShowWhateverAbort::Error(o) => o.into_engine_abort(),
        }
    }
}

impl From<crate::scan::SccAbort> for ShowWhateverAbort {
    fn from(e: crate::scan::SccAbort) -> Self {
        ShowWhateverAbort::Scanner(e)
    }
}

pub(crate) fn show_whatever_in(st: &mut EngineState) -> Result<(), ShowWhateverAbort> {
    const SHOW_CODE: i32 = 0;

    diagnostic_begin_capture_warning_here_in(st);

    let chr = st.scanner.cur_chr;
    match chr {
        SHOW_LISTS => {
            begin_diagnostic_in(st);
            show_activities_in(st);
        }
        SHOW_BOX_CODE => {
            crate::scan::scan_register_num_in(st)?;
            let n = st.scanner.cur_val;
            let contents = crate::scan::fetch_box(st, n);
            begin_diagnostic_in(st);
            print_nl_cstr_in(st, c"> \\box".as_ptr());
            print_int_in(st, n);
            print_char_in(st, b'=' as i32);
            match contents {
                Some(b) => show_box_in(st, b.ptr()),
                None => print_cstr_in(st, c"void".as_ptr()),
            }
        }
        SHOW_CODE => {
            crate::scan::get_token_in(st)?;
            print_nl_cstr_in(st, c"> ".as_ptr());
            if st.scanner.cur_cs != 0 {
                sprint_cs_in(st, st.scanner.cur_cs);
                print_char_in(st, b'=' as i32);
            }
            print_meaning_in(st);
        }
        SHOW_GROUPS => {
            begin_diagnostic_in(st);
            show_save_groups_in(st);
        }
        SHOW_IFS => {
            begin_diagnostic_in(st);
            print_nl_cstr_in(st, c"".as_ptr());
            print_ln_in(st);
            if st.scanner.cond_ptr.is_null() {
                print_nl_cstr_in(st, c"### ".as_ptr());
                print_cstr_in(st, c"no active conditionals".as_ptr());
            } else {
                let mut n = 0;
                let mut p = st.scanner.cond_ptr;
                while !p.is_null() {
                    n += 1;
                    p = p.link(&st.mem);
                }
                let mut p = st.scanner.cond_ptr;
                let mut t = st.scanner.cur_if;
                let mut l = st.scanner.if_line;
                let mut m = st.scanner.if_limit;
                while !p.is_null() {
                    print_nl_cstr_in(st, c"### level ".as_ptr());
                    print_int_in(st, n);
                    print_cstr_in(st, c": ".as_ptr());
                    print_cmd_chr_in(st, IF_TEST, t as i32);
                    if m == FI_CODE {
                        esc(st, c"else");
                    }
                    print_if_line_in(st, l);
                    n -= 1;
                    (t, l, m) = saved_cond_state(st, p);
                    p = p.link(&st.mem);
                }
            }
        }
        _ => {
            crate::scan::the_toks_in(st)?;
            print_nl_cstr_in(st, c"> ".as_ptr());
            token_show_in(st, TEMP_HEAD);
            let list = st.mem.link_ptr(TEMP_HEAD);
            crate::mem::flush_list(&mut st.mem, list);
        }
    }

    if matches!(chr, SHOW_LISTS | SHOW_BOX_CODE | SHOW_GROUPS | SHOW_IFS) {
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        end_diagnostic_in(st, true);
        print_error_prefix_in(st);
        print_cstr_in(st, c"OK".as_ptr());
        if st.shipout.selector == SELECTOR_TERM_AND_LOG
            && st.eqtb.int_par(INT_PAR_TRACING_ONLINE) <= 0
        {
            st.shipout.selector = SELECTOR_TERM_ONLY;
            print_cstr_in(st, c" (see the transcript file)".as_ptr());
            st.shipout.selector = SELECTOR_TERM_AND_LOG;
        }
    }

    capture_to_diagnostic_in(st, std::ptr::null_mut());
    if st.errors.interaction < InteractionMode::ErrorStop {
        st.errors.help(&[]);
        st.errors.error_count -= 1;
    } else if st.eqtb.int_par(INT_PAR_TRACING_ONLINE) > 0 {
        st.errors.help(&[
            r"This isn't an error message; I'm just \showing something.",
            r"Type `I\show...' to show more (e.g., \show\cs,",
            r"\showthe\count10, \showbox255, \showlists).",
        ]);
    } else {
        st.errors.help(&[
            r"This isn't an error message; I'm just \showing something.",
            r"Type `I\show...' to show more (e.g., \show\cs,",
            r"\showthe\count10, \showbox255, \showlists).",
            r"And type `I\tracingonline=1\show...' to show boxes and",
            "lists on your terminal as well as in the transcript file.",
        ]);
    }
    match error_in(st) {
        ErrorOutcome::Continue => Ok(()),
        outcome => Err(ShowWhateverAbort::Error(outcome)),
    }
}

pub(crate) fn char_warning_in(st: &mut EngineState, f: i32, c: i32) -> ErrorOutcome {
    let tracing_lost = st.eqtb.int_par(INT_PAR_TRACING_LOST_CHARS);
    if tracing_lost > 0 {
        let old_setting = st.eqtb.int_par(INT_PAR_TRACING_ONLINE);
        if tracing_lost > 1 {
            st.eqtb.set_int_par(INT_PAR_TRACING_ONLINE, 1);
        }
        if tracing_lost > 2 {
            print_error_prefix_in(st);
            print_cstr_in(st, c"Missing character: There is no ".as_ptr());
        } else {
            begin_diagnostic_in(st);
            diagnostic_begin_capture_warning_here_in(st);
            print_nl_cstr_in(st, c"Missing character: There is no ".as_ptr());
        }
        print_code_in(st, c);
        print_cstr_in(st, c" (".as_ptr());
        let area = st.fonts.font_area[f as usize];
        if area == AAT_FONT_FLAG || area == OTGR_FONT_FLAG {
            print_ucs_code_in(st, c);
        } else {
            print_hex_in(st, c);
        }
        print_char_in(st, b')' as i32);
        print_cstr_in(st, c" in font ".as_ptr());
        print_in(st, st.fonts.font_name[f as usize]);
        if tracing_lost < 3 {
            print_char_in(st, b'!' as i32);
        }
        st.eqtb.set_int_par(INT_PAR_TRACING_ONLINE, old_setting);
        if tracing_lost > 2 {
            st.errors.help(&[]);
            let outcome = error_in(st);
            if !matches!(outcome, ErrorOutcome::Continue) {
                return outcome;
            }
        } else {
            capture_to_diagnostic_in(st, std::ptr::null_mut());
            end_diagnostic_in(st, false);
        }
    }

    let font_name_c =
        unsafe { crate::texmfmp::gettexstring_in(&st.strings, st.fonts.font_name[f as usize]) };
    let prev_sel = st.shipout.selector;
    st.shipout.selector = SELECTOR_NEW_STRING;
    print_code_in(st, c);
    st.shipout.selector = prev_sel;
    let Ok(s) = crate::string_pool::make_string(&mut st.strings) else {
        return ErrorOutcome::Overflow {
            what: c"number of strings",
            n: st.strings.max_strings - st.strings.init_str_ptr,
        };
    };
    let chr_c = unsafe { crate::texmfmp::gettexstring_in(&st.strings, s) };
    flush_last_string(st);
    unsafe {
        issue_warning_str(&format!(
            "could not represent character \"{}\" (0x{:x}) in font \"{}\"",
            CStr::from_ptr(chr_c).to_string_lossy(),
            c as u32,
            CStr::from_ptr(font_name_c).to_string_lossy(),
        ));
        libc::free(font_name_c as *mut libc::c_void);
        libc::free(chr_c as *mut libc::c_void);
    }
    if !st.fonts.gave_char_warning_help {
        unsafe {
            issue_warning_cstr(
                c"  you may need to load the `fontspec` package and use (e.g.) \\setmainfont to"
                    .as_ptr(),
            );
            issue_warning_cstr(
                c"  choose a different font that covers the unrepresentable character(s)".as_ptr(),
            );
        }
        st.fonts.gave_char_warning_help = true;
    }
    ErrorOutcome::Continue
}

pub(crate) fn show_box_in(st: &mut EngineState, p: impl crate::units::MemAddr) {
    st.errors.depth_threshold = st.eqtb.int_par(INT_PAR_SHOW_BOX_DEPTH);
    st.errors.breadth_max = st.eqtb.int_par(INT_PAR_SHOW_BOX_BREADTH);
    if st.errors.breadth_max <= 0 {
        st.errors.breadth_max = 5;
    }
    let room = st.strings.pool_size - st.strings.pool_ptr - 1;
    if st.errors.depth_threshold > room {
        st.errors.depth_threshold = room;
    }
    show_node_list_in(st, p);
    print_ln_in(st);
}

pub(crate) fn short_display_in(st: &mut EngineState, p: impl crate::units::MemAddr) {
    let mut p = p.addr();
    while p > 0 {
        if p >= st.mem.hi_mem_min {
            if p <= st.mem.mem_end {
                let font = st.mem.b16s1(p) as i32;
                if font != st.errors.font_in_short_display {
                    if font > st.fonts.font_max {
                        print_char_in(st, b'*' as i32);
                    } else {
                        print_esc_in(st, st.eqtb.hash_s1(FONT_ID_BASE + font));
                    }
                    print_char_in(st, b' ' as i32);
                    st.errors.font_in_short_display = font;
                }
                print_in(st, st.mem.b16s0(p) as i32);
            }
        } else {
            match st.mem.classify(MemIdx::new(p)) {
                Ok(
                    AnyNode::HList(_)
                    | AnyNode::VList(_)
                    | AnyNode::Ins(_)
                    | AnyNode::Mark(_)
                    | AnyNode::Adjust(_)
                    | AnyNode::Unset(_),
                ) => print_cstr_in(st, c"[]".as_ptr()),
                Ok(AnyNode::Whatsit(w)) => {
                    if w.whatsit_subtype(&st.mem)
                        .is_some_and(|s| s.is_native_word())
                    {
                        let font = w.native_font(&st.mem).0;
                        if font != st.errors.font_in_short_display {
                            print_esc_in(st, st.eqtb.hash_s1(FONT_ID_BASE + font));
                            print_char_in(st, b' ' as i32);
                            st.errors.font_in_short_display = font;
                        }
                        print_native_word_in(st, w);
                    } else {
                        print_cstr_in(st, c"[]".as_ptr());
                    }
                }
                Ok(AnyNode::Rule(_)) => print_char_in(st, b'|' as i32),
                Ok(AnyNode::Glue(g)) => {
                    // Glue pointing at the shared zero spec prints nothing.
                    if g.glue_ptr(&st.mem) != ZERO_GLUE {
                        print_char_in(st, b' ' as i32);
                    }
                }
                Ok(AnyNode::Math(m)) => {
                    if m.subtype(&st.mem) >= L_CODE {
                        print_cstr_in(st, c"[]".as_ptr());
                    } else {
                        print_char_in(st, b'$' as i32);
                    }
                }
                Ok(AnyNode::Ligature(l)) => short_display_in(st, l.lig_ptr(&st.mem)),
                Ok(AnyNode::Disc(d)) => {
                    short_display_in(st, d.pre_break(&st.mem));
                    short_display_in(st, d.post_break(&st.mem));
                    for _ in 0..d.subtype(&st.mem) {
                        if st.mem.b32s1(p) != TEX_NULL {
                            p = st.mem.b32s1(p);
                        }
                    }
                }
                _ => {}
            }
        }
        p = st.mem.b32s1(p);
    }
}

/// The outer vertical list part of `show_activities`: the current page and its insertions.
fn show_page_so_far_in(st: &mut EngineState) {
    if st.shipout.page_tail != PAGE_HEAD {
        print_nl_cstr_in(st, c"### current page:".as_ptr());
        if st.shipout.output_active {
            print_cstr_in(st, c" (held over for next output)".as_ptr());
        }
        show_box_in(st, st.mem.b32s1(PAGE_HEAD));
        if st.shipout.page_contents != crate::state::PageContents::Empty {
            print_nl_cstr_in(st, c"total height ".as_ptr());
            print_totals_in(st);
            print_nl_cstr_in(st, c" goal height ".as_ptr());
            print_scaled_in(st, st.shipout.page_so_far[0]);
            let mut r = st.mem.link_ptr(PAGE_INS_HEAD);
            while r != PAGE_INS_HEAD {
                // SAFETY: the page_ins_head ring holds only page-insertion records.
                let ins = unsafe { NodePtr::<PageInsNode>::from_idx_unchecked(MemIdx::new(r)) };
                print_ln_in(st);
                esc(st, c"insert");
                let box_num = ins.subtype(&st.mem);
                print_int_in(st, box_num as i32);
                print_cstr_in(st, c" adds ".as_ptr());
                let count = st.eqtb.b32s1(COUNT_BASE + box_num as i32);
                let height = ins.height(&st.mem).0;
                let adds = if count == 1000 {
                    height
                } else {
                    crate::scaled_math::x_over_n_in(&mut st.errors, height, 1000) * count
                };
                print_scaled_in(st, adds);
                if ins.is_split_up(&st.mem) {
                    let broken_ins = ins.broken_ins(&st.mem);
                    let mut q = PAGE_HEAD;
                    let mut t = 0;
                    loop {
                        q = st.mem.link_ptr(q);
                        if let Ok(AnyNode::Ins(node)) = st.mem.classify(MemIdx::new(q)) {
                            if node.subtype(&st.mem) == box_num {
                                t += 1;
                            }
                        }
                        if q == broken_ins {
                            break;
                        }
                    }
                    print_cstr_in(st, c", #".as_ptr());
                    print_int_in(st, t);
                    print_cstr_in(st, c" might split".as_ptr());
                }
                r = r.link(&st.mem);
            }
        }
    }
    if st.mem.b32s1(CONTRIB_HEAD) != TEX_NULL {
        print_nl_cstr_in(st, c"### recent contributions:".as_ptr());
    }
}

pub(crate) fn show_activities_in(st: &mut EngineState) {
    st.input.nest[st.input.nest_ptr as usize] = st.input.cur_list;
    print_nl_cstr_in(st, c"".as_ptr());
    print_ln_in(st);
    for p in (0..=st.input.nest_ptr).rev() {
        let nest = st.input.nest[p as usize];
        let m = nest.mode;
        print_nl_cstr_in(st, c"### ".as_ptr());
        print_mode_in(st, m as i32);
        print_cstr_in(st, c" entered at line ".as_ptr());
        print_int_in(st, nest.mode_line.abs());
        if m == HMODE && nest.prev_graf != 0x83_0000 {
            let pg = nest.prev_graf;
            print_cstr_in(st, c" (language".as_ptr());
            print_int_in(st, pg % 65536);
            print_cstr_in(st, c":hyphenmin".as_ptr());
            print_int_in(st, pg / 0x40_0000);
            print_char_in(st, b',' as i32);
            print_int_in(st, (pg / 65536) % 64);
            print_char_in(st, b')' as i32);
        }
        if nest.mode_line < 0 {
            print_cstr_in(st, c" (\\output routine)".as_ptr());
        }
        if p == 0 {
            show_page_so_far_in(st);
        }
        show_box_in(st, st.mem.b32s1(nest.head));
        let (aux_s0, aux_s1) = (nest.aux_s0(), nest.aux_s1());
        match m.abs() as i32 / (MAX_COMMAND as i32 + 1) {
            0 => {
                print_nl_cstr_in(st, c"prevdepth ".as_ptr());
                if aux_s1 <= IGNORE_DEPTH {
                    print_cstr_in(st, c"ignored".as_ptr());
                } else {
                    print_scaled_in(st, aux_s1);
                }
                if nest.prev_graf != 0 {
                    print_cstr_in(st, c", prevgraf ".as_ptr());
                    print_int_in(st, nest.prev_graf);
                    if nest.prev_graf != 1 {
                        print_cstr_in(st, c" lines".as_ptr());
                    } else {
                        print_cstr_in(st, c" line".as_ptr());
                    }
                }
            }
            1 => {
                print_nl_cstr_in(st, c"spacefactor ".as_ptr());
                print_int_in(st, aux_s0);
                if m > 0 && aux_s1 > 0 {
                    print_cstr_in(st, c", current language ".as_ptr());
                    print_int_in(st, aux_s1);
                }
            }
            2 => {
                if aux_s1 != TEX_NULL {
                    print_cstr_in(st, c"this will begin denominator of:".as_ptr());
                    show_box_in(st, aux_s1);
                }
            }
            _ => {}
        }
    }
}

/// Shows sublist `list` with `c` appended to the indentation string.
fn node_list_display_in(st: &mut EngineState, c: u8, list: MemPtr) {
    st.strings.str_pool[st.strings.pool_ptr as usize] = c as u16;
    st.strings.pool_ptr += 1;
    show_node_list_in(st, list);
    st.strings.pool_ptr -= 1;
}

/// Shows the noad field at `p`, marked with `c` in the indentation string.
fn print_subsidiary_data_in(st: &mut EngineState, p: i32, c: u8) {
    if crate::hash::cur_length(&st.strings) >= st.errors.depth_threshold {
        if st.mem.b32s1(p) != EMPTY {
            print_cstr_in(st, c" []".as_ptr());
        }
        return;
    }
    st.strings.str_pool[st.strings.pool_ptr as usize] = c as u16;
    st.strings.pool_ptr += 1;
    st.mem.temp_ptr = MemPtr(p);
    match st.mem.b32s1(p) {
        MATH_CHAR => {
            print_ln_in(st);
            print_current_string_in(st);
            print_fam_and_char_in(st, p);
        }
        SUB_BOX => show_info_in(st),
        SUB_MLIST => {
            if st.mem.b32s0(p) == TEX_NULL {
                print_ln_in(st);
                print_current_string_in(st);
                print_cstr_in(st, c"{}".as_ptr());
            } else {
                show_info_in(st);
            }
        }
        _ => {}
    }
    st.strings.pool_ptr -= 1;
}

pub(crate) fn show_node_list_in(st: &mut EngineState, p: impl crate::units::MemAddr) {
    let mut p = p.addr();
    if crate::hash::cur_length(&st.strings) > st.errors.depth_threshold {
        if p > TEX_NULL {
            print_cstr_in(st, c" []".as_ptr());
        }
        return;
    }
    let mut n = 0;
    while p > 0 {
        print_ln_in(st);
        print_current_string_in(st);
        if p > st.mem.mem_end {
            print_cstr_in(st, c"Bad link, display aborted.".as_ptr());
            return;
        }
        n += 1;
        if n > st.errors.breadth_max {
            print_cstr_in(st, c"etc.".as_ptr());
            return;
        }
        if p >= st.mem.hi_mem_min {
            print_font_and_char_in(st, p);
        } else {
            match st.mem.classify(MemIdx::new(p)) {
                Ok(AnyNode::HList(b)) => show_box_node_in(st, AnyBox::HList(b)),
                Ok(AnyNode::VList(b)) => show_box_node_in(st, AnyBox::VList(b)),
                Ok(AnyNode::Unset(node)) => {
                    esc(st, c"unset");
                    print_cstr_in(st, c"box(".as_ptr());
                    print_scaled_in(st, node.height(&st.mem).0);
                    print_char_in(st, b'+' as i32);
                    print_scaled_in(st, node.depth(&st.mem).0);
                    print_cstr_in(st, c")x".as_ptr());
                    print_scaled_in(st, node.width(&st.mem).0);
                    let span_count = node.subtype(&st.mem);
                    if span_count != 0 {
                        print_cstr_in(st, c" (".as_ptr());
                        print_int_in(st, span_count as i32 + 1);
                        print_cstr_in(st, c" columns)".as_ptr());
                    }
                    if node.glue_stretch(&st.mem).0 != 0 {
                        print_cstr_in(st, c", stretch ".as_ptr());
                        let order = node.glue_order(&st.mem);
                        print_glue_in(st, node.glue_stretch(&st.mem).0, order, std::ptr::null());
                    }
                    if node.glue_shrink(&st.mem).0 != 0 {
                        print_cstr_in(st, c", shrink ".as_ptr());
                        let order = node.glue_sign(&st.mem);
                        print_glue_in(st, node.glue_shrink(&st.mem).0, order, std::ptr::null());
                    }
                    node_list_display_in(st, b'.', node.list_ptr(&st.mem));
                }
                Ok(AnyNode::Rule(node)) => {
                    esc(st, c"rule(");
                    print_rule_dimen_in(st, node.height(&st.mem).raw());
                    print_char_in(st, b'+' as i32);
                    print_rule_dimen_in(st, node.depth(&st.mem).raw());
                    print_cstr_in(st, c")x".as_ptr());
                    print_rule_dimen_in(st, node.width(&st.mem).raw());
                }
                Ok(AnyNode::Ins(node)) => {
                    esc(st, c"insert");
                    print_int_in(st, node.subtype(&st.mem) as i32);
                    print_cstr_in(st, c", natural size ".as_ptr());
                    print_scaled_in(st, node.height(&st.mem).0);
                    print_cstr_in(st, c"; split(".as_ptr());
                    print_spec_in(st, node.split_top_ptr(&st.mem).ptr(), std::ptr::null());
                    print_char_in(st, b',' as i32);
                    print_scaled_in(st, node.depth(&st.mem).0);
                    print_cstr_in(st, c"); float cost ".as_ptr());
                    print_int_in(st, node.float_cost(&st.mem));
                    node_list_display_in(st, b'.', node.ins_ptr(&st.mem));
                }
                Ok(AnyNode::Whatsit(node)) => show_whatsit_in(st, node),
                Ok(AnyNode::Glue(node)) => show_glue_in(st, node),
                Ok(AnyNode::Kern(node)) => {
                    let sub = node.subtype(&st.mem);
                    if sub != MU_GLUE {
                        esc(st, c"kern");
                        if sub != 0 {
                            print_char_in(st, b' ' as i32);
                        }
                        print_scaled_in(st, node.width(&st.mem).0);
                        if sub == ACC_KERN {
                            print_cstr_in(st, c" (for accent)".as_ptr());
                        } else if sub == SPACE_ADJUSTMENT {
                            print_cstr_in(st, c" (space adjustment)".as_ptr());
                        }
                    } else {
                        esc(st, c"mkern");
                        print_scaled_in(st, node.width(&st.mem).0);
                        print_cstr_in(st, c"mu".as_ptr());
                    }
                }
                Ok(AnyNode::MarginKern(node)) => {
                    esc(st, c"kern");
                    print_scaled_in(st, node.width(&st.mem).0);
                    if node.subtype(&st.mem) == 0 {
                        print_cstr_in(st, c" (left margin)".as_ptr());
                    } else {
                        print_cstr_in(st, c" (right margin)".as_ptr());
                    }
                }
                Ok(AnyNode::Math(node)) => {
                    let sub = node.subtype(&st.mem);
                    if sub > AFTER {
                        esc(st, if sub % 2 == 1 { c"end" } else { c"begin" });
                        let dir = if sub > R_CODE {
                            b'R'
                        } else if sub > L_CODE {
                            b'L'
                        } else {
                            b'M'
                        };
                        print_char_in(st, dir as i32);
                    } else {
                        esc(st, c"math");
                        if sub == 0 {
                            print_cstr_in(st, c"on".as_ptr());
                        } else {
                            print_cstr_in(st, c"off".as_ptr());
                        }
                        if node.width(&st.mem).0 != 0 {
                            print_cstr_in(st, c", surrounded ".as_ptr());
                            print_scaled_in(st, node.width(&st.mem).0);
                        }
                    }
                }
                Ok(AnyNode::Ligature(node)) => {
                    let font = node.font(&st.mem).0;
                    print_font_char_in(st, font, node.character(&st.mem));
                    print_cstr_in(st, c" (ligature ".as_ptr());
                    if node.subtype(&st.mem) > 1 {
                        print_char_in(st, b'|' as i32);
                    }
                    st.errors.font_in_short_display = font;
                    short_display_in(st, node.lig_ptr(&st.mem));
                    if node.subtype(&st.mem) % 2 == 1 {
                        print_char_in(st, b'|' as i32);
                    }
                    print_char_in(st, b')' as i32);
                }
                Ok(AnyNode::Penalty(node)) => {
                    esc(st, c"penalty ");
                    print_int_in(st, node.penalty(&st.mem));
                }
                Ok(AnyNode::Disc(node)) => {
                    esc(st, c"discretionary");
                    let replace_count = node.subtype(&st.mem);
                    if replace_count > 0 {
                        print_cstr_in(st, c" replacing ".as_ptr());
                        print_int_in(st, replace_count as i32);
                    }
                    node_list_display_in(st, b'.', node.pre_break(&st.mem));
                    node_list_display_in(st, b'|', node.post_break(&st.mem));
                }
                Ok(AnyNode::Mark(node)) => {
                    esc(st, c"mark");
                    let class = node.mark_class(&st.mem);
                    if class != 0 {
                        print_char_in(st, b's' as i32);
                        print_int_in(st, class);
                    }
                    print_mark_in(st, node.mark_ptr(&st.mem));
                }
                Ok(AnyNode::Adjust(node)) => {
                    esc(st, c"vadjust");
                    if node.subtype(&st.mem) != 0 {
                        print_cstr_in(st, c" pre ".as_ptr());
                    }
                    node_list_display_in(st, b'.', node.adjust_ptr(&st.mem));
                }
                Ok(AnyNode::Style(node)) => print_style_in(st, node.subtype(&st.mem) as i32),
                Ok(AnyNode::Choice(node)) => {
                    esc(st, c"mathchoice");
                    node_list_display_in(st, b'D', node.display_mlist(&st.mem));
                    node_list_display_in(st, b'T', node.text_mlist(&st.mem));
                    node_list_display_in(st, b'S', node.script_mlist(&st.mem));
                    node_list_display_in(st, b's', node.script_script_mlist(&st.mem));
                }
                Ok(AnyNode::Noad(node)) => show_noad_in(st, node),
                Err(_) => print_cstr_in(st, c"Unknown node type!".as_ptr()),
            }
        }
        p = st.mem.b32s1(p);
    }
}

fn show_box_node_in(st: &mut EngineState, b: AnyBox) {
    let is_hlist = matches!(b, AnyBox::HList(_));
    print_esc_in(st, if is_hlist { b'h' } else { b'v' } as i32);
    print_cstr_in(st, c"box(".as_ptr());
    print_scaled_in(st, b.height(&st.mem).0);
    print_char_in(st, b'+' as i32);
    print_scaled_in(st, b.depth(&st.mem).0);
    print_cstr_in(st, c")x".as_ptr());
    print_scaled_in(st, b.width(&st.mem).0);
    let g = b.glue_set(&st.mem);
    let glue_sign = b.glue_sign(&st.mem);
    if g != 0.0 && glue_sign != GlueSign::Normal {
        print_cstr_in(st, c", glue set ".as_ptr());
        if glue_sign == GlueSign::Shrinking {
            print_cstr_in(st, c"- ".as_ptr());
        }
        let order = b.glue_order(&st.mem);
        if g.abs() > 20000.0 {
            if g > 0.0 {
                print_char_in(st, b'>' as i32);
            } else {
                print_cstr_in(st, c"< -".as_ptr());
            }
            print_glue_in(st, 20000 * 65536, order, std::ptr::null());
        } else {
            let set = crate::scaled_math::tex_round(65536.0 * g);
            print_glue_in(st, set, order, std::ptr::null());
        }
    }
    if b.shift_amount(&st.mem).0 != 0 {
        print_cstr_in(st, c", shifted ".as_ptr());
        print_scaled_in(st, b.shift_amount(&st.mem).0);
    }
    if is_hlist && b.subtype(&st.mem) == DLIST {
        print_cstr_in(st, c", display".as_ptr());
    }
    node_list_display_in(st, b'.', b.list_ptr(&st.mem));
}

fn show_whatsit_in(st: &mut EngineState, node: NodePtr<WhatsitNode>) {
    let print_font = |st: &mut EngineState| {
        print_esc_in(
            st,
            st.eqtb.hash_s1(FONT_ID_BASE + node.native_font(&st.mem).0),
        );
    };
    match node.whatsit_subtype(&st.mem) {
        Some(WhatsitSubtype::Open) => {
            print_write_whatsit_in(st, c"openout", node);
            print_char_in(st, b'=' as i32);
            let (name, area, ext) = (
                node.open_name(&st.mem),
                node.open_area(&st.mem),
                node.open_ext(&st.mem),
            );
            print_file_name_in(st, name, area, ext);
        }
        Some(WhatsitSubtype::Write) => {
            print_write_whatsit_in(st, c"write", node);
            print_mark_in(st, node.write_tokens(&st.mem));
        }
        Some(WhatsitSubtype::Close) => print_write_whatsit_in(st, c"closeout", node),
        Some(WhatsitSubtype::Special) => {
            esc(st, c"special");
            print_mark_in(st, node.write_tokens(&st.mem));
        }
        Some(WhatsitSubtype::Language) => {
            esc(st, c"setlanguage");
            print_int_in(st, node.what_lang(&st.mem));
            print_cstr_in(st, c" (hyphenmin ".as_ptr());
            print_int_in(st, node.what_lhm(&st.mem) as i32);
            print_char_in(st, b',' as i32);
            print_int_in(st, node.what_rhm(&st.mem) as i32);
            print_char_in(st, b')' as i32);
        }
        Some(WhatsitSubtype::PdfSavePos) => esc(st, c"pdfsavepos"),
        Some(WhatsitSubtype::NativeWord | WhatsitSubtype::NativeWordAt) => {
            print_font(st);
            print_char_in(st, b' ' as i32);
            print_native_word_in(st, node);
        }
        Some(WhatsitSubtype::Glyph) => {
            print_font(st);
            print_cstr_in(st, c" glyph#".as_ptr());
            print_int_in(st, node.native_glyph(&st.mem) as i32);
        }
        Some(sub @ (WhatsitSubtype::Pic | WhatsitSubtype::Pdf)) => {
            if sub == WhatsitSubtype::Pic {
                esc(st, c"XeTeXpicfile");
            } else {
                esc(st, c"XeTeXpdffile");
            }
            print_cstr_in(st, c"( ".as_ptr());
            for i in 0..node.pic_path_len(&st.mem) as usize {
                let b = node.pic_path(&st.mem)[i];
                print_raw_char_in(st, b as u16, true);
            }
            print_in(st, b'"' as i32);
        }
        _ => print_cstr_in(st, c"whatsit?".as_ptr()),
    }
}

fn show_glue_in(st: &mut EngineState, node: NodePtr<crate::node::GlueNode>) {
    let sub = node.subtype(&st.mem);
    if sub >= A_LEADERS {
        esc(st, c"");
        if sub == C_LEADERS {
            print_char_in(st, b'c' as i32);
        } else if sub == X_LEADERS {
            print_char_in(st, b'x' as i32);
        }
        print_cstr_in(st, c"leaders ".as_ptr());
        print_spec_in(st, node.glue_ptr(&st.mem).ptr(), std::ptr::null());
        node_list_display_in(st, b'.', node.leader_ptr(&st.mem));
        return;
    }
    esc(st, c"glue");
    if sub != 0 {
        print_char_in(st, b'(' as i32);
        if sub < COND_MATH_GLUE {
            print_skip_param_in(st, sub as i32 - 1);
        } else if sub == COND_MATH_GLUE {
            esc(st, c"nonscript");
        } else {
            esc(st, c"mskip");
        }
        print_char_in(st, b')' as i32);
    }
    if sub != COND_MATH_GLUE {
        print_char_in(st, b' ' as i32);
        let unit = if sub < COND_MATH_GLUE {
            std::ptr::null()
        } else {
            c"mu".as_ptr()
        };
        print_spec_in(st, node.glue_ptr(&st.mem).ptr(), unit);
    }
}

fn show_noad_in(st: &mut EngineState, node: NodePtr<crate::node::Noad>) {
    let nt = node.noad_type(&st.mem);
    if nt == NodeType::FractionNoad {
        esc(st, c"fraction, thickness ");
        let thickness = node.thickness(&st.mem);
        if thickness == DEFAULT_CODE {
            print_cstr_in(st, c"= default".as_ptr());
        } else {
            print_scaled_in(st, thickness);
        }
        let left = node.fraction_left_delimiter_addr();
        if !delimiter_is_null(st, left) {
            print_cstr_in(st, c", left-delimiter ".as_ptr());
            print_delimiter_in(st, left);
        }
        let right = node.fraction_right_delimiter_addr();
        if !delimiter_is_null(st, right) {
            print_cstr_in(st, c", right-delimiter ".as_ptr());
            print_delimiter_in(st, right);
        }
        print_subsidiary_data_in(st, node.numerator_addr(), b'\\');
        print_subsidiary_data_in(st, node.denominator_addr(), b'/');
        return;
    }

    let name = match nt {
        NodeType::OrdNoad => c"mathord",
        NodeType::OpNoad => c"mathop",
        NodeType::BinNoad => c"mathbin",
        NodeType::RelNoad => c"mathrel",
        NodeType::OpenNoad => c"mathopen",
        NodeType::CloseNoad => c"mathclose",
        NodeType::PunctNoad => c"mathpunct",
        NodeType::InnerNoad => c"mathinner",
        NodeType::OverNoad => c"overline",
        NodeType::UnderNoad => c"underline",
        NodeType::VcenterNoad => c"vcenter",
        NodeType::RadicalNoad => c"radical",
        NodeType::AccentNoad => c"accent",
        NodeType::LeftNoad => c"left",
        NodeType::RightNoad if node.subtype(&st.mem) == 0 => c"right",
        NodeType::RightNoad => c"middle",
        _ => c"",
    };
    if !name.is_empty() {
        esc(st, name);
    }
    match nt {
        NodeType::RadicalNoad => print_delimiter_in(st, node.delimiter_addr()),
        NodeType::AccentNoad => print_fam_and_char_in(st, node.accent_char_addr()),
        NodeType::LeftNoad | NodeType::RightNoad => {
            print_delimiter_in(st, node.left_right_delimiter_addr())
        }
        _ => {}
    }
    if !matches!(nt, NodeType::LeftNoad | NodeType::RightNoad) {
        let sub = node.subtype(&st.mem);
        if sub != 0 {
            esc(
                st,
                if sub == LIMITS as u16 {
                    c"limits"
                } else {
                    c"nolimits"
                },
            );
        }
        print_subsidiary_data_in(st, node.nucleus_addr(), b'.');
    }
    print_subsidiary_data_in(st, node.supscr_addr(), b'^');
    print_subsidiary_data_in(st, node.subscr_addr(), b'_');
}

/// The label shown for a token list of type `index` in the error context.
fn token_list_label(index: u16, loc: i32) -> Option<&'static CStr> {
    Some(match index {
        PARAMETER => c"<argument> ",
        U_TEMPLATE | V_TEMPLATE => c"<template> ",
        BACKED_UP | BACKED_UP_CHAR if loc == TEX_NULL => c"<recently read> ",
        BACKED_UP | BACKED_UP_CHAR => c"<to be read again> ",
        INSERTED => c"<inserted text> ",
        OUTPUT_TEXT => c"<output> ",
        EVERY_PAR_TEXT => c"<everypar> ",
        EVERY_MATH_TEXT => c"<everymath> ",
        EVERY_DISPLAY_TEXT => c"<everydisplay> ",
        EVERY_HBOX_TEXT => c"<everyhbox> ",
        EVERY_VBOX_TEXT => c"<everyvbox> ",
        EVERY_JOB_TEXT => c"<everyjob> ",
        EVERY_CR_TEXT => c"<everycr> ",
        MARK_TEXT => c"<mark> ",
        EVERY_EOF_TEXT => c"<everyeof> ",
        INTER_CHAR_TEXT => c"<XeTeXinterchartoks> ",
        WRITE_TEXT => c"<write> ",
        TECTONIC_CODA_TEXT => c"<TectonicCodaTokens> ",
        _ => return None,
    })
}

/// Switches to pseudoprinting, returning how much of the line was already printed.
fn begin_pseudoprint(st: &mut EngineState) -> i32 {
    let l = st.errors.tally;
    st.errors.tally = 0;
    st.shipout.selector = SELECTOR_PSEUDO;
    st.errors.trick_count = 1_000_000;
    l
}

pub(crate) fn show_context_in(st: &mut EngineState) {
    st.input.base_ptr = st.input.input_ptr;
    st.input.input_stack[st.input.base_ptr as usize] = st.input.cur_input;
    let mut nn = -1;
    let mut bottom_line = false;

    loop {
        st.input.cur_input = st.input.input_stack[st.input.base_ptr as usize];
        st.mem.cur_synctex_tag = st.input.cur_input.synctex_tag;
        let ci = st.input.cur_input;
        if ci.state != TOKEN_LIST && (ci.name > 19 || st.input.base_ptr == 0) {
            bottom_line = true;
        }
        let error_context_lines = st.eqtb.int_par(INT_PAR_ERROR_CONTEXT_LINES);
        if st.input.base_ptr == st.input.input_ptr || bottom_line || nn < error_context_lines {
            if st.input.base_ptr == st.input.input_ptr
                || ci.state != TOKEN_LIST
                || ci.index != BACKED_UP
                || ci.loc != TEX_NULL
            {
                st.errors.tally = 0;
                let old_setting = st.shipout.selector;
                let l;
                if ci.state != TOKEN_LIST {
                    if ci.name <= 17 {
                        if ci.name == 0 {
                            if st.input.base_ptr == 0 {
                                print_nl_cstr_in(st, c"<*>".as_ptr());
                            } else {
                                print_nl_cstr_in(st, c"<insert> ".as_ptr());
                            }
                        } else {
                            print_nl_cstr_in(st, c"<read ".as_ptr());
                            if ci.name == 17 {
                                print_char_in(st, b'*' as i32);
                            } else {
                                print_int_in(st, ci.name - 1);
                            }
                            print_char_in(st, b'>' as i32);
                        }
                    } else {
                        print_nl_cstr_in(st, c"l.".as_ptr());
                        if ci.index as i32 == st.input.in_open {
                            print_int_in(st, st.mem.line);
                        } else {
                            print_int_in(st, st.input.line_stack[ci.index as usize + 1]);
                        }
                    }
                    print_char_in(st, b' ' as i32);
                    l = begin_pseudoprint(st);
                    let end_line_char = st.eqtb.int_par(INT_PAR_END_LINE_CHAR);
                    let j = if st.input.buffer[ci.limit as usize] == end_line_char {
                        ci.limit
                    } else {
                        ci.limit + 1
                    };
                    for i in ci.start..j {
                        if i == ci.loc {
                            set_trick_count(st);
                        }
                        print_char_in(st, st.input.buffer[i as usize]);
                    }
                } else {
                    match ci.index {
                        MACRO => {
                            print_ln_in(st);
                            print_cs_in(st, ci.name);
                        }
                        index => match token_list_label(index, ci.loc) {
                            Some(label) => print_nl_cstr_in(st, label.as_ptr()),
                            None => print_nl_in(st, b'?' as i32),
                        },
                    }
                    l = begin_pseudoprint(st);
                    if ci.index < MACRO {
                        show_token_list_in(st, ci.start, ci.loc, 100_000);
                    } else {
                        show_token_list_in(st, st.mem.b32s1(ci.start), ci.loc, 100_000);
                    }
                }
                st.shipout.selector = old_setting;
                if st.errors.trick_count == 1_000_000 {
                    set_trick_count(st);
                }
                let m = if st.errors.tally < st.errors.trick_count {
                    st.errors.tally - st.errors.first_count
                } else {
                    st.errors.trick_count - st.errors.first_count
                };
                let first_count = st.errors.first_count;
                let error_line = st.errors.error_line;
                let (start, n) = if l + first_count <= st.errors.half_error_line {
                    (0, l + first_count)
                } else {
                    print_cstr_in(st, c"...".as_ptr());
                    (
                        l + first_count - st.errors.half_error_line + 3,
                        st.errors.half_error_line,
                    )
                };
                for q in start..first_count {
                    print_char_in(st, st.errors.trick_buf[(q % error_line) as usize] as i32);
                }
                print_ln_in(st);
                for _ in 0..n {
                    print_raw_char_in(st, b' ' as u16, true);
                }
                let end = if m + n <= error_line {
                    first_count + m
                } else {
                    first_count + (error_line - n - 3)
                };
                for q in first_count..end {
                    print_char_in(st, st.errors.trick_buf[(q % error_line) as usize] as i32);
                }
                if m + n > error_line {
                    print_cstr_in(st, c"...".as_ptr());
                }
                nn += 1;
            }
        } else if nn == error_context_lines {
            print_nl_cstr_in(st, c"...".as_ptr());
            nn += 1;
        }
        if bottom_line {
            break;
        }
        st.input.base_ptr -= 1;
    }
    st.input.cur_input = st.input.input_stack[st.input.input_ptr as usize];
    st.mem.cur_synctex_tag = st.input.cur_input.synctex_tag;
}
