//! Residual init helper from xetex-ini.c. The trie build moved to
//! `hyphenation.rs`, the format dump/load to `format_io.rs`, and the
//! init/run/cleanup driver to `engine.rs`; only `init_io` remains here
//! pending its relocation to `io.rs`.

use crate::globals::{
    first, last, set_buffer_entry, set_cur_input_limit, set_cur_input_loc, set_first,
    set_input_file, set_last,
};
use crate::io::UFile;

const INVALID_HANDLE: usize = 0;
const UTF8: i16 = 1;

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
