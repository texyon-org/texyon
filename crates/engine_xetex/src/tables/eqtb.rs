//! The typed tables of `eqtb`: each holds one kind of value, with the level it was defined at.

use std::collections::BTreeMap;
use std::marker::PhantomData;

use crate::base::cmd::Cmd;
use crate::base::codes::code_enum;
use crate::base::command::Command;
use crate::base::consts::{CHAR_CLASS_LIMIT, LEVEL_ONE, LEVEL_ZERO, NULL_FONT, NUMBER_USVS};
use crate::base::params::{DimenPar, GluePar, IntPar, TokPar};
use crate::base::token::TokenList;
use crate::base::units::{FontId, MemPtr, PrimNum, Scaled, StrNumber};
use crate::memory::glue_store::GlueSpecId;
use crate::memory::mem::{MemState, ZERO_GLUE};
use crate::memory::mem_types::zeroed_vec;
use crate::memory::node::{BoxNode, NodePtr, ParShapeNode, PenaltiesNode};
use crate::preview::journal::{
    Delta, JournalWord, Journaled, JournaledVec, WordDelta, journaled_fields,
};
use crate::print::errors::Abort;
use crate::scanning::pgfmath::KnownCs;
use crate::state::EngineState;
use crate::tables::cs::{CsId, NameSlot};
use crate::tables::hash::{HashEntry, PrimEntry};
use crate::tables::save::{Restore, SaveEntry, SaveStackState};

pub(crate) trait EqValue: JournalWord + PartialEq {
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

impl EqValue for Scaled {}

pub(crate) trait EqLoc: Copy {
    type Value: EqValue;
    type Store: EqStore<Self, Self::Value>;
    fn index(self) -> usize;
    fn table(eqtb: &EqtbState) -> &Self::Store;
    fn table_mut(eqtb: &mut EqtbState) -> &mut Self::Store;
    fn restore(old: Old<Self>) -> Restore;
}

/// Implements [`EqLoc`] for `$loc`, kept in `eqtb.$table` and saved as `Restore::$restore`.
macro_rules! impl_eq_loc {
    ($loc:ty, $store:ident, $value:ty, $table:ident, $restore:ident, |$l:ident| $index:expr) => {
        impl EqLoc for $loc {
            type Value = $value;
            type Store = $store<Self, $value>;
            #[inline(always)]
            fn index(self) -> usize {
                let $l = self;
                $index
            }
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

impl_eq_loc!(CsId, Table, Command, meanings, Cs, |cs| CsId::index(cs));

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
    entries: BTreeMap<K, (V, u16)>,
}

impl<K, V> Default for SparseTable<K, V> {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }
}

impl<K: Ord + Copy, V: Copy + Default + PartialEq> SparseTable<K, V> {
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

#[derive(Copy, Clone)]
pub(crate) struct Old<L: EqLoc> {
    pub(crate) loc: L,
    pub(crate) value: L::Value,
    pub(crate) level: u16,
}

/// One table: a value and a level per location, journaled for checkpoints.
pub(crate) struct Table<I, T: Copy> {
    values: JournaledVec<T>,
    levels: JournaledVec<u16>,
    _index: PhantomData<fn(I)>,
}

impl<I, T: Copy> Default for Table<I, T> {
    fn default() -> Self {
        Self {
            values: JournaledVec::default(),
            levels: JournaledVec::default(),
            _index: PhantomData,
        }
    }
}

impl<I: EqLoc<Value = T>, T: EqValue> Table<I, T> {
    pub(crate) fn new(len: usize, value: T) -> Self {
        Self::at_level(len, value, LEVEL_ONE)
    }

    pub(crate) fn at_level(len: usize, value: T, level: u16) -> Self {
        Self {
            values: vec![value; len].into(),
            levels: vec![level; len].into(),
            ..Self::default()
        }
    }

    pub(crate) fn fill(&mut self, value: T, level: u16) {
        for k in 0..self.values.len() {
            self.values.set(k, value);
            self.levels.set(k, level);
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

    #[inline]
    pub(crate) fn set(&mut self, i: I, value: T, level: u16) {
        let k = i.index();
        self.values.set(k, value);
        self.levels.set(k, level);
    }

    #[inline]
    pub(crate) fn set_value(&mut self, i: I, value: T) {
        let k = i.index();
        self.values.set(k, value);
    }

    /// The values, for writes no checkpoint undoes, such as loading a format.
    pub(crate) fn values_unjournaled_mut(&mut self) -> &mut [T] {
        self.values.unjournaled_mut()
    }

    /// The levels, for writes no checkpoint undoes, such as loading a format.
    pub(crate) fn levels_unjournaled_mut(&mut self) -> &mut [u16] {
        self.levels.unjournaled_mut()
    }

    pub(crate) fn values(&self) -> &[T] {
        &self.values
    }

    pub(crate) fn levels(&self) -> &[u16] {
        &self.levels
    }
}

impl<I, T: JournalWord> Journaled for Table<I, T> {
    type Delta = TableDelta<T>;

    fn arm(&mut self) {
        self.values.arm();
        self.levels.arm();
    }

    fn disarm(&mut self) {
        self.values.disarm();
        self.levels.disarm();
    }

    fn seal(&mut self) -> TableDelta<T> {
        TableDelta {
            values: self.values.seal(),
            levels: self.levels.seal(),
        }
    }

    fn snapshot(&self) -> TableDelta<T> {
        TableDelta {
            values: self.values.snapshot(),
            levels: self.levels.snapshot(),
        }
    }

    fn apply(&mut self, delta: &TableDelta<T>) {
        self.values.apply(&delta.values);
        self.levels.apply(&delta.levels);
    }
}

/// Undoes one epoch of a [`Table`].
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

impl<T: Copy> Delta for TableDelta<T> {
    fn compose_with_older(&mut self, older: TableDelta<T>) {
        self.values.compose_with_older(older.values);
        self.levels.compose_with_older(older.levels);
    }

    fn compact(&mut self) {
        self.values.compact();
        self.levels.compact();
    }

    fn allocated_bytes(&self) -> usize {
        self.values.allocated_bytes() + self.levels.allocated_bytes()
    }
}

macro_rules! tables {
    ($($name:ident: $t:ty),* $(,)?) => {
        journaled_fields! {
            /// Undoes one epoch of `eqtb` and the hash.
            pub(crate) struct EqtbDelta for EqtbState {
                $($name: TableDelta<$t>,)*
                hash: WordDelta<HashEntry>,
                prim: WordDelta<PrimEntry>,
            }
        }

        impl EqtbState {
            pub(crate) fn release_tables(&mut self) {
                $(self.$name = Table::default();)*
                self.inter_char.clear();
            }

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
    pub(crate) fn save(&mut self, r: Restore) -> Result<(), Abort> {
        if self.save_stack_full() {
            return Err(self.overflow());
        }
        self.push(SaveEntry::Restore(r));
        Ok(())
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub(crate) struct InterCharPair {
    first: u16,
    second: u16,
}

impl InterCharPair {
    pub(crate) fn new(first: i32, second: i32) -> Option<InterCharPair> {
        let class = |c: i32| {
            u16::try_from(c)
                .ok()
                .filter(|&c| i32::from(c) < CHAR_CLASS_LIMIT)
        };
        Some(InterCharPair {
            first: class(first)?,
            second: class(second)?,
        })
    }

    pub(crate) fn classes(self) -> [i32; 2] {
        [i32::from(self.first), i32::from(self.second)]
    }
}

impl_eq_loc!(
    InterCharPair,
    SparseTable,
    TokenList,
    inter_char,
    InterChar,
    |p| usize::from(p.first) * 4096 + usize::from(p.second)
);

/// A register number: `\count`, `\dimen`, `\skip`, `\muskip`, `\toks` or `\box` 0 to 32767.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
pub(crate) struct Register(u16);

impl Register {
    pub(crate) const COUNT: u32 = 32768;

    pub(crate) fn new(n: i32) -> Option<Register> {
        u16::try_from(n)
            .ok()
            .filter(|&r| u32::from(r) < Self::COUNT)
            .map(Register)
    }

    pub(crate) const fn get(self) -> u16 {
        self.0
    }

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

macro_rules! eq_loc {
    ($(#[$doc:meta])* $loc:ident, $len:expr, $value:ty, $table:ident, $restore:ident) => {
        $(#[$doc])*
        #[derive(Copy, Clone, PartialEq, Eq, Debug)]
        pub(crate) struct $loc(u32);

        impl $loc {
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

        impl_eq_loc!($loc, Table, $value, $table, $restore, |l| {
            usize::try_from(l.0).unwrap_or(usize::MAX)
        });
    };
}

const INT_PARS: u32 = IntPar::COUNT as u32;
const DIMEN_PARS: u32 = DimenPar::COUNT as u32;
const GLUE_PARS: u32 = GluePar::COUNT as u32;
const TOK_PARS: u32 = TokPar::COUNT as u32;
const REGISTERS: usize = Register::COUNT as usize;

fn par_number(n: u32) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

fn register(n: u32) -> Register {
    Register(u16::try_from(n).unwrap_or(u16::MAX))
}

eq_loc!(IntLoc, INT_PARS as usize + REGISTERS, i32, ints, Int);

impl IntLoc {
    pub(crate) const fn par(p: IntPar) -> IntLoc {
        IntLoc(p as u32)
    }

    pub(crate) const fn count(n: Register) -> IntLoc {
        IntLoc(INT_PARS + n.0 as u32)
    }

    pub(crate) fn get(self) -> IntSlot {
        match self.0.checked_sub(INT_PARS) {
            Some(n) => IntSlot::Count(register(n)),
            None => IntSlot::Par(par_number(self.0)),
        }
    }
}

pub(crate) enum IntSlot {
    Par(u16),
    Count(Register),
}

eq_loc!(
    DimenLoc,
    DIMEN_PARS as usize + REGISTERS,
    Scaled,
    dimens,
    Dimen
);

impl DimenLoc {
    pub(crate) const fn par(p: DimenPar) -> DimenLoc {
        DimenLoc(p as u32)
    }

    pub(crate) const fn dimen(n: Register) -> DimenLoc {
        DimenLoc(DIMEN_PARS + n.0 as u32)
    }

    pub(crate) fn get(self) -> DimenSlot {
        match self.0.checked_sub(DIMEN_PARS) {
            Some(n) => DimenSlot::Dimen(register(n)),
            None => DimenSlot::Par(par_number(self.0)),
        }
    }
}

pub(crate) enum DimenSlot {
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
    GlueLoc,
    GLUE_PARS as usize + 2 * REGISTERS,
    GlueSpecId,
    glue,
    Glue
);

impl GlueLoc {
    pub(crate) const fn par(p: GluePar) -> GlueLoc {
        GlueLoc(p as u32)
    }

    pub(crate) const fn skip(n: Register) -> GlueLoc {
        GlueLoc(GLUE_PARS + n.0 as u32)
    }

    pub(crate) const fn mu_skip(n: Register) -> GlueLoc {
        GlueLoc(GLUE_PARS + Register::COUNT + n.0 as u32)
    }

    pub(crate) fn get(self) -> GlueSlot {
        match self.0.checked_sub(GLUE_PARS) {
            None => GlueSlot::Par(par_number(self.0)),
            Some(n) if n < Register::COUNT => GlueSlot::Skip(register(n)),
            Some(n) => GlueSlot::MuSkip(register(n - Register::COUNT)),
        }
    }
}

pub(crate) enum GlueSlot {
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
    ToksLoc,
    TOK_PARS as usize + REGISTERS,
    TokenList,
    toks,
    Toks
);

impl ToksLoc {
    pub(crate) const fn par(p: TokPar) -> ToksLoc {
        ToksLoc(p as u32 - TokPar::OutputRoutine as u32)
    }

    pub(crate) const fn register(n: Register) -> ToksLoc {
        ToksLoc(TOK_PARS + n.0 as u32)
    }

    pub(crate) fn get(self) -> ToksSlot {
        match self.0.checked_sub(TOK_PARS) {
            Some(n) => ToksSlot::Register(register(n)),
            None => ToksSlot::Par(par_number(self.0) + TokPar::OutputRoutine as u16),
        }
    }
}

pub(crate) enum ToksSlot {
    Par(u16),
    Register(Register),
}

code_enum! {
    pub(crate) enum ShapeLoc: u8 {
        ParShape = 0 => "parshape",
        InterLinePenalties = 1 => "interlinepenalties",
        ClubPenalties = 2 => "clubpenalties",
        WidowPenalties = 3 => "widowpenalties",
        DisplayWidowPenalties = 4 => "displaywidowpenalties",
    }
}

impl ShapeLoc {
    pub(crate) const LEN: usize = ShapeLoc::DisplayWidowPenalties as usize + 1;
}

impl_eq_loc!(ShapeLoc, Table, ShapeRef, shapes, Shape, |l| l as usize);

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

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) struct BoxLoc(Register);

impl BoxLoc {
    pub(crate) const LEN: usize = REGISTERS;

    pub(crate) const fn register(n: Register) -> BoxLoc {
        BoxLoc(n)
    }

    pub(crate) fn raw(self) -> i32 {
        i32::from(self.0.0)
    }
}

impl_eq_loc!(BoxLoc, Table, Option<NodePtr<BoxNode>>, boxes, Box, |l| {
    usize::from(l.0.0)
});

eq_loc!(FontLoc, 1 + 3 * 256, FontId, fonts, Font);

impl FontLoc {
    pub(crate) const CUR: FontLoc = FontLoc(0);
    pub(crate) const TEXT: FontLoc = FontLoc(1);
    pub(crate) const SCRIPT: FontLoc = FontLoc(1 + 256);
    pub(crate) const SCRIPT_SCRIPT: FontLoc = FontLoc(1 + 512);

    /// The math font `n` past `\textfont0`: a family plus `TEXT_SIZE`, `SCRIPT_SIZE` or
    /// `SCRIPT_SCRIPT_SIZE`. Out of range for any other `n`.
    #[inline]
    pub(crate) fn math(n: i32) -> FontLoc {
        FontLoc(u32::try_from(n + 1).unwrap_or(u32::MAX))
    }

    pub(crate) fn family(self, fam: u8) -> FontLoc {
        FontLoc(self.0 + u32::from(fam))
    }

    pub(crate) fn math_offset(self) -> i32 {
        self.raw() - 1
    }
}

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

        impl_eq_loc!($loc, Table, $value, $table, $restore, |l| {
            usize::try_from(l.0).unwrap_or(usize::MAX)
        });
    };
}

code_loc!(CatLoc, Cmd, cat_codes, Cat);
code_loc!(LcLoc, i32, lc_codes, Lc);
code_loc!(UcLoc, i32, uc_codes, Uc);
code_loc!(
    /// An `\sfcode` entry: the space factor, and XeTeX's character class times 65536.
    SfLoc,
    i32,
    sf_codes,
    Sf
);
code_loc!(
    /// A `\mathcode` entry, in XeTeX's encoding.
    MathLoc,
    i32,
    math_codes,
    Math
);
code_loc!(DelLoc, i32, del_codes, Del);

code_enum! {
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
    pub(crate) enum XetexCode: u8 {
        CharClass = 0 => "XeTeXcharclass",
        MathCodeNum = 1 => "Umathcodenum",
        MathCode = 2 => "Umathcode",
        DelCodeNum = 3 => "Udelcodenum",
        DelCode = 4 => "Udelcode",
    }
}

code_enum! {
    pub(crate) enum CaseTable: u8 {
        Lower = 0 => "lowercase",
        Upper = 1 => "uppercase",
    }
}

pub(crate) struct EqtbState {
    /// The lowest bucket handed out to a colliding name so far.
    pub hash_used: NameSlot,
    pub hash_high: u32,
    pub no_new_control_sequence: bool,
    pub prim_used: PrimNum,
    pub cs_count: i32,
    pub meanings: Table<CsId, Command>,
    pub ints: Table<IntLoc, i32>,
    pub dimens: Table<DimenLoc, Scaled>,
    pub glue: Table<GlueLoc, GlueSpecId>,
    pub toks: Table<ToksLoc, TokenList>,
    pub shapes: Table<ShapeLoc, ShapeRef>,
    pub fonts: Table<FontLoc, FontId>,
    pub cat_codes: Table<CatLoc, Cmd>,
    pub lc_codes: Table<LcLoc, i32>,
    pub uc_codes: Table<UcLoc, i32>,
    pub sf_codes: Table<SfLoc, i32>,
    pub math_codes: Table<MathLoc, i32>,
    pub del_codes: Table<DelLoc, i32>,
    pub inter_char: SparseTable<InterCharPair, TokenList>,
    pub boxes: Table<BoxLoc, Option<NodePtr<BoxNode>>>,
    pub prim: JournaledVec<PrimEntry>,
    /// The `next` and `text` of each control sequence from the first hash bucket on.
    pub hash: JournaledVec<HashEntry>,
    pub hash_extra: u32,
    pub cs_top: CsId,
    pub pgfmath: KnownCs,
}

impl Default for EqtbState {
    fn default() -> Self {
        EqtbState {
            hash_used: NameSlot::BUCKETS_END,
            hash_high: 0,
            no_new_control_sequence: false,
            prim_used: PrimNum::LAST,
            cs_count: 0,
            meanings: Default::default(),
            inter_char: Default::default(),
            ints: Default::default(),
            dimens: Default::default(),
            glue: Default::default(),
            toks: Default::default(),
            shapes: Default::default(),
            boxes: Default::default(),
            fonts: Default::default(),
            cat_codes: Default::default(),
            lc_codes: Default::default(),
            uc_codes: Default::default(),
            sf_codes: Default::default(),
            math_codes: Default::default(),
            del_codes: Default::default(),
            prim: vec![PrimEntry::default(); 2101].into(),
            hash: JournaledVec::default(),
            hash_extra: 0,
            cs_top: CsId::UNDEFINED,
            pgfmath: KnownCs::default(),
        }
    }
}

impl EqtbState {
    pub(crate) fn alloc_tables(&mut self) {
        self.cs_top = CsId::top(self.hash_extra);
        let cs = self.cs_top.index() + 1;
        self.meanings = Table::at_level(cs, Command::UndefinedCs, LEVEL_ZERO);
        self.hash = zeroed_vec(self.cs_top.hash_index().map_or(0, |i| i + 1)).into();
        self.ints = Table::new(IntLoc::LEN, 0);
        self.inter_char.clear();
        self.dimens = Table::new(DimenLoc::LEN, Scaled::ZERO);
        self.glue = Table::new(GlueLoc::LEN, ZERO_GLUE);
        self.toks = Table::new(ToksLoc::LEN, TokenList::NULL);
        self.shapes = Table::new(ShapeLoc::LEN, ShapeRef::NONE);
        self.boxes = Table::new(BoxLoc::LEN, None);
        self.fonts = Table::new(FontLoc::LEN, NULL_FONT);
        let usvs = NUMBER_USVS as usize;
        self.cat_codes = Table::new(usvs, Cmd::ESCAPE);
        self.lc_codes = Table::new(usvs, 0);
        self.uc_codes = Table::new(usvs, 0);
        self.sf_codes = Table::new(usvs, 0);
        self.math_codes = Table::new(usvs, 0);
        self.del_codes = Table::new(usvs, -1);
    }
}

impl EqtbState {
    #[inline(always)]
    pub(crate) fn meaning(&self, cs: CsId) -> Command {
        self.meanings.get(cs)
    }

    #[inline]
    pub(crate) fn cs_level(&self, cs: CsId) -> u16 {
        self.meanings.level(cs)
    }

    #[inline]
    pub(crate) fn set_meaning(&mut self, cs: CsId, m: Command, level: u16) {
        self.meanings.set(cs, m, level);
    }

    #[inline]
    pub(crate) fn hash_next(&self, p: CsId) -> Option<CsId> {
        p.hash_index()
            .and_then(|i| self.hash.get(i))
            .and_then(|h| h.next)
    }

    #[inline]
    pub(crate) fn set_hash_next(&mut self, p: CsId, next: CsId) {
        let Some(index) = p.hash_index() else {
            return;
        };
        self.hash.at_mut(index).next = Some(next);
    }

    /// The name of control sequence `p`, `text(p)`, none below the first hash bucket.
    #[inline]
    pub fn text(&self, p: CsId) -> StrNumber {
        p.hash_index()
            .and_then(|i| self.hash.get(i))
            .map_or(StrNumber::NONE, |h| h.text)
    }

    #[inline]
    pub fn set_text(&mut self, p: CsId, s: StrNumber) {
        let Some(index) = p.hash_index() else {
            return;
        };
        self.hash.at_mut(index).text = s;
    }

    #[inline]
    pub fn prim_next(&self, p: PrimNum) -> Option<PrimNum> {
        self.prim[p.index()].next
    }

    #[inline]
    pub fn set_prim_next(&mut self, p: PrimNum, next: Option<PrimNum>) {
        self.prim.at_mut(p.index()).next = next;
    }

    #[inline]
    pub fn prim_text(&self, p: PrimNum) -> Option<StrNumber> {
        self.prim[p.index()].text
    }

    #[inline]
    pub fn set_prim_text(&mut self, p: PrimNum, s: Option<StrNumber>) {
        self.prim.at_mut(p.index()).text = s;
    }

    #[inline]
    pub(crate) fn font_id_text(&self, f: FontId) -> StrNumber {
        self.text(CsId::font_id(f))
    }

    #[inline]
    pub(crate) fn cur_font(&self) -> FontId {
        self.fonts.get(FontLoc::CUR)
    }

    #[inline]
    pub(crate) fn tok_par(&self, n: TokPar) -> TokenList {
        self.toks.get(n.loc())
    }
    #[inline]
    pub(crate) fn par_shape(&self) -> Option<NodePtr<ParShapeNode>> {
        NodePtr::from_ptr(self.shapes.get(ShapeLoc::ParShape).ptr())
    }
    #[inline]
    pub(crate) fn penalties(&self, loc: ShapeLoc) -> Option<NodePtr<PenaltiesNode>> {
        NodePtr::from_ptr(self.shapes.get(loc).ptr())
    }
    #[inline]
    pub(crate) fn box_reg(&self, n: impl Into<Register>) -> Option<NodePtr<BoxNode>> {
        self.boxes.get(BoxLoc::register(n.into()))
    }
    #[inline]
    pub(crate) fn set_box_reg(&mut self, n: impl Into<Register>, b: Option<NodePtr<BoxNode>>) {
        self.boxes.set_value(BoxLoc::register(n.into()), b);
    }
    #[inline]
    pub(crate) fn skip(&self, n: impl Into<Register>) -> GlueSpecId {
        self.glue.get(GlueLoc::skip(n.into()))
    }
    #[inline]
    pub(crate) fn dimen(&self, n: impl Into<Register>) -> Scaled {
        self.dimens.get(DimenLoc::dimen(n.into()))
    }
    #[inline]
    pub(crate) fn toks(&self, n: impl Into<Register>) -> TokenList {
        self.toks.get(ToksLoc::register(n.into()))
    }
    #[inline]
    pub(crate) fn count(&self, n: impl Into<Register>) -> i32 {
        self.ints.get(IntLoc::count(n.into()))
    }
    #[inline]
    pub(crate) fn math_font(&self, n: i32) -> FontId {
        self.fonts.get(FontLoc::math(n))
    }
    #[inline(always)]
    pub(crate) fn cat_code(&self, c: i32) -> Cmd {
        self.cat_codes.get(CatLoc::of(c))
    }
    #[inline]
    pub(crate) fn lc_code(&self, c: i32) -> i32 {
        self.lc_codes.get(LcLoc::of(c))
    }
    #[inline]
    pub(crate) fn uc_code(&self, c: i32) -> i32 {
        self.uc_codes.get(UcLoc::of(c))
    }
    #[inline]
    pub(crate) fn sf_code(&self, c: i32) -> i32 {
        self.sf_codes.get(SfLoc::of(c))
    }
    #[inline]
    pub(crate) fn math_code(&self, c: i32) -> i32 {
        self.math_codes.get(MathLoc::of(c))
    }
    #[inline]
    pub(crate) fn del_code(&self, c: i32) -> i32 {
        self.del_codes.get(DelLoc::of(c))
    }

    #[inline]
    pub fn int_par(&self, n: IntPar) -> i32 {
        self.ints.get(n.loc())
    }
    #[inline]
    pub fn set_int_par(&mut self, n: IntPar, v: i32) {
        self.ints.set_value(n.loc(), v);
    }
    #[inline]
    pub fn dimen_par(&self, n: DimenPar) -> Scaled {
        self.dimens.get(n.loc())
    }
    #[inline]
    pub fn set_dimen_par(&mut self, n: DimenPar, v: Scaled) {
        self.dimens.set_value(n.loc(), v);
    }
    /// The spec of glue parameter `n`, never null.
    #[inline]
    pub fn glue_par(&self, n: GluePar) -> GlueSpecId {
        self.glue.get(n.loc())
    }

    #[inline]
    pub fn set_glue_par(&mut self, n: GluePar, spec: GlueSpecId) {
        self.glue.set_value(n.loc(), spec);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::consts::LEVEL_ZERO;
    use crate::tables::save::unsave;

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
        unsave(&mut st).unwrap();
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
