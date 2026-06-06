//! Expansion, macros, and conditionals — `expand`/`macro_call`/`conditional`/
//! `pop_cond_node`/`conv_toks`/`ins_the_toks`. Relocated from `scan.rs`
//! (= xetex0.c's expansion group) by workstream 02 (module decomposition).

// ---------------------------------------------------------------------------
// conditional (IF_ sub-codes — xetex_format.h:543–564)
// ---------------------------------------------------------------------------
const IF_CHAR_CODE: i16 = 0;
const IF_CAT_CODE: i16 = 1;
const IF_INT_CODE: i16 = 2;
const IF_DIM_CODE: i16 = 3;
const IF_ODD_CODE: i16 = 4;
const IF_VMODE_CODE: i16 = 5;
const IF_HMODE_CODE: i16 = 6;
const IF_MMODE_CODE: i16 = 7;
const IF_INNER_CODE: i16 = 8;
const IF_VOID_CODE: i16 = 9;
const IF_HBOX_CODE: i16 = 10;
const IF_VBOX_CODE: i16 = 11;
const IFX_CODE: i16 = 12;
const IF_EOF_CODE: i16 = 13;
const IF_TRUE_CODE: i16 = 14;
const IF_FALSE_CODE: i16 = 15;
const IF_CASE_CODE: i16 = 16;
const IF_DEF_CODE: i16 = 17;
const IF_CS_CODE: i16 = 18;
const IF_FONT_CHAR_CODE: i16 = 19;
const IF_IN_CSNAME_CODE: i16 = 20;
const IF_PRIMITIVE_CODE: i16 = 21;
// Node / command codes for conditional.
const IF_NODE_SIZE: i32 = 2;
const IF_CODE: u8 = 1; // xetex-constants.h:267
const VLIST_NODE_C: u16 = 1; // xetex_format.h:381
const HMODE_C: i16 = 104; // xetex_format.h:369
const MMODE_C: i16 = 207; // xetex_format.h:370
const ACTIVE_CHAR_C: u8 = 13; // xetex_format.h:246
const UNDEFINED_CS: u8 = 103; // xetex_format.h:344
const END_CS_NAME: u8 = 67; // xetex_format.h:305
const NULL_CS: i32 = 2_228_225; // xetex_format.h:199
const TOO_BIG_USV_C: i32 = 1_114_112; // xetex_format.h:22 (BIGGEST_USV + 1)
const NORMAL_SCAN: u8 = 0; // xetex-constants.h:182 / xetex_format.h:420
const INT_PAR_TRACING_COMMANDS: i32 = 36; // xetex_format.h:71

// Module-private consts shared with scan.rs (duplicated, same value).
const GARBAGE: i32 = 4_999_999 - 12; // MEM_TOP - 12 (xetex-constants.h:49)
const TEMP_HEAD: i32 = 4_999_999 - 3; // MEM_TOP - 3
const INSERTED: u16 = 5; // xetex-constants.h:204

use crate::consts::{
    ACTIVE_BASE, BOX_BASE, CS_TOKEN_FLAG, DISC_NODE, GLUE_NODE, HLIST_NODE, INT_BASE, KERN_NODE,
    MATH_NODE, PENALTY_NODE, PRIM_EQTB_BASE, SINGLE_BASE, TOO_BIG_CHAR,
};

// Command/token/code consts that live module-private in scan.rs (duplicated
// here, same values — they are referenced bare by the moved expander match arms).
const CALL: u8 = 113;
const RELAX: u8 = 0;
const SPACER: u8 = 10;
const VMODE: i16 = 1;
const OTHER_TOKEN: i32 = 0x1800000; // OTHER_CHAR << 21
const MAX_COMMAND: u8 = 102;
const BACKUP_HEAD: i32 = 4_999_999 - 13;
const BIGGEST_USV: i32 = 0x10_FFFF;
const ELSE_CODE: u8 = 3;
const END_TEMPLATE: u8 = 117;
const FI_CODE: u8 = 2;
const FROZEN_ENDV: i32 = 2_243_226 + 6;
const FROZEN_PRIMITIVE: i32 = 2_243_237; // FROZEN_CONTROL_SEQUENCE + 11
const HASH_BASE: i32 = 2_228_226;
const MAX_CHAR_VAL: i32 = 0x200000; // 1 << 21
const NO_EXPAND_FLAG: i32 = 0x10_FFFF + 2;
const OR_CODE: u8 = 4;
const UNDEFINED_PRIMITIVE: i32 = 0;
const UNLESS_CODE: i32 = 32;
const AAT_FONT_FLAG: i32 = 0xFFFF;
const OTGR_FONT_FLAG: i32 = 0xFFFE;
const END_MATCH_TOKEN: i32 = 0x1C00000;
const MATCH_TOKEN: i32 = 0x1A00000;
const RIGHT_BRACE_LIMIT: i32 = 0x600000;
const LEFT_BRACE_LIMIT: i32 = 0x400000;
const SPACE_TOKEN: i32 = 0x1400020;
const PROTECTED_TOKEN: i32 = 0x1C00001;
const INT_PAR_TRACING_IFS: i32 = 58;
use crate::errors::{
    error, int_error, not_aat_font_error, not_aat_gr_font_error, not_native_font_error, runaway,
};
use crate::ext::{
    aat_print_font_name, gr_print_font_name, map_char_to_glyph_global, print_glyph_name,
};
use crate::globals::{
    buffer_entry, cur_chr, cur_cmd, cur_cs, cur_input, cur_tok, cur_val, eqtb_b32s1, expand_depth,
    expand_depth_count, first, mem_b16s0, mem_b16s1, mem_b32s0, mem_b32s1, name_in_progress,
    pool_size, scanner_status, set_buffer_entry, set_cur_chr, set_cur_cmd, set_cur_cs,
    set_cur_input_loc, set_cur_input_start, set_cur_tok, set_cur_val, set_expand_depth_count,
    set_help_line, set_help_ptr, set_mem_b32s0, set_mem_b32s1, set_scanner_status,
};
use crate::input::begin_token_list;
use crate::io::start_input;
use crate::output::{capture_to_diagnostic, error_here_with_diagnostic, print_cmd_chr};
use crate::scan::{
    effective_char_global, get_next, get_token, get_x_token, pass_text, pseudo_start, scan_dimen,
    scan_font_ident, scan_four_bit_int_or_18, scan_int, scan_keyword, scan_pdf_ext_toks,
    scan_register_num, scan_usv_num, the_toks,
};

// Command codes used bare in expand/conditional matches (xetex_format.h:352).
const IF_TEST: u8 = 107;
const NORMAL_SCANNER: u8 = 0;
const SELECTOR_NEW_STRING: i32 = 21;

extern "C" {
    fn usingGraphite(engine: *mut libc::c_void) -> bool;
}

// ---- moved verbatim from scan.rs (workstream 02) ----

/// translated from xetex-xetex0.c:6511 (ins_the_toks)
pub unsafe fn ins_the_toks() {
    set_mem_b32s1(GARBAGE, the_toks());
    begin_token_list(mem_b32s1(TEMP_HEAD), INSERTED);
}
/// translated from xetex-xetex0.c:6271 (conditional)
pub unsafe fn conditional() {
    use crate::globals::{
        buf_size, buffer_entry, char_base_entry, cond_ptr, cur_chr, cur_cmd, cur_cs, cur_if,
        cur_list_mode, eqtb_b16s1, eqtb_b32s1, first, font_area_entry, font_bc_entry,
        font_ec_entry, font_info_b16, hash_s1, if_limit, if_line, if_stack, in_open, is_in_csname,
        line, max_buf_stack, mem_b16s1, mem_b32s0, mem_b32s1, read_open_entry, scanner_status,
        set_buffer_entry, set_cond_ptr, set_cur_chr, set_cur_cmd, set_cur_if, set_if_limit,
        set_if_line, set_is_in_csname, set_max_buf_stack, set_mem_b16s0, set_mem_b16s1,
        set_mem_b32s1, set_scanner_status, shell_escape_enabled,
    };
    use crate::mem::{flush_list_global, get_avail_global, get_node_global, TEX_NULL};

    let mut b = false;
    let e: bool;
    let mut r: u8 = 0;
    let mut m: i32 = 0;
    let mut n: i32 = 0;
    let mut p: i32;
    let mut q: i32;
    let save_scanner_status: u8;

    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_IFS) > 0
        && eqtb_b32s1(INT_BASE + INT_PAR_TRACING_COMMANDS) <= 1
    {
        crate::output::show_cur_cmd_chr();
    }

    p = get_node_global(IF_NODE_SIZE);
    set_mem_b32s1(p, cond_ptr());
    set_mem_b16s1(p, if_limit() as u16);
    set_mem_b16s0(p, cur_if() as u16);
    set_mem_b32s1(p + 1, if_line());
    set_cond_ptr(p);
    set_cur_if(cur_chr() as i16);
    set_if_limit(IF_CODE);
    set_if_line(line());

    let save_cond_ptr: i32 = cond_ptr();
    let is_unless: bool = cur_chr() >= UNLESS_CODE;
    let this_if: i16 = (cur_chr() % UNLESS_CODE) as i16;

    match this_if {
        IF_CHAR_CODE | IF_CAT_CODE => {
            get_x_token();
            if cur_cmd() == RELAX && cur_chr() == NO_EXPAND_FLAG {
                set_cur_cmd(ACTIVE_CHAR_C);
                set_cur_chr(cur_tok() - (CS_TOKEN_FLAG + ACTIVE_BASE));
            }
            if cur_cmd() > ACTIVE_CHAR_C || cur_chr() > BIGGEST_USV {
                m = RELAX as i32;
                n = TOO_BIG_USV_C;
            } else {
                m = cur_cmd() as i32;
                n = cur_chr();
            }
            get_x_token();
            if cur_cmd() == RELAX && cur_chr() == NO_EXPAND_FLAG {
                set_cur_cmd(ACTIVE_CHAR_C);
                set_cur_chr(cur_tok() - (CS_TOKEN_FLAG + ACTIVE_BASE));
            }
            if cur_cmd() > ACTIVE_CHAR_C || cur_chr() > BIGGEST_USV {
                set_cur_cmd(RELAX);
                set_cur_chr(TOO_BIG_USV_C);
            }
            b = if this_if == IF_CHAR_CODE {
                n == cur_chr()
            } else {
                m == cur_cmd() as i32
            };
        }
        IF_INT_CODE | IF_DIM_CODE => {
            if this_if == IF_INT_CODE {
                scan_int();
            } else {
                scan_dimen(false, false, false);
            }
            n = crate::globals::cur_val();
            loop {
                get_x_token();
                if cur_cmd() != SPACER {
                    break;
                }
            }
            if cur_tok() >= OTHER_TOKEN + 60 && cur_tok() <= OTHER_TOKEN + 62 {
                r = (cur_tok() - OTHER_TOKEN) as u8;
            } else {
                error_here_with_diagnostic(c"Missing = inserted for ".as_ptr());
                print_cmd_chr(IF_TEST as u16, this_if as i32);
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(1);
                set_help_line(
                    0,
                    c"I was expecting to see `<', `=', or `>'. Didn't.".as_ptr(),
                );
                crate::input::back_error();
                r = b'=';
            }
            if this_if == IF_INT_CODE {
                scan_int();
            } else {
                scan_dimen(false, false, false);
            }
            let cv = crate::globals::cur_val();
            b = match r {
                60 => n < cv,
                61 => n == cv,
                _ => n > cv,
            };
        }
        IF_ODD_CODE => {
            scan_int();
            b = (crate::globals::cur_val() % 2) != 0;
        }
        IF_VMODE_CODE => {
            b = cur_list_mode().abs() == VMODE;
        }
        IF_HMODE_CODE => {
            b = cur_list_mode().abs() == HMODE_C;
        }
        IF_MMODE_CODE => {
            b = cur_list_mode().abs() == MMODE_C;
        }
        IF_INNER_CODE => {
            b = cur_list_mode() < 0;
        }
        IF_VOID_CODE | IF_HBOX_CODE | IF_VBOX_CODE => {
            scan_register_num();
            p = if crate::globals::cur_val() < 256 {
                eqtb_b32s1(BOX_BASE + crate::globals::cur_val())
            } else {
                crate::save::find_sa_element_global(4, crate::globals::cur_val(), false);
                if crate::globals::cur_ptr() == TEX_NULL {
                    TEX_NULL
                } else {
                    mem_b32s1(crate::globals::cur_ptr() + 1)
                }
            };
            b = if this_if == IF_VOID_CODE {
                p == TEX_NULL
            } else if p == TEX_NULL {
                false
            } else if this_if == IF_HBOX_CODE {
                mem_b16s1(p) == HLIST_NODE
            } else {
                mem_b16s1(p) == VLIST_NODE_C
            };
        }
        IFX_CODE => {
            save_scanner_status = scanner_status();
            set_scanner_status(NORMAL_SCAN);
            get_next();
            n = cur_cs();
            p = cur_cmd() as i32;
            q = cur_chr();
            get_next();
            if cur_cmd() != p as u8 {
                b = false;
            } else if (cur_cmd() as i32) < CALL as i32 {
                b = cur_chr() == q;
            } else {
                p = mem_b32s1(cur_chr()); // second token's replacement text (not saved q = first token's chr)
                q = mem_b32s1(eqtb_b32s1(n));
                if p == q {
                    b = true;
                } else {
                    while p != TEX_NULL && q != TEX_NULL {
                        if mem_b32s0(p) != mem_b32s0(q) {
                            p = TEX_NULL;
                        } else {
                            p = mem_b32s1(p);
                            q = mem_b32s1(q);
                        }
                    }
                    b = p == TEX_NULL && q == TEX_NULL;
                }
            }
            set_scanner_status(save_scanner_status);
        }
        IF_EOF_CODE => {
            scan_four_bit_int_or_18();
            b = if crate::globals::cur_val() == 18 {
                !shell_escape_enabled()
            } else {
                read_open_entry(crate::globals::cur_val() as usize) == 2 // CLOSED = 2
            };
        }
        IF_TRUE_CODE => {
            b = true;
        }
        IF_FALSE_CODE => {
            b = false;
        }
        IF_DEF_CODE => {
            save_scanner_status = scanner_status();
            set_scanner_status(NORMAL_SCAN);
            get_next();
            b = cur_cmd() != UNDEFINED_CS;
            set_scanner_status(save_scanner_status);
        }
        IF_CS_CODE => {
            n = get_avail_global();
            p = n;
            e = is_in_csname();
            set_is_in_csname(true);
            loop {
                get_x_token();
                if cur_cs() == 0 {
                    q = get_avail_global();
                    set_mem_b32s1(p, q);
                    set_mem_b32s0(q, cur_tok()); // mem[q].b32.s0 = cur_tok
                    p = q;
                }
                if cur_cs() != 0 {
                    break;
                }
            }
            if cur_cmd() != END_CS_NAME {
                error_here_with_diagnostic(c"Missing ".as_ptr());
                crate::output::print_esc_cstr(c"endcsname".as_ptr());
                crate::output::print_cstr(c" inserted".as_ptr());
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(2);
                set_help_line(
                    1,
                    c"The control sequence marked <to be read again> should".as_ptr(),
                );
                set_help_line(0, c"not appear between \\csname and \\endcsname.".as_ptr());
                crate::input::back_error();
            }
            m = first();
            p = mem_b32s1(n);
            while p != TEX_NULL {
                if m >= max_buf_stack() {
                    set_max_buf_stack(m + 1);
                    if max_buf_stack() == buf_size() {
                        crate::errors::overflow(c"buffer size".as_ptr(), buf_size());
                    }
                }
                set_buffer_entry(m, mem_b32s0(p) % MAX_CHAR_VAL);
                m += 1;
                p = mem_b32s1(p);
            }
            let cur_cs_val = if m == first() {
                NULL_CS
            } else if m == first() + 1 {
                SINGLE_BASE + buffer_entry(first())
            } else {
                crate::hash::id_lookup_global(first(), m - first())
            };
            crate::globals::set_cur_cs(cur_cs_val);
            flush_list_global(n);
            b = eqtb_b16s1(cur_cs()) != UNDEFINED_CS as u16;
            set_is_in_csname(e);
        }
        IF_IN_CSNAME_CODE => {
            b = is_in_csname();
        }
        IF_FONT_CHAR_CODE => {
            scan_font_ident();
            n = crate::globals::cur_val();
            scan_usv_num();
            let fa = font_area_entry(n);
            b = if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                map_char_to_glyph_global(n, crate::globals::cur_val()) > 0
            } else {
                let bc = font_bc_entry(n) as i32;
                let ec = font_ec_entry(n) as i32;
                if bc <= crate::globals::cur_val() && ec >= crate::globals::cur_val() {
                    font_info_b16(
                        char_base_entry(n)
                            + effective_char_global(true, n, crate::globals::cur_val() as u16),
                    )
                    .s3 > 0
                } else {
                    false
                }
            };
        }
        IF_CASE_CODE => {
            scan_int();
            n = crate::globals::cur_val();
            if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_COMMANDS) > 1 {
                crate::output::begin_diagnostic();
                crate::output::diagnostic_begin_capture_warning_here();
                crate::output::print_cstr(c"{case ".as_ptr());
                crate::output::print_int(n);
                crate::output::print_char(b'}' as i32);
                capture_to_diagnostic(std::ptr::null_mut());
                crate::output::end_diagnostic(false);
            }
            while n != 0 {
                pass_text();
                if cond_ptr() == save_cond_ptr {
                    if crate::globals::cur_chr() == OR_CODE as i32 {
                        n -= 1;
                    } else {
                        // goto common_ending
                        if crate::globals::cur_chr() == FI_CODE as i32 {
                            if if_stack(in_open()) == cond_ptr() {
                                crate::output::if_warning();
                            }
                            pop_cond_node();
                        } else {
                            set_if_limit(FI_CODE);
                        }
                        return;
                    }
                } else if crate::globals::cur_chr() == FI_CODE as i32 {
                    if if_stack(in_open()) == cond_ptr() {
                        crate::output::if_warning();
                    }
                    pop_cond_node();
                }
            }
            crate::output::change_if_limit(OR_CODE as i16, save_cond_ptr);
            return;
        }
        IF_PRIMITIVE_CODE => {
            save_scanner_status = scanner_status();
            set_scanner_status(NORMAL_SCAN);
            get_next();
            set_scanner_status(save_scanner_status);
            let cs = cur_cs();
            m = if cs < HASH_BASE {
                crate::hash::prim_lookup_global(cs - SINGLE_BASE)
            } else {
                crate::hash::prim_lookup_global(hash_s1(cs))
            };
            b = cur_cmd() != UNDEFINED_CS
                && m != UNDEFINED_PRIMITIVE
                && cur_cmd() as i32 == eqtb_b16s1(PRIM_EQTB_BASE + m) as i32
                && cur_chr() == eqtb_b32s1(PRIM_EQTB_BASE + m);
        }
        _ => {}
    }

    if is_unless {
        b = !b;
    }

    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_COMMANDS) > 1 {
        crate::output::begin_diagnostic();
        crate::output::diagnostic_begin_capture_warning_here();
        if b {
            crate::output::print_cstr(c"{true}".as_ptr());
        } else {
            crate::output::print_cstr(c"{false}".as_ptr());
        }
        capture_to_diagnostic(std::ptr::null_mut());
        crate::output::end_diagnostic(false);
    }

    if b {
        crate::output::change_if_limit(ELSE_CODE as i16, save_cond_ptr);
        return;
    }

    // false branch: skip until \else or \fi at our nesting level
    loop {
        pass_text();
        if cond_ptr() == save_cond_ptr {
            if crate::globals::cur_chr() != OR_CODE as i32 {
                // common_ending
                if crate::globals::cur_chr() == FI_CODE as i32 {
                    if if_stack(in_open()) == cond_ptr() {
                        crate::output::if_warning();
                    }
                    pop_cond_node();
                } else {
                    set_if_limit(FI_CODE);
                }
                return;
            }
            error_here_with_diagnostic(c"Extra ".as_ptr());
            crate::output::print_esc_cstr(c"or".as_ptr());
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(1);
            set_help_line(0, c"I'm ignoring this; it doesn't match any \\if.".as_ptr());
            error();
        } else if crate::globals::cur_chr() == FI_CODE as i32 {
            if if_stack(in_open()) == cond_ptr() {
                crate::output::if_warning();
            }
            pop_cond_node();
        }
    }
}
/// Pop a cond_ptr node — repeated pattern in conditional (§515).
#[inline]
unsafe fn pop_cond_node() {
    use crate::globals::{
        cond_ptr, mem_b16s0, mem_b16s1, mem_b32s1, set_cond_ptr, set_cur_if, set_if_limit,
        set_if_line,
    };
    let p = cond_ptr();
    set_if_line(mem_b32s1(p + 1));
    set_cur_if(mem_b16s0(p) as i16);
    set_if_limit(mem_b16s1(p) as u8);
    set_cond_ptr(mem_b32s1(p));
    crate::mem::free_node_global(p, IF_NODE_SIZE);
}
// conv_toks
//
// translated from xetex-xetex0.c:5163 (conv_toks)
pub unsafe fn conv_toks() {
    use crate::globals::{
        avail, cur_chr, cur_cs, cur_ptr, cur_val, def_ref, font_area_entry, font_dsize_entry,
        font_layout_engine_ptr, font_name_entry, font_size_entry, hi_mem_min, job_name, mem_b16s0,
        mem_b16s1, mem_b32s0, mem_b32s1, pool_ptr, scanner_status, selector, set_avail,
        set_cur_val, set_def_ref, set_mem_b32s1, set_pool_ptr, set_scanner_status, set_selector,
        set_str_ptr, set_warning_index, str_pool_entry, str_ptr, str_start, warning_index,
    };
    use crate::mem::TEX_NULL;

    // node type constants (xetex_format.h:380–396)
    const INS_NODE_CT: u16 = 3;
    const MARK_NODE_CT: u16 = 4;
    const ADJUST_NODE_CT: u16 = 5;
    const MARGIN_KERN_NODE_CT: u16 = 40;
    // GLUE_NODE = 10, KERN_NODE = 11 already defined at module level

    // convert command chr codes (xetex_format.h:490–519)
    const NUMBER_CODE_CT: i32 = 0;
    const ROMAN_NUMERAL_CODE_CT: i32 = 1;
    const STRING_CODE_CT: i32 = 2;
    const MEANING_CODE_CT: i32 = 3;
    const FONT_NAME_CODE_CT: i32 = 4;
    const ETEX_REVISION_CODE_CT: i32 = 5;
    const EXPANDED_CODE_CT: i32 = 6;
    const LEFT_MARGIN_KERN_CODE_CT: i32 = 16;
    const RIGHT_MARGIN_KERN_CODE_CT: i32 = 17;
    const PDF_STRCMP_CODE_CT: i32 = 18;
    const PDF_CREATION_DATE_CODE_CT: i32 = 22;
    const PDF_FILE_MOD_DATE_CODE_CT: i32 = 23;
    const PDF_FILE_SIZE_CODE_CT: i32 = 24;
    const PDF_MDFIVE_SUM_CODE_CT: i32 = 25;
    const PDF_FILE_DUMP_CODE_CT: i32 = 26;
    const UNIFORM_DEVIATE_CODE_CT: i32 = 29;
    const NORMAL_DEVIATE_CODE_CT: i32 = 30;
    const XETEX_REVISION_CODE_CT: i32 = 33;
    const XETEX_VARIATION_NAME_CODE_CT: i32 = 34;
    const XETEX_FEATURE_NAME_CODE_CT: i32 = 35;
    const XETEX_SELECTOR_NAME_CODE_CT: i32 = 36;
    const XETEX_GLYPH_NAME_CODE_CT: i32 = 37;
    const XETEX_UCHAR_CODE_CT: i32 = 38;
    const XETEX_UCHARCAT_CODE_CT: i32 = 39;
    const JOB_NAME_CODE_CT: i32 = 40;

    const GLUE_PAR_LEFT_SKIP: i32 = 7; // xetex_format.h:156
    const GLUE_PAR_RIGHT_SKIP: i32 = 8; // xetex_format.h:157

    const TEMP_HEAD_CT: i32 = 4_999_999 - 3; // MEM_TOP - 3
    const GARBAGE_CT: i32 = 4_999_999 - 12; // MEM_TOP - 12

    // CONVERT command code (xetex_format.h:352)
    const CONVERT: u16 = 110;

    let mut cat: i32 = 0;
    let c = cur_chr();
    let mut fnt: i32 = 0;
    let mut arg1: i32 = 0;
    let mut arg2: i32 = 0;
    let mut p = TEX_NULL;
    let mut boolvar: bool = false;
    let mut s: i32 = 0;
    let mut b: i32;

    // ---- Phase 1: scanning ----
    match c {
        NUMBER_CODE_CT | ROMAN_NUMERAL_CODE_CT => {
            crate::scan::scan_int();
        }
        STRING_CODE_CT | MEANING_CODE_CT => {
            let save_ss = scanner_status();
            set_scanner_status(NORMAL_SCANNER);
            crate::scan::get_token();
            set_scanner_status(save_ss);
        }
        FONT_NAME_CODE_CT => {
            crate::scan::scan_font_ident();
        }
        ETEX_REVISION_CODE_CT => {}
        EXPANDED_CODE_CT => {
            let save_ss = scanner_status();
            let save_wi = warning_index();
            let save_dr = def_ref();
            let u = if str_start(str_ptr() - TOO_BIG_CHAR) < pool_ptr() {
                crate::string_pool::make_string_global()
            } else {
                0
            };
            scan_pdf_ext_toks();
            set_warning_index(save_wi);
            set_scanner_status(save_ss);
            begin_token_list(mem_b32s1(def_ref()), INSERTED);
            set_mem_b32s1(def_ref(), avail());
            set_avail(def_ref());
            set_def_ref(save_dr);
            if u != 0 {
                set_str_ptr(str_ptr() - 1);
            }
            return;
        }
        LEFT_MARGIN_KERN_CODE_CT | RIGHT_MARGIN_KERN_CODE_CT => {
            crate::scan::scan_register_num();
            if cur_val() < 256 {
                p = eqtb_b32s1(BOX_BASE + cur_val());
            } else {
                crate::save::find_sa_element_global(4, cur_val(), false);
                if cur_ptr() == TEX_NULL {
                    p = TEX_NULL;
                } else {
                    p = mem_b32s1(cur_ptr() + 1);
                }
            }
            if p == TEX_NULL || mem_b16s1(p) != HLIST_NODE {
                crate::errors::pdf_error(
                    c"marginkern".as_ptr(),
                    c"a non-empty hbox expected".as_ptr(),
                );
            }
        }
        PDF_CREATION_DATE_CODE_CT => {
            b = pool_ptr();
            crate::texmfmp::getcreationdate();
            set_mem_b32s1(GARBAGE_CT, crate::scan::str_toks(b));
            begin_token_list(mem_b32s1(TEMP_HEAD_CT), INSERTED);
            // falls through to phase 2 (no PDF_CREATION_DATE_CODE_CT case there)
        }
        PDF_FILE_MOD_DATE_CODE_CT => {
            let save_ss = scanner_status();
            let save_wi = warning_index();
            let save_dr = def_ref();
            let u = if str_start(str_ptr() - TOO_BIG_CHAR) < pool_ptr() {
                crate::string_pool::make_string_global()
            } else {
                0
            };
            scan_pdf_ext_toks();
            if selector() == SELECTOR_NEW_STRING {
                crate::errors::pdf_error(
                    c"tokens".as_ptr(),
                    c"tokens_to_string() called while selector = new_string".as_ptr(),
                );
            }
            let old_sel = selector();
            set_selector(SELECTOR_NEW_STRING);
            crate::output::show_token_list(
                mem_b32s1(def_ref()),
                TEX_NULL,
                pool_size() - pool_ptr(),
            );
            set_selector(old_sel);
            s = crate::string_pool::make_string_global();
            crate::mem::delete_token_ref_global(def_ref());
            set_def_ref(save_dr);
            set_warning_index(save_wi);
            set_scanner_status(save_ss);
            b = pool_ptr();
            crate::texmfmp::getfilemoddate(s);
            set_mem_b32s1(GARBAGE_CT, crate::scan::str_toks(b));
            if s == str_ptr() - 1 {
                set_str_ptr(str_ptr() - 1);
                set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));
            }
            begin_token_list(mem_b32s1(TEMP_HEAD_CT), INSERTED);
            if u != 0 {
                set_str_ptr(str_ptr() - 1);
            }
            return;
        }
        PDF_FILE_SIZE_CODE_CT => {
            let save_ss = scanner_status();
            let save_wi = warning_index();
            let save_dr = def_ref();
            let u = if str_start(str_ptr() - TOO_BIG_CHAR) < pool_ptr() {
                crate::string_pool::make_string_global()
            } else {
                0
            };
            scan_pdf_ext_toks();
            if selector() == SELECTOR_NEW_STRING {
                crate::errors::pdf_error(
                    c"tokens".as_ptr(),
                    c"tokens_to_string() called while selector = new_string".as_ptr(),
                );
            }
            let old_sel = selector();
            set_selector(SELECTOR_NEW_STRING);
            crate::output::show_token_list(
                mem_b32s1(def_ref()),
                TEX_NULL,
                pool_size() - pool_ptr(),
            );
            set_selector(old_sel);
            s = crate::string_pool::make_string_global();
            crate::mem::delete_token_ref_global(def_ref());
            set_def_ref(save_dr);
            set_warning_index(save_wi);
            set_scanner_status(save_ss);
            b = pool_ptr();
            crate::texmfmp::getfilesize(s);
            set_mem_b32s1(GARBAGE_CT, crate::scan::str_toks(b));
            if s == str_ptr() - 1 {
                set_str_ptr(str_ptr() - 1);
                set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));
            }
            begin_token_list(mem_b32s1(TEMP_HEAD_CT), INSERTED);
            if u != 0 {
                set_str_ptr(str_ptr() - 1);
            }
            return;
        }
        PDF_MDFIVE_SUM_CODE_CT => {
            let save_ss = scanner_status();
            let save_wi = warning_index();
            let save_dr = def_ref();
            let u = if str_start(str_ptr() - TOO_BIG_CHAR) < pool_ptr() {
                crate::string_pool::make_string_global()
            } else {
                0
            };
            boolvar = scan_keyword(c"file".as_ptr());
            scan_pdf_ext_toks();
            if selector() == SELECTOR_NEW_STRING {
                crate::errors::pdf_error(
                    c"tokens".as_ptr(),
                    c"tokens_to_string() called while selector = new_string".as_ptr(),
                );
            }
            let old_sel = selector();
            set_selector(SELECTOR_NEW_STRING);
            crate::output::show_token_list(
                mem_b32s1(def_ref()),
                TEX_NULL,
                pool_size() - pool_ptr(),
            );
            set_selector(old_sel);
            s = crate::string_pool::make_string_global();
            crate::mem::delete_token_ref_global(def_ref());
            set_def_ref(save_dr);
            set_warning_index(save_wi);
            set_scanner_status(save_ss);
            b = pool_ptr();
            crate::texmfmp::getmd5sum(s, boolvar);
            set_mem_b32s1(GARBAGE_CT, crate::scan::str_toks(b));
            if s == str_ptr() - 1 {
                set_str_ptr(str_ptr() - 1);
                set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));
            }
            begin_token_list(mem_b32s1(TEMP_HEAD_CT), INSERTED);
            if u != 0 {
                set_str_ptr(str_ptr() - 1);
            }
            return;
        }
        PDF_FILE_DUMP_CODE_CT => {
            let save_ss = scanner_status();
            let save_wi = warning_index();
            let save_dr = def_ref();
            let u = if str_start(str_ptr() - TOO_BIG_CHAR) < pool_ptr() {
                crate::string_pool::make_string_global()
            } else {
                0
            };
            set_cur_val(0);
            if scan_keyword(c"offset".as_ptr()) {
                crate::scan::scan_int();
                if cur_val() < 0 {
                    error_here_with_diagnostic(c"Bad file offset".as_ptr());
                    capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(2);
                    set_help_line(1, c"A file offset must be between 0 and 2^_31_-1,".as_ptr());
                    set_help_line(0, c"I changed this one to zero.".as_ptr());
                    int_error(cur_val());
                    set_cur_val(0);
                }
            }
            let i = cur_val();
            set_cur_val(0);
            if scan_keyword(c"length".as_ptr()) {
                crate::scan::scan_int();
                if cur_val() < 0 {
                    error_here_with_diagnostic(c"Bad dump length".as_ptr());
                    capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(2);
                    set_help_line(1, c"A dump length must be between 0 and 2^_31_-1,".as_ptr());
                    set_help_line(0, c"I changed this one to zero.".as_ptr());
                    int_error(cur_val());
                    set_cur_val(0);
                }
            }
            let j = cur_val();
            scan_pdf_ext_toks();
            if selector() == SELECTOR_NEW_STRING {
                crate::errors::pdf_error(
                    c"tokens".as_ptr(),
                    c"tokens_to_string() called while selector = new_string".as_ptr(),
                );
            }
            let old_sel = selector();
            set_selector(SELECTOR_NEW_STRING);
            crate::output::show_token_list(
                mem_b32s1(def_ref()),
                TEX_NULL,
                pool_size() - pool_ptr(),
            );
            set_selector(old_sel);
            s = crate::string_pool::make_string_global();
            crate::mem::delete_token_ref_global(def_ref());
            set_def_ref(save_dr);
            set_warning_index(save_wi);
            set_scanner_status(save_ss);
            b = pool_ptr();
            crate::texmfmp::getfiledump(s, i, j);
            set_mem_b32s1(GARBAGE_CT, crate::scan::str_toks(b));
            if s == str_ptr() - 1 {
                set_str_ptr(str_ptr() - 1);
                set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));
            }
            begin_token_list(mem_b32s1(TEMP_HEAD_CT), INSERTED);
            if u != 0 {
                set_str_ptr(str_ptr() - 1);
            }
            return;
        }
        PDF_STRCMP_CODE_CT => {
            let save_ss = scanner_status();
            let save_wi = warning_index();
            let save_dr = def_ref();
            let u = if str_start(str_ptr() - TOO_BIG_CHAR) < pool_ptr() {
                crate::string_pool::make_string_global()
            } else {
                0
            };
            crate::scan::compare_strings();
            set_def_ref(save_dr);
            set_warning_index(save_wi);
            set_scanner_status(save_ss);
            if u != 0 {
                set_str_ptr(str_ptr() - 1);
            }
            // falls through to phase 2 (PDF_STRCMP_CODE_CT case there)
        }
        XETEX_UCHAR_CODE_CT => {
            crate::scan::scan_usv_num();
        }
        XETEX_UCHARCAT_CODE_CT => {
            crate::scan::scan_usv_num();
            let saved_chr = cur_val();
            crate::scan::scan_int();
            const LEFT_BRACE_CV: i32 = 1;
            const ACTIVE_CHAR_CV: i32 = 13;
            const OUT_PARAM_CV: i32 = 5;
            const IGNORE_CV: i32 = 9;
            if cur_val() < LEFT_BRACE_CV
                || cur_val() > ACTIVE_CHAR_CV
                || cur_val() == OUT_PARAM_CV
                || cur_val() == IGNORE_CV
            {
                error_here_with_diagnostic(c"Invalid code (".as_ptr());
                crate::output::print_int(cur_val());
                crate::output::print_cstr(
                    c"), should be in the ranges 1..4, 6..8, 10..13".as_ptr(),
                );
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(1);
                set_help_line(
                    0,
                    c"I'm going to use 12 instead of that illegal code value.".as_ptr(),
                );
                error();
                cat = 12;
            } else {
                cat = cur_val();
            }
            set_cur_val(saved_chr);
        }
        XETEX_REVISION_CODE_CT => {}
        XETEX_VARIATION_NAME_CODE_CT => {
            scan_font_ident();
            fnt = cur_val();
            if font_area_entry(fnt) == AAT_FONT_FLAG {
                crate::scan::scan_int();
                arg1 = cur_val();
                arg2 = 0;
            } else {
                not_aat_font_error(CONVERT as i32, c, fnt);
            }
        }
        XETEX_FEATURE_NAME_CODE_CT => {
            scan_font_ident();
            fnt = cur_val();
            let fa = font_area_entry(fnt);
            if fa == AAT_FONT_FLAG
                || (fa == OTGR_FONT_FLAG && usingGraphite(font_layout_engine_ptr(fnt)))
            {
                crate::scan::scan_int();
                arg1 = cur_val();
                arg2 = 0;
            } else {
                not_aat_gr_font_error(CONVERT as i32, c, fnt);
            }
        }
        XETEX_SELECTOR_NAME_CODE_CT => {
            scan_font_ident();
            fnt = cur_val();
            let fa = font_area_entry(fnt);
            if fa == AAT_FONT_FLAG
                || (fa == OTGR_FONT_FLAG && usingGraphite(font_layout_engine_ptr(fnt)))
            {
                crate::scan::scan_int();
                arg1 = cur_val();
                crate::scan::scan_int();
                arg2 = cur_val();
            } else {
                not_aat_gr_font_error(CONVERT as i32, c, fnt);
            }
        }
        XETEX_GLYPH_NAME_CODE_CT => {
            scan_font_ident();
            fnt = cur_val();
            let fa = font_area_entry(fnt);
            if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                crate::scan::scan_int();
                arg1 = cur_val();
            } else {
                not_native_font_error(CONVERT as i32, c, fnt);
            }
        }
        JOB_NAME_CODE_CT => {
            if job_name() == 0 {
                crate::io::open_log_file();
            }
        }
        UNIFORM_DEVIATE_CODE_CT => {
            crate::scan::scan_int();
        }
        _ => {} // NORMAL_DEVIATE_CODE_CT and unrecognized codes
    }

    // ---- Phase 2: output ----
    let old_sel = selector();
    set_selector(SELECTOR_NEW_STRING);
    b = pool_ptr();

    match c {
        NUMBER_CODE_CT => {
            crate::output::print_int(cur_val());
        }
        ROMAN_NUMERAL_CODE_CT => {
            crate::output::print_roman_int(cur_val());
        }
        STRING_CODE_CT => {
            if cur_cs() != 0 {
                crate::output::sprint_cs(cur_cs());
            } else {
                crate::output::print_char(cur_chr());
            }
        }
        MEANING_CODE_CT => {
            crate::output::print_meaning();
        }
        FONT_NAME_CODE_CT => {
            let font_name_str = font_name_entry(cur_val());
            let fa = font_area_entry(cur_val());
            if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                let mut quote_char: i32 = '"' as i32;
                let base = str_start(font_name_str - TOO_BIG_CHAR);
                for i in 0..crate::string_pool::length_global(font_name_str) {
                    if str_pool_entry(base + i) == b'"' as u16 {
                        quote_char = '\'' as i32;
                    }
                }
                crate::output::print_char(quote_char);
                crate::output::print(font_name_str);
                crate::output::print_char(quote_char);
            } else {
                crate::output::print(font_name_str);
            }
            if font_size_entry(cur_val()) != font_dsize_entry(cur_val()) {
                crate::output::print_cstr(c" at ".as_ptr());
                crate::output::print_scaled(font_size_entry(cur_val()));
                crate::output::print_cstr(c"pt".as_ptr());
            }
        }
        ETEX_REVISION_CODE_CT => {
            crate::output::print_cstr(c".6".as_ptr());
        }
        LEFT_MARGIN_KERN_CODE_CT => {
            p = mem_b32s1(p + 5);
            while p != TEX_NULL {
                let tp = mem_b16s1(p);
                let is_low = p < hi_mem_min();
                let skip = (is_low
                    && (tp == INS_NODE_CT
                        || tp == MARK_NODE_CT
                        || tp == ADJUST_NODE_CT
                        || tp == PENALTY_NODE
                        || (tp == DISC_NODE
                            && mem_b32s0(p + 1) == TEX_NULL
                            && mem_b32s1(p + 1) == TEX_NULL
                            && mem_b16s0(p) == 0)
                        || (tp == MATH_NODE && mem_b32s1(p + 1) == 0)
                        || (tp == KERN_NODE && (mem_b32s1(p + 1) == 0 || mem_b16s0(p) == 0))
                        || (tp == GLUE_NODE && mem_b32s0(p + 1) == 0)
                        || (tp == HLIST_NODE
                            && mem_b32s1(p + 1) == 0
                            && mem_b32s1(p + 3) == 0
                            && mem_b32s1(p + 2) == 0
                            && mem_b32s1(p + 5) == TEX_NULL)))
                    || (is_low
                        && tp == GLUE_NODE
                        && mem_b16s0(p) == (GLUE_PAR_LEFT_SKIP + 1) as u16);
                if !skip {
                    break;
                }
                p = mem_b32s1(p); // LLIST_link(p) = mem[p].b32.s1
            }
            if p != TEX_NULL
                && p < hi_mem_min()
                && mem_b16s1(p) == MARGIN_KERN_NODE_CT
                && mem_b16s0(p) == 0
            {
                crate::output::print_scaled(mem_b32s1(p + 1));
            } else {
                crate::output::print('0' as i32);
            }
            crate::output::print_cstr(c"pt".as_ptr());
        }
        RIGHT_MARGIN_KERN_CODE_CT => {
            let q = mem_b32s1(p + 5);
            p = crate::mem::prev_rightmost_global(q, TEX_NULL);
            while p != TEX_NULL {
                let tp = mem_b16s1(p);
                let is_low = p < hi_mem_min();
                let skip = (is_low
                    && (tp == INS_NODE_CT
                        || tp == MARK_NODE_CT
                        || tp == ADJUST_NODE_CT
                        || tp == PENALTY_NODE
                        || (tp == DISC_NODE
                            && mem_b32s0(p + 1) == TEX_NULL
                            && mem_b32s1(p + 1) == TEX_NULL
                            && mem_b16s0(p) == 0)
                        || (tp == MATH_NODE && mem_b32s1(p + 1) == 0)
                        || (tp == KERN_NODE && (mem_b32s1(p + 1) == 0 || mem_b16s0(p) == 0))
                        || (tp == GLUE_NODE && mem_b32s0(p + 1) == 0)
                        || (tp == HLIST_NODE
                            && mem_b32s1(p + 1) == 0
                            && mem_b32s1(p + 3) == 0
                            && mem_b32s1(p + 2) == 0
                            && mem_b32s1(p + 5) == TEX_NULL)))
                    || (is_low
                        && tp == GLUE_NODE
                        && mem_b16s0(p) == (GLUE_PAR_RIGHT_SKIP + 1) as u16);
                if !skip {
                    break;
                }
                p = crate::mem::prev_rightmost_global(q, p);
            }
            if p != TEX_NULL
                && p < hi_mem_min()
                && mem_b16s1(p) == MARGIN_KERN_NODE_CT
                && mem_b16s0(p) == 1
            {
                crate::output::print_scaled(mem_b32s1(p + 1));
            } else {
                crate::output::print('0' as i32);
            }
            crate::output::print_cstr(c"pt".as_ptr());
        }
        PDF_STRCMP_CODE_CT => {
            crate::output::print_int(cur_val());
        }
        UNIFORM_DEVIATE_CODE_CT => {
            crate::output::print_int(crate::scaled_math::unif_rand(cur_val()));
        }
        NORMAL_DEVIATE_CODE_CT => {
            crate::output::print_int(crate::scaled_math::norm_rand());
        }
        XETEX_UCHAR_CODE_CT | XETEX_UCHARCAT_CODE_CT => {
            crate::output::print_char(cur_val());
        }
        XETEX_REVISION_CODE_CT => {
            crate::output::print_cstr(c".999994".as_ptr());
        }
        XETEX_VARIATION_NAME_CODE_CT => {
            if font_area_entry(fnt) == AAT_FONT_FLAG {
                aat_print_font_name(c, font_layout_engine_ptr(fnt), arg1, arg2);
            }
        }
        XETEX_FEATURE_NAME_CODE_CT | XETEX_SELECTOR_NAME_CODE_CT => {
            let fa = font_area_entry(fnt);
            if fa == AAT_FONT_FLAG {
                aat_print_font_name(c, font_layout_engine_ptr(fnt), arg1, arg2);
            } else if fa == OTGR_FONT_FLAG && usingGraphite(font_layout_engine_ptr(fnt)) {
                gr_print_font_name(c, font_layout_engine_ptr(fnt), arg1, arg2);
            }
        }
        XETEX_GLYPH_NAME_CODE_CT => {
            let fa = font_area_entry(fnt);
            if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                print_glyph_name(fnt, arg1);
            }
        }
        JOB_NAME_CODE_CT => {
            crate::output::print_file_name(job_name(), 0, 0);
        }
        _ => {}
    }

    set_selector(old_sel);
    set_mem_b32s1(GARBAGE_CT, crate::scan::str_toks_cat(b, cat as i16));
    begin_token_list(mem_b32s1(TEMP_HEAD_CT), INSERTED);
}
// translated from xetex-xetex0.c:4003 (expand)
pub unsafe fn expand() {
    use crate::globals::{
        buf_size, cond_ptr, cur_mark, cur_order, cur_ptr, cur_val_level, eqtb_b16s1, hash_s1,
        if_limit, if_stack, in_open, is_in_csname, max_buf_stack, radix, set_cond_ptr, set_cur_if,
        set_cur_order, set_cur_ptr, set_cur_val_level, set_force_eof, set_if_limit, set_if_line,
        set_is_in_csname, set_max_buf_stack, set_no_new_control_sequence, set_radix,
    };

    const TOP_BOT_MARK: u8 = 112;
    const EXPAND_AFTER: u8 = 104;
    const NO_EXPAND: u8 = 105;
    const INPUT: u8 = 106;
    const CS_NAME_CMD: u8 = 109;
    const CONVERT_CMD: u8 = 110;
    const THE_CMD: u8 = 111; // same as module-level THE
    const IF_TEST_CMD: u8 = 107;
    const FI_OR_ELSE_CMD: u8 = 108;
    const MARKS_CODE: i32 = 5;
    const MARK_VAL_IDX: usize = 7;
    const MARK_TEXT_TL: u16 = 15;
    const FROZEN_DONT_EXPAND: i32 = 2_243_226 + 9;
    const FROZEN_CS_TOKEN: i32 = CS_TOKEN_FLAG + FROZEN_DONT_EXPAND;
    const FROZEN_PRIM_CS_TOKEN: i32 = CS_TOKEN_FLAG + FROZEN_PRIMITIVE;
    const NORMAL_SCANNER: u8 = 0;
    const IF_CODE_VAL: u8 = 1;
    const IF_CASE_CODE_VAL: i32 = 16;
    const IF_NODE_SIZE_V: i32 = 2;
    const TOO_BIG_USV: i32 = 1_114_112;
    const NULL_CS_V: i32 = 2_228_225;
    const UNDEFINED_CS_CMD: u8 = 103;
    const INT_PAR_TRACING_COMMANDS: i32 = 36;
    const INT_PAR_TRACING_IFS: i32 = 58;
    const FI_CODE_VAL: i32 = 2;

    set_expand_depth_count(expand_depth_count() + 1);
    if expand_depth_count() >= expand_depth() {
        crate::errors::overflow(c"expansion depth".as_ptr(), expand_depth());
    }

    let cv_backup = cur_val();
    let cvl_backup = cur_val_level();
    let radix_backup = radix();
    let co_backup = cur_order();
    let backup_backup = mem_b32s1(BACKUP_HEAD);

    'reswitch: loop {
        if cur_cmd() < CALL {
            if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_COMMANDS) > 1 {
                crate::output::show_cur_cmd_chr();
            }
            match cur_cmd() {
                TOP_BOT_MARK => {
                    let t = (cur_chr() % MARKS_CODE) as usize;
                    if cur_chr() >= MARKS_CODE {
                        scan_register_num();
                    } else {
                        set_cur_val(0);
                    }
                    if cur_val() == 0 {
                        set_cur_ptr(cur_mark(t));
                    } else {
                        crate::save::find_sa_element_global(MARK_VAL_IDX as u16, cur_val(), false);
                        if cur_ptr() != crate::mem::TEX_NULL {
                            if !t.is_multiple_of(2) {
                                set_cur_ptr(mem_b32s1(cur_ptr() + (t as i32 / 2) + 1));
                            } else {
                                set_cur_ptr(mem_b32s0(cur_ptr() + (t as i32 / 2) + 1));
                            }
                        }
                    }
                    if cur_ptr() != crate::mem::TEX_NULL {
                        crate::input::begin_token_list(cur_ptr(), MARK_TEXT_TL);
                    }
                }
                EXPAND_AFTER => {
                    if cur_chr() == 0 {
                        get_token();
                        let t = cur_tok();
                        get_token();
                        if cur_cmd() > MAX_COMMAND {
                            expand();
                        } else {
                            crate::input::back_input();
                        }
                        set_cur_tok(t);
                        crate::input::back_input();
                    } else {
                        // \unless
                        get_token();
                        if cur_cmd() == IF_TEST_CMD && cur_chr() != IF_CASE_CODE_VAL {
                            set_cur_chr(cur_chr() + UNLESS_CODE);
                            continue 'reswitch;
                        }
                        crate::output::error_here_with_diagnostic(c"You can't use `".as_ptr());
                        crate::output::print_esc_cstr(c"unless".as_ptr());
                        crate::output::print_cstr(c"' before `".as_ptr());
                        print_cmd_chr(cur_cmd() as u16, cur_chr());
                        crate::output::print_char('\'' as i32);
                        crate::output::capture_to_diagnostic(core::ptr::null_mut());
                        set_help_ptr(1);
                        set_help_line(
                            0,
                            c"Continue, and I'll forget that it ever happened.".as_ptr(),
                        );
                        crate::input::back_error();
                    }
                }
                NO_EXPAND => {
                    if cur_chr() == 0 {
                        let save_scanner_status = scanner_status();
                        set_scanner_status(NORMAL_SCANNER);
                        get_token();
                        set_scanner_status(save_scanner_status);
                        let t = cur_tok();
                        crate::input::back_input();
                        if t >= CS_TOKEN_FLAG {
                            let p = crate::mem::get_avail_global();
                            set_mem_b32s0(p, FROZEN_CS_TOKEN);
                            set_mem_b32s1(p, cur_input().loc);
                            set_cur_input_start(p);
                            set_cur_input_loc(p);
                        }
                    } else {
                        // \primitive
                        let save_scanner_status = scanner_status();
                        set_scanner_status(NORMAL_SCANNER);
                        get_token();
                        set_scanner_status(save_scanner_status);
                        let cs = if cur_cs() < HASH_BASE {
                            crate::hash::prim_lookup_global(cur_cs() - SINGLE_BASE)
                        } else {
                            crate::hash::prim_lookup_global(hash_s1(cur_cs()))
                        };
                        set_cur_cs(cs);
                        if cur_cs() != UNDEFINED_PRIMITIVE {
                            let t = eqtb_b16s1(PRIM_EQTB_BASE + cur_cs()) as u8;
                            if t > MAX_COMMAND {
                                set_cur_cmd(t);
                                set_cur_chr(eqtb_b32s1(PRIM_EQTB_BASE + cur_cs()));
                                set_cur_tok(cur_cmd() as i32 * MAX_CHAR_VAL + cur_chr());
                                set_cur_cs(0);
                                continue 'reswitch;
                            } else {
                                crate::input::back_input();
                                let p = crate::mem::get_avail_global();
                                set_mem_b32s0(p, FROZEN_PRIM_CS_TOKEN);
                                set_mem_b32s1(p, cur_input().loc);
                                set_cur_input_loc(p);
                                set_cur_input_start(p);
                            }
                        }
                    }
                }
                CS_NAME_CMD => {
                    let r = crate::mem::get_avail_global();
                    let mut p = r;
                    let b = is_in_csname();
                    set_is_in_csname(true);
                    loop {
                        get_x_token();
                        if cur_cs() == 0 {
                            let q = crate::mem::get_avail_global();
                            set_mem_b32s1(p, q);
                            set_mem_b32s0(q, cur_tok());
                            p = q;
                        }
                        if cur_cs() != 0 {
                            break;
                        }
                    }
                    if cur_cmd() != END_CS_NAME {
                        crate::output::error_here_with_diagnostic(c"Missing ".as_ptr());
                        crate::output::print_esc_cstr(c"endcsname".as_ptr());
                        crate::output::print_cstr(c" inserted".as_ptr());
                        crate::output::capture_to_diagnostic(core::ptr::null_mut());
                        set_help_ptr(2);
                        set_help_line(
                            1,
                            c"The control sequence marked <to be read again> should".as_ptr(),
                        );
                        set_help_line(0, c"not appear between \\csname and \\endcsname.".as_ptr());
                        crate::input::back_error();
                    }
                    set_is_in_csname(b);
                    let mut j = first();
                    p = mem_b32s1(r);
                    while p != crate::mem::TEX_NULL {
                        if j >= max_buf_stack() {
                            set_max_buf_stack(j + 1);
                            if max_buf_stack() == buf_size() {
                                crate::errors::overflow(c"buffer size".as_ptr(), buf_size());
                            }
                        }
                        set_buffer_entry(j, mem_b32s0(p) % MAX_CHAR_VAL);
                        j += 1;
                        p = mem_b32s1(p);
                    }
                    let cs = if j > first() + 1 || buffer_entry(first()) > 65535 {
                        set_no_new_control_sequence(false);
                        let cs = crate::hash::id_lookup_global(first(), j - first());
                        set_no_new_control_sequence(true);
                        cs
                    } else if j == first() {
                        NULL_CS_V
                    } else {
                        SINGLE_BASE + buffer_entry(first())
                    };
                    set_cur_cs(cs);
                    crate::mem::flush_list_global(r);
                    if eqtb_b16s1(cur_cs()) == UNDEFINED_CS_CMD as u16 {
                        crate::save::eq_define_global(cur_cs(), RELAX as u16, TOO_BIG_USV);
                    }
                    set_cur_tok(cur_cs() + CS_TOKEN_FLAG);
                    crate::input::back_input();
                }
                CONVERT_CMD => {
                    conv_toks();
                }
                THE_CMD => {
                    ins_the_toks();
                }
                IF_TEST_CMD => {
                    conditional();
                }
                FI_OR_ELSE_CMD => {
                    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_IFS) > 0
                        && eqtb_b32s1(INT_BASE + INT_PAR_TRACING_COMMANDS) <= 1
                    {
                        crate::output::show_cur_cmd_chr();
                    }
                    if cur_chr() > if_limit() as i32 {
                        if if_limit() == IF_CODE_VAL {
                            crate::input::insert_relax();
                        } else {
                            crate::output::error_here_with_diagnostic(c"Extra ".as_ptr());
                            print_cmd_chr(FI_OR_ELSE_CMD as u16, cur_chr());
                            crate::output::capture_to_diagnostic(core::ptr::null_mut());
                            set_help_ptr(1);
                            set_help_line(
                                0,
                                c"I'm ignoring this; it doesn't match any \\if.".as_ptr(),
                            );
                            crate::errors::error();
                        }
                    } else {
                        while cur_chr() != FI_CODE_VAL {
                            pass_text();
                        }
                        if if_stack(in_open()) == cond_ptr() {
                            crate::output::if_warning();
                        }
                        let p = cond_ptr();
                        set_if_line(mem_b32s1(p + 1));
                        set_cur_if(mem_b16s0(p) as i16);
                        set_if_limit(mem_b16s1(p) as u8);
                        set_cond_ptr(mem_b32s1(p));
                        crate::mem::free_node_global(p, IF_NODE_SIZE_V);
                    }
                }
                INPUT => {
                    if cur_chr() == 1 {
                        set_force_eof(true);
                    } else if cur_chr() == 2 {
                        pseudo_start();
                    } else if name_in_progress() {
                        crate::input::insert_relax();
                    } else {
                        start_input(core::ptr::null());
                    }
                }
                _ => {
                    crate::output::error_here_with_diagnostic(
                        c"Undefined control sequence".as_ptr(),
                    );
                    crate::output::capture_to_diagnostic(core::ptr::null_mut());
                    set_help_ptr(5);
                    set_help_line(
                        4,
                        c"The control sequence at the end of the top line".as_ptr(),
                    );
                    set_help_line(
                        3,
                        c"of your error message was never \\def'ed. If you have".as_ptr(),
                    );
                    set_help_line(
                        2,
                        c"misspelled it (e.g., `\\hobx'), type `I' and the correct".as_ptr(),
                    );
                    set_help_line(
                        1,
                        c"spelling (e.g., `I\\hbox'). Otherwise just continue,".as_ptr(),
                    );
                    set_help_line(0, c"and I'll forget about whatever was undefined.".as_ptr());
                    crate::errors::error();
                }
            }
        } else if cur_cmd() < END_TEMPLATE {
            macro_call();
        } else {
            set_cur_tok(CS_TOKEN_FLAG + FROZEN_ENDV);
            crate::input::back_input();
        }
        break 'reswitch;
    }

    set_cur_val(cv_backup);
    set_cur_val_level(cvl_backup);
    set_radix(radix_backup);
    set_cur_order(co_backup);
    set_mem_b32s1(BACKUP_HEAD, backup_backup);
    set_expand_depth_count(expand_depth_count() - 1);
}
// translated from xetex-xetex0.c:3665 (macro_call)
pub unsafe fn macro_call() {
    use crate::globals::{
        align_state, avail, cur_chr, cur_cs, cur_tok, eqtb_b16s1, input_ptr, long_state,
        max_param_stack, mem_b32s0, mem_b32s1, par_token, param_ptr, param_size, scanner_status,
        set_align_state, set_avail, set_cur_input_loc, set_cur_input_name, set_cur_tok,
        set_long_state, set_max_param_stack, set_mem_b32s0, set_mem_b32s1, set_param_ptr,
        set_param_stack, set_scanner_status, set_warning_index, warning_index,
    };
    use crate::mem::{flush_list_global, get_avail_global, TEX_NULL};

    const MATCHING: u8 = 3; // xetex-constants.h:185
    const LONG_CALL: u8 = 114; // xetex_format.h:356
    const OUTER_CALL: u8 = 115; // xetex_format.h:357
    const MACRO: u16 = 6; // xetex-constants.h (token-list type)
    const V_TEMPLATE: u16 = 2; // xetex-constants.h:201
    const TOKEN_LIST: u16 = 0; // xetex-constants.h:261
    const INT_PAR_TRACING_MACROS: i32 = 30; // xetex_format.h:65
    const INT_PAR_TRACING_STACK_LEVELS: i32 = 55; // xetex_format.h:90

    let save_scanner_status = scanner_status();
    let save_warning_index = warning_index();
    set_warning_index(cur_cs());
    let ref_count = cur_chr();
    let mut r = mem_b32s1(ref_count);
    let mut n: i32 = 0;

    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_MACROS) > 0 {
        crate::output::begin_diagnostic();
        crate::output::diagnostic_begin_capture_warning_here();
        let tsl = eqtb_b32s1(INT_BASE + INT_PAR_TRACING_STACK_LEVELS);
        if tsl > 0 {
            if input_ptr() < tsl {
                let mut v = input_ptr();
                crate::output::print_ln();
                crate::output::print_char(b'~' as i32);
                while v > 0 {
                    crate::output::print_char(b'.' as i32);
                    v -= 1;
                }
                crate::output::print_cs(warning_index());
                crate::output::token_show(ref_count);
            } else {
                crate::output::print_char(b'~' as i32);
                crate::output::print_char(b'~' as i32);
                crate::output::print_cs(warning_index());
            }
        } else {
            crate::output::print_ln();
            crate::output::print_cs(warning_index());
            crate::output::token_show(ref_count);
        }
        capture_to_diagnostic(std::ptr::null_mut());
        crate::output::end_diagnostic(false);
    }

    if mem_b32s0(r) == PROTECTED_TOKEN {
        r = mem_b32s1(r);
    }

    // pstack: accumulates gathered parameter token lists (max 9 params)
    let mut pstack: [i32; 9] = [TEX_NULL; 9];
    let mut match_chr: i32 = 0;
    let mut rbrace_ptr: i32 = TEX_NULL;

    'mac: {
        if mem_b32s0(r) != END_MATCH_TOKEN {
            set_scanner_status(MATCHING);
            let mut unbalance: i32 = 0;
            set_long_state(eqtb_b16s1(cur_cs()) as u8);
            if long_state() >= OUTER_CALL {
                set_long_state(long_state() - 2);
            }

            'outer: loop {
                set_mem_b32s1(TEMP_HEAD, TEX_NULL);
                // Determine whether this iteration gathers a delimited parameter
                let s: i32;
                let mut p: i32;
                let mut m: i32;
                if mem_b32s0(r) >= END_MATCH_TOKEN || mem_b32s0(r) < MATCH_TOKEN {
                    s = TEX_NULL;
                    p = TEX_NULL; // unused when s == TEX_NULL
                    m = 0;
                } else {
                    match_chr = mem_b32s0(r) - MATCH_TOKEN;
                    s = mem_b32s1(r);
                    r = s;
                    p = TEMP_HEAD;
                    m = 0;
                }

                'continue_: loop {
                    get_token();

                    if cur_tok() == mem_b32s0(r) {
                        r = mem_b32s1(r);
                        if mem_b32s0(r) >= MATCH_TOKEN && mem_b32s0(r) <= END_MATCH_TOKEN {
                            if cur_tok() < LEFT_BRACE_LIMIT {
                                set_align_state(align_state() - 1);
                            }
                            break 'continue_; // goto found
                        } else {
                            continue 'continue_; // goto continue_
                        }
                    }

                    if s != r {
                        if s == TEX_NULL {
                            error_here_with_diagnostic(c"Use of ".as_ptr());
                            crate::output::sprint_cs(warning_index());
                            crate::output::print_cstr(c" doesn't match its definition".as_ptr());
                            capture_to_diagnostic(std::ptr::null_mut());
                            set_help_ptr(4);
                            set_help_line(
                                3,
                                c"If you say, e.g., `\\def\\a1{...}', then you must always"
                                    .as_ptr(),
                            );
                            set_help_line(
                                2,
                                c"put `1' after `\\a', since control sequence names are".as_ptr(),
                            );
                            set_help_line(
                                1,
                                c"made up of letters only. The macro here has not been".as_ptr(),
                            );
                            set_help_line(
                                0,
                                c"followed by the required stuff, so I'm ignoring it.".as_ptr(),
                            );
                            crate::errors::error();
                            break 'mac; // goto exit
                        } else {
                            let mut t = s;
                            loop {
                                let q = get_avail_global();
                                set_mem_b32s1(p, q);
                                set_mem_b32s0(q, mem_b32s0(t));
                                p = q;
                                m += 1;
                                let mut u = mem_b32s1(t);
                                let mut v = s;
                                loop {
                                    if u == r {
                                        if cur_tok() != mem_b32s0(v) {
                                            break; // goto done
                                        } else {
                                            r = mem_b32s1(v);
                                            continue 'continue_; // goto continue_
                                        }
                                    }
                                    if mem_b32s0(u) != mem_b32s0(v) {
                                        break; // goto done
                                    }
                                    u = mem_b32s1(u);
                                    v = mem_b32s1(v);
                                }
                                // done:
                                t = mem_b32s1(t);
                                if t == r {
                                    break;
                                }
                            }
                            r = s;
                        }
                    }

                    if cur_tok() == par_token() && long_state() != LONG_CALL {
                        if long_state() == CALL {
                            runaway();
                            error_here_with_diagnostic(c"Paragraph ended before ".as_ptr());
                            crate::output::sprint_cs(warning_index());
                            crate::output::print_cstr(c" was complete".as_ptr());
                            capture_to_diagnostic(std::ptr::null_mut());
                            set_help_ptr(3);
                            set_help_line(
                                2,
                                c"I suspect you've forgotten a `}', causing me to apply this"
                                    .as_ptr(),
                            );
                            set_help_line(
                                1,
                                c"control sequence to too much text. How can we recover?".as_ptr(),
                            );
                            set_help_line(
                                0,
                                c"My plan is to forget the whole thing and hope for the best."
                                    .as_ptr(),
                            );
                            crate::input::back_error();
                        }
                        pstack[n as usize] = mem_b32s1(TEMP_HEAD);
                        set_align_state(align_state() - unbalance);
                        for m2 in 0..=n {
                            flush_list_global(pstack[m2 as usize]);
                        }
                        break 'mac; // goto exit
                    }

                    if cur_tok() < RIGHT_BRACE_LIMIT {
                        if cur_tok() < LEFT_BRACE_LIMIT {
                            // left brace: gather a braced group
                            unbalance = 1;
                            loop {
                                let q = if avail() != TEX_NULL {
                                    let q = avail();
                                    set_avail(mem_b32s1(q));
                                    set_mem_b32s1(q, TEX_NULL);
                                    q
                                } else {
                                    get_avail_global()
                                };
                                set_mem_b32s1(p, q);
                                set_mem_b32s0(q, cur_tok());
                                p = q;

                                get_token();

                                if cur_tok() == par_token() && long_state() != LONG_CALL {
                                    if long_state() == CALL {
                                        runaway();
                                        error_here_with_diagnostic(
                                            c"Paragraph ended before ".as_ptr(),
                                        );
                                        crate::output::sprint_cs(warning_index());
                                        crate::output::print_cstr(c" was complete".as_ptr());
                                        capture_to_diagnostic(std::ptr::null_mut());
                                        set_help_ptr(3);
                                        set_help_line(2, c"I suspect you've forgotten a `}', causing me to apply this".as_ptr());
                                        set_help_line(1, c"control sequence to too much text. How can we recover?".as_ptr());
                                        set_help_line(0, c"My plan is to forget the whole thing and hope for the best.".as_ptr());
                                        crate::input::back_error();
                                    }
                                    pstack[n as usize] = mem_b32s1(TEMP_HEAD);
                                    set_align_state(align_state() - unbalance);
                                    for m2 in 0..=n {
                                        flush_list_global(pstack[m2 as usize]);
                                    }
                                    break 'mac; // goto exit
                                }

                                if cur_tok() < RIGHT_BRACE_LIMIT {
                                    if cur_tok() < LEFT_BRACE_LIMIT {
                                        unbalance += 1;
                                    } else {
                                        unbalance -= 1;
                                        if unbalance == 0 {
                                            break;
                                        } // done1
                                    }
                                }
                            }
                            // done1:
                            rbrace_ptr = p;
                            let q = get_avail_global();
                            set_mem_b32s1(p, q);
                            set_mem_b32s0(q, cur_tok());
                            p = q;
                        } else {
                            // right brace with no matching left brace (413)
                            crate::input::back_input();
                            error_here_with_diagnostic(c"Argument of ".as_ptr());
                            crate::output::sprint_cs(warning_index());
                            crate::output::print_cstr(c" has an extra }".as_ptr());
                            capture_to_diagnostic(std::ptr::null_mut());
                            set_help_ptr(6);
                            set_help_line(
                                5,
                                c"I've run across a `}' that doesn't seem to match anything."
                                    .as_ptr(),
                            );
                            set_help_line(
                                4,
                                c"For example, `\\def\\a#1{...}' and `\\a}' would produce".as_ptr(),
                            );
                            set_help_line(
                                3,
                                c"this error. If you simply proceed now, the `\\par' that".as_ptr(),
                            );
                            set_help_line(
                                2,
                                c"I've just inserted will cause me to report a runaway".as_ptr(),
                            );
                            set_help_line(
                                1,
                                c"argument that might be the root of the problem. But if".as_ptr(),
                            );
                            set_help_line(
                                0,
                                c"your `}' was spurious, just type `2' and it will go away."
                                    .as_ptr(),
                            );
                            set_align_state(align_state() + 1);
                            set_long_state(CALL);
                            set_cur_tok(par_token());
                            crate::input::ins_error();
                            continue 'continue_; // goto continue_
                        }
                    } else {
                        // 411: non-brace token
                        if cur_tok() == SPACE_TOKEN
                            && mem_b32s0(r) <= END_MATCH_TOKEN
                            && mem_b32s0(r) >= MATCH_TOKEN
                        {
                            continue 'continue_; // goto continue_
                        }
                        let q = get_avail_global();
                        set_mem_b32s1(p, q);
                        set_mem_b32s0(q, cur_tok());
                        p = q;
                    }

                    m += 1;
                    if mem_b32s0(r) > END_MATCH_TOKEN || mem_b32s0(r) < MATCH_TOKEN {
                        continue 'continue_; // goto continue_
                    }
                    break 'continue_; // goto found
                } // end 'continue_

                // found:
                if s != TEX_NULL {
                    if m == 1 && mem_b32s0(p) < RIGHT_BRACE_LIMIT {
                        // strip outer braces from single-token braced arg
                        set_mem_b32s1(rbrace_ptr, TEX_NULL);
                        set_mem_b32s1(p, avail());
                        set_avail(p);
                        p = mem_b32s1(TEMP_HEAD);
                        pstack[n as usize] = mem_b32s1(p);
                        set_mem_b32s1(p, avail());
                        set_avail(p);
                    } else {
                        pstack[n as usize] = mem_b32s1(TEMP_HEAD);
                    }

                    n += 1;

                    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_MACROS) > 0 {
                        let tsl = eqtb_b32s1(INT_BASE + INT_PAR_TRACING_STACK_LEVELS);
                        if tsl == 0 || input_ptr() < tsl {
                            crate::output::begin_diagnostic();
                            crate::output::diagnostic_begin_capture_warning_here();
                            crate::output::print_nl(match_chr);
                            crate::output::print_int(n);
                            crate::output::print_cstr(c"<-".as_ptr());
                            crate::output::show_token_list(
                                pstack[(n - 1) as usize],
                                TEX_NULL,
                                1000,
                            );
                            capture_to_diagnostic(std::ptr::null_mut());
                            crate::output::end_diagnostic(false);
                        }
                    }
                }

                if mem_b32s0(r) == END_MATCH_TOKEN {
                    break 'outer;
                }
            } // end 'outer
        } // end if mem_b32s0(r) != END_MATCH_TOKEN

        // Flush exhausted token lists before installing macro body
        while cur_input().state == TOKEN_LIST
            && cur_input().loc == TEX_NULL
            && cur_input().index != V_TEMPLATE
        {
            crate::input::end_token_list();
        }

        crate::input::begin_token_list(ref_count, MACRO);
        set_cur_input_name(warning_index());
        set_cur_input_loc(mem_b32s1(r));

        if n > 0 {
            if param_ptr() + n > max_param_stack() {
                set_max_param_stack(param_ptr() + n);
                if max_param_stack() > param_size() {
                    crate::errors::overflow(c"parameter stack size".as_ptr(), param_size());
                }
            }
            for m in 0..n {
                set_param_stack(param_ptr() + m, pstack[m as usize]);
            }
            set_param_ptr(param_ptr() + n);
        }
    } // end 'mac (exit:)

    set_scanner_status(save_scanner_status);
    set_warning_index(save_warning_index);
}
