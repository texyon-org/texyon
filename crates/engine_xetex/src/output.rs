use crate::bridge::{
    diag_append, diag_append_byte, diag_begin_error, diag_begin_warning, diag_finish,
    issue_warning, output_flush, output_putc, Diagnostic,
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
    COND_MATH_GLUE, COUNT_BASE, CS_TOKEN_FLAG, C_LEADERS, DEL_CODE_BASE, DIMEN_BASE, DLIST, EMPTY,
    END_L_CODE, EQTB_SIZE, ETEX_PEN_BASE, FILLL, FROZEN_NULL_FONT, GLUE_BASE, HASH_BASE,
    HISTORY_SPOTLESS, HISTORY_WARNING_ISSUED, HMODE, IGNORE_DEPTH, INT_BASE, LC_CODE_BASE,
    LOCAL_BASE, LOCAL_PAR_SHAPE, L_CODE, MATH_CHAR, MATH_CODE_BASE, MATH_FONT_BASE, MMODE, MU_GLUE,
    MU_SKIP_BASE, NORMAL, NULL_CS, PRIM_EQTB_BASE, R_CODE, SCALED_BASE, SCRIPT_SIZE,
    SELECTOR_LOG_ONLY, SELECTOR_NEW_STRING, SELECTOR_NO_PRINT, SELECTOR_TERM_AND_LOG,
    SELECTOR_TERM_ONLY, SF_CODE_BASE, SINGLE_BASE, SKIP_BASE, SPACE_ADJUSTMENT, SUB_BOX, SUB_MLIST,
    TEXT_SIZE, TEX_NULL, TOKS_BASE, TOO_BIG_CHAR, UC_CODE_BASE, UNDEFINED_CONTROL_SEQUENCE,
    UNLESS_CODE, VMODE, X_LEADERS,
};
use crate::errors::error;
use crate::errors::Abort;
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
    AnyBox, AnyNode, CharNode, CondNode, DelimiterField, GlueSpecNode, MathField, NodePtr,
    PageInsNode, RuleDimen, SaNode, WhatsitNode, WhatsitSubtype,
};
use crate::packaging::EXACTLY;
use crate::params::{DimenPar, GluePar, IntPar};
use crate::save::DIMEN_VAL_LIMIT;
use crate::scan::{
    CURRENT_GROUP_LEVEL_CODE, CURRENT_GROUP_TYPE_CODE, CURRENT_IF_BRANCH_CODE,
    CURRENT_IF_LEVEL_CODE, CURRENT_IF_TYPE_CODE, DIMEN_VAL, ELAPSED_TIME_CODE, ETEX_EXPR,
    ETEX_VERSION_CODE, FONT_CHAR_DP_CODE, FONT_CHAR_HT_CODE, FONT_CHAR_IC_CODE, FONT_CHAR_WD_CODE,
    GLUE_SHRINK_CODE, GLUE_SHRINK_ORDER_CODE, GLUE_STRETCH_CODE, GLUE_STRETCH_ORDER_CODE,
    GLUE_TO_MU_CODE, GLUE_VAL, HEIGHT_OFFSET, INPUT_LINE_NO_CODE, INT_VAL, LAST_NODE_TYPE_CODE,
    MAX_CHAR_VAL, MU_TO_GLUE_CODE, MU_VAL, PAR_SHAPE_DIMEN_CODE, PAR_SHAPE_INDENT_CODE,
    PAR_SHAPE_LENGTH_CODE, PDF_LAST_X_POS_CODE, PDF_LAST_Y_POS_CODE, PDF_SHELL_ESCAPE_CODE,
    RANDOM_SEED_CODE, WIDTH_OFFSET, XETEX_COUNT_FEATURES_CODE, XETEX_COUNT_GLYPHS_CODE,
    XETEX_COUNT_SELECTORS_CODE, XETEX_COUNT_VARIATIONS_CODE, XETEX_FEATURE_CODE_CODE,
    XETEX_FIND_FEATURE_BY_NAME_CODE, XETEX_FIND_SELECTOR_BY_NAME_CODE,
    XETEX_FIND_VARIATION_BY_NAME_CODE, XETEX_FIRST_CHAR_CODE, XETEX_FONT_TYPE_CODE,
    XETEX_GLYPH_BOUNDS_CODE, XETEX_GLYPH_INDEX_CODE, XETEX_IS_DEFAULT_SELECTOR_CODE,
    XETEX_IS_EXCLUSIVE_FEATURE_CODE, XETEX_LAST_CHAR_CODE, XETEX_MAP_CHAR_TO_GLYPH_CODE,
    XETEX_OT_COUNT_FEATURES_CODE, XETEX_OT_COUNT_LANGUAGES_CODE, XETEX_OT_COUNT_SCRIPTS_CODE,
    XETEX_OT_FEATURE_CODE, XETEX_OT_LANGUAGE_CODE, XETEX_OT_SCRIPT_CODE, XETEX_PDF_PAGE_COUNT_CODE,
    XETEX_SELECTOR_CODE_CODE, XETEX_VARIATION_CODE, XETEX_VARIATION_DEFAULT_CODE,
    XETEX_VARIATION_MAX_CODE, XETEX_VARIATION_MIN_CODE, XETEX_VERSION_CODE,
};
use crate::state::{EngineState, GroupCode, InteractionMode};
use crate::units::{FontId, GlueSign, MemIdx, MemPtr, NodeType, Scaled, StrNumber};

const SELECTOR_PSEUDO: i32 = 20;

fn print_the_digs(st: &mut EngineState, mut k: u8) {
    while k > 0 {
        k -= 1;
        let d = i32::from(st.errors.dig[k as usize]);
        if d < 10 {
            print_char(st, i32::from(b'0') + d);
        } else {
            print_char(st, i32::from(b'A') - 10 + d);
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

pub(crate) fn print_int(st: &mut EngineState, mut n: i32) {
    let mut k: u8 = 0;
    if n < 0 {
        print_char(st, b'-');
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
    print_the_digs(st, k);
}

pub(crate) fn print_hex(st: &mut EngineState, n: i32) {
    print_char(st, b'"');
    let k = store_digits(st, n, 16, 0);
    print_the_digs(st, k);
}

pub(crate) fn print_ucs_code(st: &mut EngineState, n: i32) {
    print(st, "U+");
    let mut k = store_digits(st, n, 16, 0);
    while k < 4 {
        st.errors.dig[k as usize] = 0;
        k += 1;
    }
    print_the_digs(st, k);
}

/// A printable TeX string: a pool string number, or literal bytes printed one
/// `print_char` each.
pub(crate) trait TexStr: Copy {
    fn print_to(self, st: &mut EngineState);
}

impl TexStr for i32 {
    #[inline]
    fn print_to(self, st: &mut EngineState) {
        print_pool_str(st, self);
    }
}

impl TexStr for StrNumber {
    #[inline]
    fn print_to(self, st: &mut EngineState) {
        print_pool_str(st, self.0);
    }
}

/// A byte prints as the single-character string of that code.
impl TexStr for u8 {
    #[inline]
    fn print_to(self, st: &mut EngineState) {
        print_pool_str(st, i32::from(self));
    }
}

impl TexStr for &str {
    #[inline]
    fn print_to(self, st: &mut EngineState) {
        print_bytes(st, self.as_bytes());
    }
}

impl TexStr for &[u8] {
    #[inline]
    fn print_to(self, st: &mut EngineState) {
        print_bytes(st, self);
    }
}

fn print_bytes(st: &mut EngineState, bytes: &[u8]) {
    for &b in bytes {
        print_char(st, i32::from(b));
    }
}

fn at_line_start(st: &EngineState) -> bool {
    let sel = st.shipout.selector;
    !((st.shipout.term_offset > 0 && sel & 1 != 0)
        || (st.shipout.file_offset > 0 && sel >= SELECTOR_LOG_ONLY))
}

pub(crate) fn print_nl(st: &mut EngineState, s: impl TexStr) {
    if !at_line_start(st) {
        print_ln(st);
    }
    print(st, s);
}

fn print_escape_char(st: &mut EngineState) {
    let c = st.eqtb.int_par(IntPar::EscapeChar);
    if (0..=BIGGEST_USV).contains(&c) {
        print_char(st, c);
    }
}

pub(crate) fn print_esc(st: &mut EngineState, s: impl TexStr) {
    print_escape_char(st);
    print(st, s);
}

pub(crate) fn print_roman_int(st: &mut EngineState, mut n: i32) {
    // Letter/divisor pairs: each letter is the previous value divided by the digit after it.
    const ROMAN_DATA: &[u8] = b"m2d5c2l5x2v5i";
    let mut j: usize = 0;
    let mut v: i32 = 1000;
    loop {
        while n >= v {
            print_char(st, i32::from(ROMAN_DATA[j]));
            n -= v;
        }
        if n <= 0 {
            return;
        }
        let mut k = j + 2;
        let mut u = v / (i32::from(ROMAN_DATA[k - 1]) - i32::from(b'0'));
        if ROMAN_DATA[k - 1] == b'2' {
            k += 2;
            u /= i32::from(ROMAN_DATA[k - 1]) - i32::from(b'0');
        }
        if n + u >= v {
            print_char(st, i32::from(ROMAN_DATA[k]));
            n += u;
        } else {
            j += 2;
            v /= i32::from(ROMAN_DATA[j - 1]) - i32::from(b'0');
        }
    }
}

pub(crate) fn print_scaled(st: &mut EngineState, s: Scaled) {
    let mut s = s.0;
    if s < 0 {
        print_char(st, b'-');
        s = -s;
    }
    print_int(st, s / 0x10000);
    print_char(st, b'.');
    s = 10 * (s % 0x10000) + 5;
    let mut delta: i32 = 10;
    loop {
        if delta > 0x10000 {
            s = s + 0x8000 - 50000;
        }
        print_char(st, i32::from(b'0') + s / 0x10000);
        s = 10 * (s % 0x10000);
        delta *= 10;
        if s <= delta {
            break;
        }
    }
}

fn warn_char(st: &mut EngineState, c: i32) {
    if let Some(diag) = &mut st.shipout.current_diagnostic {
        diag_append_byte(diag, c as u8);
    }
}

pub(crate) fn print_ln(st: &mut EngineState) {
    match st.shipout.selector {
        sel @ (SELECTOR_TERM_ONLY | SELECTOR_LOG_ONLY | SELECTOR_TERM_AND_LOG) => {
            warn_char(st, i32::from(b'\n'));
            if sel != SELECTOR_LOG_ONLY {
                output_putc(st.shipout.rust_stdout, b'\n');
            }
            if sel != SELECTOR_TERM_ONLY {
                output_putc(st.shipout.log_file, b'\n');
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
            output_putc(st.shipout.write_file[sel as usize], b'\n');
            st.shipout.write_prefix[sel as usize].push(i32::from(b'\n'));
        }
    }
}

/// Writes one byte, UTF-16 code unit or (when pseudoprinting) character to the current selector.
pub(crate) fn print_raw_char(st: &mut EngineState, s: impl Into<i32>, incr_offset: bool) {
    let s = s.into();
    match st.shipout.selector {
        sel @ (SELECTOR_TERM_ONLY | SELECTOR_LOG_ONLY | SELECTOR_TERM_AND_LOG) => {
            let to_term = sel != SELECTOR_LOG_ONLY;
            let to_log = sel != SELECTOR_TERM_ONLY;
            warn_char(st, s);
            if to_term {
                output_putc(st.shipout.rust_stdout, s);
            }
            if to_log {
                output_putc(st.shipout.log_file, s);
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
                output_putc(st.shipout.rust_stdout, b'\n');
                st.shipout.term_offset = 0;
            }
            if to_log && st.shipout.file_offset == st.config.max_print_line {
                output_putc(st.shipout.log_file, b'\n');
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
                st.strings.str_pool[pp as usize] = s as u16;
                st.strings.pool_ptr = pp + 1;
            }
        }
        sel => {
            output_putc(st.shipout.write_file[sel as usize], s);
            st.shipout.write_prefix[sel as usize].push(s);
        }
    }
    st.errors.tally += 1;
}

pub(crate) fn print_char(st: &mut EngineState, s: impl Into<i32>) {
    let s = s.into();
    if st.shipout.selector > SELECTOR_PSEUDO && !st.shipout.doing_special {
        if s >= 0x10000 {
            print_raw_char(st, 0xD800 + (s - 0x10000) / 1024, true);
            print_raw_char(st, 0xDC00 + (s - 0x10000) % 1024, true);
        } else {
            print_raw_char(st, s, true);
        }
        return;
    }

    if s == st.eqtb.int_par(IntPar::NewLineChar) && st.shipout.selector < SELECTOR_PSEUDO {
        print_ln(st);
        return;
    }

    if s < 32 && !st.shipout.doing_special {
        print_raw_char(st, b'^', true);
        print_raw_char(st, b'^', true);
        print_raw_char(st, s + 64, true);
    } else if s < 127 {
        print_raw_char(st, s, true);
    } else if s == 127 {
        if !st.shipout.doing_special {
            print_raw_char(st, b'^', true);
            print_raw_char(st, b'^', true);
            print_raw_char(st, b'?', true);
        } else {
            print_raw_char(st, s, true);
        }
    } else if s < 160 && !st.shipout.doing_special {
        let hex_digit = |d: i32| {
            if d < 10 {
                i32::from(b'0') + d
            } else {
                i32::from(b'a') + d - 10
            }
        };
        print_raw_char(st, b'^', true);
        print_raw_char(st, b'^', true);
        print_raw_char(st, hex_digit((s % 256) / 16), true);
        print_raw_char(st, hex_digit(s % 16), true);
    } else if st.shipout.selector == SELECTOR_PSEUDO {
        print_raw_char(st, s, true);
    } else {
        // UTF-8, counting one column per character.
        if s < 2048 {
            print_raw_char(st, 192 + s / 64, false);
            print_raw_char(st, 128 + s % 64, true);
        } else if s < 0x10000 {
            print_raw_char(st, 224 + s / 4096, false);
            print_raw_char(st, 128 + (s % 4096) / 64, false);
            print_raw_char(st, 128 + s % 64, true);
        } else {
            print_raw_char(st, 240 + s / 0x40000, false);
            print_raw_char(st, 128 + (s % 0x40000) / 4096, false);
            print_raw_char(st, 128 + (s % 4096) / 64, false);
            print_raw_char(st, 128 + s % 64, true);
        }
    }
}

pub(crate) fn print(st: &mut EngineState, s: impl TexStr) {
    s.print_to(st);
}

fn print_pool_str(st: &mut EngineState, s: i32) {
    if s >= st.strings.str_ptr {
        return print(st, "???");
    }
    if s <= BIGGEST_CHAR {
        if s < 0 {
            return print(st, "???");
        }
        if st.shipout.selector > SELECTOR_PSEUDO {
            print_char(st, s);
            return;
        }
        if s == st.eqtb.int_par(IntPar::NewLineChar) && st.shipout.selector < SELECTOR_PSEUDO {
            print_ln(st);
            return;
        }
        let nl = st.eqtb.int_par(IntPar::NewLineChar);
        st.eqtb.set_int_par(IntPar::NewLineChar, -1);
        print_char(st, s);
        st.eqtb.set_int_par(IntPar::NewLineChar, nl);
        return;
    }

    let pool_idx = (s - TOO_BIG_CHAR) as usize;
    let mut i = st.strings.str_start[pool_idx] as usize;
    let end = st.strings.str_start[pool_idx + 1] as usize;
    while i < end {
        let c = st.strings.str_pool[i];
        if i + 1 < end {
            if let Some(pair) = surrogate_pair(c, st.strings.str_pool[i + 1]) {
                print_char(st, pair);
                i += 2;
                continue;
            }
        }
        print_char(st, i32::from(c));
        i += 1;
    }
}

fn is_high_surrogate(c: u16) -> bool {
    (0xD800..0xDC00).contains(&c)
}

fn surrogate_pair(hi: u16, lo: u16) -> Option<i32> {
    (is_high_surrogate(hi) && (0xDC00..0xE000).contains(&lo))
        .then(|| 0x10000 + (i32::from(hi) - 0xD800) * 1024 + (i32::from(lo) - 0xDC00))
}

fn print_current_string(st: &mut EngineState) {
    let start = st.strings.cur_string_start();
    for j in start..st.strings.pool_ptr {
        let c = st.strings.str_pool[j as usize];
        print_char(st, i32::from(c));
    }
}

/// Reports the diagnostic being captured and captures into `diagnostic` from now on.
pub(crate) fn capture_to_diagnostic(st: &mut EngineState, diagnostic: Option<Box<Diagnostic>>) {
    if let Some(cur) = std::mem::replace(&mut st.shipout.current_diagnostic, diagnostic) {
        diag_finish(cur);
    }
}

/// Appends `text` to the diagnostic being captured.
pub(crate) fn append_to_diagnostic(st: &mut EngineState, text: &str) {
    if let Some(diag) = &mut st.shipout.current_diagnostic {
        diag_append(diag, text);
    }
}

/// The innermost input level that is a real file, with its current line.
fn current_file_line(st: &EngineState) -> Option<(StrNumber, i32)> {
    let mut level = st.input.in_open;
    while level > 0 && st.input.full_source_filename_stack[level as usize] == StrNumber::NONE {
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

fn diagnostic_print_file_line(st: &EngineState, diagnostic: &mut Diagnostic) {
    let Some((name, line)) = current_file_line(st) else {
        diag_append(diagnostic, "!");
        return;
    };
    let name = st.strings.texstring_bytes(name);
    let name = String::from_utf8_lossy(&name);
    diag_append(diagnostic, &format!("{name}:{line}: "));
}

pub(crate) fn diagnostic_begin_capture_warning_here(st: &mut EngineState) {
    let mut warning = diag_begin_warning();
    diagnostic_print_file_line(st, &mut warning);
    capture_to_diagnostic(st, Some(warning));
}

/// Starts an error with `message`, capturing what follows into its diagnostic.
pub(crate) fn error_here_with_diagnostic(st: &mut EngineState, message: &str) {
    let mut error = diag_begin_error();
    diagnostic_print_file_line(st, &mut error);
    diag_append(&mut error, message);
    print_error_prefix(st);
    print(st, message);
    capture_to_diagnostic(st, Some(error));
}

/// TeX's `print_err`: starts an error with `message` and closes its diagnostic.
pub(crate) fn print_err(st: &mut EngineState, message: &str) {
    error_here_with_diagnostic(st, message);
    capture_to_diagnostic(st, None);
}

/// Starts an error line with `! ` or, in file:line:error style, the source position.
pub(crate) fn print_error_prefix(st: &mut EngineState) {
    if st.config.file_line_error_style_p != 0 {
        print_file_line(st);
    } else {
        print_nl(st, "! ");
    }
}

pub(crate) fn print_file_line(st: &mut EngineState) {
    let Some((name, line)) = current_file_line(st) else {
        print_nl(st, "! ");
        return;
    };
    print_nl(st, "");
    print(st, name);
    print_char(st, b':');
    print_int(st, line);
    print(st, ": ");
}

pub(crate) fn print_size(st: &mut EngineState, s: i32) {
    print_esc(
        st,
        match s {
            TEXT_SIZE => "textfont",
            SCRIPT_SIZE => "scriptfont",
            _ => "scriptscriptfont",
        },
    );
}

fn print_write_whatsit(st: &mut EngineState, s: &str, p: NodePtr<WhatsitNode>) {
    print_esc(st, s);
    let v = p.write_stream(&st.mem);
    if v < 16 {
        print_int(st, v);
    } else if v == 16 {
        print_char(st, b'*');
    } else {
        print_char(st, b'-');
    }
}

pub(crate) fn print_file_name(st: &mut EngineState, n: StrNumber, a: StrNumber, e: StrNumber) {
    // Swaps one quote character for the other.
    const QUOTE_SUM: i32 = b'"' as i32 + b'\'' as i32;

    let mut must_quote = false;
    let mut quote_char: i32 = 0;
    for s in [a, n, e] {
        if s == StrNumber::NONE {
            continue;
        }
        for j in st.strings.str_range(s) {
            if must_quote && quote_char != 0 {
                break;
            }
            let c = i32::from(st.strings.str_pool[j]);
            if c == i32::from(b' ') {
                must_quote = true;
            } else if c == i32::from(b'"') || c == i32::from(b'\'') {
                must_quote = true;
                quote_char = QUOTE_SUM - c;
            }
        }
    }

    if must_quote {
        if quote_char == 0 {
            quote_char = i32::from(b'"');
        }
        print_char(st, quote_char);
    }

    for s in [a, n, e] {
        if s == StrNumber::NONE {
            continue;
        }
        for j in st.strings.str_range(s) {
            let c = i32::from(st.strings.str_pool[j]);
            if c == quote_char {
                print(st, quote_char);
                quote_char = QUOTE_SUM - quote_char;
                print(st, quote_char);
            }
            print(st, c);
        }
    }

    if quote_char != 0 {
        print_char(st, quote_char);
    }
}

/// The name string of a multi-letter control sequence at eqtb location `p`.
fn cs_text(st: &EngineState, p: i32) -> StrNumber {
    if (PRIM_EQTB_BASE..FROZEN_NULL_FONT).contains(&p) {
        StrNumber(st.eqtb.prim_s1(p - PRIM_EQTB_BASE) - 1)
    } else {
        st.eqtb.text(p)
    }
}

pub(crate) fn print_cs(st: &mut EngineState, p: i32) {
    if p < HASH_BASE {
        if p >= SINGLE_BASE {
            if p == NULL_CS {
                print_esc(st, "csname");
                print_esc(st, "endcsname");
                print_char(st, b' ');
            } else {
                print_esc(st, p - SINGLE_BASE);
                if st.eqtb.cat_code(p - SINGLE_BASE) == LETTER.into() {
                    print_char(st, b' ');
                }
            }
        } else if p < ACTIVE_BASE {
            print_esc(st, "IMPOSSIBLE.");
        } else {
            print_char(st, p - 1);
        }
    } else if (UNDEFINED_CONTROL_SEQUENCE..=EQTB_SIZE).contains(&p) || p > st.eqtb.eqtb_top {
        print_esc(st, "IMPOSSIBLE.");
    } else if st.eqtb.text(p).0 >= st.strings.str_ptr {
        print_esc(st, "NONEXISTENT.");
    } else {
        print_esc(st, cs_text(st, p));
        print_char(st, b' ');
    }
}

/// Like [`print_cs`] but without the trailing space.
pub(crate) fn sprint_cs(st: &mut EngineState, p: i32) {
    if p >= HASH_BASE {
        print_esc(st, cs_text(st, p));
    } else if p < SINGLE_BASE {
        print_char(st, p - 1);
    } else if p < NULL_CS {
        print_esc(st, p - SINGLE_BASE);
    } else {
        print_esc(st, "csname");
        print_esc(st, "endcsname");
    }
}

fn print_native_word(st: &mut EngineState, p: NodePtr<WhatsitNode>) {
    let len = p.native_length(&st.mem) as usize;
    let mut i = 0;
    while i < len {
        let c = p.native_text(&st.mem)[i];
        if i + 1 < len {
            if let Some(pair) = surrogate_pair(c, p.native_text(&st.mem)[i + 1]) {
                print_char(st, pair);
                i += 2;
                continue;
            }
        }
        if is_high_surrogate(c) {
            print(st, b'.');
        } else {
            print_char(st, i32::from(c));
        }
        i += 1;
    }
}

fn print_sa_num(st: &mut EngineState, q: NodePtr<SaNode>) {
    let mem = &st.mem;
    let n = if q.sa_index(mem) < DIMEN_VAL_LIMIT {
        q.sa_ptr(mem).0
    } else {
        let q1 = q.parent(mem);
        let q2 = q1.parent(mem);
        let q3 = q2.parent(mem);
        let digit = |q: NodePtr<SaNode>| i32::from(q.sa_index(mem));
        digit(q) % 64 + 64 * digit(q1) + 64 * 64 * (digit(q2) + 64 * digit(q3))
    };
    print_int(st, n);
}

pub(crate) fn begin_diagnostic(st: &mut EngineState) {
    st.shipout.old_setting = st.shipout.selector as u8;
    if st.eqtb.int_par(IntPar::TracingOnline) <= 0 && st.shipout.selector == SELECTOR_TERM_AND_LOG {
        st.shipout.selector -= 1;
        note_warning_issued(st);
    }
}

pub(crate) fn end_diagnostic(st: &mut EngineState, blank_line: bool) {
    print_nl(st, "");
    if blank_line {
        print_ln(st);
    }
    st.shipout.selector = i32::from(st.shipout.old_setting);
}

fn note_warning_issued(st: &mut EngineState) {
    if st.errors.history == HISTORY_SPOTLESS {
        st.errors.history = HISTORY_WARNING_ISSUED;
    }
}

fn mode_name(m: i32) -> Option<&'static str> {
    let group = i32::from(MAX_COMMAND) + 1;
    match m.signum() {
        0 => Some("no mode"),
        1 => ["vertical mode", "horizontal mode", "display math mode"]
            .get((m / group) as usize)
            .copied(),
        _ => [
            "internal vertical mode",
            "restricted horizontal mode",
            "math mode",
        ]
        .get((-m / group) as usize)
        .copied(),
    }
}

pub(crate) fn print_mode(st: &mut EngineState, m: i32) {
    if let Some(name) = mode_name(m) {
        print(st, name);
    }
}

pub(crate) fn print_in_mode(st: &mut EngineState, m: i32) {
    if let Some(name) = mode_name(m) {
        print(st, "' in ");
        print(st, name);
    }
}

pub(crate) fn print_group(st: &mut EngineState, e: bool) {
    let name = match st.save_stack.cur_group {
        GroupCode::BottomLevel => {
            print(st, "bottom level");
            return;
        }
        GroupCode::Simple => "simple",
        GroupCode::SemiSimple => "semi simple",
        GroupCode::Hbox => "hbox",
        GroupCode::AdjustedHbox => "adjusted hbox",
        GroupCode::Vbox => "vbox",
        GroupCode::Vtop => "vtop",
        GroupCode::Align => "align",
        GroupCode::NoAlign => "no align",
        GroupCode::Output => "output",
        GroupCode::Disc => "disc",
        GroupCode::Insert => "insert",
        GroupCode::Vcenter => "vcenter",
        GroupCode::Math => "math",
        GroupCode::MathChoice => "math choice",
        GroupCode::MathShift => "math shift",
        GroupCode::MathLeft => "math left",
    };
    print(st, name);
    print(st, " group (level ");
    print_int(st, i32::from(st.save_stack.cur_level));
    print_char(st, b')');
    let line_at = st.save_stack.saved(-1);
    if line_at != 0 {
        if e {
            print(st, " entered at line ");
        } else {
            print(st, " at line ");
        }
        print_int(st, line_at);
    }
}

pub(crate) fn token_show(st: &mut EngineState, p: impl crate::units::MemAddr) {
    let p = MemPtr(p.addr());
    if !p.is_null() {
        let head = p.link(&st.mem);
        show_token_list(st, head, TEX_NULL, 10_000_000);
    }
}

pub(crate) fn print_spec(st: &mut EngineState, p: impl crate::units::MemAddr, s: Option<&str>) {
    let p = p.addr();
    let spec = (0..st.mem.lo_mem_max)
        .contains(&p)
        .then(|| NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr(p)));
    let Some(gs) = spec else {
        print_char(st, b'*');
        return;
    };
    print_scaled(st, gs.width(&st.mem));
    if let Some(s) = s {
        print(st, s);
    }
    if gs.stretch(&st.mem) != Scaled::ZERO {
        print(st, " plus ");
        print_glue(st, gs.stretch(&st.mem), gs.stretch_order(&st.mem), s);
    }
    if gs.shrink(&st.mem) != Scaled::ZERO {
        print(st, " minus ");
        print_glue(st, gs.shrink(&st.mem), gs.shrink_order(&st.mem), s);
    }
}

pub(crate) fn print_skip_param(st: &mut EngineState, n: i32) {
    match GluePar::from_raw(n) {
        Some(g) => print_esc(st, g.name()),
        None => print(st, "[unknown glue parameter!]"),
    }
}

pub(crate) fn print_param(st: &mut EngineState, n: i32) {
    match IntPar::from_raw(n).and_then(IntPar::name) {
        Some(name) => print_esc(st, name),
        None => print(st, "[unknown integer parameter!]"),
    }
}

pub(crate) fn print_length_param(st: &mut EngineState, n: i32) {
    match DimenPar::from_raw(n) {
        Some(d) => print_esc(st, d.name()),
        None => print(st, "[unknown dimen parameter!]"),
    }
}

pub(crate) fn print_style(st: &mut EngineState, c: i32) {
    let name = match c / 2 {
        0 => "displaystyle",
        1 => "textstyle",
        2 => "scriptstyle",
        3 => "scriptscriptstyle",
        _ => return print(st, "Unknown style!"),
    };
    print_esc(st, name);
}

/// Prints a character code, as a string number when it is one.
fn print_code(st: &mut EngineState, c: i32) {
    if c < TOO_BIG_CHAR {
        print(st, c);
    } else {
        print_char(st, c);
    }
}

fn print_fam_and_char(st: &mut EngineState, p: NodePtr<MathField>) {
    print_esc(st, "fam");
    print_int(st, p.fam(&st.mem));
    print_char(st, b' ');
    print_code(st, p.full_char(&st.mem));
}

fn print_delimiter(st: &mut EngineState, p: NodePtr<DelimiterField>) {
    let (small_fam, small_char) = p.small(&st.mem);
    let (large_fam, large_char) = p.large(&st.mem);
    let a = ((i64::from(small_fam) * 256 + i64::from(small_char)) * 4096
        + i64::from(large_fam) * 256
        + i64::from(large_char)) as i32;
    if a < 0 {
        print_int(st, a);
    } else {
        print_hex(st, a);
    }
}

pub(crate) fn print_rule_dimen(st: &mut EngineState, d: RuleDimen) {
    match d {
        RuleDimen::Running => print_char(st, b'*'),
        RuleDimen::Set(d) => print_scaled(st, d),
    }
}

/// Prints `d` with its fil/fill/filll order, or the unit `s` for finite glue.
pub(crate) fn print_glue(st: &mut EngineState, d: Scaled, order: u16, s: Option<&str>) {
    print_scaled(st, d);
    if !(NORMAL..=FILLL).contains(&order) {
        print(st, "foul");
    } else if order > NORMAL {
        print(st, "fil");
        for _ in 1..order {
            print_char(st, b'l');
        }
    } else if let Some(s) = s {
        print(st, s);
    }
}

fn print_font_char(st: &mut EngineState, font: FontId, c: u16) {
    if font > st.fonts.font_max {
        print_char(st, b'*');
    } else {
        print_esc(st, st.eqtb.font_id_text(font));
    }
    print_char(st, b' ');
    print(st, i32::from(c));
}

fn print_font_and_char(st: &mut EngineState, p: MemPtr) {
    if p.0 > st.mem.mem_end {
        print_esc(st, "CLOBBERED.");
    } else {
        let ch = NodePtr::<CharNode>::at(p);
        print_font_char(st, ch.font(&st.mem), ch.character(&st.mem));
    }
}

fn print_mark(st: &mut EngineState, p: MemPtr) {
    print_char(st, b'{');
    if p.0 < st.mem.hi_mem_min || p.0 > st.mem.mem_end {
        print_esc(st, "CLOBBERED.");
    } else {
        let head = p.link(&st.mem);
        let l = st.config.max_print_line - 10;
        show_token_list(st, head, TEX_NULL, l);
    }
    print_char(st, b'}');
}

const SHOW_BOX_CODE: i32 = 1;
const SHOW_THE_CODE: i32 = 2;
const SHOW_LISTS: i32 = 3;
const SHOW_GROUPS: i32 = 4;
const SHOW_TOKENS: i32 = 5;
const SHOW_IFS: i32 = 6;

pub(crate) fn print_cmd_chr(st: &mut EngineState, cmd: u8, chr_code: i32) {
    let char_cmd = |st: &mut EngineState, s: &str| {
        print(st, s);
        print_code(st, chr_code);
    };

    match cmd {
        LEFT_BRACE => char_cmd(st, "begin-group character "),
        RIGHT_BRACE => char_cmd(st, "end-group character "),
        MATH_SHIFT => char_cmd(st, "math shift character "),
        MAC_PARAM => char_cmd(st, "macro parameter character "),
        SUP_MARK => char_cmd(st, "superscript character "),
        SUB_MARK => char_cmd(st, "subscript character "),
        ENDV => print(st, "end of alignment template"),
        SPACER => char_cmd(st, "blank space "),
        LETTER => char_cmd(st, "the letter "),
        OTHER_CHAR => char_cmd(st, "the character "),
        ASSIGN_GLUE | ASSIGN_MU_GLUE => {
            if chr_code < SKIP_BASE {
                print_skip_param(st, chr_code - GLUE_BASE);
            } else if chr_code < MU_SKIP_BASE {
                print_esc(st, "skip");
                print_int(st, chr_code - SKIP_BASE);
            } else {
                print_esc(st, "muskip");
                print_int(st, chr_code - MU_SKIP_BASE);
            }
        }
        ASSIGN_TOKS => {
            if chr_code >= TOKS_BASE {
                print_esc(st, "toks");
                print_int(st, chr_code - TOKS_BASE);
            } else {
                let name = match chr_code - LOCAL_BASE {
                    1 => "output",
                    2 => "everypar",
                    3 => "everymath",
                    4 => "everydisplay",
                    5 => "everyhbox",
                    6 => "everyvbox",
                    7 => "everyjob",
                    8 => "everycr",
                    10 => "everyeof",
                    11 => "XeTeXinterchartoks",
                    12 => "TectonicCodaTokens",
                    _ => "errhelp",
                };
                print_esc(st, name);
            }
        }
        ASSIGN_INT => {
            if chr_code < COUNT_BASE {
                print_param(st, chr_code - INT_BASE);
            } else {
                print_esc(st, "count");
                print_int(st, chr_code - COUNT_BASE);
            }
        }
        ASSIGN_DIMEN => {
            if chr_code < SCALED_BASE {
                print_length_param(st, chr_code - DIMEN_BASE);
            } else {
                print_esc(st, "dimen");
                print_int(st, chr_code - SCALED_BASE);
            }
        }
        ACCENT => print_esc(st, "accent"),
        ADVANCE => print_esc(st, "advance"),
        AFTER_ASSIGNMENT => print_esc(st, "afterassignment"),
        AFTER_GROUP => print_esc(st, "aftergroup"),
        ASSIGN_FONT_DIMEN => print_esc(st, "fontdimen"),
        BEGIN_GROUP => print_esc(st, "begingroup"),
        BREAK_PENALTY => print_esc(st, "penalty"),
        CHAR_NUM => print_esc(st, "char"),
        CS_NAME => print_esc(st, "csname"),
        DEF_FONT => print_esc(st, "font"),
        DELIM_NUM => print_esc(
            st,
            if chr_code == 1 {
                "Udelimiter"
            } else {
                "delimiter"
            },
        ),
        DIVIDE => print_esc(st, "divide"),
        END_CS_NAME => print_esc(st, "endcsname"),
        END_GROUP => print_esc(st, "endgroup"),
        EX_SPACE => print_esc(st, b' '),
        EXPAND_AFTER => print_esc(
            st,
            if chr_code == 0 {
                "expandafter"
            } else {
                "unless"
            },
        ),
        HALIGN => print_esc(st, "halign"),
        HRULE => print_esc(st, "hrule"),
        IGNORE_SPACES => print_esc(
            st,
            if chr_code == 0 {
                "ignorespaces"
            } else {
                "primitive"
            },
        ),
        INSERT => print_esc(st, "insert"),
        ITAL_CORR => print_esc(st, b'/'),
        MARK => {
            print_esc(st, "mark");
            if chr_code > 0 {
                print_char(st, b's');
            }
        }
        MATH_ACCENT => print_esc(
            st,
            if chr_code == 1 {
                "Umathaccent"
            } else {
                "mathaccent"
            },
        ),
        MATH_CHAR_NUM => print_esc(
            st,
            match chr_code {
                2 => "Umathchar",
                1 => "Umathcharnum",
                _ => "mathchar",
            },
        ),
        MATH_CHOICE => print_esc(st, "mathchoice"),
        MULTIPLY => print_esc(st, "multiply"),
        NO_ALIGN => print_esc(st, "noalign"),
        NO_BOUNDARY => print_esc(st, "noboundary"),
        NO_EXPAND => print_esc(
            st,
            if chr_code == 0 {
                "noexpand"
            } else {
                "primitive"
            },
        ),
        NON_SCRIPT => print_esc(st, "nonscript"),
        OMIT => print_esc(st, "omit"),
        RADICAL => print_esc(st, if chr_code == 1 { "Uradical" } else { "radical" }),
        READ_TO_CS => print_esc(st, if chr_code == 0 { "read" } else { "readline" }),
        RELAX => print_esc(st, "relax"),
        SET_BOX => print_esc(st, "setbox"),
        SET_PREV_GRAF => print_esc(st, "prevgraf"),
        SET_SHAPE => {
            let name = if chr_code == LOCAL_BASE + LOCAL_PAR_SHAPE {
                "parshape"
            } else {
                match chr_code - ETEX_PEN_BASE {
                    0 => "interlinepenalties",
                    1 => "clubpenalties",
                    2 => "widowpenalties",
                    3 => "displaywidowpenalties",
                    _ => return,
                }
            };
            print_esc(st, name);
        }
        THE => print_esc(
            st,
            match chr_code {
                0 => "the",
                1 => "unexpanded",
                _ => "detokenize",
            },
        ),
        TOKS_REGISTER => {
            print_esc(st, "toks");
            if chr_code != 0 {
                if let Some(q) = NodePtr::<SaNode>::from_ptr(MemPtr(chr_code)) {
                    print_sa_num(st, q);
                }
            }
        }
        VADJUST => print_esc(st, "vadjust"),
        VALIGN => print_esc(
            st,
            match chr_code {
                0 => "valign",
                x if x == i32::from(BEGIN_L_CODE) => "beginL",
                x if x == i32::from(END_L_CODE) => "endL",
                x if x == i32::from(BEGIN_R_CODE) => "beginR",
                _ => "endR",
            },
        ),
        VCENTER => print_esc(st, "vcenter"),
        VRULE => print_esc(st, "vrule"),
        PAR_END => print_esc(st, "par"),
        INPUT => print_esc(
            st,
            match chr_code {
                0 => "input",
                2 => "scantokens",
                _ => "endinput",
            },
        ),
        TOP_BOT_MARK => {
            let name = match chr_code % MARKS_CODE {
                1 => "firstmark",
                2 => "botmark",
                3 => "splitfirstmark",
                4 => "splitbotmark",
                _ => "topmark",
            };
            print_esc(st, name);
            if chr_code >= MARKS_CODE {
                print_char(st, b's');
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
                INT_VAL => "count",
                DIMEN_VAL => "dimen",
                GLUE_VAL => "skip",
                _ => "muskip",
            };
            print_esc(st, name);
            if let Some(q) = sa {
                print_sa_num(st, q);
            }
        }
        SET_AUX => print_esc(
            st,
            if chr_code == i32::from(VMODE) {
                "prevdepth"
            } else {
                "spacefactor"
            },
        ),
        SET_PAGE_INT => print_esc(
            st,
            match chr_code {
                0 => "deadcycles",
                2 => "interactionmode",
                _ => "insertpenalties",
            },
        ),
        SET_BOX_DIMEN => print_esc(
            st,
            match chr_code {
                WIDTH_OFFSET => "wd",
                HEIGHT_OFFSET => "ht",
                _ => "dp",
            },
        ),
        LAST_ITEM => {
            let name = match chr_code {
                x if x == i32::from(INT_VAL) => "lastpenalty",
                x if x == i32::from(DIMEN_VAL) => "lastkern",
                x if x == i32::from(GLUE_VAL) => "lastskip",
                INPUT_LINE_NO_CODE => "inputlineno",
                LAST_NODE_TYPE_CODE => "lastnodetype",
                ETEX_VERSION_CODE => "eTeXversion",
                XETEX_VERSION_CODE => "XeTeXversion",
                XETEX_COUNT_GLYPHS_CODE => "XeTeXcountglyphs",
                XETEX_COUNT_VARIATIONS_CODE => "XeTeXcountvariations",
                XETEX_VARIATION_CODE => "XeTeXvariation",
                XETEX_FIND_VARIATION_BY_NAME_CODE => "XeTeXfindvariationbyname",
                XETEX_VARIATION_MIN_CODE => "XeTeXvariationmin",
                XETEX_VARIATION_MAX_CODE => "XeTeXvariationmax",
                XETEX_VARIATION_DEFAULT_CODE => "XeTeXvariationdefault",
                XETEX_COUNT_FEATURES_CODE => "XeTeXcountfeatures",
                XETEX_FEATURE_CODE_CODE => "XeTeXfeaturecode",
                XETEX_FIND_FEATURE_BY_NAME_CODE => "XeTeXfindfeaturebyname",
                XETEX_IS_EXCLUSIVE_FEATURE_CODE => "XeTeXisexclusivefeature",
                XETEX_COUNT_SELECTORS_CODE => "XeTeXcountselectors",
                XETEX_SELECTOR_CODE_CODE => "XeTeXselectorcode",
                XETEX_FIND_SELECTOR_BY_NAME_CODE => "XeTeXfindselectorbyname",
                XETEX_IS_DEFAULT_SELECTOR_CODE => "XeTeXisdefaultselector",
                XETEX_OT_COUNT_SCRIPTS_CODE => "XeTeXOTcountscripts",
                XETEX_OT_COUNT_LANGUAGES_CODE => "XeTeXOTcountlanguages",
                XETEX_OT_COUNT_FEATURES_CODE => "XeTeXOTcountfeatures",
                XETEX_OT_SCRIPT_CODE => "XeTeXOTscripttag",
                XETEX_OT_LANGUAGE_CODE => "XeTeXOTlanguagetag",
                XETEX_OT_FEATURE_CODE => "XeTeXOTfeaturetag",
                XETEX_MAP_CHAR_TO_GLYPH_CODE => "XeTeXcharglyph",
                XETEX_GLYPH_INDEX_CODE => "XeTeXglyphindex",
                XETEX_GLYPH_BOUNDS_CODE => "XeTeXglyphbounds",
                XETEX_FONT_TYPE_CODE => "XeTeXfonttype",
                XETEX_FIRST_CHAR_CODE => "XeTeXfirstfontchar",
                XETEX_LAST_CHAR_CODE => "XeTeXlastfontchar",
                XETEX_PDF_PAGE_COUNT_CODE => "XeTeXpdfpagecount",
                CURRENT_GROUP_LEVEL_CODE => "currentgrouplevel",
                CURRENT_GROUP_TYPE_CODE => "currentgrouptype",
                CURRENT_IF_LEVEL_CODE => "currentiflevel",
                CURRENT_IF_TYPE_CODE => "currentiftype",
                CURRENT_IF_BRANCH_CODE => "currentifbranch",
                FONT_CHAR_WD_CODE => "fontcharwd",
                FONT_CHAR_HT_CODE => "fontcharht",
                FONT_CHAR_DP_CODE => "fontchardp",
                FONT_CHAR_IC_CODE => "fontcharic",
                PAR_SHAPE_LENGTH_CODE => "parshapelength",
                PAR_SHAPE_INDENT_CODE => "parshapeindent",
                PAR_SHAPE_DIMEN_CODE => "parshapedimen",
                x if x == ETEX_EXPR + i32::from(INT_VAL) => "numexpr",
                x if x == ETEX_EXPR + i32::from(DIMEN_VAL) => "dimexpr",
                x if x == ETEX_EXPR + i32::from(GLUE_VAL) => "glueexpr",
                x if x == ETEX_EXPR + i32::from(MU_VAL) => "muexpr",
                GLUE_STRETCH_ORDER_CODE => "gluestretchorder",
                GLUE_SHRINK_ORDER_CODE => "glueshrinkorder",
                GLUE_STRETCH_CODE => "gluestretch",
                GLUE_SHRINK_CODE => "glueshrink",
                MU_TO_GLUE_CODE => "mutoglue",
                GLUE_TO_MU_CODE => "gluetomu",
                PDF_LAST_X_POS_CODE => "pdflastxpos",
                PDF_LAST_Y_POS_CODE => "pdflastypos",
                ELAPSED_TIME_CODE => "elapsedtime",
                PDF_SHELL_ESCAPE_CODE => "shellescape",
                RANDOM_SEED_CODE => "randomseed",
                _ => "badness",
            };
            print_esc(st, name);
        }
        CONVERT => {
            let name = match chr_code {
                NUMBER_CODE => "number",
                ROMAN_NUMERAL_CODE => "romannumeral",
                STRING_CODE => "string",
                MEANING_CODE => "meaning",
                FONT_NAME_CODE => "fontname",
                ETEX_REVISION_CODE => "eTeXrevision",
                EXPANDED_CODE => "expanded",
                LEFT_MARGIN_KERN_CODE => "leftmarginkern",
                RIGHT_MARGIN_KERN_CODE => "rightmarginkern",
                PDF_CREATION_DATE_CODE => "creationdate",
                PDF_FILE_MOD_DATE_CODE => "filemoddate",
                PDF_FILE_SIZE_CODE => "filesize",
                PDF_MDFIVE_SUM_CODE => "mdfivesum",
                PDF_FILE_DUMP_CODE => "filedump",
                PDF_STRCMP_CODE => "strcmp",
                UNIFORM_DEVIATE_CODE => "uniformdeviate",
                NORMAL_DEVIATE_CODE => "normaldeviate",
                XETEX_REVISION_CODE => "XeTeXrevision",
                XETEX_VARIATION_NAME_CODE => "XeTeXvariationname",
                XETEX_FEATURE_NAME_CODE => "XeTeXfeaturename",
                XETEX_SELECTOR_NAME_CODE => "XeTeXselectorname",
                XETEX_GLYPH_NAME_CODE => "XeTeXglyphname",
                XETEX_UCHAR_CODE => "Uchar",
                XETEX_UCHARCAT_CODE => "Ucharcat",
                _ => "jobname",
            };
            print_esc(st, name);
        }
        IF_TEST => {
            if chr_code >= UNLESS_CODE {
                print_esc(st, "unless");
            }
            let name = match (chr_code % UNLESS_CODE) as i16 {
                IF_CAT_CODE => "ifcat",
                IF_INT_CODE => "ifnum",
                IF_DIM_CODE => "ifdim",
                IF_ODD_CODE => "ifodd",
                IF_VMODE_CODE => "ifvmode",
                IF_HMODE_CODE => "ifhmode",
                IF_MMODE_CODE => "ifmmode",
                IF_INNER_CODE => "ifinner",
                IF_VOID_CODE => "ifvoid",
                IF_HBOX_CODE => "ifhbox",
                IF_VBOX_CODE => "ifvbox",
                IFX_CODE => "ifx",
                IF_EOF_CODE => "ifeof",
                IF_TRUE_CODE => "iftrue",
                IF_FALSE_CODE => "iffalse",
                IF_CASE_CODE => "ifcase",
                IF_PRIMITIVE_CODE => "ifprimitive",
                IF_DEF_CODE => "ifdefined",
                IF_CS_CODE => "ifcsname",
                IF_FONT_CHAR_CODE => "iffontchar",
                IF_IN_CSNAME_CODE => "ifincsname",
                _ => "if",
            };
            print_esc(st, name);
        }
        FI_OR_ELSE => {
            let name = if chr_code == i32::from(FI_CODE) {
                "fi"
            } else if chr_code == i32::from(OR_CODE) {
                "or"
            } else {
                "else"
            };
            print_esc(st, name);
        }
        TAB_MARK => {
            if chr_code == SPAN_CODE {
                print_esc(st, "span");
            } else {
                char_cmd(st, "alignment tab character ");
            }
        }
        CAR_RET => print_esc(st, if chr_code == CR_CODE { "cr" } else { "crcr" }),
        SET_PAGE_DIMEN => {
            let name = match chr_code {
                0 => "pagegoal",
                1 => "pagetotal",
                2 => "pagestretch",
                3 => "pagefilstretch",
                4 => "pagefillstretch",
                5 => "pagefilllstretch",
                6 => "pageshrink",
                _ => "pagedepth",
            };
            print_esc(st, name);
        }
        STOP => print_esc(st, if chr_code == 1 { "dump" } else { "end" }),
        HSKIP => print_esc(
            st,
            match chr_code {
                SKIP_CODE => "hskip",
                FIL_CODE => "hfil",
                FILL_CODE => "hfill",
                SS_CODE => "hss",
                _ => "hfilneg",
            },
        ),
        VSKIP => print_esc(
            st,
            match chr_code {
                SKIP_CODE => "vskip",
                FIL_CODE => "vfil",
                FILL_CODE => "vfill",
                SS_CODE => "vss",
                _ => "vfilneg",
            },
        ),
        MSKIP => print_esc(st, "mskip"),
        KERN => print_esc(st, "kern"),
        MKERN => print_esc(st, "mkern"),
        HMOVE => print_esc(
            st,
            if chr_code == 1 {
                "moveleft"
            } else {
                "moveright"
            },
        ),
        VMOVE => print_esc(st, if chr_code == 1 { "raise" } else { "lower" }),
        MAKE_BOX => {
            let name = match chr_code {
                BOX_CODE => "box",
                COPY_CODE => "copy",
                LAST_BOX_CODE => "lastbox",
                VSPLIT_CODE => "vsplit",
                VTOP_CODE => "vtop",
                x if x == VTOP_CODE + i32::from(VMODE) => "vbox",
                _ => "hbox",
            };
            print_esc(st, name);
        }
        LEADER_SHIP => {
            let name = match chr_code - i32::from(A_LEADERS) {
                0 => "leaders",
                1 => "cleaders",
                2 => "xleaders",
                _ => "shipout",
            };
            print_esc(st, name);
        }
        START_PAR => print_esc(st, if chr_code == 0 { "noindent" } else { "indent" }),
        REMOVE_ITEM => {
            let name = if chr_code == NodeType::Glue as i32 {
                "unskip"
            } else if chr_code == NodeType::Kern as i32 {
                "unkern"
            } else {
                "unpenalty"
            };
            print_esc(st, name);
        }
        UN_HBOX => print_esc(
            st,
            if chr_code == COPY_CODE {
                "unhcopy"
            } else {
                "unhbox"
            },
        ),
        UN_VBOX => print_esc(
            st,
            match chr_code {
                COPY_CODE => "unvcopy",
                LAST_BOX_CODE => "pagediscards",
                VSPLIT_CODE => "splitdiscards",
                _ => "unvbox",
            },
        ),
        DISCRETIONARY => {
            if chr_code == 1 {
                print_esc(st, b'-');
            } else {
                print_esc(st, "discretionary");
            }
        }
        EQ_NO => print_esc(st, if chr_code == 1 { "leqno" } else { "eqno" }),
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
                ORD_NOAD => "mathord",
                OP_NOAD => "mathop",
                BIN_NOAD => "mathbin",
                REL_NOAD => "mathrel",
                OPEN_NOAD => "mathopen",
                CLOSE_NOAD => "mathclose",
                PUNCT_NOAD => "mathpunct",
                INNER_NOAD => "mathinner",
                UNDER_NOAD => "underline",
                _ => "overline",
            };
            print_esc(st, name);
        }
        LIMIT_SWITCH => print_esc(
            st,
            match chr_code as u16 {
                LIMITS => "limits",
                NO_LIMITS => "nolimits",
                _ => "displaylimits",
            },
        ),
        MATH_STYLE => print_style(st, chr_code),
        ABOVE => {
            let name = match chr_code {
                OVER_CODE => "over",
                ATOP_CODE => "atop",
                x if x == DELIMITED_CODE + ABOVE_CODE => "abovewithdelims",
                x if x == DELIMITED_CODE + OVER_CODE => "overwithdelims",
                x if x == DELIMITED_CODE + ATOP_CODE => "atopwithdelims",
                _ => "above",
            };
            print_esc(st, name);
        }
        LEFT_RIGHT => {
            let name = if chr_code == NodeType::LeftNoad as i32 {
                "left"
            } else if chr_code == i32::from(MIDDLE_NOAD) {
                "middle"
            } else {
                "right"
            };
            print_esc(st, name);
        }
        PREFIX => print_esc(
            st,
            match chr_code {
                1 => "long",
                2 => "outer",
                8 => "protected",
                _ => "global",
            },
        ),
        DEF => print_esc(
            st,
            match chr_code {
                0 => "def",
                1 => "gdef",
                2 => "edef",
                _ => "xdef",
            },
        ),
        LET => print_esc(st, if chr_code != 0 { "futurelet" } else { "let" }),
        SHORTHAND_DEF => {
            let name = match chr_code {
                CHAR_DEF_CODE => "chardef",
                MATH_CHAR_DEF_CODE => "mathchardef",
                XETEX_MATH_CHAR_DEF_CODE => "Umathchardef",
                XETEX_MATH_CHAR_NUM_DEF_CODE => "Umathcharnumdef",
                COUNT_DEF_CODE => "countdef",
                DIMEN_DEF_CODE => "dimendef",
                SKIP_DEF_CODE => "skipdef",
                MU_SKIP_DEF_CODE => "muskipdef",
                CHAR_SUB_DEF_CODE => "charsubdef",
                _ => "toksdef",
            };
            print_esc(st, name);
        }
        CHAR_GIVEN => {
            print_esc(st, "char");
            print_hex(st, chr_code);
        }
        MATH_GIVEN => {
            print_esc(st, "mathchar");
            print_hex(st, chr_code);
        }
        XETEX_MATH_GIVEN => {
            print_esc(st, "Umathchar");
            print_hex(st, crate::scan::math_class(chr_code));
            print_hex(st, crate::scan::math_fam(chr_code));
            print_hex(st, crate::scan::math_char(chr_code));
        }
        DEF_CODE => {
            let name = match chr_code {
                LC_CODE_BASE => "lccode",
                UC_CODE_BASE => "uccode",
                SF_CODE_BASE => "sfcode",
                MATH_CODE_BASE => "mathcode",
                DEL_CODE_BASE => "delcode",
                _ => "catcode",
            };
            print_esc(st, name);
        }
        XETEX_DEF_CODE => {
            let name = match chr_code {
                SF_CODE_BASE => "XeTeXcharclass",
                MATH_CODE_BASE => "Umathcodenum",
                x if x == MATH_CODE_BASE + 1 => "Umathcode",
                DEL_CODE_BASE => "Udelcodenum",
                _ => "Udelcode",
            };
            print_esc(st, name);
        }
        DEF_FAMILY => print_size(st, chr_code - MATH_FONT_BASE),
        HYPH_DATA => print_esc(
            st,
            if chr_code == 1 {
                "patterns"
            } else {
                "hyphenation"
            },
        ),
        ASSIGN_FONT_INT => {
            let name = match chr_code {
                0 => "hyphenchar",
                1 => "skewchar",
                LP_CODE_BASE => "lpcode",
                RP_CODE_BASE => "rpcode",
                _ => return,
            };
            print_esc(st, name);
        }
        SET_FONT => print_font_select(st, FontId(chr_code)),
        SET_INTERACTION => {
            const BATCH: i32 = InteractionMode::Batch as i32;
            const NONSTOP: i32 = InteractionMode::Nonstop as i32;
            const SCROLL: i32 = InteractionMode::Scroll as i32;
            let name = match chr_code {
                BATCH => "batchmode",
                NONSTOP => "nonstopmode",
                SCROLL => "scrollmode",
                _ => "errorstopmode",
            };
            print_esc(st, name);
        }
        IN_STREAM => print_esc(st, if chr_code == 0 { "closein" } else { "openin" }),
        MESSAGE => print_esc(
            st,
            if chr_code == 0 {
                "message"
            } else {
                "errmessage"
            },
        ),
        CASE_SHIFT => print_esc(
            st,
            if chr_code == LC_CODE_BASE {
                "lowercase"
            } else {
                "uppercase"
            },
        ),
        XRAY => {
            let name = match chr_code {
                SHOW_BOX_CODE => "showbox",
                SHOW_THE_CODE => "showthe",
                SHOW_LISTS => "showlists",
                SHOW_GROUPS => "showgroups",
                SHOW_TOKENS => "showtokens",
                SHOW_IFS => "showifs",
                _ => "show",
            };
            print_esc(st, name);
        }
        UNDEFINED_CS => print(st, "undefined"),
        CALL | LONG_CALL | OUTER_CALL | LONG_OUTER_CALL => {
            // Bit 0 is long, bit 1 outer, bit 2 protected.
            let mut n = i32::from(cmd - CALL);
            if st.mem.is_protected_macro(MemPtr(chr_code)) {
                n += 4;
            }
            if n / 4 % 2 != 0 {
                print_esc(st, "protected");
            }
            if n % 2 != 0 {
                print_esc(st, "long");
            }
            if n / 2 % 2 != 0 {
                print_esc(st, "outer");
            }
            if n > 0 {
                print_char(st, b' ');
            }
            print(st, "macro");
        }
        END_TEMPLATE => print_esc(st, "outer endtemplate"),
        EXTENSION => {
            let name = match chr_code {
                OPEN_NODE => "openout",
                WRITE_NODE => "write",
                CLOSE_NODE => "closeout",
                SPECIAL_NODE => "special",
                IMMEDIATE_CODE => "immediate",
                SET_LANGUAGE_CODE => "setlanguage",
                PDF_SAVE_POS_NODE => "pdfsavepos",
                RESET_TIMER_CODE => "resettimer",
                SET_RANDOM_SEED_CODE => "setrandomseed",
                PIC_FILE_CODE => "XeTeXpicfile",
                PDF_FILE_CODE => "XeTeXpdffile",
                GLYPH_CODE => "XeTeXglyph",
                XETEX_LINEBREAK_LOCALE_EXTENSION_CODE => "XeTeXlinebreaklocale",
                XETEX_INPUT_ENCODING_EXTENSION_CODE => "XeTeXinputencoding",
                XETEX_DEFAULT_ENCODING_EXTENSION_CODE => "XeTeXdefaultencoding",
                _ => return print(st, "[unknown extension!]"),
            };
            print_esc(st, name);
        }
        _ => print(st, "[unknown command code!]"),
    }
}

/// `select font <name>`, quoting native font names, plus ` at <size>pt` if scaled.
fn print_font_select(st: &mut EngineState, f: FontId) {
    print(st, "select font ");
    print_font_name_and_size(st, f);
}

/// The name of font `f`, quoted if native, then ` at <size>pt` unless at design size.
pub(crate) fn print_font_name_and_size(st: &mut EngineState, f: FontId) {
    let name = st.fonts.font_name[f];
    let area = st.fonts.font_area[f];
    if area == AAT_FONT_FLAG || area == OTGR_FONT_FLAG {
        let has_double_quote =
            st.strings.str_pool[st.strings.str_range(name)].contains(&(u16::from(b'"')));
        let quote_char = i32::from(if has_double_quote { b'\'' } else { b'"' });
        print_char(st, quote_char);
        print(st, name);
        print_char(st, quote_char);
    } else {
        print(st, name);
    }
    if st.fonts.font_size[f] != st.fonts.font_dsize[f] {
        print(st, " at ");
        print_scaled(st, st.fonts.font_size[f]);
        print(st, "pt");
    }
}

pub(crate) fn print_meaning(st: &mut EngineState) {
    let cmd = st.scanner.cur_cmd;
    let chr = st.scanner.cur_chr;
    print_cmd_chr(st, cmd, chr);
    if cmd >= CALL {
        print_char(st, b':');
        print_ln(st);
        token_show(st, chr);
    } else if cmd == TOP_BOT_MARK && (chr as usize) < 5 {
        print_char(st, b':');
        print_ln(st);
        token_show(st, st.scanner.cur_mark[chr as usize]);
    }
}

/// Walks `base_ptr` down to the file at input level `level`, reporting whether it is a real file.
fn level_is_file(st: &mut EngineState, level: i32) -> bool {
    while st.input.input_stack[st.input.base_ptr as usize].state == TOKEN_LIST
        || i32::from(st.input.input_stack[st.input.base_ptr as usize].index) > level
    {
        st.input.base_ptr -= 1;
    }
    st.input.input_stack[st.input.base_ptr as usize].name > 17
}

pub(crate) fn group_warning(st: &mut EngineState) {
    st.input.base_ptr = st.input.input_ptr;
    st.input.input_stack[st.input.base_ptr as usize] = st.input.cur_input;
    let mut i = st.input.in_open;
    let mut w = false;
    while st.input.grp_stack[i as usize] == st.save_stack.cur_boundary && i > 0 {
        if st.eqtb.int_par(IntPar::TracingNesting) > 0 && level_is_file(st, i) {
            w = true;
        }
        st.input.grp_stack[i as usize] = st.save_stack.save_index(st.save_stack.save_ptr);
        i -= 1;
    }
    if w {
        print_nl(st, "Warning: ");
        diagnostic_begin_capture_warning_here(st);
        print(st, "end of ");
        print_group(st, true);
        print(st, " of a different file");
        print_ln(st);
        if st.eqtb.int_par(IntPar::TracingNesting) > 1 {
            show_context(st);
        }
        note_warning_issued(st);
        capture_to_diagnostic(st, None);
    }
}

pub(crate) fn if_warning(st: &mut EngineState) {
    st.input.base_ptr = st.input.input_ptr;
    st.input.input_stack[st.input.base_ptr as usize] = st.input.cur_input;
    let mut i = st.input.in_open;
    let mut w = false;
    while st.input.if_stack[i as usize] == st.scanner.cond_ptr {
        if st.eqtb.int_par(IntPar::TracingNesting) > 0 && level_is_file(st, i) {
            w = true;
        }
        st.input.if_stack[i as usize] = st.scanner.cond_ptr.link(&st.mem);
        i -= 1;
    }
    if w {
        print_nl(st, "Warning: ");
        diagnostic_begin_capture_warning_here(st);
        print(st, "end of ");
        print_cmd_chr(st, IF_TEST, i32::from(st.scanner.cur_if));
        print_if_line(st, st.scanner.if_line);
        print(st, " of a different file");
        print_ln(st);
        if st.eqtb.int_par(IntPar::TracingNesting) > 1 {
            show_context(st);
        }
        capture_to_diagnostic(st, None);
        note_warning_issued(st);
    }
}

fn print_if_line(st: &mut EngineState, line: i32) {
    if line != 0 {
        print(st, " entered on line ");
        print_int(st, line);
    }
}

/// Pops the save-stack group boundary at `save_ptr` into `cur_group`.
fn leave_group_boundary(st: &mut EngineState) {
    let ss = &mut st.save_stack;
    (ss.cur_group, ss.save_ptr) = ss.boundary_at(ss.save_ptr);
}

pub(crate) fn file_warning(st: &mut EngineState) {
    let saved_ptr = st.save_stack.save_ptr;
    let saved_level = st.save_stack.cur_level;
    let saved_group = st.save_stack.cur_group;
    st.save_stack.save_ptr = st.save_stack.cur_boundary;
    while st.input.grp_stack[st.input.in_open as usize] != st.save_stack.save_ptr {
        st.save_stack.cur_level -= 1;
        print_nl(st, "Warning: ");
        diagnostic_begin_capture_warning_here(st);
        print(st, "end of file when ");
        print_group(st, true);
        print(st, " is incomplete");
        capture_to_diagnostic(st, None);
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
        print_nl(st, "Warning: ");
        diagnostic_begin_capture_warning_here(st);
        print(st, "end of file when ");
        print_cmd_chr(st, IF_TEST, i32::from(st.scanner.cur_if));
        if st.scanner.if_limit == FI_CODE {
            print_esc(st, "else");
        }
        print_if_line(st, st.scanner.if_line);
        print(st, " is incomplete");
        capture_to_diagnostic(st, None);
        let cond = NodePtr::<CondNode>::at(st.scanner.cond_ptr);
        (st.scanner.cur_if, st.scanner.if_line, st.scanner.if_limit) = cond.saved_state(&st.mem);
        st.scanner.cond_ptr = cond.link(&st.mem);
    }
    st.scanner.cond_ptr = saved_cond;
    st.scanner.if_limit = saved_limit;
    st.scanner.cur_if = saved_if;
    st.scanner.if_line = saved_line;

    print_ln(st);
    if st.eqtb.int_par(IntPar::TracingNesting) > 1 {
        diagnostic_begin_capture_warning_here(st);
        show_context(st);
        capture_to_diagnostic(st, None);
    }
    note_warning_issued(st);
}

pub(crate) fn show_cur_cmd_chr(st: &mut EngineState) {
    begin_diagnostic(st);
    diagnostic_begin_capture_warning_here(st);
    print_nl(st, b'{');
    if st.input.cur_list.mode != st.scanner.shown_mode {
        print_mode(st, i32::from(st.input.cur_list.mode));
        print(st, ": ");
        st.scanner.shown_mode = st.input.cur_list.mode;
    }
    print_cmd_chr(st, st.scanner.cur_cmd, st.scanner.cur_chr);
    if st.eqtb.int_par(IntPar::TracingIfs) > 0
        && (IF_TEST as u8..=FI_OR_ELSE).contains(&st.scanner.cur_cmd)
    {
        print(st, ": ");
        let (mut n, l) = if st.scanner.cur_cmd == FI_OR_ELSE {
            print_cmd_chr(st, IF_TEST, i32::from(st.scanner.cur_if));
            print_char(st, b' ');
            (0, st.scanner.if_line)
        } else {
            (1, st.mem.line)
        };
        let mut p = st.scanner.cond_ptr;
        while !p.is_null() {
            n += 1;
            p = p.link(&st.mem);
        }
        print(st, "(level ");
        print_int(st, n);
        print_char(st, b')');
        print_if_line(st, l);
    }
    print_char(st, b'}');
    capture_to_diagnostic(st, None);
    end_diagnostic(st, false);
}

pub(crate) fn change_if_limit(
    st: &mut EngineState,
    l: i16,
    p: impl crate::units::MemAddr,
) -> Result<(), Abort> {
    let p = p.addr();
    if p == st.scanner.cond_ptr.0 {
        st.scanner.if_limit = l as u8;
        return Ok(());
    }
    let mut q = st.scanner.cond_ptr;
    while !q.is_null() {
        let next = q.link(&st.mem);
        if next.0 == p {
            NodePtr::<CondNode>::at(q).set_if_limit(&mut st.mem, l as u16);
            return Ok(());
        }
        q = next;
    }
    Err(Abort::Confusion("if"))
}

pub(crate) fn font_feature_warning(st: &mut EngineState, feature: &[u8], setting: &[u8]) {
    begin_diagnostic(st);
    diagnostic_begin_capture_warning_here(st);
    print_nl(st, "Unknown ");
    if !setting.is_empty() {
        print(st, "selector `");
        print_raw_bytes(st, setting);
        print(st, "' for ");
    }
    print(st, "feature `");
    print_raw_bytes(st, feature);
    print(st, "' in font `");
    print_name_of_file(st);
    print(st, "'.");
    capture_to_diagnostic(st, None);
    end_diagnostic(st, false);
}

pub(crate) fn font_mapping_warning(st: &mut EngineState, mapping_name: &[u8], warning_type: i32) {
    begin_diagnostic(st);
    diagnostic_begin_capture_warning_here(st);
    if warning_type == 0 {
        print_nl(st, "Loaded mapping `");
    } else {
        print_nl(st, "Font mapping `");
    }
    print_raw_bytes(st, mapping_name);
    print(st, "' for font `");
    print_name_of_file(st);
    match warning_type {
        1 => print(st, "' not found."),
        2 => {
            print(st, "' not usable;");
            print_nl(st, "bad mapping file or incorrect mapping type.");
        }
        _ => print(st, "'."),
    }
    capture_to_diagnostic(st, None);
    end_diagnostic(st, false);
}

fn print_name_of_file(st: &mut EngineState) {
    let name = st.input.name_of_file_bytes().to_vec();
    print_raw_bytes(st, &name);
}

/// Prints `bytes` with `print_raw_char`, one column each.
pub(crate) fn print_raw_bytes(st: &mut EngineState, bytes: &[u8]) {
    for &b in bytes {
        print_raw_char(st, i32::from(b), true);
    }
}

pub(crate) fn bad_utf8_warning(st: &mut EngineState) {
    begin_diagnostic(st);
    diagnostic_begin_capture_warning_here(st);
    print_nl(st, "Invalid UTF-8 byte or sequence");
    if st.input.cur_input.name == 0 {
        print(st, " in terminal input");
    } else {
        print(st, " at line ");
        print_int(st, st.mem.line);
    }
    print(st, " replaced by U+FFFD.");
    capture_to_diagnostic(st, None);
    end_diagnostic(st, false);
}

/// Records where the error context splits, when printing reaches the current position.
fn set_trick_count(st: &mut EngineState) {
    st.errors.first_count = st.errors.tally;
    let tc = st.errors.tally + 1 + st.errors.error_line - st.errors.half_error_line;
    st.errors.trick_count = tc.max(st.errors.error_line);
}

pub(crate) fn show_token_list(st: &mut EngineState, p: impl crate::units::MemAddr, q: i32, l: i32) {
    let mut p = MemPtr(p.addr());
    let mut match_chr = i32::from(b'#');
    let mut n = i32::from(b'0');
    st.errors.tally = 0;
    while !p.is_null() && st.errors.tally < l {
        if p.0 == q {
            set_trick_count(st);
        }
        if p.0 < st.mem.hi_mem_min || p.0 > st.mem.mem_end {
            print_esc(st, "CLOBBERED.");
            return;
        }
        let w = p.info(&st.mem);
        if w >= CS_TOKEN_FLAG {
            print_cs(st, w - CS_TOKEN_FLAG);
        } else if w < 0 {
            print_esc(st, "BAD.");
        } else {
            let c = w % MAX_CHAR_VAL;
            match (w / MAX_CHAR_VAL) as u8 {
                LEFT_BRACE | RIGHT_BRACE | MATH_SHIFT | TAB_MARK | SUP_MARK | SUB_MARK | SPACER
                | LETTER | OTHER_CHAR => print_char(st, c),
                MAC_PARAM => {
                    print_char(st, c);
                    print_char(st, c);
                }
                OUT_PARAM => {
                    print_char(st, match_chr);
                    if c > 9 {
                        print_char(st, b'!');
                        return;
                    }
                    print_char(st, i32::from(b'0') + c);
                }
                MATCH => {
                    match_chr = c;
                    print_char(st, c);
                    n += 1;
                    print_char(st, n);
                    if n > i32::from(b'9') {
                        return;
                    }
                }
                END_MATCH => {
                    if c == 0 {
                        print(st, "->");
                    }
                }
                _ => print_esc(st, "BAD."),
            }
        }
        p = p.link(&st.mem);
    }
    if !p.is_null() {
        print_esc(st, "ETC.");
    }
}

fn print_totals(st: &mut EngineState) {
    print_scaled(st, st.shipout.page_so_far[1]);
    for (i, order) in [(2, ""), (3, "fil"), (4, "fill"), (5, "filll")] {
        if st.shipout.page_so_far[i] != Scaled::ZERO {
            print(st, " plus ");
            print_scaled(st, st.shipout.page_so_far[i]);
            print(st, order);
        }
    }
    if st.shipout.page_so_far[6] != Scaled::ZERO {
        print(st, " minus ");
        print_scaled(st, st.shipout.page_so_far[6]);
    }
}

fn show_info(st: &mut EngineState) {
    let p = st.mem.temp_ptr.info(&st.mem);
    show_node_list(st, p);
}

/// Drops the most recently made string.
fn flush_last_string(st: &mut EngineState) {
    st.strings.str_ptr -= 1;
    st.strings.discard_cur_string();
}

pub(crate) fn issue_message(st: &mut EngineState) -> Result<(), Abort> {
    let c = st.scanner.cur_chr as u8;
    let toks = crate::scan::scan_toks(st, false, true)?;
    GARBAGE.set_link(&mut st.mem, toks);
    let old_setting = st.shipout.selector;
    st.shipout.selector = SELECTOR_NEW_STRING;
    token_show(st, st.scanner.def_ref);
    st.shipout.selector = old_setting;
    st.mem.flush_list(st.scanner.def_ref);
    if st.strings.pool_ptr + 1 > st.strings.pool_size {
        return Err(Abort::PoolOverflow);
    }
    let s = st.strings.make_string()?;
    if c == 0 {
        let len = st.strings.length(s);
        if st.shipout.term_offset + len > st.config.max_print_line - 2 {
            print_ln(st);
        } else if st.shipout.term_offset > 0 || st.shipout.file_offset > 0 {
            print_char(st, b' ');
        }
        print(st, s);
        output_flush(st.shipout.rust_stdout);
    } else {
        error_here_with_diagnostic(st, "");
        print(st, s);
        capture_to_diagnostic(st, None);
        if !st.eqtb.local(crate::consts::LOCAL_ERR_HELP).is_null() {
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
        let outcome = error(st);
        st.errors.use_err_help = false;
        outcome?;
    }
    flush_last_string(st);
    Ok(())
}

pub(crate) fn show_save_groups(st: &mut EngineState) {
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

    print_nl(st, "");
    print_ln(st);

    loop {
        print_nl(st, "### ");
        print_group(st, true);
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

        print(st, " (");

        let mut s: &str = "";
        let entry = match st.save_stack.cur_group {
            GroupCode::Simple => {
                p += 1;
                Entry::Found2
            }
            GroupCode::Hbox | GroupCode::AdjustedHbox => {
                s = "hbox";
                Entry::BoxContext
            }
            GroupCode::Vbox => {
                s = "vbox";
                Entry::BoxContext
            }
            GroupCode::Vtop => {
                s = "vtop";
                Entry::BoxContext
            }
            GroupCode::Align if a == 0 => {
                s = if m == -VMODE { "halign" } else { "valign" };
                a = 1;
                Entry::Found1
            }
            GroupCode::Align => {
                if a == 1 {
                    print(st, "align entry");
                } else {
                    print_esc(st, "cr");
                }
                if p >= i32::from(a) {
                    p -= i32::from(a);
                }
                a = 0;
                Entry::Found
            }
            GroupCode::NoAlign => {
                p += 1;
                a = -1;
                print_esc(st, "noalign");
                Entry::Found2
            }
            GroupCode::Output => {
                print_esc(st, "output");
                Entry::Found
            }
            GroupCode::Math => Entry::Found2,
            GroupCode::Disc | GroupCode::MathChoice => {
                if st.save_stack.cur_group == GroupCode::Disc {
                    print_esc(st, "discretionary");
                } else {
                    print_esc(st, "mathchoice");
                }
                let braces = st.save_stack.saved(-2);
                for i in 1..=3 {
                    if i <= braces {
                        print(st, "{}");
                    }
                }
                Entry::Found2
            }
            GroupCode::Insert => {
                let n = st.save_stack.saved(-2);
                if n == 255 {
                    print_esc(st, "vadjust");
                } else {
                    print_esc(st, "insert");
                    print_int(st, n);
                }
                Entry::Found2
            }
            GroupCode::Vcenter => {
                s = "vcenter";
                Entry::Found1
            }
            GroupCode::SemiSimple => {
                p += 1;
                print_esc(st, "begingroup");
                Entry::Found
            }
            GroupCode::MathShift => {
                if m == MMODE {
                    print_char(st, b'$');
                } else if st.input.nest[p as usize].mode == MMODE {
                    print_cmd_chr(st, EQ_NO, st.save_stack.saved(-2));
                    print_char(st, b')');
                    st.save_stack.cur_level -= 1;
                    leave_group_boundary(st);
                    continue;
                }
                print_char(st, b'$');
                Entry::Found
            }
            GroupCode::MathLeft => {
                let delim = MemIdx::new(st.input.nest[(p + 1) as usize].e_tex_aux);
                if st.mem.node_type(delim) == Ok(NodeType::LeftNoad) {
                    print_esc(st, "left");
                } else {
                    print_esc(st, "middle");
                }
                Entry::Found
            }
            GroupCode::BottomLevel => Entry::Found,
        };

        if matches!(entry, Entry::BoxContext) {
            let i = st.save_stack.saved(-4);
            if i != 0 {
                if i < BOX_FLAG {
                    let cmd = if st.input.nest[p as usize].mode.abs() == VMODE {
                        HMOVE
                    } else {
                        VMOVE
                    };
                    print_cmd_chr(st, cmd, if i > 0 { 0 } else { 1 });
                    print_scaled(st, Scaled(i.abs()));
                    print(st, "pt");
                } else if i < SHIP_OUT_FLAG {
                    let mut box_num = i;
                    if box_num >= GLOBAL_BOX_FLAG {
                        print_esc(st, "global");
                        box_num -= GLOBAL_BOX_FLAG - BOX_FLAG;
                    }
                    print_esc(st, "setbox");
                    print_int(st, box_num - BOX_FLAG);
                    print_char(st, b'=');
                } else {
                    print_cmd_chr(st, LEADER_SHIP, i - (LEADER_FLAG - i32::from(A_LEADERS)));
                }
            }
        }

        if matches!(entry, Entry::BoxContext | Entry::Found1) {
            print_esc(st, s);
            let spec = st.save_stack.saved(-2);
            if spec != 0 {
                print_char(st, b' ');
                if st.save_stack.saved(-3) == i32::from(EXACTLY) {
                    print(st, "to");
                } else {
                    print(st, "spread");
                }
                print_scaled(st, Scaled(spec));
                print(st, "pt");
            }
        }

        if !matches!(entry, Entry::Found) {
            print_char(st, b'{');
        }

        print_char(st, b')');
        st.save_stack.cur_level -= 1;
        leave_group_boundary(st);
    }

    st.save_stack.save_ptr = saved_ptr;
    st.save_stack.cur_level = saved_level;
    st.save_stack.cur_group = saved_group;
}

pub(crate) fn show_whatever(st: &mut EngineState) -> Result<(), Abort> {
    const SHOW_CODE: i32 = 0;

    diagnostic_begin_capture_warning_here(st);

    let chr = st.scanner.cur_chr;
    match chr {
        SHOW_LISTS => {
            begin_diagnostic(st);
            show_activities(st);
        }
        SHOW_BOX_CODE => {
            crate::scan::scan_register_num(st)?;
            let n = st.scanner.cur_val;
            let contents = crate::scan::fetch_box(st, n);
            begin_diagnostic(st);
            print_nl(st, "> \\box");
            print_int(st, n);
            print_char(st, b'=');
            match contents {
                Some(b) => show_box(st, b.ptr()),
                None => print(st, "void"),
            }
        }
        SHOW_CODE => {
            crate::scan::get_token(st)?;
            print_nl(st, "> ");
            if st.scanner.cur_cs != 0 {
                sprint_cs(st, st.scanner.cur_cs);
                print_char(st, b'=');
            }
            print_meaning(st);
        }
        SHOW_GROUPS => {
            begin_diagnostic(st);
            show_save_groups(st);
        }
        SHOW_IFS => {
            begin_diagnostic(st);
            print_nl(st, "");
            print_ln(st);
            if st.scanner.cond_ptr.is_null() {
                print_nl(st, "### ");
                print(st, "no active conditionals");
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
                    print_nl(st, "### level ");
                    print_int(st, n);
                    print(st, ": ");
                    print_cmd_chr(st, IF_TEST, i32::from(t));
                    if m == FI_CODE {
                        print_esc(st, "else");
                    }
                    print_if_line(st, l);
                    n -= 1;
                    (t, l, m) = NodePtr::<CondNode>::at(p).saved_state(&st.mem);
                    p = p.link(&st.mem);
                }
            }
        }
        _ => {
            crate::scan::the_toks(st)?;
            print_nl(st, "> ");
            token_show(st, TEMP_HEAD);
            let list = TEMP_HEAD.link(&st.mem);
            st.mem.flush_list(list);
        }
    }

    if matches!(chr, SHOW_LISTS | SHOW_BOX_CODE | SHOW_GROUPS | SHOW_IFS) {
        capture_to_diagnostic(st, None);
        end_diagnostic(st, true);
        print_error_prefix(st);
        print(st, "OK");
        if st.shipout.selector == SELECTOR_TERM_AND_LOG
            && st.eqtb.int_par(IntPar::TracingOnline) <= 0
        {
            st.shipout.selector = SELECTOR_TERM_ONLY;
            print(st, " (see the transcript file)");
            st.shipout.selector = SELECTOR_TERM_AND_LOG;
        }
    }

    capture_to_diagnostic(st, None);
    if st.errors.interaction < InteractionMode::ErrorStop {
        st.errors.help(&[]);
        st.errors.error_count -= 1;
    } else if st.eqtb.int_par(IntPar::TracingOnline) > 0 {
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
    error(st)
}

pub(crate) fn char_warning(st: &mut EngineState, f: FontId, c: i32) -> Result<(), Abort> {
    let tracing_lost = st.eqtb.int_par(IntPar::TracingLostChars);
    if tracing_lost > 0 {
        let old_setting = st.eqtb.int_par(IntPar::TracingOnline);
        if tracing_lost > 1 {
            st.eqtb.set_int_par(IntPar::TracingOnline, 1);
        }
        if tracing_lost > 2 {
            print_error_prefix(st);
            print(st, "Missing character: There is no ");
        } else {
            begin_diagnostic(st);
            diagnostic_begin_capture_warning_here(st);
            print_nl(st, "Missing character: There is no ");
        }
        print_code(st, c);
        print(st, " (");
        let area = st.fonts.font_area[f];
        if area == AAT_FONT_FLAG || area == OTGR_FONT_FLAG {
            print_ucs_code(st, c);
        } else {
            print_hex(st, c);
        }
        print_char(st, b')');
        print(st, " in font ");
        print(st, st.fonts.font_name[f]);
        if tracing_lost < 3 {
            print_char(st, b'!');
        }
        st.eqtb.set_int_par(IntPar::TracingOnline, old_setting);
        if tracing_lost > 2 {
            st.errors.help(&[]);
            error(st)?;
        } else {
            capture_to_diagnostic(st, None);
            end_diagnostic(st, false);
        }
    }

    let font_name = st.strings.texstring_bytes(st.fonts.font_name[f]);
    let prev_sel = st.shipout.selector;
    st.shipout.selector = SELECTOR_NEW_STRING;
    print_code(st, c);
    st.shipout.selector = prev_sel;
    let s = st.strings.make_string()?;
    let chr = st.strings.texstring_bytes(s);
    flush_last_string(st);
    issue_warning(&format!(
        "could not represent character \"{}\" (0x{:x}) in font \"{}\"",
        String::from_utf8_lossy(&chr),
        c as u32,
        String::from_utf8_lossy(&font_name),
    ));
    if !st.fonts.gave_char_warning_help {
        issue_warning(
            "  you may need to load the `fontspec` package and use (e.g.) \\setmainfont to",
        );
        issue_warning("  choose a different font that covers the unrepresentable character(s)");
        st.fonts.gave_char_warning_help = true;
    }
    Ok(())
}

pub(crate) fn show_box(st: &mut EngineState, p: impl crate::units::MemAddr) {
    st.errors.depth_threshold = st.eqtb.int_par(IntPar::ShowBoxDepth);
    st.errors.breadth_max = st.eqtb.int_par(IntPar::ShowBoxBreadth);
    if st.errors.breadth_max <= 0 {
        st.errors.breadth_max = 5;
    }
    let room = st.strings.pool_size - st.strings.pool_ptr - 1;
    if st.errors.depth_threshold > room {
        st.errors.depth_threshold = room;
    }
    show_node_list(st, p);
    print_ln(st);
}

pub(crate) fn short_display(st: &mut EngineState, p: impl crate::units::MemAddr) {
    let mut p = MemPtr(p.addr());
    while p.0 > 0 {
        if st.mem.is_char_node(p) {
            if p.0 <= st.mem.mem_end {
                let ch = NodePtr::<CharNode>::at(p);
                let font = ch.font(&st.mem);
                if font != st.errors.font_in_short_display {
                    if font > st.fonts.font_max {
                        print_char(st, b'*');
                    } else {
                        print_esc(st, st.eqtb.font_id_text(font));
                    }
                    print_char(st, b' ');
                    st.errors.font_in_short_display = font;
                }
                print(st, i32::from(ch.character(&st.mem)));
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
                ) => print(st, "[]"),
                Ok(AnyNode::Whatsit(w)) => {
                    if w.whatsit_subtype(&st.mem)
                        .is_some_and(|s| s.is_native_word())
                    {
                        let font = w.native_font(&st.mem);
                        if font != st.errors.font_in_short_display {
                            print_esc(st, st.eqtb.font_id_text(font));
                            print_char(st, b' ');
                            st.errors.font_in_short_display = font;
                        }
                        print_native_word(st, w);
                    } else {
                        print(st, "[]");
                    }
                }
                Ok(AnyNode::Rule(_)) => print_char(st, b'|'),
                Ok(AnyNode::Glue(g)) => {
                    // Glue pointing at the shared zero spec prints nothing.
                    if g.glue_ptr(&st.mem) != ZERO_GLUE {
                        print_char(st, b' ');
                    }
                }
                Ok(AnyNode::Math(m)) => {
                    if m.subtype(&st.mem) >= L_CODE {
                        print(st, "[]");
                    } else {
                        print_char(st, b'$');
                    }
                }
                Ok(AnyNode::Ligature(l)) => short_display(st, l.lig_ptr(&st.mem)),
                Ok(AnyNode::Disc(d)) => {
                    short_display(st, d.pre_break(&st.mem));
                    short_display(st, d.post_break(&st.mem));
                    for _ in 0..d.subtype(&st.mem) {
                        if !p.link(&st.mem).is_null() {
                            p = p.link(&st.mem);
                        }
                    }
                }
                _ => {}
            }
        }
        p = p.link(&st.mem);
    }
}

/// The outer vertical list part of `show_activities`: the current page and its insertions.
fn show_page_so_far(st: &mut EngineState) {
    if st.shipout.page_tail != PAGE_HEAD {
        print_nl(st, "### current page:");
        if st.shipout.output_active {
            print(st, " (held over for next output)");
        }
        show_box(st, PAGE_HEAD.link(&st.mem));
        if st.shipout.page_contents != crate::state::PageContents::Empty {
            print_nl(st, "total height ");
            print_totals(st);
            print_nl(st, " goal height ");
            print_scaled(st, st.shipout.page_so_far[0]);
            let mut r = PAGE_INS_HEAD.link(&st.mem);
            while r != PAGE_INS_HEAD {
                let ins = NodePtr::<PageInsNode>::at(r);
                print_ln(st);
                print_esc(st, "insert");
                let box_num = ins.subtype(&st.mem);
                print_int(st, i32::from(box_num));
                print(st, " adds ");
                let count = st.eqtb.count(i32::from(box_num));
                let height = ins.height(&st.mem);
                let adds = if count == 1000 {
                    height
                } else {
                    st.errors.x_over_n(height, 1000) * count
                };
                print_scaled(st, adds);
                if ins.is_split_up(&st.mem) {
                    let broken_ins = ins.broken_ins(&st.mem);
                    let mut q = PAGE_HEAD;
                    let mut t = 0;
                    loop {
                        q = q.link(&st.mem);
                        if let Ok(AnyNode::Ins(node)) = st.mem.classify(MemIdx::new(q)) {
                            if node.subtype(&st.mem) == box_num {
                                t += 1;
                            }
                        }
                        if q == broken_ins {
                            break;
                        }
                    }
                    print(st, ", #");
                    print_int(st, t);
                    print(st, " might split");
                }
                r = r.link(&st.mem);
            }
        }
    }
    if !CONTRIB_HEAD.link(&st.mem).is_null() {
        print_nl(st, "### recent contributions:");
    }
}

pub(crate) fn show_activities(st: &mut EngineState) {
    st.input.nest[st.input.nest_ptr as usize] = st.input.cur_list;
    print_nl(st, "");
    print_ln(st);
    for p in (0..=st.input.nest_ptr).rev() {
        let nest = st.input.nest[p as usize];
        let m = nest.mode;
        print_nl(st, "### ");
        print_mode(st, i32::from(m));
        print(st, " entered at line ");
        print_int(st, nest.mode_line.abs());
        if m == HMODE && nest.prev_graf != 0x83_0000 {
            let pg = nest.prev_graf;
            print(st, " (language");
            print_int(st, pg % 65536);
            print(st, ":hyphenmin");
            print_int(st, pg / 0x40_0000);
            print_char(st, b',');
            print_int(st, (pg / 65536) % 64);
            print_char(st, b')');
        }
        if nest.mode_line < 0 {
            print(st, " (\\output routine)");
        }
        if p == 0 {
            show_page_so_far(st);
        }
        show_box(st, nest.head.link(&st.mem));
        let (aux_s0, aux_s1) = (nest.aux_s0(), nest.aux_s1());
        match i32::from(m.abs()) / (i32::from(MAX_COMMAND) + 1) {
            0 => {
                print_nl(st, "prevdepth ");
                let prev_depth = nest.prev_depth();
                if prev_depth <= IGNORE_DEPTH {
                    print(st, "ignored");
                } else {
                    print_scaled(st, prev_depth);
                }
                if nest.prev_graf != 0 {
                    print(st, ", prevgraf ");
                    print_int(st, nest.prev_graf);
                    if nest.prev_graf != 1 {
                        print(st, " lines");
                    } else {
                        print(st, " line");
                    }
                }
            }
            1 => {
                print_nl(st, "spacefactor ");
                print_int(st, aux_s0);
                if m > 0 && aux_s1 > 0 {
                    print(st, ", current language ");
                    print_int(st, aux_s1);
                }
            }
            2 => {
                if aux_s1 != TEX_NULL {
                    print(st, "this will begin denominator of:");
                    show_box(st, aux_s1);
                }
            }
            _ => {}
        }
    }
}

/// Shows sublist `list` with `c` appended to the indentation string.
fn node_list_display(st: &mut EngineState, c: u8, list: MemPtr) {
    st.strings.append_char(u16::from(c));
    show_node_list(st, list);
    st.strings.pool_ptr -= 1;
}

/// Shows the noad field at `p`, marked with `c` in the indentation string.
fn print_subsidiary_data(st: &mut EngineState, p: NodePtr<MathField>, c: u8) {
    if st.strings.cur_length() >= st.errors.depth_threshold {
        if p.math_type(&st.mem) != EMPTY {
            print(st, " []");
        }
        return;
    }
    st.strings.append_char(u16::from(c));
    st.mem.temp_ptr = p.ptr();
    match p.math_type(&st.mem) {
        MATH_CHAR => {
            print_ln(st);
            print_current_string(st);
            print_fam_and_char(st, p);
        }
        SUB_BOX => show_info(st),
        SUB_MLIST => {
            if p.info(&st.mem).is_null() {
                print_ln(st);
                print_current_string(st);
                print(st, "{}");
            } else {
                show_info(st);
            }
        }
        _ => {}
    }
    st.strings.pool_ptr -= 1;
}

pub(crate) fn show_node_list(st: &mut EngineState, p: impl crate::units::MemAddr) {
    let mut p = MemPtr(p.addr());
    if st.strings.cur_length() > st.errors.depth_threshold {
        if p.0 > TEX_NULL {
            print(st, " []");
        }
        return;
    }
    let mut n = 0;
    while p.0 > 0 {
        print_ln(st);
        print_current_string(st);
        if p.0 > st.mem.mem_end {
            print(st, "Bad link, display aborted.");
            return;
        }
        n += 1;
        if n > st.errors.breadth_max {
            print(st, "etc.");
            return;
        }
        if st.mem.is_char_node(p) {
            print_font_and_char(st, p);
        } else {
            match st.mem.classify(MemIdx::new(p)) {
                Ok(AnyNode::HList(b)) => show_box_node(st, AnyBox::HList(b)),
                Ok(AnyNode::VList(b)) => show_box_node(st, AnyBox::VList(b)),
                Ok(AnyNode::Unset(node)) => {
                    print_esc(st, "unset");
                    print(st, "box(");
                    print_scaled(st, node.height(&st.mem));
                    print_char(st, b'+');
                    print_scaled(st, node.depth(&st.mem));
                    print(st, ")x");
                    print_scaled(st, node.width(&st.mem));
                    let span_count = node.subtype(&st.mem);
                    if span_count != 0 {
                        print(st, " (");
                        print_int(st, i32::from(span_count) + 1);
                        print(st, " columns)");
                    }
                    if node.glue_stretch(&st.mem) != Scaled::ZERO {
                        print(st, ", stretch ");
                        let order = node.glue_order(&st.mem);
                        print_glue(st, node.glue_stretch(&st.mem), order, None);
                    }
                    if node.glue_shrink(&st.mem) != Scaled::ZERO {
                        print(st, ", shrink ");
                        let order = node.glue_sign(&st.mem);
                        print_glue(st, node.glue_shrink(&st.mem), order, None);
                    }
                    node_list_display(st, b'.', node.list_ptr(&st.mem));
                }
                Ok(AnyNode::Rule(node)) => {
                    print_esc(st, "rule(");
                    print_rule_dimen(st, node.height(&st.mem));
                    print_char(st, b'+');
                    print_rule_dimen(st, node.depth(&st.mem));
                    print(st, ")x");
                    print_rule_dimen(st, node.width(&st.mem));
                }
                Ok(AnyNode::Ins(node)) => {
                    print_esc(st, "insert");
                    print_int(st, i32::from(node.subtype(&st.mem)));
                    print(st, ", natural size ");
                    print_scaled(st, node.height(&st.mem));
                    print(st, "; split(");
                    print_spec(st, node.split_top_ptr(&st.mem).ptr(), None);
                    print_char(st, b',');
                    print_scaled(st, node.depth(&st.mem));
                    print(st, "); float cost ");
                    print_int(st, node.float_cost(&st.mem));
                    node_list_display(st, b'.', node.ins_ptr(&st.mem));
                }
                Ok(AnyNode::Whatsit(node)) => show_whatsit(st, node),
                Ok(AnyNode::Glue(node)) => show_glue(st, node),
                Ok(AnyNode::Kern(node)) => {
                    let sub = node.subtype(&st.mem);
                    if sub != MU_GLUE {
                        print_esc(st, "kern");
                        if sub != 0 {
                            print_char(st, b' ');
                        }
                        print_scaled(st, node.width(&st.mem));
                        if sub == ACC_KERN {
                            print(st, " (for accent)");
                        } else if sub == SPACE_ADJUSTMENT {
                            print(st, " (space adjustment)");
                        }
                    } else {
                        print_esc(st, "mkern");
                        print_scaled(st, node.width(&st.mem));
                        print(st, "mu");
                    }
                }
                Ok(AnyNode::MarginKern(node)) => {
                    print_esc(st, "kern");
                    print_scaled(st, node.width(&st.mem));
                    if node.subtype(&st.mem) == 0 {
                        print(st, " (left margin)");
                    } else {
                        print(st, " (right margin)");
                    }
                }
                Ok(AnyNode::Math(node)) => {
                    let sub = node.subtype(&st.mem);
                    if sub > AFTER {
                        print_esc(st, if sub % 2 == 1 { "end" } else { "begin" });
                        let dir = if sub > R_CODE {
                            b'R'
                        } else if sub > L_CODE {
                            b'L'
                        } else {
                            b'M'
                        };
                        print_char(st, i32::from(dir));
                    } else {
                        print_esc(st, "math");
                        if sub == 0 {
                            print(st, "on");
                        } else {
                            print(st, "off");
                        }
                        if node.width(&st.mem) != Scaled::ZERO {
                            print(st, ", surrounded ");
                            print_scaled(st, node.width(&st.mem));
                        }
                    }
                }
                Ok(AnyNode::Ligature(node)) => {
                    let font = node.font(&st.mem);
                    print_font_char(st, font, node.character(&st.mem));
                    print(st, " (ligature ");
                    if node.subtype(&st.mem) > 1 {
                        print_char(st, b'|');
                    }
                    st.errors.font_in_short_display = font;
                    short_display(st, node.lig_ptr(&st.mem));
                    if node.subtype(&st.mem) % 2 == 1 {
                        print_char(st, b'|');
                    }
                    print_char(st, b')');
                }
                Ok(AnyNode::Penalty(node)) => {
                    print_esc(st, "penalty ");
                    print_int(st, node.penalty(&st.mem));
                }
                Ok(AnyNode::Disc(node)) => {
                    print_esc(st, "discretionary");
                    let replace_count = node.subtype(&st.mem);
                    if replace_count > 0 {
                        print(st, " replacing ");
                        print_int(st, i32::from(replace_count));
                    }
                    node_list_display(st, b'.', node.pre_break(&st.mem));
                    node_list_display(st, b'|', node.post_break(&st.mem));
                }
                Ok(AnyNode::Mark(node)) => {
                    print_esc(st, "mark");
                    let class = node.mark_class(&st.mem);
                    if class != 0 {
                        print_char(st, b's');
                        print_int(st, class);
                    }
                    print_mark(st, node.mark_ptr(&st.mem));
                }
                Ok(AnyNode::Adjust(node)) => {
                    print_esc(st, "vadjust");
                    if node.subtype(&st.mem) != 0 {
                        print(st, " pre ");
                    }
                    node_list_display(st, b'.', node.adjust_ptr(&st.mem));
                }
                Ok(AnyNode::Style(node)) => print_style(st, i32::from(node.subtype(&st.mem))),
                Ok(AnyNode::Choice(node)) => {
                    print_esc(st, "mathchoice");
                    node_list_display(st, b'D', node.display_mlist(&st.mem));
                    node_list_display(st, b'T', node.text_mlist(&st.mem));
                    node_list_display(st, b'S', node.script_mlist(&st.mem));
                    node_list_display(st, b's', node.script_script_mlist(&st.mem));
                }
                Ok(AnyNode::Noad(node)) => show_noad(st, node),
                Err(_) => print(st, "Unknown node type!"),
            }
        }
        p = p.link(&st.mem);
    }
}

fn show_box_node(st: &mut EngineState, b: AnyBox) {
    let is_hlist = matches!(b, AnyBox::HList(_));
    print_esc(st, i32::from(if is_hlist { b'h' } else { b'v' }));
    print(st, "box(");
    print_scaled(st, b.height(&st.mem));
    print_char(st, b'+');
    print_scaled(st, b.depth(&st.mem));
    print(st, ")x");
    print_scaled(st, b.width(&st.mem));
    let g = b.glue_set(&st.mem);
    let glue_sign = b.glue_sign(&st.mem);
    if g != 0.0 && glue_sign != GlueSign::Normal {
        print(st, ", glue set ");
        if glue_sign == GlueSign::Shrinking {
            print(st, "- ");
        }
        let order = b.glue_order(&st.mem);
        if g.abs() > 20000.0 {
            if g > 0.0 {
                print_char(st, b'>');
            } else {
                print(st, "< -");
            }
            print_glue(st, Scaled::UNITY * 20000, order, None);
        } else {
            let set = Scaled(crate::scaled_math::tex_round(65536.0 * g));
            print_glue(st, set, order, None);
        }
    }
    if b.shift_amount(&st.mem) != Scaled::ZERO {
        print(st, ", shifted ");
        print_scaled(st, b.shift_amount(&st.mem));
    }
    if is_hlist && b.subtype(&st.mem) == DLIST {
        print(st, ", display");
    }
    node_list_display(st, b'.', b.list_ptr(&st.mem));
}

fn show_whatsit(st: &mut EngineState, node: NodePtr<WhatsitNode>) {
    let print_font = |st: &mut EngineState| {
        print_esc(st, st.eqtb.font_id_text(node.native_font(&st.mem)));
    };
    match node.whatsit_subtype(&st.mem) {
        Some(WhatsitSubtype::Open) => {
            print_write_whatsit(st, "openout", node);
            print_char(st, b'=');
            let (name, area, ext) = (
                node.open_name(&st.mem),
                node.open_area(&st.mem),
                node.open_ext(&st.mem),
            );
            print_file_name(st, name, area, ext);
        }
        Some(WhatsitSubtype::Write) => {
            print_write_whatsit(st, "write", node);
            print_mark(st, node.write_tokens(&st.mem));
        }
        Some(WhatsitSubtype::Close) => print_write_whatsit(st, "closeout", node),
        Some(WhatsitSubtype::Special) => {
            print_esc(st, "special");
            print_mark(st, node.write_tokens(&st.mem));
        }
        Some(WhatsitSubtype::Language) => {
            print_esc(st, "setlanguage");
            print_int(st, node.what_lang(&st.mem));
            print(st, " (hyphenmin ");
            print_int(st, i32::from(node.what_lhm(&st.mem)));
            print_char(st, b',');
            print_int(st, i32::from(node.what_rhm(&st.mem)));
            print_char(st, b')');
        }
        Some(WhatsitSubtype::PdfSavePos) => print_esc(st, "pdfsavepos"),
        Some(WhatsitSubtype::NativeWord | WhatsitSubtype::NativeWordAt) => {
            print_font(st);
            print_char(st, b' ');
            print_native_word(st, node);
        }
        Some(WhatsitSubtype::Glyph) => {
            print_font(st);
            print(st, " glyph#");
            print_int(st, i32::from(node.native_glyph(&st.mem)));
        }
        Some(sub @ (WhatsitSubtype::Pic | WhatsitSubtype::Pdf)) => {
            if sub == WhatsitSubtype::Pic {
                print_esc(st, "XeTeXpicfile");
            } else {
                print_esc(st, "XeTeXpdffile");
            }
            print(st, " \"");
            for i in 0..node.pic_path_len(&st.mem) as usize {
                let b = node.pic_path(&st.mem)[i];
                print_raw_char(st, i32::from(b), true);
            }
            print(st, b'"');
        }
        _ => print(st, "whatsit?"),
    }
}

fn show_glue(st: &mut EngineState, node: NodePtr<crate::node::GlueNode>) {
    let sub = node.subtype(&st.mem);
    if sub >= A_LEADERS {
        print_esc(st, "");
        if sub == C_LEADERS {
            print_char(st, b'c');
        } else if sub == X_LEADERS {
            print_char(st, b'x');
        }
        print(st, "leaders ");
        print_spec(st, node.glue_ptr(&st.mem).ptr(), None);
        node_list_display(st, b'.', node.leader_ptr(&st.mem));
        return;
    }
    print_esc(st, "glue");
    if sub != 0 {
        print_char(st, b'(');
        if sub < COND_MATH_GLUE {
            print_skip_param(st, i32::from(sub) - 1);
        } else if sub == COND_MATH_GLUE {
            print_esc(st, "nonscript");
        } else {
            print_esc(st, "mskip");
        }
        print_char(st, b')');
    }
    if sub != COND_MATH_GLUE {
        print_char(st, b' ');
        let unit = (sub >= COND_MATH_GLUE).then_some("mu");
        print_spec(st, node.glue_ptr(&st.mem).ptr(), unit);
    }
}

fn show_noad(st: &mut EngineState, node: NodePtr<crate::node::Noad>) {
    let nt = node.noad_type(&st.mem);
    if nt == NodeType::FractionNoad {
        print_esc(st, "fraction, thickness ");
        let thickness = node.thickness(&st.mem);
        if thickness == DEFAULT_CODE {
            print(st, "= default");
        } else {
            print_scaled(st, thickness);
        }
        let left = node.left_delimiter();
        if !left.is_null(&st.mem) {
            print(st, ", left-delimiter ");
            print_delimiter(st, left);
        }
        let right = node.right_delimiter();
        if !right.is_null(&st.mem) {
            print(st, ", right-delimiter ");
            print_delimiter(st, right);
        }
        print_subsidiary_data(st, node.numerator(), b'\\');
        print_subsidiary_data(st, node.denominator(), b'/');
        return;
    }

    let name = match nt {
        NodeType::OrdNoad => "mathord",
        NodeType::OpNoad => "mathop",
        NodeType::BinNoad => "mathbin",
        NodeType::RelNoad => "mathrel",
        NodeType::OpenNoad => "mathopen",
        NodeType::CloseNoad => "mathclose",
        NodeType::PunctNoad => "mathpunct",
        NodeType::InnerNoad => "mathinner",
        NodeType::OverNoad => "overline",
        NodeType::UnderNoad => "underline",
        NodeType::VcenterNoad => "vcenter",
        NodeType::RadicalNoad => "radical",
        NodeType::AccentNoad => "accent",
        NodeType::LeftNoad => "left",
        NodeType::RightNoad if node.subtype(&st.mem) == 0 => "right",
        NodeType::RightNoad => "middle",
        _ => "",
    };
    if !name.is_empty() {
        print_esc(st, name);
    }
    match nt {
        NodeType::RadicalNoad => print_delimiter(st, node.left_delimiter()),
        NodeType::AccentNoad => print_fam_and_char(st, node.accent_chr()),
        NodeType::LeftNoad | NodeType::RightNoad => print_delimiter(st, node.delimiter()),
        _ => {}
    }
    if !matches!(nt, NodeType::LeftNoad | NodeType::RightNoad) {
        let sub = node.subtype(&st.mem);
        if sub != 0 {
            print_esc(
                st,
                if sub == LIMITS as u16 {
                    "limits"
                } else {
                    "nolimits"
                },
            );
        }
        print_subsidiary_data(st, node.nucleus(), b'.');
    }
    print_subsidiary_data(st, node.supscr(), b'^');
    print_subsidiary_data(st, node.subscr(), b'_');
}

/// The label shown for a token list of type `index` in the error context.
fn token_list_label(index: u16, loc: i32) -> Option<&'static str> {
    Some(match index {
        PARAMETER => "<argument> ",
        U_TEMPLATE | V_TEMPLATE => "<template> ",
        BACKED_UP | BACKED_UP_CHAR if loc == TEX_NULL => "<recently read> ",
        BACKED_UP | BACKED_UP_CHAR => "<to be read again> ",
        INSERTED => "<inserted text> ",
        OUTPUT_TEXT => "<output> ",
        EVERY_PAR_TEXT => "<everypar> ",
        EVERY_MATH_TEXT => "<everymath> ",
        EVERY_DISPLAY_TEXT => "<everydisplay> ",
        EVERY_HBOX_TEXT => "<everyhbox> ",
        EVERY_VBOX_TEXT => "<everyvbox> ",
        EVERY_JOB_TEXT => "<everyjob> ",
        EVERY_CR_TEXT => "<everycr> ",
        MARK_TEXT => "<mark> ",
        EVERY_EOF_TEXT => "<everyeof> ",
        INTER_CHAR_TEXT => "<XeTeXinterchartoks> ",
        WRITE_TEXT => "<write> ",
        TECTONIC_CODA_TEXT => "<TectonicCodaTokens> ",
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

pub(crate) fn show_context(st: &mut EngineState) {
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
        let error_context_lines = st.eqtb.int_par(IntPar::ErrorContextLines);
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
                                print_nl(st, "<*>");
                            } else {
                                print_nl(st, "<insert> ");
                            }
                        } else {
                            print_nl(st, "<read ");
                            if ci.name == 17 {
                                print_char(st, b'*');
                            } else {
                                print_int(st, ci.name - 1);
                            }
                            print_char(st, b'>');
                        }
                    } else {
                        print_nl(st, "l.");
                        if i32::from(ci.index) == st.input.in_open {
                            print_int(st, st.mem.line);
                        } else {
                            print_int(st, st.input.line_stack[ci.index as usize + 1]);
                        }
                    }
                    print_char(st, b' ');
                    l = begin_pseudoprint(st);
                    let end_line_char = st.eqtb.int_par(IntPar::EndLineChar);
                    let j = if st.input.buffer[ci.limit as usize] == end_line_char {
                        ci.limit
                    } else {
                        ci.limit + 1
                    };
                    for i in ci.start..j {
                        if i == ci.loc {
                            set_trick_count(st);
                        }
                        print_char(st, st.input.buffer[i as usize]);
                    }
                } else {
                    match ci.index {
                        MACRO => {
                            print_ln(st);
                            print_cs(st, ci.name);
                        }
                        index => match token_list_label(index, ci.loc) {
                            Some(label) => print_nl(st, label),
                            None => print_nl(st, b'?'),
                        },
                    }
                    l = begin_pseudoprint(st);
                    if ci.index < MACRO {
                        show_token_list(st, ci.start, ci.loc, 100_000);
                    } else {
                        let list = MemPtr(ci.start).link(&st.mem);
                        show_token_list(st, list, ci.loc, 100_000);
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
                    print(st, "...");
                    (
                        l + first_count - st.errors.half_error_line + 3,
                        st.errors.half_error_line,
                    )
                };
                for q in start..first_count {
                    print_char(st, st.errors.trick_buf[(q % error_line) as usize]);
                }
                print_ln(st);
                for _ in 0..n {
                    print_raw_char(st, b' ', true);
                }
                let end = if m + n <= error_line {
                    first_count + m
                } else {
                    first_count + (error_line - n - 3)
                };
                for q in first_count..end {
                    print_char(st, st.errors.trick_buf[(q % error_line) as usize]);
                }
                if m + n > error_line {
                    print(st, "...");
                }
                nn += 1;
            }
        } else if nn == error_context_lines {
            print_nl(st, "...");
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
