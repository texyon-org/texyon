//! Save-stack management functions translated from `xetex/xetex-xetex0.c`.
use crate::consts::{
    EQTB_SIZE, GLUE_SPEC_SIZE, INT_BASE, INT_PAR_MAG, LEVEL_ONE, MEM_TOP, TEX_NULL,
    UNDEFINED_CONTROL_SEQUENCE,
};
use crate::state::{EqtbState, GroupCode, MemState, SaveStackState, ScannerState};

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

// Brace-token limits for align_state tracking (xetex-constants.h:350,352).
const LEFT_BRACE_LIMIT: i32 = 0x40_0000;
const RIGHT_BRACE_LIMIT: i32 = 0x60_0000;

// eqtb range constants (xetex_format.h:198,204,224).

// ---------------------------------------------------------------------------
// Cold-path abort sentinel
// ---------------------------------------------------------------------------

/// A cold-path abort requested by a save-stack function. Both abort paths
/// `longjmp`, so a real caller never observes the sentinel — it fires only on a
/// capacity/consistency failure that no passing test reaches.
pub(crate) enum SaveAbort {
    /// `overflow(msg, n)` — capacity exhaustion.
    Overflow(&'static std::ffi::CStr, i32),
    /// `confusion(msg)` — internal-consistency failure.
    Confusion(&'static std::ffi::CStr),
}

impl SaveAbort {
    /// Map to the unified [`EngineAbort`] terminal.
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            SaveAbort::Overflow(s, n) => crate::errors::EngineAbort::Overflow {
                what: s.as_ptr(),
                n,
            },
            SaveAbort::Confusion(s) => crate::errors::EngineAbort::Confusion(s.as_ptr()),
        }
    }
}

// ---------------------------------------------------------------------------
// new_save_level
// ---------------------------------------------------------------------------

/// Push a new grouping level onto the save stack.
///
/// translated from xetex-xetex0.c:3932 (new_save_level)
pub(crate) fn new_save_level(
    save_stack: &mut SaveStackState,
    c: GroupCode,
    line: i32,
) -> Option<SaveAbort> {
    if save_stack.save_ptr > save_stack.max_save_stack {
        save_stack.max_save_stack = save_stack.save_ptr;
        if save_stack.max_save_stack > save_stack.save_size - 7 {
            return Some(SaveAbort::Overflow(c"semantic nest size", save_stack.save_size));
        }
    }
    // `line` is passed in by the caller; it is stamped into
    // the save stack for `\tracingrestores` group-line tracking.
    save_stack.set_b32s1(save_stack.save_ptr, line);
    save_stack.save_ptr += 1;
    save_stack.set_b16s1(save_stack.save_ptr, LEVEL_BOUNDARY);
    save_stack.set_b16s0(save_stack.save_ptr, save_stack.cur_group.as_raw() as u16);
    save_stack.set_b32s1(save_stack.save_ptr, save_stack.cur_boundary);
    if save_stack.cur_level == u16::MAX {
        return Some(SaveAbort::Overflow(c"grouping levels", u16::MAX as i32));
    }
    save_stack.cur_boundary = save_stack.save_ptr;
    save_stack.cur_group = c;
    save_stack.cur_level += 1;
    save_stack.save_ptr += 1;
    None
}

// ---------------------------------------------------------------------------
// eq_save
// ---------------------------------------------------------------------------

/// Save the current value of `eqtb[p]` at level `l` on the save stack.
///
/// translated from xetex-xetex0.c:3914 (eq_save)
pub(crate) fn eq_save(
    save_stack: &mut SaveStackState,
    eqtb: &EqtbState,
    p: i32,
    l: u16,
) -> Option<SaveAbort> {
    if save_stack.save_ptr > save_stack.max_save_stack {
        save_stack.max_save_stack = save_stack.save_ptr;
        if save_stack.max_save_stack > save_stack.save_size - 7 {
            return Some(SaveAbort::Overflow(c"save size", save_stack.save_size));
        }
    }
    if l == LEVEL_ZERO {
        save_stack.set_b16s1(save_stack.save_ptr, RESTORE_ZERO);
    } else {
        save_stack.set_word(save_stack.save_ptr, eqtb.word(p));
        save_stack.save_ptr += 1;
        save_stack.set_b16s1(save_stack.save_ptr, RESTORE_OLD_VALUE);
    }
    save_stack.set_b16s0(save_stack.save_ptr, l);
    save_stack.set_b32s1(save_stack.save_ptr, p);
    save_stack.save_ptr += 1;
    None
}

// ---------------------------------------------------------------------------
// eq_word_define
// ---------------------------------------------------------------------------

/// Assign `eqtb[p].b32.s1 = w`, saving the old value if necessary.
///
/// translated from xetex-xetex0.c:3954 (eq_word_define)
pub(crate) fn eq_word_define(
    eqtb: &mut EqtbState,
    save_stack: &mut SaveStackState,
    p: i32,
    w: i32,
) -> Option<SaveAbort> {
    if eqtb.b32s1(p) == w {
        return None;
    }
    if eqtb.xeq_level_at(p) != save_stack.cur_level {
        let l = eqtb.xeq_level_at(p);
        let abort = eq_save(save_stack, eqtb, p, l);
        if abort.is_some() {
            return abort;
        }
        eqtb.set_xeq_level_at(p, save_stack.cur_level);
    }
    eqtb.set_b32s1(p, w);
    None
}

// ---------------------------------------------------------------------------
// geq_word_define
// ---------------------------------------------------------------------------

/// Globally assign `eqtb[p].b32.s1 = w` at LEVEL_ONE.
///
/// translated from xetex-xetex0.c:3977 (geq_word_define)
pub(crate) fn geq_word_define(eqtb: &mut EqtbState, p: i32, w: i32) {
    eqtb.set_b32s1(p, w);
    eqtb.set_xeq_level_at(p, LEVEL_ONE);
}

// ---------------------------------------------------------------------------
// save_for_after
// ---------------------------------------------------------------------------

/// Push token `t` onto the save stack for deferred processing after `}`.
///
/// translated from xetex-xetex0.c:3984 (save_for_after)
pub(crate) fn save_for_after(save_stack: &mut SaveStackState, t: i32) -> Option<SaveAbort> {
    if save_stack.cur_level > LEVEL_ONE {
        if save_stack.save_ptr > save_stack.max_save_stack {
            save_stack.max_save_stack = save_stack.save_ptr;
            if save_stack.max_save_stack > save_stack.save_size - 7 {
                return Some(SaveAbort::Overflow(c"save size", save_stack.save_size));
            }
        }
        save_stack.set_b16s1(save_stack.save_ptr, INSERT_TOKEN);
        save_stack.set_b16s0(save_stack.save_ptr, LEVEL_ZERO);
        save_stack.set_b32s1(save_stack.save_ptr, t);
        save_stack.save_ptr += 1;
    }
    None
}

// ---------------------------------------------------------------------------
// prepare_mag
// ---------------------------------------------------------------------------

// INT_PAR__mag = 17 (xetex_format.h:52).

/// Validates `\mag`. Both recoverable `int_error` sites return an
/// [`ErrorOutcome`]; a non-`Continue` outcome is propagated (the original would
/// have longjmp'd before reaching the `geq_word_define`, so the early return
/// matches).
pub(crate) fn prepare_mag_in(st: &mut crate::state::EngineState) -> crate::errors::ErrorOutcome {
    use crate::errors::ErrorOutcome;

    let mag = st.eqtb.b32s1(INT_BASE + INT_PAR_MAG);
    if st.scanner.mag_set > 0 && mag != st.scanner.mag_set {
        let mag_set = st.scanner.mag_set;
        let errmsg = crate::output::error_here_with_diagnostic_in(
            st,
            c"Incompatible magnification (".as_ptr(),
        );
        crate::output::print_int_in(st, mag);
        crate::output::print_cstr_in(st, c");".as_ptr());
        crate::output::print_nl_cstr_in(st, c" the previous value will be retained".as_ptr());
        unsafe { ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), mag_set) };
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 2;
        st.errors.help_line[1] =
            c"I can handle only one magnification ratio per job. So I've".as_ptr();
        st.errors.help_line[0] =
            c"reverted to the magnification you used earlier on this run.".as_ptr();
        let outcome = crate::errors::int_error_in(st, mag_set);
        if !matches!(outcome, ErrorOutcome::Continue) {
            return outcome;
        }
        geq_word_define(&mut st.eqtb, INT_BASE + INT_PAR_MAG, mag_set);
    }
    let mag2 = st.eqtb.b32s1(INT_BASE + INT_PAR_MAG);
    if mag2 <= 0 || mag2 > 32768 {
        let errmsg = crate::output::error_here_with_diagnostic_in(
            st,
            c"Illegal magnification has been changed to 1000".as_ptr(),
        );
        unsafe { ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), mag2) };
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 1;
        st.errors.help_line[0] = c"The magnification ratio must be between 1 and 32768.".as_ptr();
        let outcome = crate::errors::int_error_in(st, mag2);
        if !matches!(outcome, ErrorOutcome::Continue) {
            return outcome;
        }
        geq_word_define(&mut st.eqtb, INT_BASE + INT_PAR_MAG, 1000);
    }
    st.scanner.mag_set = st.eqtb.b32s1(INT_BASE + INT_PAR_MAG);
    ErrorOutcome::Continue
}

// ---------------------------------------------------------------------------
// eq_destroy / eq_define / geq_define
// ---------------------------------------------------------------------------

/// Free any heap resource held by an eqtb entry (token list, glue, box, or sa-ref).
///
/// translated from xetex-xetex0.c:3831 (eq_destroy)
pub(crate) fn eq_destroy(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    w: MemoryWord,
) -> Option<SaveAbort> {
    // SAFETY: `MemoryWord` union projections; the eqtb entry was last written
    // through these views by `eq_define`/`geq_define`.
    let typ = unsafe { w.b16.s1 };
    let val = unsafe { w.b32.s1 };
    match typ {
        113..=116 => crate::mem::delete_token_ref(mem, val),
        119 => crate::mem::delete_glue_ref(mem, val),
        120 => {
            if val != TEX_NULL {
                crate::mem::free_node(mem, val, mem.b32s0(val) + mem.b32s0(val) + 1);
            }
        }
        121 => {
            if let Err(msg) = crate::packaging::flush_node_list(mem, val) {
                return Some(SaveAbort::Confusion(msg));
            }
        }
        72 | 91 => {
            if !(0..=19).contains(&val) {
                delete_sa_ref(mem, scanner, val);
            }
        }
        _ => {}
    }
    None
}

/// Assign `eqtb[p] = (level=cur_level, type=t, value=e)`, saving if needed.
///
/// translated from xetex-xetex0.c:3867 (eq_define)
pub(crate) fn eq_define(
    eqtb: &mut EqtbState,
    save_stack: &mut SaveStackState,
    mem: &mut MemState,
    scanner: &mut ScannerState,
    p: i32,
    t: u16,
    e: i32,
) -> Option<SaveAbort> {
    if eqtb.b16s1(p) == t && eqtb.b32s1(p) == e {
        let w = eqtb.word(p);
        return eq_destroy(mem, scanner, w);
    }
    if eqtb.b16s0(p) == save_stack.cur_level {
        let w = eqtb.word(p);
        let abort = eq_destroy(mem, scanner, w);
        if abort.is_some() {
            return abort;
        }
    } else if save_stack.cur_level > LEVEL_ONE {
        let l = eqtb.b16s0(p);
        let abort = eq_save(save_stack, eqtb, p, l);
        if abort.is_some() {
            return abort;
        }
    }
    eqtb.set_b16s0(p, save_stack.cur_level);
    eqtb.set_b16s1(p, t);
    eqtb.set_b32s1(p, e);
    None
}

/// Globally assign `eqtb[p] = (level=LEVEL_ONE, type=t, value=e)`.
///
/// translated from xetex-xetex0.c:3886 (geq_define)
pub(crate) fn geq_define(
    eqtb: &mut EqtbState,
    mem: &mut MemState,
    scanner: &mut ScannerState,
    p: i32,
    t: u16,
    e: i32,
) -> Option<SaveAbort> {
    let w = eqtb.word(p);
    let abort = eq_destroy(mem, scanner, w);
    if abort.is_some() {
        return abort;
    }
    eqtb.set_b16s0(p, LEVEL_ONE);
    eqtb.set_b16s1(p, t);
    eqtb.set_b32s1(p, e);
    None
}

// ---------------------------------------------------------------------------
// Sparse-array (SA) register save/restore cluster
// ---------------------------------------------------------------------------

// Node-size constants (xetex-constants.h:109,120,123,134).
const POINTER_NODE_SIZE: i32 = 2;
const WORD_NODE_SIZE: i32 = 3;
const MARK_CLASS_NODE_SIZE: i32 = 4;
const INDEX_NODE_SIZE: i32 = 33;

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

fn delete_token_ref_threaded(mem: &mut MemState, p: i32) {
    if mem.b32s0(p) == TEX_NULL {
        crate::mem::flush_list(mem, p);
    } else {
        mem.set_b32s0(p, mem.b32s0(p) - 1);
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
            if save_stack.max_save_stack > save_stack.save_size - 7 {
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

#[derive(Default)]
pub(crate) struct SaMutationResult {
    flush_list: Option<i32>,
    status: i32,
}

// translated from xetex-xetex0.c:2878 (sa_destroy)
pub(crate) fn sa_destroy(mem: &mut MemState, p: i32) -> Option<i32> {
    if mem.b16s1(p) < MU_VAL_LIMIT {
        delete_glue_ref_threaded(mem, mem.b32s1(p + 1));
        None
    } else if mem.b32s1(p + 1) != TEX_NULL {
        if mem.b16s1(p) < BOX_VAL_LIMIT {
            Some(mem.b32s1(p + 1))
        } else {
            delete_token_ref_threaded(mem, mem.b32s1(p + 1));
            None
        }
    } else {
        None
    }
}

// translated from xetex-xetex0.c:2891 (sa_def)
pub(crate) fn sa_def(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    save_stack: &mut SaveStackState,
    p: i32,
    e: i32,
) -> SaMutationResult {
    mem.set_b32s0(p + 1, mem.b32s0(p + 1) + 1);
    let mut result = SaMutationResult::default();
    if mem.b32s1(p + 1) == e {
        result.flush_list = sa_destroy(mem, p);
    } else {
        if mem.b16s0(p) == save_stack.cur_level {
            result.flush_list = sa_destroy(mem, p);
        } else {
            let status = sa_save(mem, scanner, save_stack, p);
            if status != 0 {
                result.status = status;
                return result;
            }
        }
        mem.set_b16s0(p, save_stack.cur_level);
        mem.set_b32s1(p + 1, e);
    }
    result
}

// translated from xetex-xetex0.c:2908 (sa_w_def)
pub(crate) fn sa_w_def(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    save_stack: &mut SaveStackState,
    p: i32,
    w: i32,
) -> SaMutationResult {
    mem.set_b32s0(p + 1, mem.b32s0(p + 1) + 1);
    let mut result = SaMutationResult::default();
    if mem.b32s1(p + 2) != w {
        if mem.b16s0(p) != save_stack.cur_level {
            let status = sa_save(mem, scanner, save_stack, p);
            if status != 0 {
                result.status = status;
                return result;
            }
        }
        mem.set_b16s0(p, save_stack.cur_level);
        mem.set_b32s1(p + 2, w);
    }
    result
}

// translated from xetex-xetex0.c:2923 (gsa_def)
pub(crate) fn gsa_def(mem: &mut MemState, p: i32, e: i32) -> SaMutationResult {
    mem.set_b32s0(p + 1, mem.b32s0(p + 1) + 1);
    let flush_list = sa_destroy(mem, p);
    mem.set_b16s0(p, LEVEL_ONE);
    mem.set_b32s1(p + 1, e);
    SaMutationResult {
        flush_list,
        status: 0,
    }
}

// translated from xetex-xetex0.c:2933 (gsa_w_def)
pub(crate) fn gsa_w_def(mem: &mut MemState, p: i32, w: i32) -> SaMutationResult {
    mem.set_b32s0(p + 1, mem.b32s0(p + 1) + 1);
    mem.set_b16s0(p, LEVEL_ONE);
    mem.set_b32s1(p + 2, w);
    SaMutationResult::default()
}

/// Cold abort sentinel for [`finish_sa_mutation_in`].
pub(crate) enum SaMutationAbort {
    MainMemory,
    SaveOverflow,
    Confusion(&'static std::ffi::CStr),
}

impl SaMutationAbort {
    /// Map to the unified [`EngineAbort`] terminal. `MainMemory` here is
    /// a plain overflow (no `runaway`).
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            SaMutationAbort::MainMemory => crate::errors::EngineAbort::Overflow {
                what: c"main memory size".as_ptr(),
                n: MEM_TOP + 1,
            },
            SaMutationAbort::SaveOverflow => crate::errors::EngineAbort::SaveOverflow,
            SaMutationAbort::Confusion(m) => crate::errors::EngineAbort::Confusion(m.as_ptr()),
        }
    }
}

/// Applies the deferred flush + ref-decrement of an `sa_def`/`gsa_def` result.
pub(crate) fn finish_sa_mutation_in(
    st: &mut crate::state::EngineState,
    p: i32,
    result: SaMutationResult,
) -> Result<(), SaMutationAbort> {
    if result.status == crate::mem::ALLOC_FAILED {
        return Err(SaMutationAbort::MainMemory);
    }
    if result.status == SAVE_OVERFLOW {
        return Err(SaMutationAbort::SaveOverflow);
    }
    if let Some(list) = result.flush_list {
        crate::packaging::flush_node_list(&mut st.mem, list).map_err(SaMutationAbort::Confusion)?;
    }
    delete_sa_ref(&mut st.mem, &mut st.scanner, p);
    Ok(())
}

struct SaRestoreStep {
    p: i32,
    chain: i32,
    flush_list: Option<i32>,
}

// translated from xetex-xetex0.c:2942 (sa_restore)
fn sa_restore_prepare(mem: &mut MemState, scanner: &mut ScannerState) -> SaRestoreStep {
    let sc = scanner.sa_chain;
    let p = mem.b32s0(sc + 1);
    let mut flush_list = None;
    if mem.b16s0(p) == LEVEL_ONE {
        if mem.b16s1(p) >= DIMEN_VAL_LIMIT {
            flush_list = sa_destroy(mem, sc);
        }
    } else {
        if mem.b16s1(p) < DIMEN_VAL_LIMIT {
            if mem.b16s1(sc) < DIMEN_VAL_LIMIT {
                mem.set_b32s1(p + 2, mem.b32s1(sc + 2));
            } else {
                mem.set_b32s1(p + 2, 0);
            }
        } else {
            flush_list = sa_destroy(mem, p);
            mem.set_b32s1(p + 1, mem.b32s1(sc + 1));
        }
        mem.set_b16s0(p, mem.b16s0(sc));
    }
    SaRestoreStep {
        p,
        chain: sc,
        flush_list,
    }
}

// ---------------------------------------------------------------------------
// unsave
// ---------------------------------------------------------------------------

/// Cold-path abort raised by [`unsave_in`]. Surfaces the deferred
/// `eq_destroy`/`flush_node_list` `confusion`, the `back_input` input abort, and
/// the `get_avail` main-memory overflow. Callers `longjmp` on it; no passing
/// test reaches any arm.
pub(crate) enum UnsaveAbort {
    Save(SaveAbort),
    Input(crate::input::InputAbort),
    MainMemory,
}

impl UnsaveAbort {
    /// Map to the unified [`EngineAbort`] terminal. `MainMemory` runs
    /// `runaway` (via `main_memory_overflow_abort`).
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            UnsaveAbort::Save(a) => a.into_engine_abort(),
            UnsaveAbort::Input(e) => e.into_engine_abort(),
            UnsaveAbort::MainMemory => crate::mem::main_memory_overflow_abort(),
        }
    }
}

impl From<crate::input::InputAbort> for UnsaveAbort {
    fn from(e: crate::input::InputAbort) -> Self {
        UnsaveAbort::Input(e)
    }
}

/// Restore the sparse-array chain, returning the `flush_node_list` confusion.
fn sa_restore_in(st: &mut crate::state::EngineState) -> Result<(), &'static std::ffi::CStr> {
    loop {
        let step = {
            let s = &mut *st;
            sa_restore_prepare(&mut s.mem, &mut s.scanner)
        };
        if let Some(list) = step.flush_list {
            crate::packaging::flush_node_list(&mut st.mem, list)?;
        }
        delete_sa_ref(&mut st.mem, &mut st.scanner, step.p);
        st.scanner.sa_chain = st.mem.b32s1(step.chain);
        if st.mem.b16s1(step.chain) < DIMEN_VAL_LIMIT {
            crate::mem::free_node(&mut st.mem, step.chain, WORD_NODE_SIZE);
        } else {
            crate::mem::free_node(&mut st.mem, step.chain, POINTER_NODE_SIZE);
        }
        if st.scanner.sa_chain == TEX_NULL {
            break;
        }
    }
    Ok(())
}

/// Pops one grouping level. Cold aborts ride [`UnsaveAbort`].
///
/// translated from xetex-xetex0.c:2814 (unsave)
pub(crate) fn unsave_in(st: &mut crate::state::EngineState) -> Result<(), UnsaveAbort> {
    if st.save_stack.cur_level > LEVEL_ONE {
        st.save_stack.cur_level -= 1;
        let mut l: u16 = 0;
        let mut a = false;
        loop {
            st.save_stack.save_ptr -= 1;
            let sp = st.save_stack.save_ptr;
            if st.save_stack.b16s1(sp) == LEVEL_BOUNDARY {
                break; // goto done
            }
            let p = st.save_stack.b32s1(sp);
            if st.save_stack.b16s1(sp) == INSERT_TOKEN {
                let t = st.scanner.cur_tok;
                st.scanner.cur_tok = p;
                if a {
                    let pn = crate::mem::get_avail(&mut st.mem);
                    if pn == crate::mem::ALLOC_FAILED {
                        return Err(UnsaveAbort::MainMemory);
                    }
                    st.mem.set_b32s0(pn, st.scanner.cur_tok);
                    st.mem.set_b32s1(pn, st.input.cur_input.loc);
                    st.input.cur_input.loc = pn;
                    st.input.cur_input.start = pn;
                    if st.scanner.cur_tok < RIGHT_BRACE_LIMIT {
                        if st.scanner.cur_tok < LEFT_BRACE_LIMIT {
                            st.scanner.align_state -= 1;
                        } else {
                            st.scanner.align_state += 1;
                        }
                    }
                } else {
                    crate::input::back_input_in(st)?;
                    a = true;
                }
                st.scanner.cur_tok = t;
            } else if st.save_stack.b16s1(sp) == RESTORE_SA {
                sa_restore_in(st).map_err(|m| UnsaveAbort::Save(SaveAbort::Confusion(m)))?;
                st.scanner.sa_chain = p;
                st.scanner.sa_level = st.save_stack.b16s0(st.save_stack.save_ptr);
            } else {
                if st.save_stack.b16s1(sp) == RESTORE_OLD_VALUE {
                    l = st.save_stack.b16s0(sp);
                    st.save_stack.save_ptr -= 1;
                } else {
                    let w = st.eqtb.word(UNDEFINED_CONTROL_SEQUENCE);
                    st.save_stack.set_word(st.save_stack.save_ptr, w);
                }
                let sp = st.save_stack.save_ptr;
                if !(INT_BASE..=EQTB_SIZE).contains(&p) {
                    if st.eqtb.b16s0(p) == LEVEL_ONE {
                        let w = st.save_stack.word(sp);
                        if let Some(ab) = {
                            let s = &mut *st;
                            eq_destroy(&mut s.mem, &mut s.scanner, w)
                        } {
                            return Err(UnsaveAbort::Save(ab));
                        }
                    } else {
                        let w = st.eqtb.word(p);
                        if let Some(ab) = {
                            let s = &mut *st;
                            eq_destroy(&mut s.mem, &mut s.scanner, w)
                        } {
                            return Err(UnsaveAbort::Save(ab));
                        }
                        let w = st.save_stack.word(sp);
                        st.eqtb.data[p as usize] = w;
                    }
                } else if st.eqtb.xeq_level_at(p) != LEVEL_ONE {
                    let w = st.save_stack.word(sp);
                    st.eqtb.data[p as usize] = w;
                    st.eqtb.set_xeq_level_at(p, l);
                }
            }
        }
        // done:
        if st.input.grp_stack[st.input.in_open as usize] == st.save_stack.cur_boundary {
            crate::output::group_warning_in(st);
        }
        let sp = st.save_stack.save_ptr;
        st.save_stack.cur_group =
            GroupCode::from_raw(st.save_stack.b16s0(sp) as u8).expect("valid group code");
        st.save_stack.cur_boundary = st.save_stack.b32s1(sp);
        st.save_stack.save_ptr -= 1;
        Ok(())
    } else {
        Err(UnsaveAbort::Save(SaveAbort::Confusion(c"curlevel")))
    }
}

// ---------------------------------------------------------------------------
// new_index / find_sa_element
// ---------------------------------------------------------------------------

/// Allocate a new INDEX_NODE_SIZE node for the sparse-array trie.
///
/// translated from xetex-xetex0.c:4111 (new_index)
fn new_index(mem: &mut MemState, scanner: &mut ScannerState, i: u16, q: i32) -> i32 {
    let p = crate::mem::get_node(mem, INDEX_NODE_SIZE);
    if p == crate::mem::ALLOC_FAILED {
        return p;
    }
    scanner.cur_ptr = p;
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
    scanner: &mut ScannerState,
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
        q = scanner.cur_ptr;
    }
    // Allocate the data (leaf) node (lab49).
    let leaf_i = levels[levels.len() - 1];
    let p;
    if t == MARK_VAL {
        p = crate::mem::get_node(mem, MARK_CLASS_NODE_SIZE);
        if p == crate::mem::ALLOC_FAILED {
            return p;
        }
        scanner.cur_ptr = p;
        mem.set_word(p + 1, scanner.sa_null);
        mem.set_word(p + 2, scanner.sa_null);
        mem.set_word(p + 3, scanner.sa_null);
    } else if t <= DIMEN_VAL {
        p = crate::mem::get_node(mem, WORD_NODE_SIZE);
        if p == crate::mem::ALLOC_FAILED {
            return p;
        }
        scanner.cur_ptr = p;
        mem.set_b32s1(p + 2, 0);
        mem.set_b32s1(p + 1, n);
    } else {
        p = crate::mem::get_node(mem, POINTER_NODE_SIZE);
        if p == crate::mem::ALLOC_FAILED {
            return p;
        }
        scanner.cur_ptr = p;
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
    scanner.cur_ptr = scanner.sa_root[t as usize];

    // Check root level (not_found in C).
    if scanner.cur_ptr == TEX_NULL {
        if !w {
            return 0;
        }
        let p = new_index(mem, scanner, t, TEX_NULL);
        if p == crate::mem::ALLOC_FAILED {
            return p;
        }
        scanner.sa_root[t as usize] = scanner.cur_ptr;
        // lab46..lab49: allocate all 4 levels from this root.
        let p = sa_alloc_chain(mem, scanner, t, n, scanner.cur_ptr, &lvl);
        return if p == crate::mem::ALLOC_FAILED { p } else { 0 };
    }

    // Levels 0–2: index nodes.
    let mut qs = [TEX_NULL; 4];
    for k in 0..3usize {
        qs[k] = scanner.cur_ptr;
        scanner.cur_ptr = sa_get_child(mem, qs[k], lvl[k]);
        if scanner.cur_ptr == TEX_NULL {
            if !w {
                return 0;
            }
            // lab46/47/48: allocate remaining levels from qs[k].
            let p = sa_alloc_chain(mem, scanner, t, n, qs[k], &lvl[k..]);
            return if p == crate::mem::ALLOC_FAILED { p } else { 0 };
        }
    }

    // Level 3: leaf.
    qs[3] = scanner.cur_ptr;
    scanner.cur_ptr = sa_get_child(mem, qs[3], lvl[3]);
    if scanner.cur_ptr == TEX_NULL && w {
        // lab49: allocate just the data node.
        let p = sa_alloc_chain(mem, scanner, t, n, qs[3], &lvl[3..]);
        if p == crate::mem::ALLOC_FAILED {
            return p;
        }
    }
    0
}
