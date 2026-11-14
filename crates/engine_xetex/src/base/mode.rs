//! TeX's modes: what kind of list is being built, with the registers that list keeps.

use crate::base::consts::IGNORE_DEPTH;
use crate::base::units::Scaled;
use crate::memory::node::{FractionNoad, NodePtr};

/// The kind of list a mode builds.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum ListKind {
    Vertical,
    Horizontal,
    Math,
}

/// A mode without its registers, as diagnostics name it.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub(crate) enum ModeName {
    #[default]
    NoMode,
    Vertical,
    InternalVertical,
    Horizontal,
    RestrictedHorizontal,
    DisplayMath,
    Math,
}

impl ModeName {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            ModeName::NoMode => "no mode",
            ModeName::Vertical => "vertical mode",
            ModeName::InternalVertical => "internal vertical mode",
            ModeName::Horizontal => "horizontal mode",
            ModeName::RestrictedHorizontal => "restricted horizontal mode",
            ModeName::DisplayMath => "display math mode",
            ModeName::Math => "math mode",
        }
    }
}

/// The mode of one level of the semantic nest. `inner` is TeX's negative mode.
#[derive(Copy, Clone)]
pub(crate) enum Mode {
    Vertical {
        inner: bool,
        prev_depth: Scaled,
    },
    Horizontal {
        inner: bool,
        space_factor: i32,
        /// Only kept current in a paragraph.
        clang: i32,
    },
    Math {
        inner: bool,
        /// The fraction whose denominator the list is building.
        incompleat_noad: Option<NodePtr<FractionNoad>>,
    },
    /// Inside `\write`, where no list is being built.
    None,
}

impl Mode {
    /// A vertical list that ignores the depth of what came before.
    pub(crate) const fn vertical(inner: bool) -> Mode {
        Mode::Vertical {
            inner,
            prev_depth: IGNORE_DEPTH,
        }
    }

    /// A horizontal list after an ordinary character, with no language set.
    pub(crate) const fn horizontal(inner: bool) -> Mode {
        Mode::Horizontal {
            inner,
            space_factor: 1000,
            clang: 0,
        }
    }

    /// A math list with no fraction open.
    pub(crate) const fn math(inner: bool) -> Mode {
        Mode::Math {
            inner,
            incompleat_noad: None,
        }
    }

    /// The same list as an inner one, TeX's `-abs(mode)`.
    pub(crate) fn inner(self) -> Mode {
        match self {
            Mode::Vertical { prev_depth, .. } => Mode::Vertical {
                inner: true,
                prev_depth,
            },
            Mode::Horizontal {
                space_factor,
                clang,
                ..
            } => Mode::Horizontal {
                inner: true,
                space_factor,
                clang,
            },
            Mode::Math {
                incompleat_noad, ..
            } => Mode::Math {
                inner: true,
                incompleat_noad,
            },
            Mode::None => Mode::None,
        }
    }

    pub(crate) fn name(&self) -> ModeName {
        match *self {
            Mode::Vertical { inner: false, .. } => ModeName::Vertical,
            Mode::Vertical { inner: true, .. } => ModeName::InternalVertical,
            Mode::Horizontal { inner: false, .. } => ModeName::Horizontal,
            Mode::Horizontal { inner: true, .. } => ModeName::RestrictedHorizontal,
            Mode::Math { inner: false, .. } => ModeName::DisplayMath,
            Mode::Math { inner: true, .. } => ModeName::Math,
            Mode::None => ModeName::NoMode,
        }
    }

    pub(crate) fn kind(&self) -> Option<ListKind> {
        match self {
            Mode::Vertical { .. } => Some(ListKind::Vertical),
            Mode::Horizontal { .. } => Some(ListKind::Horizontal),
            Mode::Math { .. } => Some(ListKind::Math),
            Mode::None => None,
        }
    }

    #[inline]
    pub(crate) fn is_vertical(&self) -> bool {
        matches!(self, Mode::Vertical { .. })
    }

    #[inline]
    pub(crate) fn is_horizontal(&self) -> bool {
        matches!(self, Mode::Horizontal { .. })
    }

    #[inline]
    pub(crate) fn is_math(&self) -> bool {
        matches!(self, Mode::Math { .. })
    }

    /// Internal vertical, restricted horizontal or non-display math.
    #[inline]
    pub(crate) fn is_inner(&self) -> bool {
        matches!(
            self,
            Mode::Vertical { inner: true, .. }
                | Mode::Horizontal { inner: true, .. }
                | Mode::Math { inner: true, .. }
        )
    }

    /// The page or an output routine's list, a paragraph, or a display.
    #[inline]
    pub(crate) fn is_outer(&self) -> bool {
        matches!(
            self,
            Mode::Vertical { inner: false, .. }
                | Mode::Horizontal { inner: false, .. }
                | Mode::Math { inner: false, .. }
        )
    }

    /// The paragraph being built, not a restricted horizontal list.
    #[inline]
    pub(crate) fn is_paragraph(&self) -> bool {
        matches!(self, Mode::Horizontal { inner: false, .. })
    }
}
