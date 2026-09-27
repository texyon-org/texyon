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
//! recording its total size, then its capacity if it is a direct block.

use dlmalloc::Dlmalloc;
use std::{arch::wasm32, cell::UnsafeCell, ffi::c_void, ptr};

/// Bytes before each payload, and every block's alignment (`max_align_t`).
const HEADER: usize = 16;

/// dlmalloc's calloc always memsets, fresh wasm pages are already zero.
const DIRECT_MIN: usize = 1 << 20;

const WASM_PAGE: usize = 65536;

const SPARE_SLOTS: usize = 32;

// UnsafeCell rather than a Mutex keeps a lock off the hottest path in the engine.
struct Heap(UnsafeCell<Dlmalloc>);

// SAFETY: wasm32-wasip1 has no threads, so no concurrent access can occur.
unsafe impl Sync for Heap {}

static HEAP: Heap = Heap(UnsafeCell::new(Dlmalloc::new()));

/// Freed direct blocks, kept because wasm cannot return pages.
struct Spare(UnsafeCell<[(*mut u8, usize); SPARE_SLOTS]>);

// SAFETY: as for `Heap`.
unsafe impl Sync for Spare {}

static SPARE: Spare = Spare(UnsafeCell::new([(ptr::null_mut(), 0); SPARE_SLOTS]));

fn with_heap<R>(operation: impl FnOnce(&mut Dlmalloc) -> R) -> R {
    // SAFETY: single-threaded, and dlmalloc never calls back into malloc, so
    // this is the only live borrow.
    operation(unsafe { &mut *HEAP.0.get() })
}

fn with_spare<R>(operation: impl FnOnce(&mut [(*mut u8, usize); SPARE_SLOTS]) -> R) -> R {
    // SAFETY: single-threaded, and no caller nests these borrows.
    operation(unsafe { &mut *SPARE.0.get() })
}

/// `capacity` is zero for dlmalloc blocks.
///
/// # Safety
///
/// `base` must be null or a block of at least `total` bytes aligned to [`HEADER`].
unsafe fn finish_block(base: *mut u8, total: usize, capacity: usize) -> *mut c_void {
    if base.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: the caller guarantees `base` has room for the header and payload.
    unsafe {
        base.cast::<usize>().write(total);
        base.cast::<usize>().add(1).write(capacity);
        base.add(HEADER).cast()
    }
}

fn allocate_direct(total: usize) -> *mut c_void {
    let reused = with_spare(|spare| {
        let slot = spare
            .iter_mut()
            .filter(|(base, capacity)| !base.is_null() && *capacity >= total)
            .min_by_key(|(_, capacity)| *capacity)?;
        Some(std::mem::replace(slot, (ptr::null_mut(), 0)))
    });

    let (base, capacity) = match reused {
        Some((base, capacity)) => {
            // SAFETY: a spare block owns at least `capacity` >= `total` bytes.
            unsafe { ptr::write_bytes(base, 0, total) };
            (base, capacity)
        }
        None => {
            let pages = total.div_ceil(WASM_PAGE);
            let previous = wasm32::memory_grow(0, pages);
            if previous == usize::MAX {
                return ptr::null_mut();
            }
            ((previous * WASM_PAGE) as *mut u8, pages * WASM_PAGE)
        }
    };

    // SAFETY: `base` owns `capacity` >= `total` bytes, page aligned.
    unsafe { finish_block(base, total, capacity) }
}

fn allocate(size: usize, zeroed: bool) -> *mut c_void {
    // Plenty of C assumes malloc(0) is non-null.
    let Some(total) = size.max(1).checked_add(HEADER) else {
        return ptr::null_mut();
    };

    if zeroed && total >= DIRECT_MIN {
        return allocate_direct(total);
    }

    // SAFETY: HEADER is a power of two and `total` is non-zero.
    let base = with_heap(|heap| unsafe {
        if zeroed {
            heap.calloc(total, HEADER)
        } else {
            heap.malloc(total, HEADER)
        }
    });

    // SAFETY: dlmalloc returned null or `total` bytes aligned to HEADER.
    unsafe { finish_block(base, total, 0) }
}

/// # Safety
///
/// `ptr` must be a live, non-null payload from [`allocate`] or [`realloc`].
unsafe fn block(ptr: *mut c_void) -> (*mut u8, usize, usize) {
    // SAFETY: the caller guarantees a header precedes `ptr`.
    unsafe {
        let base = ptr.cast::<u8>().sub(HEADER);
        let words = base.cast::<usize>();
        (base, words.read(), words.add(1).read())
    }
}

/// Leaks the block once every slot is taken.
fn free_direct(base: *mut u8, capacity: usize) {
    with_spare(|spare| {
        if let Some(slot) = spare.iter_mut().find(|(base, _)| base.is_null()) {
            *slot = (base, capacity);
        }
    });
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
    let (base, total, capacity) = unsafe { block(ptr) };

    if capacity != 0 {
        if new_total <= capacity {
            // SAFETY: the header lies within the live block.
            unsafe { base.cast::<usize>().write(new_total) };
            return ptr;
        }
        let moved = allocate(size, false);
        if !moved.is_null() {
            // SAFETY: both blocks are live, distinct and hold the old payload.
            unsafe { ptr::copy_nonoverlapping(ptr.cast::<u8>(), moved.cast(), total - HEADER) };
            free_direct(base, capacity);
        }
        return moved;
    }

    // dlmalloc grows in place where it can, and leaves the old block intact
    // on failure as C requires.
    // SAFETY: `base`/`total` describe a live dlmalloc block of this alignment.
    let grown = with_heap(|heap| unsafe { heap.realloc(base, total, HEADER, new_total) });

    // SAFETY: dlmalloc returned null or `new_total` bytes aligned to HEADER.
    unsafe { finish_block(grown, new_total, 0) }
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
    let (base, total, capacity) = unsafe { block(ptr) };
    if capacity != 0 {
        free_direct(base, capacity);
        return;
    }
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
