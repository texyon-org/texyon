//! Format-file dump and load (the `\dump` / preloaded-format machinery),
//! translated from the format-I/O part of xetex-ini.c.

use crate::consts::{
    ACTIVE_BASE, CONTRIB_HEAD, CS_TOKEN_FLAG, EQTB_SIZE, FONT_BASE, FROZEN_CONTROL_SEQUENCE,
    HISTORY_FATAL_ERROR, INT_PAR_DAY, INT_PAR_MONTH, INT_PAR_YEAR, INVALID_HANDLE, MAX_FONT_MAX,
    MAX_HALFWORD, MEM_TOP, MIN_HALFWORD, PC_INT_BASE, SELECTOR_LOG_ONLY, SELECTOR_NEW_STRING,
    SELECTOR_TERM_AND_LOG, TOO_BIG_CHAR, UNDEFINED_CONTROL_SEQUENCE,
};
use crate::engine::{sort_avail_in, TRE_MAX_STRINGS, TRE_POOL_SIZE};
use crate::state::InteractionMode;
use crate::hyphenation::{BIGGEST_LANG, MAX_TRIE_OP, TRIE_OP_SIZE};

extern "C" {
    fn ttstub_output_open(path: *const libc::c_char, is_gz: libc::c_int) -> usize;
    fn ttstub_output_write(
        handle: usize,
        data: *const libc::c_char,
        len: libc::size_t,
    ) -> libc::size_t;
    fn ttstub_output_close(handle: usize) -> libc::c_int;
    fn ttstub_output_flush(handle: usize) -> libc::c_int;
    fn ttstub_input_open(
        path: *const libc::c_char,
        format: libc::c_int,
        is_gz: libc::c_int,
    ) -> usize;
    fn ttstub_input_read(handle: usize, data: *mut u8, len: libc::size_t) -> libc::ssize_t;

    #[link_name = "TEX_format_default"]
    static mut tre_TEX_format_default: *mut libc::c_char;
    #[link_name = "pool_free"]
    static mut tre_pool_free: i32;
    #[link_name = "strings_free"]
    static mut tre_strings_free: i32;
}

const PAGE_HEAD: i32 = MEM_TOP - 2;
const PRE_ADJUST_HEAD: i32 = MEM_TOP - 14;
const HI_MEM_STAT_USAGE: i32 = 15;
const EMPTY_STRING: i32 = 65_536;
const INT_PAR_TRACING_STATS: i32 = 31;
const FORMAT_HEADER_MAGIC: i32 = 0x5454_4E43;
const FORMAT_FOOTER_MAGIC: i32 = 0x0000_029A;
const FORMAT_SERIAL: i32 = 33;
const FORMAT_HASH_PRIME: i32 = 8_501;
const FORMAT_HYPH_PRIME: i32 = 607;
const FORMAT_HASH_BASE: i32 = 2_228_226;
const INT_VAL: i32 = 0;
const FONT_ID_BASE: i32 = 2_245_338;
const PRIM_SIZE: i32 = 2_100;
const FORMAT_INT_BASE: i32 = 7_826_729;
const TTBC_FILE_FORMAT_FORMAT: libc::c_int = 10;
const SUP_MAX_STRINGS: i32 = 2_097_151;
const SUP_FONT_MEM_SIZE: i32 = 147_483_647;
const SUP_POOL_SIZE: i32 = 40_000_000;
const SUP_HASH_EXTRA: i32 = 2_097_151;
const UNDEFINED_CS: i32 = 103;
const INTER_CHAR_VAL: i32 = 6;
const AAT_FONT_FLAG: i32 = 0xFFFF;
const OTGR_FONT_FLAG: i32 = 0xFFFE;

#[inline]
unsafe fn dump_bytes(
    fmt_out: usize,
    ptr: *const u8,
    item_size: usize,
    nitems: usize,
) -> Result<(), crate::errors::EngineAbort> {
    let byte_len = item_size * nitems;
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
        // A short write returns the abort; tt_run_engine turns it into
        // HISTORY_FATAL_ERROR.
        return Err(crate::errors::EngineAbort::TtAbort(
            c"could not write format file data".as_ptr(),
        ));
    }
    Ok(())
}
#[inline]
unsafe fn dump_slice<T>(
    fmt_out: usize,
    ptr: *const T,
    nitems: i32,
) -> Result<(), crate::errors::EngineAbort> {
    if nitems > 0 {
        dump_bytes(
            fmt_out,
            ptr as *const u8,
            std::mem::size_of::<T>(),
            nitems as usize,
        )?;
    }
    Ok(())
}
#[inline]
unsafe fn dump_one<T: Copy>(fmt_out: usize, value: T) -> Result<(), crate::errors::EngineAbort> {
    dump_slice(fmt_out, &value as *const T, 1)
}
#[inline]
unsafe fn dump_int(fmt_out: usize, value: i32) -> Result<(), crate::errors::EngineAbort> {
    dump_one(fmt_out, value)
}
#[inline]
unsafe fn dump_b32(fmt_out: usize, s0: i32, s1: i32) -> Result<(), crate::errors::EngineAbort> {
    dump_one(fmt_out, crate::mem_types::B32x2 { s0, s1 })
}
#[inline]
unsafe fn undump_bytes(
    fmt_in: usize,
    ptr: *mut u8,
    item_size: usize,
    nitems: usize,
) -> Result<(), crate::errors::EngineAbort> {
    let byte_len = item_size * nitems;
    let read = ttstub_input_read(fmt_in, ptr, byte_len);
    if read < 0 || read as usize != byte_len {
        // A short/corrupt read returns the abort.
        return Err(crate::errors::EngineAbort::TtAbort(
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
#[inline]
unsafe fn undump_slice<T>(
    fmt_in: usize,
    ptr: *mut T,
    nitems: i32,
) -> Result<(), crate::errors::EngineAbort> {
    if nitems > 0 {
        undump_bytes(
            fmt_in,
            ptr as *mut u8,
            std::mem::size_of::<T>(),
            nitems as usize,
        )?;
    }
    Ok(())
}
#[inline]
unsafe fn undump_one<T: Copy>(fmt_in: usize) -> Result<T, crate::errors::EngineAbort> {
    let mut value = std::mem::MaybeUninit::<T>::uninit();
    undump_slice(fmt_in, value.as_mut_ptr(), 1)?;
    Ok(value.assume_init())
}
#[inline]
unsafe fn undump_int(fmt_in: usize) -> Result<i32, crate::errors::EngineAbort> {
    undump_one(fmt_in)
}
unsafe fn undump_checked_i32(
    fmt_in: usize,
    ptr: *mut i32,
    nitems: i32,
    low: i32,
    high: i32,
) -> Result<bool, crate::errors::EngineAbort> {
    undump_slice(fmt_in, ptr, nitems)?;
    for i in 0..nitems {
        let value = *ptr.add(i as usize);
        if value < low || value > high {
            return Ok(false);
        }
    }
    Ok(true)
}
unsafe fn undump_upper_i32(
    fmt_in: usize,
    ptr: *mut i32,
    nitems: i32,
    high: i32,
) -> Result<bool, crate::errors::EngineAbort> {
    undump_slice(fmt_in, ptr, nitems)?;
    for i in 0..nitems {
        if *ptr.add(i as usize) > high {
            return Ok(false);
        }
    }
    Ok(true)
}
unsafe fn undump_upper_u16(
    fmt_in: usize,
    ptr: *mut u16,
    nitems: i32,
    high: u16,
) -> Result<bool, crate::errors::EngineAbort> {
    undump_slice(fmt_in, ptr, nitems)?;
    for i in 0..nitems {
        if *ptr.add(i as usize) > high {
            return Ok(false);
        }
    }
    Ok(true)
}
pub(crate) unsafe fn store_fmt_file_in(
    st: &mut crate::state::EngineState,
) -> Result<(), crate::errors::EngineAbort> {
    if st.save_stack.save_ptr != 0 {
        crate::output::error_here_with_diagnostic_in(st, c"You can't dump inside a group".as_ptr());
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 1;
        st.errors.help_line[0] = c"`{...\\dump}' is a no-no.".as_ptr();

        if st.errors.interaction == InteractionMode::ErrorStop {
            st.errors.interaction = InteractionMode::Scroll;
        }
        if st.errors.log_opened {
            // Continue falls through to the terminal below; a Halt/Hundred outcome
            // propagates as EngineAbort (into_history runs post_error_message and
            // surfaces the halt message, returns 3).
            let outcome = crate::errors::error_in(st);
            if !matches!(outcome, crate::errors::ErrorOutcome::Continue) {
                return Err(outcome.into_engine_abort());
            }
        }

        st.errors.history = HISTORY_FATAL_ERROR;
        // Absorb a finalize_dvi_file failure during terminal cleanup.
        let _ = crate::io::close_files_and_terminate_in(st);
        ttstub_output_flush(st.shipout.rust_stdout);
        // Cleanup already done above; propagate the terminal (TtAbort runs no
        // further cleanup in into_history).
        return Err(crate::errors::EngineAbort::TtAbort(
            c"\\dump inside a group".as_ptr(),
        ));
    }

    st.shipout.selector = SELECTOR_NEW_STRING;
    crate::output::print_cstr_in(st, c" (preloaded format=".as_ptr());
    let job_name = st.strings.job_name;
    crate::output::print_in(st, job_name);
    crate::output::print_char_in(st, b' ' as i32);
    let year = st.eqtb.data[(PC_INT_BASE + INT_PAR_YEAR) as usize].b32.s1;
    crate::output::print_int_in(st, year);
    crate::output::print_char_in(st, b'.' as i32);
    let month = st.eqtb.data[(PC_INT_BASE + INT_PAR_MONTH) as usize].b32.s1;
    crate::output::print_int_in(st, month);
    crate::output::print_char_in(st, b'.' as i32);
    let day = st.eqtb.data[(PC_INT_BASE + INT_PAR_DAY) as usize].b32.s1;
    crate::output::print_int_in(st, day);
    crate::output::print_char_in(st, b')' as i32);

    if st.errors.interaction == InteractionMode::Batch {
        st.shipout.selector = SELECTOR_LOG_ONLY;
    } else {
        st.shipout.selector = SELECTOR_TERM_AND_LOG;
    }

    if st.strings.pool_ptr + 1 > st.strings.pool_size {
        return Err(crate::errors::EngineAbort::Overflow {
            what: c"pool size".as_ptr(),
            n: st.strings.pool_size - st.strings.init_pool_ptr,
        });
    }

    let new_format_ident = crate::string_pool::make_string(&mut st.strings);
    let new_format_ident = match new_format_ident {
        Ok(value) => value,
        Err(crate::scan::NodeRecycleAbort::Overflow) => {
            return Err(crate::errors::EngineAbort::Overflow {
                what: c"number of strings".as_ptr(),
                n: st.strings.max_strings - st.strings.init_str_ptr,
            })
        }
        // Other recycle aborts (main-memory) → EngineAbort: OverflowRunaway runs
        // runaway() then the "main memory size" Overflow (main_memory_overflow_abort),
        // Confusion(msg) → Confusion(msg). (The string-count Overflow above keeps
        // its distinct "number of strings" message.)
        Err(e) => return Err(e.into_engine_abort()),
    };
    st.strings.format_ident = new_format_ident;
    crate::io::pack_job_name_in(st, c".fmt".as_ptr()).map_err(|e| e.into_engine_abort())?;

    let name_ptr = if st.input.name_of_file.is_empty() {
        std::ptr::null_mut()
    } else {
        st.input.name_of_file.as_ptr() as *mut libc::c_char
    };
    let fmt_out = ttstub_output_open(name_ptr, 0);
    if fmt_out == INVALID_HANDLE {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"cannot open format output file".as_ptr(),
        ));
    }

    crate::output::print_nl_cstr_in(st, c"Beginning to dump on file ".as_ptr());
    let name_string = crate::io::make_name_string_in(st).map_err(|e| e.into_engine_abort())?;
    crate::output::print_in(st, name_string);

    st.strings.str_ptr -= 1;
    st.strings.pool_ptr = st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];

    crate::output::print_nl_cstr_in(st, c"".as_ptr());
    let format_ident = st.strings.format_ident;
    crate::output::print_in(st, format_ident);

    dump_int(fmt_out, FORMAT_HEADER_MAGIC)?;
    dump_int(fmt_out, FORMAT_SERIAL)?;
    dump_int(fmt_out, st.eqtb.hash_high)?;

    while st.input.pseudo_files != crate::mem::TEX_NULL {
        crate::input::pseudo_close(&mut st.mem, &mut st.input);
    }

    dump_int(fmt_out, MEM_TOP)?;
    dump_int(fmt_out, EQTB_SIZE)?;
    dump_int(fmt_out, FORMAT_HASH_PRIME)?;
    dump_int(fmt_out, FORMAT_HYPH_PRIME)?;

    dump_int(fmt_out, st.strings.pool_ptr)?;
    dump_int(fmt_out, st.strings.str_ptr)?;
    dump_slice(
        fmt_out,
        st.strings.str_start.as_ptr(),
        st.strings.str_ptr - TOO_BIG_CHAR + 1,
    )?;
    dump_slice(fmt_out, st.strings.str_pool.as_ptr(), st.strings.pool_ptr)?;

    crate::output::print_ln_in(st);
    let str_ptr_snap = st.strings.str_ptr;
    crate::output::print_int_in(st, str_ptr_snap);
    crate::output::print_cstr_in(st, c" strings of total length ".as_ptr());
    let pool_ptr_snap = st.strings.pool_ptr;
    crate::output::print_int_in(st, pool_ptr_snap);

    sort_avail_in(&mut st.mem);
    // These mem bookkeeping scalars do not change during the dump; snapshot the
    // immutable ones once and track var_used/dyn_used as locals (written back
    // below).
    let (lo_mem_max, rover, hi_mem_min, mem_end, avail) = (
        st.mem.lo_mem_max,
        st.mem.rover,
        st.mem.hi_mem_min,
        st.mem.mem_end,
        st.mem.avail,
    );
    let mut var_used = 0i32;
    dump_int(fmt_out, lo_mem_max)?;
    dump_int(fmt_out, rover)?;

    for k in INT_VAL as usize..=INTER_CHAR_VAL as usize {
        let val = st.scanner.sa_root[k];
        dump_int(fmt_out, val)?;
    }

    // `mem` is now a Rust-owned `Vec`; take a stable base pointer once (the
    // thread-local owner never reallocates) for the free-list `dump_slice`
    // calls below.
    let mem_ptr = st.mem.data.as_ptr();
    let mut p = 0;
    let mut q = rover;
    let mut x = 0;
    loop {
        let len = q + 2 - p;
        dump_slice(fmt_out, mem_ptr.add(p as usize), len)?;
        x += len;
        var_used += q - p;
        // SAFETY: union projection; the `Vec` index is bounds-checked.
        p = q + unsafe { st.mem.data[q as usize].b32.s0 };
        // SAFETY: union projection; the `Vec` index is bounds-checked.
        q = unsafe { st.mem.data[(q + 1) as usize].b32.s1 };
        if q == rover {
            break;
        }
    }

    var_used += lo_mem_max - p;
    let mut dyn_used = mem_end + 1 - hi_mem_min;
    dump_slice(fmt_out, mem_ptr.add(p as usize), lo_mem_max + 1 - p)?;
    x += lo_mem_max + 1 - p;

    dump_int(fmt_out, hi_mem_min)?;
    dump_int(fmt_out, avail)?;
    dump_slice(
        fmt_out,
        mem_ptr.add(hi_mem_min as usize),
        mem_end + 1 - hi_mem_min,
    )?;
    x += mem_end + 1 - hi_mem_min;

    p = avail;
    while p != crate::mem::TEX_NULL {
        dyn_used -= 1;
        // SAFETY: union projection; the `Vec` index is bounds-checked.
        p = unsafe { st.mem.data[p as usize].b32.s1 };
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

    // `eqtb` is now a Rust-owned `Vec`; take a stable base pointer once (the
    // thread-local owner never reallocates) for the sparse-region `dump_slice`
    // calls below.
    let eqtb_ptr = st.eqtb.data.as_ptr();
    // Two eqtb words are "equal" for run-length dumping when their b32.s1 +
    // b16.s1 + b16.s0 fields all match.
    let eqtb_words_equal = |st: &crate::state::EngineState, a: i32, b: i32| -> bool {
        // SAFETY: union projection; the `Vec` indices are bounds-checked.
        unsafe {
            st.eqtb.data[a as usize].b32.s1 == st.eqtb.data[b as usize].b32.s1
                && st.eqtb.data[a as usize].b16.s1 == st.eqtb.data[b as usize].b16.s1
                && st.eqtb.data[a as usize].b16.s0 == st.eqtb.data[b as usize].b16.s0
        }
    };
    let mut k = ACTIVE_BASE;
    loop {
        let mut j = k;
        while j < FORMAT_INT_BASE - 1 {
            if eqtb_words_equal(st, j, j + 1) {
                break;
            }
            j += 1;
        }
        let l = if j < FORMAT_INT_BASE - 1 {
            j += 1;
            let l = j;
            while j < FORMAT_INT_BASE - 1 && eqtb_words_equal(st, j, j + 1) {
                j += 1;
            }
            l
        } else {
            FORMAT_INT_BASE
        };

        dump_int(fmt_out, l - k)?;
        dump_slice(fmt_out, eqtb_ptr.add(k as usize), l - k)?;
        k = j + 1;
        dump_int(fmt_out, k - l)?;
        if k == FORMAT_INT_BASE {
            break;
        }
    }

    // SAFETY: union projection; the `Vec` index is bounds-checked.
    let eqtb_b32s1 = |st: &crate::state::EngineState, p: i32| -> i32 {
        unsafe { st.eqtb.data[p as usize].b32.s1 }
    };
    loop {
        let mut j = k;
        while j < EQTB_SIZE {
            if eqtb_b32s1(st, j) == eqtb_b32s1(st, j + 1) {
                break;
            }
            j += 1;
        }
        let l = if j < EQTB_SIZE {
            j += 1;
            let l = j;
            while j < EQTB_SIZE && eqtb_b32s1(st, j) == eqtb_b32s1(st, j + 1) {
                j += 1;
            }
            l
        } else {
            EQTB_SIZE + 1
        };

        dump_int(fmt_out, l - k)?;
        dump_slice(fmt_out, eqtb_ptr.add(k as usize), l - k)?;
        k = j + 1;
        dump_int(fmt_out, k - l)?;
        if k > EQTB_SIZE {
            break;
        }
    }

    // These hash/eqtb scalars are immutable during the dump; snapshot once.
    let hash_high = st.eqtb.hash_high;
    let hash_used = st.eqtb.hash_used;
    // Read both halves of hash word `p` (offset −514 == HASH_BASE).
    let hash_word = |st: &crate::state::EngineState, p: i32| -> (i32, i32) {
        let w = st.eqtb.hash[(p as usize) - 514];
        (w.s0, w.s1)
    };

    if hash_high > 0 {
        dump_slice(fmt_out, eqtb_ptr.add((EQTB_SIZE + 1) as usize), hash_high)?;
    }

    dump_int(fmt_out, st.input.par_loc)?;
    dump_int(fmt_out, st.shipout.write_loc)?;

    for p in 0..=PRIM_SIZE {
        let (s0, s1) = (st.eqtb.prim[p as usize].s0, st.eqtb.prim[p as usize].s1);
        dump_b32(fmt_out, s0, s1)?;
    }

    dump_int(fmt_out, hash_used)?;
    let mut cs_count = (FROZEN_CONTROL_SEQUENCE - 1) - hash_used + hash_high;

    for p in FORMAT_HASH_BASE..=hash_used {
        let (s0, s1) = hash_word(st, p);
        if s1 != 0 {
            dump_int(fmt_out, p)?;
            dump_b32(fmt_out, s0, s1)?;
            cs_count += 1;
        }
    }

    for p in (hash_used + 1)..=(UNDEFINED_CONTROL_SEQUENCE - 1) {
        let (s0, s1) = hash_word(st, p);
        dump_b32(fmt_out, s0, s1)?;
    }
    if hash_high > 0 {
        for p in (EQTB_SIZE + 1)..=(EQTB_SIZE + hash_high) {
            let (s0, s1) = hash_word(st, p);
            dump_b32(fmt_out, s0, s1)?;
        }
    }

    dump_int(fmt_out, cs_count)?;

    crate::output::print_ln_in(st);
    crate::output::print_int_in(st, cs_count);
    crate::output::print_cstr_in(st, c" multiletter control sequences".as_ptr());

    let fmp = st.fonts.fmem_ptr;
    let font_ptr = st.fonts.font_ptr;
    dump_int(fmt_out, fmp)?;
    // Dump the entire font-table region. The interleaved `dump_int`/`dump_slice`
    // are plain file FFI and `fmp`/`font_ptr` are snapshotted scalars; the dump
    // order is preserved byte-for-byte.
    dump_slice(fmt_out, st.fonts.font_info.as_ptr(), fmp)?;
    dump_int(fmt_out, font_ptr)?;
    dump_slice(
        fmt_out,
        st.fonts.font_check.as_ptr().add(FONT_BASE as usize),
        font_ptr + 1,
    )?;
    dump_slice(
        fmt_out,
        st.fonts.font_size.as_ptr().add(FONT_BASE as usize),
        font_ptr + 1,
    )?;
    dump_slice(
        fmt_out,
        st.fonts.font_dsize.as_ptr().add(FONT_BASE as usize),
        font_ptr + 1,
    )?;
    dump_slice(
        fmt_out,
        st.fonts.font_params.as_ptr().add(FONT_BASE as usize),
        font_ptr + 1,
    )?;
    dump_slice(
        fmt_out,
        st.fonts.hyphen_char.as_ptr().add(FONT_BASE as usize),
        font_ptr + 1,
    )?;
    dump_slice(
        fmt_out,
        st.fonts.skew_char.as_ptr().add(FONT_BASE as usize),
        font_ptr + 1,
    )?;
    dump_slice(
        fmt_out,
        st.fonts.font_name.as_ptr().add(FONT_BASE as usize),
        font_ptr + 1,
    )?;
    dump_slice(
        fmt_out,
        st.fonts.font_area.as_ptr().add(FONT_BASE as usize),
        font_ptr + 1,
    )?;
    dump_slice(
        fmt_out,
        st.fonts.font_bc.as_ptr().add(FONT_BASE as usize),
        font_ptr + 1,
    )?;
    dump_slice(
        fmt_out,
        st.fonts.font_ec.as_ptr().add(FONT_BASE as usize),
        font_ptr + 1,
    )?;
    let font_dump_count = font_ptr + 1;
    dump_slice(
        fmt_out,
        st.fonts.char_base.as_ptr().add(FONT_BASE as usize),
        font_dump_count,
    )?;
    dump_slice(
        fmt_out,
        st.fonts.width_base.as_ptr().add(FONT_BASE as usize),
        font_dump_count,
    )?;
    dump_slice(
        fmt_out,
        st.fonts.height_base.as_ptr().add(FONT_BASE as usize),
        font_dump_count,
    )?;
    dump_slice(
        fmt_out,
        st.fonts.depth_base.as_ptr().add(FONT_BASE as usize),
        font_dump_count,
    )?;
    dump_slice(
        fmt_out,
        st.fonts.italic_base.as_ptr().add(FONT_BASE as usize),
        font_dump_count,
    )?;
    dump_slice(
        fmt_out,
        st.fonts.lig_kern_base.as_ptr().add(FONT_BASE as usize),
        font_dump_count,
    )?;
    dump_slice(
        fmt_out,
        st.fonts.kern_base.as_ptr().add(FONT_BASE as usize),
        font_dump_count,
    )?;
    dump_slice(
        fmt_out,
        st.fonts.exten_base.as_ptr().add(FONT_BASE as usize),
        font_dump_count,
    )?;
    dump_slice(
        fmt_out,
        st.fonts.param_base.as_ptr().add(FONT_BASE as usize),
        font_dump_count,
    )?;
    dump_slice(
        fmt_out,
        st.fonts.font_glue.as_ptr().add(FONT_BASE as usize),
        font_ptr + 1,
    )?;
    dump_slice(
        fmt_out,
        st.fonts.bchar_label.as_ptr().add(FONT_BASE as usize),
        font_ptr + 1,
    )?;
    dump_slice(
        fmt_out,
        st.fonts.font_bchar.as_ptr().add(FONT_BASE as usize),
        font_ptr + 1,
    )?;
    dump_slice(
        fmt_out,
        st.fonts.font_false_bchar.as_ptr().add(FONT_BASE as usize),
        font_ptr + 1,
    )?;

    for k in FONT_BASE..=font_ptr {
        crate::output::print_nl_cstr_in(st, c"\\font".as_ptr());
        let font_id_s1 = st.eqtb.hash[((FONT_ID_BASE + k) as usize) - 514].s1;
        crate::output::print_esc_in(st, font_id_s1);
        crate::output::print_char_in(st, b'=' as i32);

        let font_name = st.fonts.font_name[k as usize];
        let font_area = st.fonts.font_area[k as usize];
        let font_mapping = st.fonts.font_mapping[k as usize];
        if font_area == AAT_FONT_FLAG || font_area == OTGR_FONT_FLAG || !font_mapping.is_null() {
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
            // Continue resumes the font-dump loop; a Halt/Hundred outcome propagates
            // as EngineAbort (into_history prints + surfaces the halt, returns 3).
            let outcome = crate::errors::error_in(st);
            if !matches!(outcome, crate::errors::ErrorOutcome::Continue) {
                return Err(outcome.into_engine_abort());
            }
        } else {
            crate::output::print_file_name_in(st, font_name, font_area, EMPTY_STRING);
        }

        let (font_size, font_dsize) = (
            st.fonts.font_size[k as usize],
            st.fonts.font_dsize[k as usize],
        );
        if font_size != font_dsize {
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

    let (hyph_count, hyph_size) = (st.hyph.hyph_count, st.hyph.hyph_size);
    dump_int(fmt_out, hyph_count)?;
    let mut hyph_next = st.hyph.hyph_next;
    if hyph_next <= FORMAT_HYPH_PRIME {
        hyph_next = hyph_size;
        st.hyph.hyph_next = hyph_next;
    }
    dump_int(fmt_out, hyph_next)?;

    for k in 0..=hyph_size {
        let (word, link, list) = (
            st.hyph.hyph_word[k as usize],
            st.hyph.hyph_link[k as usize],
            st.hyph.hyph_list[k as usize],
        );
        if word != 0 {
            dump_int(fmt_out, k + 65_536 * link as i32)?;
            dump_int(fmt_out, word)?;
            dump_int(fmt_out, list)?;
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

    // trie_max/hyph_start/max_hyph_char/trie_op_ptr are final after init_trie;
    // snapshot them as locals for the rest of the region.
    let (trie_max, hyph_start, max_hyph_char, trie_op_ptr) = (
        st.hyph.trie_max,
        st.hyph.hyph_start,
        st.hyph.max_hyph_char,
        st.hyph.trie_op_ptr,
    );
    dump_int(fmt_out, trie_max)?;
    dump_int(fmt_out, hyph_start)?;
    // Snapshot all six trie array base pointers: the arenas never reallocate
    // during dump, so the pointers stay valid across the FFI `dump_slice`/
    // `dump_int` calls below.
    let (fmt_trie_trl, fmt_trie_tro, fmt_trie_trc, fmt_hyf_distance, fmt_hyf_num, fmt_hyf_next) = (
        st.hyph.trie_trl.as_ptr(),
        st.hyph.trie_tro.as_ptr(),
        st.hyph.trie_trc.as_ptr(),
        st.hyph.hyf_distance.as_ptr(),
        st.hyph.hyf_num.as_ptr(),
        st.hyph.hyf_next.as_ptr(),
    );
    dump_slice(fmt_out, fmt_trie_trl, trie_max + 1)?;
    dump_slice(fmt_out, fmt_trie_tro, trie_max + 1)?;
    dump_slice(fmt_out, fmt_trie_trc, trie_max + 1)?;
    dump_int(fmt_out, max_hyph_char)?;
    dump_int(fmt_out, trie_op_ptr)?;
    dump_slice(fmt_out, fmt_hyf_distance.add(1), trie_op_ptr)?;
    dump_slice(fmt_out, fmt_hyf_num.add(1), trie_op_ptr)?;
    dump_slice(fmt_out, fmt_hyf_next.add(1), trie_op_ptr)?;

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
        let used = st.hyph.trie_used[k];
        if used > 0 {
            crate::output::print_nl_cstr_in(st, c"  ".as_ptr());
            crate::output::print_int_in(st, used as i32);
            crate::output::print_cstr_in(st, c" for language ".as_ptr());
            crate::output::print_int_in(st, k as i32);
            dump_int(fmt_out, k as i32)?;
            dump_int(fmt_out, used as i32)?;
        }
    }

    dump_int(fmt_out, FORMAT_FOOTER_MAGIC)?;
    st.eqtb.data[(PC_INT_BASE + INT_PAR_TRACING_STATS) as usize].b32.s1 = 0;
    ttstub_output_close(fmt_out);
    Ok(())
}
// translated from xetex-ini.c:602 (load_fmt_file)
pub unsafe fn load_fmt_file(
    st: &mut crate::state::EngineState,
) -> Result<bool, crate::errors::EngineAbort> {
    {
        let j = st.input.cur_input.loc;

        // pack_buffered_name (xetex-ini.c:590): copy the format-default name
        // into name_of_file and set name_length.
        let v = tre_TEX_format_default;
        let len = libc::strlen(v);
        st.input.name_of_file.resize(len + 1, 0);
        libc::strcpy(st.input.name_of_file.as_mut_ptr(), v);
        st.input.name_length = len as i32;

        let name_ptr = if st.input.name_of_file.is_empty() {
            std::ptr::null_mut()
        } else {
            st.input.name_of_file.as_ptr() as *mut libc::c_char
        };
        let fmt_in = ttstub_input_open(name_ptr, TTBC_FILE_FORMAT_FORMAT, 0);
        if fmt_in == INVALID_HANDLE {
            return Err(crate::errors::EngineAbort::TtAbort(
                c"cannot open the format file".as_ptr(),
            ));
        }

        st.input.cur_input.loc = j;

        load_fmt_file_in(st, fmt_in)
    }
}

unsafe fn load_fmt_file_in(
    st: &mut crate::state::EngineState,
    fmt_in: usize,
) -> Result<bool, crate::errors::EngineAbort> {
    let mut j: i32;
    let mut k: i32;
    let mut p: i32;
    let mut q: i32;
    let mut x: i32;

    if st.config.in_initex_mode {
        st.fonts.font_info = Vec::new();
        st.strings.str_pool = Vec::new();
        st.strings.str_start = Vec::new();
        st.eqtb.data = Vec::new();
        st.eqtb.hash = Vec::new();
        st.mem.data = Vec::new();
    }

    x = undump_int(fmt_in)?;
    if x != FORMAT_HEADER_MAGIC {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }

    x = undump_int(fmt_in)?;
    if x != FORMAT_SERIAL {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"format file is of the wrong version".as_ptr(),
        ));
    }

    x = undump_int(fmt_in)?;
    if !(0..=SUP_HASH_EXTRA).contains(&x) {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    st.eqtb.hash_high = x;
    if st.eqtb.hash_extra < st.eqtb.hash_high {
        st.eqtb.hash_extra = st.eqtb.hash_high;
    }
    st.eqtb.eqtb_top = EQTB_SIZE + st.eqtb.hash_extra;
    st.eqtb.hash_top = if st.eqtb.hash_extra == 0 {
        UNDEFINED_CONTROL_SEQUENCE
    } else {
        st.eqtb.eqtb_top
    };
    let n = (1 + st.eqtb.hash_top - 514) as usize;
    st.eqtb.hash = vec![crate::mem_types::B32x2 { s0: 0, s1: 0 }; n];

    let base_hash = crate::mem_types::B32x2 { s0: 0, s1: 0 };
    st.eqtb.hash[(FORMAT_HASH_BASE - 514) as usize] = base_hash;
    for h in (FORMAT_HASH_BASE + 1)..=st.eqtb.hash_top {
        st.eqtb.hash[(h - 514) as usize] = base_hash;
    }

    // `xmalloc_array(n)` allocated `n + 1` words; match that inclusive size.
    let eqtb_words_load = (st.eqtb.eqtb_top as usize) + 1;
    st.eqtb.data = vec![
        crate::mem_types::MemoryWord {
            b32: crate::mem_types::B32x2 { s0: 0, s1: 0 }
        };
        eqtb_words_load
    ];
    st.eqtb
        .set_b16s1(UNDEFINED_CONTROL_SEQUENCE, UNDEFINED_CS as u16);
    st.eqtb
        .set_b32s1(UNDEFINED_CONTROL_SEQUENCE, crate::mem::TEX_NULL);
    st.eqtb.set_b16s0(UNDEFINED_CONTROL_SEQUENCE, 0);
    let undefined_entry = st.eqtb.word(UNDEFINED_CONTROL_SEQUENCE);
    for e in (EQTB_SIZE + 1)..=st.eqtb.eqtb_top {
        st.eqtb.data[e as usize] = undefined_entry;
    }

    st.scanner.max_reg_num = 32_767;
    st.scanner.max_reg_help_line = c"A register number must be between 0 and 32767.".as_ptr();

    x = undump_int(fmt_in)?;
    if x != MEM_TOP {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }

    st.input.cur_list.head = CONTRIB_HEAD;
    st.input.cur_list.tail = CONTRIB_HEAD;
    st.shipout.page_tail = PAGE_HEAD;
    // `xmalloc_array(MEM_TOP)` allocated `MEM_TOP + 1` words; match it.
    let mem_words_load = (MEM_TOP as usize) + 1;
    st.mem.data = vec![
        crate::mem_types::MemoryWord {
            b32: crate::mem_types::B32x2 { s0: 0, s1: 0 }
        };
        mem_words_load
    ];

    x = undump_int(fmt_in)?;
    if x != EQTB_SIZE {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    x = undump_int(fmt_in)?;
    if x != FORMAT_HASH_PRIME {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    x = undump_int(fmt_in)?;
    if x != FORMAT_HYPH_PRIME {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }

    x = undump_int(fmt_in)?;
    if x < 0 {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    if x > SUP_POOL_SIZE - tre_pool_free {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"must increase string_pool_size".as_ptr(),
        ));
    }
    st.strings.pool_ptr = x;
    if TRE_POOL_SIZE < st.strings.pool_ptr + tre_pool_free {
        TRE_POOL_SIZE = st.strings.pool_ptr + tre_pool_free;
    }
    st.strings.pool_size = TRE_POOL_SIZE;

    x = undump_int(fmt_in)?;
    if x < 0 {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    if x > SUP_MAX_STRINGS - tre_strings_free {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"must increase sup_strings".as_ptr(),
        ));
    }
    st.strings.str_ptr = x;
    if TRE_MAX_STRINGS < st.strings.str_ptr + tre_strings_free {
        TRE_MAX_STRINGS = st.strings.str_ptr + tre_strings_free;
    }
    st.strings.max_strings = TRE_MAX_STRINGS;

    st.strings.str_start = vec![0i32; (TRE_MAX_STRINGS as usize) + 1];
    let n = st.strings.str_ptr - TOO_BIG_CHAR + 1;
    let limit = st.strings.pool_ptr;
    if !undump_checked_i32(fmt_in, st.strings.str_start.as_mut_ptr(), n, 0, limit)? {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    st.strings.str_pool = vec![0u16; (TRE_POOL_SIZE as usize) + 1];
    let limit = st.strings.pool_ptr;
    undump_slice(fmt_in, st.strings.str_pool.as_mut_ptr(), limit)?;

    st.strings.init_str_ptr = st.strings.str_ptr;
    st.strings.init_pool_ptr = st.strings.pool_ptr;

    x = undump_int(fmt_in)?;
    if !(1019..=MEM_TOP - HI_MEM_STAT_USAGE).contains(&x) {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    st.mem.lo_mem_max = x;

    x = undump_int(fmt_in)?;
    if x < 20 || x > st.mem.lo_mem_max {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    st.mem.rover = x;

    for idx in INT_VAL..=INTER_CHAR_VAL {
        x = undump_int(fmt_in)?;
        if x < MIN_HALFWORD || x > st.mem.lo_mem_max {
            return Err(crate::errors::EngineAbort::TtAbort(
                c"fatal format file error".as_ptr(),
            ));
        }
        st.scanner.sa_root[idx as usize] = x;
    }

    // `mem`/`eqtb`/`hash` are Rust-owned `Vec`s; take a stable mutable base
    // pointer once (never reallocated) for the free-list / sparse-region
    // `undump_slice` calls.
    let mem_ptr = st.mem.data.as_mut_ptr();
    p = 0;
    q = st.mem.rover;
    loop {
        undump_slice(fmt_in, mem_ptr.add(p as usize), q + 2 - p)?;
        p = q + st.mem.b32s0(q);
        if p > st.mem.lo_mem_max
            || (q >= st.mem.b32s1(q + 1) && st.mem.b32s1(q + 1) != st.mem.rover)
        {
            return Err(crate::errors::EngineAbort::TtAbort(
                c"fatal format file error".as_ptr(),
            ));
        }
        q = st.mem.b32s1(q + 1);
        if q == st.mem.rover {
            break;
        }
    }

    undump_slice(fmt_in, mem_ptr.add(p as usize), st.mem.lo_mem_max + 1 - p)?;

    x = undump_int(fmt_in)?;
    if x < st.mem.lo_mem_max + 1 || x > PRE_ADJUST_HEAD {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    st.mem.hi_mem_min = x;

    x = undump_int(fmt_in)?;
    if !(MIN_HALFWORD..=MEM_TOP).contains(&x) {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    st.mem.avail = x;

    st.mem.mem_end = MEM_TOP;
    undump_slice(
        fmt_in,
        mem_ptr.add(st.mem.hi_mem_min as usize),
        st.mem.mem_end + 1 - st.mem.hi_mem_min,
    )?;
    st.mem.var_used = undump_int(fmt_in)?;
    st.mem.dyn_used = undump_int(fmt_in)?;

    let eqtb_ptr = st.eqtb.data.as_mut_ptr();
    k = ACTIVE_BASE;
    loop {
        x = undump_int(fmt_in)?;
        if x < 1 || k + x > EQTB_SIZE + 1 {
            return Err(crate::errors::EngineAbort::TtAbort(
                c"fatal format file error".as_ptr(),
            ));
        }

        undump_slice(fmt_in, eqtb_ptr.add(k as usize), x)?;
        k += x;

        x = undump_int(fmt_in)?;
        if x < 0 || k + x > EQTB_SIZE + 1 {
            return Err(crate::errors::EngineAbort::TtAbort(
                c"fatal format file error".as_ptr(),
            ));
        }

        for j1 in k..=(k + x - 1) {
            *eqtb_ptr.add(j1 as usize) = *eqtb_ptr.add((k - 1) as usize);
        }
        k += x;
        if k > EQTB_SIZE {
            break;
        }
    }

    if st.eqtb.hash_high > 0 {
        undump_slice(
            fmt_in,
            eqtb_ptr.add((EQTB_SIZE + 1) as usize),
            st.eqtb.hash_high,
        )?;
    }

    x = undump_int(fmt_in)?;
    if x < FORMAT_HASH_BASE || x > st.eqtb.hash_top {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    st.input.par_loc = x;
    st.input.par_token = CS_TOKEN_FLAG + st.input.par_loc;

    x = undump_int(fmt_in)?;
    if x < FORMAT_HASH_BASE || x > st.eqtb.hash_top {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    st.shipout.write_loc = x;

    for prim_idx in 0..=PRIM_SIZE {
        let value: crate::mem_types::B32x2 = undump_one(fmt_in)?;
        st.eqtb.prim[prim_idx as usize].s0 = value.s0;
        st.eqtb.prim[prim_idx as usize].s1 = value.s1;
    }

    x = undump_int(fmt_in)?;
    if !(FORMAT_HASH_BASE..=FROZEN_CONTROL_SEQUENCE).contains(&x) {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    st.eqtb.hash_used = x;

    p = FORMAT_HASH_BASE - 1;
    loop {
        x = undump_int(fmt_in)?;
        if x < p + 1 || x > st.eqtb.hash_used {
            return Err(crate::errors::EngineAbort::TtAbort(
                c"fatal format file error".as_ptr(),
            ));
        }
        p = x;
        let value: crate::mem_types::B32x2 = undump_one(fmt_in)?;
        st.eqtb.hash[(p - 514) as usize].s0 = value.s0;
        st.eqtb.hash[(p - 514) as usize].s1 = value.s1;
        if p == st.eqtb.hash_used {
            break;
        }
    }

    let hash_ptr = st.eqtb.hash.as_mut_ptr();
    undump_slice(
        fmt_in,
        hash_ptr.add((st.eqtb.hash_used + 1 - 514) as usize),
        (UNDEFINED_CONTROL_SEQUENCE - 1) - st.eqtb.hash_used,
    )?;
    if st.eqtb.hash_high > 0 {
        undump_slice(
            fmt_in,
            hash_ptr.add((EQTB_SIZE + 1 - 514) as usize),
            st.eqtb.hash_high,
        )?;
    }

    st.eqtb.cs_count = undump_int(fmt_in)?;

    x = undump_int(fmt_in)?;
    if x < 7 {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    if x > SUP_FONT_MEM_SIZE {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"must increase font_mem_size".as_ptr(),
        ));
    }
    st.fonts.fmem_ptr = x;
    if st.fonts.fmem_ptr > st.fonts.font_mem_size {
        st.fonts.font_mem_size = st.fonts.fmem_ptr;
    }

    let fms = st.fonts.font_mem_size;
    let fmp = st.fonts.fmem_ptr;
    let zero_word = crate::mem_types::MemoryWord {
        b32: crate::mem_types::B32x2 { s0: 0, s1: 0 },
    };
    st.fonts.font_info = vec![zero_word; (fms as usize) + 1];
    undump_slice(fmt_in, st.fonts.font_info.as_mut_ptr(), fmp)?;

    x = undump_int(fmt_in)?;
    if x < FONT_BASE {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    if x > FONT_BASE + MAX_FONT_MAX {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"must increase font_max".as_ptr(),
        ));
    }
    st.fonts.font_ptr = x;

    let font_check_len = (st.fonts.font_max as usize) + 1;
    st.fonts.font_layout_engine = vec![std::ptr::null_mut(); font_check_len];
    st.fonts.font_mapping = vec![std::ptr::null_mut(); font_check_len];
    st.fonts.font_flags = vec![0i8; font_check_len];
    st.fonts.font_letter_space = vec![0i32; font_check_len];
    st.fonts.font_check = vec![
        crate::mem_types::B16x4 {
            s0: 0,
            s1: 0,
            s2: 0,
            s3: 0,
        };
        font_check_len
    ];
    st.fonts.font_size = vec![0i32; font_check_len];
    st.fonts.font_dsize = vec![0i32; font_check_len];
    st.fonts.font_params = vec![0i32; font_check_len];
    st.fonts.font_name = vec![0i32; font_check_len];
    st.fonts.font_area = vec![0i32; font_check_len];
    st.fonts.font_bc = vec![0u16; font_check_len];
    st.fonts.font_ec = vec![0u16; font_check_len];
    st.fonts.bchar_label = vec![0i32; font_check_len];
    st.fonts.font_bchar = vec![0i32; font_check_len];
    st.fonts.font_false_bchar = vec![0i32; font_check_len];
    st.fonts.font_glue = vec![0i32; font_check_len];
    crate::globals::refresh_font_ffi_mirrors(&st.fonts);

    let base_arr_len = (st.fonts.font_max as usize) + 1;
    st.fonts.hyphen_char = vec![0i32; base_arr_len];
    st.fonts.skew_char = vec![0i32; base_arr_len];
    st.fonts.char_base = vec![0i32; base_arr_len];
    st.fonts.width_base = vec![0i32; base_arr_len];
    st.fonts.height_base = vec![0i32; base_arr_len];
    st.fonts.depth_base = vec![0i32; base_arr_len];
    st.fonts.italic_base = vec![0i32; base_arr_len];
    st.fonts.lig_kern_base = vec![0i32; base_arr_len];
    st.fonts.kern_base = vec![0i32; base_arr_len];
    st.fonts.exten_base = vec![0i32; base_arr_len];
    st.fonts.param_base = vec![0i32; base_arr_len];

    let font_undump_count = st.fonts.font_ptr + 1;
    undump_slice(
        fmt_in,
        st.fonts.font_check.as_mut_ptr().add(FONT_BASE as usize),
        font_undump_count,
    )?;
    undump_slice(
        fmt_in,
        st.fonts.font_size.as_mut_ptr().add(FONT_BASE as usize),
        font_undump_count,
    )?;
    undump_slice(
        fmt_in,
        st.fonts.font_dsize.as_mut_ptr().add(FONT_BASE as usize),
        font_undump_count,
    )?;
    if !undump_checked_i32(
        fmt_in,
        st.fonts.font_params.as_mut_ptr().add(FONT_BASE as usize),
        font_undump_count,
        MIN_HALFWORD,
        MAX_HALFWORD,
    )? {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    undump_slice(
        fmt_in,
        st.fonts.hyphen_char.as_mut_ptr().add(FONT_BASE as usize),
        font_undump_count,
    )?;
    undump_slice(
        fmt_in,
        st.fonts.skew_char.as_mut_ptr().add(FONT_BASE as usize),
        font_undump_count,
    )?;
    let font_count = font_undump_count;
    let max_str = st.strings.str_ptr;
    if !undump_upper_i32(
        fmt_in,
        st.fonts.font_name.as_mut_ptr().add(FONT_BASE as usize),
        font_count,
        max_str,
    )? {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    if !undump_upper_i32(
        fmt_in,
        st.fonts.font_area.as_mut_ptr().add(FONT_BASE as usize),
        font_count,
        max_str,
    )? {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    undump_slice(
        fmt_in,
        st.fonts.font_bc.as_mut_ptr().add(FONT_BASE as usize),
        font_count,
    )?;
    undump_slice(
        fmt_in,
        st.fonts.font_ec.as_mut_ptr().add(FONT_BASE as usize),
        font_count,
    )?;
    let base_undump_count = st.fonts.font_ptr + 1;
    undump_slice(
        fmt_in,
        st.fonts.char_base.as_mut_ptr().add(FONT_BASE as usize),
        base_undump_count,
    )?;
    undump_slice(
        fmt_in,
        st.fonts.width_base.as_mut_ptr().add(FONT_BASE as usize),
        base_undump_count,
    )?;
    undump_slice(
        fmt_in,
        st.fonts.height_base.as_mut_ptr().add(FONT_BASE as usize),
        base_undump_count,
    )?;
    undump_slice(
        fmt_in,
        st.fonts.depth_base.as_mut_ptr().add(FONT_BASE as usize),
        base_undump_count,
    )?;
    undump_slice(
        fmt_in,
        st.fonts.italic_base.as_mut_ptr().add(FONT_BASE as usize),
        base_undump_count,
    )?;
    undump_slice(
        fmt_in,
        st.fonts.lig_kern_base.as_mut_ptr().add(FONT_BASE as usize),
        base_undump_count,
    )?;
    undump_slice(
        fmt_in,
        st.fonts.kern_base.as_mut_ptr().add(FONT_BASE as usize),
        base_undump_count,
    )?;
    undump_slice(
        fmt_in,
        st.fonts.exten_base.as_mut_ptr().add(FONT_BASE as usize),
        base_undump_count,
    )?;
    undump_slice(
        fmt_in,
        st.fonts.param_base.as_mut_ptr().add(FONT_BASE as usize),
        base_undump_count,
    )?;
    let font_glue_count = st.fonts.font_ptr + 1;
    let font_glue_max = st.mem.lo_mem_max;
    if !undump_checked_i32(
        fmt_in,
        st.fonts.font_glue.as_mut_ptr().add(FONT_BASE as usize),
        font_glue_count,
        MIN_HALFWORD,
        font_glue_max,
    )? {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    let max_font_index = st.fonts.fmem_ptr - 1;
    if !undump_checked_i32(
        fmt_in,
        st.fonts.bchar_label.as_mut_ptr().add(FONT_BASE as usize),
        font_count,
        0,
        max_font_index,
    )? {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    if !undump_checked_i32(
        fmt_in,
        st.fonts.font_bchar.as_mut_ptr().add(FONT_BASE as usize),
        font_count,
        0,
        TOO_BIG_CHAR,
    )? {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    if !undump_checked_i32(
        fmt_in,
        st.fonts
            .font_false_bchar
            .as_mut_ptr()
            .add(FONT_BASE as usize),
        font_count,
        0,
        TOO_BIG_CHAR,
    )? {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }

    x = undump_int(fmt_in)?;
    if x < 0 {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    if x > st.hyph.hyph_size {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"must increase hyph_size".as_ptr(),
        ));
    }
    st.hyph.hyph_count = x;

    x = undump_int(fmt_in)?;
    if x < FORMAT_HYPH_PRIME {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    if x > st.hyph.hyph_size {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"must increase hyph_size".as_ptr(),
        ));
    }
    st.hyph.hyph_next = x;

    j = 0;
    for _ in 1..=st.hyph.hyph_count {
        j = undump_int(fmt_in)?;
        if j < 0 {
            return Err(crate::errors::EngineAbort::TtAbort(
                c"fatal format file error".as_ptr(),
            ));
        }
        if j > 65_535 {
            st.hyph.hyph_next = j / 65_536;
            j -= st.hyph.hyph_next * 65_536;
        } else {
            st.hyph.hyph_next = 0;
        }

        if j >= st.hyph.hyph_size || st.hyph.hyph_next > st.hyph.hyph_size {
            return Err(crate::errors::EngineAbort::TtAbort(
                c"fatal format file error".as_ptr(),
            ));
        }
        st.hyph.hyph_link[j as usize] = st.hyph.hyph_next as u16;

        x = undump_int(fmt_in)?;
        if x < 0 || x > st.strings.str_ptr {
            return Err(crate::errors::EngineAbort::TtAbort(
                c"fatal format file error".as_ptr(),
            ));
        }
        st.hyph.hyph_word[j as usize] = x;

        x = undump_int(fmt_in)?;
        if !(MIN_HALFWORD..=MAX_HALFWORD).contains(&x) {
            return Err(crate::errors::EngineAbort::TtAbort(
                c"fatal format file error".as_ptr(),
            ));
        }
        st.hyph.hyph_list[j as usize] = x;
    }

    j += 1;
    if j < FORMAT_HYPH_PRIME {
        j = FORMAT_HYPH_PRIME;
    }
    st.hyph.hyph_next = j;
    if st.hyph.hyph_next >= st.hyph.hyph_size {
        st.hyph.hyph_next = FORMAT_HYPH_PRIME;
    } else if st.hyph.hyph_next >= FORMAT_HYPH_PRIME {
        st.hyph.hyph_next += 1;
    }

    x = undump_int(fmt_in)?;
    if x < 0 {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    if x > st.hyph.trie_size {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"must increase trie_size".as_ptr(),
        ));
    }
    j = x;
    st.hyph.trie_max = j;

    x = undump_int(fmt_in)?;
    if x < 0 || x > j {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    st.hyph.hyph_start = x;

    if st.hyph.trie_trl.is_empty() {
        st.hyph.trie_trl = vec![0i32; (j as usize) + 1];
    }
    undump_slice(fmt_in, st.hyph.trie_trl.as_mut_ptr(), j + 1)?;
    if st.hyph.trie_tro.is_empty() {
        st.hyph.trie_tro = vec![0i32; (j as usize) + 1];
    }
    undump_slice(fmt_in, st.hyph.trie_tro.as_mut_ptr(), j + 1)?;
    if st.hyph.trie_trc.is_empty() {
        st.hyph.trie_trc = vec![0u16; (j as usize) + 1];
    }
    undump_slice(fmt_in, st.hyph.trie_trc.as_mut_ptr(), j + 1)?;

    st.hyph.max_hyph_char = undump_int(fmt_in)?;

    x = undump_int(fmt_in)?;
    if x < 0 {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }
    if x > TRIE_OP_SIZE {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"must increase TRIE_OP_SIZE".as_ptr(),
        ));
    }
    j = x;
    st.hyph.trie_op_ptr = j;

    let fmt_hyf_distance = st.hyph.hyf_distance.as_mut_ptr();
    let fmt_hyf_num = st.hyph.hyf_num.as_mut_ptr();
    let fmt_hyf_next = st.hyph.hyf_next.as_mut_ptr();
    undump_slice(fmt_in, fmt_hyf_distance.add(1), j)?;
    undump_slice(fmt_in, fmt_hyf_num.add(1), j)?;
    if !undump_upper_u16(fmt_in, fmt_hyf_next.add(1), j, MAX_TRIE_OP)? {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }

    for lang in 0..=BIGGEST_LANG as usize {
        st.hyph.trie_used[lang] = 0;
    }

    k = BIGGEST_LANG + 1;
    while j > 0 {
        x = undump_int(fmt_in)?;
        if x < 0 || x > k - 1 {
            return Err(crate::errors::EngineAbort::TtAbort(
                c"fatal format file error".as_ptr(),
            ));
        }
        k = x;

        x = undump_int(fmt_in)?;
        if x < 1 || x > j {
            return Err(crate::errors::EngineAbort::TtAbort(
                c"fatal format file error".as_ptr(),
            ));
        }
        st.hyph.trie_used[k as usize] = x as u16;
        j -= x;
        st.hyph.op_start[k as usize] = j;
    }

    st.hyph.trie_not_ready = false;

    x = undump_int(fmt_in)?;
    if x != FORMAT_FOOTER_MAGIC {
        return Err(crate::errors::EngineAbort::TtAbort(
            c"fatal format file error".as_ptr(),
        ));
    }

    if crate::io::input_close(fmt_in) {
        // The C ttstub_input_close wrapper longjmp'd on this serious internal
        // close error; propagate it to tt_run_engine as a fatal abort.
        return Err(crate::errors::EngineAbort::TtAbort(
            c"serious internal error closing the format file".as_ptr(),
        ));
    }
    Ok(true)
}
