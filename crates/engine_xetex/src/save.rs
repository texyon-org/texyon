//! The save stack, `eqtb` definitions, and the sparse-array register trie.
use crate::cmd::{BOX_REF, CALL, GLUE_REF, LONG_OUTER_CALL, REGISTER, SHAPE_REF, TOKS_REGISTER};
use crate::consts::{
    EQTB_SIZE, INDEX_NODE_SIZE, INT_BASE, INT_PAR_MAG, LEVEL_ONE, LEVEL_ZERO, MARK_CLASS_NODE_SIZE,
    TEX_NULL, UNDEFINED_CONTROL_SEQUENCE,
};
use crate::errors::Abort;
use crate::input::{LEFT_BRACE_LIMIT, RIGHT_BRACE_LIMIT};
use crate::mem::ZERO_GLUE;
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

impl EqtbState {
    pub(crate) fn eq_word_define(
        &mut self,
        save_stack: &mut SaveStackState,
        p: i32,
        w: i32,
    ) -> Result<(), Abort> {
        if self.int_at(p) == w {
            return Ok(());
        }
        let l = self.xeq_level_at(p);
        if l != save_stack.cur_level {
            save_stack.eq_save(self, p, l)?;
            self.set_xeq_level_at(p, save_stack.cur_level);
        }
        self.set_int_at(p, w);
        Ok(())
    }

    pub(crate) fn geq_word_define(&mut self, p: i32, w: i32) {
        self.set_int_at(p, w);
        self.set_xeq_level_at(p, LEVEL_ONE);
    }
}

impl SaveStackState {
    /// TeX's `check_full_save_stack`, true when no room is left.
    fn save_stack_full(&mut self) -> bool {
        if self.save_ptr > self.max_save_stack {
            self.max_save_stack = self.save_ptr;
            return self.max_save_stack > self.save_size - 7;
        }
        false
    }

    pub(crate) fn new_save_level(&mut self, c: GroupCode, line: i32) -> Result<(), Abort> {
        if self.save_stack_full() {
            return Err(Abort::SaveOverflow);
        }
        self.set_saved(0, line);
        self.save_ptr += 1;
        let (group, boundary) = (self.cur_group.as_raw() as u16, self.cur_boundary);
        self.set_save_entry(LEVEL_BOUNDARY, group, boundary);
        if self.cur_level == u16::MAX {
            return Err(Abort::Overflow {
                what: "grouping levels",
                n: u16::MAX as i32,
            });
        }
        self.cur_boundary = self.save_ptr;
        self.cur_group = c;
        self.cur_level += 1;
        self.save_ptr += 1;
        Ok(())
    }

    pub(crate) fn eq_save(&mut self, eqtb: &EqtbState, p: i32, l: u16) -> Result<(), Abort> {
        if self.save_stack_full() {
            return Err(Abort::SaveOverflow);
        }
        let kind = if l == LEVEL_ZERO {
            RESTORE_ZERO
        } else {
            self.set_word(self.save_ptr, eqtb.word(p));
            self.save_ptr += 1;
            RESTORE_OLD_VALUE
        };
        self.set_save_entry(kind, l, p);
        self.save_ptr += 1;
        Ok(())
    }

    pub(crate) fn save_for_after(&mut self, t: i32) -> Result<(), Abort> {
        if self.cur_level > LEVEL_ONE {
            if self.save_stack_full() {
                return Err(Abort::SaveOverflow);
            }
            self.set_save_entry(INSERT_TOKEN, LEVEL_ZERO, t);
            self.save_ptr += 1;
        }
        Ok(())
    }
}

/// Validates `\mag`, propagating a non-`Continue` error outcome.
pub(crate) fn prepare_mag(st: &mut EngineState) -> Result<(), Abort> {
    use crate::output::{capture_to_diagnostic, error_here_with_diagnostic};

    let mag = st.eqtb.int_par(INT_PAR_MAG);
    if st.scanner.mag_set > 0 && mag != st.scanner.mag_set {
        let mag_set = st.scanner.mag_set;
        error_here_with_diagnostic(st, "Incompatible magnification (");
        crate::output::print_int(st, mag);
        crate::output::print(st, ");");
        crate::output::print_nl(st, " the previous value will be retained");
        crate::output::append_to_diagnostic(st, &format!(" ({})", mag_set));
        capture_to_diagnostic(st, None);
        st.errors.help(&[
            "I can handle only one magnification ratio per job. So I've",
            "reverted to the magnification you used earlier on this run.",
        ]);
        crate::errors::int_error(st, mag_set)?;
        st.eqtb.geq_word_define(INT_BASE + INT_PAR_MAG, mag_set);
    }
    let mag = st.eqtb.int_par(INT_PAR_MAG);
    if mag <= 0 || mag > 32768 {
        error_here_with_diagnostic(st, "Illegal magnification has been changed to 1000");
        crate::output::append_to_diagnostic(st, &format!(" ({})", mag));
        capture_to_diagnostic(st, None);
        st.errors
            .help(&["The magnification ratio must be between 1 and 32768."]);
        crate::errors::int_error(st, mag)?;
        st.eqtb.geq_word_define(INT_BASE + INT_PAR_MAG, 1000);
    }
    st.scanner.mag_set = st.eqtb.int_par(INT_PAR_MAG);
    Ok(())
}

impl EqtbState {
    pub(crate) fn eq_define(
        &mut self,
        save_stack: &mut SaveStackState,
        mem: &mut MemState,
        scanner: &mut ScannerState,
        p: i32,
        t: u8,
        e: i32,
    ) -> Result<(), Abort> {
        if self.eq_type(p) == t && self.equiv(p) == e {
            return mem.eq_destroy(scanner, self.word(p));
        }
        if self.eq_level(p) == save_stack.cur_level {
            mem.eq_destroy(scanner, self.word(p))?;
        } else if save_stack.cur_level > LEVEL_ONE {
            save_stack.eq_save(self, p, self.eq_level(p))?;
        }
        self.set_eq_level(p, save_stack.cur_level);
        self.set_eq_type(p, t);
        self.set_equiv(p, e);
        Ok(())
    }

    pub(crate) fn geq_define(
        &mut self,
        mem: &mut MemState,
        scanner: &mut ScannerState,
        p: i32,
        t: u8,
        e: i32,
    ) -> Result<(), Abort> {
        mem.eq_destroy(scanner, self.word(p))?;
        self.set_eq_level(p, LEVEL_ONE);
        self.set_eq_type(p, t);
        self.set_equiv(p, e);
        Ok(())
    }
}

/// A sparse-array record. These carry no type tag and are reached only
/// through the trie or a register's `eqtb` entry.
#[inline]
fn sa(p: impl MemAddr) -> NodePtr<SaNode> {
    NodePtr::at(p)
}

/// Deferred flush work from an `sa_def`/`sa_w_def`-family mutation.
#[derive(Default)]
pub(crate) struct SaMutationResult {
    flush_list: Option<i32>,
}

impl MemState {
    /// Frees the heap resource an `eqtb` entry owns.
    pub(crate) fn eq_destroy(
        &mut self,
        scanner: &mut ScannerState,
        w: MemoryWord,
    ) -> Result<(), Abort> {
        // SAFETY: `MemoryWord` union projections of an `eqtb` entry.
        let (eq_type, equiv) = unsafe { (w.b16.s1, w.b32.s1) };
        let Ok(eq_type) = u8::try_from(eq_type) else {
            return Ok(());
        };
        match eq_type {
            CALL..=LONG_OUTER_CALL => self.delete_token_ref(MemPtr(equiv)),
            GLUE_REF => self.delete_glue_ref(crate::node::NodePtr::from_glue_ptr(MemPtr(equiv))),
            SHAPE_REF => {
                if equiv != TEX_NULL {
                    let size = 2 * MemPtr(equiv).info(self) + 1;
                    self.free_node(MemPtr::from_raw(equiv), size);
                }
            }
            BOX_REF => self.flush_node_list(equiv)?,
            TOKS_REGISTER | REGISTER => {
                if !(0..=19).contains(&equiv) {
                    self.delete_sa_ref(scanner, equiv);
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn delete_sa_ref(&mut self, scanner: &mut ScannerState, q: impl MemAddr) {
        let mut q = sa(q);
        q.set_sa_ref(self, q.sa_ref(self) - 1);
        if q.sa_ref(self) != TEX_NULL {
            return;
        }
        let mut s = if q.sa_index(self) < DIMEN_VAL_LIMIT {
            if q.sa_int(self) != 0 {
                return;
            }
            WORD_NODE_SIZE
        } else {
            if q.sa_index(self) < MU_VAL_LIMIT {
                if q.sa_ptr(self) != ZERO_GLUE.ptr() {
                    return;
                }
                self.delete_glue_ref(ZERO_GLUE);
            } else if !q.sa_ptr(self).is_null() {
                return;
            }
            POINTER_NODE_SIZE
        };
        loop {
            let i = (q.sa_index(self) % 64) as i32;
            let next = q.link(self);
            self.free_node(q.ptr(), s);
            if next.is_null() {
                scanner.sa_root[i as usize] = MemPtr::NULL;
                return;
            }
            q = sa(next);
            q.set_child(self, i, MemPtr::NULL);
            q.set_ref_count(self, q.ref_count(self) - 1);
            s = INDEX_NODE_SIZE;
            if q.ref_count(self) > 0 {
                break;
            }
        }
    }

    pub fn sa_save(
        &mut self,
        scanner: &mut ScannerState,
        save_stack: &mut SaveStackState,
        p: i32,
    ) -> Result<(), Abort> {
        if save_stack.cur_level != scanner.sa_level {
            if save_stack.save_stack_full() {
                return Err(Abort::SaveOverflow);
            }
            save_stack.set_save_entry(RESTORE_SA, scanner.sa_level, scanner.sa_chain.0);
            save_stack.save_ptr += 1;
            scanner.sa_chain = MemPtr::NULL;
            scanner.sa_level = save_stack.cur_level;
        }
        let p = sa(p);
        let mut i = p.sa_index(self);
        let q;
        if i < DIMEN_VAL_LIMIT {
            if p.sa_int(self) == 0 {
                q = sa(self.get_node(POINTER_NODE_SIZE)?);
                i = TOK_VAL_LIMIT;
            } else {
                q = sa(self.get_node(WORD_NODE_SIZE)?);
                let v = p.sa_int(self);
                q.set_sa_int(self, v);
            }
            q.set_sa_ptr(self, MemPtr::NULL);
        } else {
            q = sa(self.get_node(POINTER_NODE_SIZE)?);
            let v = p.sa_ptr(self);
            q.set_sa_ptr(self, v);
        }
        // sa_loc
        q.set_sa_ref(self, p.idx().0);
        q.set_sa_index(self, i);
        let lev = p.sa_lev(self);
        q.set_sa_lev(self, lev);
        q.set_link(self, scanner.sa_chain);
        scanner.sa_chain = q.ptr();
        p.set_sa_ref(self, p.sa_ref(self) + 1);
        Ok(())
    }

    /// Releases the value of `p`, returning a box list the caller must flush.
    pub(crate) fn sa_destroy(&mut self, p: impl MemAddr) -> Option<i32> {
        let p = sa(p);
        let value = p.sa_ptr(self);
        if p.sa_index(self) < MU_VAL_LIMIT {
            self.delete_glue_ref(crate::node::NodePtr::from_glue_ptr(value));
        } else if !value.is_null() {
            if p.sa_index(self) < BOX_VAL_LIMIT {
                return Some(value.0);
            }
            self.delete_token_ref(value);
        }
        None
    }

    pub(crate) fn sa_def(
        &mut self,
        scanner: &mut ScannerState,
        save_stack: &mut SaveStackState,
        p: i32,
        e: i32,
    ) -> Result<SaMutationResult, Abort> {
        let node = sa(p);
        node.set_sa_ref(self, node.sa_ref(self) + 1);
        let mut result = SaMutationResult::default();
        if node.sa_ptr(self).0 == e {
            result.flush_list = self.sa_destroy(p);
        } else {
            if node.sa_lev(self) == save_stack.cur_level {
                result.flush_list = self.sa_destroy(p);
            } else {
                self.sa_save(scanner, save_stack, p)?;
            }
            node.set_sa_lev(self, save_stack.cur_level);
            node.set_sa_ptr(self, MemPtr::from_raw(e));
        }
        Ok(result)
    }

    pub(crate) fn sa_w_def(
        &mut self,
        scanner: &mut ScannerState,
        save_stack: &mut SaveStackState,
        p: i32,
        w: i32,
    ) -> Result<SaMutationResult, Abort> {
        let node = sa(p);
        node.set_sa_ref(self, node.sa_ref(self) + 1);
        if node.sa_int(self) != w {
            if node.sa_lev(self) != save_stack.cur_level {
                self.sa_save(scanner, save_stack, p)?;
            }
            node.set_sa_lev(self, save_stack.cur_level);
            node.set_sa_int(self, w);
        }
        Ok(SaMutationResult::default())
    }

    pub(crate) fn gsa_def(&mut self, p: impl MemAddr, e: i32) -> SaMutationResult {
        let p = sa(p);
        p.set_sa_ref(self, p.sa_ref(self) + 1);
        let flush_list = self.sa_destroy(p.idx());
        p.set_sa_lev(self, LEVEL_ONE);
        p.set_sa_ptr(self, MemPtr::from_raw(e));
        SaMutationResult { flush_list }
    }

    pub(crate) fn gsa_w_def(&mut self, p: impl MemAddr, w: i32) -> SaMutationResult {
        let p = sa(p);
        p.set_sa_ref(self, p.sa_ref(self) + 1);
        p.set_sa_lev(self, LEVEL_ONE);
        p.set_sa_int(self, w);
        SaMutationResult::default()
    }
}

/// Applies the deferred flush and reference drop of an `sa_def`-family result.
pub(crate) fn finish_sa_mutation(
    st: &mut EngineState,
    p: i32,
    result: SaMutationResult,
) -> Result<(), Abort> {
    if let Some(list) = result.flush_list {
        st.mem.flush_node_list(list)?;
    }
    st.mem.delete_sa_ref(&mut st.scanner, p);
    Ok(())
}

fn sa_restore(st: &mut EngineState) -> Result<(), Abort> {
    loop {
        let chain = sa(st.scanner.sa_chain);
        let p = sa(chain.sa_ref(&st.mem));
        let mut flush_list = None;
        if p.sa_lev(&st.mem) == LEVEL_ONE {
            if p.sa_index(&st.mem) >= DIMEN_VAL_LIMIT {
                flush_list = st.mem.sa_destroy(chain.idx());
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
                flush_list = st.mem.sa_destroy(p.idx());
                let v = chain.sa_ptr(&st.mem);
                p.set_sa_ptr(&mut st.mem, v);
            }
            let lev = chain.sa_lev(&st.mem);
            p.set_sa_lev(&mut st.mem, lev);
        }
        if let Some(list) = flush_list {
            st.mem.flush_node_list(list)?;
        }
        st.mem.delete_sa_ref(&mut st.scanner, p.idx());
        st.scanner.sa_chain = chain.link(&st.mem);
        let size = if chain.sa_index(&st.mem) < DIMEN_VAL_LIMIT {
            WORD_NODE_SIZE
        } else {
            POINTER_NODE_SIZE
        };
        st.mem.free_node(chain.ptr(), size);
        if st.scanner.sa_chain.is_null() {
            return Ok(());
        }
    }
}

pub(crate) fn unsave(st: &mut EngineState) -> Result<(), Abort> {
    if st.save_stack.cur_level <= LEVEL_ONE {
        return Err(Abort::Confusion("curlevel"));
    }
    st.save_stack.cur_level -= 1;
    let mut l: u16 = 0;
    let mut a = false;
    loop {
        st.save_stack.save_ptr -= 1;
        let sp = st.save_stack.save_ptr;
        let p = st.save_stack.save_index(sp);
        match st.save_stack.save_type(sp) {
            LEVEL_BOUNDARY => break,
            INSERT_TOKEN => {
                let t = st.scanner.cur_tok;
                st.scanner.cur_tok = p;
                if a {
                    let pn = st.mem.get_avail()?;
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
                    crate::input::back_input(st)?;
                    a = true;
                }
                st.scanner.cur_tok = t;
            }
            RESTORE_SA => {
                sa_restore(st)?;
                st.scanner.sa_chain = MemPtr::from_raw(p);
                st.scanner.sa_level = st.save_stack.save_level(st.save_stack.save_ptr);
            }
            kind => {
                if kind == RESTORE_OLD_VALUE {
                    l = st.save_stack.save_level(sp);
                    st.save_stack.save_ptr -= 1;
                } else {
                    let w = st.eqtb.word(UNDEFINED_CONTROL_SEQUENCE);
                    st.save_stack.set_word(st.save_stack.save_ptr, w);
                }
                let saved = st.save_stack.word(st.save_stack.save_ptr);
                if !(INT_BASE..=EQTB_SIZE).contains(&p) {
                    if st.eqtb.eq_level(p) == LEVEL_ONE {
                        st.mem.eq_destroy(&mut st.scanner, saved)?;
                    } else {
                        st.mem.eq_destroy(&mut st.scanner, st.eqtb.word(p))?;
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
        crate::output::group_warning(st);
    }
    let ss = &mut st.save_stack;
    (ss.cur_group, ss.cur_boundary) = ss.boundary_at(ss.save_ptr);
    ss.save_ptr -= 1;
    Ok(())
}

fn sa_levels(n: i32) -> [i32; 4] {
    [n / 0x40000, (n / 4096) % 64, (n / 64) % 64, n % 64]
}

impl MemState {
    /// Allocates a trie index node below `q`, also leaving it in `cur_ptr`.
    fn new_index(
        &mut self,
        scanner: &mut ScannerState,
        i: u16,
        q: MemPtr,
    ) -> Result<NodePtr<SaNode>, Abort> {
        let p = sa(self.get_node(INDEX_NODE_SIZE)?);
        scanner.cur_ptr = p.ptr();
        p.set_sa_index(self, i);
        p.set_ref_count(self, 0);
        p.set_link(self, q);
        for k in 1..INDEX_NODE_SIZE {
            self.set_word(p.idx().offset(k), scanner.sa_null);
        }
        Ok(p)
    }

    fn add_sa_child(&mut self, q: NodePtr<SaNode>, i: i32, p: NodePtr<SaNode>) {
        q.set_child(self, i, p.ptr());
        q.set_ref_count(self, q.ref_count(self) + 1);
    }

    /// Creates the missing trie levels below `q` for `levels`, ending in the
    /// data record for type `t` and register `n`.
    fn sa_alloc_chain(
        &mut self,
        scanner: &mut ScannerState,
        t: u8,
        n: i32,
        mut q: NodePtr<SaNode>,
        levels: &[i32],
    ) -> Result<NodePtr<SaNode>, Abort> {
        let (&leaf_i, index_levels) = levels.split_last().expect("non-empty trie path");
        for &i in index_levels {
            let p = self.new_index(scanner, i as u16, q.ptr())?;
            self.add_sa_child(q, i, p);
            q = p;
        }
        let p;
        if t == MARK_VAL {
            p = sa(self.get_node(MARK_CLASS_NODE_SIZE)?);
            scanner.cur_ptr = p.ptr();
            for k in 1..=3 {
                self.set_word(p.idx().offset(k), scanner.sa_null);
            }
        } else if t <= DIMEN_VAL {
            p = sa(self.get_node(WORD_NODE_SIZE)?);
            scanner.cur_ptr = p.ptr();
            p.set_sa_int(self, 0);
            // sa_num
            p.set_sa_ptr(self, MemPtr::from_raw(n));
        } else {
            p = sa(self.get_node(POINTER_NODE_SIZE)?);
            scanner.cur_ptr = p.ptr();
            if t <= MU_VAL {
                p.set_sa_ptr(self, ZERO_GLUE.ptr());
                self.add_glue_ref(ZERO_GLUE);
            } else {
                p.set_sa_ptr(self, MemPtr::NULL);
            }
            p.set_sa_ref(self, TEX_NULL);
        }
        p.set_sa_index(self, (64 * t as i32 + leaf_i) as u16);
        p.set_sa_lev(self, LEVEL_ONE);
        p.set_link(self, q.ptr());
        self.add_sa_child(q, leaf_i, p);
        Ok(p)
    }

    /// TeX's `find_sa_element` without creation: the record for type `t`,
    /// register `n`, if any, also left in `cur_ptr`.
    pub fn find_sa_element(
        &self,
        scanner: &mut ScannerState,
        t: u8,
        n: i32,
    ) -> Option<NodePtr<SaNode>> {
        let mut p = scanner.sa_root[t as usize];
        for i in sa_levels(n) {
            if p.is_null() {
                break;
            }
            p = sa(p).child(self, i);
        }
        scanner.cur_ptr = p;
        NodePtr::from_ptr(p)
    }

    /// TeX's `find_sa_element` with creation, also leaving the record in
    /// `cur_ptr`. `None` when main memory is exhausted.
    pub fn create_sa_element(
        &mut self,
        scanner: &mut ScannerState,
        t: u8,
        n: i32,
    ) -> Result<NodePtr<SaNode>, Abort> {
        let levels = sa_levels(n);
        scanner.cur_ptr = scanner.sa_root[t as usize];
        if scanner.cur_ptr.is_null() {
            let root = self.new_index(scanner, t.into(), MemPtr::NULL)?;
            scanner.sa_root[t as usize] = root.ptr();
            return self.sa_alloc_chain(scanner, t, n, root, &levels);
        }
        for k in 0..levels.len() {
            let q = sa(scanner.cur_ptr);
            scanner.cur_ptr = q.child(self, levels[k]);
            if scanner.cur_ptr.is_null() {
                return self.sa_alloc_chain(scanner, t, n, q, &levels[k..]);
            }
        }
        Ok(sa(scanner.cur_ptr))
    }
}
