//! Save-stack management functions translated from `xetex/xetex-xetex0.c`.
//!
//! Phase A: all state accessed through the C globals via `src/globals.rs`.

use crate::globals::{
    cur_boundary, cur_group, cur_level, eqtb_b32s1, eqtb_word, line, max_save_stack,
    save_ptr, save_size, set_cur_boundary, set_cur_group, set_cur_level,
    set_eqtb_b32s1, set_max_save_stack, set_save_ptr, set_save_stack_b16s0,
    set_save_stack_b16s1, set_save_stack_b32s1, set_save_stack_word, set_xeq_level, xeq_level,
};

// Save-stack discriminant values (xetex-constants.h:260–285).
const RESTORE_OLD_VALUE: u16 = 0;
const RESTORE_ZERO: u16 = 1;
const INSERT_TOKEN: u16 = 2;
const LEVEL_BOUNDARY: u16 = 3;

// Grouping level constants (xetex-constants.h:89–90).
const LEVEL_ZERO: u16 = 0;
const LEVEL_ONE: u16 = 1;

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
