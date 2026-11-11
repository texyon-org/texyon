//! The save stack, `eqtb` definitions, and the sparse-array register trie.
use crate::codes::ValueLevel;
use crate::command::Command;
use crate::command::RegisterRef;
use crate::consts::{LEVEL_ONE, LEVEL_ZERO, TEX_NULL, UNDEFINED_CONTROL_SEQUENCE};
use crate::errors::Abort;
use crate::errors::OrConfusion;
use crate::glue_store::GlueSpecId;
use crate::mem::ZERO_GLUE;
use crate::node::{
    NodePtr, SaChild, SaElement, SaIndexNode, SaLeaf, SaMarkClassNode, SaPointerNode, SaSavedEntry,
    SaSavedPointer, SaSavedWord, SaWordNode, View,
};
use crate::params::IntPar;
use crate::state::{
    EngineState, EqtbState, GroupCode, MemState, Restore, SaveEntry, SaveStackState, ScannerState,
};
use crate::token::{Token, TokenList};
use crate::units::CsId;
use crate::units::MemIdx;
use crate::units::MemPtr;
use crate::units::RawPtr;

pub(crate) const DIMEN_VAL_LIMIT: u16 = 128;
pub(crate) const MU_VAL_LIMIT: u16 = 256;
pub(crate) const BOX_VAL_LIMIT: u16 = 320;

impl EqtbState {
    /// Gives `cs` the meaning `m` in the current group.
    pub(crate) fn eq_define_cs(
        &mut self,
        save_stack: &mut SaveStackState,
        mem: &mut MemState,
        scanner: &mut ScannerState,
        cs: CsId,
        m: Command,
    ) -> Result<(), Abort> {
        let old = self.meaning(cs);
        if old == m {
            return mem.destroy_meaning(scanner, old);
        }
        let level = self.cs_level(cs);
        if level == save_stack.cur_level {
            mem.destroy_meaning(scanner, old)?;
        } else if save_stack.cur_level > LEVEL_ONE {
            save_stack.eq_save_cs(self, cs, level)?;
        }
        self.set_meaning(cs, m, save_stack.cur_level);
        Ok(())
    }

    /// Gives `cs` the meaning `m` globally.
    pub(crate) fn geq_define_cs(
        &mut self,
        mem: &mut MemState,
        scanner: &mut ScannerState,
        cs: CsId,
        m: Command,
    ) -> Result<(), Abort> {
        mem.destroy_meaning(scanner, self.meaning(cs))?;
        self.set_meaning(cs, m, LEVEL_ONE);
        Ok(())
    }
}

impl SaveStackState {
    /// TeX's `check_full_save_stack`, true when no room is left.
    pub(crate) fn save_stack_full(&mut self) -> bool {
        if self.save_ptr > self.max_save_stack {
            self.max_save_stack = self.save_ptr;
            return self.max_save_stack.index() + 7 > self.save_size;
        }
        false
    }

    pub(crate) fn new_save_level(&mut self, c: GroupCode, line: i32) -> Result<(), Abort> {
        if self.save_stack_full() {
            return Err(self.overflow());
        }
        self.data[self.save_ptr.index()] = SaveEntry::Boundary {
            group: self.cur_group,
            below: self.cur_boundary,
            line,
        };
        if self.cur_level == u16::MAX {
            return Err(Abort::Overflow {
                capacity: crate::errors::Capacity::GroupingLevels,
                n: i32::from(u16::MAX),
            });
        }
        self.cur_boundary = Some(self.save_ptr);
        self.cur_group = c;
        self.cur_level += 1;
        self.save_ptr += 1;
        Ok(())
    }

    /// Saves the meaning `cs` has at level `l`, to be restored when the group ends.
    pub(crate) fn eq_save_cs(&mut self, eqtb: &EqtbState, cs: CsId, l: u16) -> Result<(), Abort> {
        if self.save_stack_full() {
            return Err(self.overflow());
        }
        self.push(SaveEntry::Restore(if l == LEVEL_ZERO {
            Restore::CsUndefined(cs)
        } else {
            Restore::Cs {
                cs,
                old: eqtb.meaning(cs),
                level: l,
            }
        }));
        Ok(())
    }

    pub(crate) fn save_for_after(&mut self, t: Token) -> Result<(), Abort> {
        if self.cur_level > LEVEL_ONE {
            if self.save_stack_full() {
                return Err(self.overflow());
            }
            self.push(SaveEntry::InsertToken(t));
        }
        Ok(())
    }
}

/// Validates `\mag`, propagating a non-`Continue` error outcome.
pub(crate) fn prepare_mag(st: &mut EngineState) -> Result<(), Abort> {
    let mag = st.eqtb.int_par(IntPar::Mag);
    if st.scanner.mag_set > 0 && mag != st.scanner.mag_set {
        let mag_set = st.scanner.mag_set;
        crate::errors::error(
            st,
            (
                "Incompatible magnification (",
                mag,
                ");",
                crate::output::Nl(" the previous value will be retained"),
                " (",
                mag_set,
                ")",
            ),
            &[
                "I can handle only one magnification ratio per job. So I've",
                "reverted to the magnification you used earlier on this run.",
            ],
        )?;
        st.eqtb.ints.set(IntPar::Mag.loc(), mag_set, LEVEL_ONE);
    }
    let mag = st.eqtb.int_par(IntPar::Mag);
    if mag <= 0 || mag > 32768 {
        crate::errors::error(
            st,
            (
                "Illegal magnification has been changed to 1000",
                " (",
                mag,
                ")",
            ),
            &["The magnification ratio must be between 1 and 32768."],
        )?;
        st.eqtb.ints.set(IntPar::Mag.loc(), 1000, LEVEL_ONE);
    }
    st.scanner.mag_set = st.eqtb.int_par(IntPar::Mag);
    Ok(())
}

/// Deferred flush work from an `sa_def`/`sa_w_def`-family mutation.
#[derive(Default)]
pub(crate) struct SaMutationResult {
    flush_list: Option<MemPtr>,
}

impl MemState {
    /// Drops what a control sequence's meaning holds: a macro's reference, or a sparse register's.
    pub(crate) fn destroy_meaning(
        &mut self,
        scanner: &mut ScannerState,
        m: Command,
    ) -> Result<(), Abort> {
        if let Some(body) = m.macro_body() {
            self.tokens.delete_ref(body);
            return Ok(());
        }
        let sparse = match m {
            Command::ToksRegister(r) => r.get().map(NodePtr::at),
            Command::Register(r) => match r.get()? {
                RegisterRef::Sparse(sa) => Some(sa),
                RegisterRef::Scanned(_) => None,
            },
            _ => None,
        };
        if let Some(sa) = sparse {
            self.delete_sa_ref(scanner, sa)?;
        }
        Ok(())
    }

    /// Drops a reference to register `q`. Once it is unreferenced and back
    /// at its default value, frees it and the index nodes it leaves empty.
    pub fn delete_sa_ref(
        &mut self,
        scanner: &mut ScannerState,
        q: NodePtr<SaElement>,
    ) -> Result<(), Abort> {
        let size = match q.leaf(self).or_confusion("sa")? {
            SaLeaf::Word(w) => {
                let n = &mut self[w];
                n.sa_ref -= 1;
                if n.sa_ref != TEX_NULL || n.sa_int != 0 {
                    return Ok(());
                }
                SaWordNode::SIZE
            }
            SaLeaf::Pointer(p) => {
                let n = &mut self[p];
                n.sa_ref -= 1;
                if n.sa_ref != TEX_NULL {
                    return Ok(());
                }
                if n.sa_index < MU_VAL_LIMIT {
                    if n.sa_ptr.raw() != ZERO_GLUE.raw() {
                        return Ok(());
                    }
                    self.delete_glue_ref(ZERO_GLUE);
                } else if !n.sa_ptr.is_none() {
                    return Ok(());
                }
                SaPointerNode::SIZE
            }
            SaLeaf::MarkClass(_) => return Err(Abort::Confusion("sa")),
        };
        let mut slot = self[q].slot();
        let mut parent = self[q].link.or_confusion("sa")?;
        self.free_node(q.idx(), size);
        loop {
            let n = &mut self[parent];
            n.clear_child(slot);
            n.used -= 1;
            if n.used > 0 {
                return Ok(());
            }
            let (up_slot, up) = (n.sa_index % 64, n.link);
            parent.free(self);
            match up {
                Some(up) => (slot, parent) = (usize::from(up_slot), up),
                None => {
                    let root_type = ValueLevel::from_raw(i32::from(up_slot)).or_confusion("sa")?;
                    scanner.sa_root.set(root_type, None);
                    return Ok(());
                }
            }
        }
    }

    pub fn sa_save_word(
        &mut self,
        scanner: &mut ScannerState,
        save_stack: &mut SaveStackState,
        p: NodePtr<SaWordNode>,
    ) -> Result<(), Abort> {
        start_sa_chain(scanner, save_stack)?;
        let q = NodePtr::<SaSavedWord>::alloc(self)?;
        let reg = self[p];
        let e = &mut self[q];
        e.sa_lev = reg.sa_lev;
        e.sa_index = reg.sa_index;
        e.link = scanner.sa_chain;
        e.sa_loc = Some(p);
        e.sa_int = reg.sa_int;
        scanner.sa_chain = Some(q.saved());
        self[p].sa_ref += 1;
        Ok(())
    }

    pub fn sa_save_pointer(
        &mut self,
        scanner: &mut ScannerState,
        save_stack: &mut SaveStackState,
        p: NodePtr<SaPointerNode>,
    ) -> Result<(), Abort> {
        start_sa_chain(scanner, save_stack)?;
        let q = NodePtr::<SaSavedPointer>::alloc(self)?;
        let reg = self[p];
        let e = &mut self[q];
        e.sa_lev = reg.sa_lev;
        e.sa_index = reg.sa_index;
        e.link = scanner.sa_chain;
        e.sa_loc = Some(p);
        e.sa_ptr = reg.sa_ptr;
        scanner.sa_chain = Some(q.saved());
        self[p].sa_ref += 1;
        Ok(())
    }

    /// Releases a pointer register's `value`, returning a box list the caller
    /// must flush.
    pub(crate) fn sa_destroy(&mut self, sa_index: u16, value: MemPtr) -> Option<MemPtr> {
        if sa_index < MU_VAL_LIMIT {
            self.delete_glue_ref(GlueSpecId::from_raw(value.raw()));
        } else if !value.is_none() {
            if sa_index < BOX_VAL_LIMIT {
                return Some(value);
            }
            self.tokens.delete_ref(TokenList::from_raw(value.raw()));
        }
        None
    }

    pub(crate) fn sa_def(
        &mut self,
        scanner: &mut ScannerState,
        save_stack: &mut SaveStackState,
        node: NodePtr<SaPointerNode>,
        e: MemPtr,
    ) -> Result<SaMutationResult, Abort> {
        self[node].sa_ref += 1;
        let mut result = SaMutationResult::default();
        let reg = self[node];
        if reg.sa_ptr == e {
            result.flush_list = self.sa_destroy(reg.sa_index, reg.sa_ptr);
        } else {
            if reg.sa_lev == save_stack.cur_level {
                result.flush_list = self.sa_destroy(reg.sa_index, reg.sa_ptr);
            } else {
                self.sa_save_pointer(scanner, save_stack, node)?;
            }
            let n = &mut self[node];
            n.sa_lev = save_stack.cur_level;
            n.sa_ptr = e;
        }
        Ok(result)
    }

    pub(crate) fn sa_w_def(
        &mut self,
        scanner: &mut ScannerState,
        save_stack: &mut SaveStackState,
        node: NodePtr<SaWordNode>,
        w: i32,
    ) -> Result<SaMutationResult, Abort> {
        self[node].sa_ref += 1;
        let reg = self[node];
        if reg.sa_int != w {
            if reg.sa_lev != save_stack.cur_level {
                self.sa_save_word(scanner, save_stack, node)?;
            }
            let n = &mut self[node];
            n.sa_lev = save_stack.cur_level;
            n.sa_int = w;
        }
        Ok(SaMutationResult::default())
    }

    pub(crate) fn gsa_def(&mut self, p: NodePtr<SaPointerNode>, e: MemPtr) -> SaMutationResult {
        self[p].sa_ref += 1;
        let reg = self[p];
        let flush_list = self.sa_destroy(reg.sa_index, reg.sa_ptr);
        let n = &mut self[p];
        n.sa_lev = LEVEL_ONE;
        n.sa_ptr = e;
        SaMutationResult { flush_list }
    }

    pub(crate) fn gsa_w_def(&mut self, p: NodePtr<SaWordNode>, w: i32) -> SaMutationResult {
        let n = &mut self[p];
        n.sa_ref += 1;
        n.sa_lev = LEVEL_ONE;
        n.sa_int = w;
        SaMutationResult::default()
    }
}

/// Opens a new save-chain level when the group level moved since the last save.
fn start_sa_chain(
    scanner: &mut ScannerState,
    save_stack: &mut SaveStackState,
) -> Result<(), Abort> {
    if save_stack.cur_level != scanner.sa_level {
        if save_stack.save_stack_full() {
            return Err(save_stack.overflow());
        }
        save_stack.push(SaveEntry::RestoreSa {
            chain: scanner.sa_chain,
            level: scanner.sa_level,
        });
        scanner.sa_chain = None;
        scanner.sa_level = save_stack.cur_level;
    }
    Ok(())
}

/// Applies the deferred flush and reference drop of an `sa_def`-family result.
pub(crate) fn finish_sa_mutation(
    st: &mut EngineState,
    p: NodePtr<SaElement>,
    result: SaMutationResult,
) -> Result<(), Abort> {
    if let Some(list) = result.flush_list {
        st.mem.flush_node_list(list)?;
    }
    st.mem.delete_sa_ref(&mut st.scanner, p)
}

fn sa_restore(st: &mut EngineState) -> Result<(), Abort> {
    while let Some(chain) = st.scanner.sa_chain {
        let next = st.mem[chain].link;
        match chain.entry(&st.mem) {
            SaSavedEntry::Word(c) => {
                let saved = st.mem[c];
                let p = saved.sa_loc.or_confusion("sa")?;
                if st.mem[p].sa_lev != LEVEL_ONE {
                    let n = &mut st.mem[p];
                    n.sa_int = saved.sa_int;
                    n.sa_lev = saved.sa_lev;
                }
                st.mem.delete_sa_ref(&mut st.scanner, p.element())?;
                c.free(&mut st.mem);
            }
            SaSavedEntry::Pointer(c) => {
                let saved = st.mem[c];
                let p = saved.sa_loc.or_confusion("sa")?;
                let reg = st.mem[p];
                let flush_list = if reg.sa_lev == LEVEL_ONE {
                    st.mem.sa_destroy(saved.sa_index, saved.sa_ptr)
                } else {
                    let flush_list = st.mem.sa_destroy(reg.sa_index, reg.sa_ptr);
                    let n = &mut st.mem[p];
                    n.sa_ptr = saved.sa_ptr;
                    n.sa_lev = saved.sa_lev;
                    flush_list
                };
                if let Some(list) = flush_list {
                    st.mem.flush_node_list(list)?;
                }
                st.mem.delete_sa_ref(&mut st.scanner, p.element())?;
                c.free(&mut st.mem);
            }
        }
        st.scanner.sa_chain = next;
    }
    Ok(())
}

pub(crate) fn unsave(st: &mut EngineState) -> Result<(), Abort> {
    if st.save_stack.cur_level <= LEVEL_ONE {
        return Err(Abort::Confusion("curlevel"));
    }
    st.save_stack.cur_level -= 1;
    let mut a = false;
    loop {
        st.save_stack.save_ptr -= 1;
        match st.save_stack.entry(st.save_stack.save_ptr) {
            SaveEntry::Boundary { .. } => break,
            SaveEntry::Saved(_) => return Err(Abort::Confusion("unsave")),
            SaveEntry::InsertToken(t) => {
                let cur = st.scanner.cur_tok;
                st.scanner.cur_tok = t;
                if a {
                    crate::input::prepend_to_backed_up(st, t)?;
                    if t.is_brace() {
                        if t.is_left_brace() {
                            st.scanner.align_state -= 1;
                        } else {
                            st.scanner.align_state += 1;
                        }
                    }
                } else {
                    crate::input::back_input(st)?;
                    a = true;
                }
                st.scanner.cur_tok = cur;
            }
            SaveEntry::RestoreSa { chain, level } => {
                sa_restore(st)?;
                st.scanner.sa_chain = chain;
                st.scanner.sa_level = level;
            }
            SaveEntry::Restore(Restore::Cs { cs, old, level }) => restore_cs(st, cs, old, level)?,
            SaveEntry::Restore(Restore::CsUndefined(cs)) => {
                let old = st.eqtb.meaning(UNDEFINED_CONTROL_SEQUENCE);
                let level = st.eqtb.cs_level(UNDEFINED_CONTROL_SEQUENCE);
                restore_cs(st, cs, old, level)?;
            }
            SaveEntry::Restore(Restore::Int(old)) => crate::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Dimen(old)) => crate::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Glue(old)) => crate::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Toks(old)) => crate::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Shape(old)) => crate::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Box(old)) => crate::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Font(old)) => crate::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Cat(old)) => crate::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Lc(old)) => crate::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Uc(old)) => crate::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Sf(old)) => crate::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Math(old)) => crate::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Del(old)) => crate::eqtb::restore(st, old)?,
        }
    }
    if st.input.open_levels[st.input.in_open].grp == st.save_stack.cur_boundary {
        crate::output::group_warning(st);
    }
    let ss = &mut st.save_stack;
    let (group, below, _) = ss.boundary_at(ss.save_ptr).or_confusion("unsave")?;
    (ss.cur_group, ss.cur_boundary) = (group, below);
    Ok(())
}

/// Puts back the meaning `cs` had before the group, unless it was defined globally since.
fn restore_cs(st: &mut EngineState, cs: CsId, old: Command, level: u16) -> Result<(), Abort> {
    if st.eqtb.cs_level(cs) == LEVEL_ONE {
        st.mem.destroy_meaning(&mut st.scanner, old)
    } else {
        st.mem
            .destroy_meaning(&mut st.scanner, st.eqtb.meaning(cs))?;
        st.eqtb.set_meaning(cs, old, level);
        Ok(())
    }
}

/// The four 6-bit trie slots of register `n`, from the root down.
fn sa_levels(n: i32) -> Option<[u16; 4]> {
    let digit = |d: i32| u16::try_from(d).ok().filter(|&d| d < 64);
    Some([
        digit(n / 0x40000)?,
        digit((n / 4096) % 64)?,
        digit((n / 64) % 64)?,
        digit(n % 64)?,
    ])
}

impl MemState {
    /// Allocates an index node below `parent`.
    fn new_index(
        &mut self,
        sa_index: u16,
        parent: Option<NodePtr<SaIndexNode>>,
    ) -> Result<NodePtr<SaIndexNode>, Abort> {
        let p = NodePtr::<SaIndexNode>::alloc(self)?;
        let n = &mut self[p];
        n.sa_index = sa_index;
        n.used = 0;
        n.link = parent;
        n.clear_children();
        Ok(p)
    }

    fn add_sa_child<K: SaChild>(&mut self, q: NodePtr<SaIndexNode>, i: u16, p: NodePtr<K>) {
        let n = &mut self[q];
        n.set_child(usize::from(i), Some(p));
        n.used += 1;
    }

    /// Creates the missing trie levels below `q` for `levels`, ending in the
    /// element for type `t` and register `n`.
    fn sa_alloc_chain(
        &mut self,
        t: ValueLevel,
        n: i32,
        mut q: NodePtr<SaIndexNode>,
        levels: &[u16],
    ) -> Result<SaLeaf, Abort> {
        let (&leaf_i, index_levels) = levels.split_last().or_confusion("sa")?;
        for &i in index_levels {
            let p = self.new_index(i, Some(q))?;
            self.add_sa_child(q, i, p);
            q = p;
        }
        let leaf = if t == ValueLevel::Mark {
            let m = NodePtr::<SaMarkClassNode>::alloc(self)?;
            self[m].clear_marks();
            SaLeaf::MarkClass(m)
        } else if t <= ValueLevel::Dimen {
            let w = NodePtr::<SaWordNode>::alloc(self)?;
            let r = &mut self[w];
            r.sa_int = 0;
            r.sa_num = n;
            r.sa_ref = TEX_NULL;
            SaLeaf::Word(w)
        } else {
            let p = NodePtr::<SaPointerNode>::alloc(self)?;
            let value = if t <= ValueLevel::Mu {
                self.add_glue_ref(ZERO_GLUE);
                MemIdx::from_raw(ZERO_GLUE.raw())
            } else {
                None
            };
            let r = &mut self[p];
            r.sa_ptr = value;
            r.sa_ref = TEX_NULL;
            SaLeaf::Pointer(p)
        };
        let e = leaf.element();
        let h = &mut self[e];
        h.sa_index = 64 * u16::from(t as u8) + leaf_i;
        h.sa_lev = LEVEL_ONE;
        h.link = Some(q);
        self.add_sa_child(q, leaf_i, e);
        Ok(leaf)
    }

    /// TeX's `find_sa_element` without creation: the element for type `t`,
    /// register `n`, if any.
    pub fn find_sa_element(&self, scanner: &ScannerState, t: ValueLevel, n: i32) -> Option<SaLeaf> {
        let found = self.sa_lookup(scanner.sa_root.get(t), n);
        found.map(|e| SaLeaf::of_type(t, e.idx()))
    }

    fn sa_lookup(&self, root: Option<NodePtr<SaIndexNode>>, n: i32) -> Option<NodePtr<SaElement>> {
        let [d1, d2, d3, d4] = sa_levels(n)?;
        let mut q = root?;
        for d in [d1, d2, d3] {
            q = self[q].child_index(usize::from(d))?;
        }
        self[q].child_element(usize::from(d4))
    }

    /// TeX's `find_sa_element` with creation.
    pub fn create_sa_element(
        &mut self,
        scanner: &mut ScannerState,
        t: ValueLevel,
        n: i32,
    ) -> Result<SaLeaf, Abort> {
        let levels = sa_levels(n).or_confusion("sa")?;
        let Some(mut q) = scanner.sa_root.get(t) else {
            let root = self.new_index(u16::from(t as u8), None)?;
            scanner.sa_root.set(t, Some(root));
            return self.sa_alloc_chain(t, n, root, &levels);
        };
        for k in 0..3 {
            match self[q].child_index(usize::from(levels[k])) {
                Some(child) => q = child,
                None => return self.sa_alloc_chain(t, n, q, &levels[k..]),
            }
        }
        match self[q].child_element(usize::from(levels[3])) {
            Some(e) => Ok(SaLeaf::of_type(t, e.idx())),
            None => self.sa_alloc_chain(t, n, q, &levels[3..]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An arena whose free lists hold nodes of leftover words, none `TEX_NULL`.
    fn arena() -> (MemState, ScannerState) {
        let mut mem = MemState {
            data: crate::mem_types::zeroed_vec(crate::mem::RESERVED_WORDS),
            ..Default::default()
        };
        for size in 2..=crate::node::SaIndexNode::SIZE {
            for _ in 0..4 {
                let p = mem.get_node(size).unwrap();
                for k in 0..size {
                    mem.set_word(p.words(k + 1).end - 1, 7);
                }
                mem.free_node(p, size);
            }
        }

        let mut scanner = ScannerState::default();
        scanner.sa_root.registers_mut().fill(None);
        (mem, scanner)
    }

    #[test]
    fn a_count_register_back_at_zero_frees_its_tree() {
        let (mut mem, mut scanner) = arena();
        let SaLeaf::Word(p) = mem
            .create_sa_element(&mut scanner, ValueLevel::Int, 300)
            .unwrap()
        else {
            panic!("a count register is a word element");
        };
        assert!(scanner.sa_root.get(ValueLevel::Int).is_some());
        let result = mem.gsa_w_def(p, 0);
        assert!(result.flush_list.is_none());
        mem.delete_sa_ref(&mut scanner, p.element()).unwrap();
        assert!(scanner.sa_root.get(ValueLevel::Int).is_none());
    }
}
