//! Format-file dump and load (`\dump` and the preloaded format).
//!
//! Each section of the file is described once, generic over a `FmtStream`:
//! dumping writes every item from the engine state and loading reads it back
//! into the same place, checking ranges. Items are stored big-endian, an
//! 8-byte word as one 64-bit value.

use crate::bridge::{
    input_open, input_read, output_close, output_flush, output_open, output_write, FileFormat,
};
use crate::cmd::Cmd;
use crate::consts::{
    ACTIVE_BASE, CAT_CODE_BASE, CUR_FONT_LOC, EMPTY_STRING, EQTB_SIZE, FONT_BASE,
    FROZEN_CONTROL_SEQUENCE, HASH_BASE, HISTORY_FATAL_ERROR, INT_BASE, INVALID_HANDLE,
    MAX_FONT_MAX, MAX_HALFWORD, MEM_TOP, MIN_HALFWORD, NULL_FONT, PRIM_SIZE, TEX_NULL,
    TOO_BIG_CHAR, UNDEFINED_CONTROL_SEQUENCE,
};
use crate::engine::{POOL_FREE, STRINGS_FREE};
use crate::errors::Abort;
use crate::fonts::LigChar;
use crate::fonts::{AAT_FONT_FLAG, OTGR_FONT_FLAG};
use crate::hash::{hash_slot, HASH_PRIME};
use crate::hyphenation::{TrieOp, TriePtr, BIGGEST_LANG, HYPH_PRIME, TRIE_OP_SIZE};
use crate::mem::{CONTRIB_HEAD, HI_MEM_STAT_MIN, HI_MEM_STAT_USAGE, PAGE_HEAD};
use crate::mem_types::{B16x4, B32x2, MemoryWord};
use crate::node::{link_of, FreeBlock, NodePtr};
use crate::output::{print, print_char, print_int, print_ln, print_nl};
use crate::params::IntPar;
use crate::state::{EngineState, EqtbState, InteractionMode, PerFont, Selector};
use crate::token::Token;
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

int_item!(u16, i16, i32);

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

newtype_item!(
    Scaled,
    MemPtr,
    StrNumber,
    EqtbPtr,
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
            MemPtr::from_raw(low),
            MemPtr::from_raw(high),
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

/// `v[start..start + n]`, empty when `n <= 0`.
fn eqtb_span(v: &mut [MemoryWord], start: EqtbPtr, n: i32) -> &mut [MemoryWord] {
    span(v, start.raw(), n)
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
        crate::output::print_err(st, "You can't dump inside a group");
        st.errors.help(&[r"`{...\dump}' is a no-no."]);

        if st.errors.interaction == InteractionMode::ErrorStop {
            st.errors.interaction = InteractionMode::Scroll;
        }
        if st.errors.log_opened {
            crate::errors::finish_error(st)?;
        }

        st.errors.history = HISTORY_FATAL_ERROR;
        let _ = crate::io::close_files_and_terminate(st);
        output_flush(st.shipout.rust_stdout);
        return Err(Abort::Internal("\\dump inside a group"));
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
        return Err(Abort::PoolOverflow);
    }

    st.strings.format_ident = st.strings.make_string()?;
    crate::io::pack_job_name(st, ".fmt")?;

    let handle = output_open(st.input.name_of_file_bytes(), false);
    if handle == INVALID_HANDLE {
        return Err(Abort::Internal("cannot open format output file"));
    }

    print_nl(st, "Beginning to dump on file ");
    let name_string = crate::io::make_name_string(st)?;
    print(st, name_string);

    st.strings.flush_string();

    print_nl(st, "");
    print(st, st.strings.format_ident);

    while st.input.pseudo_files.raw() != TEX_NULL {
        st.mem.pseudo_close(&mut st.input);
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

    st.input.name_of_file = dump_name.as_bytes().to_vec();

    let handle = input_open(st.input.name_of_file_bytes(), FileFormat::Format, false);
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
        eqtb.eqtb_top = EQTB_SIZE + eqtb.hash_extra;
        eqtb.hash_top = if eqtb.hash_extra == 0 {
            UNDEFINED_CONTROL_SEQUENCE
        } else {
            eqtb.eqtb_top
        };
        eqtb.hash = crate::mem_types::zeroed_vec((1 + hash_slot(eqtb.hash_top)) as usize);

        eqtb.data = crate::mem_types::zeroed_vec(eqtb.eqtb_top.index() + 1);
        eqtb.set_eq(UNDEFINED_CONTROL_SEQUENCE, Cmd::UndefinedCs, TEX_NULL, 0);
        let undefined_entry = eqtb.word(UNDEFINED_CONTROL_SEQUENCE);
        eqtb.data[(EQTB_SIZE + 1).index()..].fill(undefined_entry);

        st.scanner.max_reg_num = 32_767;
    }

    s.constant(MEM_TOP)?;
    if S::LOADING {
        st.input.cur_list.head = CONTRIB_HEAD;
        st.input.cur_list.tail = CONTRIB_HEAD;
        st.shipout.page_tail = PAGE_HEAD;
        st.mem.data = crate::mem_types::zeroed_vec(crate::mem::MEM_WORDS);
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
    if !S::LOADING {
        st.mem.sort_avail();
    }
    let mem = &mut st.mem;
    s.ptr_in(&mut mem.lo_mem_max, 1019, MEM_TOP - HI_MEM_STAT_USAGE)?;
    s.ptr_in(&mut mem.rover, 20, mem.lo_mem_max.raw())?;
    let lo_mem_max = mem.lo_mem_max;

    s.items_in(
        st.scanner.sa_root.registers_mut(),
        MemPtr::from_raw(MIN_HALFWORD),
        lo_mem_max,
    )?;

    let mut words = 0;
    let mut var_used = 0;
    let mut p = MemPtr::MEM_BOT;
    let mut q = mem.rover;
    loop {
        s.items(span(&mut mem.data, p.raw(), q + 2 - p))?;
        words += q + 2 - p;
        var_used += q - p;
        let block = NodePtr::<FreeBlock>::at(q);
        p = q + mem[block].node_size;
        let next = mem[block].rlink;
        if S::LOADING && (p > lo_mem_max || (q >= next && next != mem.rover)) {
            return Err(bad_fmt());
        }
        q = next;
        if q == mem.rover {
            break;
        }
    }
    s.items(span(&mut mem.data, p.raw(), lo_mem_max + 1 - p))?;
    words += lo_mem_max + 1 - p;
    var_used += lo_mem_max - p;

    s.ptr_in(&mut mem.hi_mem_min, lo_mem_max.raw() + 1, HI_MEM_STAT_MIN)?;
    s.ptr_in(&mut mem.avail, MIN_HALFWORD, MEM_TOP)?;
    if S::LOADING {
        mem.mem_end = MemPtr::from_raw(MEM_TOP);
    }
    let (hi_mem_min, mem_end) = (mem.hi_mem_min, mem.mem_end);
    s.items(span(
        &mut mem.data,
        hi_mem_min.raw(),
        mem_end + 1 - hi_mem_min,
    ))?;
    words += mem_end + 1 - hi_mem_min;

    if !S::LOADING {
        let mut dyn_used = mem_end + 1 - hi_mem_min;
        let mut p = mem.avail;
        while !p.is_null() {
            dyn_used -= 1;
            p = p.link(mem);
        }
        mem.var_used = var_used;
        mem.dyn_used = dyn_used;
    }
    s.int(&mut mem.var_used)?;
    s.int(&mut mem.dyn_used)?;

    if !S::LOADING {
        print_ln(st);
        print_int(st, words);
        print(st, " memory locations dumped; current usage is ");
        print_int(st, st.mem.var_used);
        print_char(st, b'&');
        print_int(st, st.mem.dyn_used);
    }
    Ok(())
}

/// Dumps `eqtb[k..=last]` as alternating runs of distinct words and of
/// repeats of the preceding word, returning the next index.
fn dump_eqtb_runs<S: FmtStream>(
    s: &mut S,
    eqtb: &mut EqtbState,
    mut k: EqtbPtr,
    last: EqtbPtr,
    same: impl Fn(&EqtbState, EqtbPtr, EqtbPtr) -> bool,
) -> Result<EqtbPtr, Abort> {
    loop {
        let mut j = k;
        while j < last && !same(eqtb, j, j + 1) {
            j += 1;
        }
        let l = if j < last {
            j += 1;
            let l = j;
            while j < last && same(eqtb, j, j + 1) {
                j += 1;
            }
            l
        } else {
            last + 1
        };

        s.int(&mut (l - k))?;
        s.items(eqtb_span(&mut eqtb.data, k, l - k))?;
        k = j + 1;
        s.int(&mut (k - l))?;
        if k > last {
            return Ok(k);
        }
    }
}

fn undump_eqtb_runs<S: FmtStream>(s: &mut S, eqtb: &mut EqtbState) -> Result<(), Abort> {
    let mut k = ACTIVE_BASE;
    loop {
        let mut x = 0;
        s.int(&mut x)?;
        if x < 1 || k + x > EQTB_SIZE + 1 {
            return Err(bad_fmt());
        }
        s.items(eqtb_span(&mut eqtb.data, k, x))?;
        k += x;

        s.int(&mut x)?;
        if x < 0 || k + x > EQTB_SIZE + 1 {
            return Err(bad_fmt());
        }
        let repeated = eqtb.word(k - 1);
        eqtb.fill(k, k + x, repeated);
        k += x;
        if k > EQTB_SIZE {
            return Ok(());
        }
    }
}

fn table_of_equivalents<S: FmtStream>(s: &mut S, st: &mut EngineState) -> Result<(), Abort> {
    let eqtb = &mut st.eqtb;
    if S::LOADING {
        undump_eqtb_runs(s, eqtb)?;
    } else {
        let k = dump_eqtb_runs(s, eqtb, ACTIVE_BASE, INT_BASE - 1, |eqtb, a, b| {
            eqtb.equiv(a) == eqtb.equiv(b)
                && eqtb.eq_type(a) == eqtb.eq_type(b)
                && eqtb.eq_level(a) == eqtb.eq_level(b)
        })?;
        dump_eqtb_runs(s, eqtb, k, EQTB_SIZE, |eqtb, a, b| {
            eqtb.int_at(a) == eqtb.int_at(b)
        })?;
    }
    s.items(eqtb_span(&mut eqtb.data, EQTB_SIZE + 1, eqtb.hash_high))?;
    if S::LOADING {
        let mut entries = ACTIVE_BASE
            .until(INT_BASE)
            .chain((EQTB_SIZE + 1).through(EQTB_SIZE + eqtb.hash_high));
        if entries.any(|k| Cmd::from_raw(i32::from(eqtb.word(k).b16().s1)).is_none()) {
            return Err(bad_fmt());
        }
    }

    // \partokenname may name an active character or a control symbol.
    s.items_in(
        std::slice::from_mut(&mut st.input.par_loc),
        ACTIVE_BASE,
        EQTB_SIZE + eqtb.hash_high,
    )?;
    if S::LOADING {
        st.input.par_token = Token::from_cs(st.input.par_loc);
    }
    s.items_in(
        std::slice::from_mut(&mut st.shipout.write_loc),
        HASH_BASE,
        eqtb.hash_top,
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
            let mut next = EqtbPtr::NONE;
            s.items_in(std::slice::from_mut(&mut next), p + 1, hash_used)?;
            p = next;
            s.items(span(&mut eqtb.hash, hash_slot(p), 1))?;
            if p == hash_used {
                break;
            }
        }
    } else {
        eqtb.cs_count = (FROZEN_CONTROL_SEQUENCE - 1) - hash_used + eqtb.hash_high;
        for mut p in HASH_BASE.through(hash_used) {
            let entry = &mut eqtb.hash[hash_slot(p) as usize];
            if entry.s1 != 0 {
                s.items(std::slice::from_mut(&mut p))?;
                s.items(std::slice::from_mut(entry))?;
                eqtb.cs_count += 1;
            }
        }
    }

    s.items(span(
        &mut eqtb.hash,
        hash_slot(hash_used + 1),
        (UNDEFINED_CONTROL_SEQUENCE - 1) - hash_used,
    ))?;
    s.items(span(
        &mut eqtb.hash,
        hash_slot(EQTB_SIZE + 1),
        eqtb.hash_high,
    ))?;
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
    let lo_mem_max = st.mem.lo_mem_max;
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
    let mut links: Vec<MemPtr> = glue.iter().map(|&g| link_of(g)).collect();
    s.items_in(&mut links, MemPtr::from_raw(MIN_HALFWORD), lo_mem_max)?;
    for (g, &p) in glue.iter_mut().zip(&links) {
        *g = NodePtr::from_spec_link(p);
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
    let set_font = ACTIVE_BASE
        .until(INT_BASE)
        .chain(EQTB_SIZE.plus(1).through(eqtb.eqtb_top))
        .filter(|&p| eqtb.eq_type(p) == Cmd::SetFont);
    if CUR_FONT_LOC
        .until(CAT_CODE_BASE)
        .chain(set_font)
        .all(loaded)
    {
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
            crate::output::print_err(
                st,
                "Can't \\dump a format with native fonts or font-mappings",
            );
            st.errors.help(&[
                "You really, really don't want to do this.",
                "It won't work, and only confuses me.",
                "(Load them at runtime, not as part of the format file.)",
            ]);
            crate::errors::finish_error(st)?;
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
            s.ptr_in(&mut hyph.hyph_list[h], MIN_HALFWORD, MAX_HALFWORD)?;
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
    use crate::consts::FONT_ID_BASE;
    use crate::mem_types::B32x2;
    use crate::state::Selector;

    fn eqtb_with_fonts() -> EqtbState {
        let mut eqtb = EqtbState {
            eqtb_top: EQTB_SIZE,
            ..Default::default()
        };
        eqtb.data = crate::mem_types::zeroed_vec(EQTB_SIZE.index() + 1);
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
        eqtb.set_eq_type(id, Cmd::SetFont);
        eqtb.set_equiv(id, 3);
        eqtb.set_equiv(CUR_FONT_LOC + 5, 2);
        assert!(check_font_words(&eqtb, font_ptr).is_ok());

        eqtb.set_equiv(id, 4);
        assert!(check_font_words(&eqtb, font_ptr).is_err());
        eqtb.set_equiv(id, 0x1_0002);
        assert!(check_font_words(&eqtb, font_ptr).is_err());
        eqtb.set_equiv(id, 3);
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
        st.eqtb.data = crate::mem_types::zeroed_vec(EQTB_SIZE.index() + 1);
        st.strings.str_start = vec![crate::units::PoolPos::ZERO; 4];
        st.eqtb.hash = vec![B32x2 { s0: 0, s1: 0 }; hash_slot(FONT_ID_BASE + 2) as usize];
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
