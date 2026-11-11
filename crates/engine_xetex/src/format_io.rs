//! Format-file dump and load (`\dump` and the preloaded format).
//!
//! Each section of the file is described once, generic over a `FmtStream`:
//! dumping writes every item from the engine state and loading reads it back
//! into the same place, checking ranges. Items are stored big-endian, an
//! 8-byte word as one 64-bit value.

use crate::bridge::{input_open, input_read, output_close, output_open, output_write, FileFormat};
use crate::command::Command;
use crate::consts::{
    ACTIVE_BASE, CAT_CODE_BASE, CUR_FONT_LOC, DEL_CODE_BASE, EMPTY_STRING, EQTB_SIZE, FONT_BASE,
    FROZEN_CONTROL_SEQUENCE, HASH_BASE, INT_BASE, INVALID_HANDLE, MAX_FONT_MAX, MAX_HALFWORD,
    MIN_HALFWORD, NULL_FONT, NUMBER_USVS, PRIM_SIZE, TEX_NULL, TOO_BIG_CHAR,
    UNDEFINED_CONTROL_SEQUENCE,
};
use crate::engine::{POOL_FREE, STRINGS_FREE};
use crate::errors::Abort;
use crate::fonts::LigChar;
use crate::fonts::{AAT_FONT_FLAG, OTGR_FONT_FLAG};
use crate::glue_store::GlueSpecId;
use crate::glue_store::{GlueSpec, GlueStore, STATIC_SPECS};
use crate::hash::{hash_slot, HASH_PRIME};
use crate::hyphenation::{TrieOp, TriePtr, BIGGEST_LANG, HYPH_PRIME, TRIE_OP_SIZE};
use crate::mem::{CONTRIB_HEAD, PAGE_HEAD, RESERVED_WORDS};
use crate::mem_types::{B16x4, B32x2, MemoryWord};
use crate::output::{print, print_char, print_int, print_ln, print_nl};
use crate::params::IntPar;
use crate::payload::PayloadStore;
use crate::state::{EngineState, EqtbState, InteractionMode, PerFont, Selector};
use crate::token::{ListSlot, Token, TokenStore};
use crate::units::CsId;
use crate::units::MemIdx;
use crate::units::RawPtr;
use crate::units::{EqtbPtr, FontId, FontInfoIdx, MemPtr, PoolPos, SavePtr, Scaled, StrNumber};

const FORMAT_HEADER_MAGIC: i32 = 0x5454_4E43;
const FORMAT_FOOTER_MAGIC: i32 = 0x0000_029A;
const SUP_MAX_STRINGS: i32 = 2_097_151;
const SUP_FONT_MEM_SIZE: i32 = 147_483_647;
const SUP_POOL_SIZE: i32 = 40_000_000;
const SUP_HASH_EXTRA: i32 = 2_097_151;

fn bad_fmt() -> Abort {
    Abort::Internal("fatal format file error")
}

/// A value with a fixed-size encoding in the format file.
trait FmtItem: Copy {
    const SIZE: usize;
    fn put(self, out: &mut Vec<u8>);
    fn get(bytes: &[u8]) -> Self;
}

macro_rules! int_item {
    ($($t:ty),*) => {$(
        impl FmtItem for $t {
            const SIZE: usize = std::mem::size_of::<$t>();
            #[inline]
            fn put(self, out: &mut Vec<u8>) {
                out.extend_from_slice(&self.to_be_bytes());
            }
            #[inline]
            fn get(bytes: &[u8]) -> Self {
                <$t>::from_be_bytes(bytes.try_into().expect("SIZE bytes"))
            }
        }
    )*};
}

int_item!(u16, i16, i32, u32);

macro_rules! newtype_item {
    ($($t:ident),*) => {$(
        impl FmtItem for $t {
            const SIZE: usize = 4;
            #[inline]
            fn put(self, out: &mut Vec<u8>) {
                self.raw().put(out);
            }
            #[inline]
            fn get(bytes: &[u8]) -> Self {
                $t::from_raw(i32::get(bytes))
            }
        }
    )*};
}

impl FmtItem for MemPtr {
    const SIZE: usize = 4;
    #[inline]
    fn put(self, out: &mut Vec<u8>) {
        self.raw().put(out);
    }
    #[inline]
    fn get(bytes: &[u8]) -> Self {
        MemIdx::from_raw(i32::get(bytes))
    }
}

newtype_item!(
    Scaled,
    StrNumber,
    EqtbPtr,
    CsId,
    PoolPos,
    FontInfoIdx,
    TriePtr,
    LigChar
);

impl FmtItem for B32x2 {
    const SIZE: usize = 8;
    #[inline]
    fn put(self, out: &mut Vec<u8>) {
        self.s1.put(out);
        self.s0.put(out);
    }
    #[inline]
    fn get(bytes: &[u8]) -> Self {
        B32x2 {
            s1: i32::get(&bytes[..4]),
            s0: i32::get(&bytes[4..]),
        }
    }
}

impl FmtItem for MemoryWord {
    const SIZE: usize = 8;
    #[inline]
    fn put(self, out: &mut Vec<u8>) {
        self.b32().put(out);
    }
    #[inline]
    fn get(bytes: &[u8]) -> Self {
        MemoryWord {
            b32: B32x2::get(bytes),
        }
    }
}

impl FmtItem for GlueSpec {
    const SIZE: usize = 20;
    #[inline]
    fn put(self, out: &mut Vec<u8>) {
        for w in bytemuck::cast::<_, [u32; 5]>(self) {
            w.put(out);
        }
    }
    #[inline]
    fn get(bytes: &[u8]) -> Self {
        let mut words = [0u32; 5];
        for (w, chunk) in words.iter_mut().zip(bytes.chunks_exact(4)) {
            *w = u32::get(chunk);
        }
        bytemuck::cast(words)
    }
}

impl FmtItem for Token {
    const SIZE: usize = 4;
    #[inline]
    fn put(self, out: &mut Vec<u8>) {
        self.0.put(out);
    }
    #[inline]
    fn get(bytes: &[u8]) -> Self {
        Token(i32::get(bytes))
    }
}

impl FmtItem for ListSlot {
    const SIZE: usize = 12;
    #[inline]
    fn put(self, out: &mut Vec<u8>) {
        for w in bytemuck::cast::<_, [u32; 3]>(self) {
            w.put(out);
        }
    }
    #[inline]
    fn get(bytes: &[u8]) -> Self {
        bytemuck::cast([0, 4, 8].map(|i| u32::get(&bytes[i..i + 4])))
    }
}

impl FmtItem for B16x4 {
    const SIZE: usize = 8;
    #[inline]
    fn put(self, out: &mut Vec<u8>) {
        for q in [self.s3, self.s2, self.s1, self.s0] {
            q.put(out);
        }
    }
    #[inline]
    fn get(bytes: &[u8]) -> Self {
        B16x4 {
            s3: u16::get(&bytes[0..2]),
            s2: u16::get(&bytes[2..4]),
            s1: u16::get(&bytes[4..6]),
            s0: u16::get(&bytes[6..8]),
        }
    }
}

/// A TFM check sum, one 16-bit quarter per byte.
impl FmtItem for [u8; 4] {
    const SIZE: usize = 8;
    #[inline]
    fn put(self, out: &mut Vec<u8>) {
        for b in self {
            u16::from(b).put(out);
        }
    }
    #[inline]
    fn get(bytes: &[u8]) -> Self {
        [bytes[1], bytes[3], bytes[5], bytes[7]]
    }
}

/// One direction over the format file.
trait FmtStream {
    const LOADING: bool;

    fn items<T: FmtItem>(&mut self, v: &mut [T]) -> Result<(), Abort>;

    fn int(&mut self, v: &mut i32) -> Result<(), Abort> {
        self.items(std::slice::from_mut(v))
    }

    /// Items that must lie in `low..=high` when loaded.
    fn items_in<T: FmtItem + PartialOrd>(
        &mut self,
        v: &mut [T],
        low: T,
        high: T,
    ) -> Result<(), Abort> {
        self.items(v)?;
        if Self::LOADING && v.iter().any(|&x| x < low || x > high) {
            return Err(bad_fmt());
        }
        Ok(())
    }

    fn int_in(&mut self, v: &mut i32, low: i32, high: i32) -> Result<(), Abort> {
        self.items_in(std::slice::from_mut(v), low, high)
    }

    fn ptr_in(&mut self, v: &mut MemPtr, low: i32, high: i32) -> Result<(), Abort> {
        self.items_in(
            std::slice::from_mut(v),
            MemIdx::from_raw(low),
            MemIdx::from_raw(high),
        )
    }

    /// A size that must be at least `low` and fit the engine's `high` limit.
    fn size<T: FmtItem + PartialOrd>(
        &mut self,
        v: &mut T,
        low: T,
        high: T,
        too_big: &'static str,
    ) -> Result<(), Abort> {
        self.items(std::slice::from_mut(v))?;
        if Self::LOADING {
            if *v < low {
                return Err(bad_fmt());
            }
            if *v > high {
                return Err(Abort::Internal(too_big));
            }
        }
        Ok(())
    }

    fn constant(&mut self, c: i32) -> Result<(), Abort> {
        let mut v = c;
        self.int(&mut v)?;
        if v != c {
            return Err(bad_fmt());
        }
        Ok(())
    }
}

struct Dump {
    handle: usize,
    buf: Vec<u8>,
}

impl FmtStream for Dump {
    const LOADING: bool = false;

    fn items<T: FmtItem>(&mut self, v: &mut [T]) -> Result<(), Abort> {
        if v.is_empty() {
            return Ok(());
        }
        self.buf.clear();
        self.buf.reserve(v.len() * T::SIZE);
        for &x in v.iter() {
            x.put(&mut self.buf);
        }
        if !output_write(self.handle, &self.buf) {
            return Err(Abort::Internal("could not write format file data"));
        }
        Ok(())
    }
}

struct Undump {
    handle: usize,
    buf: Box<[u8]>,
}

impl Undump {
    const CHUNK: usize = 1 << 16;

    fn new(handle: usize) -> Undump {
        Undump {
            handle,
            buf: vec![0; Self::CHUNK].into_boxed_slice(),
        }
    }
}

impl FmtStream for Undump {
    const LOADING: bool = true;

    fn items<T: FmtItem>(&mut self, v: &mut [T]) -> Result<(), Abort> {
        for chunk in v.chunks_mut(Self::CHUNK / T::SIZE) {
            let bytes = &mut self.buf[..chunk.len() * T::SIZE];
            if !input_read(self.handle, bytes) {
                return Err(Abort::Internal("could not read format file data"));
            }
            for (x, b) in chunk.iter_mut().zip(bytes.chunks(T::SIZE)) {
                *x = T::get(b);
            }
        }
        Ok(())
    }
}

fn span<T>(v: &mut [T], start: i32, n: i32) -> &mut [T] {
    if n <= 0 {
        return &mut [];
    }
    &mut v[start as usize..(start + n) as usize]
}

/// The per-font entries `font_base..=font_ptr`.
fn fonts_of<T>(v: &mut PerFont<T>, font_ptr: FontId) -> &mut [T] {
    &mut v.as_mut_slice()[FONT_BASE as usize..=font_ptr.index()]
}

pub(crate) fn store_fmt_file(st: &mut EngineState) -> Result<(), Abort> {
    if st.save_stack.save_ptr != SavePtr::ZERO {
        return Err(Abort::DumpInGroup);
    }

    st.shipout.selector = Selector::NewString;
    print(st, " (preloaded format=");
    print(st, st.strings.job_name);
    print_char(st, b' ');
    print_int(st, st.eqtb.int_par(IntPar::Year));
    print_char(st, b'.');
    print_int(st, st.eqtb.int_par(IntPar::Month));
    print_char(st, b'.');
    print_int(st, st.eqtb.int_par(IntPar::Day));
    print_char(st, b')');

    st.shipout.selector = if st.errors.interaction == InteractionMode::Batch {
        Selector::LogOnly
    } else {
        Selector::TermAndLog
    };

    if st.strings.pool_ptr + 1 > st.strings.pool_size {
        return Err(st.strings.pool_overflow());
    }

    st.strings.format_ident = st.strings.make_string()?;
    let path = crate::io::pack_job_name(st, ".fmt")?;

    let handle = output_open(&path, false);
    if handle == INVALID_HANDLE {
        return Err(Abort::Internal("cannot open format output file"));
    }

    print_nl(st, "Beginning to dump on file ");
    let name_string = crate::io::make_name_string(st, &path)?;
    print(st, name_string);

    st.strings.flush_string();

    print_nl(st, "");
    print(st, st.strings.format_ident);

    while st.input.pseudo_files.is_some() {
        st.mem.pseudo_close(&mut st.input)?;
    }

    let mut dump = Dump {
        handle,
        buf: Vec::new(),
    };
    format_file(&mut dump, st)?;
    st.eqtb.set_int_par(IntPar::TracingStats, 0);
    output_close(handle);
    Ok(())
}

pub fn load_fmt_file(st: &mut EngineState, dump_name: &str) -> Result<(), Abort> {
    let loc = st.input.cur_input.loc;

    let handle = input_open(dump_name.as_bytes(), FileFormat::Format, false);
    if handle == INVALID_HANDLE {
        return Err(Abort::Internal("cannot open the format file"));
    }

    st.input.cur_input.loc = loc;

    if st.config.in_initex_mode {
        st.fonts.font_info = Vec::new();
        st.strings.str_pool = Vec::new();
        st.strings.str_start = Vec::new();
        st.eqtb.data = Vec::new();
        st.eqtb.hash = Vec::new();
        st.mem.data = Vec::new();
    }

    format_file(&mut Undump::new(handle), st)?;

    if crate::io::input_close(handle) {
        return Err(Abort::Internal(
            "serious internal error closing the format file",
        ));
    }
    Ok(())
}

fn format_file<S: FmtStream>(s: &mut S, st: &mut EngineState) -> Result<(), Abort> {
    header(s, st)?;
    string_pool(s, st)?;
    dynamic_memory(s, st)?;
    table_of_equivalents(s, st)?;
    font_info(s, st)?;
    hyphenation_tables(s, st)?;
    s.constant(FORMAT_FOOTER_MAGIC)
}

fn header<S: FmtStream>(s: &mut S, st: &mut EngineState) -> Result<(), Abort> {
    s.constant(FORMAT_HEADER_MAGIC)?;
    let serial = crate::FORMAT_SERIAL as i32;
    let mut v = serial;
    s.int(&mut v)?;
    if v != serial {
        return Err(Abort::Internal("format file is of the wrong version"));
    }

    let eqtb = &mut st.eqtb;
    s.int_in(&mut eqtb.hash_high, 0, SUP_HASH_EXTRA)?;
    if S::LOADING {
        eqtb.hash_extra = eqtb.hash_extra.max(eqtb.hash_high);
        eqtb.alloc_tables();

        st.scanner.max_reg_num = 32_767;
    }

    if S::LOADING {
        st.input.cur_list.head = CONTRIB_HEAD;
        st.input.cur_list.tail = CONTRIB_HEAD;
        st.shipout.page_tail = Some(PAGE_HEAD);
    }

    s.constant(EQTB_SIZE.raw())?;
    s.constant(HASH_PRIME)?;
    s.constant(HYPH_PRIME)
}

fn string_pool<S: FmtStream>(s: &mut S, st: &mut EngineState) -> Result<(), Abort> {
    let strings = &mut st.strings;
    s.size(
        &mut strings.pool_ptr,
        PoolPos::ZERO,
        PoolPos::from_raw(SUP_POOL_SIZE - POOL_FREE),
        "must increase string_pool_size",
    )?;
    s.size(
        &mut strings.str_ptr,
        StrNumber::NONE,
        StrNumber::from_raw(SUP_MAX_STRINGS - STRINGS_FREE),
        "must increase sup_strings",
    )?;
    let (pool_ptr, str_ptr) = (strings.pool_ptr, strings.str_ptr);
    if S::LOADING {
        strings.pool_size = strings.pool_size.max(pool_ptr + POOL_FREE);
        strings.max_strings = strings.max_strings.max(str_ptr.offset(STRINGS_FREE));
        strings.str_start = vec![PoolPos::ZERO; strings.max_strings.slot() + 1];
        strings.str_pool = vec![0; strings.pool_size.index() + 1];
    }

    let starts = str_ptr - StrNumber::from_raw(TOO_BIG_CHAR) + 1;
    s.items_in(
        span(&mut strings.str_start, 0, starts),
        PoolPos::ZERO,
        pool_ptr,
    )?;
    s.items(span(&mut strings.str_pool, 0, pool_ptr.raw()))?;

    if S::LOADING {
        strings.init_str_ptr = str_ptr;
        strings.init_pool_ptr = pool_ptr;
    } else {
        print_ln(st);
        print_int(st, str_ptr.raw());
        print(st, " strings of total length ");
        print_int(st, pool_ptr.raw());
    }
    Ok(())
}

fn dynamic_memory<S: FmtStream>(s: &mut S, st: &mut EngineState) -> Result<(), Abort> {
    let mem = &mut st.mem;
    let too_big = || Abort::Internal("mem exceeds the format's limit");
    let mut words = i32::try_from(mem.data.len()).map_err(|_| too_big())?;
    let reserved = i32::try_from(RESERVED_WORDS).map_err(|_| too_big())?;
    s.int_in(&mut words, reserved, i32::MAX)?;
    let last = MemIdx::from_raw(words - 1);
    s.items_in(&mut mem.free.0, None, last)?;
    s.items_in(st.scanner.sa_root.registers_mut(), None, last)?;
    if S::LOADING {
        mem.data = crate::mem_types::zeroed_vec(usize::try_from(words).map_err(|_| bad_fmt())?);
    }
    s.items(&mut mem.data)?;
    token_lists(s, &mut mem.tokens)?;
    glue_specs(s, &mut mem.glue)?;
    payloads(s, &mut mem.payloads)?;

    if !S::LOADING {
        print_ln(st);
        print_int(st, words);
        print(st, " memory locations dumped");
    }
    Ok(())
}

/// The array node payloads, each as its length, `-1` for a free slot, then its words.
fn payloads<S: FmtStream>(s: &mut S, store: &mut PayloadStore) -> Result<(), Abort> {
    let too_big = || Abort::Internal("payload store exceeds the format's limit");
    let mut slots: Vec<Option<Vec<u32>>> = store.slots().map(|w| w.map(<[u32]>::to_vec)).collect();
    let mut n = i32::try_from(slots.len()).map_err(|_| too_big())?;
    s.int_in(&mut n, 0, i32::MAX)?;
    if S::LOADING {
        slots = vec![None; usize::try_from(n).map_err(|_| bad_fmt())?];
    }
    for slot in &mut slots {
        let mut len = match slot {
            Some(words) => i32::try_from(words.len()).map_err(|_| too_big())?,
            None => -1,
        };
        s.int_in(&mut len, -1, i32::MAX)?;
        if S::LOADING {
            *slot = usize::try_from(len).ok().map(|len| vec![0; len]);
        }
        if let Some(words) = slot {
            s.items(words)?;
        }
    }
    if S::LOADING {
        store.load(slots);
    }
    Ok(())
}

fn glue_specs<S: FmtStream>(s: &mut S, glue: &mut GlueStore) -> Result<(), Abort> {
    let marks = glue.marks();
    let too_big = || Abort::Internal("glue store exceeds the format's limit");
    let mut len = i32::try_from(marks.len).map_err(|_| too_big())?;
    let mut free = marks.free;
    let fixed = i32::try_from(STATIC_SPECS).map_err(|_| too_big())?;
    s.int_in(&mut len, fixed, i32::MAX)?;
    s.int(&mut free)?;
    if S::LOADING && free != TEX_NULL && !(fixed..len).contains(&free) {
        return Err(bad_fmt());
    }
    if S::LOADING {
        glue.reset_for_load(usize::try_from(len).map_err(|_| bad_fmt())?, free);
    }
    s.items(glue.specs_mut())
}

fn token_lists<S: FmtStream>(s: &mut S, toks: &mut TokenStore) -> Result<(), Abort> {
    let mut marks = toks.marks();
    let too_big = || Abort::Internal("token store exceeds the format's limit");
    let mut tokens = i32::try_from(marks.tokens).map_err(|_| too_big())?;
    let mut lists = i32::try_from(marks.lists).map_err(|_| too_big())?;
    s.int_in(&mut tokens, 1, i32::MAX)?;
    s.int_in(&mut lists, 2, i32::MAX)?;
    s.items(&mut marks.free_runs)?;
    s.items(std::slice::from_mut(&mut marks.free_lists))?;
    if S::LOADING {
        marks.tokens = usize::try_from(tokens).map_err(|_| bad_fmt())?;
        marks.lists = usize::try_from(lists).map_err(|_| bad_fmt())?;
        toks.reset_for_load(marks);
    }
    let (tokens, lists) = toks.parts_mut();
    s.items(tokens)?;
    s.items(lists)?;
    if S::LOADING && !toks.is_consistent() {
        return Err(bad_fmt());
    }
    Ok(())
}

/// Dumps `v` as alternating runs of distinct values and of repeats of the preceding value.
fn dump_runs<S: FmtStream, T: FmtItem>(
    s: &mut S,
    v: &mut [T],
    same: impl Fn(&T, &T) -> bool,
) -> Result<(), Abort> {
    let count = |n: usize| i32::try_from(n).map_err(|_| Abort::Internal("eqtb run too long"));
    let last = v.len().saturating_sub(1);
    let mut k = 0;
    loop {
        let mut j = k;
        while j < last && !same(&v[j], &v[j + 1]) {
            j += 1;
        }
        let l = if j < last {
            j += 1;
            let l = j;
            while j < last && same(&v[j], &v[j + 1]) {
                j += 1;
            }
            l
        } else {
            last + 1
        };
        s.int(&mut count(l - k)?)?;
        s.items(&mut v[k..l])?;
        k = j + 1;
        s.int(&mut count(k - l)?)?;
        if k > last {
            return Ok(());
        }
    }
}

/// Reads the runs [`dump_runs`] wrote into `v`, which they must fill exactly.
fn undump_runs<S: FmtStream, T: FmtItem>(s: &mut S, v: &mut [T]) -> Result<(), Abort> {
    let mut k = 0;
    loop {
        let mut x = 0;
        s.int(&mut x)?;
        let n = usize::try_from(x)
            .ok()
            .filter(|&n| n >= 1 && k + n <= v.len());
        let n = n.ok_or_else(bad_fmt)?;
        s.items(&mut v[k..k + n])?;
        k += n;

        s.int(&mut x)?;
        let n = usize::try_from(x).ok().filter(|&n| k + n <= v.len());
        let n = n.ok_or_else(bad_fmt)?;
        let repeated = v[k - 1];
        v[k..k + n].fill(repeated);
        k += n;
        if k == v.len() {
            return Ok(());
        }
    }
}

/// Dumps or loads the values of `v` as runs.
fn runs<S: FmtStream, T: FmtItem + PartialEq>(s: &mut S, v: &mut [T]) -> Result<(), Abort> {
    if S::LOADING {
        undump_runs(s, v)
    } else {
        dump_runs(s, v, T::eq)
    }
}

/// The meanings of `from..=to` in the format's word layout.
fn cs_words(eqtb: &EqtbState, from: CsId, to: CsId) -> Vec<MemoryWord> {
    from.through(to)
        .map(|cs| EqtbState::cs_entry(eqtb.meaning(cs), eqtb.cs_level(cs)))
        .collect()
}

/// Sets the meanings from `from` on to the words a format holds, rejecting any that is no meaning.
fn set_cs_words(eqtb: &mut EqtbState, from: CsId, words: &[MemoryWord]) -> Result<(), Abort> {
    let last = i32::try_from(words.len()).map_err(|_| bad_fmt())? - 1;
    for (cs, &w) in from.through(from + last).zip(words) {
        let (m, level) = EqtbState::from_cs_entry(w).ok_or_else(bad_fmt)?;
        eqtb.set_meaning(cs, m, level);
    }
    Ok(())
}

fn table_of_equivalents<S: FmtStream>(s: &mut S, st: &mut EngineState) -> Result<(), Abort> {
    let eqtb = &mut st.eqtb;
    let same_entry = |a: &MemoryWord, b: &MemoryWord| a.b32() == b.b32();
    let same_int = |a: &MemoryWord, b: &MemoryWord| a.b32().s1 == b.b32().s1;
    let extra_top = UNDEFINED_CONTROL_SEQUENCE + eqtb.hash_high;
    let mut words = cs_words(eqtb, ACTIVE_BASE, UNDEFINED_CONTROL_SEQUENCE);
    let (entries, del_codes) = eqtb.data.split_at_mut(crate::state::data_slot(INT_BASE));
    let del_codes = &mut del_codes
        [crate::state::data_slot(DEL_CODE_BASE) - crate::state::data_slot(INT_BASE)..]
        [..NUMBER_USVS as usize];
    if S::LOADING {
        undump_runs(s, &mut words)?;
        undump_runs(s, entries)?;
        undump_runs(s, del_codes)?;
    } else {
        dump_runs(s, &mut words, same_entry)?;
        dump_runs(s, entries, same_entry)?;
        dump_runs(s, del_codes, same_int)?;
    }
    runs(s, eqtb.ints.values_mut())?;
    runs(s, eqtb.dimens.values_mut())?;
    let mut extra = cs_words(eqtb, UNDEFINED_CONTROL_SEQUENCE + 1, extra_top);
    s.items(&mut extra)?;
    if S::LOADING {
        set_cs_words(eqtb, ACTIVE_BASE, &words)?;
        set_cs_words(eqtb, UNDEFINED_CONTROL_SEQUENCE + 1, &extra)?;
    }

    // \partokenname may name an active character or a control symbol.
    s.items_in(
        std::slice::from_mut(&mut st.input.par_loc),
        ACTIVE_BASE,
        UNDEFINED_CONTROL_SEQUENCE + eqtb.hash_high,
    )?;
    if S::LOADING {
        st.input.par_token = Token::from_cs(st.input.par_loc);
    }
    s.items_in(
        std::slice::from_mut(&mut st.shipout.write_loc),
        HASH_BASE,
        eqtb.cs_top,
    )?;

    s.items(&mut eqtb.prim[..=PRIM_SIZE as usize])?;

    s.items_in(
        std::slice::from_mut(&mut eqtb.hash_used),
        HASH_BASE,
        FROZEN_CONTROL_SEQUENCE,
    )?;
    let hash_used = eqtb.hash_used;
    if S::LOADING {
        let mut p = HASH_BASE - 1;
        loop {
            let mut next = CsId::NONE;
            s.items_in(std::slice::from_mut(&mut next), p + 1, hash_used)?;
            p = next;
            s.items(std::slice::from_mut(&mut eqtb.hash[hash_slot(p)]))?;
            if p == hash_used {
                break;
            }
        }
    } else {
        eqtb.cs_count = (FROZEN_CONTROL_SEQUENCE - 1) - hash_used + eqtb.hash_high;
        for mut p in HASH_BASE.through(hash_used) {
            let entry = &mut eqtb.hash[hash_slot(p)];
            if entry.s1 != 0 {
                s.items(std::slice::from_mut(&mut p))?;
                s.items(std::slice::from_mut(entry))?;
                eqtb.cs_count += 1;
            }
        }
    }

    s.items(&mut eqtb.hash[hash_slot(hash_used + 1)..hash_slot(UNDEFINED_CONTROL_SEQUENCE)])?;
    let extra = hash_slot(UNDEFINED_CONTROL_SEQUENCE + 1);
    let extra_end = hash_slot(UNDEFINED_CONTROL_SEQUENCE + eqtb.hash_high + 1);
    s.items(&mut eqtb.hash[extra..extra_end])?;
    s.int(&mut eqtb.cs_count)?;

    if !S::LOADING {
        print_ln(st);
        print_int(st, st.eqtb.cs_count);
        print(st, " multiletter control sequences");
    }
    Ok(())
}

fn font_info<S: FmtStream>(s: &mut S, st: &mut EngineState) -> Result<(), Abort> {
    let max_str = st.strings.str_ptr;
    let fonts = &mut st.fonts;
    s.size(
        &mut fonts.fmem_ptr,
        FontInfoIdx::from_raw(7),
        FontInfoIdx::from_raw(SUP_FONT_MEM_SIZE),
        "must increase font_mem_size",
    )?;
    if S::LOADING {
        fonts.font_mem_size = fonts.font_mem_size.max(fonts.fmem_ptr);
        fonts.font_info = crate::mem_types::zeroed_vec(fonts.font_mem_size.index() + 1);
    }
    s.items(span(&mut fonts.font_info, 0, fonts.fmem_ptr.raw()))?;

    let mut font_ptr = fonts.font_ptr.raw();
    s.size(
        &mut font_ptr,
        FONT_BASE,
        FONT_BASE + MAX_FONT_MAX,
        "must increase font_max",
    )?;
    fonts.font_ptr = FontId::try_from(font_ptr).map_err(|_| bad_fmt())?;
    if S::LOADING {
        fonts.alloc_font_arrays();
    }

    let last = fonts.font_ptr;
    let max_font_index = fonts.fmem_ptr - 1;
    s.items(fonts_of(&mut fonts.font_check, last))?;
    s.items(fonts_of(&mut fonts.font_size, last))?;
    s.items(fonts_of(&mut fonts.font_dsize, last))?;
    s.items_in(
        fonts_of(&mut fonts.font_params, last),
        MIN_HALFWORD,
        MAX_HALFWORD,
    )?;
    s.items(fonts_of(&mut fonts.hyphen_char, last))?;
    s.items(fonts_of(&mut fonts.skew_char, last))?;
    s.items_in(
        fonts_of(&mut fonts.font_name, last),
        StrNumber::from_raw(i32::MIN),
        max_str,
    )?;
    s.items_in(
        fonts_of(&mut fonts.font_area, last),
        StrNumber::from_raw(i32::MIN),
        max_str,
    )?;
    s.items(fonts_of(&mut fonts.font_bc, last))?;
    s.items(fonts_of(&mut fonts.font_ec, last))?;
    s.items(fonts_of(&mut fonts.char_base, last))?;
    s.items(fonts_of(&mut fonts.width_base, last))?;
    s.items(fonts_of(&mut fonts.height_base, last))?;
    s.items(fonts_of(&mut fonts.depth_base, last))?;
    s.items(fonts_of(&mut fonts.italic_base, last))?;
    s.items(fonts_of(&mut fonts.lig_kern_base, last))?;
    s.items(fonts_of(&mut fonts.kern_base, last))?;
    s.items(fonts_of(&mut fonts.exten_base, last))?;
    s.items(fonts_of(&mut fonts.param_base, last))?;
    let glue = fonts_of(&mut fonts.font_glue, last);
    let mut links: Vec<i32> = glue
        .iter()
        .map(|g| g.map_or(TEX_NULL, GlueSpecId::raw))
        .collect();
    s.items(&mut links)?;
    if S::LOADING
        && links
            .iter()
            .any(|&p| p != TEX_NULL && !st.mem.glue.contains(p))
    {
        return Err(bad_fmt());
    }
    for (g, &p) in glue.iter_mut().zip(&links) {
        *g = GlueSpecId::from_link(p);
    }
    s.items_in(
        fonts_of(&mut fonts.bchar_label, last),
        FontInfoIdx::ZERO,
        max_font_index,
    )?;
    s.items_in(
        fonts_of(&mut fonts.font_bchar, last),
        LigChar::of(0),
        LigChar::NONE,
    )?;
    s.items_in(
        fonts_of(&mut fonts.font_false_bchar, last),
        LigChar::of(0),
        LigChar::NONE,
    )?;

    if S::LOADING {
        check_font_words(&st.eqtb, st.fonts.font_ptr)?;
    } else {
        print_dumped_fonts(st)?;
    }
    Ok(())
}

/// Rejects a format whose eqtb names a font it does not load.
fn check_font_words(eqtb: &EqtbState, font_ptr: FontId) -> Result<(), Abort> {
    let loaded = |p| FontId::try_from(eqtb.equiv(p)).is_ok_and(|f| f <= font_ptr);
    let meanings_ok = eqtb.meanings.iter().all(|m| match m {
        Command::SetFont(f) => *f <= font_ptr,
        _ => true,
    });
    if meanings_ok && CUR_FONT_LOC.until(CAT_CODE_BASE).all(loaded) {
        Ok(())
    } else {
        Err(bad_fmt())
    }
}

fn print_dumped_fonts(st: &mut EngineState) -> Result<(), Abort> {
    let font_ptr = st.fonts.font_ptr;
    for f in std::iter::once(NULL_FONT).chain(font_ptr.loaded()) {
        print_nl(st, "\\font");
        crate::output::print_esc(st, st.eqtb.font_id_text(f));
        print_char(st, b'=');

        let font_name = st.fonts.font_name[f];
        let font_area = st.fonts.font_area[f];
        if font_area == AAT_FONT_FLAG
            || font_area == OTGR_FONT_FLAG
            || st.fonts.font_mapping[f].is_some()
        {
            crate::output::print_file_name(st, font_name, EMPTY_STRING, EMPTY_STRING);
            crate::errors::error(
                st,
                "Can't \\dump a format with native fonts or font-mappings",
                &[
                    "You really, really don't want to do this.",
                    "It won't work, and only confuses me.",
                    "(Load them at runtime, not as part of the format file.)",
                ],
            )?;
        } else {
            crate::output::print_file_name(st, font_name, font_area, EMPTY_STRING);
        }

        let font_size = st.fonts.font_size[f];
        if font_size != st.fonts.font_dsize[f] {
            print(st, " at ");
            crate::output::print_scaled(st, font_size);
            print(st, "pt");
        }
    }

    print_ln(st);
    print_int(st, st.fonts.fmem_ptr.raw() - 7);
    print(st, " words of font info for ");
    print_int(st, font_ptr.raw());
    if font_ptr.raw() != FONT_BASE + 1 {
        print(st, " preloaded fonts");
    } else {
        print(st, " preloaded font");
    }
    Ok(())
}

fn hyphenation_tables<S: FmtStream>(s: &mut S, st: &mut EngineState) -> Result<(), Abort> {
    let hyph = &mut st.hyph;
    s.size(
        &mut hyph.hyph_count,
        0,
        hyph.hyph_size,
        "must increase hyph_size",
    )?;
    if !S::LOADING && hyph.hyph_next <= HYPH_PRIME {
        hyph.hyph_next = hyph.hyph_size;
    }
    s.size(
        &mut hyph.hyph_next,
        HYPH_PRIME,
        hyph.hyph_size,
        "must increase hyph_size",
    )?;

    if S::LOADING {
        let mut j = 0;
        for _ in 1..=hyph.hyph_count {
            s.int(&mut j)?;
            if j < 0 {
                return Err(bad_fmt());
            }
            if j > 65_535 {
                hyph.hyph_next = j / 65_536;
                j -= hyph.hyph_next * 65_536;
            } else {
                hyph.hyph_next = 0;
            }

            if j >= hyph.hyph_size || hyph.hyph_next > hyph.hyph_size {
                return Err(bad_fmt());
            }
            let h = j as usize;
            hyph.hyph_link[h] = hyph.hyph_next as u16;
            s.items_in(
                std::slice::from_mut(&mut hyph.hyph_word[h]),
                StrNumber::from_raw(0),
                st.strings.str_ptr,
            )?;
            s.ptr_in(&mut hyph.hyph_list[h], TEX_NULL, MAX_HALFWORD)?;
        }

        hyph.hyph_next = (j + 1).max(HYPH_PRIME);
        if hyph.hyph_next >= hyph.hyph_size {
            hyph.hyph_next = HYPH_PRIME;
        } else if hyph.hyph_next >= HYPH_PRIME {
            hyph.hyph_next += 1;
        }
    } else {
        for k in 0..=hyph.hyph_size {
            let h = k as usize;
            if hyph.hyph_word[h] != StrNumber::NONE {
                s.int(&mut (k + 65_536 * i32::from(hyph.hyph_link[h])))?;
                s.items(std::slice::from_mut(&mut hyph.hyph_word[h]))?;
                s.items(std::slice::from_mut(&mut hyph.hyph_list[h]))?;
            }
        }

        let hyph_count = hyph.hyph_count;
        print_ln(st);
        print_int(st, hyph_count);
        if hyph_count != 1 {
            print(st, " hyphenation exceptions");
        } else {
            print(st, " hyphenation exception");
        }
        if st.hyph.trie_not_ready {
            st.hyph.init_trie()?;
        }
    }

    let hyph = &mut st.hyph;
    s.size(
        &mut hyph.trie_max,
        TriePtr::NULL,
        TriePtr::from_raw(hyph.trie_size),
        "must increase trie_size",
    )?;
    let trie_max = hyph.trie_max;
    s.items_in(
        std::slice::from_mut(&mut hyph.hyph_start),
        TriePtr::NULL,
        trie_max,
    )?;
    if S::LOADING {
        let trie_len = trie_max.index() + 1;
        if hyph.trie_trl.is_empty() {
            hyph.trie_trl = vec![0; trie_len];
        }
        if hyph.trie_tro.is_empty() {
            hyph.trie_tro = vec![0; trie_len];
        }
        if hyph.trie_trc.is_empty() {
            hyph.trie_trc = vec![0; trie_len];
        }
    }
    s.items(span(&mut hyph.trie_trl, 0, trie_max.raw() + 1))?;
    s.items(span(&mut hyph.trie_tro, 0, trie_max.raw() + 1))?;
    s.items(span(&mut hyph.trie_trc, 0, trie_max.raw() + 1))?;
    s.int(&mut hyph.max_hyph_char)?;

    s.size(
        &mut hyph.trie_op_ptr,
        0,
        TRIE_OP_SIZE,
        "must increase TRIE_OP_SIZE",
    )?;
    let op_count = hyph.trie_op_ptr;
    s.items(span(&mut hyph.hyf_distance, 1, op_count))?;
    s.items(span(&mut hyph.hyf_num, 1, op_count))?;
    s.items_in(span(&mut hyph.hyf_next, 1, op_count), 0, TrieOp::MAX.raw())?;

    if S::LOADING {
        hyph.trie_used[..=BIGGEST_LANG as usize].fill(0);
        let mut j = op_count;
        let mut k = BIGGEST_LANG + 1;
        while j > 0 {
            let below = k - 1;
            s.int_in(&mut k, 0, below)?;
            let mut used = 0;
            s.int_in(&mut used, 1, j)?;
            hyph.trie_used[k as usize] = used as u16;
            j -= used;
            hyph.op_start[k as usize] = j as usize;
        }
        hyph.trie_not_ready = false;
    } else {
        print_nl(st, "Hyphenation trie of length ");
        print_int(st, trie_max.raw());
        print(st, " has ");
        print_int(st, op_count);
        if op_count != 1 {
            print(st, " ops");
        } else {
            print(st, " op");
        }
        print(st, " out of ");
        print_int(st, TRIE_OP_SIZE);

        for k in (0..=BIGGEST_LANG).rev() {
            let mut used = i32::from(st.hyph.trie_used[k as usize]);
            if used > 0 {
                print_nl(st, "  ");
                print_int(st, used);
                print(st, " for language ");
                print_int(st, k);
                s.int(&mut { k })?;
                s.int(&mut used)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::Cmd;
    use crate::consts::{FONT_ID_BASE, LEVEL_ONE};
    use crate::state::Selector;

    fn eqtb_with_fonts() -> EqtbState {
        let mut eqtb = EqtbState::default();
        eqtb.alloc_tables();
        eqtb
    }

    #[test]
    fn font_words_read_back_through_their_low_half() {
        let mut eqtb = eqtb_with_fonts();
        eqtb.set_equiv(CUR_FONT_LOC, FontId::new(300).raw());
        eqtb.set_equiv(CUR_FONT_LOC + 1, FontId::new(7).raw());
        assert_eq!(eqtb.cur_font(), FontId::new(300));
        assert_eq!(eqtb.math_font(0), FontId::new(7));
    }

    #[test]
    fn format_load_rejects_a_font_word_past_font_ptr() {
        let font_ptr = FontId::new(3);
        let mut eqtb = eqtb_with_fonts();
        let id = FONT_ID_BASE + 2;
        eqtb.set_meaning(id, Command::SetFont(FontId::new(3)), LEVEL_ONE);
        eqtb.set_equiv(CUR_FONT_LOC + 5, 2);
        assert!(check_font_words(&eqtb, font_ptr).is_ok());

        eqtb.set_meaning(id, Command::SetFont(FontId::new(4)), LEVEL_ONE);
        assert!(check_font_words(&eqtb, font_ptr).is_err());
        let wide = EqtbState::entry(Cmd::SetFont, 0x1_0002, LEVEL_ONE);
        assert_eq!(EqtbState::from_cs_entry(wide), None);
        eqtb.set_meaning(id, Command::SetFont(FontId::new(3)), LEVEL_ONE);
        eqtb.set_equiv(CUR_FONT_LOC + 5, 4);
        assert!(check_font_words(&eqtb, font_ptr).is_err());
    }

    #[derive(Default)]
    struct VecDump(Vec<u8>);

    impl FmtStream for VecDump {
        const LOADING: bool = false;
        fn items<T: FmtItem>(&mut self, v: &mut [T]) -> Result<(), Abort> {
            for &x in v.iter() {
                x.put(&mut self.0);
            }
            Ok(())
        }
    }

    struct SliceUndump<'a>(&'a [u8]);

    impl FmtStream for SliceUndump<'_> {
        const LOADING: bool = true;
        fn items<T: FmtItem>(&mut self, v: &mut [T]) -> Result<(), Abort> {
            for x in v.iter_mut() {
                let (head, rest) = self.0.split_at(T::SIZE);
                *x = T::get(head);
                self.0 = rest;
            }
            Ok(())
        }
    }

    fn font_state() -> EngineState {
        let mut st = EngineState::default();
        st.shipout.selector = Selector::NoPrint;
        st.eqtb.alloc_tables();
        st.strings.str_start = vec![crate::units::PoolPos::ZERO; 4];
        st.fonts.font_max = FontId::new(1);
        st.fonts.fmem_ptr = FontInfoIdx::from_raw(7);
        st.fonts.font_info = crate::mem_types::zeroed_vec(8);
        st.fonts.alloc_font_arrays();
        st
    }

    #[test]
    fn font_info_load_restores_font_ptr() {
        let mut src = font_state();
        src.fonts.font_ptr = NULL_FONT;
        src.fonts.font_size[NULL_FONT] = Scaled::from_raw(12);
        let mut dump = VecDump::default();
        font_info(&mut dump, &mut src).expect("dump");

        let mut dst = font_state();
        dst.fonts.font_ptr = FontId::new(1);
        font_info(&mut SliceUndump(&dump.0), &mut dst).expect("load");
        assert_eq!(dst.fonts.font_ptr, NULL_FONT);
        assert_eq!(dst.fonts.font_size[NULL_FONT], Scaled::from_raw(12));
    }
}
