//! Low-level `memory_word` bit-layout types, mirroring `xetex-core.h:99–104`.
//!
//! During the C→Rust migration, the C and
//! Rust code share a single backing array of `memory_word` / [`MemoryWord`]
//! values. `#[repr(C)]` is therefore *required*, not a stylistic choice.

use std::mem::size_of;

/// Two 32-bit signed ints.
///
/// Matches the C `b32x2` in `xetex-core.h:89/94`. The endian-dependent
/// field order in the C source is just re-ordering the *declarations*
/// of `s0` and `s1`; the in-memory byte layout is the same on every
/// platform (little-endian `s0` first on LE, big-endian `s0` first
/// on BE — both cases, the lower-addressed word is called `s0`).
/// So a straightforward `#[repr(C)]` struct with `s0` then `s1`
/// round-trips correctly.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct B32x2 {
    pub s0: i32,
    pub s1: i32,
}

/// Four 16-bit unsigned ints (the "four quarters" packing).
///
/// Matches the C `b16x4` in `xetex-core.h:90/95`.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct B16x4 {
    pub s0: u16,
    pub s1: u16,
    pub s2: u16,
    pub s3: u16,
}

/// The fundamental TeX memory word: a tagged union of the four
/// interpretations the engine uses.
///
/// Corresponds exactly to the C `memory_word` union (`xetex-core.h:99`).
/// The `#[repr(C)]` is load-bearing: the C and Rust engine share the same
/// backing array, so the union layout
/// must match bit-for-bit.
#[repr(C)]
#[derive(Copy, Clone)]
pub union MemoryWord {
    pub b32: B32x2,
    pub b16: B16x4,
    pub gr: f64,
    pub ptr: *mut std::ffi::c_void,
}

impl Default for MemoryWord {
    #[inline]
    fn default() -> Self {
        MemoryWord {
            b32: B32x2 { s0: 0, s1: 0 },
        }
    }
}

// The C engine's algorithms assume `sizeof(memory_word) == 8`, and the
// layout of `mem[]` relies on it. Catch any drift at compile time.
const _: () = {
    assert!(size_of::<MemoryWord>() == 8);
    assert!(size_of::<B32x2>() == 8);
    assert!(size_of::<B16x4>() == 8);
};

/// Marker for the arena word types whose all-zero bit pattern is a valid
/// value, so a lazily-zeroed allocation is equivalent to `vec![zero; n]`.
///
/// # Safety
/// Implementors must be plain-old-data with every field valid at all-zero
/// bits (no niches, no references).
pub(crate) unsafe trait ZeroWord: Copy {}
// SAFETY: unions/structs of i32/u16/f64/raw-pointer; zero bits are valid
// for every interpretation.
unsafe impl ZeroWord for MemoryWord {}
unsafe impl ZeroWord for B32x2 {}

/// Allocate a `Vec` of `len` zeroed words without dirtying its pages.
///
/// `vec![zero; len]` on these custom word types misses std's `IsZero`
/// specialization and writes the whole arena upfront — for the big engine
/// arenas (`mem`, `eqtb`, `hash`, `font_info`) that is ~256 MB of page
/// faults + memsets per run that the C engine never paid (`xmalloc` /
/// lazy `xcalloc`). `alloc_zeroed` lowers to `calloc`/`mmap`, so the
/// kernel hands back copy-on-write zero pages and untouched words never
/// cost anything, restoring the C cost model with identical contents.
pub(crate) fn zeroed_vec<T: ZeroWord>(len: usize) -> Vec<T> {
    if len == 0 {
        return Vec::new();
    }
    let layout = std::alloc::Layout::array::<T>(len).expect("arena size overflow");
    // SAFETY: `layout` is non-zero-sized; all-zero bytes are a valid `T`
    // (`ZeroWord` contract); length == capacity matches the allocation
    // layout, so `Vec`'s destructor deallocates with the same layout.
    unsafe {
        let ptr = std::alloc::alloc_zeroed(layout) as *mut T;
        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        Vec::from_raw_parts(ptr, len, len)
    }
}

/// Rust mirror of C `list_state_record` (xetex-xetexd.h:334–342).
///
/// Represents one entry on TeX's "semantic nest" (mode stack).
/// Must stay layout-compatible with the C struct.
///
/// C layout on x86_64 (little-endian):
///   offset  0: mode (short = i16, 2 B)
///   offset  2: [2 B padding]
///   offset  4: head (int32_t, 4 B)
///   offset  8: tail (int32_t, 4 B)
///   offset 12: eTeX_aux (int32_t, 4 B)
///   offset 16: prev_graf (int32_t, 4 B)
///   offset 20: mode_line (int32_t, 4 B)
///   offset 24: aux (memory_word, 8 B)
///   total: 32 B
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ListStateRecord {
    pub mode: i16,
    // 2 bytes implicit padding
    pub head: crate::units::MemPtr,
    pub tail: crate::units::MemPtr,
    pub e_tex_aux: crate::units::MemPtr,
    pub prev_graf: i32,
    pub mode_line: i32,
    pub aux: MemoryWord,
}

const _: () = assert!(size_of::<ListStateRecord>() == 32);

impl ListStateRecord {
    /// Read `aux.b32.s0`. Reading a union field is `unsafe`; localized here.
    #[inline]
    pub fn aux_s0(&self) -> i32 { unsafe { self.aux.b32.s0 } }
    /// Read `aux.b32.s1`.
    #[inline]
    pub fn aux_s1(&self) -> i32 { unsafe { self.aux.b32.s1 } }
    /// Write `aux.b32.s0` (writing a Copy union field is safe).
    #[inline]
    pub fn set_aux_s0(&mut self, v: i32) { self.aux.b32.s0 = v; }
    /// Write `aux.b32.s1`.
    #[inline]
    pub fn set_aux_s1(&mut self, v: i32) { self.aux.b32.s1 = v; }
}
