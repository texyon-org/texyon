//! TeX's `memory_word` and the other word-level records of the engine arenas.

use std::mem::size_of;

/// Two 32-bit halves, `s0` at the lower address.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct B32x2 {
    pub s0: i32,
    pub s1: i32,
}

/// Four 16-bit quarters.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct B16x4 {
    pub s0: u16,
    pub s1: u16,
    pub s2: u16,
    pub s3: u16,
}

/// TeX's `memory_word`. The format file stores these words verbatim.
#[repr(C)]
#[derive(Copy, Clone)]
pub union MemoryWord {
    pub b32: B32x2,
    pub b16: B16x4,
    pub gr: f64,
}

impl MemoryWord {
    #[inline]
    pub fn b32(self) -> B32x2 {
        // SAFETY: every field is 8 bytes of plain data, so any bits read as any field.
        unsafe { self.b32 }
    }

    #[inline]
    pub fn b16(self) -> B16x4 {
        // SAFETY: as in `b32`.
        unsafe { self.b16 }
    }

    #[inline]
    pub fn gr(self) -> f64 {
        // SAFETY: as in `b32`.
        unsafe { self.gr }
    }
}

impl Default for MemoryWord {
    #[inline]
    fn default() -> Self {
        MemoryWord {
            b32: B32x2 { s0: 0, s1: 0 },
        }
    }
}

const _: () = {
    assert!(size_of::<MemoryWord>() == 8);
    assert!(size_of::<B32x2>() == 8);
    assert!(size_of::<B16x4>() == 8);
};

/// Word types whose all-zero bit pattern is a valid value.
///
/// # Safety
/// Every field of an implementor must be valid at all-zero bits.
pub(crate) unsafe trait ZeroWord: Copy {}
// SAFETY: i32, u16 and f64 are all valid at zero bits.
unsafe impl ZeroWord for MemoryWord {}
// SAFETY: both fields are i32.
unsafe impl ZeroWord for B32x2 {}

/// A `Vec` of `len` zeroed words that leaves untouched pages unmapped.
///
/// `vec![zero; len]` would write every word of the large arenas up front.
pub(crate) fn zeroed_vec<T: ZeroWord>(len: usize) -> Vec<T> {
    if len == 0 {
        return Vec::new();
    }
    let layout = std::alloc::Layout::array::<T>(len).expect("arena size overflow");
    // SAFETY: `layout` is non-zero-sized and all-zero bytes are a valid `T`
    // (`ZeroWord` contract). Length == capacity matches the allocation
    // layout, so `Vec`'s destructor deallocates with the same layout.
    unsafe {
        let ptr = std::alloc::alloc_zeroed(layout) as *mut T;
        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        Vec::from_raw_parts(ptr, len, len)
    }
}

/// One level of TeX's semantic nest (`list_state_record`).
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ListStateRecord {
    pub mode: i16,
    pub head: crate::units::MemPtr,
    pub tail: crate::units::MemPtr,
    pub e_tex_aux: crate::units::MemPtr,
    pub prev_graf: i32,
    pub mode_line: i32,
    pub aux: MemoryWord,
}

const _: () = assert!(size_of::<ListStateRecord>() == 32);

impl ListStateRecord {
    #[inline]
    pub fn aux_s0(&self) -> i32 {
        self.aux.b32().s0
    }
    #[inline]
    pub fn aux_s1(&self) -> i32 {
        self.aux.b32().s1
    }
    #[inline]
    pub fn set_aux_s0(&mut self, v: i32) {
        self.aux.b32.s0 = v;
    }
    #[inline]
    pub fn set_aux_s1(&mut self, v: i32) {
        self.aux.b32.s1 = v;
    }

    /// Links `p` after the tail and makes it the new tail.
    #[inline]
    pub fn tail_append(&mut self, mem: &mut crate::state::MemState, p: crate::units::MemPtr) {
        self.tail.set_link(mem, p);
        self.tail = p;
    }
}
