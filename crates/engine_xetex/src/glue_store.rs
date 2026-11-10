//! Glue specifications, kept apart from `mem` and shared by reference count.

use crate::consts::TEX_NULL;
use crate::errors::Abort;
use crate::journal::{WordDelta, WordJournal};
use crate::node::Stored;
use crate::units::{GlueOrder, Scaled};

/// A glue specification in the [`GlueStore`]. Glue pointers are never null.
#[derive(Copy, Clone, PartialEq, Eq, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(transparent)]
pub struct GlueSpecId(i32);

impl GlueSpecId {
    /// The spec from its raw form in an eqtb or save stack word, a sparse register or a format.
    #[inline]
    pub(crate) const fn from_raw(raw: i32) -> GlueSpecId {
        GlueSpecId(raw)
    }

    /// The raw form for an eqtb or save stack word, a sparse register or a format.
    #[inline]
    pub(crate) const fn raw(self) -> i32 {
        self.0
    }

    /// The spec a raw word holds, `None` for null.
    #[inline]
    pub(crate) fn from_link(raw: i32) -> Option<GlueSpecId> {
        (raw != TEX_NULL).then_some(GlueSpecId(raw))
    }

    #[inline]
    fn slot(self) -> usize {
        self.0 as usize
    }
}

/// A glue specification.
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlueSpec {
    pub shrink_order: Stored<GlueOrder>,
    pub stretch_order: Stored<GlueOrder>,
    /// The reference count, `TEX_NULL` for a single reference. On a free spec, the next free one.
    pub ref_count: i32,
    pub width: Scaled,
    pub stretch: Scaled,
    pub shrink: Scaled,
}

pub(crate) const ZERO_GLUE: GlueSpecId = GlueSpecId(0);
pub(crate) const FIL_GLUE: GlueSpecId = GlueSpecId(1);
pub(crate) const FILL_GLUE: GlueSpecId = GlueSpecId(2);
pub(crate) const SS_GLUE: GlueSpecId = GlueSpecId(3);
pub(crate) const FIL_NEG_GLUE: GlueSpecId = GlueSpecId(4);
pub(crate) const STATIC_SPECS: usize = 5;
/// Specs take five words, and get up to a tenth of main memory.
const MAX_SPECS: usize = 1 << 22;

/// All glue specifications, the static ones first.
pub(crate) struct GlueStore {
    data: Vec<GlueSpec>,
    /// The first free spec, threaded through `ref_count`.
    free: i32,
    pub(crate) journal: WordJournal<GlueSpec>,
}

impl Default for GlueStore {
    fn default() -> Self {
        Self {
            data: vec![bytemuck::Zeroable::zeroed(); STATIC_SPECS],
            free: TEX_NULL,
            journal: WordJournal::default(),
        }
    }
}

/// The glue store's scalars, for checkpoints and formats.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) struct GlueStoreMarks {
    pub(crate) len: usize,
    pub(crate) free: i32,
}

impl GlueStore {
    #[inline]
    pub(crate) fn get(&self, id: GlueSpecId) -> &GlueSpec {
        &self.data[id.slot()]
    }

    #[inline]
    pub(crate) fn get_mut(&mut self, id: GlueSpecId) -> &mut GlueSpec {
        let i = id.slot();
        if self.journal.armed() {
            self.journal.note(i, self.data[i]);
        }
        &mut self.data[i]
    }

    /// A fresh spec, its fields uninitialised.
    pub(crate) fn alloc(&mut self) -> Result<GlueSpecId, Abort> {
        if let Some(p) = GlueSpecId::from_link(self.free) {
            self.free = self.get(p).ref_count;
            return Ok(p);
        }
        if self.data.len() >= MAX_SPECS {
            return Err(Abort::NODE_OVERFLOW);
        }
        let p = i32::try_from(self.data.len()).map_err(|_| Abort::NODE_OVERFLOW)?;
        self.data.push(bytemuck::Zeroable::zeroed());
        Ok(GlueSpecId(p))
    }

    pub(crate) fn free(&mut self, p: GlueSpecId) {
        let next = self.free;
        self.get_mut(p).ref_count = next;
        self.free = p.0;
    }

    /// Whether `p` is a spec of this store, for printing a possibly clobbered pointer.
    pub(crate) fn contains(&self, raw: i32) -> bool {
        usize::try_from(raw).is_ok_and(|i| i < self.data.len())
    }

    pub(crate) fn specs(&self) -> &[GlueSpec] {
        &self.data
    }

    pub(crate) fn specs_mut(&mut self) -> &mut [GlueSpec] {
        &mut self.data
    }

    pub(crate) fn marks(&self) -> GlueStoreMarks {
        GlueStoreMarks {
            len: self.data.len(),
            free: self.free,
        }
    }

    /// Rewinds to `marks`, after the journal put back the specs that existed then.
    pub(crate) fn restore_marks(&mut self, marks: GlueStoreMarks) {
        self.data.truncate(marks.len);
        self.free = marks.free;
        if self.journal.armed() {
            self.journal.rebase(marks.len);
        }
    }

    /// Empties the store to `len` zero specs with free list `free`, for a format load to fill.
    pub(crate) fn reset_for_load(&mut self, len: usize, free: i32) {
        self.data = vec![bytemuck::Zeroable::zeroed(); len];
        self.free = free;
    }

    /// Closes the journal's epoch, see [`WordJournal::seal`].
    pub(crate) fn seal_journal(&mut self) -> WordDelta<GlueSpec> {
        self.journal.seal(&self.data)
    }
}

impl std::ops::Index<GlueSpecId> for crate::state::MemState {
    type Output = GlueSpec;

    #[inline]
    fn index(&self, id: GlueSpecId) -> &GlueSpec {
        self.glue.get(id)
    }
}

impl std::ops::IndexMut<GlueSpecId> for crate::state::MemState {
    #[inline]
    fn index_mut(&mut self, id: GlueSpecId) -> &mut GlueSpec {
        self.glue.get_mut(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freed_specs_are_reused_first() {
        let mut store = GlueStore::default();
        let a = store.alloc().unwrap();
        let b = store.alloc().unwrap();
        assert_eq!((a.raw(), b.raw()), (5, 6));
        store.free(a);
        store.free(b);
        assert_eq!(store.alloc().unwrap(), b);
        assert_eq!(store.alloc().unwrap(), a);
        assert_eq!(store.alloc().unwrap().raw(), 7);
    }
}
