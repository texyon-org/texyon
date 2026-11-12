//! The save stack, `eqtb` definitions, and the sparse-array register trie.
use crate::command::Command;
use crate::consts::LEVEL_ONE;
use crate::errors::Abort;
use crate::errors::OrConfusion;
use crate::params::IntPar;
use crate::state::{EngineState, GroupCode, MemState, Restore, SaveEntry, SaveStackState};
use crate::token::Token;

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

impl MemState {
    /// Drops what a control sequence's meaning holds: a macro's reference.
    pub(crate) fn destroy_meaning(&mut self, m: Command) -> Result<(), Abort> {
        if let Some(body) = m.macro_body() {
            self.tokens.delete_ref(body);
        }
        Ok(())
    }
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
            SaveEntry::Restore(Restore::Cs(old)) => crate::eqtb::restore(st, old)?,
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
            SaveEntry::Restore(Restore::InterChar(old)) => crate::eqtb::restore(st, old)?,
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

impl MemState {}
