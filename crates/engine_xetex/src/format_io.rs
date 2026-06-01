//! Format-file dump and load (the `\dump` / preloaded-format machinery),
//! translated from the format-I/O part of xetex-ini.c.

use crate::consts::{
    ACTIVE_BASE, BATCH_MODE, CS_TOKEN_FLAG, EQTB_SIZE, ERROR_STOP_MODE, MAX_HALFWORD, MIN_HALFWORD,
    SCROLL_MODE, SELECTOR_TERM_AND_LOG, TOO_BIG_CHAR,
};
use crate::engine::{intpar, sort_avail, TRE_MAX_STRINGS, TRE_POOL_SIZE};
use crate::globals::{
    avail, cs_count, cur_input, dyn_used, eqtb_b16s0, eqtb_b16s1, eqtb_b32s1, eqtb_word, fmem_ptr,
    font_area_entry, font_dsize_entry, font_mapping_ptr, font_max, font_mem_size, font_name_entry,
    font_ptr, font_size_entry, format_ident, hash_high, hash_used, hi_mem_min, hyph_count,
    hyph_link_entry, hyph_list_entry, hyph_next, hyph_size, hyph_start, hyph_word_entry,
    in_initex_mode, init_pool_ptr, interaction, job_name, lo_mem_max, max_hyph_char, mem_b32s0,
    mem_b32s1, mem_end, par_loc, pool_ptr, pool_size, rover, set_avail, set_cs_count,
    set_cur_input_loc, set_cur_list_head, set_cur_list_tail, set_dyn_used, set_eqtb_b16s0,
    set_eqtb_b16s1, set_eqtb_b32s1, set_eqtb_word, set_fmem_ptr, set_font_mem_size, set_font_ptr,
    set_format_ident, set_hash_high, set_hash_used, set_help_line, set_help_ptr, set_hi_mem_min,
    set_hyph_count, set_hyph_link_entry, set_hyph_list_entry, set_hyph_next, set_hyph_start,
    set_hyph_word_entry, set_interaction, set_lo_mem_max, set_max_hyph_char, set_max_reg_help_line,
    set_max_reg_num, set_mem_end, set_op_start_entry, set_page_tail, set_par_loc, set_par_token,
    set_pool_ptr, set_prim_s0, set_prim_s1, set_rover, set_selector, set_str_ptr, set_trie_max,
    set_trie_not_ready, set_trie_op_ptr, set_trie_used_entry, set_var_used, set_write_loc, str_ptr,
    str_start, trie_max, trie_not_ready, trie_op_ptr, trie_size, trie_used_entry, var_used,
    write_loc,
};
use crate::hyphenation::{init_trie, BIGGEST_LANG, MAX_TRIE_OP, TRIE_OP_SIZE};

extern "C" {
    fn _tt_abort(fmt: *const libc::c_char) -> !;
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
    fn ttstub_input_close(handle: usize) -> libc::c_int;

    #[link_name = "TEX_format_default"]
    static mut tre_TEX_format_default: *mut libc::c_char;
    #[link_name = "format_default_length"]
    static mut tre_format_default_length: i32;
    #[link_name = "pool_free"]
    static mut tre_pool_free: i32;
    #[link_name = "strings_free"]
    static mut tre_strings_free: i32;
}

const INVALID_HANDLE: usize = 0;
const MEM_TOP: i32 = 4_999_999;
const FONT_BASE: i32 = 0;
const MAX_FONT_MAX: i32 = 9_000;
const CONTRIB_HEAD: i32 = MEM_TOP - 1;
const PAGE_HEAD: i32 = MEM_TOP - 2;
const PRE_ADJUST_HEAD: i32 = MEM_TOP - 14;
const HI_MEM_STAT_USAGE: i32 = 15;
const HISTORY_FATAL_ERROR: i32 = 3;
const EMPTY_STRING: i32 = 65_536;
const INT_PAR_DAY: i32 = 21;
const INT_PAR_MONTH: i32 = 22;
const INT_PAR_YEAR: i32 = 23;
const INT_PAR_TRACING_STATS: i32 = 31;
const FORMAT_HEADER_MAGIC: i32 = 0x5454_4E43;
const FORMAT_FOOTER_MAGIC: i32 = 0x0000_029A;
const FORMAT_SERIAL: i32 = 33;
const FORMAT_HASH_PRIME: i32 = 8_501;
const FORMAT_HYPH_PRIME: i32 = 607;
const FORMAT_HASH_BASE: i32 = 2_228_226;
const INT_VAL: i32 = 0;
const FROZEN_CONTROL_SEQUENCE: i32 = 2_243_226;
const UNDEFINED_CONTROL_SEQUENCE: i32 = 2_254_339;
const FONT_ID_BASE: i32 = 2_245_338;
const PRIM_SIZE: i32 = 2_100;
const FORMAT_INT_BASE: i32 = 7_826_729;
const SELECTOR_LOG_ONLY: i32 = 18;
const SELECTOR_NEW_STRING: i32 = 21;
const TTBC_FILE_FORMAT_FORMAT: libc::c_int = 10;
const SUP_MAX_STRINGS: i32 = 2_097_151;
const SUP_FONT_MEM_SIZE: i32 = 147_483_647;
const SUP_POOL_SIZE: i32 = 40_000_000;
const SUP_HASH_EXTRA: i32 = 2_097_151;
const UNDEFINED_CS: i32 = 103;
const INTER_CHAR_VAL: i32 = 6;
const AAT_FONT_FLAG: i32 = 0xFFFF;
const OTGR_FONT_FLAG: i32 = 0xFFFE;
const PC_INT_BASE: i32 = 7_826_729;

// ---- moved verbatim from ini.rs (workstream 02) ----

#[inline]
unsafe fn dump_bytes(fmt_out: usize, ptr: *const u8, item_size: usize, nitems: usize) {
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
        _tt_abort(c"could not write format file data".as_ptr());
    }
}
#[inline]
unsafe fn dump_slice<T>(fmt_out: usize, ptr: *const T, nitems: i32) {
    if nitems > 0 {
        dump_bytes(
            fmt_out,
            ptr as *const u8,
            std::mem::size_of::<T>(),
            nitems as usize,
        );
    }
}
#[inline]
unsafe fn dump_one<T: Copy>(fmt_out: usize, value: T) {
    dump_slice(fmt_out, &value as *const T, 1);
}
#[inline]
unsafe fn dump_int(fmt_out: usize, value: i32) {
    dump_one(fmt_out, value);
}
#[inline]
unsafe fn dump_b32(fmt_out: usize, s0: i32, s1: i32) {
    dump_one(fmt_out, crate::mem_types::B32x2 { s0, s1 });
}
#[inline]
unsafe fn undump_bytes(fmt_in: usize, ptr: *mut u8, item_size: usize, nitems: usize) {
    let byte_len = item_size * nitems;
    let read = ttstub_input_read(fmt_in, ptr, byte_len);
    if read < 0 || read as usize != byte_len {
        _tt_abort(c"could not read format file data".as_ptr());
    }

    if cfg!(target_endian = "little") && item_size > 1 {
        let bytes = std::slice::from_raw_parts_mut(ptr, byte_len);
        for item in bytes.chunks_exact_mut(item_size) {
            item.reverse();
        }
    }
}
#[inline]
unsafe fn undump_slice<T>(fmt_in: usize, ptr: *mut T, nitems: i32) {
    if nitems > 0 {
        undump_bytes(
            fmt_in,
            ptr as *mut u8,
            std::mem::size_of::<T>(),
            nitems as usize,
        );
    }
}
#[inline]
unsafe fn undump_one<T: Copy>(fmt_in: usize) -> T {
    let mut value = std::mem::MaybeUninit::<T>::uninit();
    undump_slice(fmt_in, value.as_mut_ptr(), 1);
    value.assume_init()
}
#[inline]
unsafe fn undump_int(fmt_in: usize) -> i32 {
    undump_one(fmt_in)
}
unsafe fn undump_checked_i32(
    fmt_in: usize,
    ptr: *mut i32,
    nitems: i32,
    low: i32,
    high: i32,
) -> bool {
    undump_slice(fmt_in, ptr, nitems);
    for i in 0..nitems {
        let value = *ptr.add(i as usize);
        if value < low || value > high {
            return false;
        }
    }
    true
}
unsafe fn undump_upper_i32(fmt_in: usize, ptr: *mut i32, nitems: i32, high: i32) -> bool {
    undump_slice(fmt_in, ptr, nitems);
    for i in 0..nitems {
        if *ptr.add(i as usize) > high {
            return false;
        }
    }
    true
}
unsafe fn undump_upper_u16(fmt_in: usize, ptr: *mut u16, nitems: i32, high: u16) -> bool {
    undump_slice(fmt_in, ptr, nitems);
    for i in 0..nitems {
        if *ptr.add(i as usize) > high {
            return false;
        }
    }
    true
}
// translated from xetex-ini.c:587 (store_fmt_file)
#[no_mangle]
pub unsafe extern "C" fn store_fmt_file() {
    if crate::globals::save_ptr() != 0 {
        crate::output::error_here_with_diagnostic(c"You can't dump inside a group".as_ptr());
        crate::output::capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(1);
        set_help_line(0, c"`{...\\dump}' is a no-no.".as_ptr());

        if interaction() == ERROR_STOP_MODE {
            set_interaction(SCROLL_MODE);
        }
        if crate::globals::log_opened() {
            crate::errors::error();
        }

        crate::globals::set_history(HISTORY_FATAL_ERROR);
        crate::io::close_files_and_terminate();
        ttstub_output_flush(crate::globals::rust_stdout());
        _tt_abort(c"\\dump inside a group".as_ptr());
    }

    set_selector(SELECTOR_NEW_STRING);
    crate::output::print_cstr(c" (preloaded format=".as_ptr());
    crate::output::print(job_name());
    crate::output::print_char(b' ' as i32);
    crate::output::print_int(intpar(INT_PAR_YEAR));
    crate::output::print_char(b'.' as i32);
    crate::output::print_int(intpar(INT_PAR_MONTH));
    crate::output::print_char(b'.' as i32);
    crate::output::print_int(intpar(INT_PAR_DAY));
    crate::output::print_char(b')' as i32);

    if interaction() == BATCH_MODE {
        set_selector(SELECTOR_LOG_ONLY);
    } else {
        set_selector(SELECTOR_TERM_AND_LOG);
    }

    if pool_ptr() + 1 > pool_size() {
        crate::errors::overflow(c"pool size".as_ptr(), pool_size() - init_pool_ptr());
    }

    set_format_ident(crate::string_pool::make_string());
    crate::io::pack_job_name(c".fmt".as_ptr());

    let fmt_out = ttstub_output_open(crate::globals::name_of_file(), 0);
    if fmt_out == INVALID_HANDLE {
        _tt_abort(c"cannot open format output file".as_ptr());
    }

    crate::output::print_nl_cstr(c"Beginning to dump on file ".as_ptr());
    crate::output::print(crate::io::make_name_string());

    set_str_ptr(str_ptr() - 1);
    set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));

    crate::output::print_nl_cstr(c"".as_ptr());
    crate::output::print(format_ident());

    dump_int(fmt_out, FORMAT_HEADER_MAGIC);
    dump_int(fmt_out, FORMAT_SERIAL);
    dump_int(fmt_out, hash_high());

    while crate::globals::pseudo_files() != crate::mem::TEX_NULL {
        crate::input::pseudo_close();
    }

    dump_int(fmt_out, MEM_TOP);
    dump_int(fmt_out, EQTB_SIZE);
    dump_int(fmt_out, FORMAT_HASH_PRIME);
    dump_int(fmt_out, FORMAT_HYPH_PRIME);

    dump_int(fmt_out, pool_ptr());
    dump_int(fmt_out, str_ptr());
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.strings.str_start.as_ptr(),
            str_ptr() - TOO_BIG_CHAR + 1,
        );
        dump_slice(fmt_out, state.strings.str_pool.as_ptr(), pool_ptr());
    });

    crate::output::print_ln();
    crate::output::print_int(str_ptr());
    crate::output::print_cstr(c" strings of total length ".as_ptr());
    crate::output::print_int(pool_ptr());

    sort_avail();
    set_var_used(0);
    dump_int(fmt_out, lo_mem_max());
    dump_int(fmt_out, rover());

    for k in INT_VAL as usize..=INTER_CHAR_VAL as usize {
        let val = crate::state::with_state(|state| state.scanner.sa_root[k]);
        dump_int(fmt_out, val);
    }

    // `mem` is now a Rust-owned `Vec`; take a stable base pointer once (the
    // thread-local owner never reallocates) so the free-list `dump_slice`
    // calls below — interleaved with `mem_*`/`var_used` accessor reads — hold
    // no borrow. (session 58, the session-51 eqtb pattern.)
    let mem_ptr = crate::state::with_state(|s| s.mem.data.as_ptr());
    let mut p = 0;
    let mut q = rover();
    let mut x = 0;
    loop {
        let len = q + 2 - p;
        dump_slice(fmt_out, mem_ptr.add(p as usize), len);
        x += len;
        set_var_used(var_used() + q - p);
        p = q + mem_b32s0(q);
        q = mem_b32s1(q + 1);
        if q == rover() {
            break;
        }
    }

    set_var_used(var_used() + lo_mem_max() - p);
    set_dyn_used(mem_end() + 1 - hi_mem_min());
    dump_slice(fmt_out, mem_ptr.add(p as usize), lo_mem_max() + 1 - p);
    x += lo_mem_max() + 1 - p;

    dump_int(fmt_out, hi_mem_min());
    dump_int(fmt_out, avail());
    dump_slice(
        fmt_out,
        mem_ptr.add(hi_mem_min() as usize),
        mem_end() + 1 - hi_mem_min(),
    );
    x += mem_end() + 1 - hi_mem_min();

    p = avail();
    while p != crate::mem::TEX_NULL {
        set_dyn_used(dyn_used() - 1);
        p = mem_b32s1(p);
    }

    dump_int(fmt_out, var_used());
    dump_int(fmt_out, dyn_used());

    crate::output::print_ln();
    crate::output::print_int(x);
    crate::output::print_cstr(c" memory locations dumped; current usage is ".as_ptr());
    crate::output::print_int(var_used());
    crate::output::print_char(b'&' as i32);
    crate::output::print_int(dyn_used());

    // `eqtb` is now a Rust-owned `Vec`; take a stable base pointer once (the
    // thread-local owner never reallocates) so the sparse-region `dump_slice`
    // calls below — interleaved with `eqtb_*` accessor reads — hold no borrow.
    let eqtb_ptr = crate::state::with_state(|s| s.eqtb.data.as_ptr());
    let mut k = ACTIVE_BASE;
    loop {
        let mut j = k;
        while j < FORMAT_INT_BASE - 1 {
            if eqtb_b32s1(j) == eqtb_b32s1(j + 1)
                && eqtb_b16s1(j) == eqtb_b16s1(j + 1)
                && eqtb_b16s0(j) == eqtb_b16s0(j + 1)
            {
                break;
            }
            j += 1;
        }
        let l = if j < FORMAT_INT_BASE - 1 {
            j += 1;
            let l = j;
            while j < FORMAT_INT_BASE - 1
                && eqtb_b32s1(j) == eqtb_b32s1(j + 1)
                && eqtb_b16s1(j) == eqtb_b16s1(j + 1)
                && eqtb_b16s0(j) == eqtb_b16s0(j + 1)
            {
                j += 1;
            }
            l
        } else {
            FORMAT_INT_BASE
        };

        dump_int(fmt_out, l - k);
        dump_slice(fmt_out, eqtb_ptr.add(k as usize), l - k);
        k = j + 1;
        dump_int(fmt_out, k - l);
        if k == FORMAT_INT_BASE {
            break;
        }
    }

    loop {
        let mut j = k;
        while j < EQTB_SIZE {
            if eqtb_b32s1(j) == eqtb_b32s1(j + 1) {
                break;
            }
            j += 1;
        }
        let l = if j < EQTB_SIZE {
            j += 1;
            let l = j;
            while j < EQTB_SIZE && eqtb_b32s1(j) == eqtb_b32s1(j + 1) {
                j += 1;
            }
            l
        } else {
            EQTB_SIZE + 1
        };

        dump_int(fmt_out, l - k);
        dump_slice(fmt_out, eqtb_ptr.add(k as usize), l - k);
        k = j + 1;
        dump_int(fmt_out, k - l);
        if k > EQTB_SIZE {
            break;
        }
    }

    if hash_high() > 0 {
        dump_slice(fmt_out, eqtb_ptr.add((EQTB_SIZE + 1) as usize), hash_high());
    }

    dump_int(fmt_out, par_loc());
    dump_int(fmt_out, write_loc());

    for p in 0..=PRIM_SIZE {
        dump_b32(
            fmt_out,
            crate::globals::prim_s0(p),
            crate::globals::prim_s1(p),
        );
    }

    dump_int(fmt_out, hash_used());
    set_cs_count((FROZEN_CONTROL_SEQUENCE - 1) - hash_used() + hash_high());

    for p in FORMAT_HASH_BASE..=hash_used() {
        if crate::globals::hash_s1(p) != 0 {
            dump_int(fmt_out, p);
            dump_b32(
                fmt_out,
                crate::globals::hash_s0(p),
                crate::globals::hash_s1(p),
            );
            set_cs_count(cs_count() + 1);
        }
    }

    for p in (hash_used() + 1)..=(UNDEFINED_CONTROL_SEQUENCE - 1) {
        dump_b32(
            fmt_out,
            crate::globals::hash_s0(p),
            crate::globals::hash_s1(p),
        );
    }
    if hash_high() > 0 {
        for p in (EQTB_SIZE + 1)..=(EQTB_SIZE + hash_high()) {
            dump_b32(
                fmt_out,
                crate::globals::hash_s0(p),
                crate::globals::hash_s1(p),
            );
        }
    }

    dump_int(fmt_out, cs_count());

    crate::output::print_ln();
    crate::output::print_int(cs_count());
    crate::output::print_cstr(c" multiletter control sequences".as_ptr());

    let fmp = fmem_ptr();
    dump_int(fmt_out, fmp);
    crate::state::with_state(|s| {
        dump_slice(fmt_out, s.fonts.font_info.as_ptr(), fmp);
    });
    dump_int(fmt_out, font_ptr());
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.fonts.font_check.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.fonts.font_size.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
        dump_slice(
            fmt_out,
            state.fonts.font_dsize.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.fonts.font_params.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.fonts.hyphen_char.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
        dump_slice(
            fmt_out,
            state.fonts.skew_char.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.fonts.font_name.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.fonts.font_area.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.fonts.font_bc.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.fonts.font_ec.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });
    let font_dump_count = font_ptr() + 1;
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.fonts.char_base.as_ptr().add(FONT_BASE as usize),
            font_dump_count,
        );
        dump_slice(
            fmt_out,
            state.fonts.width_base.as_ptr().add(FONT_BASE as usize),
            font_dump_count,
        );
        dump_slice(
            fmt_out,
            state.fonts.height_base.as_ptr().add(FONT_BASE as usize),
            font_dump_count,
        );
        dump_slice(
            fmt_out,
            state.fonts.depth_base.as_ptr().add(FONT_BASE as usize),
            font_dump_count,
        );
        dump_slice(
            fmt_out,
            state.fonts.italic_base.as_ptr().add(FONT_BASE as usize),
            font_dump_count,
        );
        dump_slice(
            fmt_out,
            state.fonts.lig_kern_base.as_ptr().add(FONT_BASE as usize),
            font_dump_count,
        );
        dump_slice(
            fmt_out,
            state.fonts.kern_base.as_ptr().add(FONT_BASE as usize),
            font_dump_count,
        );
        dump_slice(
            fmt_out,
            state.fonts.exten_base.as_ptr().add(FONT_BASE as usize),
            font_dump_count,
        );
        dump_slice(
            fmt_out,
            state.fonts.param_base.as_ptr().add(FONT_BASE as usize),
            font_dump_count,
        );
    });
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.fonts.font_glue.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.fonts.bchar_label.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.fonts.font_bchar.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state
                .fonts
                .font_false_bchar
                .as_ptr()
                .add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });

    for k in FONT_BASE..=font_ptr() {
        crate::output::print_nl_cstr(c"\\font".as_ptr());
        crate::output::print_esc(crate::globals::hash_s1(FONT_ID_BASE + k));
        crate::output::print_char(b'=' as i32);

        if font_area_entry(k) == AAT_FONT_FLAG
            || font_area_entry(k) == OTGR_FONT_FLAG
            || !font_mapping_ptr(k).is_null()
        {
            crate::output::print_file_name(font_name_entry(k), EMPTY_STRING, EMPTY_STRING);
            crate::output::error_here_with_diagnostic(
                c"Can't \\dump a format with native fonts or font-mappings".as_ptr(),
            );
            crate::output::capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(3);
            set_help_line(2, c"You really, really don't want to do this.".as_ptr());
            set_help_line(1, c"It won't work, and only confuses me.".as_ptr());
            set_help_line(
                0,
                c"(Load them at runtime, not as part of the format file.)".as_ptr(),
            );
            crate::errors::error();
        } else {
            crate::output::print_file_name(font_name_entry(k), font_area_entry(k), EMPTY_STRING);
        }

        if font_size_entry(k) != font_dsize_entry(k) {
            crate::output::print_cstr(c" at ".as_ptr());
            crate::output::print_scaled(font_size_entry(k));
            crate::output::print_cstr(c"pt".as_ptr());
        }
    }

    crate::output::print_ln();
    crate::output::print_int(fmem_ptr() - 7);
    crate::output::print_cstr(c" words of font info for ".as_ptr());
    crate::output::print_int(font_ptr());
    if font_ptr() != FONT_BASE + 1 {
        crate::output::print_cstr(c" preloaded fonts".as_ptr());
    } else {
        crate::output::print_cstr(c" preloaded font".as_ptr());
    }

    dump_int(fmt_out, hyph_count());
    if hyph_next() <= FORMAT_HYPH_PRIME {
        set_hyph_next(hyph_size());
    }
    dump_int(fmt_out, hyph_next());

    for k in 0..=hyph_size() {
        if hyph_word_entry(k) != 0 {
            dump_int(fmt_out, k + 65_536 * hyph_link_entry(k) as i32);
            dump_int(fmt_out, hyph_word_entry(k));
            dump_int(fmt_out, hyph_list_entry(k));
        }
    }

    crate::output::print_ln();
    crate::output::print_int(hyph_count());
    if hyph_count() != 1 {
        crate::output::print_cstr(c" hyphenation exceptions".as_ptr());
    } else {
        crate::output::print_cstr(c" hyphenation exception".as_ptr());
    }

    if trie_not_ready() {
        init_trie();
    }

    dump_int(fmt_out, trie_max());
    dump_int(fmt_out, hyph_start());
    let fmt_trie_trl = crate::state::with_state(|state| state.hyph.trie_trl.as_ptr());
    let fmt_trie_tro = crate::state::with_state(|state| state.hyph.trie_tro.as_ptr());
    let fmt_trie_trc = crate::state::with_state(|state| state.hyph.trie_trc.as_ptr());
    dump_slice(fmt_out, fmt_trie_trl, trie_max() + 1);
    dump_slice(fmt_out, fmt_trie_tro, trie_max() + 1);
    dump_slice(fmt_out, fmt_trie_trc, trie_max() + 1);
    dump_int(fmt_out, max_hyph_char());
    dump_int(fmt_out, trie_op_ptr());
    let fmt_hyf_distance = crate::state::with_state(|state| state.hyph.hyf_distance.as_ptr());
    let fmt_hyf_num = crate::state::with_state(|state| state.hyph.hyf_num.as_ptr());
    let fmt_hyf_next = crate::state::with_state(|state| state.hyph.hyf_next.as_ptr());
    dump_slice(fmt_out, fmt_hyf_distance.add(1), trie_op_ptr());
    dump_slice(fmt_out, fmt_hyf_num.add(1), trie_op_ptr());
    dump_slice(fmt_out, fmt_hyf_next.add(1), trie_op_ptr());

    crate::output::print_nl_cstr(c"Hyphenation trie of length ".as_ptr());
    crate::output::print_int(trie_max());
    crate::output::print_cstr(c" has ".as_ptr());
    crate::output::print_int(trie_op_ptr());
    if trie_op_ptr() != 1 {
        crate::output::print_cstr(c" ops".as_ptr());
    } else {
        crate::output::print_cstr(c" op".as_ptr());
    }
    crate::output::print_cstr(c" out of ".as_ptr());
    crate::output::print_int(TRIE_OP_SIZE);

    for k in (0..=BIGGEST_LANG as usize).rev() {
        let used = trie_used_entry(k);
        if used > 0 {
            crate::output::print_nl_cstr(c"  ".as_ptr());
            crate::output::print_int(used as i32);
            crate::output::print_cstr(c" for language ".as_ptr());
            crate::output::print_int(k as i32);
            dump_int(fmt_out, k as i32);
            dump_int(fmt_out, used as i32);
        }
    }

    dump_int(fmt_out, FORMAT_FOOTER_MAGIC);
    set_eqtb_b32s1(PC_INT_BASE + INT_PAR_TRACING_STATS, 0);
    ttstub_output_close(fmt_out);
}
// translated from xetex-ini.c:590 (pack_buffered_name)
unsafe fn pack_buffered_name(_n: i16, _a: i32, _b: i32) {
    crate::globals::set_name_of_file(tre_TEX_format_default as *mut libc::c_char);
    crate::globals::set_name_length(libc::strlen(tre_TEX_format_default) as i32);
}
#[inline]
unsafe fn bad_format_file() -> ! {
    _tt_abort(c"fatal format file error".as_ptr());
}
// translated from xetex-ini.c:602 (load_fmt_file)
#[no_mangle]
pub unsafe extern "C" fn load_fmt_file() -> bool {
    let mut j: i32;
    let mut k: i32;
    let mut p: i32;
    let mut q: i32;
    let mut x: i32;

    j = cur_input().loc;
    pack_buffered_name((tre_format_default_length - 4) as i16, 1, 0);

    let fmt_in = ttstub_input_open(crate::globals::name_of_file(), TTBC_FILE_FORMAT_FORMAT, 0);
    if fmt_in == INVALID_HANDLE {
        _tt_abort(c"cannot open the format file".as_ptr());
    }

    set_cur_input_loc(j);

    if in_initex_mode() {
        crate::state::with_state_mut(|state| {
            state.fonts.font_info = Vec::new();
            state.strings.str_pool = Vec::new();
            state.strings.str_start = Vec::new();
            state.eqtb.data = Vec::new();
            state.eqtb.hash = Vec::new();
            state.mem.data = Vec::new();
        });
    }

    x = undump_int(fmt_in);
    if x != FORMAT_HEADER_MAGIC {
        bad_format_file();
    }

    x = undump_int(fmt_in);
    if x != FORMAT_SERIAL {
        _tt_abort(c"format file is of the wrong version".as_ptr());
    }

    x = undump_int(fmt_in);
    if !(0..=SUP_HASH_EXTRA).contains(&x) {
        bad_format_file();
    }
    set_hash_high(x);
    let hash_high_val = hash_high();
    crate::state::with_state_mut(|s| {
        if s.eqtb.hash_extra < hash_high_val {
            s.eqtb.hash_extra = hash_high_val;
        }
        s.eqtb.eqtb_top = EQTB_SIZE + s.eqtb.hash_extra;
        s.eqtb.hash_top = if s.eqtb.hash_extra == 0 {
            UNDEFINED_CONTROL_SEQUENCE
        } else {
            s.eqtb.eqtb_top
        };
        let n = (1 + s.eqtb.hash_top - 514) as usize;
        s.eqtb.hash = vec![crate::mem_types::B32x2 { s0: 0, s1: 0 }; n];
    });

    crate::globals::set_hash_s0(FORMAT_HASH_BASE, 0);
    crate::globals::set_hash_s1(FORMAT_HASH_BASE, 0);
    for h in (FORMAT_HASH_BASE + 1)..=crate::globals::hash_top() {
        crate::globals::set_hash_s0(h, crate::globals::hash_s0(FORMAT_HASH_BASE));
        crate::globals::set_hash_s1(h, crate::globals::hash_s1(FORMAT_HASH_BASE));
    }

    // `xmalloc_array(n)` allocated `n + 1` words; match that inclusive size.
    let eqtb_words_load = (crate::globals::eqtb_top() as usize) + 1;
    crate::state::with_state_mut(|s| {
        s.eqtb.data = vec![
            crate::mem_types::MemoryWord {
                b32: crate::mem_types::B32x2 { s0: 0, s1: 0 }
            };
            eqtb_words_load
        ];
    });
    set_eqtb_b16s1(UNDEFINED_CONTROL_SEQUENCE, UNDEFINED_CS as u16);
    set_eqtb_b32s1(UNDEFINED_CONTROL_SEQUENCE, crate::mem::TEX_NULL);
    set_eqtb_b16s0(UNDEFINED_CONTROL_SEQUENCE, 0);
    let undefined_entry = eqtb_word(UNDEFINED_CONTROL_SEQUENCE);
    for e in (EQTB_SIZE + 1)..=crate::globals::eqtb_top() {
        set_eqtb_word(e, undefined_entry);
    }

    set_max_reg_num(32_767);
    set_max_reg_help_line(c"A register number must be between 0 and 32767.".as_ptr());

    x = undump_int(fmt_in);
    if x != MEM_TOP {
        bad_format_file();
    }

    set_cur_list_head(CONTRIB_HEAD);
    set_cur_list_tail(CONTRIB_HEAD);
    set_page_tail(PAGE_HEAD);
    // `xmalloc_array(MEM_TOP)` allocated `MEM_TOP + 1` words; match it.
    let mem_words_load = (MEM_TOP as usize) + 1;
    crate::state::with_state_mut(|s| {
        s.mem.data = vec![
            crate::mem_types::MemoryWord {
                b32: crate::mem_types::B32x2 { s0: 0, s1: 0 }
            };
            mem_words_load
        ];
    });

    x = undump_int(fmt_in);
    if x != EQTB_SIZE {
        bad_format_file();
    }
    x = undump_int(fmt_in);
    if x != FORMAT_HASH_PRIME {
        bad_format_file();
    }
    x = undump_int(fmt_in);
    if x != FORMAT_HYPH_PRIME {
        bad_format_file();
    }

    x = undump_int(fmt_in);
    if x < 0 {
        bad_format_file();
    }
    if x > SUP_POOL_SIZE - tre_pool_free {
        _tt_abort(c"must increase string_pool_size".as_ptr());
    }
    set_pool_ptr(x);
    if TRE_POOL_SIZE < pool_ptr() + tre_pool_free {
        TRE_POOL_SIZE = pool_ptr() + tre_pool_free;
    }
    crate::state::with_state_mut(|state| {
        state.strings.pool_size = TRE_POOL_SIZE;
    });

    x = undump_int(fmt_in);
    if x < 0 {
        bad_format_file();
    }
    if x > SUP_MAX_STRINGS - tre_strings_free {
        _tt_abort(c"must increase sup_strings".as_ptr());
    }
    set_str_ptr(x);
    if TRE_MAX_STRINGS < str_ptr() + tre_strings_free {
        TRE_MAX_STRINGS = str_ptr() + tre_strings_free;
    }
    crate::state::with_state_mut(|state| {
        state.strings.max_strings = TRE_MAX_STRINGS;
    });

    crate::state::with_state_mut(|state| {
        state.strings.str_start = vec![0i32; (TRE_MAX_STRINGS as usize) + 1];
    });
    let n = str_ptr() - TOO_BIG_CHAR + 1;
    let limit = pool_ptr();
    if !crate::state::with_state_mut(|state| {
        undump_checked_i32(fmt_in, state.strings.str_start.as_mut_ptr(), n, 0, limit)
    }) {
        bad_format_file();
    }
    crate::state::with_state_mut(|state| {
        state.strings.str_pool = vec![0u16; (TRE_POOL_SIZE as usize) + 1];
    });
    let limit = pool_ptr();
    crate::state::with_state_mut(|state| {
        undump_slice(fmt_in, state.strings.str_pool.as_mut_ptr(), limit);
    });

    crate::state::with_state_mut(|state| {
        state.strings.init_str_ptr = state.strings.str_ptr;
        state.strings.init_pool_ptr = state.strings.pool_ptr;
    });

    x = undump_int(fmt_in);
    if !(1019..=MEM_TOP - HI_MEM_STAT_USAGE).contains(&x) {
        bad_format_file();
    }
    set_lo_mem_max(x);

    x = undump_int(fmt_in);
    if x < 20 || x > lo_mem_max() {
        bad_format_file();
    }
    set_rover(x);

    for idx in INT_VAL..=INTER_CHAR_VAL {
        x = undump_int(fmt_in);
        if x < MIN_HALFWORD || x > lo_mem_max() {
            bad_format_file();
        }
        crate::state::with_state_mut(|state| state.scanner.sa_root[idx as usize] = x);
    }

    // `mem` is now a Rust-owned `Vec`; take a stable mutable base pointer once
    // (never reallocated) for the free-list `undump_slice` calls — interleaved
    // with `mem_*` accessor reads, so it holds no borrow. (session 58)
    let mem_ptr = crate::state::with_state_mut(|s| s.mem.data.as_mut_ptr());
    p = 0;
    q = rover();
    loop {
        undump_slice(fmt_in, mem_ptr.add(p as usize), q + 2 - p);
        p = q + mem_b32s0(q);
        if p > lo_mem_max() || (q >= mem_b32s1(q + 1) && mem_b32s1(q + 1) != rover()) {
            bad_format_file();
        }
        q = mem_b32s1(q + 1);
        if q == rover() {
            break;
        }
    }

    undump_slice(fmt_in, mem_ptr.add(p as usize), lo_mem_max() + 1 - p);

    x = undump_int(fmt_in);
    if x < lo_mem_max() + 1 || x > PRE_ADJUST_HEAD {
        bad_format_file();
    }
    set_hi_mem_min(x);

    x = undump_int(fmt_in);
    if !(MIN_HALFWORD..=MEM_TOP).contains(&x) {
        bad_format_file();
    }
    set_avail(x);

    set_mem_end(MEM_TOP);
    undump_slice(
        fmt_in,
        mem_ptr.add(hi_mem_min() as usize),
        mem_end() + 1 - hi_mem_min(),
    );
    set_var_used(undump_int(fmt_in));
    set_dyn_used(undump_int(fmt_in));

    // `eqtb` is now a Rust-owned `Vec`; take a stable mutable base pointer once
    // (never reallocated) for the sparse-region `undump_slice`/replication below.
    let eqtb_ptr = crate::state::with_state_mut(|s| s.eqtb.data.as_mut_ptr());
    k = ACTIVE_BASE;
    loop {
        x = undump_int(fmt_in);
        if x < 1 || k + x > EQTB_SIZE + 1 {
            bad_format_file();
        }

        undump_slice(fmt_in, eqtb_ptr.add(k as usize), x);
        k += x;

        x = undump_int(fmt_in);
        if x < 0 || k + x > EQTB_SIZE + 1 {
            bad_format_file();
        }

        for j1 in k..=(k + x - 1) {
            *eqtb_ptr.add(j1 as usize) = *eqtb_ptr.add((k - 1) as usize);
        }
        k += x;
        if k > EQTB_SIZE {
            break;
        }
    }

    if hash_high() > 0 {
        undump_slice(fmt_in, eqtb_ptr.add((EQTB_SIZE + 1) as usize), hash_high());
    }

    x = undump_int(fmt_in);
    if x < FORMAT_HASH_BASE || x > crate::globals::hash_top() {
        bad_format_file();
    }
    set_par_loc(x);
    set_par_token(CS_TOKEN_FLAG + par_loc());

    x = undump_int(fmt_in);
    if x < FORMAT_HASH_BASE || x > crate::globals::hash_top() {
        bad_format_file();
    }
    set_write_loc(x);

    for prim_idx in 0..=PRIM_SIZE {
        let value: crate::mem_types::B32x2 = undump_one(fmt_in);
        set_prim_s0(prim_idx, value.s0);
        set_prim_s1(prim_idx, value.s1);
    }

    x = undump_int(fmt_in);
    if !(FORMAT_HASH_BASE..=FROZEN_CONTROL_SEQUENCE).contains(&x) {
        bad_format_file();
    }
    set_hash_used(x);

    p = FORMAT_HASH_BASE - 1;
    loop {
        x = undump_int(fmt_in);
        if x < p + 1 || x > hash_used() {
            bad_format_file();
        }
        p = x;
        let value: crate::mem_types::B32x2 = undump_one(fmt_in);
        crate::globals::set_hash_s0(p, value.s0);
        crate::globals::set_hash_s1(p, value.s1);
        if p == hash_used() {
            break;
        }
    }

    let hash_ptr = crate::state::with_state_mut(|s| s.eqtb.hash.as_mut_ptr());
    undump_slice(
        fmt_in,
        hash_ptr.add((hash_used() + 1 - 514) as usize),
        (UNDEFINED_CONTROL_SEQUENCE - 1) - hash_used(),
    );
    if hash_high() > 0 {
        undump_slice(
            fmt_in,
            hash_ptr.add((EQTB_SIZE + 1 - 514) as usize),
            hash_high(),
        );
    }

    set_cs_count(undump_int(fmt_in));

    x = undump_int(fmt_in);
    if x < 7 {
        bad_format_file();
    }
    if x > SUP_FONT_MEM_SIZE {
        _tt_abort(c"must increase font_mem_size".as_ptr());
    }
    set_fmem_ptr(x);
    if fmem_ptr() > font_mem_size() {
        set_font_mem_size(fmem_ptr());
    }

    // pre-compute before mutable borrow (both backed by with_state)
    let fms = font_mem_size();
    let fmp = fmem_ptr();
    let zero_word = crate::mem_types::MemoryWord {
        b32: crate::mem_types::B32x2 { s0: 0, s1: 0 },
    };
    crate::state::with_state_mut(|s| {
        s.fonts.font_info = vec![zero_word; (fms as usize) + 1];
        undump_slice(fmt_in, s.fonts.font_info.as_mut_ptr(), fmp);
    });

    x = undump_int(fmt_in);
    if x < FONT_BASE {
        bad_format_file();
    }
    if x > FONT_BASE + MAX_FONT_MAX {
        _tt_abort(c"must increase font_max".as_ptr());
    }
    set_font_ptr(x);

    let font_check_len = (font_max() as usize) + 1;
    crate::state::with_state_mut(|state| {
        state.fonts.font_layout_engine = vec![std::ptr::null_mut(); font_check_len];
        state.fonts.font_mapping = vec![std::ptr::null_mut(); font_check_len];
        state.fonts.font_flags = vec![0i8; font_check_len];
        state.fonts.font_letter_space = vec![0i32; font_check_len];
        state.fonts.font_check = vec![
            crate::mem_types::B16x4 {
                s0: 0,
                s1: 0,
                s2: 0,
                s3: 0,
            };
            font_check_len
        ];
        state.fonts.font_size = vec![0i32; font_check_len];
        state.fonts.font_dsize = vec![0i32; font_check_len];
        state.fonts.font_params = vec![0i32; font_check_len];
        state.fonts.font_name = vec![0i32; font_check_len];
        state.fonts.font_area = vec![0i32; font_check_len];
        state.fonts.font_bc = vec![0u16; font_check_len];
        state.fonts.font_ec = vec![0u16; font_check_len];
        state.fonts.bchar_label = vec![0i32; font_check_len];
        state.fonts.font_bchar = vec![0i32; font_check_len];
        state.fonts.font_false_bchar = vec![0i32; font_check_len];
        state.fonts.font_glue = vec![0i32; font_check_len];
    });
    let base_arr_len = (font_max() as usize) + 1;
    crate::state::with_state_mut(|state| {
        state.fonts.hyphen_char = vec![0i32; base_arr_len];
        state.fonts.skew_char = vec![0i32; base_arr_len];
        state.fonts.char_base = vec![0i32; base_arr_len];
        state.fonts.width_base = vec![0i32; base_arr_len];
        state.fonts.height_base = vec![0i32; base_arr_len];
        state.fonts.depth_base = vec![0i32; base_arr_len];
        state.fonts.italic_base = vec![0i32; base_arr_len];
        state.fonts.lig_kern_base = vec![0i32; base_arr_len];
        state.fonts.kern_base = vec![0i32; base_arr_len];
        state.fonts.exten_base = vec![0i32; base_arr_len];
        state.fonts.param_base = vec![0i32; base_arr_len];
    });

    let font_undump_count = font_ptr() + 1;
    crate::state::with_state_mut(|state| {
        undump_slice(
            fmt_in,
            state.fonts.font_check.as_mut_ptr().add(FONT_BASE as usize),
            font_undump_count,
        );
    });
    crate::state::with_state_mut(|state| {
        undump_slice(
            fmt_in,
            state.fonts.font_size.as_mut_ptr().add(FONT_BASE as usize),
            font_undump_count,
        );
        undump_slice(
            fmt_in,
            state.fonts.font_dsize.as_mut_ptr().add(FONT_BASE as usize),
            font_undump_count,
        );
    });
    if !crate::state::with_state_mut(|state| {
        undump_checked_i32(
            fmt_in,
            state.fonts.font_params.as_mut_ptr().add(FONT_BASE as usize),
            font_undump_count,
            MIN_HALFWORD,
            MAX_HALFWORD,
        )
    }) {
        bad_format_file();
    }
    crate::state::with_state_mut(|state| {
        undump_slice(
            fmt_in,
            state.fonts.hyphen_char.as_mut_ptr().add(FONT_BASE as usize),
            font_undump_count,
        );
        undump_slice(
            fmt_in,
            state.fonts.skew_char.as_mut_ptr().add(FONT_BASE as usize),
            font_undump_count,
        );
    });
    let font_count = font_undump_count;
    let max_str = str_ptr();
    if !crate::state::with_state_mut(|state| {
        undump_upper_i32(
            fmt_in,
            state.fonts.font_name.as_mut_ptr().add(FONT_BASE as usize),
            font_count,
            max_str,
        )
    }) {
        bad_format_file();
    }
    if !crate::state::with_state_mut(|state| {
        undump_upper_i32(
            fmt_in,
            state.fonts.font_area.as_mut_ptr().add(FONT_BASE as usize),
            font_count,
            max_str,
        )
    }) {
        bad_format_file();
    }
    crate::state::with_state_mut(|state| {
        undump_slice(
            fmt_in,
            state.fonts.font_bc.as_mut_ptr().add(FONT_BASE as usize),
            font_count,
        );
    });
    crate::state::with_state_mut(|state| {
        undump_slice(
            fmt_in,
            state.fonts.font_ec.as_mut_ptr().add(FONT_BASE as usize),
            font_count,
        );
    });
    let base_undump_count = font_ptr() + 1;
    crate::state::with_state_mut(|state| {
        undump_slice(
            fmt_in,
            state.fonts.char_base.as_mut_ptr().add(FONT_BASE as usize),
            base_undump_count,
        );
        undump_slice(
            fmt_in,
            state.fonts.width_base.as_mut_ptr().add(FONT_BASE as usize),
            base_undump_count,
        );
        undump_slice(
            fmt_in,
            state.fonts.height_base.as_mut_ptr().add(FONT_BASE as usize),
            base_undump_count,
        );
        undump_slice(
            fmt_in,
            state.fonts.depth_base.as_mut_ptr().add(FONT_BASE as usize),
            base_undump_count,
        );
        undump_slice(
            fmt_in,
            state.fonts.italic_base.as_mut_ptr().add(FONT_BASE as usize),
            base_undump_count,
        );
        undump_slice(
            fmt_in,
            state
                .fonts
                .lig_kern_base
                .as_mut_ptr()
                .add(FONT_BASE as usize),
            base_undump_count,
        );
        undump_slice(
            fmt_in,
            state.fonts.kern_base.as_mut_ptr().add(FONT_BASE as usize),
            base_undump_count,
        );
        undump_slice(
            fmt_in,
            state.fonts.exten_base.as_mut_ptr().add(FONT_BASE as usize),
            base_undump_count,
        );
        undump_slice(
            fmt_in,
            state.fonts.param_base.as_mut_ptr().add(FONT_BASE as usize),
            base_undump_count,
        );
    });
    let font_glue_count = font_ptr() + 1;
    let font_glue_max = lo_mem_max();
    if !crate::state::with_state_mut(|state| {
        undump_checked_i32(
            fmt_in,
            state.fonts.font_glue.as_mut_ptr().add(FONT_BASE as usize),
            font_glue_count,
            MIN_HALFWORD,
            font_glue_max,
        )
    }) {
        bad_format_file();
    }
    let max_font_index = fmem_ptr() - 1;
    if !crate::state::with_state_mut(|state| {
        undump_checked_i32(
            fmt_in,
            state.fonts.bchar_label.as_mut_ptr().add(FONT_BASE as usize),
            font_count,
            0,
            max_font_index,
        )
    }) {
        bad_format_file();
    }
    if !crate::state::with_state_mut(|state| {
        undump_checked_i32(
            fmt_in,
            state.fonts.font_bchar.as_mut_ptr().add(FONT_BASE as usize),
            font_count,
            0,
            TOO_BIG_CHAR,
        )
    }) {
        bad_format_file();
    }
    if !crate::state::with_state_mut(|state| {
        undump_checked_i32(
            fmt_in,
            state
                .fonts
                .font_false_bchar
                .as_mut_ptr()
                .add(FONT_BASE as usize),
            font_count,
            0,
            TOO_BIG_CHAR,
        )
    }) {
        bad_format_file();
    }

    x = undump_int(fmt_in);
    if x < 0 {
        bad_format_file();
    }
    if x > hyph_size() {
        _tt_abort(c"must increase hyph_size".as_ptr());
    }
    set_hyph_count(x);

    x = undump_int(fmt_in);
    if x < FORMAT_HYPH_PRIME {
        bad_format_file();
    }
    if x > hyph_size() {
        _tt_abort(c"must increase hyph_size".as_ptr());
    }
    set_hyph_next(x);

    j = 0;
    for _ in 1..=hyph_count() {
        j = undump_int(fmt_in);
        if j < 0 {
            bad_format_file();
        }
        if j > 65_535 {
            set_hyph_next(j / 65_536);
            j -= hyph_next() * 65_536;
        } else {
            set_hyph_next(0);
        }

        if j >= hyph_size() || hyph_next() > hyph_size() {
            bad_format_file();
        }
        set_hyph_link_entry(j, hyph_next() as u16);

        x = undump_int(fmt_in);
        if x < 0 || x > str_ptr() {
            bad_format_file();
        }
        set_hyph_word_entry(j, x);

        x = undump_int(fmt_in);
        if !(MIN_HALFWORD..=MAX_HALFWORD).contains(&x) {
            bad_format_file();
        }
        set_hyph_list_entry(j, x);
    }

    j += 1;
    if j < FORMAT_HYPH_PRIME {
        j = FORMAT_HYPH_PRIME;
    }
    set_hyph_next(j);
    if hyph_next() >= hyph_size() {
        set_hyph_next(FORMAT_HYPH_PRIME);
    } else if hyph_next() >= FORMAT_HYPH_PRIME {
        set_hyph_next(hyph_next() + 1);
    }

    x = undump_int(fmt_in);
    if x < 0 {
        bad_format_file();
    }
    if x > trie_size() {
        _tt_abort(c"must increase trie_size".as_ptr());
    }
    j = x;
    set_trie_max(j);

    x = undump_int(fmt_in);
    if x < 0 || x > j {
        bad_format_file();
    }
    set_hyph_start(x);

    crate::state::with_state_mut(|state| {
        if state.hyph.trie_trl.is_empty() {
            state.hyph.trie_trl = vec![0i32; (j as usize) + 1];
        }
        undump_slice(fmt_in, state.hyph.trie_trl.as_mut_ptr(), j + 1);
        if state.hyph.trie_tro.is_empty() {
            state.hyph.trie_tro = vec![0i32; (j as usize) + 1];
        }
        undump_slice(fmt_in, state.hyph.trie_tro.as_mut_ptr(), j + 1);
        if state.hyph.trie_trc.is_empty() {
            state.hyph.trie_trc = vec![0u16; (j as usize) + 1];
        }
        undump_slice(fmt_in, state.hyph.trie_trc.as_mut_ptr(), j + 1);
    });

    set_max_hyph_char(undump_int(fmt_in));

    x = undump_int(fmt_in);
    if x < 0 {
        bad_format_file();
    }
    if x > TRIE_OP_SIZE {
        _tt_abort(c"must increase TRIE_OP_SIZE".as_ptr());
    }
    j = x;
    set_trie_op_ptr(j);

    let fmt_hyf_distance =
        crate::state::with_state_mut(|state| state.hyph.hyf_distance.as_mut_ptr());
    let fmt_hyf_num = crate::state::with_state_mut(|state| state.hyph.hyf_num.as_mut_ptr());
    let fmt_hyf_next = crate::state::with_state_mut(|state| state.hyph.hyf_next.as_mut_ptr());
    undump_slice(fmt_in, fmt_hyf_distance.add(1), j);
    undump_slice(fmt_in, fmt_hyf_num.add(1), j);
    if !undump_upper_u16(fmt_in, fmt_hyf_next.add(1), j, MAX_TRIE_OP) {
        bad_format_file();
    }

    for lang in 0..=BIGGEST_LANG as usize {
        set_trie_used_entry(lang, 0);
    }

    k = BIGGEST_LANG + 1;
    while j > 0 {
        x = undump_int(fmt_in);
        if x < 0 || x > k - 1 {
            bad_format_file();
        }
        k = x;

        x = undump_int(fmt_in);
        if x < 1 || x > j {
            bad_format_file();
        }
        set_trie_used_entry(k as usize, x as u16);
        j -= x;
        set_op_start_entry(k as usize, j);
    }

    set_trie_not_ready(false);

    x = undump_int(fmt_in);
    if x != FORMAT_FOOTER_MAGIC {
        bad_format_file();
    }

    ttstub_input_close(fmt_in);
    true
}
