//! The word-level records of the engine arenas.

use std::alloc;
use std::alloc::Layout;
use std::mem::size_of;

use crate::base::mode::Mode;
use crate::base::units::{MemAddr, MemIdx, MemPtr, Scaled};
use crate::memory::mem::{CONTRIB_HEAD, MemState};
use crate::memory::node::{FractionNoad, NodePtr};
use crate::print::errors::Abort;

/// Word types whose all-zero bit pattern is a valid value.
///
/// # Safety
/// Every field of an implementor must be valid at all-zero bits.
pub(crate) unsafe trait ZeroWord: Copy {}
// SAFETY: every bit pattern is a valid u32.
unsafe impl ZeroWord for u32 {}

/// A `Vec` of `len` zeroed words that leaves untouched pages unmapped.
///
/// `vec![zero; len]` would write every word of the large arenas up front.
pub(crate) fn zeroed_vec<T: ZeroWord>(len: usize) -> Vec<T> {
    if len == 0 {
        return Vec::new();
    }
    let layout = Layout::array::<T>(len).expect("arena size overflow");
    // SAFETY: `layout` is non-zero-sized and all-zero bytes are a valid `T`
    // (`ZeroWord` contract). Length == capacity matches the allocation
    // layout, so `Vec`'s destructor deallocates with the same layout.
    unsafe {
        let ptr = alloc::alloc_zeroed(layout) as *mut T;
        if ptr.is_null() {
            alloc::handle_alloc_error(layout);
        }
        Vec::from_raw_parts(ptr, len, len)
    }
}

#[derive(Copy, Clone)]
pub struct ListStateRecord {
    pub mode: Mode,
    pub head: MemIdx,
    pub tail: MemIdx,
    pub e_tex_aux: MemPtr,
    pub prev_graf: i32,
    pub mode_line: i32,
}

const _: () = assert!(size_of::<ListStateRecord>() == 32);

/// No list, as before INITEX sets up the page.
impl Default for ListStateRecord {
    fn default() -> Self {
        Self {
            mode: Mode::None,
            head: CONTRIB_HEAD,
            tail: CONTRIB_HEAD,
            e_tex_aux: None,
            prev_graf: 0,
            mode_line: 0,
        }
    }
}

impl ListStateRecord {
    #[inline]
    pub fn prev_depth(&self) -> Result<Scaled, Abort> {
        match self.mode {
            Mode::Vertical { prev_depth, .. } => Ok(prev_depth),
            _ => Err(Abort::Confusion("prev depth")),
        }
    }
    #[inline]
    pub fn set_prev_depth(&mut self, d: Scaled) -> Result<(), Abort> {
        match &mut self.mode {
            Mode::Vertical { prev_depth, .. } => *prev_depth = d,
            _ => return Err(Abort::Confusion("prev depth")),
        }
        Ok(())
    }
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
    #[inline]
    pub fn incompleat_noad(&self) -> Result<Option<NodePtr<FractionNoad>>, Abort> {
        match self.mode {
            Mode::Math {
                incompleat_noad, ..
            } => Ok(incompleat_noad),
            _ => Err(Abort::Confusion("incompleat noad")),
        }
    }
    #[inline]
    pub fn set_incompleat_noad(&mut self, n: Option<NodePtr<FractionNoad>>) -> Result<(), Abort> {
        match &mut self.mode {
            Mode::Math {
                incompleat_noad, ..
            } => *incompleat_noad = n,
            _ => return Err(Abort::Confusion("incompleat noad")),
        }
        Ok(())
    }

    #[inline]
    pub fn tail_append(&mut self, mem: &mut MemState, p: impl MemAddr) {
        let p = p.addr();
        self.tail.set_link(mem, Some(p));
        self.tail = p;
    }
}
