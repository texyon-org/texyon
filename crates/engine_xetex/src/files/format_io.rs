//! Format-file dump and load (`\dump` and the preloaded format).
//!
//! Each section of the file is described once, generic over a `FmtStream`:
//! dumping writes every item from the engine state and loading reads it back
//! into the same place, checking ranges. Items are stored big-endian, an
//! 8-byte word as one 64-bit value.

use std::rc::Rc;
use std::{iter, slice};

use crate::FORMAT_SERIAL;
use crate::base::cmd::Cmd;
use crate::base::command::Command;
use crate::base::consts::{
    EMPTY_STRING, FONT_BASE, LEVEL_ONE, MAX_FONT_MAX, MAX_HALFWORD, NULL_FONT, PRIM_SIZE, TEX_NULL,
    TOO_BIG_CHAR,
};
use crate::base::params::IntPar;
use crate::base::token::{ListSlot, Token, TokenList, TokenStore};
use crate::base::units::{
    FontId, MemIdx, MemPtr, PoolPos, PrimNum, RawPtr, SavePtr, Scaled, StrNumber,
};
use crate::build::hyphenation::{BIGGEST_LANG, HYPH_PRIME, TRIE_OP_SIZE, TrieOp, TriePtr};
use crate::engine::{POOL_FREE, STRINGS_FREE};
use crate::env::EngineEnv;
use crate::ffi::bridge::{FileFormat, input_open};
use crate::files::io::{make_name_string, pack_job_name};
use crate::fonts::tfm::{
    CharInfo, CharTag, ExtIdx, ExtRecipe, FontMetrics, FontParams, LigKernAction, LigKernIdx,
    LigKernStep, LigOp, TfmMetrics,
};
use crate::fonts::{AAT_FONT_FLAG, LigChar, OTGR_FONT_FLAG, PerFont};
use crate::memory::glue_store::{GlueSpec, GlueSpecId, GlueStore, STATIC_SPECS};
use crate::memory::mem::{CONTRIB_HEAD, PAGE_HEAD, RESERVED_WORDS};
use crate::memory::mem_types::zeroed_vec;
use crate::memory::node::{NodePtr, View};
use crate::preview::journal::JournaledVec;
use crate::print::errors::{Abort, InteractionMode, error};
use crate::print::{
    Selector, print, print_char, print_esc, print_file_name, print_int, print_ln, print_nl,
    print_scaled,
};
use crate::state::EngineState;
use crate::tables::cs::{CsId, NameSlot};
use crate::tables::eqtb::{EqStore, EqtbState, InterCharPair, ShapeRef};
use crate::tables::hash::{HASH_PRIME, HashEntry, PrimEntry};

const FORMAT_HEADER_MAGIC: i32 = 0x5454_4E43;
const FORMAT_FOOTER_MAGIC: i32 = 0x0000_029A;
const SUP_MAX_STRINGS: i32 = 2_097_151;
const SUP_FONT_MEM_SIZE: i32 = 147_483_647;
const SUP_POOL_SIZE: i32 = 40_000_000;
const SUP_HASH_EXTRA: i32 = 2_097_151;

fn bad_fmt() -> Abort {
    Abort::Internal("fatal format file error")
}

trait FmtItem: Copy {
    const SIZE: usize;
    fn put(self, out: &mut Vec<u8>);
    fn get(bytes: &[u8]) -> Option<Self>;
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
            fn get(bytes: &[u8]) -> Option<Self> {
                bytes.try_into().ok().map(<$t>::from_be_bytes)
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
            fn get(bytes: &[u8]) -> Option<Self> {
                i32::get(bytes).map($t::from_raw)
            }
        }
    )*};
}

impl FmtItem for Cmd {
    const SIZE: usize = 1;
    #[inline]
    fn put(self, out: &mut Vec<u8>) {
        out.push(self.raw().to_le_bytes()[0]);
    }
    #[inline]
    fn get(bytes: &[u8]) -> Option<Self> {
        let &[code] = bytes else { return None };
        Some(Cmd::from_catcode_bits(i32::from(code)))
    }
}

impl FmtItem for Command {
    const SIZE: usize = 6;
    #[inline]
    fn put(self, out: &mut Vec<u8>) {
        let (cmd, chr) = self.encode();
        u16::from(cmd).put(out);
        chr.put(out);
    }
    #[inline]
    fn get(bytes: &[u8]) -> Option<Self> {
        let (cmd, chr) = bytes.split_at_checked(2)?;
        let cmd = Cmd::from_raw(i32::from(u16::get(cmd)?))?;
        Command::decode(cmd, i32::get(chr)?)
    }
}

impl FmtItem for FontId {
    const SIZE: usize = 2;
    #[inline]
    fn put(self, out: &mut Vec<u8>) {
        self.get().put(out);
    }
    #[inline]
    fn get(bytes: &[u8]) -> Option<Self> {
        u16::get(bytes).map(FontId::new)
    }
}

impl FmtItem for ShapeRef {
    const SIZE: usize = MemPtr::SIZE;
    #[inline]
    fn put(self, out: &mut Vec<u8>) {
        self.ptr().put(out);
    }
    #[inline]
    fn get(bytes: &[u8]) -> Option<Self> {
        MemPtr::get(bytes).map(ShapeRef::from_ptr)
    }
}

impl<K: View + 'static> FmtItem for Option<NodePtr<K>> {
    const SIZE: usize = MemPtr::SIZE;
    #[inline]
    fn put(self, out: &mut Vec<u8>) {
        self.map(NodePtr::idx).put(out);
    }
    #[inline]
    fn get(bytes: &[u8]) -> Option<Self> {
        MemPtr::get(bytes).map(|p| p.map(NodePtr::from_idx_unchecked))
    }
}

impl FmtItem for MemPtr {
    const SIZE: usize = 4;
    #[inline]
    fn put(self, out: &mut Vec<u8>) {
        self.raw().put(out);
    }
    #[inline]
    fn get(bytes: &[u8]) -> Option<Self> {
        i32::get(bytes).map(MemIdx::from_raw)
    }
}

newtype_item!(
    GlueSpecId, TokenList, Scaled, StrNumber, PoolPos, TriePtr, LigChar
);

impl FmtItem for CsId {
    const SIZE: usize = 4;
    #[inline]
    fn put(self, out: &mut Vec<u8>) {
        self.raw().put(out);
    }
    #[inline]
    fn get(bytes: &[u8]) -> Option<Self> {
        CsId::from_raw(i32::get(bytes)?)
    }
}

/// A control sequence or none, stored as TeX's 0.
impl FmtItem for Option<CsId> {
    const SIZE: usize = 4;
    #[inline]
    fn put(self, out: &mut Vec<u8>) {
        self.map_or(0, CsId::raw).put(out);
    }
    #[inline]
    fn get(bytes: &[u8]) -> Option<Self> {
        let raw = i32::get(bytes)?;
        (raw >= 0).then(|| CsId::from_raw(raw))
    }
}

/// A hash slot: its name, then the next slot in its chain (0 for none).
impl FmtItem for HashEntry {
    const SIZE: usize = 8;
    #[inline]
    fn put(self, out: &mut Vec<u8>) {
        self.text.put(out);
        self.next.put(out);
    }
    #[inline]
    fn get(bytes: &[u8]) -> Option<Self> {
        let (text, next) = bytes.split_at_checked(4)?;
        Some(HashEntry {
            text: StrNumber::get(text)?,
            next: Option::<CsId>::get(next)?,
        })
    }
}

/// A primitive slot: the next slot in its chain (0 for none), then its name (-1 for none).
impl FmtItem for PrimEntry {
    const SIZE: usize = 8;
    #[inline]
    fn put(self, out: &mut Vec<u8>) {
        self.next.map_or(0, PrimNum::raw).put(out);
        self.text.map_or(-1, StrNumber::raw).put(out);
    }
    #[inline]
    fn get(bytes: &[u8]) -> Option<Self> {
        let (next, text) = bytes.split_at_checked(4)?;
        let next = i32::get(next)?;
        let text = i32::get(text)?;
        Some(PrimEntry {
            next: if next == 0 {
                None
            } else {
                Some(PrimNum::from_raw(next)?)
            },
            text: (text >= 0).then_some(StrNumber::from_raw(text)),
        })
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
    fn get(bytes: &[u8]) -> Option<Self> {
        let mut words = [0u32; 5];
        for (w, chunk) in words.iter_mut().zip(bytes.as_chunks::<4>().0) {
            *w = u32::get(chunk)?;
        }
        Some(bytemuck::cast(words))
    }
}

impl FmtItem for Token {
    const SIZE: usize = 4;
    #[inline]
    fn put(self, out: &mut Vec<u8>) {
        self.0.put(out);
    }
    #[inline]
    fn get(bytes: &[u8]) -> Option<Self> {
        i32::get(bytes).map(Token)
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
    fn get(bytes: &[u8]) -> Option<Self> {
        let mut words = [0u32; 3];
        for (w, chunk) in words.iter_mut().zip(bytes.as_chunks::<4>().0) {
            *w = u32::get(chunk)?;
        }
        Some(bytemuck::cast(words))
    }
}

fn put_words<const N: usize>(words: [i32; N], out: &mut Vec<u8>) {
    for w in words {
        w.put(out);
    }
}

fn get_words<const N: usize>(bytes: &[u8]) -> Option<[i32; N]> {
    let mut words = [0; N];
    for (w, chunk) in words.iter_mut().zip(bytes.as_chunks::<4>().0) {
        *w = i32::get(chunk)?;
    }
    Some(words)
}

/// A TFM character: tag kind (-1 for a missing character), remainder and dimensions.
impl FmtItem for Option<CharInfo> {
    const SIZE: usize = 24;
    fn put(self, out: &mut Vec<u8>) {
        let Some(ci) = self else {
            return put_words([-1, 0, 0, 0, 0, 0], out);
        };
        let (kind, rem) = match ci.tag {
            CharTag::None => (0, 0),
            CharTag::Lig(k) => (1, i32::from(k.raw())),
            CharTag::List(c) => (2, i32::from(c)),
            CharTag::Ext(e) => (3, i32::from(e.raw())),
        };
        put_words(
            [
                kind,
                rem,
                ci.width.raw(),
                ci.height.raw(),
                ci.depth.raw(),
                ci.italic.raw(),
            ],
            out,
        );
    }
    fn get(bytes: &[u8]) -> Option<Self> {
        let [kind, rem, width, height, depth, italic] = get_words(bytes)?;
        let tag = match kind {
            -1 => return Some(None),
            0 => CharTag::None,
            1 => CharTag::Lig(LigKernIdx::from_raw(u16::try_from(rem).ok()?)),
            2 => CharTag::List(u16::try_from(rem).ok()?),
            3 => CharTag::Ext(ExtIdx::from_raw(u8::try_from(rem).ok()?)),
            _ => return None,
        };
        Some(Some(CharInfo {
            width: Scaled::from_raw(width),
            height: Scaled::from_raw(height),
            depth: Scaled::from_raw(depth),
            italic: Scaled::from_raw(italic),
            tag,
        }))
    }
}

/// A lig/kern step: kind (jump, kern, ligature), then its fields, -1 for no skip.
impl FmtItem for LigKernStep {
    const SIZE: usize = 20;
    fn put(self, out: &mut Vec<u8>) {
        let words = match self {
            LigKernStep::Jump(k) => [0, i32::from(k.raw()), 0, 0, 0],
            LigKernStep::Rule {
                next_char,
                skip,
                action,
            } => {
                let skip = skip.map_or(-1, i32::from);
                match action {
                    LigKernAction::Kern(w) => [1, i32::from(next_char), skip, w.raw(), 0],
                    LigKernAction::Lig { op, char } => [
                        2,
                        i32::from(next_char),
                        skip,
                        i32::from(op.byte()),
                        i32::from(char),
                    ],
                }
            }
        };
        put_words(words, out);
    }
    fn get(bytes: &[u8]) -> Option<Self> {
        let [kind, a, b, c, d] = get_words(bytes)?;
        if kind == 0 {
            return Some(LigKernStep::Jump(LigKernIdx::from_raw(
                u16::try_from(a).ok()?,
            )));
        }
        let action = match kind {
            1 => LigKernAction::Kern(Scaled::from_raw(c)),
            2 => LigKernAction::Lig {
                op: LigOp::from_byte(u8::try_from(c).ok()?)?,
                char: u16::try_from(d).ok()?,
            },
            _ => return None,
        };
        Some(LigKernStep::Rule {
            next_char: u16::try_from(a).ok()?,
            skip: if b == -1 {
                None
            } else {
                Some(u8::try_from(b).ok()?)
            },
            action,
        })
    }
}

/// An extensible recipe, -1 for a missing piece.
impl FmtItem for ExtRecipe {
    const SIZE: usize = 16;
    fn put(self, out: &mut Vec<u8>) {
        let piece = |p: Option<u16>| p.map_or(-1, i32::from);
        put_words(
            [
                piece(self.top),
                piece(self.mid),
                piece(self.bot),
                i32::from(self.rep),
            ],
            out,
        );
    }
    fn get(bytes: &[u8]) -> Option<Self> {
        let [top, mid, bot, rep] = get_words(bytes)?;
        let piece = |p: i32| -> Option<Option<u16>> {
            if p == -1 {
                Some(None)
            } else {
                u16::try_from(p).ok().map(Some)
            }
        };
        Some(ExtRecipe {
            top: piece(top)?,
            mid: piece(mid)?,
            bot: piece(bot)?,
            rep: u16::try_from(rep).ok()?,
        })
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
    fn get(bytes: &[u8]) -> Option<Self> {
        let &[_, a, _, b, _, c, _, d] = bytes else {
            return None;
        };
        Some([a, b, c, d])
    }
}

trait FmtStream {
    const LOADING: bool;

    fn items<T: FmtItem>(&mut self, v: &mut [T]) -> Result<(), Abort>;

    fn int(&mut self, v: &mut i32) -> Result<(), Abort> {
        self.items(slice::from_mut(v))
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
        self.items_in(slice::from_mut(v), low, high)
    }

    fn ptr_in(&mut self, v: &mut MemPtr, low: i32, high: i32) -> Result<(), Abort> {
        self.items_in(
            slice::from_mut(v),
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
        self.items(slice::from_mut(v))?;
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

/// Collects the whole format, written out once it is complete.
#[derive(Default)]
struct Dump {
    bytes: Vec<u8>,
}

impl FmtStream for Dump {
    const LOADING: bool = false;

    fn items<T: FmtItem>(&mut self, v: &mut [T]) -> Result<(), Abort> {
        if v.is_empty() {
            return Ok(());
        }
        self.bytes.reserve(v.len() * T::SIZE);
        for &x in v.iter() {
            x.put(&mut self.bytes);
        }
        Ok(())
    }
}

/// Reads a format read into memory whole.
struct Undump {
    bytes: Vec<u8>,
    pos: usize,
}

impl FmtStream for Undump {
    const LOADING: bool = true;

    fn items<T: FmtItem>(&mut self, v: &mut [T]) -> Result<(), Abort> {
        let end = self.pos + v.len() * T::SIZE;
        let Some(bytes) = self.bytes.get(self.pos..end) else {
            return Err(Abort::Internal("could not read format file data"));
        };
        for (x, b) in v.iter_mut().zip(bytes.chunks(T::SIZE)) {
            *x = T::get(b).ok_or_else(bad_fmt)?;
        }
        self.pos = end;
        Ok(())
    }
}

fn span<T>(v: &mut [T], start: i32, n: i32) -> &mut [T] {
    if n <= 0 {
        return &mut [];
    }
    &mut v[start as usize..(start + n) as usize]
}

fn fonts_of<T>(v: &mut PerFont<T>, font_ptr: FontId) -> &mut [T] {
    &mut v.as_mut_slice()[FONT_BASE as usize..=font_ptr.index()]
}

pub(crate) fn store_fmt_file(st: &mut EngineState, env: &mut EngineEnv) -> Result<(), Abort> {
    if st.save_stack.save_ptr != SavePtr::ZERO {
        return Err(Abort::DumpInGroup);
    }

    st.shipout.selector = Selector::NewString;
    print(st, env, " (preloaded format=");
    print(st, env, st.strings.job_name);
    print_char(st, env, b' ');
    print_int(st, env, st.eqtb.int_par(IntPar::Year));
    print_char(st, env, b'.');
    print_int(st, env, st.eqtb.int_par(IntPar::Month));
    print_char(st, env, b'.');
    print_int(st, env, st.eqtb.int_par(IntPar::Day));
    print_char(st, env, b')');

    st.shipout.selector = if st.errors.interaction == InteractionMode::Batch {
        Selector::LogOnly
    } else {
        Selector::TermAndLog
    };

    if st.strings.pool_ptr + 1 > st.strings.pool_size {
        return Err(st.strings.pool_overflow());
    }

    st.strings.format_ident = st.strings.make_string()?;
    let path = pack_job_name(st, ".fmt")?;

    let Some(mut handle) = env.open(&path, false) else {
        return Err(Abort::Internal("cannot open format output file"));
    };

    print_nl(st, env, "Beginning to dump on file ");
    let name_string = make_name_string(st, &path)?;
    print(st, env, name_string);

    st.strings.flush_string();

    print_nl(st, env, "");
    print(st, env, st.strings.format_ident);

    while st.input.pseudo_files.is_some() {
        st.mem.pseudo_close(&mut st.input)?;
    }

    let mut dump = Dump::default();
    format_file(&mut dump, st, env)?;
    if env.host.write_output(&mut handle, &dump.bytes) {
        return Err(Abort::Internal("could not write format file data"));
    }
    st.eqtb.set_int_par(IntPar::TracingStats, 0);
    env.host.close_output(handle);
    Ok(())
}

pub fn load_fmt_file(
    st: &mut EngineState,
    env: &mut EngineEnv,
    dump_name: &str,
) -> Result<(), Abort> {
    let loc = st.input.cur_input.loc;

    let Some(opened) = input_open(env, dump_name.as_bytes(), FileFormat::Format, false) else {
        return Err(Abort::Internal("cannot open the format file"));
    };
    let mut file = opened.source;
    let bytes = file.read_to_end(env);
    if file.close(env) {
        return Err(Abort::Internal(
            "serious internal error closing the format file",
        ));
    }
    let Some(bytes) = bytes else {
        return Err(Abort::Internal("could not read format file data"));
    };

    st.input.cur_input.loc = loc;

    if st.config.in_initex_mode {
        st.strings.str_pool = Vec::new();
        st.strings.str_start = Vec::new();
        st.eqtb.release_tables();
        st.eqtb.hash = JournaledVec::default();
        st.mem.data = JournaledVec::default();
    }

    format_file(&mut Undump { bytes, pos: 0 }, st, env)?;
    Ok(())
}

fn format_file<S: FmtStream>(
    s: &mut S,
    st: &mut EngineState,
    env: &mut EngineEnv,
) -> Result<(), Abort> {
    header(s, st)?;
    string_pool(s, st, env)?;
    dynamic_memory(s, st, env)?;
    table_of_equivalents(s, st, env)?;
    font_info(s, st, env)?;
    hyphenation_tables(s, st, env)?;
    s.constant(FORMAT_FOOTER_MAGIC)
}

fn header<S: FmtStream>(s: &mut S, st: &mut EngineState) -> Result<(), Abort> {
    s.constant(FORMAT_HEADER_MAGIC)?;
    let serial = FORMAT_SERIAL as i32;
    let mut v = serial;
    s.int(&mut v)?;
    if v != serial {
        return Err(Abort::Internal("format file is of the wrong version"));
    }

    let eqtb = &mut st.eqtb;
    s.items_in(
        slice::from_mut(&mut eqtb.hash_high),
        0,
        SUP_HASH_EXTRA.cast_unsigned(),
    )?;
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

    s.constant(CsId::UNDEFINED.raw())?;
    s.constant(HASH_PRIME)?;
    s.constant(HYPH_PRIME)
}

fn string_pool<S: FmtStream>(
    s: &mut S,
    st: &mut EngineState,
    env: &mut EngineEnv,
) -> Result<(), Abort> {
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
        print_ln(st, env);
        print_int(st, env, str_ptr.raw());
        print(st, env, " strings of total length ");
        print_int(st, env, pool_ptr.raw());
    }
    Ok(())
}

fn dynamic_memory<S: FmtStream>(
    s: &mut S,
    st: &mut EngineState,
    env: &mut EngineEnv,
) -> Result<(), Abort> {
    let mem = &mut st.mem;
    let too_big = || Abort::Internal("mem exceeds the format's limit");
    let mut words = i32::try_from(mem.data.len()).map_err(|_| too_big())?;
    let reserved = i32::try_from(RESERVED_WORDS).map_err(|_| too_big())?;
    s.int_in(&mut words, reserved, i32::MAX)?;
    let last = MemIdx::from_raw(words - 1);
    s.items_in(&mut mem.free.0, None, last)?;
    if S::LOADING {
        mem.data = zeroed_vec(usize::try_from(words).map_err(|_| bad_fmt())?).into();
    }
    s.items(mem.data.unjournaled_mut())?;
    token_lists(s, &mut mem.tokens)?;
    glue_specs(s, &mut mem.glue)?;

    if !S::LOADING {
        print_ln(st, env);
        print_int(st, env, words);
        print(st, env, " memory locations dumped");
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
    s.items(glue.specs_unjournaled_mut())
}

fn token_lists<S: FmtStream>(s: &mut S, toks: &mut TokenStore) -> Result<(), Abort> {
    let mut marks = toks.marks();
    let too_big = || Abort::Internal("token store exceeds the format's limit");
    let mut tokens = i32::try_from(marks.tokens).map_err(|_| too_big())?;
    let mut lists = i32::try_from(marks.lists).map_err(|_| too_big())?;
    s.int_in(&mut tokens, 1, i32::MAX)?;
    s.int_in(&mut lists, 2, i32::MAX)?;
    s.items(&mut marks.free_runs)?;
    s.items(slice::from_mut(&mut marks.free_lists))?;
    if S::LOADING {
        marks.tokens = usize::try_from(tokens).map_err(|_| bad_fmt())?;
        marks.lists = usize::try_from(lists).map_err(|_| bad_fmt())?;
        toks.reset_for_load(marks);
    }
    let (tokens, lists) = toks.parts_unjournaled_mut();
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

fn runs<S: FmtStream, T: FmtItem + PartialEq>(s: &mut S, v: &mut [T]) -> Result<(), Abort> {
    if S::LOADING {
        undump_runs(s, v)
    } else {
        dump_runs(s, v, T::eq)
    }
}

/// The `\XeTeXinterchartoks` lists: a count, then each pair's classes and list.
fn inter_char_toks<S: FmtStream>(s: &mut S, eqtb: &mut EqtbState) -> Result<(), Abort> {
    let mut entries: Vec<([i32; 2], TokenList)> = eqtb
        .inter_char
        .entries()
        .map(|(pair, list, _)| (pair.classes(), list))
        .collect();
    let mut n = i32::try_from(entries.len()).map_err(|_| bad_fmt())?;
    s.int_in(&mut n, 0, i32::MAX)?;
    if S::LOADING {
        let n = usize::try_from(n).map_err(|_| bad_fmt())?;
        entries = vec![([0; 2], TokenList::NULL); n];
    }
    for (classes, list) in &mut entries {
        s.items(classes)?;
        s.items(slice::from_mut(list))?;
    }
    if S::LOADING {
        eqtb.inter_char.clear();
        for ([first, second], list) in entries {
            let pair = InterCharPair::new(first, second).ok_or_else(bad_fmt)?;
            eqtb.inter_char.set(pair, list, LEVEL_ONE);
        }
    }
    Ok(())
}

fn table_of_equivalents<S: FmtStream>(
    s: &mut S,
    st: &mut EngineState,
    env: &mut EngineEnv,
) -> Result<(), Abort> {
    let eqtb = &mut st.eqtb;
    let named = CsId::FIRST.index()..=CsId::top(eqtb.hash_high).index();
    runs(
        s,
        &mut eqtb.meanings.values_unjournaled_mut()[named.clone()],
    )?;
    runs(s, &mut eqtb.meanings.levels_unjournaled_mut()[named])?;
    runs(s, eqtb.cat_codes.values_unjournaled_mut())?;
    runs(s, eqtb.lc_codes.values_unjournaled_mut())?;
    runs(s, eqtb.uc_codes.values_unjournaled_mut())?;
    runs(s, eqtb.sf_codes.values_unjournaled_mut())?;
    runs(s, eqtb.math_codes.values_unjournaled_mut())?;
    runs(s, eqtb.del_codes.values_unjournaled_mut())?;
    runs(s, eqtb.ints.values_unjournaled_mut())?;
    runs(s, eqtb.dimens.values_unjournaled_mut())?;
    runs(s, eqtb.glue.values_unjournaled_mut())?;
    runs(s, eqtb.toks.values_unjournaled_mut())?;
    runs(s, eqtb.shapes.values_unjournaled_mut())?;
    runs(s, eqtb.boxes.values_unjournaled_mut())?;
    runs(s, eqtb.fonts.values_unjournaled_mut())?;
    inter_char_toks(s, eqtb)?;

    // \partokenname may name an active character or a control symbol.
    s.items_in(
        slice::from_mut(&mut st.input.par_loc),
        Some(CsId::FIRST),
        Some(CsId::top(eqtb.hash_high)),
    )?;
    if S::LOADING {
        st.input.par_token = Token::from_cs(st.input.par_loc.ok_or_else(bad_fmt)?);
    }
    s.items_in(
        slice::from_mut(&mut st.shipout.write_loc),
        Some(NameSlot::bucket(0).cs()),
        Some(eqtb.cs_top),
    )?;

    s.items(&mut eqtb.prim.unjournaled_mut()[..=PRIM_SIZE as usize])?;

    // The buckets from 0 through `hash_used` go entry by entry, as WEB dumps
    // them. A bucket's slot number is its index in `hash`, and one past the last
    // bucket is the first frozen control sequence, which WEB's loop includes.
    let mut used = eqtb.hash_used.raw();
    s.items_in(slice::from_mut(&mut used), 0, NameSlot::BUCKETS_END.raw())?;
    eqtb.hash_used = NameSlot::from_raw(used);
    let used = used as usize;
    if S::LOADING {
        let mut lowest = 0;
        loop {
            let mut p: u32 = lowest;
            s.items_in(slice::from_mut(&mut p), lowest, used as u32)?;
            s.items(slice::from_mut(
                &mut eqtb.hash.unjournaled_mut()[p as usize],
            ))?;
            if p as usize == used {
                break;
            }
            lowest = p + 1;
        }
    } else {
        eqtb.cs_count = NameSlot::BUCKETS_END.raw().cast_signed() - (used as i32) - 1
            + eqtb.hash_high.cast_signed();
        for p in 0..=used {
            let entry = &mut eqtb.hash.unjournaled_mut()[p];
            if entry.text != StrNumber::NONE {
                s.items(slice::from_mut(&mut (p as u32)))?;
                s.items(slice::from_mut(entry))?;
                eqtb.cs_count += 1;
            }
        }
    }

    let undefined = CsId::UNDEFINED.hash_index().ok_or_else(bad_fmt)?;
    s.items(&mut eqtb.hash.unjournaled_mut()[used + 1..undefined])?;
    let extras = eqtb.hash_high as usize;
    s.items(&mut eqtb.hash.unjournaled_mut()[undefined + 1..=undefined + extras])?;
    s.int(&mut eqtb.cs_count)?;

    if !S::LOADING {
        print_ln(st, env);
        print_int(st, env, st.eqtb.cs_count);
        print(st, env, " multiletter control sequences");
    }
    Ok(())
}

fn font_info<S: FmtStream>(
    s: &mut S,
    st: &mut EngineState,
    env: &mut EngineEnv,
) -> Result<(), Abort> {
    let max_str = st.strings.str_ptr;
    let fonts = &mut st.fonts;
    s.size(
        &mut fonts.fmem_ptr,
        7,
        SUP_FONT_MEM_SIZE,
        "must increase font_mem_size",
    )?;
    if S::LOADING {
        fonts.font_mem_size = fonts.font_mem_size.max(fonts.fmem_ptr);
    }

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
    s.items(fonts_of(&mut fonts.font_check, last))?;
    s.items(fonts_of(&mut fonts.font_size, last))?;
    s.items(fonts_of(&mut fonts.font_dsize, last))?;
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
    for (metrics, params) in fonts_of(&mut fonts.metrics, last)
        .iter_mut()
        .zip(fonts_of(&mut fonts.params, last))
    {
        font_metrics(s, metrics, params)?;
    }
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
        print_dumped_fonts(st, env)?;
    }
    Ok(())
}

fn counted_items<S: FmtStream, T: FmtItem>(
    s: &mut S,
    v: &mut Vec<T>,
    max: i32,
    fill: T,
) -> Result<(), Abort> {
    let mut n = i32::try_from(v.len()).map_err(|_| bad_fmt())?;
    s.int_in(&mut n, 0, max)?;
    if S::LOADING {
        *v = vec![fill; usize::try_from(n).map_err(|_| bad_fmt())?];
    }
    s.items(v)
}

fn font_metrics<S: FmtStream>(
    s: &mut S,
    metrics: &mut FontMetrics,
    params: &mut FontParams,
) -> Result<(), Abort> {
    let mut native = i32::from(matches!(metrics, FontMetrics::Native { .. }));
    s.int_in(&mut native, 0, 1)?;
    if native == 1 {
        let mut extent = match *metrics {
            FontMetrics::Native { ascent, descent } => [ascent, descent],
            FontMetrics::Tfm(_) => [Scaled::ZERO; 2],
        };
        s.items(&mut extent)?;
        if S::LOADING {
            let [ascent, descent] = extent;
            *metrics = FontMetrics::Native { ascent, descent };
        }
    } else {
        let tfm = match metrics {
            FontMetrics::Tfm(m) => Rc::clone(m),
            FontMetrics::Native { .. } => Rc::default(),
        };
        let mut bc = tfm.bc();
        s.items(slice::from_mut(&mut bc))?;
        let mut chars = tfm.chars().to_vec();
        counted_items(s, &mut chars, 256, None)?;
        let mut lig_kern = tfm.lig_kern().to_vec();
        let jump = LigKernStep::Jump(LigKernIdx::from_raw(0));
        counted_items(s, &mut lig_kern, 32_767, jump)?;
        let mut exten = tfm.exten().to_vec();
        let empty = ExtRecipe {
            top: None,
            mid: None,
            bot: None,
            rep: 0,
        };
        counted_items(s, &mut exten, 256, empty)?;
        let mut label = tfm.bchar_label().map_or(-1, |k| i32::from(k.raw()));
        s.int_in(&mut label, -1, 32_767)?;
        if S::LOADING {
            let label = u16::try_from(label).ok().map(LigKernIdx::from_raw);
            let tfm = TfmMetrics::new(bc, chars.into(), lig_kern.into(), exten.into(), label)
                .ok_or_else(bad_fmt)?;
            *metrics = FontMetrics::Tfm(Rc::new(tfm));
        }
    }
    let mut values = params.as_slice().to_vec();
    counted_items(s, &mut values, SUP_FONT_MEM_SIZE, Scaled::ZERO)?;
    if S::LOADING {
        *params = FontParams::new(values);
    }
    Ok(())
}

/// Rejects a format whose eqtb names a font it does not load.
fn check_font_words(eqtb: &EqtbState, font_ptr: FontId) -> Result<(), Abort> {
    let meanings_ok = eqtb.meanings.values().iter().all(|m| match m {
        Command::SetFont(f) => *f <= font_ptr,
        _ => true,
    });
    if meanings_ok && eqtb.fonts.values().iter().all(|&f| f <= font_ptr) {
        Ok(())
    } else {
        Err(bad_fmt())
    }
}

fn print_dumped_fonts(st: &mut EngineState, env: &mut EngineEnv) -> Result<(), Abort> {
    let font_ptr = st.fonts.font_ptr;
    for f in iter::once(NULL_FONT).chain(font_ptr.loaded()) {
        print_nl(st, env, "\\font");
        print_esc(st, env, st.eqtb.font_id_text(f));
        print_char(st, env, b'=');

        let font_name = st.fonts.font_name[f];
        let font_area = st.fonts.font_area[f];
        if font_area == AAT_FONT_FLAG
            || font_area == OTGR_FONT_FLAG
            || st.fonts.font_mapping[f].is_some()
        {
            print_file_name(st, env, font_name, EMPTY_STRING, EMPTY_STRING);
            error(
                st,
                env,
                "Can't \\dump a format with native fonts or font-mappings",
                &[
                    "You really, really don't want to do this.",
                    "It won't work, and only confuses me.",
                    "(Load them at runtime, not as part of the format file.)",
                ],
            )?;
        } else {
            print_file_name(st, env, font_name, font_area, EMPTY_STRING);
        }

        let font_size = st.fonts.font_size[f];
        if font_size != st.fonts.font_dsize[f] {
            print(st, env, " at ");
            print_scaled(st, env, font_size);
            print(st, env, "pt");
        }
    }

    print_ln(st, env);
    print_int(st, env, st.fonts.fmem_ptr - 7);
    print(st, env, " words of font info for ");
    print_int(st, env, font_ptr.raw());
    if font_ptr.raw() != FONT_BASE + 1 {
        print(st, env, " preloaded fonts");
    } else {
        print(st, env, " preloaded font");
    }
    Ok(())
}

fn hyphenation_tables<S: FmtStream>(
    s: &mut S,
    st: &mut EngineState,
    env: &mut EngineEnv,
) -> Result<(), Abort> {
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
                slice::from_mut(&mut hyph.hyph_word[h]),
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
                s.items(slice::from_mut(&mut hyph.hyph_word[h]))?;
                s.items(slice::from_mut(&mut hyph.hyph_list[h]))?;
            }
        }

        let hyph_count = hyph.hyph_count;
        print_ln(st, env);
        print_int(st, env, hyph_count);
        if hyph_count != 1 {
            print(st, env, " hyphenation exceptions");
        } else {
            print(st, env, " hyphenation exception");
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
        slice::from_mut(&mut hyph.hyph_start),
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
        print_nl(st, env, "Hyphenation trie of length ");
        print_int(st, env, trie_max.raw());
        print(st, env, " has ");
        print_int(st, env, op_count);
        if op_count != 1 {
            print(st, env, " ops");
        } else {
            print(st, env, " op");
        }
        print(st, env, " out of ");
        print_int(st, env, TRIE_OP_SIZE);

        for k in (0..=BIGGEST_LANG).rev() {
            let mut used = i32::from(st.hyph.trie_used[k as usize]);
            if used > 0 {
                print_nl(st, env, "  ");
                print_int(st, env, used);
                print(st, env, " for language ");
                print_int(st, env, k);
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
    use crate::base::cmd::Cmd;
    use crate::base::consts::LEVEL_ONE;
    use crate::env::test_env;
    use crate::fonts::tfm::{
        CharInfo, CharTag, ExtIdx, ExtRecipe, LigKernAction, LigKernIdx, LigKernStep, LigOp,
    };
    use crate::print::Selector;
    use crate::tables::eqtb::FontLoc;

    fn eqtb_with_fonts() -> EqtbState {
        let mut eqtb = EqtbState::default();
        eqtb.alloc_tables();
        eqtb
    }

    #[test]
    fn format_load_rejects_a_font_word_past_font_ptr() {
        let font_ptr = FontId::new(3);
        let mut eqtb = eqtb_with_fonts();
        let id = CsId::font_id(FontId::new(2));
        eqtb.set_meaning(id, Command::SetFont(FontId::new(3)), LEVEL_ONE);
        eqtb.fonts.set_value(FontLoc::math(4), FontId::new(2));
        assert!(check_font_words(&eqtb, font_ptr).is_ok());

        eqtb.set_meaning(id, Command::SetFont(FontId::new(4)), LEVEL_ONE);
        assert!(check_font_words(&eqtb, font_ptr).is_err());
        let mut wide = Vec::new();
        u16::from(Cmd::SetFont).put(&mut wide);
        0x1_0002_i32.put(&mut wide);
        assert_eq!(Command::get(&wide), None);
        eqtb.set_meaning(id, Command::SetFont(FontId::new(3)), LEVEL_ONE);
        eqtb.fonts.set_value(FontLoc::math(4), FontId::new(4));
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
                *x = T::get(head).ok_or_else(bad_fmt)?;
                self.0 = rest;
            }
            Ok(())
        }
    }

    fn font_state() -> EngineState {
        let mut st = EngineState::default();
        st.shipout.selector = Selector::NoPrint;
        st.eqtb.alloc_tables();
        st.strings.str_start = vec![PoolPos::ZERO; 4];
        st.fonts.font_max = FontId::new(1);
        st.fonts.fmem_ptr = 7;
        st.fonts.alloc_font_arrays();
        st
    }

    #[test]
    fn font_info_load_restores_font_ptr() {
        let env = &mut test_env();
        let mut src = font_state();
        src.fonts.font_ptr = NULL_FONT;
        src.fonts.font_size[NULL_FONT] = Scaled::from_raw(12);
        let mut dump = VecDump::default();
        font_info(&mut dump, &mut src, env).expect("dump");

        let mut dst = font_state();
        dst.fonts.font_ptr = FontId::new(1);
        font_info(&mut SliceUndump(&dump.0), &mut dst, env).expect("load");
        assert_eq!(dst.fonts.font_ptr, NULL_FONT);
        assert_eq!(dst.fonts.font_size[NULL_FONT], Scaled::from_raw(12));
    }

    #[test]
    fn font_metrics_round_trip() {
        let env = &mut test_env();
        let pt = |v: i32| Scaled::from_raw(v * 65_536);
        let ci = |tag| CharInfo {
            width: pt(5),
            height: pt(7),
            depth: pt(1),
            italic: pt(2),
            tag,
        };
        let metrics = TfmMetrics::new(
            65,
            vec![
                Some(ci(CharTag::Lig(LigKernIdx::from_raw(0)))),
                None,
                Some(ci(CharTag::List(65))),
                Some(ci(CharTag::Ext(ExtIdx::from_raw(0)))),
            ]
            .into(),
            vec![
                LigKernStep::Rule {
                    next_char: 66,
                    skip: Some(0),
                    action: LigKernAction::Kern(pt(-1)),
                },
                LigKernStep::Rule {
                    next_char: 67,
                    skip: None,
                    action: LigKernAction::Lig {
                        op: LigOp::KeepBothSkip2,
                        char: 68,
                    },
                },
                LigKernStep::Jump(LigKernIdx::from_raw(1)),
            ]
            .into(),
            vec![ExtRecipe {
                top: Some(65),
                mid: None,
                bot: Some(67),
                rep: 68,
            }]
            .into(),
            Some(LigKernIdx::from_raw(2)),
        )
        .expect("consistent metrics");
        let metrics = FontMetrics::Tfm(Rc::new(metrics));
        let params = FontParams::new(vec![pt(3), Scaled::ZERO, pt(-4)]);

        let mut src = font_state();
        src.fonts.font_ptr = NULL_FONT;
        src.fonts.metrics[NULL_FONT] = metrics.clone();
        src.fonts.params[NULL_FONT] = params.clone();
        let mut dump = VecDump::default();
        font_info(&mut dump, &mut src, env).expect("dump");

        let mut dst = font_state();
        font_info(&mut SliceUndump(&dump.0), &mut dst, env).expect("load");
        assert_eq!(dst.fonts.metrics[NULL_FONT], metrics);
        assert_eq!(dst.fonts.params[NULL_FONT], params);
    }

    #[test]
    fn inconsistent_metrics_are_rejected() {
        let dangling = CharInfo {
            tag: CharTag::Lig(LigKernIdx::from_raw(0)),
            ..CharInfo::NULL
        };
        let metrics = TfmMetrics::new(
            0,
            vec![Some(dangling)].into(),
            Box::new([]),
            Box::new([]),
            None,
        );
        assert_eq!(metrics, None);
        let cycle = CharInfo {
            tag: CharTag::List(0),
            ..CharInfo::NULL
        };
        let metrics = TfmMetrics::new(
            0,
            vec![Some(cycle)].into(),
            Box::new([]),
            Box::new([]),
            None,
        );
        assert_eq!(metrics, None);
    }
}
