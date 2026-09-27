//! `malloc` and friends for the engine's C, on an allocator that can grow.
//!
//! wasi-libc's `sbrk` allocator never grows memory past `--initial-memory`,
//! and XeTeX wants hundreds of megabytes. The linker resolves the engine's
//! `malloc` here instead (see `--allow-multiple-definition` in
//! tools/wasm/env.sh), onto a private `dlmalloc` bounded only by
//! `--max-memory`. It is private rather than Rust's global allocator so that
//! `free` can never re-enter the allocator that called it.
//!
//! C's `free` does not pass a size, so each block starts with a [`HEADER`]
//! recording its total size.

use dlmalloc::Dlmalloc;
use std::{cell::UnsafeCell, ffi::c_void, ptr};

/// Bytes before each payload, and every block's alignment (`max_align_t`).
const HEADER: usize = 16;

// UnsafeCell rather than a Mutex keeps a lock off the hottest path in the engine.
struct Heap(UnsafeCell<Dlmalloc>);

// SAFETY: wasm32-wasip1 has no threads, so no concurrent access can occur.
unsafe impl Sync for Heap {}

static HEAP: Heap = Heap(UnsafeCell::new(Dlmalloc::new()));

fn with_heap<R>(operation: impl FnOnce(&mut Dlmalloc) -> R) -> R {
    // SAFETY: single-threaded, and dlmalloc never calls back into malloc, so
    // this is the only live borrow.
    operation(unsafe { &mut *HEAP.0.get() })
}

/// Record `total` in a fresh block's header and return its payload.
///
/// # Safety
///
/// `base` must be null or a block of at least `total` bytes aligned to [`HEADER`].
unsafe fn finish_block(base: *mut u8, total: usize) -> *mut c_void {
    if base.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: the caller guarantees `base` has room for the header and payload.
    unsafe {
        base.cast::<usize>().write(total);
        base.add(HEADER).cast()
    }
}

fn allocate(size: usize, zeroed: bool) -> *mut c_void {
    // Plenty of C assumes malloc(0) is non-null.
    let Some(total) = size.max(1).checked_add(HEADER) else {
        return ptr::null_mut();
    };

    // SAFETY: HEADER is a power of two and `total` is non-zero.
    let base = with_heap(|heap| unsafe {
        if zeroed {
            heap.calloc(total, HEADER)
        } else {
            heap.malloc(total, HEADER)
        }
    });

    // SAFETY: dlmalloc returned null or `total` bytes aligned to HEADER.
    unsafe { finish_block(base, total) }
}

/// A block's base pointer and total size, from its header.
///
/// # Safety
///
/// `ptr` must be a live, non-null payload from [`allocate`] or [`realloc`].
unsafe fn block(ptr: *mut c_void) -> (*mut u8, usize) {
    // SAFETY: the caller guarantees a header precedes `ptr`.
    unsafe {
        let base = ptr.cast::<u8>().sub(HEADER);
        (base, base.cast::<usize>().read())
    }
}

/// C's `malloc`.
///
/// # Safety
///
/// The result must be released with [`free`].
#[no_mangle]
pub unsafe extern "C" fn malloc(size: usize) -> *mut c_void {
    allocate(size, false)
}

/// C's `calloc`.
///
/// # Safety
///
/// The result must be released with [`free`].
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
/// `ptr` must be null or a live block from this allocator.
#[no_mangle]
pub unsafe extern "C" fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void {
    if ptr.is_null() {
        return allocate(size, false);
    }

    if size == 0 {
        // SAFETY: forwarded from the caller.
        unsafe { free(ptr) };
        return ptr::null_mut();
    }

    let Some(new_total) = size.checked_add(HEADER) else {
        return ptr::null_mut();
    };

    // SAFETY: `ptr` is a live block, per the caller.
    let (base, total) = unsafe { block(ptr) };
    // dlmalloc grows in place where it can, and leaves the old block intact
    // on failure as C requires.
    // SAFETY: `base`/`total` describe a live dlmalloc block of this alignment.
    let grown = with_heap(|heap| unsafe { heap.realloc(base, total, HEADER, new_total) });

    // SAFETY: dlmalloc returned null or `new_total` bytes aligned to HEADER.
    unsafe { finish_block(grown, new_total) }
}

/// C's `free`.
///
/// # Safety
///
/// `ptr` must be null or a live block from this allocator, unused afterwards.
#[no_mangle]
pub unsafe extern "C" fn free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }

    // SAFETY: `ptr` is a live block, per the caller.
    let (base, total) = unsafe { block(ptr) };
    // SAFETY: `base`/`total` describe a live dlmalloc block of this alignment.
    with_heap(|heap| unsafe { heap.free(base, total, HEADER) });
}

/// glibc's `malloc_usable_size`, which some C probes for.
///
/// # Safety
///
/// `ptr` must be null or a live block from this allocator.
#[no_mangle]
pub unsafe extern "C" fn malloc_usable_size(ptr: *mut c_void) -> usize {
    if ptr.is_null() {
        return 0;
    }

    // SAFETY: `ptr` is a live block, per the caller.
    unsafe { block(ptr) }.1 - HEADER
}
