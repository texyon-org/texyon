//! Shared compile-time constants for the XeTeX engine, mirroring the WEB `@d` definitions.

use crate::cmd::Cmd;
use crate::units::EqtbPtr;

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
pub const TEX_NULL: i32 = MIN_HALFWORD;
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

pub const VMODE: i16 = 1;
pub const HMODE: i16 = VMODE + Cmd::MAX_COMMAND as i16 + 1;
pub const MMODE: i16 = HMODE + Cmd::MAX_COMMAND as i16 + 1;
pub const LEVEL_ZERO: u16 = 0;
pub const LEVEL_ONE: u16 = 1;

/// Node sizes, in 32-bit words.
pub const SMALL_NODE_SIZE: i32 = 4;
pub const MEDIUM_NODE_SIZE: i32 = 6;
pub const RULE_NODE_SIZE: i32 = 10;
pub const BOX_NODE_SIZE: i32 = 16;
pub const INS_NODE_SIZE: i32 = 10;
pub const PAGE_INS_NODE_SIZE: i32 = 8;
pub const MARGIN_KERN_NODE_SIZE: i32 = 6;
pub const GLYPH_NODE_SIZE: i32 = 10;
pub const NATIVE_NODE_SIZE: i32 = 12;
pub const OPEN_NODE_SIZE: i32 = 6;
pub const WRITE_NODE_SIZE: i32 = 4;
pub const PIC_NODE_SIZE: i32 = 18;
pub const INDEX_NODE_SIZE: i32 = 66;
pub const MARK_CLASS_NODE_SIZE: i32 = 8;
pub const STYLE_NODE_SIZE: i32 = 6;
pub const NOAD_SIZE: i32 = 8;
pub const ACCENT_NOAD_SIZE: i32 = 10;
pub const RADICAL_NOAD_SIZE: i32 = 10;
pub const FRACTION_NOAD_SIZE: i32 = 12;

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
pub const ACTIVE_BASE: EqtbPtr = EqtbPtr::from_raw(1);
pub const SINGLE_BASE: EqtbPtr = ACTIVE_BASE.plus(NUMBER_USVS);
pub const NULL_CS: EqtbPtr = SINGLE_BASE.plus(TOO_BIG_CHAR);
pub const HASH_BASE: EqtbPtr = NULL_CS.plus(1);
pub const FROZEN_CONTROL_SEQUENCE: EqtbPtr = HASH_BASE.plus(crate::hash::HASH_SIZE);

pub const FROZEN_PROTECTION: EqtbPtr = FROZEN_CONTROL_SEQUENCE;
pub const FROZEN_CR: EqtbPtr = FROZEN_CONTROL_SEQUENCE.plus(1);
pub const FROZEN_END_GROUP: EqtbPtr = FROZEN_CONTROL_SEQUENCE.plus(2);
pub const FROZEN_RIGHT: EqtbPtr = FROZEN_CONTROL_SEQUENCE.plus(3);
pub const FROZEN_FI: EqtbPtr = FROZEN_CONTROL_SEQUENCE.plus(4);
pub const FROZEN_END_TEMPLATE: EqtbPtr = FROZEN_CONTROL_SEQUENCE.plus(5);
pub const FROZEN_ENDV: EqtbPtr = FROZEN_CONTROL_SEQUENCE.plus(6);
pub const FROZEN_RELAX: EqtbPtr = FROZEN_CONTROL_SEQUENCE.plus(7);
pub const END_WRITE: EqtbPtr = FROZEN_CONTROL_SEQUENCE.plus(8);
pub const FROZEN_DONT_EXPAND: EqtbPtr = FROZEN_CONTROL_SEQUENCE.plus(9);
pub const FROZEN_SPECIAL: EqtbPtr = FROZEN_CONTROL_SEQUENCE.plus(10);
pub const FROZEN_PRIMITIVE: EqtbPtr = FROZEN_CONTROL_SEQUENCE.plus(11);
pub const PRIM_EQTB_BASE: EqtbPtr = FROZEN_PRIMITIVE.plus(1);
pub const PRIM_SIZE: i32 = 2100;
pub const FROZEN_NULL_FONT: EqtbPtr = PRIM_EQTB_BASE.plus(PRIM_SIZE);
pub const FONT_ID_BASE: EqtbPtr = FROZEN_NULL_FONT.plus(-FONT_BASE);
pub const UNDEFINED_CONTROL_SEQUENCE: EqtbPtr = FROZEN_NULL_FONT.plus(MAX_FONT_MAX + 1);

pub const GLUE_BASE: EqtbPtr = UNDEFINED_CONTROL_SEQUENCE.plus(1);
pub const SKIP_BASE: EqtbPtr = GLUE_BASE.plus(19);
pub const MU_SKIP_BASE: EqtbPtr = SKIP_BASE.plus(256);
pub const LOCAL_BASE: EqtbPtr = MU_SKIP_BASE.plus(256);
pub const TOKS_BASE: EqtbPtr = LOCAL_BASE.plus(13);
pub const ETEX_PEN_BASE: EqtbPtr = TOKS_BASE.plus(256);
pub const BOX_BASE: EqtbPtr = ETEX_PEN_BASE.plus(4);
pub const CUR_FONT_LOC: EqtbPtr = BOX_BASE.plus(256);
pub const MATH_FONT_BASE: EqtbPtr = CUR_FONT_LOC.plus(1);
pub const CAT_CODE_BASE: EqtbPtr = MATH_FONT_BASE.plus(3 * NUMBER_MATH_FAMILIES);
pub const LC_CODE_BASE: EqtbPtr = CAT_CODE_BASE.plus(NUMBER_USVS);
pub const UC_CODE_BASE: EqtbPtr = LC_CODE_BASE.plus(NUMBER_USVS);
pub const SF_CODE_BASE: EqtbPtr = UC_CODE_BASE.plus(NUMBER_USVS);
pub const MATH_CODE_BASE: EqtbPtr = SF_CODE_BASE.plus(NUMBER_USVS);
pub const INT_BASE: EqtbPtr = MATH_CODE_BASE.plus(NUMBER_USVS);
pub const COUNT_BASE: EqtbPtr = INT_BASE.plus(86);
pub const DEL_CODE_BASE: EqtbPtr = COUNT_BASE.plus(256);
pub const DIMEN_BASE: EqtbPtr = DEL_CODE_BASE.plus(NUMBER_USVS);
pub const SCALED_BASE: EqtbPtr = DIMEN_BASE.plus(23);
pub const EQTB_SIZE: EqtbPtr = SCALED_BASE.plus(255);
/// Entries in `xeq_level`, one per location from `INT_BASE` to `EQTB_SIZE`.
pub const XEQ_LEVEL_SIZE: usize = (EQTB_SIZE.raw() - INT_BASE.raw() + 1) as usize;

const _: () = {
    assert!(FONT_BASE == 0 && NULL_FONT.get() == 0);
    assert!(HMODE == 105 && MMODE == 209);
    assert!(SINGLE_BASE.raw() == 1_114_113);
    assert!(NULL_CS.raw() == 1_179_649);
    assert!(HASH_BASE.raw() == 1_179_650);
    assert!(PRIM_EQTB_BASE.raw() == 1_194_662);
    assert!(FROZEN_NULL_FONT.raw() == 1_196_762);
    assert!(UNDEFINED_CONTROL_SEQUENCE.raw() == 1_205_763);
    assert!(GLUE_BASE.raw() == 1_205_764);
    assert!(SKIP_BASE.raw() == 1_205_783);
    assert!(MU_SKIP_BASE.raw() == 1_206_039);
    assert!(LOCAL_BASE.raw() == 1_206_295);
    assert!(TOKS_BASE.raw() == 1_206_308);
    assert!(ETEX_PEN_BASE.raw() == 1_206_564);
    assert!(BOX_BASE.raw() == 1_206_568);
    assert!(CUR_FONT_LOC.raw() == 1_206_824);
    assert!(MATH_FONT_BASE.raw() == 1_206_825);
    assert!(CAT_CODE_BASE.raw() == 1_207_593);
    assert!(LC_CODE_BASE.raw() == 2_321_705);
    assert!(UC_CODE_BASE.raw() == 3_435_817);
    assert!(SF_CODE_BASE.raw() == 4_549_929);
    assert!(MATH_CODE_BASE.raw() == 5_664_041);
    assert!(INT_BASE.raw() == 6_778_153);
    assert!(COUNT_BASE.raw() == 6_778_239);
    assert!(DEL_CODE_BASE.raw() == 6_778_495);
    assert!(DIMEN_BASE.raw() == 7_892_607);
    assert!(SCALED_BASE.raw() == 7_892_630);
    assert!(EQTB_SIZE.raw() == 7_892_885);
    assert!(XEQ_LEVEL_SIZE == 1_114_733);
};

/// `\parshape`, first in the local region.
pub const PAR_SHAPE_LOC: EqtbPtr = LOCAL_BASE;

// Offsets from `ETEX_PEN_BASE`.
pub const ETEX_PENALTIES_PAR_INTER_LINE_PENALTIES: i32 = 0;
pub const ETEX_PENALTIES_PAR_CLUB_PENALTIES: i32 = 1;
pub const ETEX_PENALTIES_PAR_WIDOW_PENALTIES: i32 = 2;
pub const ETEX_PENALTIES_PAR_DISPLAY_WIDOW_PENALTIES: i32 = 3;
