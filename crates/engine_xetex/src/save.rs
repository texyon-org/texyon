//! Save-stack management functions translated from `xetex/xetex-xetex0.c`.
//!
//! Phase A: all state accessed through the C globals via `src/globals.rs`.
use crate::consts::{EQTB_SIZE, GLUE_SPEC_SIZE, INT_BASE, TEX_NULL};
use crate::output::capture_to_diagnostic;
use crate::output::error_here_with_diagnostic;
use crate::output::print_cstr;
use crate::output::print_int;
use crate::output::print_nl_cstr;
use crate::scan::flush_node_list;
use crate::state::{MemState, SaveStackState, ScannerState};

use crate::globals::{
    align_state, cur_boundary, cur_group, cur_input, cur_level, cur_ptr, cur_tok, eqtb_b16s0,
    eqtb_b16s1, eqtb_b32s1, eqtb_word, grp_stack, in_open, line, mag_set, max_save_stack,
    mem_b16s0, mem_b16s1, mem_b32s0, mem_b32s1, sa_chain, save_ptr, save_size, save_stack_b16s0,
    save_stack_b16s1, save_stack_b32s1, save_stack_word, set_align_state, set_cur_boundary,
    set_cur_group, set_cur_input_loc, set_cur_input_start, set_cur_level, set_cur_ptr, set_cur_tok,
    set_eqtb_b16s0, set_eqtb_b16s1, set_eqtb_b32s1, set_eqtb_word, set_mag_set, set_max_save_stack,
    set_mem_b16s0, set_mem_b32s0, set_mem_b32s1, set_sa_chain, set_sa_level, set_save_ptr,
    set_save_stack_b16s0, set_save_stack_b16s1, set_save_stack_b32s1, set_save_stack_word,
    set_xeq_level, xeq_level,
};
use crate::mem_types::MemoryWord;

extern "C" {

    fn ttstub_diag_printf(d: *mut libc::c_void, fmt: *const libc::c_char, ...) -> libc::c_int;

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
const UNDEFINED_CONTROL_SEQUENCE: i32 = 2_254_339;

// ---------------------------------------------------------------------------
// new_save_level
// ---------------------------------------------------------------------------

/// Push a new grouping level onto the save stack.
///
/// translated from xetex-xetex0.c:3932 (new_save_level)
pub unsafe fn new_save_level(c: u8) {
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
pub unsafe fn geq_word_define(p: i32, w: i32) {
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
        crate::globals::set_help_line(
            1,
            c"I can handle only one magnification ratio per job. So I've".as_ptr(),
        );
        crate::globals::set_help_line(
            0,
            c"reverted to the magnification you used earlier on this run.".as_ptr(),
        );
        crate::errors::int_error(mag_set());
        geq_word_define(INT_BASE + INT_PAR_MAG, mag_set());
    }
    let mag2 = eqtb_b32s1(INT_BASE + INT_PAR_MAG);
    if mag2 <= 0 || mag2 > 32768 {
        let errmsg =
            error_here_with_diagnostic(c"Illegal magnification has been changed to 1000".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), mag2);
        capture_to_diagnostic(std::ptr::null_mut());
        crate::globals::set_help_ptr(1);
        crate::globals::set_help_line(
            0,
            c"The magnification ratio must be between 1 and 32768.".as_ptr(),
        );
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
        113..=116 => crate::mem::delete_token_ref(val),
        119 => crate::mem::delete_glue_ref(val),
        120 => {
            if val != TEX_NULL {
                crate::mem::free_node_global(val, mem_b32s0(val) + mem_b32s0(val) + 1);
            }
        }
        121 => flush_node_list(val),
        72 | 91 => {
            if !(0..=19).contains(&val) {
                delete_sa_ref_global(val);
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

// Node-size constants (xetex-constants.h:109,120,123,134).
const POINTER_NODE_SIZE: i32 = 2;
const WORD_NODE_SIZE: i32 = 3;
const MARK_CLASS_NODE_SIZE: i32 = 4;
const INDEX_NODE_SIZE: i32 = 33;
const MEM_TOP: i32 = 4_999_999;

// SA register type values (xetex_format.h:568–570; xetex-constants.h:288,300).
const DIMEN_VAL: u16 = 1;
const MU_VAL: u16 = 3;
const MARK_VAL: u16 = 7;

// SA register type limits (xetex-constants.h:326–331).
const DIMEN_VAL_LIMIT: u16 = 128;
const MU_VAL_LIMIT: u16 = 256;
const BOX_VAL_LIMIT: u16 = 320;
const TOK_VAL_LIMIT: u16 = 384;

// Save-stack discriminant for SA save/restore (xetex-constants.h:291).
const RESTORE_SA: u16 = 4;
const SAVE_OVERFLOW: i32 = i32::MIN + 1;

fn delete_glue_ref_threaded(mem: &mut MemState, p: i32) {
    if mem.b32s1(p) == TEX_NULL {
        crate::mem::free_node(mem, p, GLUE_SPEC_SIZE);
    } else {
        mem.set_b32s1(p, mem.b32s1(p) - 1);
    }
}

// translated from xetex-xetex0.c:2789 (delete_sa_ref)
pub fn delete_sa_ref(mem: &mut MemState, scanner: &mut ScannerState, q: i32) {
    mem.set_b32s0(q + 1, mem.b32s0(q + 1) - 1);
    if mem.b32s0(q + 1) != TEX_NULL {
        return;
    }
    let mut s: i32;
    if mem.b16s1(q) < DIMEN_VAL_LIMIT {
        if mem.b32s1(q + 2) == 0 {
            s = WORD_NODE_SIZE;
        } else {
            return;
        }
    } else {
        if mem.b16s1(q) < MU_VAL_LIMIT {
            if mem.b32s1(q + 1) == 0 {
                delete_glue_ref_threaded(mem, 0);
            } else {
                return;
            }
        } else if mem.b32s1(q + 1) != TEX_NULL {
            return;
        }
        s = POINTER_NODE_SIZE;
    }
    let mut q = q;
    loop {
        let i = (mem.b16s1(q) % 64) as i32;
        let p = q;
        q = mem.b32s1(p);
        crate::mem::free_node(mem, p, s);
        if q == TEX_NULL {
            scanner.sa_root[i as usize] = TEX_NULL;
            return;
        }
        if (i & 1) != 0 {
            mem.set_b32s1(q + i / 2 + 1, TEX_NULL);
        } else {
            mem.set_b32s0(q + i / 2 + 1, TEX_NULL);
        }
        mem.set_b16s0(q, mem.b16s0(q) - 1);
        s = INDEX_NODE_SIZE;
        if mem.b16s0(q) > 0 {
            break;
        }
    }
}

pub unsafe fn delete_sa_ref_global(q: i32) {
    crate::state::with_state_mut(|st| delete_sa_ref(&mut st.mem, &mut st.scanner, q));
}

// translated from xetex-xetex0.c:2837 (sa_save)
pub fn sa_save(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    save_stack: &mut SaveStackState,
    p: i32,
) -> i32 {
    if save_stack.cur_level != scanner.sa_level {
        if save_stack.save_ptr > save_stack.max_save_stack {
            save_stack.max_save_stack = save_stack.save_ptr;
            if save_stack.max_save_stack > save_size() - 7 {
                return SAVE_OVERFLOW;
            }
        }
        save_stack.data[save_stack.save_ptr as usize].b16.s1 = RESTORE_SA;
        save_stack.data[save_stack.save_ptr as usize].b16.s0 = scanner.sa_level;
        save_stack.data[save_stack.save_ptr as usize].b32.s1 = scanner.sa_chain;
        save_stack.save_ptr += 1;
        scanner.sa_chain = TEX_NULL;
        scanner.sa_level = save_stack.cur_level;
    }
    let mut i = mem.b16s1(p);
    let q: i32;
    if i < DIMEN_VAL_LIMIT {
        if mem.b32s1(p + 2) == 0 {
            q = crate::mem::get_node(mem, POINTER_NODE_SIZE);
            if q == crate::mem::ALLOC_FAILED {
                return q;
            }
            i = TOK_VAL_LIMIT;
        } else {
            q = crate::mem::get_node(mem, WORD_NODE_SIZE);
            if q == crate::mem::ALLOC_FAILED {
                return q;
            }
            mem.set_b32s1(q + 2, mem.b32s1(p + 2));
        }
        mem.set_b32s1(q + 1, TEX_NULL);
    } else {
        q = crate::mem::get_node(mem, POINTER_NODE_SIZE);
        if q == crate::mem::ALLOC_FAILED {
            return q;
        }
        mem.set_b32s1(q + 1, mem.b32s1(p + 1));
    }
    mem.set_b32s0(q + 1, p);
    mem.set_b16s1(q, i);
    mem.set_b16s0(q, mem.b16s0(p));
    mem.set_b32s1(q, scanner.sa_chain);
    scanner.sa_chain = q;
    mem.set_b32s0(p + 1, mem.b32s0(p + 1) + 1);
    0
}

pub unsafe fn sa_save_global(p: i32) {
    let r = crate::state::with_state_mut(|st| {
        sa_save(&mut st.mem, &mut st.scanner, &mut st.save_stack, p)
    });
    if r == crate::mem::ALLOC_FAILED {
        crate::errors::overflow(c"main memory size".as_ptr(), MEM_TOP + 1);
    }
    if r == SAVE_OVERFLOW {
        crate::errors::overflow(c"save size".as_ptr(), save_size());
    }
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
            sa_save_global(p);
        }
        set_mem_b16s0(p, cur_level());
        set_mem_b32s1(p + 1, e);
    }
    delete_sa_ref_global(p);
}

// translated from xetex-xetex0.c:2908 (sa_w_def)
#[no_mangle]
pub unsafe extern "C" fn sa_w_def(p: i32, w: i32) {
    set_mem_b32s0(p + 1, mem_b32s0(p + 1) + 1);
    if mem_b32s1(p + 2) != w {
        if mem_b16s0(p) != cur_level() {
            sa_save_global(p);
        }
        set_mem_b16s0(p, cur_level());
        set_mem_b32s1(p + 2, w);
    }
    delete_sa_ref_global(p);
}

// translated from xetex-xetex0.c:2923 (gsa_def)
#[no_mangle]
pub unsafe extern "C" fn gsa_def(p: i32, e: i32) {
    set_mem_b32s0(p + 1, mem_b32s0(p + 1) + 1);
    sa_destroy(p);
    set_mem_b16s0(p, LEVEL_ONE);
    set_mem_b32s1(p + 1, e);
    delete_sa_ref_global(p);
}

// translated from xetex-xetex0.c:2933 (gsa_w_def)
#[no_mangle]
pub unsafe extern "C" fn gsa_w_def(p: i32, w: i32) {
    set_mem_b32s0(p + 1, mem_b32s0(p + 1) + 1);
    set_mem_b16s0(p, LEVEL_ONE);
    set_mem_b32s1(p + 2, w);
    delete_sa_ref_global(p);
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
        delete_sa_ref_global(p);
        let sc = sa_chain();
        set_sa_chain(mem_b32s1(sc));
        if mem_b16s1(sc) < DIMEN_VAL_LIMIT {
            crate::mem::free_node_global(sc, WORD_NODE_SIZE);
        } else {
            crate::mem::free_node_global(sc, POINTER_NODE_SIZE);
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
                    let pn = crate::mem::get_avail_global();
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
                    crate::input::back_input();
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
                if !(INT_BASE..=EQTB_SIZE).contains(&p) {
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

// ---------------------------------------------------------------------------
// new_index / find_sa_element
// ---------------------------------------------------------------------------

/// Allocate a new INDEX_NODE_SIZE node for the sparse-array trie.
///
/// translated from xetex-xetex0.c:4111 (new_index)
fn new_index(mem: &mut MemState, scanner: &ScannerState, i: u16, q: i32) -> i32 {
    let p = crate::mem::get_node(mem, INDEX_NODE_SIZE);
    if p == crate::mem::ALLOC_FAILED {
        return p;
    }
    set_cur_ptr(p);
    mem.set_b16s1(p, i);
    mem.set_b16s0(p, 0);
    mem.set_b32s1(p, q);
    let null = scanner.sa_null;
    for k in 1..INDEX_NODE_SIZE {
        mem.set_word(p + k, null);
    }
    p
}

/// Helper: read the child pointer at trie index `i` within node `q`.
/// Odd `i` → b32.s1; even `i` → b32.s0.
#[inline]
fn sa_get_child(mem: &MemState, q: i32, i: i32) -> i32 {
    if i & 1 != 0 {
        mem.b32s1(q + i / 2 + 1)
    } else {
        mem.b32s0(q + i / 2 + 1)
    }
}

/// Helper: write the child pointer and increment the ref-count in node `q`.
#[inline]
fn sa_set_child(mem: &mut MemState, q: i32, i: i32, p: i32) {
    if i & 1 != 0 {
        mem.set_b32s1(q + i / 2 + 1, p);
    } else {
        mem.set_b32s0(q + i / 2 + 1, p);
    }
    mem.set_b16s0(q, mem.b16s0(q) + 1);
}

/// Allocate a chain of index nodes starting at (`q`, `i`) then continuing
/// through `rest`, finishing with a data leaf node for type `t` / register `n`.
/// Corresponds to C's "fall-through" chain lab46 → lab47 → lab48 → lab49.
fn sa_alloc_chain(
    mem: &mut MemState,
    scanner: &ScannerState,
    t: u16,
    n: i32,
    mut q: i32,
    levels: &[i32],
) -> i32 {
    // Allocate intermediate index nodes for all but the last level.
    for &lv_i in &levels[..levels.len() - 1] {
        let p = new_index(mem, scanner, lv_i as u16, q);
        if p == crate::mem::ALLOC_FAILED {
            return p;
        }
        sa_set_child(mem, q, lv_i, p);
        q = cur_ptr();
    }
    // Allocate the data (leaf) node (lab49).
    let leaf_i = levels[levels.len() - 1];
    let p;
    if t == MARK_VAL {
        p = crate::mem::get_node(mem, MARK_CLASS_NODE_SIZE);
        if p == crate::mem::ALLOC_FAILED {
            return p;
        }
        set_cur_ptr(p);
        mem.set_word(p + 1, scanner.sa_null);
        mem.set_word(p + 2, scanner.sa_null);
        mem.set_word(p + 3, scanner.sa_null);
    } else if t <= DIMEN_VAL {
        p = crate::mem::get_node(mem, WORD_NODE_SIZE);
        if p == crate::mem::ALLOC_FAILED {
            return p;
        }
        set_cur_ptr(p);
        mem.set_b32s1(p + 2, 0);
        mem.set_b32s1(p + 1, n);
    } else {
        p = crate::mem::get_node(mem, POINTER_NODE_SIZE);
        if p == crate::mem::ALLOC_FAILED {
            return p;
        }
        set_cur_ptr(p);
        if t <= MU_VAL {
            mem.set_b32s1(p + 1, 0);
            // GLUE_SPEC_ref_count(0)++ : mem[0].b32.s1
            mem.set_b32s1(0, mem.b32s1(0) + 1);
        } else {
            mem.set_b32s1(p + 1, TEX_NULL);
        }
        mem.set_b32s0(p + 1, TEX_NULL);
    }
    mem.set_b16s1(p, (64 * t as i32 + leaf_i) as u16);
    mem.set_b16s0(p, 1); // level_one
    mem.set_b32s1(p, q);
    sa_set_child(mem, q, leaf_i, p);
    p
}

/// Navigate (and optionally create) a path in the sparse-array trie.
///
/// translated from xetex-xetex0.c:4129 (find_sa_element)
pub fn find_sa_element(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    t: u16,
    n: i32,
    w: bool,
) -> i32 {
    // Trie-level indices: break n into four 6-bit digits (roughly).
    let lvl = [n / 0x40000, (n / 4096) % 64, (n / 64) % 64, n % 64];

    // Traversal: navigate the 4-level trie, recording parents for linking.
    set_cur_ptr(scanner.sa_root[t as usize]);

    // Check root level (not_found in C).
    if cur_ptr() == TEX_NULL {
        if !w {
            return 0;
        }
        let p = new_index(mem, scanner, t, TEX_NULL);
        if p == crate::mem::ALLOC_FAILED {
            return p;
        }
        scanner.sa_root[t as usize] = cur_ptr();
        // lab46..lab49: allocate all 4 levels from this root.
        let p = sa_alloc_chain(mem, scanner, t, n, cur_ptr(), &lvl);
        return if p == crate::mem::ALLOC_FAILED { p } else { 0 };
    }

    // Levels 0–2: index nodes.
    let mut qs = [TEX_NULL; 4];
    for k in 0..3usize {
        qs[k] = cur_ptr();
        set_cur_ptr(sa_get_child(mem, qs[k], lvl[k]));
        if cur_ptr() == TEX_NULL {
            if !w {
                return 0;
            }
            // lab46/47/48: allocate remaining levels from qs[k].
            let p = sa_alloc_chain(mem, scanner, t, n, qs[k], &lvl[k..]);
            return if p == crate::mem::ALLOC_FAILED { p } else { 0 };
        }
    }

    // Level 3: leaf.
    qs[3] = cur_ptr();
    set_cur_ptr(sa_get_child(mem, qs[3], lvl[3]));
    if cur_ptr() == TEX_NULL && w {
        // lab49: allocate just the data node.
        let p = sa_alloc_chain(mem, scanner, t, n, qs[3], &lvl[3..]);
        if p == crate::mem::ALLOC_FAILED {
            return p;
        }
    }
    0
}

pub unsafe fn find_sa_element_global(t: u16, n: i32, w: bool) {
    let r =
        crate::state::with_state_mut(|st| find_sa_element(&mut st.mem, &mut st.scanner, t, n, w));
    if r == crate::mem::ALLOC_FAILED {
        crate::errors::overflow(c"main memory size".as_ptr(), MEM_TOP + 1);
    }
}
