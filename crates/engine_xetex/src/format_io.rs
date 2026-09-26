//! Format-file dump and load (`\dump` and the preloaded format).
//!
//! Every multi-byte item is stored big-endian.

use crate::bridge::{
    input_open, input_read, output_close, output_flush, output_open, output_write,
    TTBC_FILE_FORMAT_FORMAT,
};
use crate::cmd::UNDEFINED_CS;
use crate::consts::{
    ACTIVE_BASE, CS_TOKEN_FLAG, EMPTY_STRING, EQTB_SIZE, FONT_BASE, FROZEN_CONTROL_SEQUENCE,
    HASH_BASE, HISTORY_FATAL_ERROR, INT_BASE, INT_PAR_DAY, INT_PAR_MONTH, INT_PAR_TRACING_STATS,
    INT_PAR_YEAR, INVALID_HANDLE, MAX_FONT_MAX, MAX_HALFWORD, MEM_TOP, MIN_HALFWORD, PRIM_SIZE,
    SELECTOR_LOG_ONLY, SELECTOR_NEW_STRING, SELECTOR_TERM_AND_LOG, TEX_NULL, TOO_BIG_CHAR,
    UNDEFINED_CONTROL_SEQUENCE,
};
use crate::engine::{POOL_FREE, STRINGS_FREE};
use crate::errors::Abort;
use crate::fonts::{AAT_FONT_FLAG, OTGR_FONT_FLAG};
use crate::hash::{HASH_OFFSET, HASH_PRIME};
use crate::hyphenation::{BIGGEST_LANG, HYPH_PRIME, MAX_TRIE_OP, TRIE_OP_SIZE};
use crate::mem::{CONTRIB_HEAD, HI_MEM_STAT_MIN, HI_MEM_STAT_USAGE, PAGE_HEAD};
use crate::scan::{INTER_CHAR_VAL, INT_VAL};
use crate::state::{EngineState, EqtbState, InteractionMode};
use crate::units::{FontId, StrNumber};

const FORMAT_HEADER_MAGIC: i32 = 0x5454_4E43;
const FORMAT_FOOTER_MAGIC: i32 = 0x0000_029A;
const SUP_MAX_STRINGS: i32 = 2_097_151;
const SUP_FONT_MEM_SIZE: i32 = 147_483_647;
const SUP_POOL_SIZE: i32 = 40_000_000;
const SUP_HASH_EXTRA: i32 = 2_097_151;

fn bad_fmt() -> Abort {
    Abort::Internal("fatal format file error")
}

/// Plain data stored in a format file: every byte pattern is a valid value
/// and there is no padding.
///
/// # Safety
///
/// Only implement this for such types.
unsafe trait FmtItem: Copy {}

// SAFETY: integers, and `repr(C)` words made of 8 bytes of integers or an f64.
unsafe impl FmtItem for u8 {}
// SAFETY: as above.
unsafe impl FmtItem for u16 {}
// SAFETY: as above.
unsafe impl FmtItem for i16 {}
// SAFETY: as above.
unsafe impl FmtItem for i32 {}
// SAFETY: `Scaled` is a transparent `i32`.
unsafe impl FmtItem for crate::units::Scaled {}
// SAFETY: `StrNumber` is a transparent `i32`.
unsafe impl FmtItem for crate::units::StrNumber {}
// SAFETY: as above.
unsafe impl FmtItem for crate::mem_types::B16x4 {}
// SAFETY: as above.
unsafe impl FmtItem for crate::mem_types::B32x2 {}
// SAFETY: as above.
unsafe impl FmtItem for crate::mem_types::MemoryWord {}

fn fmt_bytes<T: FmtItem>(items: &[T]) -> &[u8] {
    // SAFETY: `FmtItem` values have no padding, so every byte is initialised.
    unsafe { std::slice::from_raw_parts(items.as_ptr().cast(), std::mem::size_of_val(items)) }
}

fn fmt_bytes_mut<T: FmtItem>(items: &mut [T]) -> &mut [u8] {
    // SAFETY: as `fmt_bytes`, and any bytes written form a valid `FmtItem`.
    unsafe {
        std::slice::from_raw_parts_mut(items.as_mut_ptr().cast(), std::mem::size_of_val(items))
    }
}

/// Writes `v[start..start + n]`, byte-swapping each item to big-endian.
fn dump_slice<T: FmtItem>(fmt_out: usize, v: &[T], start: i32, n: i32) -> Result<(), Abort> {
    if n <= 0 {
        return Ok(());
    }
    let items = &v[start as usize..(start + n) as usize];
    let item_size = std::mem::size_of::<T>();
    let byte_len = std::mem::size_of_val(items);
    let written = if cfg!(target_endian = "little") && item_size > 1 {
        let mut bytes = fmt_bytes(items).to_vec();
        for item in bytes.chunks_exact_mut(item_size) {
            item.reverse();
        }
        output_write(fmt_out, &bytes)
    } else {
        output_write(fmt_out, fmt_bytes(items))
    };

    if written != byte_len {
        return Err(Abort::Internal("could not write format file data"));
    }
    Ok(())
}

fn dump_one<T: FmtItem>(fmt_out: usize, value: T) -> Result<(), Abort> {
    dump_slice(fmt_out, &[value], 0, 1)
}

fn dump_int(fmt_out: usize, value: i32) -> Result<(), Abort> {
    dump_one(fmt_out, value)
}

/// Reads `n` items into `v[start..]`, converting each from big-endian.
fn undump_slice<T: FmtItem>(fmt_in: usize, v: &mut [T], start: i32, n: i32) -> Result<(), Abort> {
    if n <= 0 {
        return Ok(());
    }
    let items = &mut v[start as usize..(start + n) as usize];
    let item_size = std::mem::size_of::<T>();
    let bytes = fmt_bytes_mut(items);
    let byte_len = bytes.len();
    let read = input_read(fmt_in, bytes);
    if read < 0 || read as usize != byte_len {
        return Err(Abort::Internal("could not read format file data"));
    }

    if cfg!(target_endian = "little") && item_size > 1 {
        for item in bytes.chunks_exact_mut(item_size) {
            item.reverse();
        }
    }
    Ok(())
}

fn undump_int(fmt_in: usize) -> Result<i32, Abort> {
    let mut value = [0i32];
    undump_slice(fmt_in, &mut value, 0, 1)?;
    Ok(value[0])
}

/// Reads an int that must lie in `low..=high`.
fn undump_in_range(fmt_in: usize, low: i32, high: i32) -> Result<i32, Abort> {
    let x = undump_int(fmt_in)?;
    if x < low || x > high {
        return Err(bad_fmt());
    }
    Ok(x)
}

/// Reads a size that must be at least `low` and fit the engine's `high` limit.
fn undump_size(fmt_in: usize, low: i32, high: i32, too_big: &'static str) -> Result<i32, Abort> {
    let x = undump_int(fmt_in)?;
    if x < low {
        return Err(bad_fmt());
    }
    if x > high {
        return Err(Abort::Internal(too_big));
    }
    Ok(x)
}

fn expect_int(fmt_in: usize, expected: i32) -> Result<(), Abort> {
    if undump_int(fmt_in)? != expected {
        return Err(bad_fmt());
    }
    Ok(())
}

/// Reads `n` items into `v[start..]` that must each lie in `low..=high`.
fn undump_checked<T: FmtItem + PartialOrd>(
    fmt_in: usize,
    v: &mut [T],
    start: i32,
    n: i32,
    low: T,
    high: T,
) -> Result<(), Abort> {
    undump_slice(fmt_in, v, start, n)?;
    if n > 0 {
        let items = &v[start as usize..(start + n) as usize];
        if items.iter().any(|&x| x < low || x > high) {
            return Err(bad_fmt());
        }
    }
    Ok(())
}

/// Dumps `eqtb[k..=last]` as alternating runs of distinct words and of
/// repeats of the preceding word, returning the next index.
fn dump_eqtb_runs(
    fmt_out: usize,
    eqtb: &EqtbState,
    mut k: i32,
    last: i32,
    same: impl Fn(&EqtbState, i32, i32) -> bool,
) -> Result<i32, Abort> {
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

        dump_int(fmt_out, l - k)?;
        dump_slice(fmt_out, &eqtb.data, k, l - k)?;
        k = j + 1;
        dump_int(fmt_out, k - l)?;
        if k > last {
            return Ok(k);
        }
    }
}

pub(crate) fn store_fmt_file(st: &mut EngineState) -> Result<(), Abort> {
    if st.save_stack.save_ptr != 0 {
        crate::output::print_err(st, "You can't dump inside a group");
        st.errors.help(&[r"`{...\dump}' is a no-no."]);

        if st.errors.interaction == InteractionMode::ErrorStop {
            st.errors.interaction = InteractionMode::Scroll;
        }
        if st.errors.log_opened {
            crate::errors::error(st)?;
        }

        st.errors.history = HISTORY_FATAL_ERROR;
        let _ = crate::io::close_files_and_terminate(st);
        output_flush(st.shipout.rust_stdout);
        return Err(Abort::Internal("\\dump inside a group"));
    }

    st.shipout.selector = SELECTOR_NEW_STRING;
    crate::output::print(st, " (preloaded format=");
    crate::output::print(st, st.strings.job_name);
    crate::output::print_char(st, b' ');
    crate::output::print_int(st, st.eqtb.int_par(INT_PAR_YEAR));
    crate::output::print_char(st, b'.');
    crate::output::print_int(st, st.eqtb.int_par(INT_PAR_MONTH));
    crate::output::print_char(st, b'.');
    crate::output::print_int(st, st.eqtb.int_par(INT_PAR_DAY));
    crate::output::print_char(st, b')');

    st.shipout.selector = if st.errors.interaction == InteractionMode::Batch {
        SELECTOR_LOG_ONLY
    } else {
        SELECTOR_TERM_AND_LOG
    };

    if st.strings.pool_ptr + 1 > st.strings.pool_size {
        return Err(Abort::PoolOverflow);
    }

    st.strings.format_ident = st.strings.make_string()?;
    crate::io::pack_job_name(st, ".fmt")?;

    let fmt_out = output_open(st.input.name_of_file_bytes(), false);
    if fmt_out == INVALID_HANDLE {
        return Err(Abort::Internal("cannot open format output file"));
    }

    crate::output::print_nl(st, "Beginning to dump on file ");
    let name_string = crate::io::make_name_string(st)?;
    crate::output::print(st, name_string);

    st.strings.str_ptr -= 1;
    st.strings.discard_cur_string();

    crate::output::print_nl(st, "");
    crate::output::print(st, st.strings.format_ident);

    dump_int(fmt_out, FORMAT_HEADER_MAGIC)?;
    dump_int(fmt_out, crate::FORMAT_SERIAL as i32)?;
    dump_int(fmt_out, st.eqtb.hash_high)?;

    while st.input.pseudo_files.0 != TEX_NULL {
        st.mem.pseudo_close(&mut st.input);
    }

    dump_int(fmt_out, MEM_TOP)?;
    dump_int(fmt_out, EQTB_SIZE)?;
    dump_int(fmt_out, HASH_PRIME)?;
    dump_int(fmt_out, HYPH_PRIME)?;

    let pool_ptr = st.strings.pool_ptr;
    let str_ptr = st.strings.str_ptr;
    dump_int(fmt_out, pool_ptr)?;
    dump_int(fmt_out, str_ptr)?;
    dump_slice(
        fmt_out,
        &st.strings.str_start,
        0,
        str_ptr - TOO_BIG_CHAR + 1,
    )?;
    dump_slice(fmt_out, &st.strings.str_pool, 0, pool_ptr)?;

    crate::output::print_ln(st);
    crate::output::print_int(st, str_ptr);
    crate::output::print(st, " strings of total length ");
    crate::output::print_int(st, pool_ptr);

    st.mem.sort_avail();
    let mem = &st.mem;
    let (lo_mem_max, rover, hi_mem_min, mem_end, avail) = (
        mem.lo_mem_max,
        mem.rover,
        mem.hi_mem_min,
        mem.mem_end,
        mem.avail,
    );
    dump_int(fmt_out, lo_mem_max)?;
    dump_int(fmt_out, rover)?;

    for root in &st.scanner.sa_root[INT_VAL as usize..=INTER_CHAR_VAL as usize] {
        dump_int(fmt_out, root.0)?;
    }

    let mut var_used = 0;
    let mut p = 0;
    let mut q = rover;
    let mut x = 0;
    loop {
        dump_slice(fmt_out, &mem.data, p, q + 2 - p)?;
        x += q + 2 - p;
        var_used += q - p;
        let block = crate::node::NodePtr::<crate::node::FreeBlock>::at(q);
        p = q + block.node_size(mem);
        q = block.rlink(mem);
        if q == rover {
            break;
        }
    }

    var_used += lo_mem_max - p;
    let mut dyn_used = mem_end + 1 - hi_mem_min;
    dump_slice(fmt_out, &mem.data, p, lo_mem_max + 1 - p)?;
    x += lo_mem_max + 1 - p;

    dump_int(fmt_out, hi_mem_min)?;
    dump_int(fmt_out, avail)?;
    dump_slice(fmt_out, &mem.data, hi_mem_min, mem_end + 1 - hi_mem_min)?;
    x += mem_end + 1 - hi_mem_min;

    let mut p = crate::units::MemPtr(avail);
    while !p.is_null() {
        dyn_used -= 1;
        p = p.link(mem);
    }

    st.mem.var_used = var_used;
    st.mem.dyn_used = dyn_used;
    dump_int(fmt_out, var_used)?;
    dump_int(fmt_out, dyn_used)?;

    crate::output::print_ln(st);
    crate::output::print_int(st, x);
    crate::output::print(st, " memory locations dumped; current usage is ");
    crate::output::print_int(st, var_used);
    crate::output::print_char(st, b'&');
    crate::output::print_int(st, dyn_used);

    let k = dump_eqtb_runs(
        fmt_out,
        &st.eqtb,
        ACTIVE_BASE,
        INT_BASE - 1,
        |eqtb, a, b| {
            eqtb.equiv(a) == eqtb.equiv(b)
                && eqtb.eq_type(a) == eqtb.eq_type(b)
                && eqtb.eq_level(a) == eqtb.eq_level(b)
        },
    )?;
    dump_eqtb_runs(fmt_out, &st.eqtb, k, EQTB_SIZE, |eqtb, a, b| {
        eqtb.int_at(a) == eqtb.int_at(b)
    })?;

    let hash_high = st.eqtb.hash_high;
    let hash_used = st.eqtb.hash_used;
    dump_slice(fmt_out, &st.eqtb.data, EQTB_SIZE + 1, hash_high)?;

    dump_int(fmt_out, st.input.par_loc)?;
    dump_int(fmt_out, st.shipout.write_loc)?;

    dump_slice(fmt_out, &st.eqtb.prim, 0, PRIM_SIZE + 1)?;

    dump_int(fmt_out, hash_used)?;
    let mut cs_count = (FROZEN_CONTROL_SEQUENCE - 1) - hash_used + hash_high;

    let hash = &st.eqtb.hash;
    for p in HASH_BASE..=hash_used {
        let entry = hash[(p - HASH_OFFSET) as usize];
        if entry.s1 != 0 {
            dump_int(fmt_out, p)?;
            dump_one(fmt_out, entry)?;
            cs_count += 1;
        }
    }

    dump_slice(
        fmt_out,
        hash,
        hash_used + 1 - HASH_OFFSET,
        (UNDEFINED_CONTROL_SEQUENCE - 1) - hash_used,
    )?;
    dump_slice(fmt_out, hash, EQTB_SIZE + 1 - HASH_OFFSET, hash_high)?;

    dump_int(fmt_out, cs_count)?;

    crate::output::print_ln(st);
    crate::output::print_int(st, cs_count);
    crate::output::print(st, " multiletter control sequences");

    let fmp = st.fonts.fmem_ptr;
    let font_ptr = st.fonts.font_ptr.0;
    let n = font_ptr + 1;
    let fonts = &st.fonts;
    dump_int(fmt_out, fmp)?;
    dump_slice(fmt_out, &fonts.font_info, 0, fmp)?;
    dump_int(fmt_out, font_ptr)?;
    dump_slice(fmt_out, fonts.font_check.as_slice(), FONT_BASE, n)?;
    dump_slice(fmt_out, fonts.font_size.as_slice(), FONT_BASE, n)?;
    dump_slice(fmt_out, fonts.font_dsize.as_slice(), FONT_BASE, n)?;
    dump_slice(fmt_out, fonts.font_params.as_slice(), FONT_BASE, n)?;
    dump_slice(fmt_out, fonts.hyphen_char.as_slice(), FONT_BASE, n)?;
    dump_slice(fmt_out, fonts.skew_char.as_slice(), FONT_BASE, n)?;
    dump_slice(fmt_out, fonts.font_name.as_slice(), FONT_BASE, n)?;
    dump_slice(fmt_out, fonts.font_area.as_slice(), FONT_BASE, n)?;
    dump_slice(fmt_out, fonts.font_bc.as_slice(), FONT_BASE, n)?;
    dump_slice(fmt_out, fonts.font_ec.as_slice(), FONT_BASE, n)?;
    dump_slice(fmt_out, fonts.char_base.as_slice(), FONT_BASE, n)?;
    dump_slice(fmt_out, fonts.width_base.as_slice(), FONT_BASE, n)?;
    dump_slice(fmt_out, fonts.height_base.as_slice(), FONT_BASE, n)?;
    dump_slice(fmt_out, fonts.depth_base.as_slice(), FONT_BASE, n)?;
    dump_slice(fmt_out, fonts.italic_base.as_slice(), FONT_BASE, n)?;
    dump_slice(fmt_out, fonts.lig_kern_base.as_slice(), FONT_BASE, n)?;
    dump_slice(fmt_out, fonts.kern_base.as_slice(), FONT_BASE, n)?;
    dump_slice(fmt_out, fonts.exten_base.as_slice(), FONT_BASE, n)?;
    dump_slice(fmt_out, fonts.param_base.as_slice(), FONT_BASE, n)?;
    dump_slice(fmt_out, fonts.font_glue.as_slice(), FONT_BASE, n)?;
    dump_slice(fmt_out, fonts.bchar_label.as_slice(), FONT_BASE, n)?;
    dump_slice(fmt_out, fonts.font_bchar.as_slice(), FONT_BASE, n)?;
    dump_slice(fmt_out, fonts.font_false_bchar.as_slice(), FONT_BASE, n)?;

    for k in FONT_BASE..=font_ptr {
        let f = FontId(k);
        crate::output::print_nl(st, "\\font");
        crate::output::print_esc(st, st.eqtb.font_id_text(f));
        crate::output::print_char(st, b'=');

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
            crate::errors::error(st)?;
        } else {
            crate::output::print_file_name(st, font_name, font_area, EMPTY_STRING);
        }

        let font_size = st.fonts.font_size[f];
        if font_size != st.fonts.font_dsize[f] {
            crate::output::print(st, " at ");
            crate::output::print_scaled(st, font_size);
            crate::output::print(st, "pt");
        }
    }

    crate::output::print_ln(st);
    crate::output::print_int(st, fmp - 7);
    crate::output::print(st, " words of font info for ");
    crate::output::print_int(st, font_ptr);
    if font_ptr != FONT_BASE + 1 {
        crate::output::print(st, " preloaded fonts");
    } else {
        crate::output::print(st, " preloaded font");
    }

    let hyph_count = st.hyph.hyph_count;
    dump_int(fmt_out, hyph_count)?;
    if st.hyph.hyph_next <= HYPH_PRIME {
        st.hyph.hyph_next = st.hyph.hyph_size;
    }
    dump_int(fmt_out, st.hyph.hyph_next)?;

    let hyph = &st.hyph;
    for k in 0..=hyph.hyph_size {
        let h = k as usize;
        if hyph.hyph_word[h] != StrNumber::NONE {
            dump_int(fmt_out, k + 65_536 * i32::from(hyph.hyph_link[h]))?;
            dump_int(fmt_out, hyph.hyph_word[h].0)?;
            dump_int(fmt_out, hyph.hyph_list[h])?;
        }
    }

    crate::output::print_ln(st);
    crate::output::print_int(st, hyph_count);
    if hyph_count != 1 {
        crate::output::print(st, " hyphenation exceptions");
    } else {
        crate::output::print(st, " hyphenation exception");
    }

    if st.hyph.trie_not_ready {
        st.hyph.init_trie()?;
    }

    let hyph = &st.hyph;
    let trie_max = hyph.trie_max;
    let trie_op_ptr = hyph.trie_op_ptr;
    dump_int(fmt_out, trie_max)?;
    dump_int(fmt_out, hyph.hyph_start)?;
    dump_slice(fmt_out, &hyph.trie_trl, 0, trie_max + 1)?;
    dump_slice(fmt_out, &hyph.trie_tro, 0, trie_max + 1)?;
    dump_slice(fmt_out, &hyph.trie_trc, 0, trie_max + 1)?;
    dump_int(fmt_out, hyph.max_hyph_char)?;
    dump_int(fmt_out, trie_op_ptr)?;
    dump_slice(fmt_out, &hyph.hyf_distance, 1, trie_op_ptr)?;
    dump_slice(fmt_out, &hyph.hyf_num, 1, trie_op_ptr)?;
    dump_slice(fmt_out, &hyph.hyf_next, 1, trie_op_ptr)?;

    crate::output::print_nl(st, "Hyphenation trie of length ");
    crate::output::print_int(st, trie_max);
    crate::output::print(st, " has ");
    crate::output::print_int(st, trie_op_ptr);
    if trie_op_ptr != 1 {
        crate::output::print(st, " ops");
    } else {
        crate::output::print(st, " op");
    }
    crate::output::print(st, " out of ");
    crate::output::print_int(st, TRIE_OP_SIZE);

    for k in (0..=BIGGEST_LANG as usize).rev() {
        let used = i32::from(st.hyph.trie_used[k]);
        if used > 0 {
            crate::output::print_nl(st, "  ");
            crate::output::print_int(st, used);
            crate::output::print(st, " for language ");
            crate::output::print_int(st, k as i32);
            dump_int(fmt_out, k as i32)?;
            dump_int(fmt_out, used)?;
        }
    }

    dump_int(fmt_out, FORMAT_FOOTER_MAGIC)?;
    st.eqtb.set_int_par(INT_PAR_TRACING_STATS, 0);
    output_close(fmt_out);
    Ok(())
}

pub fn load_fmt_file(st: &mut EngineState, dump_name: &str) -> Result<bool, Abort> {
    let loc = st.input.cur_input.loc;

    st.input.name_of_file = dump_name.as_bytes().to_vec();

    let fmt_in = input_open(
        st.input.name_of_file_bytes(),
        TTBC_FILE_FORMAT_FORMAT,
        false,
    );
    if fmt_in == INVALID_HANDLE {
        return Err(Abort::Internal("cannot open the format file"));
    }

    st.input.cur_input.loc = loc;

    read_fmt_file(st, fmt_in)
}

fn read_fmt_file(st: &mut EngineState, fmt_in: usize) -> Result<bool, Abort> {
    if st.config.in_initex_mode {
        st.fonts.font_info = Vec::new();
        st.strings.str_pool = Vec::new();
        st.strings.str_start = Vec::new();
        st.eqtb.data = Vec::new();
        st.eqtb.hash = Vec::new();
        st.mem.data = Vec::new();
    }

    expect_int(fmt_in, FORMAT_HEADER_MAGIC)?;
    if undump_int(fmt_in)? != crate::FORMAT_SERIAL as i32 {
        return Err(Abort::Internal("format file is of the wrong version"));
    }

    let eqtb = &mut st.eqtb;
    eqtb.hash_high = undump_in_range(fmt_in, 0, SUP_HASH_EXTRA)?;
    eqtb.hash_extra = eqtb.hash_extra.max(eqtb.hash_high);
    eqtb.eqtb_top = EQTB_SIZE + eqtb.hash_extra;
    eqtb.hash_top = if eqtb.hash_extra == 0 {
        UNDEFINED_CONTROL_SEQUENCE
    } else {
        eqtb.eqtb_top
    };
    eqtb.hash = crate::mem_types::zeroed_vec((1 + eqtb.hash_top - HASH_OFFSET) as usize);

    eqtb.data = crate::mem_types::zeroed_vec(eqtb.eqtb_top as usize + 1);
    eqtb.set_eq(UNDEFINED_CONTROL_SEQUENCE, UNDEFINED_CS, TEX_NULL, 0);
    let undefined_entry = eqtb.word(UNDEFINED_CONTROL_SEQUENCE);
    eqtb.data[(EQTB_SIZE + 1) as usize..].fill(undefined_entry);

    st.scanner.max_reg_num = 32_767;

    expect_int(fmt_in, MEM_TOP)?;

    st.input.cur_list.head = CONTRIB_HEAD;
    st.input.cur_list.tail = CONTRIB_HEAD;
    st.shipout.page_tail = PAGE_HEAD;
    st.mem.data = crate::mem_types::zeroed_vec(MEM_TOP as usize + 1);

    expect_int(fmt_in, EQTB_SIZE)?;
    expect_int(fmt_in, HASH_PRIME)?;
    expect_int(fmt_in, HYPH_PRIME)?;

    let strings = &mut st.strings;
    strings.pool_ptr = undump_size(
        fmt_in,
        0,
        SUP_POOL_SIZE - POOL_FREE,
        "must increase string_pool_size",
    )?;
    strings.pool_size = strings.pool_size.max(strings.pool_ptr + POOL_FREE);

    strings.str_ptr = undump_size(
        fmt_in,
        0,
        SUP_MAX_STRINGS - STRINGS_FREE,
        "must increase sup_strings",
    )?;
    strings.max_strings = strings.max_strings.max(strings.str_ptr + STRINGS_FREE);

    strings.str_start = vec![0; strings.max_strings as usize + 1];
    undump_checked(
        fmt_in,
        &mut strings.str_start,
        0,
        strings.str_ptr - TOO_BIG_CHAR + 1,
        0,
        strings.pool_ptr,
    )?;
    strings.str_pool = vec![0; strings.pool_size as usize + 1];
    undump_slice(fmt_in, &mut strings.str_pool, 0, strings.pool_ptr)?;

    strings.init_str_ptr = strings.str_ptr;
    strings.init_pool_ptr = strings.pool_ptr;

    let mem = &mut st.mem;
    mem.lo_mem_max = undump_in_range(fmt_in, 1019, MEM_TOP - HI_MEM_STAT_USAGE)?;
    mem.rover = undump_in_range(fmt_in, 20, mem.lo_mem_max)?;

    for root in &mut st.scanner.sa_root[INT_VAL as usize..=INTER_CHAR_VAL as usize] {
        root.0 = undump_in_range(fmt_in, MIN_HALFWORD, mem.lo_mem_max)?;
    }

    let mut p = 0;
    let mut q = mem.rover;
    loop {
        undump_slice(fmt_in, &mut mem.data, p, q + 2 - p)?;
        let block = crate::node::NodePtr::<crate::node::FreeBlock>::at(q);
        p = q + block.node_size(mem);
        let next = block.rlink(mem);
        if p > mem.lo_mem_max || (q >= next && next != mem.rover) {
            return Err(bad_fmt());
        }
        q = next;
        if q == mem.rover {
            break;
        }
    }

    undump_slice(fmt_in, &mut mem.data, p, mem.lo_mem_max + 1 - p)?;

    mem.hi_mem_min = undump_in_range(fmt_in, mem.lo_mem_max + 1, HI_MEM_STAT_MIN)?;
    mem.avail = undump_in_range(fmt_in, MIN_HALFWORD, MEM_TOP)?;
    mem.mem_end = MEM_TOP;
    undump_slice(
        fmt_in,
        &mut mem.data,
        mem.hi_mem_min,
        mem.mem_end + 1 - mem.hi_mem_min,
    )?;
    mem.var_used = undump_int(fmt_in)?;
    mem.dyn_used = undump_int(fmt_in)?;

    let eqtb = &mut st.eqtb;
    let mut k = ACTIVE_BASE;
    loop {
        let x = undump_int(fmt_in)?;
        if x < 1 || k + x > EQTB_SIZE + 1 {
            return Err(bad_fmt());
        }
        undump_slice(fmt_in, &mut eqtb.data, k, x)?;
        k += x;

        let x = undump_int(fmt_in)?;
        if x < 0 || k + x > EQTB_SIZE + 1 {
            return Err(bad_fmt());
        }
        let repeated = eqtb.data[(k - 1) as usize];
        eqtb.data[k as usize..(k + x) as usize].fill(repeated);
        k += x;
        if k > EQTB_SIZE {
            break;
        }
    }

    undump_slice(fmt_in, &mut eqtb.data, EQTB_SIZE + 1, eqtb.hash_high)?;

    st.input.par_loc = undump_in_range(fmt_in, HASH_BASE, eqtb.hash_top)?;
    st.input.par_token = CS_TOKEN_FLAG + st.input.par_loc;
    st.shipout.write_loc = undump_in_range(fmt_in, HASH_BASE, eqtb.hash_top)?;

    undump_slice(fmt_in, &mut eqtb.prim, 0, PRIM_SIZE + 1)?;

    eqtb.hash_used = undump_in_range(fmt_in, HASH_BASE, FROZEN_CONTROL_SEQUENCE)?;

    let mut p = HASH_BASE - 1;
    loop {
        p = undump_in_range(fmt_in, p + 1, eqtb.hash_used)?;
        undump_slice(fmt_in, &mut eqtb.hash, p - HASH_OFFSET, 1)?;
        if p == eqtb.hash_used {
            break;
        }
    }

    undump_slice(
        fmt_in,
        &mut eqtb.hash,
        eqtb.hash_used + 1 - HASH_OFFSET,
        (UNDEFINED_CONTROL_SEQUENCE - 1) - eqtb.hash_used,
    )?;
    undump_slice(
        fmt_in,
        &mut eqtb.hash,
        EQTB_SIZE + 1 - HASH_OFFSET,
        eqtb.hash_high,
    )?;

    eqtb.cs_count = undump_int(fmt_in)?;

    let fonts = &mut st.fonts;
    fonts.fmem_ptr = undump_size(fmt_in, 7, SUP_FONT_MEM_SIZE, "must increase font_mem_size")?;
    fonts.font_mem_size = fonts.font_mem_size.max(fonts.fmem_ptr);

    fonts.font_info = crate::mem_types::zeroed_vec(fonts.font_mem_size as usize + 1);
    undump_slice(fmt_in, &mut fonts.font_info, 0, fonts.fmem_ptr)?;

    fonts.font_ptr = FontId(undump_size(
        fmt_in,
        FONT_BASE,
        FONT_BASE + MAX_FONT_MAX,
        "must increase font_max",
    )?);

    fonts.alloc_font_arrays();

    let n = fonts.font_ptr.0 + 1;
    let max_str = st.strings.str_ptr;
    undump_slice(fmt_in, fonts.font_check.as_mut_slice(), FONT_BASE, n)?;
    undump_slice(fmt_in, fonts.font_size.as_mut_slice(), FONT_BASE, n)?;
    undump_slice(fmt_in, fonts.font_dsize.as_mut_slice(), FONT_BASE, n)?;
    undump_checked(
        fmt_in,
        fonts.font_params.as_mut_slice(),
        FONT_BASE,
        n,
        MIN_HALFWORD,
        MAX_HALFWORD,
    )?;
    undump_slice(fmt_in, fonts.hyphen_char.as_mut_slice(), FONT_BASE, n)?;
    undump_slice(fmt_in, fonts.skew_char.as_mut_slice(), FONT_BASE, n)?;
    undump_checked(
        fmt_in,
        fonts.font_name.as_mut_slice(),
        FONT_BASE,
        n,
        StrNumber(i32::MIN),
        StrNumber(max_str),
    )?;
    undump_checked(
        fmt_in,
        fonts.font_area.as_mut_slice(),
        FONT_BASE,
        n,
        StrNumber(i32::MIN),
        StrNumber(max_str),
    )?;
    undump_slice(fmt_in, fonts.font_bc.as_mut_slice(), FONT_BASE, n)?;
    undump_slice(fmt_in, fonts.font_ec.as_mut_slice(), FONT_BASE, n)?;
    undump_slice(fmt_in, fonts.char_base.as_mut_slice(), FONT_BASE, n)?;
    undump_slice(fmt_in, fonts.width_base.as_mut_slice(), FONT_BASE, n)?;
    undump_slice(fmt_in, fonts.height_base.as_mut_slice(), FONT_BASE, n)?;
    undump_slice(fmt_in, fonts.depth_base.as_mut_slice(), FONT_BASE, n)?;
    undump_slice(fmt_in, fonts.italic_base.as_mut_slice(), FONT_BASE, n)?;
    undump_slice(fmt_in, fonts.lig_kern_base.as_mut_slice(), FONT_BASE, n)?;
    undump_slice(fmt_in, fonts.kern_base.as_mut_slice(), FONT_BASE, n)?;
    undump_slice(fmt_in, fonts.exten_base.as_mut_slice(), FONT_BASE, n)?;
    undump_slice(fmt_in, fonts.param_base.as_mut_slice(), FONT_BASE, n)?;
    undump_checked(
        fmt_in,
        fonts.font_glue.as_mut_slice(),
        FONT_BASE,
        n,
        MIN_HALFWORD,
        st.mem.lo_mem_max,
    )?;
    let max_font_index = fonts.fmem_ptr - 1;
    undump_checked(
        fmt_in,
        fonts.bchar_label.as_mut_slice(),
        FONT_BASE,
        n,
        0,
        max_font_index,
    )?;
    undump_checked(
        fmt_in,
        fonts.font_bchar.as_mut_slice(),
        FONT_BASE,
        n,
        0,
        TOO_BIG_CHAR,
    )?;
    undump_checked(
        fmt_in,
        fonts.font_false_bchar.as_mut_slice(),
        FONT_BASE,
        n,
        0,
        TOO_BIG_CHAR,
    )?;

    let hyph = &mut st.hyph;
    hyph.hyph_count = undump_size(fmt_in, 0, hyph.hyph_size, "must increase hyph_size")?;
    hyph.hyph_next = undump_size(
        fmt_in,
        HYPH_PRIME,
        hyph.hyph_size,
        "must increase hyph_size",
    )?;

    let mut j = 0;
    for _ in 1..=hyph.hyph_count {
        j = undump_int(fmt_in)?;
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
        hyph.hyph_link[j as usize] = hyph.hyph_next as u16;
        hyph.hyph_word[j as usize] = StrNumber(undump_in_range(fmt_in, 0, st.strings.str_ptr)?);
        hyph.hyph_list[j as usize] = undump_in_range(fmt_in, MIN_HALFWORD, MAX_HALFWORD)?;
    }

    hyph.hyph_next = (j + 1).max(HYPH_PRIME);
    if hyph.hyph_next >= hyph.hyph_size {
        hyph.hyph_next = HYPH_PRIME;
    } else if hyph.hyph_next >= HYPH_PRIME {
        hyph.hyph_next += 1;
    }

    let trie_max = undump_size(fmt_in, 0, hyph.trie_size, "must increase trie_size")?;
    hyph.trie_max = trie_max;
    hyph.hyph_start = undump_in_range(fmt_in, 0, trie_max)?;

    let trie_len = trie_max as usize + 1;
    if hyph.trie_trl.is_empty() {
        hyph.trie_trl = vec![0; trie_len];
    }
    undump_slice(fmt_in, &mut hyph.trie_trl, 0, trie_max + 1)?;
    if hyph.trie_tro.is_empty() {
        hyph.trie_tro = vec![0; trie_len];
    }
    undump_slice(fmt_in, &mut hyph.trie_tro, 0, trie_max + 1)?;
    if hyph.trie_trc.is_empty() {
        hyph.trie_trc = vec![0; trie_len];
    }
    undump_slice(fmt_in, &mut hyph.trie_trc, 0, trie_max + 1)?;

    hyph.max_hyph_char = undump_int(fmt_in)?;

    let mut j = undump_size(fmt_in, 0, TRIE_OP_SIZE, "must increase TRIE_OP_SIZE")?;
    hyph.trie_op_ptr = j;

    undump_slice(fmt_in, &mut hyph.hyf_distance, 1, j)?;
    undump_slice(fmt_in, &mut hyph.hyf_num, 1, j)?;
    undump_checked(fmt_in, &mut hyph.hyf_next, 1, j, 0, MAX_TRIE_OP)?;

    hyph.trie_used[..=BIGGEST_LANG as usize].fill(0);

    let mut k = BIGGEST_LANG + 1;
    while j > 0 {
        k = undump_in_range(fmt_in, 0, k - 1)?;
        let used = undump_in_range(fmt_in, 1, j)?;
        hyph.trie_used[k as usize] = used as u16;
        j -= used;
        hyph.op_start[k as usize] = j;
    }

    hyph.trie_not_ready = false;

    expect_int(fmt_in, FORMAT_FOOTER_MAGIC)?;

    if crate::io::input_close(fmt_in) {
        return Err(Abort::Internal(
            "serious internal error closing the format file",
        ));
    }
    Ok(true)
}
