//! Shared compile-time constants for the XeTeX engine, mirroring the WEB `@d` definitions.

pub const BIGGEST_CHAR: i32 = 0xFFFF;
pub const NUMBER_USVS: i32 = 0x10_FFFF + 1; // 1,114,112
pub const TOO_BIG_CHAR: i32 = 65_536;

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
