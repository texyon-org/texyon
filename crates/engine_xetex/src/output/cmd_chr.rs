//! Symbolic printing of commands: `print_cmd_chr` and `print_meaning`.

use crate::cmd::{
    ABOVE, ACCENT, ADVANCE, AFTER_ASSIGNMENT, AFTER_GROUP, ASSIGN_DIMEN, ASSIGN_FONT_DIMEN,
    ASSIGN_FONT_INT, ASSIGN_GLUE, ASSIGN_INT, ASSIGN_MU_GLUE, ASSIGN_TOKS, BEGIN_GROUP,
    BREAK_PENALTY, CALL, CAR_RET, CASE_SHIFT, CHAR_GIVEN, CHAR_NUM, CONVERT, CS_NAME, DEF,
    DEF_CODE, DEF_FAMILY, DEF_FONT, DELIM_NUM, DISCRETIONARY, DIVIDE, ENDV, END_CS_NAME, END_GROUP,
    END_TEMPLATE, EQ_NO, EXPAND_AFTER, EXTENSION, EX_SPACE, FI_OR_ELSE, HALIGN, HMOVE, HRULE,
    HSKIP, HYPH_DATA, IF_TEST, IGNORE_SPACES, INPUT, INSERT, IN_STREAM, ITAL_CORR, KERN, LAST_ITEM,
    LEADER_SHIP, LEFT_BRACE, LEFT_RIGHT, LET, LETTER, LIMIT_SWITCH, LONG_CALL, LONG_OUTER_CALL,
    MAC_PARAM, MAKE_BOX, MARK, MATH_ACCENT, MATH_CHAR_NUM, MATH_CHOICE, MATH_COMP, MATH_GIVEN,
    MATH_SHIFT, MATH_STYLE, MESSAGE, MKERN, MSKIP, MULTIPLY, NON_SCRIPT, NO_ALIGN, NO_BOUNDARY,
    NO_EXPAND, OMIT, OTHER_CHAR, OUTER_CALL, PAR_END, PREFIX, RADICAL, READ_TO_CS, REGISTER, RELAX,
    REMOVE_ITEM, RIGHT_BRACE, SET_AUX, SET_BOX, SET_BOX_DIMEN, SET_FONT, SET_INTERACTION,
    SET_PAGE_DIMEN, SET_PAGE_INT, SET_PREV_GRAF, SET_SHAPE, SHORTHAND_DEF, SPACER, START_PAR, STOP,
    SUB_MARK, SUP_MARK, TAB_MARK, THE, TOKS_REGISTER, TOP_BOT_MARK, UNDEFINED_CS, UN_HBOX, UN_VBOX,
    VADJUST, VALIGN, VCENTER, VMOVE, VRULE, VSKIP, XETEX_DEF_CODE, XETEX_MATH_GIVEN, XRAY,
};
use crate::codes::{
    BoxCode, BoxDimenCode, ConvertCode, DelimNumCode, ExtensionCode, FiOrElseCode, IfCode,
    InputCode, LastItemCode, MarkCode, MathAccentCode, MathCharNumCode, PageIntCode, RadicalCode,
    ShorthandDefCode, ShowCode, SkipCode, TheCode, ValueLevel,
};
use crate::consts::{
    A_LEADERS, BEGIN_L_CODE, BEGIN_R_CODE, COUNT_BASE, DEL_CODE_BASE, DIMEN_BASE, END_L_CODE,
    ETEX_PEN_BASE, GLUE_BASE, INT_BASE, LC_CODE_BASE, LOCAL_BASE, MATH_CODE_BASE, MATH_FONT_BASE,
    MU_SKIP_BASE, PAR_SHAPE_LOC, SCALED_BASE, SF_CODE_BASE, SKIP_BASE, TOKS_BASE, UC_CODE_BASE,
    VMODE,
};
use crate::main_control::{CR_CODE, LP_CODE_BASE, RP_CODE_BASE, SPAN_CODE};
use crate::math::{
    ABOVE_CODE, ATOP_CODE, DELIMITED_CODE, LIMITS, MIDDLE_NOAD, NO_LIMITS, OVER_CODE,
};
use crate::node::{NodePtr, SaNode};
use crate::params::TokPar;
use crate::state::{EngineState, InteractionMode};
use crate::units::{FontId, MemPtr, NodeType};

use super::{
    print, print_char, print_code, print_esc, print_font_select, print_hex, print_int,
    print_length_param, print_ln, print_param, print_sa_num, print_size, print_skip_param,
    print_style, token_show,
};

pub(crate) fn print_cmd_chr(st: &mut EngineState, cmd: u8, chr_code: i32) {
    let char_cmd = |st: &mut EngineState, s: &str| {
        print(st, s);
        print_code(st, chr_code);
    };

    match cmd {
        LEFT_BRACE => char_cmd(st, "begin-group character "),
        RIGHT_BRACE => char_cmd(st, "end-group character "),
        MATH_SHIFT => char_cmd(st, "math shift character "),
        MAC_PARAM => char_cmd(st, "macro parameter character "),
        SUP_MARK => char_cmd(st, "superscript character "),
        SUB_MARK => char_cmd(st, "subscript character "),
        ENDV => print(st, "end of alignment template"),
        SPACER => char_cmd(st, "blank space "),
        LETTER => char_cmd(st, "the letter "),
        OTHER_CHAR => char_cmd(st, "the character "),
        ASSIGN_GLUE | ASSIGN_MU_GLUE => {
            if chr_code < SKIP_BASE {
                print_skip_param(st, chr_code - GLUE_BASE);
            } else if chr_code < MU_SKIP_BASE {
                print_esc(st, "skip");
                print_int(st, chr_code - SKIP_BASE);
            } else {
                print_esc(st, "muskip");
                print_int(st, chr_code - MU_SKIP_BASE);
            }
        }
        ASSIGN_TOKS => {
            if chr_code >= TOKS_BASE {
                print_esc(st, "toks");
                print_int(st, chr_code - TOKS_BASE);
            } else {
                let par = TokPar::from_raw(chr_code - LOCAL_BASE).unwrap_or(TokPar::ErrHelp);
                print_esc(st, par.name());
            }
        }
        ASSIGN_INT => {
            if chr_code < COUNT_BASE {
                print_param(st, chr_code - INT_BASE);
            } else {
                print_esc(st, "count");
                print_int(st, chr_code - COUNT_BASE);
            }
        }
        ASSIGN_DIMEN => {
            if chr_code < SCALED_BASE {
                print_length_param(st, chr_code - DIMEN_BASE);
            } else {
                print_esc(st, "dimen");
                print_int(st, chr_code - SCALED_BASE);
            }
        }
        ACCENT => print_esc(st, "accent"),
        ADVANCE => print_esc(st, "advance"),
        AFTER_ASSIGNMENT => print_esc(st, "afterassignment"),
        AFTER_GROUP => print_esc(st, "aftergroup"),
        ASSIGN_FONT_DIMEN => print_esc(st, "fontdimen"),
        BEGIN_GROUP => print_esc(st, "begingroup"),
        BREAK_PENALTY => print_esc(st, "penalty"),
        CHAR_NUM => print_esc(st, "char"),
        CS_NAME => print_esc(st, "csname"),
        DEF_FONT => print_esc(st, "font"),
        DELIM_NUM => print_esc(
            st,
            DelimNumCode::from_raw(chr_code).map_or("delimiter", DelimNumCode::name),
        ),
        DIVIDE => print_esc(st, "divide"),
        END_CS_NAME => print_esc(st, "endcsname"),
        END_GROUP => print_esc(st, "endgroup"),
        EX_SPACE => print_esc(st, b' '),
        EXPAND_AFTER => print_esc(
            st,
            if chr_code == 0 {
                "expandafter"
            } else {
                "unless"
            },
        ),
        HALIGN => print_esc(st, "halign"),
        HRULE => print_esc(st, "hrule"),
        IGNORE_SPACES => print_esc(
            st,
            if chr_code == 0 {
                "ignorespaces"
            } else {
                "primitive"
            },
        ),
        INSERT => print_esc(st, "insert"),
        ITAL_CORR => print_esc(st, b'/'),
        MARK => {
            print_esc(st, "mark");
            if chr_code > 0 {
                print_char(st, b's');
            }
        }
        MATH_ACCENT => print_esc(
            st,
            MathAccentCode::from_raw(chr_code).map_or("mathaccent", MathAccentCode::name),
        ),
        MATH_CHAR_NUM => print_esc(
            st,
            MathCharNumCode::from_raw(chr_code).map_or("mathchar", MathCharNumCode::name),
        ),
        MATH_CHOICE => print_esc(st, "mathchoice"),
        MULTIPLY => print_esc(st, "multiply"),
        NO_ALIGN => print_esc(st, "noalign"),
        NO_BOUNDARY => print_esc(st, "noboundary"),
        NO_EXPAND => print_esc(
            st,
            if chr_code == 0 {
                "noexpand"
            } else {
                "primitive"
            },
        ),
        NON_SCRIPT => print_esc(st, "nonscript"),
        OMIT => print_esc(st, "omit"),
        RADICAL => print_esc(
            st,
            RadicalCode::from_raw(chr_code).map_or("radical", RadicalCode::name),
        ),
        READ_TO_CS => print_esc(st, if chr_code == 0 { "read" } else { "readline" }),
        RELAX => print_esc(st, "relax"),
        SET_BOX => print_esc(st, "setbox"),
        SET_PREV_GRAF => print_esc(st, "prevgraf"),
        SET_SHAPE => {
            let name = if chr_code == PAR_SHAPE_LOC {
                "parshape"
            } else {
                match chr_code - ETEX_PEN_BASE {
                    0 => "interlinepenalties",
                    1 => "clubpenalties",
                    2 => "widowpenalties",
                    3 => "displaywidowpenalties",
                    _ => return,
                }
            };
            print_esc(st, name);
        }
        THE => print_esc(
            st,
            TheCode::from_raw(chr_code).map_or("detokenize", TheCode::name),
        ),
        TOKS_REGISTER => {
            print_esc(st, "toks");
            if chr_code != 0 {
                if let Some(q) = NodePtr::<SaNode>::from_ptr(MemPtr(chr_code)) {
                    print_sa_num(st, q);
                }
            }
        }
        VADJUST => print_esc(st, "vadjust"),
        VALIGN => print_esc(
            st,
            match chr_code {
                0 => "valign",
                x if x == i32::from(BEGIN_L_CODE) => "beginL",
                x if x == i32::from(END_L_CODE) => "endL",
                x if x == i32::from(BEGIN_R_CODE) => "beginR",
                _ => "endR",
            },
        ),
        VCENTER => print_esc(st, "vcenter"),
        VRULE => print_esc(st, "vrule"),
        PAR_END => print_esc(st, "par"),
        INPUT => print_esc(
            st,
            InputCode::from_raw(chr_code).map_or("endinput", InputCode::name),
        ),
        TOP_BOT_MARK => {
            let mark = MarkCode::from_raw(chr_code % MarkCode::MARKS).unwrap_or(MarkCode::Top);
            print_esc(st, mark.name());
            if chr_code >= MarkCode::MARKS {
                print_char(st, b's');
            }
        }
        REGISTER => {
            let sa = if (0..=19).contains(&chr_code) {
                None
            } else {
                NodePtr::<SaNode>::from_ptr(MemPtr(chr_code))
            };
            let level = sa.map_or(ValueLevel::from_raw(chr_code), |q| Some(q.sa_type(&st.mem)));
            let name = match level {
                Some(ValueLevel::Int) => "count",
                Some(ValueLevel::Dimen) => "dimen",
                Some(ValueLevel::Glue) => "skip",
                _ => "muskip",
            };
            print_esc(st, name);
            if let Some(q) = sa {
                print_sa_num(st, q);
            }
        }
        SET_AUX => print_esc(
            st,
            if chr_code == i32::from(VMODE) {
                "prevdepth"
            } else {
                "spacefactor"
            },
        ),
        SET_PAGE_INT => print_esc(
            st,
            PageIntCode::from_raw(chr_code).map_or("insertpenalties", PageIntCode::name),
        ),
        SET_BOX_DIMEN => print_esc(
            st,
            match BoxDimenCode::from_raw(chr_code) {
                Some(BoxDimenCode::Width) => "wd",
                Some(BoxDimenCode::Height) => "ht",
                _ => "dp",
            },
        ),
        LAST_ITEM => print_esc(
            st,
            LastItemCode::from_raw(chr_code).map_or("badness", LastItemCode::name),
        ),
        CONVERT => print_esc(
            st,
            ConvertCode::from_raw(chr_code).map_or("jobname", ConvertCode::name),
        ),
        IF_TEST => {
            if chr_code >= IfCode::UNLESS {
                print_esc(st, "unless");
            }
            let name = IfCode::from_raw(chr_code % IfCode::UNLESS).map_or("if", IfCode::name);
            print_esc(st, name);
        }
        FI_OR_ELSE => {
            let name = if chr_code == FiOrElseCode::Fi.raw() {
                "fi"
            } else if chr_code == FiOrElseCode::Or.raw() {
                "or"
            } else {
                "else"
            };
            print_esc(st, name);
        }
        TAB_MARK => {
            if chr_code == SPAN_CODE {
                print_esc(st, "span");
            } else {
                char_cmd(st, "alignment tab character ");
            }
        }
        CAR_RET => print_esc(st, if chr_code == CR_CODE { "cr" } else { "crcr" }),
        SET_PAGE_DIMEN => {
            let name = match chr_code {
                0 => "pagegoal",
                1 => "pagetotal",
                2 => "pagestretch",
                3 => "pagefilstretch",
                4 => "pagefillstretch",
                5 => "pagefilllstretch",
                6 => "pageshrink",
                _ => "pagedepth",
            };
            print_esc(st, name);
        }
        STOP => print_esc(st, if chr_code == 1 { "dump" } else { "end" }),
        HSKIP => print_esc(
            st,
            match SkipCode::from_raw(chr_code) {
                Some(SkipCode::Skip) => "hskip",
                Some(SkipCode::Fil) => "hfil",
                Some(SkipCode::Fill) => "hfill",
                Some(SkipCode::Ss) => "hss",
                _ => "hfilneg",
            },
        ),
        VSKIP => print_esc(
            st,
            match SkipCode::from_raw(chr_code) {
                Some(SkipCode::Skip) => "vskip",
                Some(SkipCode::Fil) => "vfil",
                Some(SkipCode::Fill) => "vfill",
                Some(SkipCode::Ss) => "vss",
                _ => "vfilneg",
            },
        ),
        MSKIP => print_esc(st, "mskip"),
        KERN => print_esc(st, "kern"),
        MKERN => print_esc(st, "mkern"),
        HMOVE => print_esc(
            st,
            if chr_code == 1 {
                "moveleft"
            } else {
                "moveright"
            },
        ),
        VMOVE => print_esc(st, if chr_code == 1 { "raise" } else { "lower" }),
        MAKE_BOX => print_esc(
            st,
            BoxCode::from_raw(chr_code).map_or("hbox", BoxCode::name),
        ),
        LEADER_SHIP => {
            let name = match chr_code - i32::from(A_LEADERS) {
                0 => "leaders",
                1 => "cleaders",
                2 => "xleaders",
                _ => "shipout",
            };
            print_esc(st, name);
        }
        START_PAR => print_esc(st, if chr_code == 0 { "noindent" } else { "indent" }),
        REMOVE_ITEM => {
            let name = if chr_code == NodeType::Glue as i32 {
                "unskip"
            } else if chr_code == NodeType::Kern as i32 {
                "unkern"
            } else {
                "unpenalty"
            };
            print_esc(st, name);
        }
        UN_HBOX => print_esc(
            st,
            if chr_code == BoxCode::Copy.raw() {
                "unhcopy"
            } else {
                "unhbox"
            },
        ),
        UN_VBOX => print_esc(
            st,
            match BoxCode::from_raw(chr_code) {
                Some(BoxCode::Copy) => "unvcopy",
                Some(BoxCode::LastBox) => "pagediscards",
                Some(BoxCode::Vsplit) => "splitdiscards",
                _ => "unvbox",
            },
        ),
        DISCRETIONARY => {
            if chr_code == 1 {
                print_esc(st, b'-');
            } else {
                print_esc(st, "discretionary");
            }
        }
        EQ_NO => print_esc(st, if chr_code == 1 { "leqno" } else { "eqno" }),
        MATH_COMP => {
            const ORD_NOAD: i32 = NodeType::OrdNoad as i32;
            const OP_NOAD: i32 = NodeType::OpNoad as i32;
            const BIN_NOAD: i32 = NodeType::BinNoad as i32;
            const REL_NOAD: i32 = NodeType::RelNoad as i32;
            const OPEN_NOAD: i32 = NodeType::OpenNoad as i32;
            const CLOSE_NOAD: i32 = NodeType::CloseNoad as i32;
            const PUNCT_NOAD: i32 = NodeType::PunctNoad as i32;
            const INNER_NOAD: i32 = NodeType::InnerNoad as i32;
            const UNDER_NOAD: i32 = NodeType::UnderNoad as i32;
            let name = match chr_code {
                ORD_NOAD => "mathord",
                OP_NOAD => "mathop",
                BIN_NOAD => "mathbin",
                REL_NOAD => "mathrel",
                OPEN_NOAD => "mathopen",
                CLOSE_NOAD => "mathclose",
                PUNCT_NOAD => "mathpunct",
                INNER_NOAD => "mathinner",
                UNDER_NOAD => "underline",
                _ => "overline",
            };
            print_esc(st, name);
        }
        LIMIT_SWITCH => print_esc(
            st,
            match chr_code as u16 {
                LIMITS => "limits",
                NO_LIMITS => "nolimits",
                _ => "displaylimits",
            },
        ),
        MATH_STYLE => print_style(st, chr_code),
        ABOVE => {
            let name = match chr_code {
                OVER_CODE => "over",
                ATOP_CODE => "atop",
                x if x == DELIMITED_CODE + ABOVE_CODE => "abovewithdelims",
                x if x == DELIMITED_CODE + OVER_CODE => "overwithdelims",
                x if x == DELIMITED_CODE + ATOP_CODE => "atopwithdelims",
                _ => "above",
            };
            print_esc(st, name);
        }
        LEFT_RIGHT => {
            let name = if chr_code == NodeType::LeftNoad as i32 {
                "left"
            } else if chr_code == i32::from(MIDDLE_NOAD) {
                "middle"
            } else {
                "right"
            };
            print_esc(st, name);
        }
        PREFIX => print_esc(
            st,
            match chr_code {
                1 => "long",
                2 => "outer",
                8 => "protected",
                _ => "global",
            },
        ),
        DEF => print_esc(
            st,
            match chr_code {
                0 => "def",
                1 => "gdef",
                2 => "edef",
                _ => "xdef",
            },
        ),
        LET => print_esc(st, if chr_code != 0 { "futurelet" } else { "let" }),
        SHORTHAND_DEF => print_esc(
            st,
            ShorthandDefCode::from_raw(chr_code).map_or("toksdef", ShorthandDefCode::name),
        ),
        CHAR_GIVEN => {
            print_esc(st, "char");
            print_hex(st, chr_code);
        }
        MATH_GIVEN => {
            print_esc(st, "mathchar");
            print_hex(st, chr_code);
        }
        XETEX_MATH_GIVEN => {
            print_esc(st, "Umathchar");
            print_hex(st, crate::scan::math_class(chr_code));
            print_hex(st, crate::scan::math_fam(chr_code));
            print_hex(st, crate::scan::math_char(chr_code));
        }
        DEF_CODE => {
            let name = match chr_code {
                LC_CODE_BASE => "lccode",
                UC_CODE_BASE => "uccode",
                SF_CODE_BASE => "sfcode",
                MATH_CODE_BASE => "mathcode",
                DEL_CODE_BASE => "delcode",
                _ => "catcode",
            };
            print_esc(st, name);
        }
        XETEX_DEF_CODE => {
            let name = match chr_code {
                SF_CODE_BASE => "XeTeXcharclass",
                MATH_CODE_BASE => "Umathcodenum",
                x if x == MATH_CODE_BASE + 1 => "Umathcode",
                DEL_CODE_BASE => "Udelcodenum",
                _ => "Udelcode",
            };
            print_esc(st, name);
        }
        DEF_FAMILY => print_size(st, chr_code - MATH_FONT_BASE),
        HYPH_DATA => print_esc(
            st,
            if chr_code == 1 {
                "patterns"
            } else {
                "hyphenation"
            },
        ),
        ASSIGN_FONT_INT => {
            let name = match chr_code {
                0 => "hyphenchar",
                1 => "skewchar",
                LP_CODE_BASE => "lpcode",
                RP_CODE_BASE => "rpcode",
                _ => return,
            };
            print_esc(st, name);
        }
        SET_FONT => print_font_select(st, FontId(chr_code)),
        SET_INTERACTION => {
            const BATCH: i32 = InteractionMode::Batch as i32;
            const NONSTOP: i32 = InteractionMode::Nonstop as i32;
            const SCROLL: i32 = InteractionMode::Scroll as i32;
            let name = match chr_code {
                BATCH => "batchmode",
                NONSTOP => "nonstopmode",
                SCROLL => "scrollmode",
                _ => "errorstopmode",
            };
            print_esc(st, name);
        }
        IN_STREAM => print_esc(st, if chr_code == 0 { "closein" } else { "openin" }),
        MESSAGE => print_esc(
            st,
            if chr_code == 0 {
                "message"
            } else {
                "errmessage"
            },
        ),
        CASE_SHIFT => print_esc(
            st,
            if chr_code == LC_CODE_BASE {
                "lowercase"
            } else {
                "uppercase"
            },
        ),
        XRAY => print_esc(
            st,
            ShowCode::from_raw(chr_code).map_or("show", ShowCode::name),
        ),
        UNDEFINED_CS => print(st, "undefined"),
        CALL | LONG_CALL | OUTER_CALL | LONG_OUTER_CALL => {
            // Bit 0 is long, bit 1 outer, bit 2 protected.
            let mut n = i32::from(cmd - CALL);
            if st.mem.is_protected_macro(MemPtr(chr_code)) {
                n += 4;
            }
            if n / 4 % 2 != 0 {
                print_esc(st, "protected");
            }
            if n % 2 != 0 {
                print_esc(st, "long");
            }
            if n / 2 % 2 != 0 {
                print_esc(st, "outer");
            }
            if n > 0 {
                print_char(st, b' ');
            }
            print(st, "macro");
        }
        END_TEMPLATE => print_esc(st, "outer endtemplate"),
        EXTENSION => match ExtensionCode::from_raw(chr_code) {
            Some(code) => print_esc(st, code.name()),
            None => print(st, "[unknown extension!]"),
        },
        _ => print(st, "[unknown command code!]"),
    }
}

pub(crate) fn print_meaning(st: &mut EngineState) {
    let cmd = st.scanner.cur_cmd;
    let chr = st.scanner.cur_chr;
    print_cmd_chr(st, cmd, chr);
    if cmd >= CALL {
        print_char(st, b':');
        print_ln(st);
        token_show(st, chr);
    } else if cmd == TOP_BOT_MARK {
        if let Some(mark) = MarkCode::from_raw(chr) {
            print_char(st, b':');
            print_ln(st);
            token_show(st, st.scanner.cur_mark[mark]);
        }
    }
}
