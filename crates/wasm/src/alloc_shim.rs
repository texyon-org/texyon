//! `malloc` and friends for the engine's C, on an allocator that can grow.
//!
//! # Why this exists
//!
//! wasi-libc's allocator sits on `sbrk`, which is bounded by the linker-provided
//! heap end and does not grow the WebAssembly memory past it. Measured on this
//! toolchain (wasi-sdk 33): in a module with 64 MB of initial memory, a 30 MB
//! `malloc` succeeds, a 50 MB `malloc` returns null, and `memory.size` never
//! moves. XeTeX allocates its arenas in tens of megabytes — roughly 275 MB in
//! total before a single checkpoint — so that is not survivable, and raising
//! `--initial-memory` only moves the wall.
//!
//! Defining the C entry points here makes the linker resolve every `malloc` in
//! the engine's C to this module instead. The backing allocator is its own
//! `dlmalloc` instance that grows the memory directly, so it is bounded only by
//! `--max-memory`.
//!
//! # Why its own dlmalloc, rather than Rust's global allocator
//!
//! Because `free` must not be able to re-enter the allocator that called it.
//! Routing these through `std::alloc` looks right and deadlocks the moment
//! Rust's own allocator is itself implemented over `malloc`/`free`, which is a
//! per-target and per-release detail we should not depend on. A private
//! instance is correct regardless of what Rust's allocator turns out to be.
//!
//! # Layout
//!
//! `dlmalloc::free` needs the size and alignment the block was created with,
//! which C's `free` does not supply, so every block carries a header recording
//! its total size. The header is [`HEADER`] bytes so the payload keeps the
//! 16-byte alignment C requires of `malloc`.

use dlmalloc::Dlmalloc;
use std::{cell::UnsafeCell, ffi::c_void, ptr};

/// Bytes reserved before each payload, and the alignment every block gets.
///
/// 16 is `max_align_t` on wasm32, which is what `malloc` must satisfy.
const HEADER: usize = 16;

/// The heap behind the engine's C.
///
/// wasm32-wasip1 is single-threaded and the engine holds a process-global lock
/// besides, so there is exactly one caller and no lock is needed. `UnsafeCell`
/// rather than a `Mutex` keeps `malloc` from being a lock acquisition on a path
/// the engine takes millions of times per document.
struct Heap(UnsafeCell<Dlmalloc>);

// SAFETY: the module is single-threaded — wasm32-wasip1 has no threads — so the
// aliasing this promises cannot actually occur.
unsafe impl Sync for Heap {}

static HEAP: Heap = Heap(UnsafeCell::new(Dlmalloc::new()));

/// Borrow the heap.
///
/// # Safety
///
/// The returned reference must not be held across a call that could allocate.
#[allow(clippy::mut_from_ref)]
unsafe fn heap() -> &'static mut Dlmalloc {
    &mut *HEAP.0.get()
}

/// Allocate `size` payload bytes, optionally zeroed.
///
/// # Safety
///
/// The result is owned by the caller and must be released with [`free`].
unsafe fn allocate(size: usize, zeroed: bool) -> *mut c_void {
    // C's malloc(0) may return null, but plenty of C assumes it does not, so
    // hand back a unique one-byte block instead.
    let size = size.max(1);

    let Some(total) = size.checked_add(HEADER) else {
        return ptr::null_mut();
    };

    let base = if zeroed {
        heap().calloc(total, HEADER)
    } else {
        heap().malloc(total, HEADER)
    };

    if base.is_null() {
        return ptr::null_mut();
    }

    base.cast::<usize>().write(total);
    base.add(HEADER).cast()
}

/// Recover a block's base pointer and total size from its header.
///
/// # Safety
///
/// `ptr` must be non-null and must have come from [`allocate`].
unsafe fn block(ptr: *mut c_void) -> (*mut u8, usize) {
    let base = ptr.cast::<u8>().sub(HEADER);
    (base, base.cast::<usize>().read())
}

/// C's `malloc`.
///
/// # Safety
///
/// Safe to call with any size; the result must be released with [`free`].
#[no_mangle]
pub unsafe extern "C" fn malloc(size: usize) -> *mut c_void {
    allocate(size, false)
}

/// C's `calloc`.
///
/// # Safety
///
/// Safe to call with any counts; the result must be released with [`free`].
#[no_mangle]
pub unsafe extern "C" fn calloc(count: usize, size: usize) -> *mut c_void {
    match count.checked_mul(size) {
        Some(total) => allocate(total, true),
        None => ptr::null_mut(),
    }
}

/// C's `realloc`.
///
/// # Safety
///
/// `ptr` must be null or have come from [`malloc`], [`calloc`] or `realloc`.
#[no_mangle]
pub unsafe extern "C" fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void {
    if ptr.is_null() {
        return allocate(size, false);
    }

    if size == 0 {
        free(ptr);
        return ptr::null_mut();
    }

    let (base, total) = block(ptr);

    let Some(new_total) = size.checked_add(HEADER) else {
        return ptr::null_mut();
    };

    // dlmalloc grows in place where it can, which matters: TeX's arrays are
    // grown by realloc, and copying tens of megabytes per growth step would be
    // felt on the first compile.
    let grown = heap().realloc(base, total, HEADER, new_total);
    if grown.is_null() {
        // C requires the original block to survive a failed realloc.
        return ptr::null_mut();
    }

    grown.cast::<usize>().write(new_total);
    grown.add(HEADER).cast()
}

/// C's `free`.
///
/// # Safety
///
/// `ptr` must be null or have come from [`malloc`], [`calloc`] or [`realloc`],
/// and must not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }

    let (base, total) = block(ptr);
    heap().free(base, total, HEADER);
}

/// glibc's `malloc_usable_size`, which some C probes for.
///
/// # Safety
///
/// `ptr` must be null or have come from [`malloc`].
#[no_mangle]
pub unsafe extern "C" fn malloc_usable_size(ptr: *mut c_void) -> usize {
    if ptr.is_null() {
        return 0;
    }

    block(ptr).1 - HEADER
}
