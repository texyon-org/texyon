//! Expansion, macros, and conditionals — `expand`/`macro_call`/`conditional`/
//! `pop_cond_node`/`conv_toks`/`ins_the_toks`. From xetex0.c's expansion group.

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
const HMODE_C: i16 = 104; // xetex_format.h:369
const MMODE_C: i16 = 207; // xetex_format.h:370
const ACTIVE_CHAR_C: u8 = 13; // xetex_format.h:246
const UNDEFINED_CS: u8 = 103; // xetex_format.h:344
const END_CS_NAME: u8 = 67; // xetex_format.h:305
const TOO_BIG_USV_C: i32 = 1_114_112; // xetex_format.h:22 (BIGGEST_USV + 1)
const NORMAL_SCAN: u8 = 0; // xetex-constants.h:182 / xetex_format.h:420
const INT_PAR_TRACING_COMMANDS: i32 = 36; // xetex_format.h:71

// Module-private consts shared with scan.rs (duplicated, same value).
const GARBAGE: i32 = 4_999_999 - 12; // MEM_TOP - 12 (xetex-constants.h:49)
const TEMP_HEAD: i32 = 4_999_999 - 3; // MEM_TOP - 3

use crate::consts::{
    ACTIVE_BASE, BACKUP_HEAD, BOX_BASE, CS_TOKEN_FLAG, ELSE_CODE, END_TEMPLATE, FI_CODE,
    FROZEN_ENDV, FROZEN_PRIMITIVE, INSERTED, INT_BASE, INT_PAR_TRACING_IFS,
    MAX_COMMAND, NORMAL_SCANNER, NULL_CS, OR_CODE,
    PRIM_EQTB_BASE, RELAX, SELECTOR_NEW_STRING, SINGLE_BASE, SPACER, TOO_BIG_CHAR,
    UNDEFINED_PRIMITIVE, UNLESS_CODE, VMODE,
};

// Command/token/code consts that live module-private in scan.rs (duplicated
// here, same values — they are referenced bare by the moved expander match arms).
const CALL: u8 = 113;
const OTHER_TOKEN: i32 = 0x1800000; // OTHER_CHAR << 21
const BIGGEST_USV: i32 = 0x10_FFFF;
const HASH_BASE: i32 = 2_228_226;
const MAX_CHAR_VAL: i32 = 0x200000; // 1 << 21
const NO_EXPAND_FLAG: i32 = 0x10_FFFF + 2;
const AAT_FONT_FLAG: i32 = 0xFFFF;
const OTGR_FONT_FLAG: i32 = 0xFFFE;
const END_MATCH_TOKEN: i32 = 0x1C00000;
const MATCH_TOKEN: i32 = 0x1A00000;
const RIGHT_BRACE_LIMIT: i32 = 0x600000;
const LEFT_BRACE_LIMIT: i32 = 0x400000;
const SPACE_TOKEN: i32 = 0x1400020;
const PROTECTED_TOKEN: i32 = 0x1C00001;

// Command codes used bare in expand/conditional matches (xetex_format.h:352).
const IF_TEST: u8 = 107;

extern "C" {
    fn usingGraphite(engine: *mut libc::c_void) -> bool;
}

use crate::node::{NodePtr, SaNode};
use crate::scan::SccAbort;
use crate::units::{MemIdx, MemPtr, NodeType};

pub(crate) fn ins_the_toks_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    let v = crate::scan::the_toks_in(st)?;
    st.mem.set_b32s1(GARBAGE, v);
    let th = st.mem.b32s1(TEMP_HEAD);
    crate::input::begin_token_list_in(st, th, INSERTED)?;
    Ok(())
}
fn pop_cond_node_in(st: &mut crate::state::EngineState) {
    let p = st.scanner.cond_ptr.0;
    st.scanner.if_line = st.mem.b32s1(p + 1);
    st.scanner.cur_if = st.mem.b16s0(p) as i16;
    st.scanner.if_limit = st.mem.b16s1(p) as u8;
    st.scanner.cond_ptr.0 = st.mem.b32s1(p);
    crate::mem::free_node(&mut st.mem, p, IF_NODE_SIZE);
}

pub(crate) fn conditional_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    use crate::mem::TEX_NULL;
    use crate::scan::{get_avail_scc, get_node_scc, scc_err};

    let mut b = false;
    let e: bool;
    let r: u8;
    let mut m: i32;
    let mut n: i32;
    let mut p: i32;
    let mut q: i32;
    let save_scanner_status: u8;

    if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_IFS) > 0
        && st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_COMMANDS) <= 1
    {
        crate::output::show_cur_cmd_chr_in(st);
    }

    p = get_node_scc(st, IF_NODE_SIZE)?;
    st.mem.set_b32s1(p, st.scanner.cond_ptr.0);
    st.mem.set_b16s1(p, st.scanner.if_limit as u16);
    st.mem.set_b16s0(p, st.scanner.cur_if as u16);
    st.mem.set_b32s1(p + 1, st.scanner.if_line);
    st.scanner.cond_ptr.0 = p;
    st.scanner.cur_if = st.scanner.cur_chr as i16;
    st.scanner.if_limit = IF_CODE;
    st.scanner.if_line = st.mem.line;

    let save_cond_ptr: i32 = st.scanner.cond_ptr.0;
    let is_unless: bool = st.scanner.cur_chr >= UNLESS_CODE;
    let this_if: i16 = (st.scanner.cur_chr % UNLESS_CODE) as i16;

    match this_if {
        IF_CHAR_CODE | IF_CAT_CODE => {
            crate::scan::get_x_token_in(st)?;
            if st.scanner.cur_cmd == RELAX && st.scanner.cur_chr == NO_EXPAND_FLAG {
                st.scanner.cur_cmd = ACTIVE_CHAR_C;
                st.scanner.cur_chr = st.scanner.cur_tok - (CS_TOKEN_FLAG + ACTIVE_BASE);
            }
            if st.scanner.cur_cmd > ACTIVE_CHAR_C || st.scanner.cur_chr > BIGGEST_USV {
                m = RELAX as i32;
                n = TOO_BIG_USV_C;
            } else {
                m = st.scanner.cur_cmd as i32;
                n = st.scanner.cur_chr;
            }
            crate::scan::get_x_token_in(st)?;
            if st.scanner.cur_cmd == RELAX && st.scanner.cur_chr == NO_EXPAND_FLAG {
                st.scanner.cur_cmd = ACTIVE_CHAR_C;
                st.scanner.cur_chr = st.scanner.cur_tok - (CS_TOKEN_FLAG + ACTIVE_BASE);
            }
            if st.scanner.cur_cmd > ACTIVE_CHAR_C || st.scanner.cur_chr > BIGGEST_USV {
                st.scanner.cur_cmd = RELAX;
                st.scanner.cur_chr = TOO_BIG_USV_C;
            }
            b = if this_if == IF_CHAR_CODE {
                n == st.scanner.cur_chr
            } else {
                m == st.scanner.cur_cmd as i32
            };
        }
        IF_INT_CODE | IF_DIM_CODE => {
            if this_if == IF_INT_CODE {
                crate::scan::scan_int_in(st)?;
            } else {
                crate::scan::scan_dimen_in(st, false, false, false)?;
            }
            n = st.scanner.cur_val;
            loop {
                crate::scan::get_x_token_in(st)?;
                if st.scanner.cur_cmd != SPACER {
                    break;
                }
            }
            if st.scanner.cur_tok >= OTHER_TOKEN + 60 && st.scanner.cur_tok <= OTHER_TOKEN + 62 {
                r = (st.scanner.cur_tok - OTHER_TOKEN) as u8;
            } else {
                crate::output::error_here_with_diagnostic_in(
                    st,
                    c"Missing = inserted for ".as_ptr(),
                );
                crate::output::print_cmd_chr_in(st, IF_TEST as u16, this_if as i32);
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                st.errors.help_ptr = 1;
                st.errors.help_line[0] =
                    c"I was expecting to see `<', `=', or `>'. Didn't.".as_ptr();
                let oc = crate::input::back_error_in(st)?;
                scc_err(oc)?;
                r = b'=';
            }
            if this_if == IF_INT_CODE {
                crate::scan::scan_int_in(st)?;
            } else {
                crate::scan::scan_dimen_in(st, false, false, false)?;
            }
            let cv = st.scanner.cur_val;
            b = match r {
                60 => n < cv,
                61 => n == cv,
                _ => n > cv,
            };
        }
        IF_ODD_CODE => {
            crate::scan::scan_int_in(st)?;
            b = (st.scanner.cur_val % 2) != 0;
        }
        IF_VMODE_CODE => {
            b = st.input.cur_list.mode.abs() == VMODE;
        }
        IF_HMODE_CODE => {
            b = st.input.cur_list.mode.abs() == HMODE_C;
        }
        IF_MMODE_CODE => {
            b = st.input.cur_list.mode.abs() == MMODE_C;
        }
        IF_INNER_CODE => {
            b = st.input.cur_list.mode < 0;
        }
        IF_VOID_CODE | IF_HBOX_CODE | IF_VBOX_CODE => {
            crate::scan::scan_register_num_in(st)?;
            p = if st.scanner.cur_val < 256 {
                st.eqtb.b32s1(BOX_BASE + st.scanner.cur_val)
            } else {
                let __cv2 = st.scanner.cur_val;
                crate::save::find_sa_element(&mut st.mem, &mut st.scanner, 4, __cv2, false);
                if st.scanner.cur_ptr.0 == TEX_NULL {
                    TEX_NULL
                } else {
                    NodePtr::<SaNode>::from_ptr(MemPtr::from_raw(st.scanner.cur_ptr.0))
                        .expect("non-null sa element")
                        .sa_ptr(&st.mem)
                        .0
                }
            };
            b = if this_if == IF_VOID_CODE {
                p == TEX_NULL
            } else if p == TEX_NULL {
                false
            } else if this_if == IF_HBOX_CODE {
                st.mem.node_type(MemIdx::new(p)) == Ok(NodeType::HList)
            } else {
                st.mem.node_type(MemIdx::new(p)) == Ok(NodeType::VList)
            };
        }
        IFX_CODE => {
            save_scanner_status = st.scanner.scanner_status;
            st.scanner.scanner_status = NORMAL_SCAN;
            crate::scan::get_next_scc(st)?;
            n = st.scanner.cur_cs;
            p = st.scanner.cur_cmd as i32;
            q = st.scanner.cur_chr;
            crate::scan::get_next_scc(st)?;
            if st.scanner.cur_cmd != p as u8 {
                b = false;
            } else if (st.scanner.cur_cmd as i32) < CALL as i32 {
                b = st.scanner.cur_chr == q;
            } else {
                p = st.mem.b32s1(st.scanner.cur_chr);
                q = st.mem.b32s1(st.eqtb.b32s1(n));
                if p == q {
                    b = true;
                } else {
                    while p != TEX_NULL && q != TEX_NULL {
                        if st.mem.b32s0(p) != st.mem.b32s0(q) {
                            p = TEX_NULL;
                        } else {
                            p = st.mem.b32s1(p);
                            q = st.mem.b32s1(q);
                        }
                    }
                    b = p == TEX_NULL && q == TEX_NULL;
                }
            }
            st.scanner.scanner_status = save_scanner_status;
        }
        IF_EOF_CODE => {
            crate::scan::scan_four_bit_int_or_18_in(st)?;
            b = if st.scanner.cur_val == 18 {
                !st.config.shell_escape_enabled
            } else {
                st.input.read_open[st.scanner.cur_val as usize] == 2 // CLOSED = 2
            };
        }
        IF_TRUE_CODE => {
            b = true;
        }
        IF_FALSE_CODE => {
            b = false;
        }
        IF_DEF_CODE => {
            save_scanner_status = st.scanner.scanner_status;
            st.scanner.scanner_status = NORMAL_SCAN;
            crate::scan::get_next_scc(st)?;
            b = st.scanner.cur_cmd != UNDEFINED_CS;
            st.scanner.scanner_status = save_scanner_status;
        }
        IF_CS_CODE => {
            n = get_avail_scc(st)?;
            p = n;
            e = st.scanner.is_in_csname;
            st.scanner.is_in_csname = true;
            loop {
                crate::scan::get_x_token_in(st)?;
                if st.scanner.cur_cs == 0 {
                    q = get_avail_scc(st)?;
                    st.mem.set_b32s1(p, q);
                    st.mem.set_b32s0(q, st.scanner.cur_tok);
                    p = q;
                }
                if st.scanner.cur_cs != 0 {
                    break;
                }
            }
            if st.scanner.cur_cmd != END_CS_NAME {
                crate::output::error_here_with_diagnostic_in(st, c"Missing ".as_ptr());
                crate::output::print_esc_cstr_in(st, c"endcsname".as_ptr());
                crate::output::print_cstr_in(st, c" inserted".as_ptr());
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                st.errors.help_ptr = 2;
                st.errors.help_line[1] =
                    c"The control sequence marked <to be read again> should".as_ptr();
                st.errors.help_line[0] = c"not appear between \\csname and \\endcsname.".as_ptr();
                let oc = crate::input::back_error_in(st)?;
                scc_err(oc)?;
            }
            m = st.input.first;
            p = st.mem.b32s1(n);
            while p != TEX_NULL {
                if m >= st.input.max_buf_stack {
                    st.input.max_buf_stack = m + 1;
                    if st.input.max_buf_stack == st.input.buf_size {
                        return Err(SccAbort::Input(crate::input::InputAbort::Overflow {
                            what: c"buffer size".as_ptr(),
                            n: st.input.buf_size,
                        }));
                    }
                }
                st.input.buffer[m as usize] = st.mem.b32s0(p) % MAX_CHAR_VAL;
                m += 1;
                p = st.mem.b32s1(p);
            }
            let cur_cs_val = if m == st.input.first {
                NULL_CS
            } else if m == st.input.first + 1 {
                SINGLE_BASE + st.input.buffer[st.input.first as usize]
            } else {
                let s = &mut *st;
                crate::hash::id_lookup(
                    &mut s.eqtb,
                    &mut s.strings,
                    &s.input.buffer,
                    s.input.first,
                    m - s.input.first,
                )
                .map_err(|err| SccAbort::Input(crate::scan::hash_err_abort(err)))?
            };
            st.scanner.cur_cs = cur_cs_val;
            crate::mem::flush_list(&mut st.mem, n);
            b = st.eqtb.b16s1(st.scanner.cur_cs) != UNDEFINED_CS as u16;
            st.scanner.is_in_csname = e;
        }
        IF_IN_CSNAME_CODE => {
            b = st.scanner.is_in_csname;
        }
        IF_FONT_CHAR_CODE => {
            crate::scan::scan_font_ident_in(st)?;
            n = st.scanner.cur_val;
            crate::scan::scan_usv_num_in(st)?;
            let fa = st.fonts.font_area[n as usize];
            b = if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                unsafe { crate::ext::map_char_to_glyph(&st.fonts, n, st.scanner.cur_val) }? > 0
            } else {
                let bc = st.fonts.font_bc[n as usize] as i32;
                let ec = st.fonts.font_ec[n as usize] as i32;
                if bc <= st.scanner.cur_val && ec >= st.scanner.cur_val {
                    let ec_char =
                        crate::fonts::effective_char(&mut st.fonts, true, n, st.scanner.cur_val as u16);
                    let i = unsafe {
                        st.fonts.font_info[(st.fonts.char_base[n as usize] + ec_char) as usize].b16
                    };
                    i.s3 > 0
                } else {
                    false
                }
            };
        }
        IF_CASE_CODE => {
            crate::scan::scan_int_in(st)?;
            n = st.scanner.cur_val;
            if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_COMMANDS) > 1 {
                crate::output::begin_diagnostic_in(st);
                crate::output::diagnostic_begin_capture_warning_here_in(st);
                crate::output::print_cstr_in(st, c"{case ".as_ptr());
                crate::output::print_int_in(st, n);
                crate::output::print_char_in(st, b'}' as i32);
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                crate::output::end_diagnostic_in(st, false);
            }
            while n != 0 {
                crate::scan::pass_text_in(st)?;
                if st.scanner.cond_ptr.0 == save_cond_ptr {
                    if st.scanner.cur_chr == OR_CODE as i32 {
                        n -= 1;
                    } else {
                        // goto common_ending
                        if st.scanner.cur_chr == FI_CODE as i32 {
                            if st.input.if_stack[st.input.in_open as usize] == st.scanner.cond_ptr.0 {
                                crate::output::if_warning_in(st);
                            }
                            pop_cond_node_in(st);
                        } else {
                            st.scanner.if_limit = FI_CODE;
                        }
                        return Ok(());
                    }
                } else if st.scanner.cur_chr == FI_CODE as i32 {
                    if st.input.if_stack[st.input.in_open as usize] == st.scanner.cond_ptr.0 {
                        crate::output::if_warning_in(st);
                    }
                    pop_cond_node_in(st);
                }
            }
            scc_err(crate::output::change_if_limit_in(
                st,
                OR_CODE as i16,
                save_cond_ptr,
            ))?;
            return Ok(());
        }
        IF_PRIMITIVE_CODE => {
            save_scanner_status = st.scanner.scanner_status;
            st.scanner.scanner_status = NORMAL_SCAN;
            crate::scan::get_next_scc(st)?;
            st.scanner.scanner_status = save_scanner_status;
            let cs = st.scanner.cur_cs;
            m = if cs < HASH_BASE {
                crate::hash::prim_lookup(&mut st.eqtb, &mut st.strings, cs - SINGLE_BASE)
                    .map_err(|err| SccAbort::Input(crate::scan::hash_err_abort(err)))?
            } else {
                let s = st.eqtb.hash[(cs as usize) - 514].s1;
                crate::hash::prim_lookup(&mut st.eqtb, &mut st.strings, s)
                    .map_err(|err| SccAbort::Input(crate::scan::hash_err_abort(err)))?
            };
            b = st.scanner.cur_cmd != UNDEFINED_CS
                && m != UNDEFINED_PRIMITIVE
                && st.scanner.cur_cmd as i32 == st.eqtb.b16s1(PRIM_EQTB_BASE + m) as i32
                && st.scanner.cur_chr == st.eqtb.b32s1(PRIM_EQTB_BASE + m);
        }
        _ => {}
    }

    if is_unless {
        b = !b;
    }

    if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_COMMANDS) > 1 {
        crate::output::begin_diagnostic_in(st);
        crate::output::diagnostic_begin_capture_warning_here_in(st);
        if b {
            crate::output::print_cstr_in(st, c"{true}".as_ptr());
        } else {
            crate::output::print_cstr_in(st, c"{false}".as_ptr());
        }
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        crate::output::end_diagnostic_in(st, false);
    }

    if b {
        scc_err(crate::output::change_if_limit_in(
            st,
            ELSE_CODE as i16,
            save_cond_ptr,
        ))?;
        return Ok(());
    }

    // false branch: skip until \else or \fi at our nesting level
    loop {
        crate::scan::pass_text_in(st)?;
        if st.scanner.cond_ptr.0 == save_cond_ptr {
            if st.scanner.cur_chr != OR_CODE as i32 {
                // common_ending
                if st.scanner.cur_chr == FI_CODE as i32 {
                    if st.input.if_stack[st.input.in_open as usize] == st.scanner.cond_ptr.0 {
                        crate::output::if_warning_in(st);
                    }
                    pop_cond_node_in(st);
                } else {
                    st.scanner.if_limit = FI_CODE;
                }
                return Ok(());
            }
            crate::output::error_here_with_diagnostic_in(st, c"Extra ".as_ptr());
            crate::output::print_esc_cstr_in(st, c"or".as_ptr());
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.errors.help_ptr = 1;
            st.errors.help_line[0] = c"I'm ignoring this; it doesn't match any \\if.".as_ptr();
            scc_err(crate::errors::error_in(st))?;
        } else if st.scanner.cur_chr == FI_CODE as i32 {
            if st.input.if_stack[st.input.in_open as usize] == st.scanner.cond_ptr.0 {
                crate::output::if_warning_in(st);
            }
            pop_cond_node_in(st);
        }
    }
}
pub(crate) fn conv_toks_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    use crate::mem::TEX_NULL;
    use crate::scan::{scc_err, str_toks_scc};

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

    const GLUE_PAR_LEFT_SKIP: i32 = 7;
    const GLUE_PAR_RIGHT_SKIP: i32 = 8;

    const TEMP_HEAD_CT: i32 = 4_999_999 - 3;
    const GARBAGE_CT: i32 = 4_999_999 - 12;
    const CONVERT: u16 = 110;

    // pdf_error("tokens", "tokens_to_string() called while selector = new_string")
    macro_rules! tokens_selector_check {
        () => {
            if st.shipout.selector == SELECTOR_NEW_STRING {
                return Err(SccAbort::PdfError {
                    t: c"tokens".as_ptr(),
                    p: c"tokens_to_string() called while selector = new_string".as_ptr(),
                });
            }
        };
    }

    let mut cat: i32 = 0;
    let c = st.scanner.cur_chr;
    let mut fnt: i32 = 0;
    let mut arg1: i32 = 0;
    let mut arg2: i32 = 0;
    let mut p = TEX_NULL;
    let boolvar: bool;
    let s: i32;
    let mut b: i32;

    // ---- Phase 1: scanning ----
    match c {
        NUMBER_CODE_CT | ROMAN_NUMERAL_CODE_CT => {
            crate::scan::scan_int_in(st)?;
        }
        STRING_CODE_CT | MEANING_CODE_CT => {
            let save_ss = st.scanner.scanner_status;
            st.scanner.scanner_status = NORMAL_SCANNER;
            crate::scan::get_token_in(st)?;
            st.scanner.scanner_status = save_ss;
        }
        FONT_NAME_CODE_CT => {
            crate::scan::scan_font_ident_in(st)?;
        }
        ETEX_REVISION_CODE_CT => {}
        EXPANDED_CODE_CT => {
            let save_ss = st.scanner.scanner_status;
            let save_wi = st.scanner.warning_index;
            let save_dr = st.scanner.def_ref.0;
            let u = if st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize]
                < st.strings.pool_ptr
            {
                crate::string_pool::make_string(&mut st.strings)
                    .map_err(|_| make_string_overflow_scc(st))?
            } else {
                0
            };
            crate::scan::scan_pdf_ext_toks_in(st)?;
            st.scanner.warning_index = save_wi;
            st.scanner.scanner_status = save_ss;
            let dr = st.scanner.def_ref.0;
            let tl = st.mem.b32s1(dr);
            crate::input::begin_token_list_in(st, tl, INSERTED)?;
            let dr = st.scanner.def_ref.0;
            st.mem.set_b32s1(dr, st.mem.avail);
            st.mem.avail = dr;
            st.scanner.def_ref.0 = save_dr;
            if u != 0 {
                st.strings.str_ptr -= 1;
            }
            return Ok(());
        }
        LEFT_MARGIN_KERN_CODE_CT | RIGHT_MARGIN_KERN_CODE_CT => {
            crate::scan::scan_register_num_in(st)?;
            if st.scanner.cur_val < 256 {
                p = st.eqtb.b32s1(BOX_BASE + st.scanner.cur_val);
            } else {
                let __cv1 = st.scanner.cur_val;
                crate::save::find_sa_element(&mut st.mem, &mut st.scanner, 4, __cv1, false);
                if st.scanner.cur_ptr.0 == TEX_NULL {
                    p = TEX_NULL;
                } else {
                    p = NodePtr::<SaNode>::from_ptr(MemPtr::from_raw(st.scanner.cur_ptr.0))
                        .expect("non-null sa element")
                        .sa_ptr(&st.mem)
                        .0;
                }
            }
            if p == TEX_NULL || st.mem.node_type(MemIdx::new(p)) != Ok(NodeType::HList) {
                return Err(SccAbort::PdfError {
                    t: c"marginkern".as_ptr(),
                    p: c"a non-empty hbox expected".as_ptr(),
                });
            }
        }
        PDF_CREATION_DATE_CODE_CT => {
            b = st.strings.pool_ptr;
            unsafe { crate::texmfmp::getcreationdate_in(&st.texmf, &mut st.strings) };
            let v = str_toks_scc(st, b, 0)?;
            st.mem.set_b32s1(GARBAGE_CT, v);
            let tl = st.mem.b32s1(TEMP_HEAD_CT);
            crate::input::begin_token_list_in(st, tl, INSERTED)?;
        }
        PDF_FILE_MOD_DATE_CODE_CT => {
            let save_ss = st.scanner.scanner_status;
            let save_wi = st.scanner.warning_index;
            let save_dr = st.scanner.def_ref.0;
            let u = if st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize]
                < st.strings.pool_ptr
            {
                crate::string_pool::make_string(&mut st.strings)
                    .map_err(|_| make_string_overflow_scc(st))?
            } else {
                0
            };
            crate::scan::scan_pdf_ext_toks_in(st)?;
            tokens_selector_check!();
            let old_sel = st.shipout.selector;
            st.shipout.selector = SELECTOR_NEW_STRING;
            let dr = st.scanner.def_ref.0;
            let arg = st.mem.b32s1(dr);
            let l = st.strings.pool_size - st.strings.pool_ptr;
            crate::output::show_token_list_in(st, arg, TEX_NULL, l);
            st.shipout.selector = old_sel;
            s = crate::string_pool::make_string(&mut st.strings)
                .map_err(|_| make_string_overflow_scc(st))?;
            let dr = st.scanner.def_ref.0;
            crate::mem::delete_token_ref(&mut st.mem, dr);
            st.scanner.def_ref.0 = save_dr;
            st.scanner.warning_index = save_wi;
            st.scanner.scanner_status = save_ss;
            b = st.strings.pool_ptr;
            unsafe { crate::texmfmp::getfilemoddate_in(&mut st.strings, s) }?;
            let v = str_toks_scc(st, b, 0)?;
            st.mem.set_b32s1(GARBAGE_CT, v);
            if s == st.strings.str_ptr - 1 {
                st.strings.str_ptr -= 1;
                st.strings.pool_ptr =
                    st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
            }
            let tl = st.mem.b32s1(TEMP_HEAD_CT);
            crate::input::begin_token_list_in(st, tl, INSERTED)?;
            if u != 0 {
                st.strings.str_ptr -= 1;
            }
            return Ok(());
        }
        PDF_FILE_SIZE_CODE_CT => {
            let save_ss = st.scanner.scanner_status;
            let save_wi = st.scanner.warning_index;
            let save_dr = st.scanner.def_ref.0;
            let u = if st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize]
                < st.strings.pool_ptr
            {
                crate::string_pool::make_string(&mut st.strings)
                    .map_err(|_| make_string_overflow_scc(st))?
            } else {
                0
            };
            crate::scan::scan_pdf_ext_toks_in(st)?;
            tokens_selector_check!();
            let old_sel = st.shipout.selector;
            st.shipout.selector = SELECTOR_NEW_STRING;
            let dr = st.scanner.def_ref.0;
            let arg = st.mem.b32s1(dr);
            let l = st.strings.pool_size - st.strings.pool_ptr;
            crate::output::show_token_list_in(st, arg, TEX_NULL, l);
            st.shipout.selector = old_sel;
            s = crate::string_pool::make_string(&mut st.strings)
                .map_err(|_| make_string_overflow_scc(st))?;
            let dr = st.scanner.def_ref.0;
            crate::mem::delete_token_ref(&mut st.mem, dr);
            st.scanner.def_ref.0 = save_dr;
            st.scanner.warning_index = save_wi;
            st.scanner.scanner_status = save_ss;
            b = st.strings.pool_ptr;
            unsafe { crate::texmfmp::getfilesize_in(&mut st.strings, s) }?;
            let v = str_toks_scc(st, b, 0)?;
            st.mem.set_b32s1(GARBAGE_CT, v);
            if s == st.strings.str_ptr - 1 {
                st.strings.str_ptr -= 1;
                st.strings.pool_ptr =
                    st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
            }
            let tl = st.mem.b32s1(TEMP_HEAD_CT);
            crate::input::begin_token_list_in(st, tl, INSERTED)?;
            if u != 0 {
                st.strings.str_ptr -= 1;
            }
            return Ok(());
        }
        PDF_MDFIVE_SUM_CODE_CT => {
            let save_ss = st.scanner.scanner_status;
            let save_wi = st.scanner.warning_index;
            let save_dr = st.scanner.def_ref.0;
            let u = if st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize]
                < st.strings.pool_ptr
            {
                crate::string_pool::make_string(&mut st.strings)
                    .map_err(|_| make_string_overflow_scc(st))?
            } else {
                0
            };
            boolvar = crate::scan::scan_keyword_in(st, c"file".as_ptr())?;
            crate::scan::scan_pdf_ext_toks_in(st)?;
            tokens_selector_check!();
            let old_sel = st.shipout.selector;
            st.shipout.selector = SELECTOR_NEW_STRING;
            let dr = st.scanner.def_ref.0;
            let arg = st.mem.b32s1(dr);
            let l = st.strings.pool_size - st.strings.pool_ptr;
            crate::output::show_token_list_in(st, arg, TEX_NULL, l);
            st.shipout.selector = old_sel;
            s = crate::string_pool::make_string(&mut st.strings)
                .map_err(|_| make_string_overflow_scc(st))?;
            let dr = st.scanner.def_ref.0;
            crate::mem::delete_token_ref(&mut st.mem, dr);
            st.scanner.def_ref.0 = save_dr;
            st.scanner.warning_index = save_wi;
            st.scanner.scanner_status = save_ss;
            b = st.strings.pool_ptr;
            unsafe { crate::texmfmp::getmd5sum_in(&mut st.strings, s, boolvar) };
            let v = str_toks_scc(st, b, 0)?;
            st.mem.set_b32s1(GARBAGE_CT, v);
            if s == st.strings.str_ptr - 1 {
                st.strings.str_ptr -= 1;
                st.strings.pool_ptr =
                    st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
            }
            let tl = st.mem.b32s1(TEMP_HEAD_CT);
            crate::input::begin_token_list_in(st, tl, INSERTED)?;
            if u != 0 {
                st.strings.str_ptr -= 1;
            }
            return Ok(());
        }
        PDF_FILE_DUMP_CODE_CT => {
            let save_ss = st.scanner.scanner_status;
            let save_wi = st.scanner.warning_index;
            let save_dr = st.scanner.def_ref.0;
            let u = if st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize]
                < st.strings.pool_ptr
            {
                crate::string_pool::make_string(&mut st.strings)
                    .map_err(|_| make_string_overflow_scc(st))?
            } else {
                0
            };
            st.scanner.cur_val = 0;
            if crate::scan::scan_keyword_in(st, c"offset".as_ptr())? {
                crate::scan::scan_int_in(st)?;
                if st.scanner.cur_val < 0 {
                    crate::output::error_here_with_diagnostic_in(st, c"Bad file offset".as_ptr());
                    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                    st.errors.help_ptr = 2;
                    st.errors.help_line[1] =
                        c"A file offset must be between 0 and 2^_31_-1,".as_ptr();
                    st.errors.help_line[0] = c"I changed this one to zero.".as_ptr();
                    let cv = st.scanner.cur_val;
                    scc_err(crate::errors::int_error_in(st, cv))?;
                    st.scanner.cur_val = 0;
                }
            }
            let i = st.scanner.cur_val;
            st.scanner.cur_val = 0;
            if crate::scan::scan_keyword_in(st, c"length".as_ptr())? {
                crate::scan::scan_int_in(st)?;
                if st.scanner.cur_val < 0 {
                    crate::output::error_here_with_diagnostic_in(st, c"Bad dump length".as_ptr());
                    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                    st.errors.help_ptr = 2;
                    st.errors.help_line[1] =
                        c"A dump length must be between 0 and 2^_31_-1,".as_ptr();
                    st.errors.help_line[0] = c"I changed this one to zero.".as_ptr();
                    let cv = st.scanner.cur_val;
                    scc_err(crate::errors::int_error_in(st, cv))?;
                    st.scanner.cur_val = 0;
                }
            }
            let j = st.scanner.cur_val;
            crate::scan::scan_pdf_ext_toks_in(st)?;
            tokens_selector_check!();
            let old_sel = st.shipout.selector;
            st.shipout.selector = SELECTOR_NEW_STRING;
            let dr = st.scanner.def_ref.0;
            let arg = st.mem.b32s1(dr);
            let l = st.strings.pool_size - st.strings.pool_ptr;
            crate::output::show_token_list_in(st, arg, TEX_NULL, l);
            st.shipout.selector = old_sel;
            s = crate::string_pool::make_string(&mut st.strings)
                .map_err(|_| make_string_overflow_scc(st))?;
            let dr = st.scanner.def_ref.0;
            crate::mem::delete_token_ref(&mut st.mem, dr);
            st.scanner.def_ref.0 = save_dr;
            st.scanner.warning_index = save_wi;
            st.scanner.scanner_status = save_ss;
            b = st.strings.pool_ptr;
            unsafe { crate::texmfmp::getfiledump_in(&mut st.strings, s, i, j) }?;
            let v = str_toks_scc(st, b, 0)?;
            st.mem.set_b32s1(GARBAGE_CT, v);
            if s == st.strings.str_ptr - 1 {
                st.strings.str_ptr -= 1;
                st.strings.pool_ptr =
                    st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
            }
            let tl = st.mem.b32s1(TEMP_HEAD_CT);
            crate::input::begin_token_list_in(st, tl, INSERTED)?;
            if u != 0 {
                st.strings.str_ptr -= 1;
            }
            return Ok(());
        }
        PDF_STRCMP_CODE_CT => {
            let save_ss = st.scanner.scanner_status;
            let save_wi = st.scanner.warning_index;
            let save_dr = st.scanner.def_ref.0;
            let u = if st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize]
                < st.strings.pool_ptr
            {
                crate::string_pool::make_string(&mut st.strings)
                    .map_err(|_| make_string_overflow_scc(st))?
            } else {
                0
            };
            crate::scan::compare_strings_in(st)?;
            st.scanner.def_ref.0 = save_dr;
            st.scanner.warning_index = save_wi;
            st.scanner.scanner_status = save_ss;
            if u != 0 {
                st.strings.str_ptr -= 1;
            }
            // falls through to phase 2 (PDF_STRCMP_CODE_CT case there)
        }
        XETEX_UCHAR_CODE_CT => {
            crate::scan::scan_usv_num_in(st)?;
        }
        XETEX_UCHARCAT_CODE_CT => {
            crate::scan::scan_usv_num_in(st)?;
            let saved_chr = st.scanner.cur_val;
            crate::scan::scan_int_in(st)?;
            const LEFT_BRACE_CV: i32 = 1;
            const ACTIVE_CHAR_CV: i32 = 13;
            const OUT_PARAM_CV: i32 = 5;
            const IGNORE_CV: i32 = 9;
            if st.scanner.cur_val < LEFT_BRACE_CV
                || st.scanner.cur_val > ACTIVE_CHAR_CV
                || st.scanner.cur_val == OUT_PARAM_CV
                || st.scanner.cur_val == IGNORE_CV
            {
                crate::output::error_here_with_diagnostic_in(st, c"Invalid code (".as_ptr());
                crate::output::print_int_in(st, st.scanner.cur_val);
                crate::output::print_cstr_in(
                    st,
                    c"), should be in the ranges 1..4, 6..8, 10..13".as_ptr(),
                );
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                st.errors.help_ptr = 1;
                st.errors.help_line[0] =
                    c"I'm going to use 12 instead of that illegal code value.".as_ptr();
                scc_err(crate::errors::error_in(st))?;
                cat = 12;
            } else {
                cat = st.scanner.cur_val;
            }
            st.scanner.cur_val = saved_chr;
        }
        XETEX_REVISION_CODE_CT => {}
        XETEX_VARIATION_NAME_CODE_CT => {
            crate::scan::scan_font_ident_in(st)?;
            fnt = st.scanner.cur_val;
            if st.fonts.font_area[fnt as usize] == AAT_FONT_FLAG {
                crate::scan::scan_int_in(st)?;
                arg1 = st.scanner.cur_val;
                arg2 = 0;
            } else {
                scc_err(crate::errors::not_font_error_common_in(
                    st,
                    CONVERT,
                    c,
                    fnt,
                    c"; not an AAT font".as_ptr(),
                ))?;
            }
        }
        XETEX_FEATURE_NAME_CODE_CT => {
            crate::scan::scan_font_ident_in(st)?;
            fnt = st.scanner.cur_val;
            let fa = st.fonts.font_area[fnt as usize];
            if fa == AAT_FONT_FLAG
                || (fa == OTGR_FONT_FLAG
                    && unsafe { usingGraphite(st.fonts.font_layout_engine[fnt as usize]) })
            {
                crate::scan::scan_int_in(st)?;
                arg1 = st.scanner.cur_val;
                arg2 = 0;
            } else {
                scc_err(crate::errors::not_font_error_common_in(
                    st,
                    CONVERT,
                    c,
                    fnt,
                    c"; not an AAT or Graphite font".as_ptr(),
                ))?;
            }
        }
        XETEX_SELECTOR_NAME_CODE_CT => {
            crate::scan::scan_font_ident_in(st)?;
            fnt = st.scanner.cur_val;
            let fa = st.fonts.font_area[fnt as usize];
            if fa == AAT_FONT_FLAG
                || (fa == OTGR_FONT_FLAG
                    && unsafe { usingGraphite(st.fonts.font_layout_engine[fnt as usize]) })
            {
                crate::scan::scan_int_in(st)?;
                arg1 = st.scanner.cur_val;
                crate::scan::scan_int_in(st)?;
                arg2 = st.scanner.cur_val;
            } else {
                scc_err(crate::errors::not_font_error_common_in(
                    st,
                    CONVERT,
                    c,
                    fnt,
                    c"; not an AAT or Graphite font".as_ptr(),
                ))?;
            }
        }
        XETEX_GLYPH_NAME_CODE_CT => {
            crate::scan::scan_font_ident_in(st)?;
            fnt = st.scanner.cur_val;
            let fa = st.fonts.font_area[fnt as usize];
            if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                crate::scan::scan_int_in(st)?;
                arg1 = st.scanner.cur_val;
            } else {
                scc_err(crate::errors::not_font_error_common_in(
                    st,
                    CONVERT,
                    c,
                    fnt,
                    c"; not a native platform font".as_ptr(),
                ))?;
            }
        }
        JOB_NAME_CODE_CT => {
            if st.strings.job_name == 0 {
                unsafe { crate::io::open_log_file_in(st) }.map_err(SccAbort::OpenLog)?;
            }
        }
        UNIFORM_DEVIATE_CODE_CT => {
            crate::scan::scan_int_in(st)?;
        }
        _ => {} // NORMAL_DEVIATE_CODE_CT and unrecognized codes
    }

    // ---- Phase 2: output ----
    let old_sel = st.shipout.selector;
    st.shipout.selector = SELECTOR_NEW_STRING;
    b = st.strings.pool_ptr;

    match c {
        NUMBER_CODE_CT => {
            crate::output::print_int_in(st, st.scanner.cur_val);
        }
        ROMAN_NUMERAL_CODE_CT => {
            crate::output::print_roman_int_in(st, st.scanner.cur_val);
        }
        STRING_CODE_CT => {
            if st.scanner.cur_cs != 0 {
                crate::output::sprint_cs_in(st, st.scanner.cur_cs);
            } else {
                crate::output::print_char_in(st, st.scanner.cur_chr);
            }
        }
        MEANING_CODE_CT => {
            crate::output::print_meaning_in(st);
        }
        FONT_NAME_CODE_CT => {
            let font_name_str = st.fonts.font_name[st.scanner.cur_val as usize];
            let fa = st.fonts.font_area[st.scanner.cur_val as usize];
            if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                let mut quote_char: i32 = '"' as i32;
                let base = st.strings.str_start[(font_name_str - TOO_BIG_CHAR) as usize];
                for i in 0..crate::string_pool::length(&st.strings, font_name_str) {
                    if st.strings.str_pool[(base + i) as usize] == b'"' as u16 {
                        quote_char = '\'' as i32;
                    }
                }
                crate::output::print_char_in(st, quote_char);
                crate::output::print_in(st, font_name_str);
                crate::output::print_char_in(st, quote_char);
            } else {
                crate::output::print_in(st, font_name_str);
            }
            if st.fonts.font_size[st.scanner.cur_val as usize] != st.fonts.font_dsize[st.scanner.cur_val as usize] {
                crate::output::print_cstr_in(st, c" at ".as_ptr());
                let fs = st.fonts.font_size[st.scanner.cur_val as usize];
                crate::output::print_scaled_in(st, fs);
                crate::output::print_cstr_in(st, c"pt".as_ptr());
            }
        }
        ETEX_REVISION_CODE_CT => {
            crate::output::print_cstr_in(st, c".6".as_ptr());
        }
        LEFT_MARGIN_KERN_CODE_CT => {
            // `p` is the guaranteed-HLIST box register (validated at the scan
            // above); read its list head through the typed box handle.
            let bx = st
                .mem
                .classify_box(MemIdx::new(p))
                .expect("valid box")
                .expect("hlist box register");
            p = bx.list_ptr(&st.mem).0;
            while p != TEX_NULL {
                let tp = st.mem.node_type(MemIdx::new(p));
                let is_low = p < st.mem.hi_mem_min;
                let skip = (is_low
                    && (tp == Ok(NodeType::Ins)
                        || tp == Ok(NodeType::Mark)
                        || tp == Ok(NodeType::Adjust)
                        || tp == Ok(NodeType::Penalty)
                        || (tp == Ok(NodeType::Disc)
                            && st.mem.b32s0(p + 1) == TEX_NULL
                            && st.mem.b32s1(p + 1) == TEX_NULL
                            && st.mem.b16s0(p) == 0)
                        || (tp == Ok(NodeType::Math) && st.mem.b32s1(p + 1) == 0)
                        || (tp == Ok(NodeType::Kern)
                            && (st.mem.b32s1(p + 1) == 0 || st.mem.b16s0(p) == 0))
                        || (tp == Ok(NodeType::Glue) && st.mem.b32s0(p + 1) == 0)
                        || (tp == Ok(NodeType::HList)
                            && st.mem.b32s1(p + 1) == 0
                            && st.mem.b32s1(p + 3) == 0
                            && st.mem.b32s1(p + 2) == 0
                            && st.mem.b32s1(p + 5) == TEX_NULL)))
                    || (is_low
                        && tp == Ok(NodeType::Glue)
                        && st.mem.b16s0(p) == (GLUE_PAR_LEFT_SKIP + 1) as u16);
                if !skip {
                    break;
                }
                p = st.mem.b32s1(p);
            }
            if p != TEX_NULL
                && p < st.mem.hi_mem_min
                && st.mem.node_type(MemIdx::new(p)) == Ok(NodeType::MarginKern)
                && st.mem.b16s0(p) == 0
            {
                let mk = st
                    .mem
                    .classify_margin_kern(MemIdx::new(p))
                    .expect("valid margin kern")
                    .expect("margin kern");
                let w = mk.width(&st.mem).0;
                crate::output::print_scaled_in(st, w);
            } else {
                crate::output::print_char_in(st, '0' as i32);
            }
            crate::output::print_cstr_in(st, c"pt".as_ptr());
        }
        RIGHT_MARGIN_KERN_CODE_CT => {
            let bx = st
                .mem
                .classify_box(MemIdx::new(p))
                .expect("valid box")
                .expect("hlist box register");
            let q = bx.list_ptr(&st.mem).0;
            p = crate::mem::prev_rightmost(&st.mem, q, TEX_NULL);
            while p != TEX_NULL {
                let tp = st.mem.node_type(MemIdx::new(p));
                let is_low = p < st.mem.hi_mem_min;
                let skip = (is_low
                    && (tp == Ok(NodeType::Ins)
                        || tp == Ok(NodeType::Mark)
                        || tp == Ok(NodeType::Adjust)
                        || tp == Ok(NodeType::Penalty)
                        || (tp == Ok(NodeType::Disc)
                            && st.mem.b32s0(p + 1) == TEX_NULL
                            && st.mem.b32s1(p + 1) == TEX_NULL
                            && st.mem.b16s0(p) == 0)
                        || (tp == Ok(NodeType::Math) && st.mem.b32s1(p + 1) == 0)
                        || (tp == Ok(NodeType::Kern)
                            && (st.mem.b32s1(p + 1) == 0 || st.mem.b16s0(p) == 0))
                        || (tp == Ok(NodeType::Glue) && st.mem.b32s0(p + 1) == 0)
                        || (tp == Ok(NodeType::HList)
                            && st.mem.b32s1(p + 1) == 0
                            && st.mem.b32s1(p + 3) == 0
                            && st.mem.b32s1(p + 2) == 0
                            && st.mem.b32s1(p + 5) == TEX_NULL)))
                    || (is_low
                        && tp == Ok(NodeType::Glue)
                        && st.mem.b16s0(p) == (GLUE_PAR_RIGHT_SKIP + 1) as u16);
                if !skip {
                    break;
                }
                p = crate::mem::prev_rightmost(&st.mem, q, p);
            }
            if p != TEX_NULL
                && p < st.mem.hi_mem_min
                && st.mem.node_type(MemIdx::new(p)) == Ok(NodeType::MarginKern)
                && st.mem.b16s0(p) == 1
            {
                let mk = st
                    .mem
                    .classify_margin_kern(MemIdx::new(p))
                    .expect("valid margin kern")
                    .expect("margin kern");
                let w = mk.width(&st.mem).0;
                crate::output::print_scaled_in(st, w);
            } else {
                crate::output::print_char_in(st, '0' as i32);
            }
            crate::output::print_cstr_in(st, c"pt".as_ptr());
        }
        PDF_STRCMP_CODE_CT => {
            crate::output::print_int_in(st, st.scanner.cur_val);
        }
        UNIFORM_DEVIATE_CODE_CT => {
            let v = crate::scaled_math::unif_rand(&mut st.errors, st.scanner.cur_val);
            crate::output::print_int_in(st, v);
        }
        NORMAL_DEVIATE_CODE_CT => {
            let v = unsafe { crate::scaled_math::norm_rand(st) };
            crate::output::print_int_in(st, v);
        }
        XETEX_UCHAR_CODE_CT | XETEX_UCHARCAT_CODE_CT => {
            crate::output::print_char_in(st, st.scanner.cur_val);
        }
        XETEX_REVISION_CODE_CT => {
            crate::output::print_cstr_in(st, c".999994".as_ptr());
        }
        XETEX_VARIATION_NAME_CODE_CT => {
            if st.fonts.font_area[fnt as usize] == AAT_FONT_FLAG {
                crate::ext::aat_print_font_name(
                    c,
                    st.fonts.font_layout_engine[fnt as usize],
                    arg1,
                    arg2,
                );
            }
        }
        XETEX_FEATURE_NAME_CODE_CT | XETEX_SELECTOR_NAME_CODE_CT => {
            let fa = st.fonts.font_area[fnt as usize];
            let engine = st.fonts.font_layout_engine[fnt as usize];
            if fa == AAT_FONT_FLAG {
                crate::ext::aat_print_font_name(c, engine, arg1, arg2);
            } else if fa == OTGR_FONT_FLAG && unsafe { usingGraphite(engine) } {
                unsafe { crate::ext::gr_print_font_name_in(st, c, engine, arg1, arg2) };
            }
        }
        XETEX_GLYPH_NAME_CODE_CT => {
            let fa = st.fonts.font_area[fnt as usize];
            if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                unsafe { crate::ext::print_glyph_name_in(st, fnt, arg1) }?;
            }
        }
        JOB_NAME_CODE_CT => {
            let jn = st.strings.job_name;
            crate::output::print_file_name_in(st, jn, 0, 0);
        }
        _ => {}
    }

    st.shipout.selector = old_sel;
    let v = str_toks_scc(st, b, cat as i16)?;
    st.mem.set_b32s1(GARBAGE_CT, v);
    let tl = st.mem.b32s1(TEMP_HEAD_CT);
    crate::input::begin_token_list_in(st, tl, INSERTED)?;
    Ok(())
}

/// `make_string` pool overflow lifted into [`SccAbort`] for `conv_toks`.
fn make_string_overflow_scc(st: &crate::state::EngineState) -> SccAbort {
    SccAbort::Input(crate::input::InputAbort::Overflow {
        what: c"number of strings".as_ptr(),
        n: st.strings.max_strings - st.strings.init_str_ptr,
    })
}
pub(crate) fn expand_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    use crate::scan::{get_avail_scc, scc_err};

    const TOP_BOT_MARK: u8 = 112;
    const EXPAND_AFTER: u8 = 104;
    const NO_EXPAND: u8 = 105;
    const INPUT: u8 = 106;
    const CS_NAME_CMD: u8 = 109;
    const CONVERT_CMD: u8 = 110;
    const THE_CMD: u8 = 111;
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
    const UNDEFINED_CS_CMD: u16 = 103;
    const INT_PAR_TRACING_COMMANDS: i32 = 36;
    const INT_PAR_TRACING_IFS: i32 = 58;
    const FI_CODE_VAL: i32 = 2;

    st.scanner.expand_depth_count += 1;
    if st.scanner.expand_depth_count >= st.scanner.expand_depth {
        return Err(SccAbort::Input(crate::input::InputAbort::Overflow {
            what: c"expansion depth".as_ptr(),
            n: st.scanner.expand_depth,
        }));
    }

    let cv_backup = st.scanner.cur_val;
    let cvl_backup = st.scanner.cur_val_level;
    let radix_backup = st.scanner.radix;
    let co_backup = st.scanner.cur_order;
    let backup_backup = st.mem.b32s1(BACKUP_HEAD);

    'reswitch: loop {
        if st.scanner.cur_cmd < CALL {
            if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_COMMANDS) > 1 {
                crate::output::show_cur_cmd_chr_in(st);
            }
            match st.scanner.cur_cmd {
                TOP_BOT_MARK => {
                    let t = (st.scanner.cur_chr % MARKS_CODE) as usize;
                    if st.scanner.cur_chr >= MARKS_CODE {
                        crate::scan::scan_register_num_in(st)?;
                    } else {
                        st.scanner.cur_val = 0;
                    }
                    if st.scanner.cur_val == 0 {
                        st.scanner.cur_ptr.0 = st.scanner.cur_mark[t];
                    } else {
                        let __cv0 = st.scanner.cur_val;
                        crate::save::find_sa_element(
                            &mut st.mem,
                            &mut st.scanner,
                            MARK_VAL_IDX as u16,
                            __cv0,
                            false,
                        );
                        if st.scanner.cur_ptr.0 != crate::mem::TEX_NULL {
                            let sa = NodePtr::<SaNode>::from_ptr(MemPtr::from_raw(st.scanner.cur_ptr.0))
                                .expect("non-null sa element");
                            st.scanner.cur_ptr.0 = sa.child(&st.mem, t as i32).0;
                        }
                    }
                    if st.scanner.cur_ptr.0 != crate::mem::TEX_NULL {
                        crate::input::begin_token_list_in(st, st.scanner.cur_ptr.0, MARK_TEXT_TL)?;
                    }
                }
                EXPAND_AFTER => {
                    if st.scanner.cur_chr == 0 {
                        crate::scan::get_token_in(st)?;
                        let t = st.scanner.cur_tok;
                        crate::scan::get_token_in(st)?;
                        if st.scanner.cur_cmd > MAX_COMMAND {
                            expand_in(st)?;
                        } else {
                            crate::input::back_input_in(st)?;
                        }
                        st.scanner.cur_tok = t;
                        crate::input::back_input_in(st)?;
                    } else {
                        // \unless
                        crate::scan::get_token_in(st)?;
                        if st.scanner.cur_cmd == IF_TEST_CMD && st.scanner.cur_chr != IF_CASE_CODE_VAL {
                            st.scanner.cur_chr = st.scanner.cur_chr + UNLESS_CODE;
                            continue 'reswitch;
                        }
                        crate::output::error_here_with_diagnostic_in(
                            st,
                            c"You can't use `".as_ptr(),
                        );
                        crate::output::print_esc_cstr_in(st, c"unless".as_ptr());
                        crate::output::print_cstr_in(st, c"' before `".as_ptr());
                        crate::output::print_cmd_chr_in(st, st.scanner.cur_cmd as u16, st.scanner.cur_chr);
                        crate::output::print_char_in(st, '\'' as i32);
                        crate::output::capture_to_diagnostic_in(st, core::ptr::null_mut());
                        st.errors.help_ptr = 1;
                        st.errors.help_line[0] =
                            c"Continue, and I'll forget that it ever happened.".as_ptr();
                        let oc = crate::input::back_error_in(st)?;
                        scc_err(oc)?;
                    }
                }
                NO_EXPAND => {
                    if st.scanner.cur_chr == 0 {
                        let save_scanner_status = st.scanner.scanner_status;
                        st.scanner.scanner_status = NORMAL_SCANNER;
                        crate::scan::get_token_in(st)?;
                        st.scanner.scanner_status = save_scanner_status;
                        let t = st.scanner.cur_tok;
                        crate::input::back_input_in(st)?;
                        if t >= CS_TOKEN_FLAG {
                            let p = get_avail_scc(st)?;
                            st.mem.set_b32s0(p, FROZEN_CS_TOKEN);
                            st.mem.set_b32s1(p, st.input.cur_input.loc);
                            st.input.cur_input.start = p;
                            st.input.cur_input.loc = p;
                        }
                    } else {
                        // \primitive
                        let save_scanner_status = st.scanner.scanner_status;
                        st.scanner.scanner_status = NORMAL_SCANNER;
                        crate::scan::get_token_in(st)?;
                        st.scanner.scanner_status = save_scanner_status;
                        let cs = if st.scanner.cur_cs < HASH_BASE {
                            crate::hash::prim_lookup(
                                &mut st.eqtb,
                                &mut st.strings,
                                st.scanner.cur_cs - SINGLE_BASE,
                            )
                            .map_err(|e| SccAbort::Input(crate::scan::hash_err_abort(e)))?
                        } else {
                            let s = st.eqtb.hash_s1(st.scanner.cur_cs);
                            crate::hash::prim_lookup(&mut st.eqtb, &mut st.strings, s)
                                .map_err(|e| SccAbort::Input(crate::scan::hash_err_abort(e)))?
                        };
                        st.scanner.cur_cs = cs;
                        if st.scanner.cur_cs != UNDEFINED_PRIMITIVE {
                            let t = st.eqtb.b16s1(PRIM_EQTB_BASE + st.scanner.cur_cs) as u8;
                            if t > MAX_COMMAND {
                                st.scanner.cur_cmd = t;
                                st.scanner.cur_chr = st.eqtb.b32s1(PRIM_EQTB_BASE + st.scanner.cur_cs);
                                st.scanner.cur_tok = st.scanner.cur_cmd as i32 * MAX_CHAR_VAL + st.scanner.cur_chr;
                                st.scanner.cur_cs = 0;
                                continue 'reswitch;
                            } else {
                                crate::input::back_input_in(st)?;
                                let p = get_avail_scc(st)?;
                                st.mem.set_b32s0(p, FROZEN_PRIM_CS_TOKEN);
                                st.mem.set_b32s1(p, st.input.cur_input.loc);
                                st.input.cur_input.loc = p;
                                st.input.cur_input.start = p;
                            }
                        }
                    }
                }
                CS_NAME_CMD => {
                    let r = get_avail_scc(st)?;
                    let mut p = r;
                    let b = st.scanner.is_in_csname;
                    st.scanner.is_in_csname = true;
                    loop {
                        crate::scan::get_x_token_in(st)?;
                        if st.scanner.cur_cs == 0 {
                            let q = get_avail_scc(st)?;
                            st.mem.set_b32s1(p, q);
                            st.mem.set_b32s0(q, st.scanner.cur_tok);
                            p = q;
                        }
                        if st.scanner.cur_cs != 0 {
                            break;
                        }
                    }
                    if st.scanner.cur_cmd != END_CS_NAME {
                        crate::output::error_here_with_diagnostic_in(st, c"Missing ".as_ptr());
                        crate::output::print_esc_cstr_in(st, c"endcsname".as_ptr());
                        crate::output::print_cstr_in(st, c" inserted".as_ptr());
                        crate::output::capture_to_diagnostic_in(st, core::ptr::null_mut());
                        st.errors.help_ptr = 2;
                        st.errors.help_line[1] =
                            c"The control sequence marked <to be read again> should".as_ptr();
                        st.errors.help_line[0] =
                            c"not appear between \\csname and \\endcsname.".as_ptr();
                        let oc = crate::input::back_error_in(st)?;
                        scc_err(oc)?;
                    }
                    st.scanner.is_in_csname = b;
                    let mut j = st.input.first;
                    p = st.mem.b32s1(r);
                    while p != crate::mem::TEX_NULL {
                        if j >= st.input.max_buf_stack {
                            st.input.max_buf_stack = j + 1;
                            if st.input.max_buf_stack == st.input.buf_size {
                                return Err(SccAbort::Input(crate::input::InputAbort::Overflow {
                                    what: c"buffer size".as_ptr(),
                                    n: st.input.buf_size,
                                }));
                            }
                        }
                        st.input.buffer[j as usize] = st.mem.b32s0(p) % MAX_CHAR_VAL;
                        j += 1;
                        p = st.mem.b32s1(p);
                    }
                    let cs = if j > st.input.first + 1 || st.input.buffer[st.input.first as usize] > 65535 {
                        st.eqtb.no_new_control_sequence = false;
                        let s = &mut *st;
                        let cs = crate::hash::id_lookup(
                            &mut s.eqtb,
                            &mut s.strings,
                            &s.input.buffer,
                            s.input.first,
                            j - s.input.first,
                        )
                        .map_err(|e| SccAbort::Input(crate::scan::hash_err_abort(e)))?;
                        st.eqtb.no_new_control_sequence = true;
                        cs
                    } else if j == st.input.first {
                        NULL_CS_V
                    } else {
                        SINGLE_BASE + st.input.buffer[st.input.first as usize]
                    };
                    st.scanner.cur_cs = cs;
                    crate::mem::flush_list(&mut st.mem, r);
                    if st.eqtb.b16s1(st.scanner.cur_cs) == UNDEFINED_CS_CMD {
                        let cur_cs = st.scanner.cur_cs;
                        let s = &mut *st;
                        crate::save::eq_define(
                            &mut s.eqtb,
                            &mut s.save_stack,
                            &mut s.mem,
                            &mut s.scanner,
                            cur_cs,
                            RELAX as u16,
                            TOO_BIG_USV,
                        );
                    }
                    st.scanner.cur_tok = st.scanner.cur_cs + CS_TOKEN_FLAG;
                    crate::input::back_input_in(st)?;
                }
                CONVERT_CMD => {
                    conv_toks_in(st)?;
                }
                THE_CMD => {
                    ins_the_toks_in(st)?;
                }
                IF_TEST_CMD => {
                    conditional_in(st)?;
                }
                FI_OR_ELSE_CMD => {
                    if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_IFS) > 0
                        && st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_COMMANDS) <= 1
                    {
                        crate::output::show_cur_cmd_chr_in(st);
                    }
                    if st.scanner.cur_chr > st.scanner.if_limit as i32 {
                        if st.scanner.if_limit == IF_CODE_VAL {
                            crate::input::insert_relax_in(st)?;
                        } else {
                            crate::output::error_here_with_diagnostic_in(st, c"Extra ".as_ptr());
                            crate::output::print_cmd_chr_in(st, FI_OR_ELSE_CMD as u16, st.scanner.cur_chr);
                            crate::output::capture_to_diagnostic_in(st, core::ptr::null_mut());
                            st.errors.help_ptr = 1;
                            st.errors.help_line[0] =
                                c"I'm ignoring this; it doesn't match any \\if.".as_ptr();
                            scc_err(crate::errors::error_in(st))?;
                        }
                    } else {
                        while st.scanner.cur_chr != FI_CODE_VAL {
                            crate::scan::pass_text_in(st)?;
                        }
                        if st.input.if_stack[st.input.in_open as usize] == st.scanner.cond_ptr.0 {
                            crate::output::if_warning_in(st);
                        }
                        let p = st.scanner.cond_ptr.0;
                        st.scanner.if_line = st.mem.b32s1(p + 1);
                        st.scanner.cur_if = st.mem.b16s0(p) as i16;
                        st.scanner.if_limit = st.mem.b16s1(p) as u8;
                        st.scanner.cond_ptr.0 = st.mem.b32s1(p);
                        crate::mem::free_node(&mut st.mem, p, IF_NODE_SIZE_V);
                    }
                }
                INPUT => {
                    if st.scanner.cur_chr == 1 {
                        st.input.force_eof = true;
                    } else if st.scanner.cur_chr == 2 {
                        crate::scan::pseudo_start_in(st)?;
                    } else if st.scanner.name_in_progress {
                        crate::input::insert_relax_in(st)?;
                    } else {
                        unsafe { crate::io::start_input_in(st, core::ptr::null()) }?;
                    }
                }
                _ => {
                    crate::output::error_here_with_diagnostic_in(
                        st,
                        c"Undefined control sequence".as_ptr(),
                    );
                    crate::output::capture_to_diagnostic_in(st, core::ptr::null_mut());
                    st.errors.help_ptr = 5;
                    st.errors.help_line[4] =
                        c"The control sequence at the end of the top line".as_ptr();
                    st.errors.help_line[3] =
                        c"of your error message was never \\def'ed. If you have".as_ptr();
                    st.errors.help_line[2] =
                        c"misspelled it (e.g., `\\hobx'), type `I' and the correct".as_ptr();
                    st.errors.help_line[1] =
                        c"spelling (e.g., `I\\hbox'). Otherwise just continue,".as_ptr();
                    st.errors.help_line[0] =
                        c"and I'll forget about whatever was undefined.".as_ptr();
                    scc_err(crate::errors::error_in(st))?;
                }
            }
        } else if st.scanner.cur_cmd < END_TEMPLATE {
            macro_call_in(st)?;
        } else {
            st.scanner.cur_tok = CS_TOKEN_FLAG + FROZEN_ENDV;
            crate::input::back_input_in(st)?;
        }
        break 'reswitch;
    }

    st.scanner.cur_val = cv_backup;
    st.scanner.cur_val_level = cvl_backup;
    st.scanner.radix = radix_backup;
    st.scanner.cur_order = co_backup;
    st.mem.set_b32s1(BACKUP_HEAD, backup_backup);
    st.scanner.expand_depth_count -= 1;
    Ok(())
}
pub(crate) fn macro_call_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    use crate::mem::TEX_NULL;
    use crate::scan::{get_avail_scc, scc_err};

    const MATCHING: u8 = 3;
    const LONG_CALL: u8 = 114;
    const OUTER_CALL: u8 = 115;
    const MACRO: u16 = 6;
    const V_TEMPLATE: u16 = 2;
    const TOKEN_LIST: u16 = 0;
    const INT_PAR_TRACING_MACROS: i32 = 30;
    const INT_PAR_TRACING_STACK_LEVELS: i32 = 55;

    let save_scanner_status = st.scanner.scanner_status;
    let save_warning_index = st.scanner.warning_index;
    st.scanner.warning_index = st.scanner.cur_cs;
    let ref_count = st.scanner.cur_chr;
    let mut r = st.mem.b32s1(ref_count);
    let mut n: i32 = 0;

    if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_MACROS) > 0 {
        crate::output::begin_diagnostic_in(st);
        crate::output::diagnostic_begin_capture_warning_here_in(st);
        let tsl = st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_STACK_LEVELS);
        if tsl > 0 {
            if st.input.input_ptr < tsl {
                let mut v = st.input.input_ptr;
                crate::output::print_ln_in(st);
                crate::output::print_char_in(st, b'~' as i32);
                while v > 0 {
                    crate::output::print_char_in(st, b'.' as i32);
                    v -= 1;
                }
                crate::output::print_cs_in(st, st.scanner.warning_index);
                crate::output::token_show_in(st, ref_count);
            } else {
                crate::output::print_char_in(st, b'~' as i32);
                crate::output::print_char_in(st, b'~' as i32);
                crate::output::print_cs_in(st, st.scanner.warning_index);
            }
        } else {
            crate::output::print_ln_in(st);
            crate::output::print_cs_in(st, st.scanner.warning_index);
            crate::output::token_show_in(st, ref_count);
        }
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        crate::output::end_diagnostic_in(st, false);
    }

    if st.mem.b32s0(r) == PROTECTED_TOKEN {
        r = st.mem.b32s1(r);
    }

    // pstack: accumulates gathered parameter token lists (max 9 params)
    let mut pstack: [i32; 9] = [TEX_NULL; 9];
    let mut match_chr: i32 = 0;
    let mut rbrace_ptr: i32 = TEX_NULL;

    'mac: {
        if st.mem.b32s0(r) != END_MATCH_TOKEN {
            st.scanner.scanner_status = MATCHING;
            let mut unbalance: i32 = 0;
            st.scanner.long_state = st.eqtb.b16s1(st.scanner.cur_cs) as u8;
            if st.scanner.long_state >= OUTER_CALL {
                st.scanner.long_state -= 2;
            }

            'outer: loop {
                st.mem.set_b32s1(TEMP_HEAD, TEX_NULL);
                // Determine whether this iteration gathers a delimited parameter
                let s: i32;
                let mut p: i32;
                let mut m: i32;
                if st.mem.b32s0(r) >= END_MATCH_TOKEN || st.mem.b32s0(r) < MATCH_TOKEN {
                    s = TEX_NULL;
                    p = TEX_NULL; // unused when s == TEX_NULL
                    m = 0;
                } else {
                    match_chr = st.mem.b32s0(r) - MATCH_TOKEN;
                    s = st.mem.b32s1(r);
                    r = s;
                    p = TEMP_HEAD;
                    m = 0;
                }

                'continue_: loop {
                    crate::scan::get_token_in(st)?;

                    if st.scanner.cur_tok == st.mem.b32s0(r) {
                        r = st.mem.b32s1(r);
                        if st.mem.b32s0(r) >= MATCH_TOKEN && st.mem.b32s0(r) <= END_MATCH_TOKEN {
                            if st.scanner.cur_tok < LEFT_BRACE_LIMIT {
                                st.scanner.align_state -= 1;
                            }
                            break 'continue_; // goto found
                        } else {
                            continue 'continue_; // goto continue_
                        }
                    }

                    if s != r {
                        if s == TEX_NULL {
                            crate::output::error_here_with_diagnostic_in(st, c"Use of ".as_ptr());
                            crate::output::sprint_cs_in(st, st.scanner.warning_index);
                            crate::output::print_cstr_in(
                                st,
                                c" doesn't match its definition".as_ptr(),
                            );
                            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                            st.errors.help_ptr = 4;
                            st.errors.help_line[3] =
                                c"If you say, e.g., `\\def\\a1{...}', then you must always"
                                    .as_ptr();
                            st.errors.help_line[2] =
                                c"put `1' after `\\a', since control sequence names are".as_ptr();
                            st.errors.help_line[1] =
                                c"made up of letters only. The macro here has not been".as_ptr();
                            st.errors.help_line[0] =
                                c"followed by the required stuff, so I'm ignoring it.".as_ptr();
                            scc_err(crate::errors::error_in(st))?;
                            break 'mac; // goto exit
                        } else {
                            let mut t = s;
                            loop {
                                let q = get_avail_scc(st)?;
                                st.mem.set_b32s1(p, q);
                                let v0 = st.mem.b32s0(t);
                                st.mem.set_b32s0(q, v0);
                                p = q;
                                m += 1;
                                let mut u = st.mem.b32s1(t);
                                let mut v = s;
                                loop {
                                    if u == r {
                                        if st.scanner.cur_tok != st.mem.b32s0(v) {
                                            break; // goto done
                                        } else {
                                            r = st.mem.b32s1(v);
                                            continue 'continue_; // goto continue_
                                        }
                                    }
                                    if st.mem.b32s0(u) != st.mem.b32s0(v) {
                                        break; // goto done
                                    }
                                    u = st.mem.b32s1(u);
                                    v = st.mem.b32s1(v);
                                }
                                // done:
                                t = st.mem.b32s1(t);
                                if t == r {
                                    break;
                                }
                            }
                            r = s;
                        }
                    }

                    if st.scanner.cur_tok == st.input.par_token && st.scanner.long_state != LONG_CALL {
                        if st.scanner.long_state == CALL {
                            crate::errors::runaway_in(st);
                            crate::output::error_here_with_diagnostic_in(
                                st,
                                c"Paragraph ended before ".as_ptr(),
                            );
                            crate::output::sprint_cs_in(st, st.scanner.warning_index);
                            crate::output::print_cstr_in(st, c" was complete".as_ptr());
                            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                            st.errors.help_ptr = 3;
                            st.errors.help_line[2] =
                                c"I suspect you've forgotten a `}', causing me to apply this"
                                    .as_ptr();
                            st.errors.help_line[1] =
                                c"control sequence to too much text. How can we recover?".as_ptr();
                            st.errors.help_line[0] =
                                c"My plan is to forget the whole thing and hope for the best."
                                    .as_ptr();
                            let oc = crate::input::back_error_in(st)?;
                            scc_err(oc)?;
                        }
                        pstack[n as usize] = st.mem.b32s1(TEMP_HEAD);
                        st.scanner.align_state -= unbalance;
                        for m2 in 0..=n {
                            crate::mem::flush_list(&mut st.mem, pstack[m2 as usize]);
                        }
                        break 'mac; // goto exit
                    }

                    if st.scanner.cur_tok < RIGHT_BRACE_LIMIT {
                        if st.scanner.cur_tok < LEFT_BRACE_LIMIT {
                            // left brace: gather a braced group
                            unbalance = 1;
                            loop {
                                let q = if st.mem.avail != TEX_NULL {
                                    let q = st.mem.avail;
                                    st.mem.avail = st.mem.b32s1(q);
                                    st.mem.set_b32s1(q, TEX_NULL);
                                    q
                                } else {
                                    get_avail_scc(st)?
                                };
                                st.mem.set_b32s1(p, q);
                                st.mem.set_b32s0(q, st.scanner.cur_tok);
                                p = q;

                                crate::scan::get_token_in(st)?;

                                if st.scanner.cur_tok == st.input.par_token
                                    && st.scanner.long_state != LONG_CALL
                                {
                                    if st.scanner.long_state == CALL {
                                        crate::errors::runaway_in(st);
                                        crate::output::error_here_with_diagnostic_in(
                                            st,
                                            c"Paragraph ended before ".as_ptr(),
                                        );
                                        crate::output::sprint_cs_in(st, st.scanner.warning_index);
                                        crate::output::print_cstr_in(st, c" was complete".as_ptr());
                                        crate::output::capture_to_diagnostic_in(
                                            st,
                                            std::ptr::null_mut(),
                                        );
                                        st.errors.help_ptr = 3;
                                        st.errors.help_line[2] = c"I suspect you've forgotten a `}', causing me to apply this".as_ptr();
                                        st.errors.help_line[1] = c"control sequence to too much text. How can we recover?".as_ptr();
                                        st.errors.help_line[0] = c"My plan is to forget the whole thing and hope for the best.".as_ptr();
                                        let oc = crate::input::back_error_in(st)?;
                                        scc_err(oc)?;
                                    }
                                    pstack[n as usize] = st.mem.b32s1(TEMP_HEAD);
                                    st.scanner.align_state -= unbalance;
                                    for m2 in 0..=n {
                                        crate::mem::flush_list(&mut st.mem, pstack[m2 as usize]);
                                    }
                                    break 'mac; // goto exit
                                }

                                if st.scanner.cur_tok < RIGHT_BRACE_LIMIT {
                                    if st.scanner.cur_tok < LEFT_BRACE_LIMIT {
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
                            let q = get_avail_scc(st)?;
                            st.mem.set_b32s1(p, q);
                            st.mem.set_b32s0(q, st.scanner.cur_tok);
                            p = q;
                        } else {
                            // right brace with no matching left brace (413)
                            crate::input::back_input_in(st)?;
                            crate::output::error_here_with_diagnostic_in(
                                st,
                                c"Argument of ".as_ptr(),
                            );
                            crate::output::sprint_cs_in(st, st.scanner.warning_index);
                            crate::output::print_cstr_in(st, c" has an extra }".as_ptr());
                            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                            st.errors.help_ptr = 6;
                            st.errors.help_line[5] =
                                c"I've run across a `}' that doesn't seem to match anything."
                                    .as_ptr();
                            st.errors.help_line[4] =
                                c"For example, `\\def\\a#1{...}' and `\\a}' would produce".as_ptr();
                            st.errors.help_line[3] =
                                c"this error. If you simply proceed now, the `\\par' that".as_ptr();
                            st.errors.help_line[2] =
                                c"I've just inserted will cause me to report a runaway".as_ptr();
                            st.errors.help_line[1] =
                                c"argument that might be the root of the problem. But if".as_ptr();
                            st.errors.help_line[0] =
                                c"your `}' was spurious, just type `2' and it will go away."
                                    .as_ptr();
                            st.scanner.align_state += 1;
                            st.scanner.long_state = CALL;
                            st.scanner.cur_tok = st.input.par_token;
                            let oc = crate::input::ins_error_in(st)?;
                            scc_err(oc)?;
                            continue 'continue_; // goto continue_
                        }
                    } else {
                        // 411: non-brace token
                        if st.scanner.cur_tok == SPACE_TOKEN
                            && st.mem.b32s0(r) <= END_MATCH_TOKEN
                            && st.mem.b32s0(r) >= MATCH_TOKEN
                        {
                            continue 'continue_; // goto continue_
                        }
                        let q = get_avail_scc(st)?;
                        st.mem.set_b32s1(p, q);
                        st.mem.set_b32s0(q, st.scanner.cur_tok);
                        p = q;
                    }

                    m += 1;
                    if st.mem.b32s0(r) > END_MATCH_TOKEN || st.mem.b32s0(r) < MATCH_TOKEN {
                        continue 'continue_; // goto continue_
                    }
                    break 'continue_; // goto found
                } // end 'continue_

                // found:
                if s != TEX_NULL {
                    if m == 1 && st.mem.b32s0(p) < RIGHT_BRACE_LIMIT {
                        // strip outer braces from single-token braced arg
                        st.mem.set_b32s1(rbrace_ptr, TEX_NULL);
                        st.mem.set_b32s1(p, st.mem.avail);
                        st.mem.avail = p;
                        p = st.mem.b32s1(TEMP_HEAD);
                        pstack[n as usize] = st.mem.b32s1(p);
                        st.mem.set_b32s1(p, st.mem.avail);
                        st.mem.avail = p;
                    } else {
                        pstack[n as usize] = st.mem.b32s1(TEMP_HEAD);
                    }

                    n += 1;

                    if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_MACROS) > 0 {
                        let tsl = st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_STACK_LEVELS);
                        if tsl == 0 || st.input.input_ptr < tsl {
                            crate::output::begin_diagnostic_in(st);
                            crate::output::diagnostic_begin_capture_warning_here_in(st);
                            crate::output::print_nl_in(st, match_chr);
                            crate::output::print_int_in(st, n);
                            crate::output::print_cstr_in(st, c"<-".as_ptr());
                            crate::output::show_token_list_in(
                                st,
                                pstack[(n - 1) as usize],
                                TEX_NULL,
                                1000,
                            );
                            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                            crate::output::end_diagnostic_in(st, false);
                        }
                    }
                }

                if st.mem.b32s0(r) == END_MATCH_TOKEN {
                    break 'outer;
                }
            } // end 'outer
        } // end if mem_b32s0(r) != END_MATCH_TOKEN

        // Flush exhausted token lists before installing macro body
        while st.input.cur_input.state == TOKEN_LIST
            && st.input.cur_input.loc == TEX_NULL
            && st.input.cur_input.index != V_TEMPLATE
        {
            crate::input::end_token_list_in(st)?;
        }

        crate::input::begin_token_list_in(st, ref_count, MACRO)?;
        st.input.cur_input.name = st.scanner.warning_index;
        st.input.cur_input.loc = st.mem.b32s1(r);

        if n > 0 {
            if st.input.param_ptr + n > st.input.max_param_stack {
                st.input.max_param_stack = st.input.param_ptr + n;
                if st.input.max_param_stack > st.input.param_size {
                    return Err(SccAbort::Input(crate::input::InputAbort::Overflow {
                        what: c"parameter stack size".as_ptr(),
                        n: st.input.param_size,
                    }));
                }
            }
            for m in 0..n {
                st.input.param_stack[(st.input.param_ptr + m) as usize] = pstack[m as usize];
            }
            st.input.param_ptr += n;
        }
    } // end 'mac (exit:)

    st.scanner.scanner_status = save_scanner_status;
    st.scanner.warning_index = save_warning_index;
    Ok(())
}
