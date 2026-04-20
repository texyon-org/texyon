//! Scanner range-validation helpers and mu_error.
//!
//! Translated from xetex-xetex0.c.

use crate::globals::{
    align_state, cur_chr, cur_cmd, cur_cs, cur_tok, cur_val, eqtb_b32s1, eqtb_is_null,
    max_reg_help_line, max_reg_num, mem_b32s0, mem_b32s1, no_new_control_sequence,
    set_align_state, set_cur_chr, set_cur_cmd, set_cur_cs, set_cur_tok, set_cur_val, set_help_line,
    set_help_ptr, set_mem_b32s0, set_mem_b32s1, set_no_new_control_sequence,
};
use crate::mem::new_rule;

extern "C" {
    fn expand();
    fn get_next();
    fn macro_call();
    fn begin_token_list(p: i32, t: u16);
    fn scan_dimen(mu: bool, inf: bool, shortcut: bool);
    fn scan_int();
    fn scan_glue(level: i16);
    fn scan_toks(macro_def: bool, xpand: bool) -> i32;
    fn error_here_with_diagnostic(msg: *const libc::c_char) -> *mut std::ffi::c_void;
    fn capture_to_diagnostic(diag: *mut std::ffi::c_void);
    fn int_error(n: i32);
    fn ttstub_diag_printf(
        d: *mut std::ffi::c_void,
        fmt: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn error();
}

// Value-type constants for scan_glue (xetex_format.h:568–570).
const GLUE_VAL: i16 = 2;
const MU_VAL_SCAN: i16 = 3;

// INT_BASE + INT_PAR__xetex_hyphenatable_length offset for max_hyphenatable_length
// (xetex_format.h:568, xetex_format.h:115; xetex_format.h:7826729 = INT_BASE).
const INT_BASE: i32 = 7_826_729;
const INT_PAR_XETEX_HYPHENATABLE_LENGTH: i32 = 80;    // xetex_format.h:115
const INT_PAR_XETEX_INPUT_NORMALIZATION: i32 = 74;    // xetex_format.h:109
const INT_PAR_XETEX_TRACING_FONTS: i32 = 77;          // xetex_format.h:112
const HYPHENATABLE_LENGTH_LIMIT: i32 = 4095; // xetex-constants.h:334

// xetex-constants.h:26
const BIGGEST_USV: i32 = 0x10_FFFF;
// xetex-constants.h:335
const CHAR_CLASS_LIMIT: i32 = 4096;
// xetex-constants.h:343
const ACTIVE_MATH_CHAR: i32 = 0x1F_FFFF;
// xetex-constants.h:25
const BIGGEST_CHAR: i32 = 0xFFFF;
// xetex_format.h (NUMBER_MATH_FAMILIES)
const NUMBER_MATH_FAMILIES: i32 = 256;

// translated from xetex-xetex0.c:5050 (mu_error)
#[no_mangle]
pub unsafe extern "C" fn mu_error() {
    error_here_with_diagnostic(c"Incompatible glue units".as_ptr());
    capture_to_diagnostic(std::ptr::null_mut());
    set_help_ptr(1);
    set_help_line(0, c"I'm going to assume that 1mu=1pt when they're mixed.".as_ptr());
    error();
}

// translated from xetex-xetex0.c:5078 (scan_char_class)
#[no_mangle]
pub unsafe extern "C" fn scan_char_class() {
    scan_int();
    if cur_val() < 0 || cur_val() > CHAR_CLASS_LIMIT {
        let errmsg = error_here_with_diagnostic(c"Bad character class".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(1, c"A character class must be between 0 and 4096.".as_ptr());
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// translated from xetex-xetex0.c:5096 (scan_char_class_not_ignored)
#[no_mangle]
pub unsafe extern "C" fn scan_char_class_not_ignored() {
    scan_int();
    if cur_val() < 0 || cur_val() > CHAR_CLASS_LIMIT {
        let errmsg = error_here_with_diagnostic(c"Bad character class".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"A class for inter-character transitions must be between 0 and 4095.".as_ptr(),
        );
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// translated from xetex-xetex0.c:5114 (scan_eight_bit_int)
#[no_mangle]
pub unsafe extern "C" fn scan_eight_bit_int() {
    scan_int();
    if cur_val() < 0 || cur_val() > 255 {
        let errmsg = error_here_with_diagnostic(c"Bad register code".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"A register code or char class must be between 0 and 255.".as_ptr(),
        );
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// translated from xetex-xetex0.c:5132 (scan_usv_num)
#[no_mangle]
pub unsafe extern "C" fn scan_usv_num() {
    scan_int();
    if cur_val() < 0 || cur_val() > BIGGEST_USV {
        let errmsg = error_here_with_diagnostic(c"Bad character code".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"A Unicode scalar value must be between 0 and \"10FFFF.".as_ptr(),
        );
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// translated from xetex-xetex0.c:5150 (scan_char_num)
#[no_mangle]
pub unsafe extern "C" fn scan_char_num() {
    scan_int();
    if cur_val() < 0 || cur_val() > BIGGEST_CHAR {
        let errmsg = error_here_with_diagnostic(c"Bad character code".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"A character number must be between 0 and 65535.".as_ptr(),
        );
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// translated from xetex-xetex0.c:5330 (scan_math_class_int)
#[no_mangle]
pub unsafe extern "C" fn scan_math_class_int() {
    scan_int();
    if cur_val() < 0 || cur_val() > 7 {
        let errmsg = error_here_with_diagnostic(c"Bad math class".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"Since I expected to read a number between 0 and 7,".as_ptr(),
        );
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// translated from xetex-xetex0.c:5348 (scan_math_fam_int)
#[no_mangle]
pub unsafe extern "C" fn scan_math_fam_int() {
    scan_int();
    if cur_val() < 0 || cur_val() > NUMBER_MATH_FAMILIES - 1 {
        let errmsg = error_here_with_diagnostic(c"Bad math family".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"Since I expected to read a number between 0 and 255,".as_ptr(),
        );
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// translated from xetex-xetex0.c:5366 (scan_four_bit_int)
#[no_mangle]
pub unsafe extern "C" fn scan_four_bit_int() {
    scan_int();
    if cur_val() < 0 || cur_val() > 15 {
        let errmsg = error_here_with_diagnostic(c"Bad number".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"Since I expected to read a number between 0 and 15,".as_ptr(),
        );
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// translated from xetex-xetex0.c:5384 (scan_fifteen_bit_int)
#[no_mangle]
pub unsafe extern "C" fn scan_fifteen_bit_int() {
    scan_int();
    if cur_val() < 0 || cur_val() > 32767 {
        let errmsg = error_here_with_diagnostic(c"Bad mathchar".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(1, c"A mathchar number must be between 0 and 32767.".as_ptr());
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// translated from xetex-xetex0.c:5168 (scan_xetex_math_char_int)
// math_char(x) = (x as u32) & 0x1FFFFF (xetex-xetexd.h:1205)
#[no_mangle]
pub unsafe extern "C" fn scan_xetex_math_char_int() {
    scan_int();
    let mc = (cur_val() as u32) & 0x1F_FFFF;
    if mc == ACTIVE_MATH_CHAR as u32 {
        if cur_val() != ACTIVE_MATH_CHAR {
            let errmsg = error_here_with_diagnostic(c"Bad active XeTeX math code".as_ptr());
            ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(2);
            set_help_line(
                1,
                c"Since I ignore class and family for active math chars,".as_ptr(),
            );
            set_help_line(0, c"I changed this one to \"1FFFFF.".as_ptr());
            int_error(cur_val());
            set_cur_val(ACTIVE_MATH_CHAR);
        }
    } else if mc > BIGGEST_USV as u32 {
        let errmsg = error_here_with_diagnostic(c"Bad XeTeX math character code".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"Since I expected a character number between 0 and \"10FFFF,".as_ptr(),
        );
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// translated from xetex-xetex0.c:5245 (scan_register_num)
#[no_mangle]
pub unsafe extern "C" fn scan_register_num() {
    scan_int();
    if cur_val() < 0 || cur_val() > max_reg_num() {
        let errmsg = error_here_with_diagnostic(c"Bad register code".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(1, max_reg_help_line());
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// translated from xetex-xetex0.c:5263 (scan_four_bit_int_or_18)
#[no_mangle]
pub unsafe extern "C" fn scan_four_bit_int_or_18() {
    scan_int();
    if cur_val() < 0 || (cur_val() > 15 && cur_val() != 18) {
        let errmsg = error_here_with_diagnostic(c"Bad number".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"Since I expected to read a number between 0 and 15,".as_ptr(),
        );
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// translated from xetex-xetex0.c:5403 (scan_delimiter_int)
#[no_mangle]
pub unsafe extern "C" fn scan_delimiter_int() {
    scan_int();
    if cur_val() < 0 || cur_val() > 0x7FF_FFFF {
        let errmsg = error_here_with_diagnostic(c"Bad delimiter code".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"A numeric delimiter code must be between 0 and 2^{27}-1.".as_ptr(),
        );
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// ---------------------------------------------------------------------------
// max_hyphenatable_length
// ---------------------------------------------------------------------------

/// Returns the maximum hyphenatable word length, clamped to HYPHENATABLE_LENGTH_LIMIT.
///
/// translated from xetex-xetex0.c:11330 (max_hyphenatable_length)
#[no_mangle]
pub unsafe extern "C" fn max_hyphenatable_length() -> i32 {
    let v = eqtb_b32s1(INT_BASE + INT_PAR_XETEX_HYPHENATABLE_LENGTH);
    if v > HYPHENATABLE_LENGTH_LIMIT { HYPHENATABLE_LENGTH_LIMIT } else { v }
}

// ---------------------------------------------------------------------------
// scan_normal_glue / scan_mu_glue
// ---------------------------------------------------------------------------

/// Scan a glue specification (non-math).
///
/// translated from xetex-xetex0.c:6519 (scan_normal_glue)
#[no_mangle]
pub unsafe extern "C" fn scan_normal_glue() {
    scan_glue(GLUE_VAL);
}

/// Scan a mu-glue specification (math mode).
///
/// translated from xetex-xetex0.c:6524 (scan_mu_glue)
#[no_mangle]
pub unsafe extern "C" fn scan_mu_glue() {
    scan_glue(MU_VAL_SCAN);
}

// ---------------------------------------------------------------------------
// get_input_normalization_state / get_tracing_fonts_state / scan_pdf_ext_toks
// ---------------------------------------------------------------------------

/// Returns INTPAR(xetex_input_normalization), or 0 if eqtb is not yet initialized.
///
/// translated from xetex-xetex0.c:9073 (get_input_normalization_state)
#[no_mangle]
pub unsafe extern "C" fn get_input_normalization_state() -> i32 {
    if eqtb_is_null() { 0 } else { eqtb_b32s1(INT_BASE + INT_PAR_XETEX_INPUT_NORMALIZATION) }
}

/// Returns INTPAR(xetex_tracing_fonts).
///
/// translated from xetex-xetex0.c:9082 (get_tracing_fonts_state)
#[no_mangle]
pub unsafe extern "C" fn get_tracing_fonts_state() -> i32 {
    eqtb_b32s1(INT_BASE + INT_PAR_XETEX_TRACING_FONTS)
}

/// Scan a PDF extension token list (xpand=true, macro_def=false).
///
/// translated from xetex-xetex0.c:15945 (scan_pdf_ext_toks)
#[no_mangle]
pub unsafe extern "C" fn scan_pdf_ext_toks() {
    scan_toks(false, true);
}

// ---------------------------------------------------------------------------
// x_token / scan_optional_equals
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// get_token
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:3653 (get_token)
#[no_mangle]
pub unsafe extern "C" fn get_token() {
    set_no_new_control_sequence(false);
    get_next();
    set_no_new_control_sequence(true);
    if cur_cs() == 0 {
        set_cur_tok(cur_cmd() as i32 * MAX_CHAR_VAL + cur_chr());
    } else {
        set_cur_tok(CS_TOKEN_FLAG + cur_cs());
    }
}

// Command-code constants (xetex_format.h / xetex-constants.h).
const MAX_COMMAND: u8 = 102;
const MAX_CHAR_VAL: i32 = 0x200000; // 1 << 21
const CS_TOKEN_FLAG: i32 = 0x1FFFFFF;
const SPACER: u8 = 10;
const OTHER_TOKEN: i32 = 0x1800000; // OTHER_CHAR << 21
const CALL: u8 = 113;
const END_TEMPLATE: u8 = 117;
const ENDV: u8 = 9;
// FROZEN_CONTROL_SEQUENCE + 6 (xetex_format.h:201, xetex-constants.h:61)
const FROZEN_ENDV: i32 = 2_243_226 + 6;
// END_MATCH_TOKEN + 1 (xetex-constants.h:360–361)
const PROTECTED_TOKEN: i32 = 0x1C00001;

/// translated from xetex-xetex0.c:4316 (x_token)
#[no_mangle]
pub unsafe extern "C" fn x_token() {
    while cur_cmd() > MAX_COMMAND {
        expand();
        get_next();
    }
    if cur_cs() == 0 {
        set_cur_tok(cur_cmd() as i32 * MAX_CHAR_VAL + cur_chr());
    } else {
        set_cur_tok(CS_TOKEN_FLAG + cur_cs());
    }
}

// Token-encoding constants (xetex-constants.h:353).
const MATH_SHIFT_TOKEN: i32 = 0x600000;

/// translated from xetex-xetex0.c:11444 (insert_dollar_sign)
#[no_mangle]
pub unsafe extern "C" fn insert_dollar_sign() {
    crate::input::back_input();
    set_cur_tok(MATH_SHIFT_TOKEN + '$' as i32);
    crate::output::error_here_with_diagnostic(c"Missing $ inserted".as_ptr());
    crate::output::capture_to_diagnostic(std::ptr::null_mut());
    set_help_ptr(2);
    set_help_line(1, c"I've inserted a begin-math/end-math symbol since I think".as_ptr());
    set_help_line(0, c"you left one out. Proceed, with fingers crossed.".as_ptr());
    crate::input::ins_error();
}

/// translated from xetex-xetex0.c:4355 (scan_optional_equals)
#[no_mangle]
pub unsafe extern "C" fn scan_optional_equals() {
    loop {
        get_x_token();
        if cur_cmd() != SPACER {
            break;
        }
    }
    if cur_tok() != OTHER_TOKEN + '=' as i32 {
        crate::input::back_input();
    }
}

// Command-code values for scan_left_brace (xetex_format.h).
const RELAX: u8 = 0;
const LEFT_BRACE: u8 = 1;
const LEFT_BRACE_TOKEN: i32 = 0x200000;

/// translated from xetex-xetex0.c:4319 (scan_left_brace)
#[no_mangle]
pub unsafe extern "C" fn scan_left_brace() {
    loop {
        get_x_token();
        if cur_cmd() != SPACER && cur_cmd() != RELAX {
            break;
        }
    }
    if cur_cmd() != LEFT_BRACE {
        crate::output::error_here_with_diagnostic(c"Missing { inserted".as_ptr());
        crate::output::capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(4);
        set_help_line(3, c"A left brace was mandatory here, so I've put one in.".as_ptr());
        set_help_line(2, c"You might want to delete and/or insert some corrections".as_ptr());
        set_help_line(1, c"so that I will find a matching right brace soon.".as_ptr());
        set_help_line(0, c"(If you're confused by all this, try typing `I}' now.)".as_ptr());
        crate::input::back_error();
        set_cur_tok(LEFT_BRACE_TOKEN + '{' as i32);
        set_cur_cmd(LEFT_BRACE);
        set_cur_chr('{' as i32);
        set_align_state(align_state() + 1);
    }
}

// ---------------------------------------------------------------------------
// get_x_token / get_x_or_protected
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:4272 (get_x_token)
#[no_mangle]
pub unsafe extern "C" fn get_x_token() {
    loop {
        get_next();
        if cur_cmd() <= MAX_COMMAND {
            break;
        }
        if cur_cmd() >= CALL {
            if cur_cmd() < END_TEMPLATE {
                macro_call();
            } else {
                set_cur_cs(FROZEN_ENDV);
                set_cur_cmd(ENDV);
                break;
            }
        } else {
            expand();
        }
    }
    if cur_cs() == 0 {
        set_cur_tok(cur_cmd() as i32 * MAX_CHAR_VAL + cur_chr());
    } else {
        set_cur_tok(CS_TOKEN_FLAG + cur_cs());
    }
}

/// translated from xetex-xetex0.c:4533 (get_x_or_protected)
#[no_mangle]
pub unsafe extern "C" fn get_x_or_protected() {
    loop {
        get_token();
        if cur_cmd() <= MAX_COMMAND {
            return;
        }
        if cur_cmd() >= CALL && cur_cmd() < END_TEMPLATE {
            if mem_b32s0(mem_b32s1(cur_chr())) == PROTECTED_TOKEN {
                return;
            }
        }
        expand();
    }
}

// ---------------------------------------------------------------------------
// scan_keyword
// ---------------------------------------------------------------------------

// MEM_TOP - 13 (xetex_format.h:30, xetex-constants.h:50)
const BACKUP_HEAD: i32 = 4_999_999 - 13;
// xetex-constants.h:202
const BACKED_UP: u16 = 3;

/// translated from xetex-xetex0.c:4284 (scan_keyword)
#[no_mangle]
pub unsafe extern "C" fn scan_keyword(s: *const libc::c_char) -> bool {
    let mut p = BACKUP_HEAD;
    set_mem_b32s1(p, crate::mem::TEX_NULL);

    let slen = libc::strlen(s);

    if slen == 1 {
        let c = *s as i32;
        let save_cur_cs = cur_cs();
        loop {
            get_x_token();
            if cur_cs() == 0 && (cur_chr() == c || cur_chr() == c - 32) {
                let q = crate::mem::get_avail();
                set_mem_b32s1(p, q);
                set_mem_b32s0(q, cur_tok());
                p = q;
                crate::mem::flush_list(mem_b32s1(BACKUP_HEAD));
                return true;
            } else if cur_cmd() != SPACER || p != BACKUP_HEAD {
                back_input();
                if p != BACKUP_HEAD {
                    begin_token_list(mem_b32s1(BACKUP_HEAD), BACKED_UP);
                }
                set_cur_cs(save_cur_cs);
                return false;
            }
        }
    }

    let mut i = 0usize;
    while i < slen {
        get_x_token();
        let ch = *s.add(i) as i32;
        if cur_cs() == 0 && (cur_chr() == ch || cur_chr() == ch - 32) {
            let q = crate::mem::get_avail();
            set_mem_b32s1(p, q);
            set_mem_b32s0(q, cur_tok());
            p = q;
            i += 1;
        } else if cur_cmd() != SPACER || p != BACKUP_HEAD {
            back_input();
            if p != BACKUP_HEAD {
                begin_token_list(mem_b32s1(BACKUP_HEAD), BACKED_UP);
            }
            return false;
        }
    }
    crate::mem::flush_list(mem_b32s1(BACKUP_HEAD));
    true
}

// ---------------------------------------------------------------------------
// scan_rule_spec
// ---------------------------------------------------------------------------

// xetex_format.h:273 / xetex-constants.h:316,339
const VRULE: u8 = 35;
const DEFAULT_RULE: i32 = 26214;

/// translated from xetex-xetex0.c:6267 (scan_rule_spec)
#[no_mangle]
pub unsafe extern "C" fn scan_rule_spec() -> i32 {
    let q = new_rule();
    if cur_cmd() == VRULE {
        set_mem_b32s1(q + 1, DEFAULT_RULE);
    } else {
        set_mem_b32s1(q + 3, DEFAULT_RULE);
        set_mem_b32s1(q + 2, 0);
    }
    loop {
        if scan_keyword(c"width".as_ptr()) {
            scan_dimen(false, false, false);
            set_mem_b32s1(q + 1, cur_val());
        } else if scan_keyword(c"height".as_ptr()) {
            scan_dimen(false, false, false);
            set_mem_b32s1(q + 3, cur_val());
        } else if scan_keyword(c"depth".as_ptr()) {
            scan_dimen(false, false, false);
            set_mem_b32s1(q + 2, cur_val());
        } else {
            break;
        }
    }
    q
}

#[inline]
unsafe fn back_input() {
    crate::input::back_input();
}
