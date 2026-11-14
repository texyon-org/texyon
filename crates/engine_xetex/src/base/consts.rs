//! Shared compile-time constants for the XeTeX engine, mirroring the WEB `@d` definitions.

use crate::base::units::{FontId, Scaled, StrNumber};

pub const BIGGEST_CHAR: i32 = 0xFFFF;
pub const TOO_BIG_CHAR: i32 = 65_536;
/// Pool string 65536 marks the start of the pool, so the empty string is the one after it.
pub const EMPTY_STRING: StrNumber = StrNumber::from_raw(TOO_BIG_CHAR + 1);
pub const BIGGEST_USV: i32 = 0x10_FFFF;
pub const NUMBER_USVS: i32 = BIGGEST_USV + 1;
pub const TOO_BIG_USV: i32 = BIGGEST_USV + 1;
pub const ACTIVE_MATH_CHAR: i32 = 0x1F_FFFF;
pub const BIGGEST_LANG: i32 = 255;
pub const CHAR_CLASS_LIMIT: i32 = 4096;

pub const MAX_HALFWORD: i32 = 0x3FFF_FFFF;
/// `max_dimen`, the largest legal dimension.
pub const MAX_DIMEN: Scaled = Scaled::from_raw(0x3FFF_FFFF);
pub const MIN_HALFWORD: i32 = -0x0FFF_FFFF;
pub const TEX_NULL: i32 = 0;
pub const NULL_FLAG: i32 = -0x4000_0000;
pub const IGNORE_DEPTH: Scaled = Scaled::from_raw(-65_536_000);
pub const TEX_INFINITY: i32 = 0x7FFF_FFFF;
/// `TeX_infinity` as a length, for positions that are not set.
pub const INFINITE_DIMEN: Scaled = Scaled::from_raw(TEX_INFINITY);
pub const INF_BAD: i32 = 10_000;
pub const INF_PENALTY: i32 = INF_BAD;
pub const EJECT_PENALTY: i32 = -INF_PENALTY;
pub const AWFUL_BAD: i32 = MAX_HALFWORD;
/// The cap on dynamic memory in 32-bit words, reported as `main memory size`. It stops runaway
/// documents long before a browser tab runs out, and is about 6.7 times TeX Live's.
pub const MAIN_MEMORY_WORDS: i32 = 1 << 26;
pub const FONT_BASE: i32 = 0;
pub const NULL_FONT: FontId = FontId::new(0);
/// The `\font` size that means the design size, `scaled 1000`.
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

pub const PRIM_SIZE: i32 = 2100;

const _: () = {
    assert!(FONT_BASE == 0 && NULL_FONT.get() == 0);
};
