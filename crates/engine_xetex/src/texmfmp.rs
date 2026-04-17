//! Miscellaneous C-callable helpers from xetex-texmfmp.c.
//!
//! Functions involving static C state (`last_source_name`, `start_time_str`),
//! `makepdftime`, or complex bridge calls remain in C for now.

/// Fill in date/time fields from a Unix timestamp (seconds since epoch).
///
/// translated from xetex-texmfmp.c:116 (get_date_and_time)
#[no_mangle]
pub unsafe extern "C" fn get_date_and_time(
    source_date_epoch: libc::time_t,
    minutes: *mut i32,
    day: *mut i32,
    month: *mut i32,
    year: *mut i32,
) {
    // SAFETY: source_date_epoch is a valid time value; localtime returns a
    // thread-local pointer that is valid until the next call.
    let tmptr = libc::localtime(&source_date_epoch);
    *minutes = (*tmptr).tm_hour * 60 + (*tmptr).tm_min;
    *day = (*tmptr).tm_mday;
    *month = (*tmptr).tm_mon + 1;
    *year = (*tmptr).tm_year + 1900;
}

/// Write the current wall-clock time into `*seconds` and `*micros`.
///
/// translated from xetex-texmfmp.c:128 (get_seconds_and_micros)
#[no_mangle]
pub unsafe extern "C" fn get_seconds_and_micros(seconds: *mut i32, micros: *mut i32) {
    // SAFETY: tv is properly initialised; null timezone arg is allowed by POSIX.
    let mut tv = libc::timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    libc::gettimeofday(&mut tv, std::ptr::null_mut());
    *seconds = tv.tv_sec as i32;
    *micros = tv.tv_usec as i32;
}
