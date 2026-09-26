//! The save stack, `eqtb` definitions, and the sparse-array register trie.
use crate::bridge::diag_append_str;
use crate::cmd::{BOX_REF, CALL, GLUE_REF, LONG_OUTER_CALL, REGISTER, SHAPE_REF, TOKS_REGISTER};
use crate::consts::{
    EQTB_SIZE, INDEX_NODE_SIZE, INT_BASE, INT_PAR_MAG, LEVEL_ONE, LEVEL_ZERO, MARK_CLASS_NODE_SIZE,
    MEM_TOP, TEX_NULL, UNDEFINED_CONTROL_SEQUENCE,
};
use crate::input::{LEFT_BRACE_LIMIT, RIGHT_BRACE_LIMIT};
use crate::mem::{delete_glue_ref, delete_token_ref, free_node, get_node, ZERO_GLUE};
use crate::mem_types::MemoryWord;
use crate::node::{NodePtr, SaNode};
use crate::scan::{DIMEN_VAL, MARK_VAL, MU_VAL};
use crate::state::{EngineState, EqtbState, GroupCode, MemState, SaveStackState, ScannerState};
use crate::units::{MemAddr, MemPtr};

// Save-stack entry types.
const RESTORE_OLD_VALUE: u16 = 0;
const RESTORE_ZERO: u16 = 1;
const INSERT_TOKEN: u16 = 2;
const LEVEL_BOUNDARY: u16 = 3;
const RESTORE_SA: u16 = 4;

const POINTER_NODE_SIZE: i32 = 2;
const WORD_NODE_SIZE: i32 = 3;

pub(crate) const DIMEN_VAL_LIMIT: u16 = 128;
pub(crate) const MU_VAL_LIMIT: u16 = 256;
pub(crate) const BOX_VAL_LIMIT: u16 = 320;
pub(crate) const TOK_VAL_LIMIT: u16 = 384;

/// A save-stack abort, which ends the run.
pub(crate) enum SaveAbort {
    Overflow(&'static std::ffi::CStr, i32),
    Confusion(&'static std::ffi::CStr),
}

impl SaveAbort {
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

/// TeX's `check_full_save_stack`, true when no room is left.
fn save_stack_full(save_stack: &mut SaveStackState) -> bool {
    if save_stack.save_ptr > save_stack.max_save_stack {
        save_stack.max_save_stack = save_stack.save_ptr;
        return save_stack.max_save_stack > save_stack.save_size - 7;
    }
    false
}

fn set_save_entry(save_stack: &mut SaveStackState, kind: u16, level: u16, index: i32) {
    let k = save_stack.save_ptr;
    save_stack.set_b16s1(k, kind);
    save_stack.set_b16s0(k, level);
    save_stack.set_b32s1(k, index);
}

pub(crate) fn new_save_level(
    save_stack: &mut SaveStackState,
    c: GroupCode,
    line: i32,
) -> Result<(), SaveAbort> {
    if save_stack_full(save_stack) {
        return Err(SaveAbort::Overflow(
            c"semantic nest size",
            save_stack.save_size,
        ));
    }
    save_stack.set_b32s1(save_stack.save_ptr, line);
    save_stack.save_ptr += 1;
    let (group, boundary) = (
        save_stack.cur_group.as_raw() as u16,
        save_stack.cur_boundary,
    );
    set_save_entry(save_stack, LEVEL_BOUNDARY, group, boundary);
    if save_stack.cur_level == u16::MAX {
        return Err(SaveAbort::Overflow(c"grouping levels", u16::MAX as i32));
    }
    save_stack.cur_boundary = save_stack.save_ptr;
    save_stack.cur_group = c;
    save_stack.cur_level += 1;
    save_stack.save_ptr += 1;
    Ok(())
}

pub(crate) fn eq_save(
    save_stack: &mut SaveStackState,
    eqtb: &EqtbState,
    p: i32,
    l: u16,
) -> Result<(), SaveAbort> {
    if save_stack_full(save_stack) {
        return Err(SaveAbort::Overflow(c"save size", save_stack.save_size));
    }
    let kind = if l == LEVEL_ZERO {
        RESTORE_ZERO
    } else {
        save_stack.set_word(save_stack.save_ptr, eqtb.word(p));
        save_stack.save_ptr += 1;
        RESTORE_OLD_VALUE
    };
    set_save_entry(save_stack, kind, l, p);
    save_stack.save_ptr += 1;
    Ok(())
}

pub(crate) fn eq_word_define(
    eqtb: &mut EqtbState,
    save_stack: &mut SaveStackState,
    p: i32,
    w: i32,
) -> Result<(), SaveAbort> {
    if eqtb.int_at(p) == w {
        return Ok(());
    }
    let l = eqtb.xeq_level_at(p);
    if l != save_stack.cur_level {
        eq_save(save_stack, eqtb, p, l)?;
        eqtb.set_xeq_level_at(p, save_stack.cur_level);
    }
    eqtb.set_int_at(p, w);
    Ok(())
}

pub(crate) fn geq_word_define(eqtb: &mut EqtbState, p: i32, w: i32) {
    eqtb.set_int_at(p, w);
    eqtb.set_xeq_level_at(p, LEVEL_ONE);
}

pub(crate) fn save_for_after(save_stack: &mut SaveStackState, t: i32) -> Result<(), SaveAbort> {
    if save_stack.cur_level > LEVEL_ONE {
        if save_stack_full(save_stack) {
            return Err(SaveAbort::Overflow(c"save size", save_stack.save_size));
        }
        set_save_entry(save_stack, INSERT_TOKEN, LEVEL_ZERO, t);
        save_stack.save_ptr += 1;
    }
    Ok(())
}

/// Validates `\mag`, propagating a non-`Continue` error outcome.
pub(crate) fn prepare_mag_in(st: &mut EngineState) -> crate::errors::ErrorOutcome {
    use crate::errors::ErrorOutcome;
    use crate::output::{capture_to_diagnostic_in, error_here_with_diagnostic_in};

    let mag = st.eqtb.int_par(INT_PAR_MAG);
    if st.scanner.mag_set > 0 && mag != st.scanner.mag_set {
        let mag_set = st.scanner.mag_set;
        let errmsg = error_here_with_diagnostic_in(st, c"Incompatible magnification (".as_ptr());
        crate::output::print_int_in(st, mag);
        crate::output::print_cstr_in(st, c");".as_ptr());
        crate::output::print_nl_cstr_in(st, c" the previous value will be retained".as_ptr());
        unsafe { diag_append_str(errmsg, &format!(" ({})", mag_set)) };
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help(&[
            "I can handle only one magnification ratio per job. So I've",
            "reverted to the magnification you used earlier on this run.",
        ]);
        let outcome = crate::errors::int_error_in(st, mag_set);
        if !matches!(outcome, ErrorOutcome::Continue) {
            return outcome;
        }
        geq_word_define(&mut st.eqtb, INT_BASE + INT_PAR_MAG, mag_set);
    }
    let mag = st.eqtb.int_par(INT_PAR_MAG);
    if mag <= 0 || mag > 32768 {
        let errmsg = error_here_with_diagnostic_in(
            st,
            c"Illegal magnification has been changed to 1000".as_ptr(),
        );
        unsafe { diag_append_str(errmsg, &format!(" ({})", mag)) };
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors
            .help(&["The magnification ratio must be between 1 and 32768."]);
        let outcome = crate::errors::int_error_in(st, mag);
        if !matches!(outcome, ErrorOutcome::Continue) {
            return outcome;
        }
        geq_word_define(&mut st.eqtb, INT_BASE + INT_PAR_MAG, 1000);
    }
    st.scanner.mag_set = st.eqtb.int_par(INT_PAR_MAG);
    ErrorOutcome::Continue
}

/// Frees the heap resource an `eqtb` entry owns.
pub(crate) fn eq_destroy(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    w: MemoryWord,
) -> Result<(), SaveAbort> {
    // SAFETY: `MemoryWord` union projections of an `eqtb` entry.
    let (eq_type, equiv) = unsafe { (w.b16.s1, w.b32.s1) };
    let Ok(eq_type) = u8::try_from(eq_type) else {
        return Ok(());
    };
    match eq_type {
        CALL..=LONG_OUTER_CALL => delete_token_ref(mem, MemPtr(equiv)),
        GLUE_REF => delete_glue_ref(mem, crate::node::NodePtr::from_glue_ptr(MemPtr(equiv))),
        SHAPE_REF => {
            if equiv != TEX_NULL {
                let size = 2 * mem.b32s0(equiv) + 1;
                free_node(mem, MemPtr::from_raw(equiv), size);
            }
        }
        BOX_REF => crate::packaging::flush_node_list(mem, equiv).map_err(SaveAbort::Confusion)?,
        TOKS_REGISTER | REGISTER => {
            if !(0..=19).contains(&equiv) {
                delete_sa_ref(mem, scanner, equiv);
            }
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn eq_define(
    eqtb: &mut EqtbState,
    save_stack: &mut SaveStackState,
    mem: &mut MemState,
    scanner: &mut ScannerState,
    p: i32,
    t: u8,
    e: i32,
) -> Result<(), SaveAbort> {
    if eqtb.eq_type(p) == t && eqtb.equiv(p) == e {
        return eq_destroy(mem, scanner, eqtb.word(p));
    }
    if eqtb.eq_level(p) == save_stack.cur_level {
        eq_destroy(mem, scanner, eqtb.word(p))?;
    } else if save_stack.cur_level > LEVEL_ONE {
        eq_save(save_stack, eqtb, p, eqtb.eq_level(p))?;
    }
    eqtb.set_eq_level(p, save_stack.cur_level);
    eqtb.set_eq_type(p, t);
    eqtb.set_equiv(p, e);
    Ok(())
}

pub(crate) fn geq_define(
    eqtb: &mut EqtbState,
    mem: &mut MemState,
    scanner: &mut ScannerState,
    p: i32,
    t: u8,
    e: i32,
) -> Result<(), SaveAbort> {
    eq_destroy(mem, scanner, eqtb.word(p))?;
    eqtb.set_eq_level(p, LEVEL_ONE);
    eqtb.set_eq_type(p, t);
    eqtb.set_equiv(p, e);
    Ok(())
}

/// A sparse-array record. These carry no type tag and are reached only
/// through the trie or a register's `eqtb` entry.
#[inline]
fn sa(p: impl MemAddr) -> NodePtr<SaNode> {
    NodePtr::at(p)
}

pub fn delete_sa_ref(mem: &mut MemState, scanner: &mut ScannerState, q: impl MemAddr) {
    let mut q = sa(q);
    q.set_sa_ref(mem, q.sa_ref(mem) - 1);
    if q.sa_ref(mem) != TEX_NULL {
        return;
    }
    let mut s = if q.sa_index(mem) < DIMEN_VAL_LIMIT {
        if q.sa_int(mem) != 0 {
            return;
        }
        WORD_NODE_SIZE
    } else {
        if q.sa_index(mem) < MU_VAL_LIMIT {
            if q.sa_ptr(mem) != ZERO_GLUE.ptr() {
                return;
            }
            delete_glue_ref(mem, ZERO_GLUE);
        } else if !q.sa_ptr(mem).is_null() {
            return;
        }
        POINTER_NODE_SIZE
    };
    loop {
        let i = (q.sa_index(mem) % 64) as i32;
        let next = q.link(mem);
        free_node(mem, q.ptr(), s);
        if next.is_null() {
            scanner.sa_root[i as usize] = MemPtr::NULL;
            return;
        }
        q = sa(next);
        q.set_child(mem, i, MemPtr::NULL);
        q.set_ref_count(mem, q.ref_count(mem) - 1);
        s = INDEX_NODE_SIZE;
        if q.ref_count(mem) > 0 {
            break;
        }
    }
}

pub fn sa_save(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    save_stack: &mut SaveStackState,
    p: i32,
) -> Result<(), SaMutationAbort> {
    if save_stack.cur_level != scanner.sa_level {
        if save_stack_full(save_stack) {
            return Err(SaMutationAbort::SaveOverflow);
        }
        set_save_entry(save_stack, RESTORE_SA, scanner.sa_level, scanner.sa_chain.0);
        save_stack.save_ptr += 1;
        scanner.sa_chain = MemPtr::NULL;
        scanner.sa_level = save_stack.cur_level;
    }
    let p = sa(p);
    let mut i = p.sa_index(mem);
    let q;
    if i < DIMEN_VAL_LIMIT {
        if p.sa_int(mem) == 0 {
            q = sa(get_node(mem, POINTER_NODE_SIZE).ok_or(SaMutationAbort::MainMemory)?);
            i = TOK_VAL_LIMIT;
        } else {
            q = sa(get_node(mem, WORD_NODE_SIZE).ok_or(SaMutationAbort::MainMemory)?);
            let v = p.sa_int(mem);
            q.set_sa_int(mem, v);
        }
        q.set_sa_ptr(mem, MemPtr::NULL);
    } else {
        q = sa(get_node(mem, POINTER_NODE_SIZE).ok_or(SaMutationAbort::MainMemory)?);
        let v = p.sa_ptr(mem);
        q.set_sa_ptr(mem, v);
    }
    // sa_loc
    q.set_sa_ref(mem, p.idx().0);
    q.set_sa_index(mem, i);
    let lev = p.sa_lev(mem);
    q.set_sa_lev(mem, lev);
    q.set_link(mem, scanner.sa_chain);
    scanner.sa_chain = q.ptr();
    p.set_sa_ref(mem, p.sa_ref(mem) + 1);
    Ok(())
}

/// Deferred flush work from an `sa_def`/`sa_w_def`-family mutation.
#[derive(Default)]
pub(crate) struct SaMutationResult {
    flush_list: Option<i32>,
}

/// Releases the value of `p`, returning a box list the caller must flush.
pub(crate) fn sa_destroy(mem: &mut MemState, p: impl MemAddr) -> Option<i32> {
    let p = sa(p);
    let value = p.sa_ptr(mem);
    if p.sa_index(mem) < MU_VAL_LIMIT {
        delete_glue_ref(mem, crate::node::NodePtr::from_glue_ptr(value));
    } else if !value.is_null() {
        if p.sa_index(mem) < BOX_VAL_LIMIT {
            return Some(value.0);
        }
        delete_token_ref(mem, value);
    }
    None
}

pub(crate) fn sa_def(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    save_stack: &mut SaveStackState,
    p: i32,
    e: i32,
) -> Result<SaMutationResult, SaMutationAbort> {
    let node = sa(p);
    node.set_sa_ref(mem, node.sa_ref(mem) + 1);
    let mut result = SaMutationResult::default();
    if node.sa_ptr(mem).0 == e {
        result.flush_list = sa_destroy(mem, p);
    } else {
        if node.sa_lev(mem) == save_stack.cur_level {
            result.flush_list = sa_destroy(mem, p);
        } else {
            sa_save(mem, scanner, save_stack, p)?;
        }
        node.set_sa_lev(mem, save_stack.cur_level);
        node.set_sa_ptr(mem, MemPtr::from_raw(e));
    }
    Ok(result)
}

pub(crate) fn sa_w_def(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    save_stack: &mut SaveStackState,
    p: i32,
    w: i32,
) -> Result<SaMutationResult, SaMutationAbort> {
    let node = sa(p);
    node.set_sa_ref(mem, node.sa_ref(mem) + 1);
    if node.sa_int(mem) != w {
        if node.sa_lev(mem) != save_stack.cur_level {
            sa_save(mem, scanner, save_stack, p)?;
        }
        node.set_sa_lev(mem, save_stack.cur_level);
        node.set_sa_int(mem, w);
    }
    Ok(SaMutationResult::default())
}

pub(crate) fn gsa_def(mem: &mut MemState, p: impl MemAddr, e: i32) -> SaMutationResult {
    let p = sa(p);
    p.set_sa_ref(mem, p.sa_ref(mem) + 1);
    let flush_list = sa_destroy(mem, p.idx());
    p.set_sa_lev(mem, LEVEL_ONE);
    p.set_sa_ptr(mem, MemPtr::from_raw(e));
    SaMutationResult { flush_list }
}

pub(crate) fn gsa_w_def(mem: &mut MemState, p: impl MemAddr, w: i32) -> SaMutationResult {
    let p = sa(p);
    p.set_sa_ref(mem, p.sa_ref(mem) + 1);
    p.set_sa_lev(mem, LEVEL_ONE);
    p.set_sa_int(mem, w);
    SaMutationResult::default()
}

pub(crate) enum SaMutationAbort {
    MainMemory,
    SaveOverflow,
    Confusion(&'static std::ffi::CStr),
}

impl SaMutationAbort {
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

/// Applies the deferred flush and reference drop of an `sa_def`-family result.
pub(crate) fn finish_sa_mutation_in(
    st: &mut EngineState,
    p: i32,
    result: SaMutationResult,
) -> Result<(), SaMutationAbort> {
    if let Some(list) = result.flush_list {
        crate::packaging::flush_node_list(&mut st.mem, list).map_err(SaMutationAbort::Confusion)?;
    }
    delete_sa_ref(&mut st.mem, &mut st.scanner, p);
    Ok(())
}

pub(crate) enum UnsaveAbort {
    Save(SaveAbort),
    Input(crate::input::InputAbort),
    MainMemory,
}

impl UnsaveAbort {
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            UnsaveAbort::Save(a) => a.into_engine_abort(),
            UnsaveAbort::Input(e) => e.into_engine_abort(),
            UnsaveAbort::MainMemory => crate::errors::EngineAbort::MainMemoryOverflow,
        }
    }
}

impl From<crate::input::InputAbort> for UnsaveAbort {
    fn from(e: crate::input::InputAbort) -> Self {
        UnsaveAbort::Input(e)
    }
}

impl From<SaveAbort> for UnsaveAbort {
    fn from(e: SaveAbort) -> Self {
        UnsaveAbort::Save(e)
    }
}

fn sa_restore_in(st: &mut EngineState) -> Result<(), &'static std::ffi::CStr> {
    loop {
        let chain = sa(st.scanner.sa_chain);
        let p = sa(chain.sa_ref(&st.mem));
        let mut flush_list = None;
        if p.sa_lev(&st.mem) == LEVEL_ONE {
            if p.sa_index(&st.mem) >= DIMEN_VAL_LIMIT {
                flush_list = sa_destroy(&mut st.mem, chain.idx());
            }
        } else {
            if p.sa_index(&st.mem) < DIMEN_VAL_LIMIT {
                let v = if chain.sa_index(&st.mem) < DIMEN_VAL_LIMIT {
                    chain.sa_int(&st.mem)
                } else {
                    0
                };
                p.set_sa_int(&mut st.mem, v);
            } else {
                flush_list = sa_destroy(&mut st.mem, p.idx());
                let v = chain.sa_ptr(&st.mem);
                p.set_sa_ptr(&mut st.mem, v);
            }
            let lev = chain.sa_lev(&st.mem);
            p.set_sa_lev(&mut st.mem, lev);
        }
        if let Some(list) = flush_list {
            crate::packaging::flush_node_list(&mut st.mem, list)?;
        }
        delete_sa_ref(&mut st.mem, &mut st.scanner, p.idx());
        st.scanner.sa_chain = chain.link(&st.mem);
        let size = if chain.sa_index(&st.mem) < DIMEN_VAL_LIMIT {
            WORD_NODE_SIZE
        } else {
            POINTER_NODE_SIZE
        };
        free_node(&mut st.mem, chain.ptr(), size);
        if st.scanner.sa_chain.is_null() {
            return Ok(());
        }
    }
}

pub(crate) fn unsave_in(st: &mut EngineState) -> Result<(), UnsaveAbort> {
    if st.save_stack.cur_level <= LEVEL_ONE {
        return Err(UnsaveAbort::Save(SaveAbort::Confusion(c"curlevel")));
    }
    st.save_stack.cur_level -= 1;
    let mut l: u16 = 0;
    let mut a = false;
    loop {
        st.save_stack.save_ptr -= 1;
        let sp = st.save_stack.save_ptr;
        let p = st.save_stack.b32s1(sp);
        match st.save_stack.b16s1(sp) {
            LEVEL_BOUNDARY => break,
            INSERT_TOKEN => {
                let t = st.scanner.cur_tok;
                st.scanner.cur_tok = p;
                if a {
                    let pn = crate::mem::get_avail(&mut st.mem).ok_or(UnsaveAbort::MainMemory)?;
                    pn.set_info(&mut st.mem, st.scanner.cur_tok);
                    pn.set_link(&mut st.mem, MemPtr::from_raw(st.input.cur_input.loc));
                    st.input.cur_input.loc = pn.0;
                    st.input.cur_input.start = pn.0;
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
            }
            RESTORE_SA => {
                sa_restore_in(st).map_err(|m| UnsaveAbort::Save(SaveAbort::Confusion(m)))?;
                st.scanner.sa_chain = MemPtr::from_raw(p);
                st.scanner.sa_level = st.save_stack.b16s0(st.save_stack.save_ptr);
            }
            kind => {
                if kind == RESTORE_OLD_VALUE {
                    l = st.save_stack.b16s0(sp);
                    st.save_stack.save_ptr -= 1;
                } else {
                    let w = st.eqtb.word(UNDEFINED_CONTROL_SEQUENCE);
                    st.save_stack.set_word(st.save_stack.save_ptr, w);
                }
                let saved = st.save_stack.word(st.save_stack.save_ptr);
                if !(INT_BASE..=EQTB_SIZE).contains(&p) {
                    if st.eqtb.eq_level(p) == LEVEL_ONE {
                        eq_destroy(&mut st.mem, &mut st.scanner, saved)?;
                    } else {
                        eq_destroy(&mut st.mem, &mut st.scanner, st.eqtb.word(p))?;
                        st.eqtb.set_word(p, saved);
                    }
                } else if st.eqtb.xeq_level_at(p) != LEVEL_ONE {
                    st.eqtb.set_word(p, saved);
                    st.eqtb.set_xeq_level_at(p, l);
                }
            }
        }
    }
    if st.input.grp_stack[st.input.in_open as usize] == st.save_stack.cur_boundary {
        crate::output::group_warning_in(st);
    }
    let ss = &mut st.save_stack;
    (ss.cur_group, ss.cur_boundary) = ss.boundary_at(ss.save_ptr);
    ss.save_ptr -= 1;
    Ok(())
}

/// Allocates a trie index node below `q`, also leaving it in `cur_ptr`.
fn new_index(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    i: u16,
    q: MemPtr,
) -> Option<NodePtr<SaNode>> {
    let p = sa(get_node(mem, INDEX_NODE_SIZE)?);
    scanner.cur_ptr = p.ptr();
    p.set_sa_index(mem, i);
    p.set_ref_count(mem, 0);
    p.set_link(mem, q);
    for k in 1..INDEX_NODE_SIZE {
        mem.set_word(p.idx().offset(k), scanner.sa_null);
    }
    Some(p)
}

fn add_sa_child(mem: &mut MemState, q: NodePtr<SaNode>, i: i32, p: NodePtr<SaNode>) {
    q.set_child(mem, i, p.ptr());
    q.set_ref_count(mem, q.ref_count(mem) + 1);
}

/// Creates the missing trie levels below `q` for `levels`, ending in the
/// data record for type `t` and register `n`.
fn sa_alloc_chain(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    t: u8,
    n: i32,
    mut q: NodePtr<SaNode>,
    levels: &[i32],
) -> Option<NodePtr<SaNode>> {
    let (&leaf_i, index_levels) = levels.split_last().expect("non-empty trie path");
    for &i in index_levels {
        let p = new_index(mem, scanner, i as u16, q.ptr())?;
        add_sa_child(mem, q, i, p);
        q = p;
    }
    let p;
    if t == MARK_VAL {
        p = sa(get_node(mem, MARK_CLASS_NODE_SIZE)?);
        scanner.cur_ptr = p.ptr();
        for k in 1..=3 {
            mem.set_word(p.idx().offset(k), scanner.sa_null);
        }
    } else if t <= DIMEN_VAL {
        p = sa(get_node(mem, WORD_NODE_SIZE)?);
        scanner.cur_ptr = p.ptr();
        p.set_sa_int(mem, 0);
        // sa_num
        p.set_sa_ptr(mem, MemPtr::from_raw(n));
    } else {
        p = sa(get_node(mem, POINTER_NODE_SIZE)?);
        scanner.cur_ptr = p.ptr();
        if t <= MU_VAL {
            p.set_sa_ptr(mem, ZERO_GLUE.ptr());
            crate::mem::add_glue_ref(mem, ZERO_GLUE);
        } else {
            p.set_sa_ptr(mem, MemPtr::NULL);
        }
        p.set_sa_ref(mem, TEX_NULL);
    }
    p.set_sa_index(mem, (64 * t as i32 + leaf_i) as u16);
    p.set_sa_lev(mem, LEVEL_ONE);
    p.set_link(mem, q.ptr());
    add_sa_child(mem, q, leaf_i, p);
    Some(p)
}

fn sa_levels(n: i32) -> [i32; 4] {
    [n / 0x40000, (n / 4096) % 64, (n / 64) % 64, n % 64]
}

/// TeX's `find_sa_element` without creation: the record for type `t`,
/// register `n`, if any, also left in `cur_ptr`.
pub fn find_sa_element(
    mem: &MemState,
    scanner: &mut ScannerState,
    t: u8,
    n: i32,
) -> Option<NodePtr<SaNode>> {
    let mut p = scanner.sa_root[t as usize];
    for i in sa_levels(n) {
        if p.is_null() {
            break;
        }
        p = sa(p).child(mem, i);
    }
    scanner.cur_ptr = p;
    NodePtr::from_ptr(p)
}

/// TeX's `find_sa_element` with creation, also leaving the record in
/// `cur_ptr`. `None` when main memory is exhausted.
pub fn create_sa_element(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    t: u8,
    n: i32,
) -> Option<NodePtr<SaNode>> {
    let levels = sa_levels(n);
    scanner.cur_ptr = scanner.sa_root[t as usize];
    if scanner.cur_ptr.is_null() {
        let root = new_index(mem, scanner, t.into(), MemPtr::NULL)?;
        scanner.sa_root[t as usize] = root.ptr();
        return sa_alloc_chain(mem, scanner, t, n, root, &levels);
    }
    for k in 0..levels.len() {
        let q = sa(scanner.cur_ptr);
        scanner.cur_ptr = q.child(mem, levels[k]);
        if scanner.cur_ptr.is_null() {
            return sa_alloc_chain(mem, scanner, t, n, q, &levels[k..]);
        }
    }
    Some(sa(scanner.cur_ptr))
}
