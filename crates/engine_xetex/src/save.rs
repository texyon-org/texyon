//! Save-stack management functions translated from `xetex/xetex-xetex0.c`.
//!
//! Phase A: all state accessed through the C globals via `src/globals.rs`.

use crate::globals::{
    align_state, cur_boundary, cur_group, cur_input, cur_level, cur_tok, eqtb_b16s0, eqtb_b16s1,
    eqtb_b32s1, eqtb_word, grp_stack, in_open, line, mag_set, max_save_stack, mem_b16s0,
    mem_b16s1, mem_b32s0, mem_b32s1, sa_chain, sa_level, sa_root, save_ptr, save_size,
    save_stack_b16s0, save_stack_b16s1, save_stack_b32s1, save_stack_word, set_align_state, set_cur_boundary,
    set_cur_group, set_cur_input_loc, set_cur_input_start, set_cur_level, set_cur_tok,
    set_eqtb_b16s0, set_eqtb_b16s1, set_eqtb_b32s1, set_eqtb_word, set_mag_set,
    set_max_save_stack, set_mem_b16s0, set_mem_b16s1, set_mem_b32s0, set_mem_b32s1, set_sa_chain,
    set_sa_level, set_sa_root, set_save_ptr, set_save_stack_b16s0, set_save_stack_b16s1,
    set_save_stack_b32s1, set_save_stack_word, set_xeq_level, xeq_level,
};
use crate::mem_types::MemoryWord;

const TEX_NULL: i32 = -0x0FFF_FFFF; // MIN_HALFWORD

extern "C" {
    fn error_here_with_diagnostic(msg: *const libc::c_char) -> *mut libc::c_void;
    fn capture_to_diagnostic(diag: *mut libc::c_void);
    fn print_int(n: i32);
    fn print_cstr(s: *const libc::c_char);
    fn print_nl_cstr(s: *const libc::c_char);
    fn ttstub_diag_printf(d: *mut libc::c_void, fmt: *const libc::c_char, ...) -> libc::c_int;
    fn flush_node_list(p: i32);
    fn back_input();
}

// Save-stack discriminant values (xetex-constants.h:260–285).
const RESTORE_OLD_VALUE: u16 = 0;
const RESTORE_ZERO: u16 = 1;
const INSERT_TOKEN: u16 = 2;
const LEVEL_BOUNDARY: u16 = 3;

// Grouping level constants (xetex-constants.h:89–90).
const LEVEL_ZERO: u16 = 0;
const LEVEL_ONE: u16 = 1;

// Brace-token limits for align_state tracking (xetex-constants.h:350,352).
const LEFT_BRACE_LIMIT: i32 = 0x40_0000;
const RIGHT_BRACE_LIMIT: i32 = 0x60_0000;

// eqtb range constants (xetex_format.h:198,204,224).
const INT_BASE: i32 = 7_826_729;
const EQTB_SIZE: i32 = 8_941_458;
const UNDEFINED_CONTROL_SEQUENCE: i32 = 2_254_339;

// ---------------------------------------------------------------------------
// new_save_level
// ---------------------------------------------------------------------------

/// Push a new grouping level onto the save stack.
///
/// translated from xetex-xetex0.c:3932 (new_save_level)
#[no_mangle]
pub unsafe extern "C" fn new_save_level(c: u8) {
    if save_ptr() > max_save_stack() {
        set_max_save_stack(save_ptr());
        if max_save_stack() > save_size() - 7 {
            crate::errors::overflow(c"semantic nest size".as_ptr(), save_size());
        }
    }
    set_save_stack_b32s1(save_ptr(), line());
    set_save_ptr(save_ptr() + 1);
    set_save_stack_b16s1(save_ptr(), LEVEL_BOUNDARY);
    set_save_stack_b16s0(save_ptr(), cur_group() as u16);
    set_save_stack_b32s1(save_ptr(), cur_boundary());
    if cur_level() == u16::MAX {
        crate::errors::overflow(c"grouping levels".as_ptr(), u16::MAX as i32);
    }
    set_cur_boundary(save_ptr());
    set_cur_group(c);
    set_cur_level(cur_level() + 1);
    set_save_ptr(save_ptr() + 1);
}

// ---------------------------------------------------------------------------
// eq_save
// ---------------------------------------------------------------------------

/// Save the current value of `eqtb[p]` at level `l` on the save stack.
///
/// translated from xetex-xetex0.c:3914 (eq_save)
#[no_mangle]
pub unsafe extern "C" fn eq_save(p: i32, l: u16) {
    if save_ptr() > max_save_stack() {
        set_max_save_stack(save_ptr());
        if max_save_stack() > save_size() - 7 {
            crate::errors::overflow(c"save size".as_ptr(), save_size());
        }
    }
    if l == LEVEL_ZERO {
        set_save_stack_b16s1(save_ptr(), RESTORE_ZERO);
    } else {
        set_save_stack_word(save_ptr(), eqtb_word(p));
        set_save_ptr(save_ptr() + 1);
        set_save_stack_b16s1(save_ptr(), RESTORE_OLD_VALUE);
    }
    set_save_stack_b16s0(save_ptr(), l);
    set_save_stack_b32s1(save_ptr(), p);
    set_save_ptr(save_ptr() + 1);
}

// ---------------------------------------------------------------------------
// eq_word_define
// ---------------------------------------------------------------------------

/// Assign `eqtb[p].b32.s1 = w`, saving the old value if necessary.
///
/// translated from xetex-xetex0.c:3954 (eq_word_define)
#[no_mangle]
pub unsafe extern "C" fn eq_word_define(p: i32, w: i32) {
    if eqtb_b32s1(p) == w {
        return;
    }
    if xeq_level(p) != cur_level() {
        eq_save(p, xeq_level(p));
        set_xeq_level(p, cur_level());
    }
    set_eqtb_b32s1(p, w);
}

// ---------------------------------------------------------------------------
// geq_word_define
// ---------------------------------------------------------------------------

/// Globally assign `eqtb[p].b32.s1 = w` at LEVEL_ONE.
///
/// translated from xetex-xetex0.c:3977 (geq_word_define)
#[no_mangle]
pub unsafe extern "C" fn geq_word_define(p: i32, w: i32) {
    set_eqtb_b32s1(p, w);
    set_xeq_level(p, LEVEL_ONE);
}

// ---------------------------------------------------------------------------
// save_for_after
// ---------------------------------------------------------------------------

/// Push token `t` onto the save stack for deferred processing after `}`.
///
/// translated from xetex-xetex0.c:3984 (save_for_after)
#[no_mangle]
pub unsafe extern "C" fn save_for_after(t: i32) {
    if cur_level() > LEVEL_ONE {
        if save_ptr() > max_save_stack() {
            set_max_save_stack(save_ptr());
            if max_save_stack() > save_size() - 7 {
                crate::errors::overflow(c"save size".as_ptr(), save_size());
            }
        }
        set_save_stack_b16s1(save_ptr(), INSERT_TOKEN);
        set_save_stack_b16s0(save_ptr(), LEVEL_ZERO);
        set_save_stack_b32s1(save_ptr(), t);
        set_save_ptr(save_ptr() + 1);
    }
}

// ---------------------------------------------------------------------------
// prepare_mag
// ---------------------------------------------------------------------------

// INT_PAR__mag = 17 (xetex_format.h:52).
const INT_PAR_MAG: i32 = 17;

/// Validate and lock in the current \mag parameter.
///
/// translated from xetex-xetex0.c:3997 (prepare_mag)
#[no_mangle]
pub unsafe extern "C" fn prepare_mag() {
    let mag = eqtb_b32s1(INT_BASE + INT_PAR_MAG);
    if mag_set() > 0 && mag != mag_set() {
        let errmsg = error_here_with_diagnostic(c"Incompatible magnification (".as_ptr());
        print_int(mag);
        print_cstr(c");".as_ptr());
        print_nl_cstr(c" the previous value will be retained".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), mag_set());
        capture_to_diagnostic(std::ptr::null_mut());
        crate::globals::set_help_ptr(2);
        crate::globals::set_help_line(1, c"I can handle only one magnification ratio per job. So I've".as_ptr());
        crate::globals::set_help_line(0, c"reverted to the magnification you used earlier on this run.".as_ptr());
        crate::errors::int_error(mag_set());
        geq_word_define(INT_BASE + INT_PAR_MAG, mag_set());
    }
    let mag2 = eqtb_b32s1(INT_BASE + INT_PAR_MAG);
    if mag2 <= 0 || mag2 > 32768 {
        let errmsg = error_here_with_diagnostic(c"Illegal magnification has been changed to 1000".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), mag2);
        capture_to_diagnostic(std::ptr::null_mut());
        crate::globals::set_help_ptr(1);
        crate::globals::set_help_line(0, c"The magnification ratio must be between 1 and 32768.".as_ptr());
        crate::errors::int_error(mag2);
        geq_word_define(INT_BASE + INT_PAR_MAG, 1000);
    }
    set_mag_set(eqtb_b32s1(INT_BASE + INT_PAR_MAG));
}

// ---------------------------------------------------------------------------
// eq_destroy / eq_define / geq_define
// ---------------------------------------------------------------------------

/// Free any heap resource held by an eqtb entry (token list, glue, box, or sa-ref).
///
/// translated from xetex-xetex0.c:3831 (eq_destroy)
#[no_mangle]
pub unsafe extern "C" fn eq_destroy(w: MemoryWord) {
    let typ = w.b16.s1;
    let val = w.b32.s1;
    match typ {
        113 | 114 | 115 | 116 => crate::mem::delete_token_ref(val),
        119 => crate::mem::delete_glue_ref(val),
        120 => {
            if val != TEX_NULL {
                crate::mem::free_node(val, mem_b32s0(val) + mem_b32s0(val) + 1);
            }
        }
        121 => flush_node_list(val),
        72 | 91 => {
            if val < 0 || val > 19 {
                delete_sa_ref(val);
            }
        }
        _ => {}
    }
}

/// Assign `eqtb[p] = (level=cur_level, type=t, value=e)`, saving if needed.
///
/// translated from xetex-xetex0.c:3867 (eq_define)
#[no_mangle]
pub unsafe extern "C" fn eq_define(p: i32, t: u16, e: i32) {
    if eqtb_b16s1(p) == t && eqtb_b32s1(p) == e {
        eq_destroy(eqtb_word(p));
        return;
    }
    if eqtb_b16s0(p) == cur_level() {
        eq_destroy(eqtb_word(p));
    } else if cur_level() > LEVEL_ONE {
        eq_save(p, eqtb_b16s0(p));
    }
    set_eqtb_b16s0(p, cur_level());
    set_eqtb_b16s1(p, t);
    set_eqtb_b32s1(p, e);
}

/// Globally assign `eqtb[p] = (level=LEVEL_ONE, type=t, value=e)`.
///
/// translated from xetex-xetex0.c:3886 (geq_define)
#[no_mangle]
pub unsafe extern "C" fn geq_define(p: i32, t: u16, e: i32) {
    eq_destroy(eqtb_word(p));
    set_eqtb_b16s0(p, LEVEL_ONE);
    set_eqtb_b16s1(p, t);
    set_eqtb_b32s1(p, e);
}

// ---------------------------------------------------------------------------
// Sparse-array (SA) register save/restore cluster
// ---------------------------------------------------------------------------

// Node-size constants (xetex-constants.h:109,120,134).
const POINTER_NODE_SIZE: i32 = 2;
const WORD_NODE_SIZE: i32 = 3;
const INDEX_NODE_SIZE: i32 = 33;

// SA register type limits (xetex-constants.h:326–331).
const DIMEN_VAL_LIMIT: u16 = 128;
const MU_VAL_LIMIT: u16 = 256;
const BOX_VAL_LIMIT: u16 = 320;
const TOK_VAL_LIMIT: u16 = 384;

// Save-stack discriminant for SA save/restore (xetex-constants.h:291).
const RESTORE_SA: u16 = 4;

// translated from xetex-xetex0.c:2789 (delete_sa_ref)
#[no_mangle]
pub unsafe extern "C" fn delete_sa_ref(q: i32) {
    set_mem_b32s0(q + 1, mem_b32s0(q + 1) - 1);
    if mem_b32s0(q + 1) != TEX_NULL {
        return;
    }
    let mut s: i32;
    if mem_b16s1(q) < DIMEN_VAL_LIMIT {
        if mem_b32s1(q + 2) == 0 {
            s = WORD_NODE_SIZE;
        } else {
            return;
        }
    } else {
        if mem_b16s1(q) < MU_VAL_LIMIT {
            if mem_b32s1(q + 1) == 0 {
                crate::mem::delete_glue_ref(0);
            } else {
                return;
            }
        } else if mem_b32s1(q + 1) != TEX_NULL {
            return;
        }
        s = POINTER_NODE_SIZE;
    }
    let mut q = q;
    loop {
        let i = (mem_b16s1(q) % 64) as i32;
        let p = q;
        q = mem_b32s1(p);
        crate::mem::free_node(p, s);
        if q == TEX_NULL {
            set_sa_root(i as usize, TEX_NULL);
            return;
        }
        if (i & 1) != 0 {
            set_mem_b32s1(q + i / 2 + 1, TEX_NULL);
        } else {
            set_mem_b32s0(q + i / 2 + 1, TEX_NULL);
        }
        set_mem_b16s0(q, mem_b16s0(q) - 1);
        s = INDEX_NODE_SIZE;
        if mem_b16s0(q) > 0 {
            break;
        }
    }
}

// translated from xetex-xetex0.c:2837 (sa_save)
#[no_mangle]
pub unsafe extern "C" fn sa_save(p: i32) {
    if cur_level() != sa_level() {
        if save_ptr() > max_save_stack() {
            set_max_save_stack(save_ptr());
            if max_save_stack() > save_size() - 7 {
                crate::errors::overflow(c"save size".as_ptr(), save_size());
            }
        }
        set_save_stack_b16s1(save_ptr(), RESTORE_SA);
        set_save_stack_b16s0(save_ptr(), sa_level());
        set_save_stack_b32s1(save_ptr(), sa_chain());
        set_save_ptr(save_ptr() + 1);
        set_sa_chain(TEX_NULL);
        set_sa_level(cur_level());
    }
    let mut i = mem_b16s1(p);
    let q: i32;
    if i < DIMEN_VAL_LIMIT {
        if mem_b32s1(p + 2) == 0 {
            q = crate::mem::get_node(POINTER_NODE_SIZE);
            i = TOK_VAL_LIMIT;
        } else {
            q = crate::mem::get_node(WORD_NODE_SIZE);
            set_mem_b32s1(q + 2, mem_b32s1(p + 2));
        }
        set_mem_b32s1(q + 1, TEX_NULL);
    } else {
        q = crate::mem::get_node(POINTER_NODE_SIZE);
        set_mem_b32s1(q + 1, mem_b32s1(p + 1));
    }
    set_mem_b32s0(q + 1, p);
    set_mem_b16s1(q, i);
    set_mem_b16s0(q, mem_b16s0(p));
    set_mem_b32s1(q, sa_chain());
    set_sa_chain(q);
    set_mem_b32s0(p + 1, mem_b32s0(p + 1) + 1);
}

// translated from xetex-xetex0.c:2878 (sa_destroy)
#[no_mangle]
pub unsafe extern "C" fn sa_destroy(p: i32) {
    if mem_b16s1(p) < MU_VAL_LIMIT {
        crate::mem::delete_glue_ref(mem_b32s1(p + 1));
    } else if mem_b32s1(p + 1) != TEX_NULL {
        if mem_b16s1(p) < BOX_VAL_LIMIT {
            flush_node_list(mem_b32s1(p + 1));
        } else {
            crate::mem::delete_token_ref(mem_b32s1(p + 1));
        }
    }
}

// translated from xetex-xetex0.c:2891 (sa_def)
#[no_mangle]
pub unsafe extern "C" fn sa_def(p: i32, e: i32) {
    set_mem_b32s0(p + 1, mem_b32s0(p + 1) + 1);
    if mem_b32s1(p + 1) == e {
        sa_destroy(p);
    } else {
        if mem_b16s0(p) == cur_level() {
            sa_destroy(p);
        } else {
            sa_save(p);
        }
        set_mem_b16s0(p, cur_level());
        set_mem_b32s1(p + 1, e);
    }
    delete_sa_ref(p);
}

// translated from xetex-xetex0.c:2908 (sa_w_def)
#[no_mangle]
pub unsafe extern "C" fn sa_w_def(p: i32, w: i32) {
    set_mem_b32s0(p + 1, mem_b32s0(p + 1) + 1);
    if mem_b32s1(p + 2) != w {
        if mem_b16s0(p) != cur_level() {
            sa_save(p);
        }
        set_mem_b16s0(p, cur_level());
        set_mem_b32s1(p + 2, w);
    }
    delete_sa_ref(p);
}

// translated from xetex-xetex0.c:2923 (gsa_def)
#[no_mangle]
pub unsafe extern "C" fn gsa_def(p: i32, e: i32) {
    set_mem_b32s0(p + 1, mem_b32s0(p + 1) + 1);
    sa_destroy(p);
    set_mem_b16s0(p, LEVEL_ONE);
    set_mem_b32s1(p + 1, e);
    delete_sa_ref(p);
}

// translated from xetex-xetex0.c:2933 (gsa_w_def)
#[no_mangle]
pub unsafe extern "C" fn gsa_w_def(p: i32, w: i32) {
    set_mem_b32s0(p + 1, mem_b32s0(p + 1) + 1);
    set_mem_b16s0(p, LEVEL_ONE);
    set_mem_b32s1(p + 2, w);
    delete_sa_ref(p);
}

// translated from xetex-xetex0.c:2942 (sa_restore)
#[no_mangle]
pub unsafe extern "C" fn sa_restore() {
    loop {
        let p = mem_b32s0(sa_chain() + 1);
        if mem_b16s0(p) == LEVEL_ONE {
            if mem_b16s1(p) >= DIMEN_VAL_LIMIT {
                sa_destroy(sa_chain());
            }
        } else {
            if mem_b16s1(p) < DIMEN_VAL_LIMIT {
                if mem_b16s1(sa_chain()) < DIMEN_VAL_LIMIT {
                    set_mem_b32s1(p + 2, mem_b32s1(sa_chain() + 2));
                } else {
                    set_mem_b32s1(p + 2, 0);
                }
            } else {
                sa_destroy(p);
                set_mem_b32s1(p + 1, mem_b32s1(sa_chain() + 1));
            }
            set_mem_b16s0(p, mem_b16s0(sa_chain()));
        }
        delete_sa_ref(p);
        let sc = sa_chain();
        set_sa_chain(mem_b32s1(sc));
        if mem_b16s1(sc) < DIMEN_VAL_LIMIT {
            crate::mem::free_node(sc, WORD_NODE_SIZE);
        } else {
            crate::mem::free_node(sc, POINTER_NODE_SIZE);
        }
        if sa_chain() == TEX_NULL {
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// unsave
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:2814 (unsave)
#[no_mangle]
pub unsafe extern "C" fn unsave() {
    if cur_level() > LEVEL_ONE {
        set_cur_level(cur_level() - 1);
        let mut l: u16 = 0;
        let mut a = false;
        loop {
            set_save_ptr(save_ptr() - 1);
            if save_stack_b16s1(save_ptr()) == LEVEL_BOUNDARY {
                break; // goto done
            }
            let p = save_stack_b32s1(save_ptr());
            if save_stack_b16s1(save_ptr()) == INSERT_TOKEN {
                let t = cur_tok();
                set_cur_tok(p);
                if a {
                    let pn = crate::mem::get_avail();
                    set_mem_b32s0(pn, cur_tok());
                    set_mem_b32s1(pn, cur_input().loc);
                    set_cur_input_loc(pn);
                    set_cur_input_start(pn);
                    if cur_tok() < RIGHT_BRACE_LIMIT {
                        if cur_tok() < LEFT_BRACE_LIMIT {
                            set_align_state(align_state() - 1);
                        } else {
                            set_align_state(align_state() + 1);
                        }
                    }
                } else {
                    back_input();
                    a = true;
                }
                set_cur_tok(t);
            } else if save_stack_b16s1(save_ptr()) == RESTORE_SA {
                sa_restore();
                set_sa_chain(p);
                set_sa_level(save_stack_b16s0(save_ptr()));
            } else {
                if save_stack_b16s1(save_ptr()) == RESTORE_OLD_VALUE {
                    l = save_stack_b16s0(save_ptr());
                    set_save_ptr(save_ptr() - 1);
                } else {
                    set_save_stack_word(save_ptr(), eqtb_word(UNDEFINED_CONTROL_SEQUENCE));
                }
                if p < INT_BASE || p > EQTB_SIZE {
                    if eqtb_b16s0(p) == LEVEL_ONE {
                        eq_destroy(save_stack_word(save_ptr()));
                    } else {
                        eq_destroy(eqtb_word(p));
                        set_eqtb_word(p, save_stack_word(save_ptr()));
                    }
                } else if xeq_level(p) != LEVEL_ONE {
                    set_eqtb_word(p, save_stack_word(save_ptr()));
                    set_xeq_level(p, l);
                }
            }
        }
        // done:
        if grp_stack(in_open()) == cur_boundary() {
            crate::output::group_warning();
        }
        set_cur_group(save_stack_b16s0(save_ptr()) as u8);
        set_cur_boundary(save_stack_b32s1(save_ptr()));
        set_save_ptr(save_ptr() - 1);
    } else {
        crate::errors::confusion(c"curlevel".as_ptr());
    }
}
