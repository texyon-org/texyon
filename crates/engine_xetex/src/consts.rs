//! Shared compile-time constants for the XeTeX engine, mirroring the WEB `@d` definitions.

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
pub const MEM_TOP: i32 = 4_999_999;
pub const FONT_BASE: i32 = 0;
pub const NULL_FONT: crate::units::FontId = crate::units::FontId::from_raw(FONT_BASE);
/// The `\font` size that means the design size, `scaled 1000`.
pub const DESIGN_SIZE: crate::units::Scaled = crate::units::Scaled::from_raw(-1000);
pub const MAX_FONT_MAX: i32 = 9_000;
pub const NON_ADDRESS: i32 = 0;
pub const MIN_TRIE_OP: u16 = 0;
pub const INVALID_HANDLE: usize = 0;

pub const HISTORY_SPOTLESS: i32 = 0;
pub const HISTORY_WARNING_ISSUED: i32 = 1;
pub const HISTORY_ERROR_ISSUED: i32 = 2;
pub const HISTORY_FATAL_ERROR: i32 = 3;
pub const JUST_OPEN: u8 = 1;
pub const CLOSED: u8 = 2;

pub const VMODE: i16 = 1;
pub const HMODE: i16 = 104;
pub const MMODE: i16 = 207;
pub const LEVEL_ZERO: u16 = 0;
pub const LEVEL_ONE: u16 = 1;
pub const LEFT_TO_RIGHT: i16 = 0;
pub const RIGHT_TO_LEFT: i16 = 1;
pub const LEFT_SIDE: i32 = 0;
pub const RIGHT_SIDE: i32 = 1;

pub const SMALL_NODE_SIZE: i32 = 2;
pub const MEDIUM_NODE_SIZE: i32 = 3;
pub const GLUE_SPEC_SIZE: i32 = 4;
pub const RULE_NODE_SIZE: i32 = 5;
pub const BOX_NODE_SIZE: i32 = 8;
pub const INS_NODE_SIZE: i32 = 5;
pub const MARGIN_KERN_NODE_SIZE: i32 = 3;
pub const GLYPH_NODE_SIZE: i32 = 5;
pub const NATIVE_NODE_SIZE: i32 = 6;
pub const OPEN_NODE_SIZE: i32 = 3;
pub const WRITE_NODE_SIZE: i32 = 2;
pub const PIC_NODE_SIZE: i32 = 9;
pub const INDEX_NODE_SIZE: i32 = 33;
pub const MARK_CLASS_NODE_SIZE: i32 = 4;
pub const STYLE_NODE_SIZE: i32 = 3;
pub const NOAD_SIZE: i32 = 4;
pub const ACCENT_NOAD_SIZE: i32 = 5;
pub const RADICAL_NOAD_SIZE: i32 = 5;
pub const FRACTION_NOAD_SIZE: i32 = 6;

// Node subtypes.
pub const NORMAL: u16 = 0;
pub const EXPLICIT: u16 = 1;
pub const ACC_KERN: u16 = 2;
pub const SPACE_ADJUSTMENT: u16 = 3;
pub const DLIST: u16 = 2;
pub const COND_MATH_GLUE: u16 = 98;
pub const MU_GLUE: u16 = 99;
pub const A_LEADERS: u16 = 100;
pub const C_LEADERS: u16 = A_LEADERS + 1;
pub const X_LEADERS: u16 = A_LEADERS + 2;
pub const BEFORE: u16 = 0;
pub const AFTER: u16 = 1;
pub const BEGIN_M_CODE: u16 = 2;
pub const END_M_CODE: u16 = 3;
pub const L_CODE: u16 = 4;
pub const R_CODE: u16 = 8;
pub const BEGIN_L_CODE: u16 = L_CODE + BEGIN_M_CODE;
pub const END_L_CODE: u16 = L_CODE + END_M_CODE;
pub const BEGIN_R_CODE: u16 = R_CODE + BEGIN_M_CODE;

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
pub const NULL_CS: EqtbPtr = SINGLE_BASE.plus(NUMBER_USVS);
pub const HASH_BASE: EqtbPtr = NULL_CS.plus(1);
pub const FROZEN_CONTROL_SEQUENCE: EqtbPtr = EqtbPtr::from_raw(2_243_226);

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
pub const COUNT_BASE: EqtbPtr = INT_BASE.plus(83);
pub const DEL_CODE_BASE: EqtbPtr = COUNT_BASE.plus(256);
pub const DIMEN_BASE: EqtbPtr = DEL_CODE_BASE.plus(NUMBER_USVS);
pub const SCALED_BASE: EqtbPtr = DIMEN_BASE.plus(23);
pub const EQTB_SIZE: EqtbPtr = SCALED_BASE.plus(255);

const _: () = {
    assert!(SINGLE_BASE.raw() == 1_114_113);
    assert!(NULL_CS.raw() == 2_228_225);
    assert!(HASH_BASE.raw() == 2_228_226);
    assert!(PRIM_EQTB_BASE.raw() == 2_243_238);
    assert!(FROZEN_NULL_FONT.raw() == 2_245_338);
    assert!(UNDEFINED_CONTROL_SEQUENCE.raw() == 2_254_339);
    assert!(GLUE_BASE.raw() == 2_254_340);
    assert!(SKIP_BASE.raw() == 2_254_359);
    assert!(MU_SKIP_BASE.raw() == 2_254_615);
    assert!(LOCAL_BASE.raw() == 2_254_871);
    assert!(TOKS_BASE.raw() == 2_254_884);
    assert!(ETEX_PEN_BASE.raw() == 2_255_140);
    assert!(BOX_BASE.raw() == 2_255_144);
    assert!(CUR_FONT_LOC.raw() == 2_255_400);
    assert!(MATH_FONT_BASE.raw() == 2_255_401);
    assert!(CAT_CODE_BASE.raw() == 2_256_169);
    assert!(LC_CODE_BASE.raw() == 3_370_281);
    assert!(UC_CODE_BASE.raw() == 4_484_393);
    assert!(SF_CODE_BASE.raw() == 5_598_505);
    assert!(MATH_CODE_BASE.raw() == 6_712_617);
    assert!(INT_BASE.raw() == 7_826_729);
    assert!(COUNT_BASE.raw() == 7_826_812);
    assert!(DEL_CODE_BASE.raw() == 7_827_068);
    assert!(DIMEN_BASE.raw() == 8_941_180);
    assert!(SCALED_BASE.raw() == 8_941_203);
    assert!(EQTB_SIZE.raw() == 8_941_458);
};

/// `\parshape`, first in the local region.
pub const PAR_SHAPE_LOC: EqtbPtr = LOCAL_BASE;

// Offsets from `ETEX_PEN_BASE`.
pub const ETEX_PENALTIES_PAR_INTER_LINE_PENALTIES: i32 = 0;
pub const ETEX_PENALTIES_PAR_CLUB_PENALTIES: i32 = 1;
pub const ETEX_PENALTIES_PAR_WIDOW_PENALTIES: i32 = 2;
pub const ETEX_PENALTIES_PAR_DISPLAY_WIDOW_PENALTIES: i32 = 3;
