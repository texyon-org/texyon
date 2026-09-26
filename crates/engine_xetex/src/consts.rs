//! Shared compile-time constants for the XeTeX engine, mirroring the WEB `@d` definitions.

pub const BIGGEST_CHAR: i32 = 0xFFFF;
pub const NUMBER_USVS: i32 = 0x10_FFFF + 1; // 1,114,112
pub const TOO_BIG_CHAR: i32 = 65_536;
/// The empty string's `str_number` (xetex-stringpool.h:12). NOT `TOO_BIG_CHAR`
/// itself: pool string 65536 is the "this marks the start of the stringpool"
/// marker; the empty string is the one after it.
pub const EMPTY_STRING: i32 = TOO_BIG_CHAR + 1;

pub const CS_TOKEN_FLAG: i32 = 0x1FFF_FFF;
pub const MAX_HALFWORD: i32 = 0x3FFF_FFFF;
pub const MIN_HALFWORD: i32 = -0x0FFF_FFFF;
pub const TEX_NULL: i32 = -0x0FFF_FFFF; // MIN_HALFWORD
pub const NULL_FLAG: i32 = -0x4000_0000;

pub const SMALL_NODE_SIZE: i32 = 2;
pub const MEDIUM_NODE_SIZE: i32 = 3;
pub const GLUE_SPEC_SIZE: i32 = 4;
pub const RULE_NODE_SIZE: i32 = 5;
pub const BOX_NODE_SIZE: i32 = 8;
pub const MARGIN_KERN_NODE_SIZE: i32 = 3;

pub const HLIST_NODE: u16 = 0;
pub const RULE_NODE: u16 = 2;
pub const LIGATURE_NODE: u16 = 6;
pub const DISC_NODE: u16 = 7;
pub const MATH_NODE: u16 = 9;
pub const GLUE_NODE: u16 = 10;
pub const KERN_NODE: u16 = 11;
pub const PENALTY_NODE: u16 = 12;
pub const MARGIN_KERN_NODE: u16 = 40;

pub const SELECTOR_TERM_ONLY: i32 = 17;
pub const SELECTOR_TERM_AND_LOG: i32 = 19;

// eqtb bases & parameter counts
pub const ACTIVE_BASE: i32 = 1;
pub const SINGLE_BASE: i32 = ACTIVE_BASE + NUMBER_USVS; // 1_114_113
pub const PRIM_EQTB_BASE: i32 = 2_243_238;
pub const GLUE_BASE: i32 = 2_254_340;
pub const LOCAL_BASE: i32 = 2_254_871;
pub const TOKS_BASE: i32 = 2_254_884;
pub const ETEX_PEN_BASE: i32 = TOKS_BASE + 256;
pub const BOX_BASE: i32 = ETEX_PEN_BASE + 4;
pub const CUR_FONT_LOC: i32 = BOX_BASE + 256;
pub const MATH_FONT_BASE: i32 = CUR_FONT_LOC + 1;
pub const CAT_CODE_BASE: i32 = MATH_FONT_BASE + 768; // 3 * 256 number math family fonts
pub const LC_CODE_BASE: i32 = CAT_CODE_BASE + NUMBER_USVS;
pub const UC_CODE_BASE: i32 = LC_CODE_BASE + NUMBER_USVS;
pub const SF_CODE_BASE: i32 = UC_CODE_BASE + NUMBER_USVS;
pub const MATH_CODE_BASE: i32 = SF_CODE_BASE + NUMBER_USVS;
pub const INT_BASE: i32 = MATH_CODE_BASE + NUMBER_USVS;
pub const DEL_CODE_BASE: i32 = INT_BASE + 339; // number of int pars
pub const DIMEN_BASE: i32 = DEL_CODE_BASE + NUMBER_USVS;
pub const EQTB_SIZE: i32 = DIMEN_BASE + 278; // number of dimen pars

// Static assertions to verify computed values are correct
const _: () = {
    assert!(ACTIVE_BASE == 1);
    assert!(SINGLE_BASE == 1_114_113);
    assert!(PRIM_EQTB_BASE == 2_243_238);
    assert!(GLUE_BASE == 2_254_340);
    assert!(LOCAL_BASE == 2_254_871);
    assert!(TOKS_BASE == 2_254_884);
    assert!(ETEX_PEN_BASE == 2_255_140);
    assert!(BOX_BASE == 2_255_144);
    assert!(CUR_FONT_LOC == 2_255_400);
    assert!(MATH_FONT_BASE == 2_255_401);
    assert!(CAT_CODE_BASE == 2_256_169);
    assert!(LC_CODE_BASE == 3_370_281);
    assert!(UC_CODE_BASE == 4_484_393);
    assert!(SF_CODE_BASE == 5_598_505);
    assert!(MATH_CODE_BASE == 6_712_617);
    assert!(INT_BASE == 7_826_729);
    assert!(DEL_CODE_BASE == 7_827_068);
    assert!(DIMEN_BASE == 8_941_180);
    assert!(EQTB_SIZE == 8_941_458);
};

// ---------------------------------------------------------------------------
// Consolidated constants (formerly duplicated per-file; single source of truth)
// ---------------------------------------------------------------------------
pub const ACCENT_NOAD_SIZE: i32 = 5;
pub const AWFUL_BAD: i32 = MAX_HALFWORD;
pub const A_LEADERS: u16 = 100;
pub const BACKED_UP: u16 = 3;
pub const BEFORE: i32 = 0;
pub const BIGGEST_LANG: i32 = 255;
pub const CHAR_CLASS_LIMIT: i32 = 4096;
pub const CLOSED: u8 = 2;
pub const COUNT_BASE: i32 = 7_826_812;
pub const DIMEN_PAR_HANG_INDENT: i32 = 17;
pub const DIMEN_PAR_HSIZE: i32 = 3;
pub const DLIST: u16 = 2;
pub const EJECT_PENALTY: i32 = -10_000;
pub const ELSE_CODE: u8 = 3;
pub const EXPLICIT: u16 = 1;
pub const FI_CODE: u8 = 2;
pub const FONT_BASE: i32 = 0;
pub const FRACTION_NOAD_SIZE: i32 = 6;
pub const FROZEN_CONTROL_SEQUENCE: i32 = 2_243_226;
pub const FROZEN_ENDV: i32 = 2_243_226 + 6;
pub const FROZEN_PRIMITIVE: i32 = 2_243_237;
pub const GLUE_VAL: i16 = 2;
pub const HISTORY_FATAL_ERROR: i32 = 3;
pub const HISTORY_SPOTLESS: i32 = 0;
pub const HMODE: i16 = 104;
pub const IGNORE_DEPTH: i32 = -65_536_000;
pub const INSERTED: u16 = 5;
pub const INT_PAR_DAY: i32 = 21;
pub const INT_PAR_END_LINE_CHAR: i32 = 48;
pub const INT_PAR_HANG_AFTER: i32 = 41;
pub const INT_PAR_LANGUAGE: i32 = 50;
pub const INT_PAR_LEFT_HYPHEN_MIN: i32 = 51;
pub const INT_PAR_LOOSENESS: i32 = 19;
pub const INT_PAR_MAG: i32 = 17;
pub const INT_PAR_MONTH: i32 = 22;
pub const INT_PAR_NEW_LINE_CHAR: i32 = 49;
pub const INT_PAR_RIGHT_HYPHEN_MIN: i32 = 52;
pub const INT_PAR_TEXXET: i32 = 69;
pub const INT_PAR_TRACING_IFS: i32 = 58;
pub const INT_PAR_TRACING_ONLINE: i32 = 29;
pub const INT_PAR_XETEX_GENERATE_ACTUAL_TEXT: i32 = 79;
pub const INT_PAR_XETEX_TRACING_FONTS: i32 = 77;
pub const INT_PAR_YEAR: i32 = 23;
pub const INVALID_HANDLE: usize = 0;
pub const JUST_OPEN: u8 = 1;
pub const LEFT_SIDE: i32 = 0;
pub const LEFT_TO_RIGHT: i16 = 0;
pub const LEVEL_ONE: u16 = 1;
pub const LIG_TAG: u16 = 1;
pub const LOCAL_PAR_SHAPE: i32 = 0;
pub const MAX_FONT_MAX: i32 = 9_000;
pub const MEM_TOP: i32 = 4_999_999;
pub const MIN_TRIE_OP: i32 = 0;
pub const MMODE: i16 = 207;
pub const MU_GLUE: u16 = 99;
pub const MU_SKIP_BASE: i32 = 2_254_615;
pub const MU_VAL_SCAN: i16 = 3;
pub const NATIVE_GLYPH_INFO_SIZE: usize = 10;
pub const NEW_LINE: u16 = 33;
pub const NOAD_SIZE: i32 = 4;
pub const NON_ADDRESS: i32 = 0;
pub const NORMAL_SCANNER: u8 = 0;
pub const NULL_CS: i32 = 2_228_225;
pub const NUMBER_MATH_FAMILIES: i32 = 256;
pub const OR_CODE: u8 = 4;
pub const OUTPUT_TEXT: u16 = 7;
pub const PIC_NODE_SIZE: i32 = 9;
pub const RADICAL_NOAD_SIZE: i32 = 5;
pub const RIGHT_SIDE: i32 = 1;
pub const RIGHT_TO_LEFT: i16 = 1;
pub const SCALED_BASE: i32 = 8_941_203;
pub const SCRIPT_SIZE: i32 = 256;
pub const SELECTOR_LOG_ONLY: i32 = 18;
pub const SELECTOR_NEW_STRING: i32 = 21;
pub const SELECTOR_NO_PRINT: i32 = 16;
pub const SKIP_BASE: i32 = 2_254_359;
pub const SPACE_ADJUSTMENT: u16 = 3;
pub const SPACE_CODE: i32 = 2;
pub const STYLE_NODE_SIZE: i32 = 3;
pub const SUB_BOX: i32 = 2;
pub const TEXT_SIZE: i32 = 0;
pub const TOKEN_LIST: u16 = 0;
pub const UNDEFINED_CONTROL_SEQUENCE: i32 = 2_254_339;
pub const UNDEFINED_PRIMITIVE: i32 = 0;
pub const UNLESS_CODE: i32 = 32;
pub const VMODE: i16 = 1;
pub const WRITE_TEXT: u16 = 18;
pub const XETEX_INT: i32 = 27;
