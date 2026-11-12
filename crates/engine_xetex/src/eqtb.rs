//! The typed tables of `eqtb`: each holds one kind of value, with the level it was defined at.

use std::marker::PhantomData;

use crate::cmd::Cmd;
use crate::codes::code_enum;
use crate::command::Command;
use crate::consts::LEVEL_ONE;
use crate::cs::CsId;
use crate::errors::Abort;
use crate::glue_store::GlueSpecId;
use crate::journal::{JournalWord, WordDelta, WordJournal};
use crate::node::{BoxNode, NodePtr, ParShapeNode, PenaltiesNode};
use crate::params::{DimenPar, GluePar, IntPar, TokPar};
use crate::state::{EngineState, EqtbState, MemState, Restore, SaveEntry, SaveStackState};
use crate::token::TokenList;
use crate::units::{FontId, MemPtr, Scaled};

/// A value an `eqtb` table holds.
pub(crate) trait EqValue: JournalWord + PartialEq {
    /// Drops what the value owns, once no table entry holds it.
    #[inline]
    fn destroy(self, _mem: &mut MemState) -> Result<(), Abort> {
        Ok(())
    }
}

impl EqValue for i32 {}
impl EqValue for FontId {}
impl EqValue for Cmd {}

impl EqValue for Command {
    #[inline]
    fn destroy(self, mem: &mut MemState) -> Result<(), Abort> {
        mem.destroy_meaning(self)
    }
}

impl EqLoc for CsId {
    type Value = Command;
    #[inline(always)]
    fn index(self) -> usize {
        CsId::index(self)
    }
    type Store = Table<Self, Command>;
    #[inline(always)]
    fn table(eqtb: &EqtbState) -> &Self::Store {
        &eqtb.meanings
    }
    #[inline(always)]
    fn table_mut(eqtb: &mut EqtbState) -> &mut Self::Store {
        &mut eqtb.meanings
    }
    fn restore(old: Old<Self>) -> Restore {
        Restore::Cs(old)
    }
}
impl EqValue for Scaled {}

/// A location in one of the tables.
pub(crate) trait EqLoc: Copy {
    type Value: EqValue;
    type Store: EqStore<Self, Self::Value>;
    fn index(self) -> usize;
    fn table(eqtb: &EqtbState) -> &Self::Store;
    fn table_mut(eqtb: &mut EqtbState) -> &mut Self::Store;
    /// The save stack entry that puts `old` back.
    fn restore(old: Old<Self>) -> Restore;
}

/// Where a location type keeps its values and levels.
pub(crate) trait EqStore<L, T> {
    fn get(&self, loc: L) -> T;
    fn level(&self, loc: L) -> u16;
    fn set(&mut self, loc: L, value: T, level: u16);
}

impl<I: EqLoc<Value = T>, T: EqValue> EqStore<I, T> for Table<I, T> {
    #[inline(always)]
    fn get(&self, loc: I) -> T {
        Table::get(self, loc)
    }
    #[inline(always)]
    fn level(&self, loc: I) -> u16 {
        Table::level(self, loc)
    }
    #[inline(always)]
    fn set(&mut self, loc: I, value: T, level: u16) {
        Table::set(self, loc, value, level);
    }
}

/// A table for a huge, mostly default location space: it stores only the
/// locations that differ from `V::default()` at level one.
#[derive(Clone)]
pub(crate) struct SparseTable<K, V> {
    entries: std::collections::BTreeMap<K, (V, u16)>,
}

impl<K, V> Default for SparseTable<K, V> {
    fn default() -> Self {
        Self {
            entries: std::collections::BTreeMap::new(),
        }
    }
}

impl<K: Ord + Copy, V: Copy + Default + PartialEq> SparseTable<K, V> {
    /// The stored entries in key order, for the format and the oracle.
    pub(crate) fn entries(&self) -> impl Iterator<Item = (K, V, u16)> + '_ {
        self.entries.iter().map(|(&k, &(v, level))| (k, v, level))
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }
}

impl<K: Ord + Copy, V: Copy + Default + PartialEq> EqStore<K, V> for SparseTable<K, V> {
    #[inline]
    fn get(&self, loc: K) -> V {
        self.entries.get(&loc).map_or_else(V::default, |e| e.0)
    }
    #[inline]
    fn level(&self, loc: K) -> u16 {
        self.entries.get(&loc).map_or(LEVEL_ONE, |e| e.1)
    }
    fn set(&mut self, loc: K, value: V, level: u16) {
        if value == V::default() && level == LEVEL_ONE {
            self.entries.remove(&loc);
        } else {
            self.entries.insert(loc, (value, level));
        }
    }
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
        Self::at_level(len, value, LEVEL_ONE)
    }

    /// `len` locations holding `value` at `level`.
    pub(crate) fn at_level(len: usize, value: T, level: u16) -> Self {
        Self {
            values: vec![value; len],
            levels: vec![level; len],
            ..Self::default()
        }
    }

    /// Sets every location to `value` at `level`.
    pub(crate) fn fill(&mut self, value: T, level: u16) {
        for k in 0..self.values.len() {
            if self.values_journal.armed() {
                self.values_journal.note(k, self.values[k]);
                self.levels_journal.note(k, self.levels[k]);
            }
            self.values[k] = value;
            self.levels[k] = level;
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

    /// The levels, for the format.
    pub(crate) fn levels_mut(&mut self) -> &mut [u16] {
        &mut self.levels
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

            /// Frees every table, as at the end of a run.
            pub(crate) fn release_tables(&mut self) {
                $(self.$name = Table::default();)*
                self.inter_char.clear();
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
                for (pair, list, level) in self.inter_char.entries() {
                    (pair, list, level).hash(h);
                }
            }
        }
    };
}

tables!(
    meanings: Command,
    ints: i32,
    dimens: Scaled,
    glue: GlueSpecId,
    toks: TokenList,
    shapes: ShapeRef,
    boxes: Option<NodePtr<BoxNode>>,
    fonts: FontId,
    cat_codes: Cmd,
    lc_codes: i32,
    uc_codes: i32,
    sf_codes: i32,
    math_codes: i32,
    del_codes: i32,
);

impl EqtbState {
    /// Gives `loc` the value `v`, globally or in the current group.
    pub(crate) fn define<L: EqLoc>(
        &mut self,
        save_stack: &mut SaveStackState,
        mem: &mut MemState,
        global: bool,
        loc: L,
        v: L::Value,
    ) -> Result<(), Abort> {
        let table = L::table(self);
        let (old, level) = (table.get(loc), table.level(loc));
        if global {
            old.destroy(mem)?;
            L::table_mut(self).set(loc, v, LEVEL_ONE);
            return Ok(());
        }
        if old == v {
            return v.destroy(mem);
        }
        if level == save_stack.cur_level {
            old.destroy(mem)?;
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
    st.eqtb
        .define(&mut st.save_stack, &mut st.mem, global, loc, v)
}

/// Puts back what a group changed, unless the location was defined globally since.
pub(crate) fn restore<L: EqLoc>(st: &mut EngineState, old: Old<L>) -> Result<(), Abort> {
    let table = L::table(&st.eqtb);
    if table.level(old.loc) == LEVEL_ONE {
        old.value.destroy(&mut st.mem)
    } else {
        table.get(old.loc).destroy(&mut st.mem)?;
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

/// A `\XeTeXinterchartoks` location: the classes before and after a boundary.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub(crate) struct InterCharPair {
    first: u16,
    second: u16,
}

impl InterCharPair {
    /// The pair of classes `first` and `second`, `None` unless both are below `CHAR_CLASS_LIMIT`.
    pub(crate) fn new(first: i32, second: i32) -> Option<InterCharPair> {
        let class = |c: i32| {
            u16::try_from(c)
                .ok()
                .filter(|&c| i32::from(c) < crate::consts::CHAR_CLASS_LIMIT)
        };
        Some(InterCharPair {
            first: class(first)?,
            second: class(second)?,
        })
    }

    /// The two classes, for the format.
    pub(crate) fn classes(self) -> [i32; 2] {
        [i32::from(self.first), i32::from(self.second)]
    }
}

impl EqLoc for InterCharPair {
    type Value = TokenList;
    type Store = SparseTable<Self, TokenList>;
    fn index(self) -> usize {
        usize::from(self.first) * 4096 + usize::from(self.second)
    }
    #[inline(always)]
    fn table(eqtb: &EqtbState) -> &Self::Store {
        &eqtb.inter_char
    }
    #[inline(always)]
    fn table_mut(eqtb: &mut EqtbState) -> &mut Self::Store {
        &mut eqtb.inter_char
    }
    fn restore(old: Old<Self>) -> Restore {
        Restore::InterChar(old)
    }
}

/// A register number: `\count`, `\dimen`, `\skip`, `\muskip`, `\toks` or `\box` 0 to 32767.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
pub(crate) struct Register(u16);

impl Register {
    /// Registers of each kind.
    pub(crate) const COUNT: u32 = 32768;

    /// Register `n`, `None` outside `0..COUNT`.
    pub(crate) fn new(n: i32) -> Option<Register> {
        u16::try_from(n)
            .ok()
            .filter(|&r| u32::from(r) < Self::COUNT)
            .map(Register)
    }

    pub(crate) const fn get(self) -> u16 {
        self.0
    }

    /// Register `n` below 256.
    pub(crate) const fn of(n: u8) -> Register {
        Register(n as u16)
    }
}

impl From<u8> for Register {
    fn from(n: u8) -> Register {
        Register(u16::from(n))
    }
}

impl From<Register> for i32 {
    fn from(n: Register) -> i32 {
        i32::from(n.0)
    }
}

/// Declares a location type: an index into one table below `LEN`.
macro_rules! eq_loc {
    ($(#[$doc:meta])* $loc:ident, $len:expr, $value:ty, $table:ident, $restore:ident) => {
        $(#[$doc])*
        #[derive(Copy, Clone, PartialEq, Eq, Debug)]
        pub(crate) struct $loc(u32);

        impl $loc {
            /// Locations in the table.
            pub(crate) const LEN: usize = $len;

            pub(crate) const fn raw(self) -> i32 {
                self.0.cast_signed()
            }

            pub(crate) fn from_raw(raw: i32) -> Option<$loc> {
                u32::try_from(raw)
                    .ok()
                    .filter(|&r| usize::try_from(r).is_ok_and(|r| r < Self::LEN))
                    .map($loc)
            }
        }

        impl EqLoc for $loc {
            type Value = $value;
            #[inline(always)]
            fn index(self) -> usize {
                usize::try_from(self.0).unwrap_or(usize::MAX)
            }
            type Store = Table<Self, $value>;
            #[inline(always)]
            fn table(eqtb: &EqtbState) -> &Self::Store {
                &eqtb.$table
            }
            #[inline(always)]
            fn table_mut(eqtb: &mut EqtbState) -> &mut Self::Store {
                &mut eqtb.$table
            }
            fn restore(old: Old<Self>) -> Restore {
                Restore::$restore(old)
            }
        }
    };
}

const INT_PARS: u32 = IntPar::COUNT as u32;
const DIMEN_PARS: u32 = DimenPar::COUNT as u32;
const GLUE_PARS: u32 = GluePar::COUNT as u32;
const TOK_PARS: u32 = TokPar::COUNT as u32;
const REGISTERS: usize = Register::COUNT as usize;

/// The parameter number of location `n`, below the registers.
fn par_number(n: u32) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

/// The register `n` places past the parameters.
fn register(n: u32) -> Register {
    Register(u16::try_from(n).unwrap_or(u16::MAX))
}

eq_loc!(
    /// A location in the integer table: an integer parameter or a `\count` register.
    IntLoc, INT_PARS as usize + REGISTERS, i32, ints, Int
);

impl IntLoc {
    pub(crate) const fn par(p: IntPar) -> IntLoc {
        IntLoc(p as u32)
    }

    /// `\count n`.
    pub(crate) const fn count(n: Register) -> IntLoc {
        IntLoc(INT_PARS + n.0 as u32)
    }

    /// The parameter or register this location is.
    pub(crate) fn get(self) -> IntSlot {
        match self.0.checked_sub(INT_PARS) {
            Some(n) => IntSlot::Count(register(n)),
            None => IntSlot::Par(par_number(self.0)),
        }
    }
}

/// What an [`IntLoc`] names.
pub(crate) enum IntSlot {
    /// The parameter numbered as in [`IntPar`].
    Par(u16),
    Count(Register),
}

eq_loc!(
    /// A location in the dimension table: a dimension parameter or a `\dimen` register.
    DimenLoc, DIMEN_PARS as usize + REGISTERS, Scaled, dimens, Dimen
);

impl DimenLoc {
    pub(crate) const fn par(p: DimenPar) -> DimenLoc {
        DimenLoc(p as u32)
    }

    /// `\dimen n`.
    pub(crate) const fn dimen(n: Register) -> DimenLoc {
        DimenLoc(DIMEN_PARS + n.0 as u32)
    }

    /// The parameter or register this location is.
    pub(crate) fn get(self) -> DimenSlot {
        match self.0.checked_sub(DIMEN_PARS) {
            Some(n) => DimenSlot::Dimen(register(n)),
            None => DimenSlot::Par(par_number(self.0)),
        }
    }
}

/// What a [`DimenLoc`] names.
pub(crate) enum DimenSlot {
    /// The parameter numbered as in [`DimenPar`].
    Par(u16),
    Dimen(Register),
}

impl EqValue for GlueSpecId {
    #[inline]
    fn destroy(self, mem: &mut MemState) -> Result<(), Abort> {
        mem.delete_glue_ref(self);
        Ok(())
    }
}

eq_loc!(
    /// A location in the glue table: a glue parameter, a `\skip` or a `\muskip` register.
    GlueLoc, GLUE_PARS as usize + 2 * REGISTERS, GlueSpecId, glue, Glue
);

impl GlueLoc {
    pub(crate) const fn par(p: GluePar) -> GlueLoc {
        GlueLoc(p as u32)
    }

    /// `\skip n`.
    pub(crate) const fn skip(n: Register) -> GlueLoc {
        GlueLoc(GLUE_PARS + n.0 as u32)
    }

    /// `\muskip n`.
    pub(crate) const fn mu_skip(n: Register) -> GlueLoc {
        GlueLoc(GLUE_PARS + Register::COUNT + n.0 as u32)
    }

    /// The parameter or register this location is.
    pub(crate) fn get(self) -> GlueSlot {
        match self.0.checked_sub(GLUE_PARS) {
            None => GlueSlot::Par(par_number(self.0)),
            Some(n) if n < Register::COUNT => GlueSlot::Skip(register(n)),
            Some(n) => GlueSlot::MuSkip(register(n - Register::COUNT)),
        }
    }
}

/// What a [`GlueLoc`] names.
pub(crate) enum GlueSlot {
    /// The parameter numbered as in [`GluePar`].
    Par(u16),
    Skip(Register),
    MuSkip(Register),
}

impl EqValue for TokenList {
    #[inline]
    fn destroy(self, mem: &mut MemState) -> Result<(), Abort> {
        mem.tokens.delete_ref(self);
        Ok(())
    }
}

eq_loc!(
    /// A location in the token list table: a token list parameter or a `\toks` register.
    ToksLoc, TOK_PARS as usize + REGISTERS, TokenList, toks, Toks
);

impl ToksLoc {
    pub(crate) const fn par(p: TokPar) -> ToksLoc {
        ToksLoc(p as u32 - TokPar::OutputRoutine as u32)
    }

    /// `\toks n`.
    pub(crate) const fn register(n: Register) -> ToksLoc {
        ToksLoc(TOK_PARS + n.0 as u32)
    }

    /// The parameter or register this location is.
    pub(crate) fn get(self) -> ToksSlot {
        match self.0.checked_sub(TOK_PARS) {
            Some(n) => ToksSlot::Register(register(n)),
            None => ToksSlot::Par(par_number(self.0) + TokPar::OutputRoutine as u16),
        }
    }
}

/// What a [`ToksLoc`] names.
pub(crate) enum ToksSlot {
    /// The parameter numbered as in [`TokPar`].
    Par(u16),
    Register(Register),
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
    type Store = Table<Self, ShapeRef>;
    #[inline(always)]
    fn table(eqtb: &EqtbState) -> &Self::Store {
        &eqtb.shapes
    }
    #[inline(always)]
    fn table_mut(eqtb: &mut EqtbState) -> &mut Self::Store {
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
    fn destroy(self, mem: &mut MemState) -> Result<(), Abort> {
        mem.free_shape_ref(self.0);
        Ok(())
    }
}

impl EqValue for Option<NodePtr<BoxNode>> {
    #[inline]
    fn destroy(self, mem: &mut MemState) -> Result<(), Abort> {
        mem.flush_node_list(self.map(NodePtr::idx))
    }
}

/// A `\box` register.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) struct BoxLoc(Register);

impl BoxLoc {
    /// Locations in the table.
    pub(crate) const LEN: usize = REGISTERS;

    pub(crate) const fn register(n: Register) -> BoxLoc {
        BoxLoc(n)
    }

    pub(crate) fn raw(self) -> i32 {
        i32::from(self.0 .0)
    }
}

impl EqLoc for BoxLoc {
    type Value = Option<NodePtr<BoxNode>>;
    #[inline(always)]
    fn index(self) -> usize {
        usize::from(self.0 .0)
    }
    type Store = Table<Self, Self::Value>;
    #[inline(always)]
    fn table(eqtb: &EqtbState) -> &Self::Store {
        &eqtb.boxes
    }
    #[inline(always)]
    fn table_mut(eqtb: &mut EqtbState) -> &mut Self::Store {
        &mut eqtb.boxes
    }
    fn restore(old: Old<Self>) -> Restore {
        Restore::Box(old)
    }
}

eq_loc!(
    /// A location in the font table: the current font or a math family font of some size.
    FontLoc, 1 + 3 * 256, FontId, fonts, Font
);

impl FontLoc {
    /// `\font`, the current font.
    pub(crate) const CUR: FontLoc = FontLoc(0);
    /// `\textfont0`, `\scriptfont0` and `\scriptscriptfont0`.
    pub(crate) const TEXT: FontLoc = FontLoc(1);
    pub(crate) const SCRIPT: FontLoc = FontLoc(1 + 256);
    pub(crate) const SCRIPT_SCRIPT: FontLoc = FontLoc(1 + 512);

    /// The math font `n` past `\textfont0`: a family plus `TEXT_SIZE`, `SCRIPT_SIZE` or
    /// `SCRIPT_SCRIPT_SIZE`. Out of range for any other `n`.
    #[inline]
    pub(crate) fn math(n: i32) -> FontLoc {
        FontLoc(u32::try_from(n + 1).unwrap_or(u32::MAX))
    }

    /// Family `fam` of the size this location starts.
    pub(crate) fn family(self, fam: u8) -> FontLoc {
        FontLoc(self.0 + u32::from(fam))
    }

    /// The size and family offset past `\textfont0`, as `print_size` takes it.
    pub(crate) fn math_offset(self) -> i32 {
        self.raw() - 1
    }
}

/// Declares the location type of one per-character code table.
macro_rules! code_loc {
    ($(#[$doc:meta])* $loc:ident, $value:ty, $table:ident, $restore:ident) => {
        $(#[$doc])*
        #[derive(Copy, Clone, PartialEq, Eq, Debug)]
        pub(crate) struct $loc(u32);

        impl $loc {
            /// The entry of character `c`, out of range for anything but a USV.
            #[inline(always)]
            pub(crate) fn of(c: i32) -> $loc {
                $loc(u32::try_from(c).unwrap_or(u32::MAX))
            }

            pub(crate) fn raw(self) -> i32 {
                i32::try_from(self.0).unwrap_or(-1)
            }
        }

        impl EqLoc for $loc {
            type Value = $value;
            #[inline(always)]
            fn index(self) -> usize {
                usize::try_from(self.0).unwrap_or(usize::MAX)
            }
            type Store = Table<Self, $value>;
            #[inline(always)]
            fn table(eqtb: &EqtbState) -> &Self::Store {
                &eqtb.$table
            }
            #[inline(always)]
            fn table_mut(eqtb: &mut EqtbState) -> &mut Self::Store {
                &mut eqtb.$table
            }
            fn restore(old: Old<Self>) -> Restore {
                Restore::$restore(old)
            }
        }
    };
}

code_loc!(
    /// A `\catcode` entry.
    CatLoc, Cmd, cat_codes, Cat
);
code_loc!(
    /// An `\lccode` entry.
    LcLoc, i32, lc_codes, Lc
);
code_loc!(
    /// A `\uccode` entry.
    UcLoc, i32, uc_codes, Uc
);
code_loc!(
    /// An `\sfcode` entry: the space factor, and XeTeX's character class times 65536.
    SfLoc, i32, sf_codes, Sf
);
code_loc!(
    /// A `\mathcode` entry, in XeTeX's encoding.
    MathLoc, i32, math_codes, Math
);
code_loc!(
    /// A `\delcode` entry.
    DelLoc, i32, del_codes, Del
);

code_enum! {
    /// The table a `\catcode`-like primitive sets.
    pub(crate) enum CodeTable: u8 {
        Cat = 0 => "catcode",
        Lc = 1 => "lccode",
        Uc = 2 => "uccode",
        Sf = 3 => "sfcode",
        Math = 4 => "mathcode",
        Del = 5 => "delcode",
    }
}

code_enum! {
    /// What a XeTeX code primitive sets: a character class, or a math or delimiter code
    /// as one number or in parts.
    pub(crate) enum XetexCode: u8 {
        CharClass = 0 => "XeTeXcharclass",
        MathCodeNum = 1 => "Umathcodenum",
        MathCode = 2 => "Umathcode",
        DelCodeNum = 3 => "Udelcodenum",
        DelCode = 4 => "Udelcode",
    }
}

code_enum! {
    /// The table `\lowercase` or `\uppercase` maps through.
    pub(crate) enum CaseTable: u8 {
        Lower = 0 => "lowercase",
        Upper = 1 => "uppercase",
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
        let (eqtb, ss, mem) = (&mut st.eqtb, &mut st.save_stack, &mut st.mem);
        eqtb.define(ss, mem, global, loc, v).unwrap();
    }

    #[test]
    fn a_group_restores_a_local_value_but_keeps_a_global_one() {
        let mut st = state();
        let (a, b) = (
            IntLoc::count(Register::of(1)),
            IntLoc::count(Register::of(2)),
        );
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
        let last = Register::new(32767).unwrap();
        assert!(
            matches!(IntLoc::count(Register::of(3)).get(), IntSlot::Count(r) if r == Register::of(3))
        );
        assert!(matches!(IntLoc::par(IntPar::Mag).get(), IntSlot::Par(17)));
        assert_eq!(
            IntLoc::from_raw(IntLoc::count(last).raw()),
            Some(IntLoc::count(last))
        );
        assert_eq!(IntLoc::from_raw(IntLoc::count(last).raw() + 1), None);
        assert_eq!(Register::new(32768), None);
        assert!(
            matches!(DimenLoc::dimen(Register::of(0)).get(), DimenSlot::Dimen(r) if r == Register::of(0))
        );
        assert!(matches!(GlueLoc::mu_skip(last).get(), GlueSlot::MuSkip(r) if r == last));
        assert!(matches!(GlueLoc::skip(last).get(), GlueSlot::Skip(r) if r == last));
    }
}
