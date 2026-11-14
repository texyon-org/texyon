//! Small C-runtime compatibility surface.

#[cfg(not(windows))]
pub use libc::{gmtime, localtime};

#[cfg(windows)]
unsafe extern "C" {
    #[link_name = "_gmtime64"]
    pub fn gmtime(time: *const libc::time_t) -> *mut libc::tm;

    #[link_name = "_localtime64"]
    pub fn localtime(time: *const libc::time_t) -> *mut libc::tm;
}
