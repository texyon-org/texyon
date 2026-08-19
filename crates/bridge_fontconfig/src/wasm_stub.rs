//! A fontconfig that owns no fonts, for targets that have no system font
//! directory to scan.
//!
//! WebAssembly is such a target: a browser gives a wasm module no filesystem to
//! discover fonts in, and every font a document uses arrives from the Tectonic
//! bundle by name instead. The font manager still wants the fontconfig C API,
//! so this provides it, answering every query with "nothing matched".
//!
//! Because `search_for_host_platform_fonts` then finds no candidates, a
//! `\setmainfont{Palatino}`-style request fails the same way it would on a
//! machine that simply does not have the font installed. Fonts referenced by
//! filename — `\font\f="[texgyrepagella-regular.otf]"` — never come through
//! here and keep working.

#![allow(nonstandard_style)]

use crate::sys::{FcBool, FcFontSet, FcObjectSet, FcPattern, FcResult, FcTrue};
use std::{cell::Cell, ptr};

/// The allocation behind an opaque `FcPattern`.
///
/// Real fontconfig patterns are reference counted and hold a property table.
/// Nothing here ever has a property to report, so only the count is real.
struct StubPattern {
    refs: Cell<usize>,
}

/// Take a raw pattern back as a reference, if it is not null.
///
/// # Safety
///
/// `p` must be null or a pointer returned by [`FcNameParse`].
unsafe fn pattern(p: *mut FcPattern) -> Option<&'static StubPattern> {
    (p as *const StubPattern).as_ref()
}

#[no_mangle]
pub extern "C" fn FcInit() -> FcBool {
    FcTrue
}

#[no_mangle]
pub extern "C" fn FcConfigGetCurrent() -> *mut crate::sys::FcConfig {
    // The caller only ever passes this straight back to FcFontList, which
    // ignores it. A null "current config" is what fontconfig itself returns
    // before initialization, and it is never dereferenced here.
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn FcNameParse(_name: *const libc::c_char) -> *mut FcPattern {
    let pat = Box::new(StubPattern { refs: Cell::new(1) });
    Box::into_raw(pat) as *mut FcPattern
}

#[no_mangle]
pub extern "C" fn FcPatternReference(pat: *mut FcPattern) {
    // SAFETY: our caller owns a reference to `pat`, so it came from FcNameParse.
    if let Some(pat) = unsafe { pattern(pat) } {
        pat.refs.set(pat.refs.get() + 1);
    }
}

#[no_mangle]
pub extern "C" fn FcPatternDestroy(pat: *mut FcPattern) {
    // SAFETY: our caller owns a reference to `pat`, so it came from FcNameParse.
    let Some(p) = (unsafe { pattern(pat) }) else {
        return;
    };

    let remaining = p.refs.get() - 1;
    p.refs.set(remaining);

    if remaining == 0 {
        // SAFETY: the last reference is going away, so nobody can observe the
        // allocation after this, and the pointer came from Box::into_raw.
        drop(unsafe { Box::from_raw(pat as *mut StubPattern) });
    }
}

#[no_mangle]
pub extern "C" fn FcPatternGetString(
    _p: *mut FcPattern,
    _object: *const libc::c_char,
    _n: libc::c_int,
    _s: *mut *const libc::c_char,
) -> FcResult {
    FcResult::NoMatch
}

#[no_mangle]
pub extern "C" fn FcPatternGetInteger(
    _p: *mut FcPattern,
    _object: *const libc::c_char,
    _n: libc::c_int,
    _i: *mut libc::c_int,
) -> FcResult {
    FcResult::NoMatch
}

#[no_mangle]
pub extern "C" fn FcObjectSetCreate() -> *mut FcObjectSet {
    Box::into_raw(Box::new(())) as *mut FcObjectSet
}

#[no_mangle]
pub extern "C" fn FcObjectSetAdd(_os: *mut FcObjectSet, _object: *const libc::c_char) -> FcBool {
    FcTrue
}

#[no_mangle]
pub extern "C" fn FcObjectSetDestroy(os: *mut FcObjectSet) {
    if !os.is_null() {
        // SAFETY: the caller owns `os`, so it came from FcObjectSetCreate.
        drop(unsafe { Box::from_raw(os as *mut ()) });
    }
}

#[no_mangle]
pub extern "C" fn FcFontList(
    _config: *mut crate::sys::FcConfig,
    _p: *mut FcPattern,
    _os: *mut FcObjectSet,
) -> *mut FcFontSet {
    // An empty set, laid out the way fontconfig lays one out: callers read
    // `nfont` and then index `fonts`, which they never reach.
    Box::into_raw(Box::new(FcFontSet::empty())) as *mut FcFontSet
}

#[no_mangle]
pub extern "C" fn FcFontSetDestroy(fs: *mut FcFontSet) {
    if !fs.is_null() {
        // SAFETY: the caller owns `fs`, so it came from FcFontList.
        drop(unsafe { Box::from_raw(fs) });
    }
}
