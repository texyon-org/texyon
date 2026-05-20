//! Rust translation of initialization helpers from xetex-ini.c.

use crate::globals::{
    first, last, pool_size, set_buffer_entry, set_cur_input_limit, set_cur_input_loc, set_first,
    set_input_file, set_last, set_pool_ptr, set_str_ptr, set_str_start, string_vacancies,
};
use crate::io::UFile;

const INVALID_HANDLE: usize = 0;
const UTF8: i16 = 1;
const TOO_BIG_CHAR: i32 = 65_536;

extern "C" {
    fn _tt_abort(fmt: *const libc::c_char) -> !;
}

static mut STDIN_UFILE: UFile = UFile {
    handle: INVALID_HANDLE,
    saved_char: -1,
    skip_next_lf: 0,
    encoding_mode: UTF8,
    conversion_data: std::ptr::null_mut(),
};

// translated from xetex-ini.c:3035 (init_io)
#[no_mangle]
pub unsafe extern "C" fn init_io() {
    // This is largely vestigial at this point.
    STDIN_UFILE.handle = INVALID_HANDLE;
    STDIN_UFILE.saved_char = -1;
    STDIN_UFILE.skip_next_lf = 0;
    STDIN_UFILE.encoding_mode = UTF8;
    STDIN_UFILE.conversion_data = std::ptr::null_mut();
    set_input_file(0, (&raw mut STDIN_UFILE).cast());

    set_buffer_entry(first(), 0);
    set_last(first());
    set_cur_input_loc(first());
    set_cur_input_limit(last());
    set_first(last() + 1);
}

// translated from xetex-ini.c:3452 (get_strings_started)
#[no_mangle]
pub unsafe extern "C" fn get_strings_started() {
    set_pool_ptr(0);
    set_str_ptr(0);
    set_str_start(0, 0);
    set_str_ptr(TOO_BIG_CHAR);

    if crate::string_pool::load_pool_strings(pool_size() - string_vacancies()) == 0 {
        _tt_abort(c"must increase pool_size".as_ptr());
    }
}
