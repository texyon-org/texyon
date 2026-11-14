//! The save stack, `eqtb` definitions, and the sparse-array register trie.

use crate::base::command::Command;
use crate::base::consts::LEVEL_ONE;
use crate::base::params::IntPar;
use crate::base::token::Token;
use crate::base::units::SavePtr;
use crate::memory::mem::MemState;
use crate::print::errors::Abort;
use crate::print::errors::OrConfusion;
use crate::state::EngineState;
use crate::tables::cs::CsId;
use crate::tables::cs::Frozen;

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
                capacity: crate::print::errors::Capacity::GroupingLevels,
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
        crate::print::errors::error(
            st,
            (
                "Incompatible magnification (",
                mag,
                ");",
                crate::print::Nl(" the previous value will be retained"),
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
        crate::print::errors::error(
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
                    crate::scanning::input::prepend_to_backed_up(st, t)?;
                    if t.is_brace() {
                        if t.is_left_brace() {
                            st.scanner.align_state -= 1;
                        } else {
                            st.scanner.align_state += 1;
                        }
                    }
                } else {
                    crate::scanning::input::back_input(st)?;
                    a = true;
                }
                st.scanner.cur_tok = cur;
            }
            SaveEntry::Restore(Restore::Cs(old)) => crate::tables::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Int(old)) => crate::tables::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Dimen(old)) => crate::tables::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Glue(old)) => crate::tables::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Toks(old)) => crate::tables::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Shape(old)) => crate::tables::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Box(old)) => crate::tables::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Font(old)) => crate::tables::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Cat(old)) => crate::tables::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Lc(old)) => crate::tables::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Uc(old)) => crate::tables::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Sf(old)) => crate::tables::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Math(old)) => crate::tables::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::Del(old)) => crate::tables::eqtb::restore(st, old)?,
            SaveEntry::Restore(Restore::InterChar(old)) => crate::tables::eqtb::restore(st, old)?,
        }
    }
    if st.input.open_levels[st.input.in_open].grp == st.save_stack.cur_boundary {
        crate::print::group_warning(st);
    }
    let ss = &mut st.save_stack;
    let (group, below, _) = ss.boundary_at(ss.save_ptr).or_confusion("unsave")?;
    (ss.cur_group, ss.cur_boundary) = (group, below);
    Ok(())
}

impl MemState {}

crate::base::codes::code_enum! {
    /// The kind of group on the save stack (`cur_group`).
    #[derive(Default)]
    pub(crate) enum GroupCode: u8 {
        #[default]
        BottomLevel = 0,
        Simple = 1,
        Hbox = 2,
        AdjustedHbox = 3,
        Vbox = 4,
        Vtop = 5,
        Align = 6,
        NoAlign = 7,
        Output = 8,
        Math = 9,
        Disc = 10,
        Insert = 11,
        Vcenter = 12,
        MathChoice = 13,
        SemiSimple = 14,
        MathShift = 15,
        MathLeft = 16,
    }
}

/// The command that closes a group `}` cannot close, as TeX names it.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum Closer {
    EndGroup,
    MathShift,
    Right,
}

impl crate::print::TexPrint for Closer {
    fn print_to(&self, st: &mut EngineState) {
        use crate::print::{Esc, print};
        match self {
            Closer::EndGroup => print(st, Esc("endgroup")),
            Closer::MathShift => print(st, '$'),
            Closer::Right => print(st, Esc("right")),
        }
    }
}

/// What `off_save` inserts to close the current group, shown as the error names it.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) struct Insertion(pub Option<Closer>);

impl Insertion {
    /// The inserted tokens, in reading order.
    pub(crate) fn tokens(self) -> Vec<crate::base::token::Token> {
        use crate::base::token::Token;
        match self.0 {
            Some(Closer::EndGroup) => vec![Token::from_cs(Frozen::EndGroup.cs())],
            Some(Closer::MathShift) => vec![Token::math_shift(b'$')],
            Some(Closer::Right) => vec![Token::from_cs(Frozen::Right.cs()), Token::other(b'.')],
            None => vec![Token::right_brace(b'}')],
        }
    }
}

impl crate::print::TexPrint for Insertion {
    fn print_to(&self, st: &mut EngineState) {
        use crate::print::{Esc, print};
        match self.0 {
            Some(Closer::Right) => print(st, Esc("right.")),
            Some(closer) => print(st, closer),
            None => print(st, '}'),
        }
    }
}

impl GroupCode {
    /// What closes this group, `None` for groups that `}` closes.
    pub(crate) fn closer(self) -> Option<Closer> {
        match self {
            GroupCode::SemiSimple => Some(Closer::EndGroup),
            GroupCode::MathShift => Some(Closer::MathShift),
            GroupCode::MathLeft => Some(Closer::Right),
            _ => None,
        }
    }
}

/// One entry of the save stack.
#[derive(Copy, Clone)]
pub(crate) enum SaveEntry {
    /// Where a group began: the enclosing group's code and boundary, and the line this one began on.
    Boundary {
        group: GroupCode,
        below: Option<SavePtr>,
        line: i32,
    },
    /// A value to put back when the group ends.
    Restore(Restore),
    /// A token `\aftergroup` inserts when the group ends.
    InsertToken(crate::base::token::Token),
    /// Something a construction keeps until its group ends.
    Saved(Saved),
}

/// An old `eqtb` value, with the level it had.
#[derive(Copy, Clone)]
pub(crate) enum Restore {
    Cs(crate::tables::eqtb::Old<CsId>),
    Int(crate::tables::eqtb::Old<crate::tables::eqtb::IntLoc>),
    Dimen(crate::tables::eqtb::Old<crate::tables::eqtb::DimenLoc>),
    Glue(crate::tables::eqtb::Old<crate::tables::eqtb::GlueLoc>),
    Toks(crate::tables::eqtb::Old<crate::tables::eqtb::ToksLoc>),
    Shape(crate::tables::eqtb::Old<crate::tables::eqtb::ShapeLoc>),
    Box(crate::tables::eqtb::Old<crate::tables::eqtb::BoxLoc>),
    Font(crate::tables::eqtb::Old<crate::tables::eqtb::FontLoc>),
    Cat(crate::tables::eqtb::Old<crate::tables::eqtb::CatLoc>),
    Lc(crate::tables::eqtb::Old<crate::tables::eqtb::LcLoc>),
    Uc(crate::tables::eqtb::Old<crate::tables::eqtb::UcLoc>),
    Sf(crate::tables::eqtb::Old<crate::tables::eqtb::SfLoc>),
    Math(crate::tables::eqtb::Old<crate::tables::eqtb::MathLoc>),
    Del(crate::tables::eqtb::Old<crate::tables::eqtb::DelLoc>),
    InterChar(crate::tables::eqtb::Old<crate::tables::eqtb::InterCharPair>),
}

/// What a construction keeps on the save stack below its group.
#[derive(Copy, Clone, Debug, PartialEq, Hash)]
pub(crate) enum Saved {
    /// Where a box goes when its group ends, `box_context`.
    BoxContext(crate::build::main_control::BoxContext),
    Spec(crate::build::packaging::PackSpec),
    /// An `\insert` with its box number, or a `\vadjust` (number 255).
    Insert {
        n: i32,
        pre: bool,
    },
    /// The noad field a math group fills.
    MathGroup(crate::memory::node::MathSlot),
    /// An `\eqno` (false) or `\leqno` (true).
    EqNo {
        left: bool,
    },
    /// How many `\mathchoice` parts are done.
    ChoicePart(u8),
    /// How many `\discretionary` parts are done.
    DiscPart(u8),
}

#[derive(Default)]
pub(crate) struct SaveStackState {
    pub save_ptr: SavePtr,
    pub max_save_stack: SavePtr,
    pub save_size: usize,
    pub cur_level: u16,
    pub cur_group: GroupCode,
    /// The current group's boundary entry, none at the bottom level.
    pub cur_boundary: Option<SavePtr>,
    pub data: Vec<SaveEntry>,
}

impl SaveStackState {
    /// The save stack ran out.
    pub(crate) fn overflow(&self) -> crate::print::errors::Abort {
        crate::print::errors::Abort::overflow(
            crate::print::errors::Capacity::SaveSize,
            self.save_size,
        )
    }

    /// The stack of `size` free entries.
    pub(crate) fn alloc(&mut self, size: usize) {
        self.data = vec![SaveEntry::Saved(Saved::ChoicePart(0)); size];
    }

    #[inline]
    pub(crate) fn entry(&self, k: SavePtr) -> SaveEntry {
        self.data[k.index()]
    }

    /// Writes `e` at `save_ptr` and moves past it.
    #[inline]
    pub(crate) fn push(&mut self, e: SaveEntry) {
        self.data[self.save_ptr.index()] = e;
        self.save_ptr += 1;
    }

    /// Keeps `s` until the next group ends.
    #[inline]
    pub(crate) fn push_saved(&mut self, s: Saved) {
        self.push(SaveEntry::Saved(s));
    }

    /// Pops what [`push_saved`](Self::push_saved) kept, `None` if the top holds something else.
    pub(crate) fn pop_saved(&mut self) -> Option<Saved> {
        self.save_ptr -= 1;
        self.saved(0)
    }

    /// What a construction kept `k` entries above `save_ptr`.
    #[inline]
    pub(crate) fn saved(&self, k: i32) -> Option<Saved> {
        match self.data.get((self.save_ptr + k).index())? {
            SaveEntry::Saved(s) => Some(*s),
            _ => None,
        }
    }

    /// The entry just below `save_ptr`, to update a kept count in place.
    #[inline]
    pub(crate) fn top_saved_mut(&mut self) -> Option<&mut Saved> {
        match self.data.get_mut((self.save_ptr - 1).index())? {
            SaveEntry::Saved(s) => Some(s),
            _ => None,
        }
    }

    /// Pops the size spec a box group kept.
    pub(crate) fn pop_spec(
        &mut self,
    ) -> Result<crate::build::packaging::PackSpec, crate::print::errors::Abort> {
        match self.pop_saved() {
            Some(Saved::Spec(spec)) => Ok(spec),
            _ => Err(crate::print::errors::Abort::Confusion("pop_spec")),
        }
    }

    /// The enclosing group, its boundary, and the starting line kept by the boundary at `k`.
    #[inline]
    pub(crate) fn boundary_at(&self, k: SavePtr) -> Option<(GroupCode, Option<SavePtr>, i32)> {
        match self.entry(k) {
            SaveEntry::Boundary { group, below, line } => Some((group, below, line)),
            _ => None,
        }
    }

    /// What [`boundary_at`](Self::boundary_at) gives for the current group.
    #[inline]
    pub(crate) fn boundary(&self) -> Option<(GroupCode, Option<SavePtr>, i32)> {
        self.boundary_at(self.cur_boundary?)
    }

    /// What the current group's construction kept `k` entries below its boundary.
    #[inline]
    pub(crate) fn kept(&self, k: i32) -> Option<Saved> {
        match self.data.get((self.cur_boundary? - k).index())? {
            SaveEntry::Saved(s) => Some(*s),
            _ => None,
        }
    }
}
