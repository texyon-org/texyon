//! The typed tables of `eqtb`: each holds one kind of value, with the level it was defined at.

use std::marker::PhantomData;

use crate::codes::code_enum;
use crate::consts::LEVEL_ONE;
use crate::errors::Abort;
use crate::glue_store::GlueSpecId;
use crate::journal::{JournalWord, WordDelta, WordJournal};
use crate::node::{BoxNode, NodePtr, ParShapeNode, PenaltiesNode};
use crate::params::{DimenPar, GluePar, IntPar, TokPar};
use crate::state::{
    EngineState, EqtbState, MemState, Restore, SaveEntry, SaveStackState, ScannerState,
};
use crate::token::TokenList;
use crate::units::{MemPtr, Scaled};

/// A value an `eqtb` table holds.
pub(crate) trait EqValue: JournalWord + PartialEq {
    /// Drops what the value owns, once no table entry holds it.
    #[inline]
    fn destroy(self, _mem: &mut MemState, _scanner: &mut ScannerState) -> Result<(), Abort> {
        Ok(())
    }
}

impl EqValue for i32 {}
impl EqValue for Scaled {}

/// A location in one of the tables.
pub(crate) trait EqLoc: Copy {
    type Value: EqValue;
    fn index(self) -> usize;
    fn table(eqtb: &EqtbState) -> &Table<Self, Self::Value>;
    fn table_mut(eqtb: &mut EqtbState) -> &mut Table<Self, Self::Value>;
    /// The save stack entry that puts `old` back.
    fn restore(old: Old<Self>) -> Restore;
}

/// The value a location held before a group changed it, and its level.
#[derive(Copy, Clone)]
pub(crate) struct Old<L: EqLoc> {
    pub(crate) loc: L,
    pub(crate) value: L::Value,
    pub(crate) level: u16,
}

/// One table: a value and a level per location, journaled for checkpoints.
pub(crate) struct Table<I, T: Copy> {
    values: Vec<T>,
    levels: Vec<u16>,
    values_journal: WordJournal<T>,
    levels_journal: WordJournal<u16>,
    _index: PhantomData<fn(I)>,
}

impl<I, T: Copy> Default for Table<I, T> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            levels: Vec::new(),
            values_journal: WordJournal::default(),
            levels_journal: WordJournal::default(),
            _index: PhantomData,
        }
    }
}

impl<I: EqLoc<Value = T>, T: EqValue> Table<I, T> {
    /// `len` locations holding `value` at level one.
    pub(crate) fn new(len: usize, value: T) -> Self {
        Self {
            values: vec![value; len],
            levels: vec![LEVEL_ONE; len],
            ..Self::default()
        }
    }

    #[inline(always)]
    pub(crate) fn get(&self, i: I) -> T {
        self.values[i.index()]
    }

    #[inline]
    pub(crate) fn level(&self, i: I) -> u16 {
        self.levels[i.index()]
    }

    /// Sets the value and level of `i`.
    #[inline]
    pub(crate) fn set(&mut self, i: I, value: T, level: u16) {
        let k = i.index();
        if self.values_journal.armed() {
            self.values_journal.note(k, self.values[k]);
            self.levels_journal.note(k, self.levels[k]);
        }
        self.values[k] = value;
        self.levels[k] = level;
    }

    /// Sets the value of `i`, keeping its level.
    #[inline]
    pub(crate) fn set_value(&mut self, i: I, value: T) {
        let k = i.index();
        if self.values_journal.armed() {
            self.values_journal.note(k, self.values[k]);
        }
        self.values[k] = value;
    }

    /// The values, for the format.
    pub(crate) fn values_mut(&mut self) -> &mut [T] {
        &mut self.values
    }

    pub(crate) fn values(&self) -> &[T] {
        &self.values
    }

    pub(crate) fn levels(&self) -> &[u16] {
        &self.levels
    }

    pub(crate) fn arm(&mut self) {
        self.values_journal.arm(self.values.len());
        self.levels_journal.arm(self.levels.len());
    }

    pub(crate) fn disarm(&mut self) {
        self.values_journal.disarm();
        self.levels_journal.disarm();
    }

    /// Closes the journal's epoch, see [`WordJournal::seal`].
    pub(crate) fn seal(&mut self) -> TableDelta<T> {
        TableDelta {
            values: self.values_journal.seal(&self.values),
            levels: self.levels_journal.seal(&self.levels),
        }
    }

    pub(crate) fn snapshot(&self) -> TableDelta<T> {
        TableDelta {
            values: self.values_journal.snapshot(),
            levels: self.levels_journal.snapshot(),
        }
    }
}

/// The pre-images one epoch recorded for a [`Table`].
#[derive(Clone)]
pub(crate) struct TableDelta<T: Copy> {
    values: WordDelta<T>,
    levels: WordDelta<u16>,
}

impl<T: Copy> Default for TableDelta<T> {
    fn default() -> Self {
        Self {
            values: WordDelta::default(),
            levels: WordDelta::default(),
        }
    }
}

impl<T: JournalWord> TableDelta<T> {
    pub(crate) fn apply_to<I>(&self, table: &mut Table<I, T>) {
        self.values.apply_to(&mut table.values);
        self.levels.apply_to(&mut table.levels);
    }

    pub(crate) fn compose_with_older(&mut self, older: TableDelta<T>) {
        self.values.compose_with_older(older.values);
        self.levels.compose_with_older(older.levels);
    }

    pub(crate) fn allocated_bytes(&self) -> usize {
        self.values.allocated_bytes() + self.levels.allocated_bytes()
    }

    pub(crate) fn compact(&mut self) {
        self.values.compact();
        self.levels.compact();
    }
}

/// Declares the tables once: their deltas and the journal calls that cover all of them.
macro_rules! tables {
    ($($name:ident: $t:ty),* $(,)?) => {
        /// Every table's delta.
        #[derive(Default, Clone)]
        pub(crate) struct TablesDelta {
            $($name: TableDelta<$t>,)*
        }

        impl TablesDelta {
            pub(crate) fn apply_to(&self, eqtb: &mut EqtbState) {
                $(self.$name.apply_to(&mut eqtb.$name);)*
            }

            pub(crate) fn compose_with_older(&mut self, older: TablesDelta) {
                $(self.$name.compose_with_older(older.$name);)*
            }

            pub(crate) fn allocated_bytes(&self) -> usize {
                0 $(+ self.$name.allocated_bytes())*
            }

            pub(crate) fn compact(&mut self) {
                $(self.$name.compact();)*
            }
        }

        impl EqtbState {
            pub(crate) fn arm_tables(&mut self) {
                $(self.$name.arm();)*
            }

            pub(crate) fn disarm_tables(&mut self) {
                $(self.$name.disarm();)*
            }

            pub(crate) fn seal_tables(&mut self) -> TablesDelta {
                TablesDelta {
                    $($name: self.$name.seal(),)*
                }
            }

            pub(crate) fn snapshot_tables(&self) -> TablesDelta {
                TablesDelta {
                    $($name: self.$name.snapshot(),)*
                }
            }

            /// Feeds every table's values and levels to `h`, for the checkpoint oracle.
            pub(crate) fn hash_tables(&self, h: &mut impl std::hash::Hasher) {
                use std::hash::Hash;
                $(
                    self.$name.values().hash(h);
                    self.$name.levels().hash(h);
                )*
            }
        }
    };
}

tables!(
    ints: i32,
    dimens: Scaled,
    glue: GlueSpecId,
    toks: TokenList,
    shapes: ShapeRef,
    boxes: Option<NodePtr<BoxNode>>,
);

impl EqtbState {
    /// Gives `loc` the value `v`, globally or in the current group.
    pub(crate) fn define<L: EqLoc>(
        &mut self,
        save_stack: &mut SaveStackState,
        mem: &mut MemState,
        scanner: &mut ScannerState,
        global: bool,
        loc: L,
        v: L::Value,
    ) -> Result<(), Abort> {
        let table = L::table(self);
        let (old, level) = (table.get(loc), table.level(loc));
        if global {
            old.destroy(mem, scanner)?;
            L::table_mut(self).set(loc, v, LEVEL_ONE);
            return Ok(());
        }
        if old == v {
            return v.destroy(mem, scanner);
        }
        if level == save_stack.cur_level {
            old.destroy(mem, scanner)?;
        } else if save_stack.cur_level > LEVEL_ONE {
            save_stack.save(L::restore(Old {
                loc,
                value: old,
                level,
            }))?;
        }
        L::table_mut(self).set(loc, v, save_stack.cur_level);
        Ok(())
    }
}

/// TeX's `define` for a typed location: `\global` or in the current group.
pub(crate) fn define<L: EqLoc>(
    st: &mut EngineState,
    global: bool,
    loc: L,
    v: L::Value,
) -> Result<(), Abort> {
    st.eqtb.define(
        &mut st.save_stack,
        &mut st.mem,
        &mut st.scanner,
        global,
        loc,
        v,
    )
}

/// Puts back what a group changed, unless the location was defined globally since.
pub(crate) fn restore<L: EqLoc>(st: &mut EngineState, old: Old<L>) -> Result<(), Abort> {
    let table = L::table(&st.eqtb);
    if table.level(old.loc) == LEVEL_ONE {
        old.value.destroy(&mut st.mem, &mut st.scanner)
    } else {
        table.get(old.loc).destroy(&mut st.mem, &mut st.scanner)?;
        L::table_mut(&mut st.eqtb).set(old.loc, old.value, old.level);
        Ok(())
    }
}

impl SaveStackState {
    /// Pushes a restore entry.
    pub(crate) fn save(&mut self, r: Restore) -> Result<(), Abort> {
        if self.save_stack_full() {
            return Err(self.overflow());
        }
        self.push(SaveEntry::Restore(r));
        Ok(())
    }
}

/// Declares a location type: an index into one table below `LEN`.
macro_rules! eq_loc {
    ($(#[$doc:meta])* $loc:ident, $len:expr, $value:ty, $table:ident, $restore:ident) => {
        $(#[$doc])*
        #[derive(Copy, Clone, PartialEq, Eq, Debug)]
        pub(crate) struct $loc(u16);

        impl $loc {
            /// Locations in the table.
            pub(crate) const LEN: usize = $len;

            pub(crate) const fn raw(self) -> i32 {
                self.0 as i32
            }

            pub(crate) fn from_raw(raw: i32) -> Option<$loc> {
                u16::try_from(raw)
                    .ok()
                    .filter(|&r| usize::from(r) < Self::LEN)
                    .map($loc)
            }
        }

        impl EqLoc for $loc {
            type Value = $value;
            #[inline(always)]
            fn index(self) -> usize {
                usize::from(self.0)
            }
            #[inline(always)]
            fn table(eqtb: &EqtbState) -> &Table<Self, $value> {
                &eqtb.$table
            }
            #[inline(always)]
            fn table_mut(eqtb: &mut EqtbState) -> &mut Table<Self, $value> {
                &mut eqtb.$table
            }
            fn restore(old: Old<Self>) -> Restore {
                Restore::$restore(old)
            }
        }
    };
}

const INT_PARS: u16 = IntPar::COUNT as u16;
const DIMEN_PARS: u16 = DimenPar::COUNT as u16;
const GLUE_PARS: u16 = GluePar::COUNT as u16;
const TOK_PARS: u16 = TokPar::COUNT as u16;

eq_loc!(
    /// A location in the integer table: an integer parameter or a `\count` register.
    IntLoc, INT_PARS as usize + 256, i32, ints, Int
);

impl IntLoc {
    pub(crate) const fn par(p: IntPar) -> IntLoc {
        IntLoc(p as u16)
    }

    /// `\count n`.
    pub(crate) const fn count(n: u8) -> IntLoc {
        IntLoc(INT_PARS + n as u16)
    }

    /// The parameter or register this location is.
    pub(crate) fn get(self) -> IntSlot {
        match self.0.checked_sub(INT_PARS) {
            Some(n) => IntSlot::Count(n),
            None => IntSlot::Par(self.0),
        }
    }
}

/// What an [`IntLoc`] names.
pub(crate) enum IntSlot {
    /// The parameter numbered as in [`IntPar`].
    Par(u16),
    Count(u16),
}

eq_loc!(
    /// A location in the dimension table: a dimension parameter or a `\dimen` register.
    DimenLoc, DIMEN_PARS as usize + 256, Scaled, dimens, Dimen
);

impl DimenLoc {
    pub(crate) const fn par(p: DimenPar) -> DimenLoc {
        DimenLoc(p as u16)
    }

    /// `\dimen n`.
    pub(crate) const fn dimen(n: u8) -> DimenLoc {
        DimenLoc(DIMEN_PARS + n as u16)
    }

    /// The parameter or register this location is.
    pub(crate) fn get(self) -> DimenSlot {
        match self.0.checked_sub(DIMEN_PARS) {
            Some(n) => DimenSlot::Dimen(n),
            None => DimenSlot::Par(self.0),
        }
    }
}

/// What a [`DimenLoc`] names.
pub(crate) enum DimenSlot {
    /// The parameter numbered as in [`DimenPar`].
    Par(u16),
    Dimen(u16),
}

impl EqValue for GlueSpecId {
    #[inline]
    fn destroy(self, mem: &mut MemState, _scanner: &mut ScannerState) -> Result<(), Abort> {
        mem.delete_glue_ref(self);
        Ok(())
    }
}

eq_loc!(
    /// A location in the glue table: a glue parameter, a `\skip` or a `\muskip` register.
    GlueLoc, GLUE_PARS as usize + 512, GlueSpecId, glue, Glue
);

impl GlueLoc {
    pub(crate) const fn par(p: GluePar) -> GlueLoc {
        GlueLoc(p as u16)
    }

    /// `\skip n`.
    pub(crate) const fn skip(n: u8) -> GlueLoc {
        GlueLoc(GLUE_PARS + n as u16)
    }

    /// `\muskip n`.
    pub(crate) const fn mu_skip(n: u8) -> GlueLoc {
        GlueLoc(GLUE_PARS + 256 + n as u16)
    }

    /// The parameter or register this location is.
    pub(crate) fn get(self) -> GlueSlot {
        match self.0.checked_sub(GLUE_PARS) {
            None => GlueSlot::Par(self.0),
            Some(n) if n < 256 => GlueSlot::Skip(n),
            Some(n) => GlueSlot::MuSkip(n - 256),
        }
    }
}

/// What a [`GlueLoc`] names.
pub(crate) enum GlueSlot {
    /// The parameter numbered as in [`GluePar`].
    Par(u16),
    Skip(u16),
    MuSkip(u16),
}

impl EqValue for TokenList {
    #[inline]
    fn destroy(self, mem: &mut MemState, _scanner: &mut ScannerState) -> Result<(), Abort> {
        mem.tokens.delete_ref(self);
        Ok(())
    }
}

eq_loc!(
    /// A location in the token list table: a token list parameter or a `\toks` register.
    ToksLoc, TOK_PARS as usize + 256, TokenList, toks, Toks
);

impl ToksLoc {
    pub(crate) const fn par(p: TokPar) -> ToksLoc {
        ToksLoc(p as u16 - TokPar::OutputRoutine as u16)
    }

    /// `\toks n`.
    pub(crate) const fn register(n: u8) -> ToksLoc {
        ToksLoc(TOK_PARS + n as u16)
    }

    /// The parameter or register this location is.
    pub(crate) fn get(self) -> ToksSlot {
        match self.0.checked_sub(TOK_PARS) {
            Some(n) => ToksSlot::Register(n),
            None => ToksSlot::Par(self.0 + TokPar::OutputRoutine as u16),
        }
    }
}

/// What a [`ToksLoc`] names.
pub(crate) enum ToksSlot {
    /// The parameter numbered as in [`TokPar`].
    Par(u16),
    Register(u16),
}

code_enum! {
    /// A location in the shape table: `\parshape` or one of e-TeX's penalty arrays.
    pub(crate) enum ShapeLoc: u8 {
        ParShape = 0 => "parshape",
        InterLinePenalties = 1 => "interlinepenalties",
        ClubPenalties = 2 => "clubpenalties",
        WidowPenalties = 3 => "widowpenalties",
        DisplayWidowPenalties = 4 => "displaywidowpenalties",
    }
}

impl ShapeLoc {
    /// Locations in the table.
    pub(crate) const LEN: usize = ShapeLoc::DisplayWidowPenalties as usize + 1;
}

impl EqLoc for ShapeLoc {
    type Value = ShapeRef;
    #[inline(always)]
    fn index(self) -> usize {
        self as usize
    }
    #[inline(always)]
    fn table(eqtb: &EqtbState) -> &Table<Self, ShapeRef> {
        &eqtb.shapes
    }
    #[inline(always)]
    fn table_mut(eqtb: &mut EqtbState) -> &mut Table<Self, ShapeRef> {
        &mut eqtb.shapes
    }
    fn restore(old: Old<Self>) -> Restore {
        Restore::Shape(old)
    }
}

/// A `\parshape` node or an e-TeX penalty array, as the shape table holds it.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Hash, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(transparent)]
pub(crate) struct ShapeRef(MemPtr);

impl ShapeRef {
    pub(crate) const NONE: ShapeRef = ShapeRef(None);

    pub(crate) fn par_shape(p: NodePtr<ParShapeNode>) -> ShapeRef {
        ShapeRef(Some(p.idx()))
    }

    pub(crate) fn penalties(p: NodePtr<PenaltiesNode>) -> ShapeRef {
        ShapeRef(Some(p.idx()))
    }

    pub(crate) fn ptr(self) -> MemPtr {
        self.0
    }

    /// The shape a format word holds.
    pub(crate) fn from_ptr(p: MemPtr) -> ShapeRef {
        ShapeRef(p)
    }
}

impl EqValue for ShapeRef {
    #[inline]
    fn destroy(self, mem: &mut MemState, _scanner: &mut ScannerState) -> Result<(), Abort> {
        mem.free_shape_ref(self.0);
        Ok(())
    }
}

impl EqValue for Option<NodePtr<BoxNode>> {
    #[inline]
    fn destroy(self, mem: &mut MemState, _scanner: &mut ScannerState) -> Result<(), Abort> {
        mem.flush_node_list(self.map(NodePtr::idx))
    }
}

/// A `\box` register.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) struct BoxLoc(u8);

impl BoxLoc {
    /// Locations in the table.
    pub(crate) const LEN: usize = 256;

    pub(crate) const fn register(n: u8) -> BoxLoc {
        BoxLoc(n)
    }

    pub(crate) const fn raw(self) -> i32 {
        self.0 as i32
    }
}

impl EqLoc for BoxLoc {
    type Value = Option<NodePtr<BoxNode>>;
    #[inline(always)]
    fn index(self) -> usize {
        usize::from(self.0)
    }
    #[inline(always)]
    fn table(eqtb: &EqtbState) -> &Table<Self, Self::Value> {
        &eqtb.boxes
    }
    #[inline(always)]
    fn table_mut(eqtb: &mut EqtbState) -> &mut Table<Self, Self::Value> {
        &mut eqtb.boxes
    }
    fn restore(old: Old<Self>) -> Restore {
        Restore::Box(old)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consts::LEVEL_ZERO;

    fn state() -> EngineState {
        let mut st = EngineState::default();
        st.eqtb.ints = Table::new(IntLoc::LEN, 0);
        st.save_stack.alloc(16);
        st.save_stack.save_size = 16;
        st.save_stack.cur_level = LEVEL_ONE;
        st.input.open_levels = vec![Default::default()];
        st
    }

    fn define(st: &mut EngineState, global: bool, loc: IntLoc, v: i32) {
        let (eqtb, ss, mem, sc) = (
            &mut st.eqtb,
            &mut st.save_stack,
            &mut st.mem,
            &mut st.scanner,
        );
        eqtb.define(ss, mem, sc, global, loc, v).unwrap();
    }

    #[test]
    fn a_group_restores_a_local_value_but_keeps_a_global_one() {
        let mut st = state();
        let (a, b) = (IntLoc::count(1), IntLoc::count(2));
        define(&mut st, false, a, 5);
        define(&mut st, false, b, 6);
        st.save_stack.new_save_level(Default::default(), 0).unwrap();
        define(&mut st, false, a, 7);
        define(&mut st, true, b, 8);
        define(&mut st, false, b, 9);
        assert_eq!((st.eqtb.ints.get(a), st.eqtb.ints.level(a)), (7, 2));
        crate::save::unsave(&mut st).unwrap();
        assert_eq!((st.eqtb.ints.get(a), st.eqtb.ints.level(a)), (5, LEVEL_ONE));
        assert_eq!((st.eqtb.ints.get(b), st.eqtb.ints.level(b)), (8, LEVEL_ONE));
        assert_ne!(st.eqtb.ints.level(a), LEVEL_ZERO);
    }

    #[test]
    fn locations_name_their_parameter_or_register() {
        assert!(matches!(IntLoc::count(3).get(), IntSlot::Count(3)));
        assert!(matches!(IntLoc::par(IntPar::Mag).get(), IntSlot::Par(17)));
        assert_eq!(
            IntLoc::from_raw(IntLoc::count(255).raw()),
            Some(IntLoc::count(255))
        );
        assert_eq!(IntLoc::from_raw(IntLoc::count(255).raw() + 1), None);
        assert!(matches!(DimenLoc::dimen(0).get(), DimenSlot::Dimen(0)));
    }
}
