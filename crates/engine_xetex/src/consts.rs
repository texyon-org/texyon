//! Shared compile-time constants for the XeTeX engine, mirroring the WEB `@d` definitions.

use crate::units::CsId;

pub const BIGGEST_CHAR: i32 = 0xFFFF;
pub const TOO_BIG_CHAR: i32 = 65_536;
/// Pool string 65536 marks the start of the pool, so the empty string is the one after it.
pub const EMPTY_STRING: crate::units::StrNumber =
    crate::units::StrNumber::from_raw(TOO_BIG_CHAR + 1);
pub const BIGGEST_USV: i32 = 0x10_FFFF;
pub const NUMBER_USVS: i32 = BIGGEST_USV + 1;
pub const TOO_BIG_USV: i32 = BIGGEST_USV + 1;
pub const ACTIVE_MATH_CHAR: i32 = 0x1F_FFFF;
pub const BIGGEST_LANG: i32 = 255;
pub const CHAR_CLASS_LIMIT: i32 = 4096;
pub const CS_TOKEN_FLAG: i32 = 0x1FF_FFFF;

pub const MAX_HALFWORD: i32 = 0x3FFF_FFFF;
/// `max_dimen`, the largest legal dimension.
pub const MAX_DIMEN: crate::units::Scaled = crate::units::Scaled::from_raw(0x3FFF_FFFF);
pub const MIN_HALFWORD: i32 = -0x0FFF_FFFF;
pub const TEX_NULL: i32 = 0;
pub const NULL_FLAG: i32 = -0x4000_0000;
pub const IGNORE_DEPTH: crate::units::Scaled = crate::units::Scaled::from_raw(-65_536_000);
pub const TEX_INFINITY: i32 = 0x7FFF_FFFF;
/// `TeX_infinity` as a length, for positions that are not set.
pub const INFINITE_DIMEN: crate::units::Scaled = crate::units::Scaled::from_raw(TEX_INFINITY);
pub const INF_BAD: i32 = 10_000;
pub const INF_PENALTY: i32 = INF_BAD;
pub const EJECT_PENALTY: i32 = -INF_PENALTY;
pub const AWFUL_BAD: i32 = MAX_HALFWORD;
/// The cap on dynamic memory in 32-bit words, reported as `main memory size`. It stops runaway
/// documents long before a browser tab runs out, and is about 6.7 times TeX Live's.
pub const MAIN_MEMORY_WORDS: i32 = 1 << 26;
pub const FONT_BASE: i32 = 0;
pub const NULL_FONT: crate::units::FontId = crate::units::FontId::new(0);
/// The `\font` size that means the design size, `scaled 1000`.
pub const DESIGN_SIZE: crate::units::Scaled = crate::units::Scaled::from_raw(-1000);
pub const MAX_FONT_MAX: i32 = 9_000;
pub const INVALID_HANDLE: usize = 0;

pub const HISTORY_SPOTLESS: i32 = 0;
pub const HISTORY_WARNING_ISSUED: i32 = 1;
pub const HISTORY_ERROR_ISSUED: i32 = 2;
pub const HISTORY_FATAL_ERROR: i32 = 3;
pub const JUST_OPEN: u8 = 1;
pub const CLOSED: u8 = 2;

pub const LEVEL_ZERO: u16 = 0;
pub const LEVEL_ONE: u16 = 1;

// Node subtypes.
pub const NORMAL: u16 = 0;

// Math noad field types and sizes.
pub const EMPTY: i32 = 0;
pub const MATH_CHAR: i32 = 1;
pub const SUB_BOX: i32 = 2;
pub const SUB_MLIST: i32 = 3;
pub const MATH_TEXT_CHAR: i32 = 4;
pub const TEXT_SIZE: i32 = 0;
pub const SCRIPT_SIZE: i32 = 256;
pub const NUMBER_MATH_FAMILIES: i32 = 256;

// The eqtb regions.
pub const ACTIVE_BASE: CsId = CsId::from_raw(1);
pub const SINGLE_BASE: CsId = ACTIVE_BASE.plus(NUMBER_USVS);
pub const NULL_CS: CsId = SINGLE_BASE.plus(TOO_BIG_CHAR);
pub const HASH_BASE: CsId = NULL_CS.plus(1);
pub const FROZEN_CONTROL_SEQUENCE: CsId = HASH_BASE.plus(crate::hash::HASH_SIZE);

pub const FROZEN_PROTECTION: CsId = FROZEN_CONTROL_SEQUENCE;
pub const FROZEN_CR: CsId = FROZEN_CONTROL_SEQUENCE.plus(1);
pub const FROZEN_END_GROUP: CsId = FROZEN_CONTROL_SEQUENCE.plus(2);
pub const FROZEN_RIGHT: CsId = FROZEN_CONTROL_SEQUENCE.plus(3);
pub const FROZEN_FI: CsId = FROZEN_CONTROL_SEQUENCE.plus(4);
pub const FROZEN_END_TEMPLATE: CsId = FROZEN_CONTROL_SEQUENCE.plus(5);
pub const FROZEN_ENDV: CsId = FROZEN_CONTROL_SEQUENCE.plus(6);
pub const FROZEN_RELAX: CsId = FROZEN_CONTROL_SEQUENCE.plus(7);
pub const END_WRITE: CsId = FROZEN_CONTROL_SEQUENCE.plus(8);
pub const FROZEN_DONT_EXPAND: CsId = FROZEN_CONTROL_SEQUENCE.plus(9);
pub const FROZEN_SPECIAL: CsId = FROZEN_CONTROL_SEQUENCE.plus(10);
pub const FROZEN_PRIMITIVE: CsId = FROZEN_CONTROL_SEQUENCE.plus(11);
pub const PRIM_EQTB_BASE: CsId = FROZEN_PRIMITIVE.plus(1);
pub const PRIM_SIZE: i32 = 2100;
pub const FROZEN_NULL_FONT: CsId = PRIM_EQTB_BASE.plus(PRIM_SIZE);
pub const FONT_ID_BASE: CsId = FROZEN_NULL_FONT.plus(-FONT_BASE);
pub const UNDEFINED_CONTROL_SEQUENCE: CsId = FROZEN_NULL_FONT.plus(MAX_FONT_MAX + 1);

const _: () = {
    assert!(FONT_BASE == 0 && NULL_FONT.get() == 0);
    assert!(SINGLE_BASE.raw() == 1_114_113);
    assert!(NULL_CS.raw() == 1_179_649);
    assert!(HASH_BASE.raw() == 1_179_650);
    assert!(PRIM_EQTB_BASE.raw() == 1_194_662);
    assert!(FROZEN_NULL_FONT.raw() == 1_196_762);
    assert!(UNDEFINED_CONTROL_SEQUENCE.raw() == 1_205_763);
};
