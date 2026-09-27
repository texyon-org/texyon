//! `malloc` and friends for C and Rust's default allocator, replacing
//! wasi-libc's (see `--allow-multiple-definition` in tools/wasm/env.sh).
//!
//! It is faster than wasi-libc's, mostly because large zeroed blocks come
//! straight from `memory.grow` instead of being memset.

use dlmalloc::Dlmalloc;
use std::{arch::wasm32, cell::UnsafeCell, ffi::c_void, ptr};

/// dlmalloc's calloc always memsets, fresh wasm pages are already zero.
const DIRECT_MIN: usize = 1 << 20;

const WASM_PAGE: usize = 65536;

const DIRECT_SLOTS: usize = 64;

const EINVAL: i32 = 28;
const ENOMEM: i32 = 48;

/// A page-aligned block outside dlmalloc. `size` is zero once freed, and
/// the pages are kept because wasm cannot return them.
#[derive(Clone, Copy)]
struct Direct {
    base: *mut u8,
    capacity: usize,
    size: usize,
}

const EMPTY: Direct = Direct {
    base: ptr::null_mut(),
    capacity: 0,
    size: 0,
};

struct State {
    heap: Dlmalloc,
    direct: [Direct; DIRECT_SLOTS],
}

// UnsafeCell rather than a Mutex keeps a lock off the hottest path in the engine.
struct Global(UnsafeCell<State>);

// SAFETY: wasm32-wasip1 has no threads, so no concurrent access can occur.
unsafe impl Sync for Global {}

static STATE: Global = Global(UnsafeCell::new(State {
    heap: Dlmalloc::new(),
    direct: [EMPTY; DIRECT_SLOTS],
}));

fn with_state<R>(operation: impl FnOnce(&mut State) -> R) -> R {
    // SAFETY: single-threaded, and dlmalloc never calls back into malloc, so
    // this is the only live borrow.
    operation(unsafe { &mut *STATE.0.get() })
}

impl State {
    fn live_direct(&mut self, ptr: *mut u8) -> Option<&mut Direct> {
        if ptr as usize % WASM_PAGE != 0 {
            return None;
        }
        self.direct
            .iter_mut()
            .find(|block| block.base == ptr && block.size != 0)
    }

    fn allocate_direct(&mut self, size: usize) -> Option<*mut u8> {
        let spare = self
            .direct
            .iter_mut()
            .filter(|block| !block.base.is_null() && block.size == 0 && block.capacity >= size)
            .min_by_key(|block| block.capacity);

        if let Some(block) = spare {
            // SAFETY: a spare block owns `capacity` >= `size` bytes.
            unsafe { ptr::write_bytes(block.base, 0, size) };
            block.size = size;
            return Some(block.base);
        }

        let slot = self.direct.iter_mut().find(|block| block.base.is_null())?;
        let pages = size.div_ceil(WASM_PAGE);
        let previous = wasm32::memory_grow(0, pages);
        if previous == usize::MAX {
            return None;
        }
        *slot = Direct {
            base: (previous * WASM_PAGE) as *mut u8,
            capacity: pages * WASM_PAGE,
            size,
        };
        Some(slot.base)
    }

    fn calloc(&mut self, size: usize) -> *mut u8 {
        if size >= DIRECT_MIN {
            if let Some(base) = self.allocate_direct(size) {
                return base;
            }
        }
        // SAFETY: 8 is dlmalloc's natural alignment on wasm32.
        unsafe { self.heap.calloc(size, 8) }
    }

    /// # Safety
    ///
    /// `ptr` must be a live block from this allocator.
    unsafe fn realloc(&mut self, ptr: *mut u8, size: usize) -> *mut u8 {
        let Some(block) = self.live_direct(ptr) else {
            // SAFETY: forwarded from the caller.
            return unsafe { self.heap.c_realloc(ptr, size) };
        };

        if size <= block.capacity {
            block.size = size;
            return ptr;
        }

        let old_size = block.size;
        // SAFETY: dlmalloc has no preconditions on `size`.
        let moved = unsafe { self.heap.c_malloc(size) };
        if !moved.is_null() {
            // SAFETY: both blocks are live and distinct, and hold `old_size` bytes.
            unsafe { ptr::copy_nonoverlapping(ptr, moved, old_size) };
            if let Some(block) = self.live_direct(ptr) {
                block.size = 0;
            }
        }
        moved
    }

    /// # Safety
    ///
    /// `ptr` must be a live block from this allocator.
    unsafe fn free(&mut self, ptr: *mut u8) {
        match self.live_direct(ptr) {
            Some(block) => block.size = 0,
            // SAFETY: forwarded from the caller.
            None => unsafe { self.heap.c_free(ptr) },
        }
    }
}

/// C's `malloc`.
///
/// # Safety
///
/// The result must be released with [`free`].
#[no_mangle]
pub unsafe extern "C" fn malloc(size: usize) -> *mut c_void {
    // SAFETY: dlmalloc has no preconditions on `size`.
    with_state(|state| unsafe { state.heap.c_malloc(size) }).cast()
}

/// C's `calloc`.
///
/// # Safety
///
/// The result must be released with [`free`].
#[no_mangle]
pub unsafe extern "C" fn calloc(count: usize, size: usize) -> *mut c_void {
    match count.checked_mul(size) {
        Some(total) => with_state(|state| state.calloc(total)).cast(),
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
        // SAFETY: forwarded from the caller.
        return unsafe { malloc(size) };
    }
    if size == 0 {
        // SAFETY: forwarded from the caller.
        unsafe { free(ptr) };
        return ptr::null_mut();
    }
    // SAFETY: forwarded from the caller.
    with_state(|state| unsafe { state.realloc(ptr.cast(), size) }).cast()
}

/// C's `free`.
///
/// # Safety
///
/// `ptr` must be null or a live block from this allocator, unused afterwards.
#[no_mangle]
pub unsafe extern "C" fn free(ptr: *mut c_void) {
    if !ptr.is_null() {
        // SAFETY: forwarded from the caller.
        with_state(|state| unsafe { state.free(ptr.cast()) });
    }
}

/// C's `aligned_alloc`.
///
/// # Safety
///
/// The result must be released with [`free`].
#[no_mangle]
pub unsafe extern "C" fn aligned_alloc(align: usize, size: usize) -> *mut c_void {
    if !align.is_power_of_two() {
        return ptr::null_mut();
    }
    // SAFETY: `align` is a power of two.
    with_state(|state| unsafe { state.heap.c_memalign(align, size) }).cast()
}

/// C's `posix_memalign`, which Rust uses for over-aligned layouts. Without it
/// wasi-libc's would allocate blocks that our `free` cannot release.
///
/// # Safety
///
/// `out` must be writable, and the result must be released with [`free`].
#[no_mangle]
pub unsafe extern "C" fn posix_memalign(out: *mut *mut c_void, align: usize, size: usize) -> i32 {
    if !align.is_power_of_two() || align % size_of::<usize>() != 0 {
        return EINVAL;
    }
    // SAFETY: `align` is a valid alignment.
    let block = unsafe { aligned_alloc(align, size) };
    if block.is_null() {
        return ENOMEM;
    }
    // SAFETY: the caller guarantees `out` is writable.
    unsafe { out.write(block) };
    0
}
