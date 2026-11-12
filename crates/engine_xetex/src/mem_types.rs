//! The word-level records of the engine arenas.

use std::mem::size_of;

use crate::errors::Abort;
use crate::mode::Mode;

/// Two 32-bit halves, `s0` at the lower address.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct B32x2 {
    pub s0: i32,
    pub s1: i32,
}

const _: () = assert!(size_of::<B32x2>() == 8);

/// Word types whose all-zero bit pattern is a valid value.
///
/// # Safety
/// Every field of an implementor must be valid at all-zero bits.
pub(crate) unsafe trait ZeroWord: Copy {}
// SAFETY: both fields are i32.
unsafe impl ZeroWord for B32x2 {}
// SAFETY: every bit pattern is a valid u32.
unsafe impl ZeroWord for u32 {}

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
#[derive(Copy, Clone)]
pub struct ListStateRecord {
    pub mode: Mode,
    pub head: crate::units::MemIdx,
    pub tail: crate::units::MemIdx,
    pub e_tex_aux: crate::units::MemPtr,
    pub prev_graf: i32,
    pub mode_line: i32,
}

const _: () = assert!(size_of::<ListStateRecord>() == 32);

/// No list, as before INITEX sets up the page.
impl Default for ListStateRecord {
    fn default() -> Self {
        Self {
            mode: Mode::None,
            head: crate::mem::CONTRIB_HEAD,
            tail: crate::mem::CONTRIB_HEAD,
            e_tex_aux: None,
            prev_graf: 0,
            mode_line: 0,
        }
    }
}

impl ListStateRecord {
    /// `prev_depth` of the vertical list being built.
    #[inline]
    pub fn prev_depth(&self) -> Result<crate::units::Scaled, Abort> {
        match self.mode {
            Mode::Vertical { prev_depth, .. } => Ok(prev_depth),
            _ => Err(Abort::Confusion("prev depth")),
        }
    }
    #[inline]
    pub fn set_prev_depth(&mut self, d: crate::units::Scaled) -> Result<(), Abort> {
        match &mut self.mode {
            Mode::Vertical { prev_depth, .. } => *prev_depth = d,
            _ => return Err(Abort::Confusion("prev depth")),
        }
        Ok(())
    }
    /// `space_factor` of the horizontal list being built.
    #[inline]
    pub fn space_factor(&self) -> Result<i32, Abort> {
        match self.mode {
            Mode::Horizontal { space_factor, .. } => Ok(space_factor),
            _ => Err(Abort::Confusion("space factor")),
        }
    }
    #[inline]
    pub fn set_space_factor(&mut self, f: i32) -> Result<(), Abort> {
        match &mut self.mode {
            Mode::Horizontal { space_factor, .. } => *space_factor = f,
            _ => return Err(Abort::Confusion("space factor")),
        }
        Ok(())
    }
    /// `clang`, the current language of the horizontal list being built.
    #[inline]
    pub fn clang(&self) -> Result<i32, Abort> {
        match self.mode {
            Mode::Horizontal { clang, .. } => Ok(clang),
            _ => Err(Abort::Confusion("clang")),
        }
    }
    #[inline]
    pub fn set_clang(&mut self, l: i32) -> Result<(), Abort> {
        match &mut self.mode {
            Mode::Horizontal { clang, .. } => *clang = l,
            _ => return Err(Abort::Confusion("clang")),
        }
        Ok(())
    }
    /// `incompleat_noad`, the pending fraction of the math list being built.
    #[inline]
    pub fn incompleat_noad(
        &self,
    ) -> Result<Option<crate::node::NodePtr<crate::node::FractionNoad>>, Abort> {
        match self.mode {
            Mode::Math {
                incompleat_noad, ..
            } => Ok(incompleat_noad),
            _ => Err(Abort::Confusion("incompleat noad")),
        }
    }
    #[inline]
    pub fn set_incompleat_noad(
        &mut self,
        n: Option<crate::node::NodePtr<crate::node::FractionNoad>>,
    ) -> Result<(), Abort> {
        match &mut self.mode {
            Mode::Math {
                incompleat_noad, ..
            } => *incompleat_noad = n,
            _ => return Err(Abort::Confusion("incompleat noad")),
        }
        Ok(())
    }

    /// Links `p` after the tail and makes it the new tail.
    #[inline]
    pub fn tail_append(&mut self, mem: &mut crate::state::MemState, p: impl crate::units::MemAddr) {
        let p = p.addr();
        self.tail.set_link(mem, Some(p));
        self.tail = p;
    }
}
