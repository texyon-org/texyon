//! Format-file dump and load (`\dump` and the preloaded format).
//!
//! Every multi-byte item is stored big-endian.

use std::ffi::CStr;

use crate::bridge::{
    ttstub_input_open, ttstub_input_read, ttstub_output_close, ttstub_output_flush,
    ttstub_output_open, ttstub_output_write, TTBC_FILE_FORMAT_FORMAT,
};
use crate::cmd::UNDEFINED_CS;
use crate::consts::{
    ACTIVE_BASE, CS_TOKEN_FLAG, EMPTY_STRING, EQTB_SIZE, FONT_BASE, FONT_ID_BASE,
    FROZEN_CONTROL_SEQUENCE, HASH_BASE, HISTORY_FATAL_ERROR, INT_BASE, INT_PAR_DAY, INT_PAR_MONTH,
    INT_PAR_TRACING_STATS, INT_PAR_YEAR, INVALID_HANDLE, MAX_FONT_MAX, MAX_HALFWORD, MEM_TOP,
    MIN_HALFWORD, PRIM_SIZE, SELECTOR_LOG_ONLY, SELECTOR_NEW_STRING, SELECTOR_TERM_AND_LOG,
    TEX_NULL, TOO_BIG_CHAR, UNDEFINED_CONTROL_SEQUENCE,
};
use crate::engine::{
    alloc_font_arrays, set_eq, sort_avail_in, POOL_FREE, REGISTER_HELP, STRINGS_FREE,
    TEX_FORMAT_DEFAULT,
};
use crate::errors::EngineAbort;
use crate::fonts::{AAT_FONT_FLAG, OTGR_FONT_FLAG};
use crate::hash::{HASH_OFFSET, HASH_PRIME};
use crate::hyphenation::{BIGGEST_LANG, HYPH_PRIME, MAX_TRIE_OP, TRIE_OP_SIZE};
use crate::io::name_of_file_ptr;
use crate::mem::{CONTRIB_HEAD, HI_MEM_STAT_MIN, HI_MEM_STAT_USAGE, PAGE_HEAD};
use crate::scan::{INTER_CHAR_VAL, INT_VAL};
use crate::state::{EngineState, EqtbState, InteractionMode};

const FORMAT_HEADER_MAGIC: i32 = 0x5454_4E43;
const FORMAT_FOOTER_MAGIC: i32 = 0x0000_029A;
const SUP_MAX_STRINGS: i32 = 2_097_151;
const SUP_FONT_MEM_SIZE: i32 = 147_483_647;
const SUP_POOL_SIZE: i32 = 40_000_000;
const SUP_HASH_EXTRA: i32 = 2_097_151;

fn bad_fmt() -> EngineAbort {
    EngineAbort::TtAbort(c"fatal format file error".as_ptr())
}

/// Writes `v[start..start + n]`, byte-swapping each item to big-endian.
unsafe fn dump_slice<T>(fmt_out: usize, v: &[T], start: i32, n: i32) -> Result<(), EngineAbort> {
    if n <= 0 {
        return Ok(());
    }
    let items = &v[start as usize..(start + n) as usize];
    let item_size = std::mem::size_of::<T>();
    let byte_len = std::mem::size_of_val(items);
    let ptr = items.as_ptr() as *const u8;
    let written = if cfg!(target_endian = "little") && item_size > 1 {
        let mut bytes = std::slice::from_raw_parts(ptr, byte_len).to_vec();
        for item in bytes.chunks_exact_mut(item_size) {
            item.reverse();
        }
        ttstub_output_write(fmt_out, bytes.as_ptr() as *const libc::c_char, byte_len)
    } else {
        ttstub_output_write(fmt_out, ptr as *const libc::c_char, byte_len)
    };

    if written != byte_len {
        return Err(EngineAbort::TtAbort(
            c"could not write format file data".as_ptr(),
        ));
    }
    Ok(())
}

unsafe fn dump_one<T: Copy>(fmt_out: usize, value: T) -> Result<(), EngineAbort> {
    dump_slice(fmt_out, &[value], 0, 1)
}

unsafe fn dump_int(fmt_out: usize, value: i32) -> Result<(), EngineAbort> {
    dump_one(fmt_out, value)
}

/// Reads `n` items into `v[start..]`, converting each from big-endian.
unsafe fn undump_slice<T>(
    fmt_in: usize,
    v: &mut [T],
    start: i32,
    n: i32,
) -> Result<(), EngineAbort> {
    if n <= 0 {
        return Ok(());
    }
    let items = &mut v[start as usize..(start + n) as usize];
    let item_size = std::mem::size_of::<T>();
    let byte_len = std::mem::size_of_val(items);
    let ptr = items.as_mut_ptr() as *mut u8;
    let read = ttstub_input_read(fmt_in, ptr, byte_len);
    if read < 0 || read as usize != byte_len {
        return Err(EngineAbort::TtAbort(
            c"could not read format file data".as_ptr(),
        ));
    }

    if cfg!(target_endian = "little") && item_size > 1 {
        let bytes = std::slice::from_raw_parts_mut(ptr, byte_len);
        for item in bytes.chunks_exact_mut(item_size) {
            item.reverse();
        }
    }
    Ok(())
}

unsafe fn undump_int(fmt_in: usize) -> Result<i32, EngineAbort> {
    let mut value = [0i32];
    undump_slice(fmt_in, &mut value, 0, 1)?;
    Ok(value[0])
}

/// Reads an int that must lie in `low..=high`.
unsafe fn undump_in_range(fmt_in: usize, low: i32, high: i32) -> Result<i32, EngineAbort> {
    let x = undump_int(fmt_in)?;
    if x < low || x > high {
        return Err(bad_fmt());
    }
    Ok(x)
}

/// Reads a size that must be at least `low` and fit the engine's `high` limit.
unsafe fn undump_size(
    fmt_in: usize,
    low: i32,
    high: i32,
    too_big: &'static CStr,
) -> Result<i32, EngineAbort> {
    let x = undump_int(fmt_in)?;
    if x < low {
        return Err(bad_fmt());
    }
    if x > high {
        return Err(EngineAbort::TtAbort(too_big.as_ptr()));
    }
    Ok(x)
}

unsafe fn expect_int(fmt_in: usize, expected: i32) -> Result<(), EngineAbort> {
    if undump_int(fmt_in)? != expected {
        return Err(bad_fmt());
    }
    Ok(())
}

/// Reads `n` items into `v[start..]` that must each lie in `low..=high`.
unsafe fn undump_checked<T: Copy + PartialOrd>(
    fmt_in: usize,
    v: &mut [T],
    start: i32,
    n: i32,
    low: T,
    high: T,
) -> Result<(), EngineAbort> {
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
unsafe fn dump_eqtb_runs(
    fmt_out: usize,
    eqtb: &EqtbState,
    mut k: i32,
    last: i32,
    same: impl Fn(&EqtbState, i32, i32) -> bool,
) -> Result<i32, EngineAbort> {
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

pub(crate) unsafe fn store_fmt_file_in(st: &mut EngineState) -> Result<(), EngineAbort> {
    if st.save_stack.save_ptr != 0 {
        crate::output::error_here_with_diagnostic_in(st, c"You can't dump inside a group".as_ptr());
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 1;
        st.errors.help_line[0] = c"`{...\\dump}' is a no-no.".as_ptr();

        if st.errors.interaction == InteractionMode::ErrorStop {
            st.errors.interaction = InteractionMode::Scroll;
        }
        if st.errors.log_opened {
            let outcome = crate::errors::error_in(st);
            if !matches!(outcome, crate::errors::ErrorOutcome::Continue) {
                return Err(outcome.into_engine_abort());
            }
        }

        st.errors.history = HISTORY_FATAL_ERROR;
        let _ = crate::io::close_files_and_terminate_in(st);
        ttstub_output_flush(st.shipout.rust_stdout);
        return Err(EngineAbort::TtAbort(c"\\dump inside a group".as_ptr()));
    }

    st.shipout.selector = SELECTOR_NEW_STRING;
    crate::output::print_cstr_in(st, c" (preloaded format=".as_ptr());
    crate::output::print_in(st, st.strings.job_name);
    crate::output::print_char_in(st, b' ' as i32);
    crate::output::print_int_in(st, st.eqtb.b32s1(INT_BASE + INT_PAR_YEAR));
    crate::output::print_char_in(st, b'.' as i32);
    crate::output::print_int_in(st, st.eqtb.b32s1(INT_BASE + INT_PAR_MONTH));
    crate::output::print_char_in(st, b'.' as i32);
    crate::output::print_int_in(st, st.eqtb.b32s1(INT_BASE + INT_PAR_DAY));
    crate::output::print_char_in(st, b')' as i32);

    st.shipout.selector = if st.errors.interaction == InteractionMode::Batch {
        SELECTOR_LOG_ONLY
    } else {
        SELECTOR_TERM_AND_LOG
    };

    if st.strings.pool_ptr + 1 > st.strings.pool_size {
        return Err(EngineAbort::Overflow {
            what: c"pool size".as_ptr(),
            n: st.strings.pool_size - st.strings.init_pool_ptr,
        });
    }

    st.strings.format_ident = match crate::string_pool::make_string(&mut st.strings) {
        Ok(value) => value,
        Err(crate::scan::NodeRecycleAbort::Overflow) => {
            return Err(EngineAbort::Overflow {
                what: c"number of strings".as_ptr(),
                n: st.strings.max_strings - st.strings.init_str_ptr,
            })
        }
        Err(e) => return Err(e.into_engine_abort()),
    };
    crate::io::pack_job_name_in(st, c".fmt".as_ptr()).map_err(|e| e.into_engine_abort())?;

    let fmt_out = ttstub_output_open(name_of_file_ptr(st), 0);
    if fmt_out == INVALID_HANDLE {
        return Err(EngineAbort::TtAbort(
            c"cannot open format output file".as_ptr(),
        ));
    }

    crate::output::print_nl_cstr_in(st, c"Beginning to dump on file ".as_ptr());
    let name_string = crate::io::make_name_string_in(st).map_err(|e| e.into_engine_abort())?;
    crate::output::print_in(st, name_string);

    st.strings.str_ptr -= 1;
    st.strings.pool_ptr = st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];

    crate::output::print_nl_cstr_in(st, c"".as_ptr());
    crate::output::print_in(st, st.strings.format_ident);

    dump_int(fmt_out, FORMAT_HEADER_MAGIC)?;
    dump_int(fmt_out, crate::FORMAT_SERIAL as i32)?;
    dump_int(fmt_out, st.eqtb.hash_high)?;

    while st.input.pseudo_files.0 != TEX_NULL {
        crate::input::pseudo_close(&mut st.mem, &mut st.input);
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

    crate::output::print_ln_in(st);
    crate::output::print_int_in(st, str_ptr);
    crate::output::print_cstr_in(st, c" strings of total length ".as_ptr());
    crate::output::print_int_in(st, pool_ptr);

    sort_avail_in(&mut st.mem);
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
        p = q + mem.b32s0(q);
        q = mem.b32s1(q + 1);
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

    p = avail;
    while p != TEX_NULL {
        dyn_used -= 1;
        p = mem.b32s1(p);
    }

    st.mem.var_used = var_used;
    st.mem.dyn_used = dyn_used;
    dump_int(fmt_out, var_used)?;
    dump_int(fmt_out, dyn_used)?;

    crate::output::print_ln_in(st);
    crate::output::print_int_in(st, x);
    crate::output::print_cstr_in(st, c" memory locations dumped; current usage is ".as_ptr());
    crate::output::print_int_in(st, var_used);
    crate::output::print_char_in(st, b'&' as i32);
    crate::output::print_int_in(st, dyn_used);

    let k = dump_eqtb_runs(
        fmt_out,
        &st.eqtb,
        ACTIVE_BASE,
        INT_BASE - 1,
        |eqtb, a, b| {
            eqtb.b32s1(a) == eqtb.b32s1(b)
                && eqtb.b16s1(a) == eqtb.b16s1(b)
                && eqtb.b16s0(a) == eqtb.b16s0(b)
        },
    )?;
    dump_eqtb_runs(fmt_out, &st.eqtb, k, EQTB_SIZE, |eqtb, a, b| {
        eqtb.b32s1(a) == eqtb.b32s1(b)
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

    crate::output::print_ln_in(st);
    crate::output::print_int_in(st, cs_count);
    crate::output::print_cstr_in(st, c" multiletter control sequences".as_ptr());

    let fmp = st.fonts.fmem_ptr;
    let font_ptr = st.fonts.font_ptr;
    let n = font_ptr + 1;
    let fonts = &st.fonts;
    dump_int(fmt_out, fmp)?;
    dump_slice(fmt_out, &fonts.font_info, 0, fmp)?;
    dump_int(fmt_out, font_ptr)?;
    dump_slice(fmt_out, &fonts.font_check, FONT_BASE, n)?;
    dump_slice(fmt_out, &fonts.font_size, FONT_BASE, n)?;
    dump_slice(fmt_out, &fonts.font_dsize, FONT_BASE, n)?;
    dump_slice(fmt_out, &fonts.font_params, FONT_BASE, n)?;
    dump_slice(fmt_out, &fonts.hyphen_char, FONT_BASE, n)?;
    dump_slice(fmt_out, &fonts.skew_char, FONT_BASE, n)?;
    dump_slice(fmt_out, &fonts.font_name, FONT_BASE, n)?;
    dump_slice(fmt_out, &fonts.font_area, FONT_BASE, n)?;
    dump_slice(fmt_out, &fonts.font_bc, FONT_BASE, n)?;
    dump_slice(fmt_out, &fonts.font_ec, FONT_BASE, n)?;
    dump_slice(fmt_out, &fonts.char_base, FONT_BASE, n)?;
    dump_slice(fmt_out, &fonts.width_base, FONT_BASE, n)?;
    dump_slice(fmt_out, &fonts.height_base, FONT_BASE, n)?;
    dump_slice(fmt_out, &fonts.depth_base, FONT_BASE, n)?;
    dump_slice(fmt_out, &fonts.italic_base, FONT_BASE, n)?;
    dump_slice(fmt_out, &fonts.lig_kern_base, FONT_BASE, n)?;
    dump_slice(fmt_out, &fonts.kern_base, FONT_BASE, n)?;
    dump_slice(fmt_out, &fonts.exten_base, FONT_BASE, n)?;
    dump_slice(fmt_out, &fonts.param_base, FONT_BASE, n)?;
    dump_slice(fmt_out, &fonts.font_glue, FONT_BASE, n)?;
    dump_slice(fmt_out, &fonts.bchar_label, FONT_BASE, n)?;
    dump_slice(fmt_out, &fonts.font_bchar, FONT_BASE, n)?;
    dump_slice(fmt_out, &fonts.font_false_bchar, FONT_BASE, n)?;

    for k in FONT_BASE..=font_ptr {
        let f = k as usize;
        crate::output::print_nl_cstr_in(st, c"\\font".as_ptr());
        crate::output::print_esc_in(st, st.eqtb.hash_s1(FONT_ID_BASE + k));
        crate::output::print_char_in(st, b'=' as i32);

        let font_name = st.fonts.font_name[f];
        let font_area = st.fonts.font_area[f];
        if font_area == AAT_FONT_FLAG
            || font_area == OTGR_FONT_FLAG
            || st.fonts.font_mapping[f].is_some()
        {
            crate::output::print_file_name_in(st, font_name, EMPTY_STRING, EMPTY_STRING);
            crate::output::error_here_with_diagnostic_in(
                st,
                c"Can't \\dump a format with native fonts or font-mappings".as_ptr(),
            );
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.errors.help_ptr = 3;
            st.errors.help_line[2] = c"You really, really don't want to do this.".as_ptr();
            st.errors.help_line[1] = c"It won't work, and only confuses me.".as_ptr();
            st.errors.help_line[0] =
                c"(Load them at runtime, not as part of the format file.)".as_ptr();
            let outcome = crate::errors::error_in(st);
            if !matches!(outcome, crate::errors::ErrorOutcome::Continue) {
                return Err(outcome.into_engine_abort());
            }
        } else {
            crate::output::print_file_name_in(st, font_name, font_area, EMPTY_STRING);
        }

        let font_size = st.fonts.font_size[f];
        if font_size != st.fonts.font_dsize[f] {
            crate::output::print_cstr_in(st, c" at ".as_ptr());
            crate::output::print_scaled_in(st, font_size);
            crate::output::print_cstr_in(st, c"pt".as_ptr());
        }
    }

    crate::output::print_ln_in(st);
    crate::output::print_int_in(st, fmp - 7);
    crate::output::print_cstr_in(st, c" words of font info for ".as_ptr());
    crate::output::print_int_in(st, font_ptr);
    if font_ptr != FONT_BASE + 1 {
        crate::output::print_cstr_in(st, c" preloaded fonts".as_ptr());
    } else {
        crate::output::print_cstr_in(st, c" preloaded font".as_ptr());
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
        if hyph.hyph_word[h] != 0 {
            dump_int(fmt_out, k + 65_536 * hyph.hyph_link[h] as i32)?;
            dump_int(fmt_out, hyph.hyph_word[h])?;
            dump_int(fmt_out, hyph.hyph_list[h])?;
        }
    }

    crate::output::print_ln_in(st);
    crate::output::print_int_in(st, hyph_count);
    if hyph_count != 1 {
        crate::output::print_cstr_in(st, c" hyphenation exceptions".as_ptr());
    } else {
        crate::output::print_cstr_in(st, c" hyphenation exception".as_ptr());
    }

    if st.hyph.trie_not_ready {
        crate::hyphenation::init_trie_in(&mut st.hyph).map_err(|e| e.into_engine_abort())?;
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

    crate::output::print_nl_cstr_in(st, c"Hyphenation trie of length ".as_ptr());
    crate::output::print_int_in(st, trie_max);
    crate::output::print_cstr_in(st, c" has ".as_ptr());
    crate::output::print_int_in(st, trie_op_ptr);
    if trie_op_ptr != 1 {
        crate::output::print_cstr_in(st, c" ops".as_ptr());
    } else {
        crate::output::print_cstr_in(st, c" op".as_ptr());
    }
    crate::output::print_cstr_in(st, c" out of ".as_ptr());
    crate::output::print_int_in(st, TRIE_OP_SIZE);

    for k in (0..=BIGGEST_LANG as usize).rev() {
        let used = st.hyph.trie_used[k] as i32;
        if used > 0 {
            crate::output::print_nl_cstr_in(st, c"  ".as_ptr());
            crate::output::print_int_in(st, used);
            crate::output::print_cstr_in(st, c" for language ".as_ptr());
            crate::output::print_int_in(st, k as i32);
            dump_int(fmt_out, k as i32)?;
            dump_int(fmt_out, used)?;
        }
    }

    dump_int(fmt_out, FORMAT_FOOTER_MAGIC)?;
    st.eqtb.set_b32s1(INT_BASE + INT_PAR_TRACING_STATS, 0);
    ttstub_output_close(fmt_out);
    Ok(())
}

pub unsafe fn load_fmt_file(st: &mut EngineState) -> Result<bool, EngineAbort> {
    let loc = st.input.cur_input.loc;

    let len = libc::strlen(TEX_FORMAT_DEFAULT);
    st.input.name_of_file.resize(len + 1, 0);
    libc::strcpy(st.input.name_of_file.as_mut_ptr(), TEX_FORMAT_DEFAULT);
    st.input.name_length = len as i32;

    let fmt_in = ttstub_input_open(name_of_file_ptr(st), TTBC_FILE_FORMAT_FORMAT, 0);
    if fmt_in == INVALID_HANDLE {
        return Err(EngineAbort::TtAbort(
            c"cannot open the format file".as_ptr(),
        ));
    }

    st.input.cur_input.loc = loc;

    load_fmt_file_in(st, fmt_in)
}

unsafe fn load_fmt_file_in(st: &mut EngineState, fmt_in: usize) -> Result<bool, EngineAbort> {
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
        return Err(EngineAbort::TtAbort(
            c"format file is of the wrong version".as_ptr(),
        ));
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
    set_eq(eqtb, UNDEFINED_CONTROL_SEQUENCE, UNDEFINED_CS, TEX_NULL, 0);
    let undefined_entry = eqtb.word(UNDEFINED_CONTROL_SEQUENCE);
    eqtb.data[(EQTB_SIZE + 1) as usize..].fill(undefined_entry);

    st.scanner.max_reg_num = 32_767;
    st.scanner.max_reg_help_line = REGISTER_HELP.as_ptr();

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
        c"must increase string_pool_size",
    )?;
    strings.pool_size = strings.pool_size.max(strings.pool_ptr + POOL_FREE);

    strings.str_ptr = undump_size(
        fmt_in,
        0,
        SUP_MAX_STRINGS - STRINGS_FREE,
        c"must increase sup_strings",
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
        p = q + mem.b32s0(q);
        let next = mem.b32s1(q + 1);
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
    fonts.fmem_ptr = undump_size(fmt_in, 7, SUP_FONT_MEM_SIZE, c"must increase font_mem_size")?;
    fonts.font_mem_size = fonts.font_mem_size.max(fonts.fmem_ptr);

    fonts.font_info = crate::mem_types::zeroed_vec(fonts.font_mem_size as usize + 1);
    undump_slice(fmt_in, &mut fonts.font_info, 0, fonts.fmem_ptr)?;

    fonts.font_ptr = undump_size(
        fmt_in,
        FONT_BASE,
        FONT_BASE + MAX_FONT_MAX,
        c"must increase font_max",
    )?;

    alloc_font_arrays(fonts);

    let n = fonts.font_ptr + 1;
    let max_str = st.strings.str_ptr;
    undump_slice(fmt_in, &mut fonts.font_check, FONT_BASE, n)?;
    undump_slice(fmt_in, &mut fonts.font_size, FONT_BASE, n)?;
    undump_slice(fmt_in, &mut fonts.font_dsize, FONT_BASE, n)?;
    undump_checked(
        fmt_in,
        &mut fonts.font_params,
        FONT_BASE,
        n,
        MIN_HALFWORD,
        MAX_HALFWORD,
    )?;
    undump_slice(fmt_in, &mut fonts.hyphen_char, FONT_BASE, n)?;
    undump_slice(fmt_in, &mut fonts.skew_char, FONT_BASE, n)?;
    undump_checked(
        fmt_in,
        &mut fonts.font_name,
        FONT_BASE,
        n,
        i32::MIN,
        max_str,
    )?;
    undump_checked(
        fmt_in,
        &mut fonts.font_area,
        FONT_BASE,
        n,
        i32::MIN,
        max_str,
    )?;
    undump_slice(fmt_in, &mut fonts.font_bc, FONT_BASE, n)?;
    undump_slice(fmt_in, &mut fonts.font_ec, FONT_BASE, n)?;
    undump_slice(fmt_in, &mut fonts.char_base, FONT_BASE, n)?;
    undump_slice(fmt_in, &mut fonts.width_base, FONT_BASE, n)?;
    undump_slice(fmt_in, &mut fonts.height_base, FONT_BASE, n)?;
    undump_slice(fmt_in, &mut fonts.depth_base, FONT_BASE, n)?;
    undump_slice(fmt_in, &mut fonts.italic_base, FONT_BASE, n)?;
    undump_slice(fmt_in, &mut fonts.lig_kern_base, FONT_BASE, n)?;
    undump_slice(fmt_in, &mut fonts.kern_base, FONT_BASE, n)?;
    undump_slice(fmt_in, &mut fonts.exten_base, FONT_BASE, n)?;
    undump_slice(fmt_in, &mut fonts.param_base, FONT_BASE, n)?;
    undump_checked(
        fmt_in,
        &mut fonts.font_glue,
        FONT_BASE,
        n,
        MIN_HALFWORD,
        st.mem.lo_mem_max,
    )?;
    let max_font_index = fonts.fmem_ptr - 1;
    undump_checked(
        fmt_in,
        &mut fonts.bchar_label,
        FONT_BASE,
        n,
        0,
        max_font_index,
    )?;
    undump_checked(fmt_in, &mut fonts.font_bchar, FONT_BASE, n, 0, TOO_BIG_CHAR)?;
    undump_checked(
        fmt_in,
        &mut fonts.font_false_bchar,
        FONT_BASE,
        n,
        0,
        TOO_BIG_CHAR,
    )?;

    let hyph = &mut st.hyph;
    hyph.hyph_count = undump_size(fmt_in, 0, hyph.hyph_size, c"must increase hyph_size")?;
    hyph.hyph_next = undump_size(
        fmt_in,
        HYPH_PRIME,
        hyph.hyph_size,
        c"must increase hyph_size",
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
        hyph.hyph_word[j as usize] = undump_in_range(fmt_in, 0, st.strings.str_ptr)?;
        hyph.hyph_list[j as usize] = undump_in_range(fmt_in, MIN_HALFWORD, MAX_HALFWORD)?;
    }

    hyph.hyph_next = (j + 1).max(HYPH_PRIME);
    if hyph.hyph_next >= hyph.hyph_size {
        hyph.hyph_next = HYPH_PRIME;
    } else if hyph.hyph_next >= HYPH_PRIME {
        hyph.hyph_next += 1;
    }

    let trie_max = undump_size(fmt_in, 0, hyph.trie_size, c"must increase trie_size")?;
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

    let mut j = undump_size(fmt_in, 0, TRIE_OP_SIZE, c"must increase TRIE_OP_SIZE")?;
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
        return Err(EngineAbort::TtAbort(
            c"serious internal error closing the format file".as_ptr(),
        ));
    }
    Ok(true)
}
