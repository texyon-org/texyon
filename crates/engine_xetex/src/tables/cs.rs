//! Control sequences: a dense id that keys the meaning tables, and what each
//! one is. The numbering keeps TeX's region order and is private to this module.

use std::num::NonZeroU32;

use crate::base::command::Usv;
use crate::base::consts::{BIGGEST_USV, MAX_FONT_MAX, PRIM_SIZE, TOO_BIG_CHAR};
use crate::base::units::{FontId, PrimNum};
use crate::tables::hash::HASH_SIZE;

// Offsets from the first control sequence, which is number 1.
const ACTIVE_BASE: u32 = 0;
const SINGLE_BASE: u32 = ACTIVE_BASE + (BIGGEST_USV + 1).cast_unsigned();
const NULL_CS: u32 = SINGLE_BASE + TOO_BIG_CHAR.cast_unsigned();
const HASH_BASE: u32 = NULL_CS + 1;
const FROZEN_BASE: u32 = HASH_BASE + HASH_SIZE.cast_unsigned();
const PRIM_BASE: u32 = FROZEN_BASE + Frozen::ALL.len() as u32;
const FONT_ID_BASE: u32 = PRIM_BASE + PRIM_SIZE.cast_unsigned();
const UNDEFINED: u32 = FONT_ID_BASE + MAX_FONT_MAX.cast_unsigned() + 1;
const EXTRA_BASE: u32 = UNDEFINED + 1;

/// A control sequence. Its number keys the meaning tables, [`CsId::kind`] says
/// what it is. TeX's 0, no control sequence, is `None`.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
#[repr(transparent)]
pub struct CsId(NonZeroU32);

// SAFETY: `CsId` is transparent over `NonZeroU32`, so `None` is the zero word.
unsafe impl bytemuck::ZeroableInOption for CsId {}
// SAFETY: as above, and every non-zero word is a valid `CsId`.
unsafe impl bytemuck::PodInOption for CsId {}

/// TeX's inaccessible copies of control sequences, which keep a fixed meaning.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum Frozen {
    /// `\inaccessible`, inserted for a missing control sequence.
    Protection,
    Cr,
    EndGroup,
    Right,
    Fi,
    EndTemplate,
    Endv,
    Relax,
    /// `\endwrite`, which ends the text of a `\write`.
    EndWrite,
    /// `\notexpanded:`, which `\noexpand` puts before a token.
    DontExpand,
    Special,
    Primitive,
}

impl Frozen {
    pub const ALL: [Frozen; 12] = [
        Frozen::Protection,
        Frozen::Cr,
        Frozen::EndGroup,
        Frozen::Right,
        Frozen::Fi,
        Frozen::EndTemplate,
        Frozen::Endv,
        Frozen::Relax,
        Frozen::EndWrite,
        Frozen::DontExpand,
        Frozen::Special,
        Frozen::Primitive,
    ];

    #[inline]
    pub const fn cs(self) -> CsId {
        CsId::at(FROZEN_BASE + self as u32)
    }
}

/// A slot of the name hash: the hash buckets, then the extra names past them.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct NameSlot(u32);

impl NameSlot {
    pub const BUCKETS_END: NameSlot = NameSlot(HASH_SIZE.cast_unsigned());

    #[inline]
    pub const fn bucket(h: u32) -> NameSlot {
        NameSlot(h)
    }

    #[inline]
    pub const fn extra(n: u32) -> NameSlot {
        NameSlot(HASH_SIZE.cast_unsigned() + n - 1)
    }

    pub fn all(extras: u32) -> impl Iterator<Item = NameSlot> {
        (0..HASH_SIZE.cast_unsigned() + extras).map(NameSlot)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub const fn from_raw(raw: u32) -> NameSlot {
        NameSlot(raw)
    }

    #[inline]
    pub fn prev_bucket(self) -> Option<NameSlot> {
        self.0.checked_sub(1).map(NameSlot)
    }

    #[inline]
    pub const fn cs(self) -> CsId {
        if self.0 < HASH_SIZE.cast_unsigned() {
            CsId::at(HASH_BASE + self.0)
        } else {
            CsId::at(EXTRA_BASE + self.0 - HASH_SIZE.cast_unsigned())
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum CsKind {
    Active(Usv),
    /// A control symbol or one-letter name with a code below 65536.
    Single(u16),
    Null,
    Named(NameSlot),
    Frozen(Frozen),
    Primitive(PrimNum),
    FontId(FontId),
    /// Where an unknown name lands when no new names may be made.
    Undefined,
}

impl From<CsKind> for CsId {
    #[inline]
    fn from(kind: CsKind) -> CsId {
        match kind {
            CsKind::Active(c) => CsId::active(c),
            CsKind::Single(c) => CsId::single(c),
            CsKind::Null => CsId::NULL,
            CsKind::Named(slot) => slot.cs(),
            CsKind::Frozen(f) => f.cs(),
            CsKind::Primitive(p) => CsId::primitive(p),
            CsKind::FontId(f) => CsId::font_id(f),
            CsKind::Undefined => CsId::UNDEFINED,
        }
    }
}

impl CsId {
    pub const FIRST: CsId = CsId::at(ACTIVE_BASE);
    pub const NULL: CsId = CsId::at(NULL_CS);
    pub const UNDEFINED: CsId = CsId::at(UNDEFINED);

    const fn at(n: u32) -> CsId {
        CsId(NonZeroU32::MIN.saturating_add(n))
    }

    #[inline]
    pub const fn active(c: Usv) -> CsId {
        CsId::at(ACTIVE_BASE + c.raw().cast_unsigned())
    }

    #[inline]
    pub const fn single(c: u16) -> CsId {
        CsId::at(SINGLE_BASE + c as u32)
    }

    #[inline]
    pub const fn primitive(p: PrimNum) -> CsId {
        CsId::at(PRIM_BASE + p.raw().cast_unsigned() - 1)
    }

    #[inline]
    pub const fn font_id(f: FontId) -> CsId {
        CsId::at(FONT_ID_BASE + f.get() as u32)
    }

    #[inline]
    pub fn of_active(c: i32) -> Option<CsId> {
        Usv::new(c).map(CsId::active)
    }

    /// The control sequence named by a single character, `None` for an astral one,
    /// which goes through the hash.
    #[inline]
    pub fn of_char(c: i32) -> Option<CsId> {
        u16::try_from(c).ok().map(CsId::single)
    }

    pub fn top(hash_extra: u32) -> CsId {
        CsId::at(UNDEFINED + hash_extra)
    }

    pub fn kind(self) -> CsKind {
        let n = self.0.get() - 1;
        if n < SINGLE_BASE {
            CsKind::Active(Usv::nth(n - ACTIVE_BASE))
        } else if n < NULL_CS {
            CsKind::Single((n - SINGLE_BASE) as u16)
        } else if n == NULL_CS {
            CsKind::Null
        } else if n < FROZEN_BASE {
            CsKind::Named(NameSlot(n - HASH_BASE))
        } else if n < PRIM_BASE {
            CsKind::Frozen(Frozen::ALL[(n - FROZEN_BASE) as usize])
        } else if n < FONT_ID_BASE {
            CsKind::Primitive(PrimNum::FIRST.plus(n - PRIM_BASE))
        } else if n < UNDEFINED {
            CsKind::FontId(FontId::new((n - FONT_ID_BASE) as u16))
        } else if n == UNDEFINED {
            CsKind::Undefined
        } else {
            CsKind::Named(NameSlot(n - EXTRA_BASE + HASH_SIZE.cast_unsigned()))
        }
    }

    /// A name `\def` and friends may give a new meaning. WEB's test starts past
    /// `frozen_control_sequence`, so the inserted `\inaccessible` is one.
    pub fn is_redefinable(self) -> bool {
        matches!(
            self.kind(),
            CsKind::Active(_)
                | CsKind::Single(_)
                | CsKind::Null
                | CsKind::Named(_)
                | CsKind::Frozen(Frozen::Protection)
        )
    }

    /// Whether the control sequence has a name in the hash, as everything from
    /// the first bucket on does.
    #[inline]
    pub fn has_hash_text(self) -> bool {
        self.hash_index().is_some()
    }

    /// The control sequence's slot in the `hash` table, which holds the text of
    /// every one from the first bucket on.
    #[inline]
    pub fn hash_index(self) -> Option<usize> {
        (self.0.get() - 1)
            .checked_sub(HASH_BASE)
            .map(|i| i as usize)
    }

    #[inline(always)]
    pub const fn from_raw(raw: i32) -> Option<CsId> {
        // Negative values clamp to 0, so one zero test rejects both.
        let clamped = if raw < 0 { 0 } else { raw.cast_unsigned() };
        match NonZeroU32::new(clamped) {
            Some(n) => Some(CsId(n)),
            None => None,
        }
    }

    #[inline(always)]
    pub const fn raw(self) -> i32 {
        self.0.get().cast_signed()
    }

    #[inline(always)]
    pub fn index(self) -> usize {
        self.0.get() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inaccessible_is_redefinable_and_other_frozen_names_are_not() {
        assert!(Frozen::Protection.cs().is_redefinable());
        assert!(!Frozen::Relax.cs().is_redefinable());
        assert!(!Frozen::Fi.cs().is_redefinable());
    }

    #[test]
    fn regions_keep_webs_numbers() {
        assert_eq!(CsId::active(Usv::nth(0)).raw(), 1);
        assert_eq!(CsId::single(0).raw(), 1_114_113);
        assert_eq!(CsId::NULL.raw(), 1_179_649);
        assert_eq!(NameSlot::bucket(0).cs().raw(), 1_179_650);
        assert_eq!(Frozen::Protection.cs().raw(), 1_194_650);
        assert_eq!(CsId::font_id(FontId::new(0)).raw(), 1_196_762);
        assert_eq!(CsId::UNDEFINED.raw(), 1_205_763);
        assert_eq!(NameSlot::extra(1).cs().raw(), 1_205_764);
    }

    #[test]
    fn kind_round_trips_every_region() {
        let kinds = [
            CsKind::Active(Usv::nth(0)),
            CsKind::Active(Usv::nth(0x10_FFFF)),
            CsKind::Single(0),
            CsKind::Single(0xFFFF),
            CsKind::Null,
            CsKind::Named(NameSlot::bucket(0)),
            CsKind::Named(NameSlot::bucket(HASH_SIZE.cast_unsigned() - 1)),
            CsKind::Primitive(PrimNum::FIRST),
            CsKind::Primitive(PrimNum::LAST),
            CsKind::FontId(FontId::new(0)),
            CsKind::FontId(FontId::new(9000)),
            CsKind::Undefined,
            CsKind::Named(NameSlot::extra(1)),
            CsKind::Named(NameSlot::extra(600_000)),
        ];
        for kind in kinds.into_iter().chain(Frozen::ALL.map(CsKind::Frozen)) {
            assert_eq!(CsId::from(kind).kind(), kind);
        }
    }
}
