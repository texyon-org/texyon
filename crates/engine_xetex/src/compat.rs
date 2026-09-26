//! Small C-runtime compatibility surface.

#[cfg(not(windows))]
pub use libc::{__errno_location as errno_location, gmtime, localtime, strftime};

#[cfg(windows)]
extern "C" {
    #[link_name = "_errno"]
    pub fn errno_location() -> *mut libc::c_int;

    #[link_name = "_gmtime64"]
    pub fn gmtime(time: *const libc::time_t) -> *mut libc::tm;

    #[link_name = "_localtime64"]
    pub fn localtime(time: *const libc::time_t) -> *mut libc::tm;

    pub fn strftime(
        buffer: *mut libc::c_char,
        count: libc::size_t,
        format: *const libc::c_char,
        time: *const libc::tm,
    ) -> libc::size_t;
}
