//! Symbolic printing of commands: `print_cmd_chr` and `print_meaning`.

use crate::cmd::Cmd;
use crate::codes::{
    BoxCode, BoxDimenCode, ConvertCode, DelimNumCode, ExtensionCode, FiOrElseCode, IfCode,
    InputCode, LastItemCode, LeaderCode, LeftRightCode, LimitCode, MarkCode, MathAccentCode,
    MathCharNumCode, PageIntCode, RadicalCode, ShorthandDefCode, ShowCode, SkipCode, TheCode,
    ValignCode, ValueLevel,
};
use crate::consts::{
    COUNT_BASE, DEL_CODE_BASE, DIMEN_BASE, ETEX_PEN_BASE, GLUE_BASE, INT_BASE, LC_CODE_BASE,
    LOCAL_BASE, MATH_CODE_BASE, MATH_FONT_BASE, MU_SKIP_BASE, PAR_SHAPE_LOC, SCALED_BASE,
    SF_CODE_BASE, SKIP_BASE, TOKS_BASE, UC_CODE_BASE, VMODE,
};
use crate::main_control::{CR_CODE, LP_CODE_BASE, RP_CODE_BASE, SPAN_CODE};
use crate::math::{ABOVE_CODE, ATOP_CODE, DELIMITED_CODE, OVER_CODE};
use crate::node::{NodePtr, SaElement};
use crate::params::TokPar;
use crate::state::{EngineState, InteractionMode};
use crate::token::TokenList;
use crate::units::{EqtbPtr, FontId, MemPtr, NodeType};

use super::{
    print, print_char, print_code, print_esc, print_font_select, print_hex, print_int,
    print_length_param, print_ln, print_param, print_sa_num, print_size, print_skip_param,
    print_style, token_show,
};

/// A command with its modifier, printed as its primitive name.
#[derive(Copy, Clone)]
pub(crate) struct CmdChr(pub Cmd, pub i32);

impl CmdChr {
    /// The pair TeX prints `m` as.
    pub(crate) fn of(m: crate::command::Command) -> CmdChr {
        let (cmd, chr) = m.encode();
        CmdChr(cmd, chr)
    }
}

impl crate::output::TexPrint for CmdChr {
    fn print_to(&self, st: &mut EngineState) {
        print_cmd_chr(st, self.0, self.1);
    }
}

pub(crate) fn print_cmd_chr(st: &mut EngineState, cmd: Cmd, chr_code: i32) {
    let char_cmd = |st: &mut EngineState, s: &str| {
        print(st, s);
        print_code(st, chr_code);
    };

    match cmd {
        Cmd::LeftBrace => char_cmd(st, "begin-group character "),
        Cmd::RightBrace => char_cmd(st, "end-group character "),
        Cmd::MathShift => char_cmd(st, "math shift character "),
        Cmd::MacParam => char_cmd(st, "macro parameter character "),
        Cmd::SupMark => char_cmd(st, "superscript character "),
        Cmd::SubMark => char_cmd(st, "subscript character "),
        Cmd::Endv => print(st, "end of alignment template"),
        Cmd::Spacer => char_cmd(st, "blank space "),
        Cmd::Letter => char_cmd(st, "the letter "),
        Cmd::OtherChar => char_cmd(st, "the character "),
        Cmd::AssignGlue | Cmd::AssignMuGlue => {
            let chr_code = EqtbPtr::from_raw(chr_code);
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
        Cmd::AssignToks => {
            let chr_code = EqtbPtr::from_raw(chr_code);
            if chr_code >= TOKS_BASE {
                print_esc(st, "toks");
                print_int(st, chr_code - TOKS_BASE);
            } else {
                let par = TokPar::from_raw(chr_code - LOCAL_BASE).unwrap_or(TokPar::ErrHelp);
                print_esc(st, par.name());
            }
        }
        Cmd::AssignInt => {
            let chr_code = EqtbPtr::from_raw(chr_code);
            if chr_code < COUNT_BASE {
                print_param(st, chr_code - INT_BASE);
            } else {
                print_esc(st, "count");
                print_int(st, chr_code - COUNT_BASE);
            }
        }
        Cmd::AssignDimen => {
            let chr_code = EqtbPtr::from_raw(chr_code);
            if chr_code < SCALED_BASE {
                print_length_param(st, chr_code - DIMEN_BASE);
            } else {
                print_esc(st, "dimen");
                print_int(st, chr_code - SCALED_BASE);
            }
        }
        Cmd::Accent => print_esc(st, "accent"),
        Cmd::Advance => print_esc(st, "advance"),
        Cmd::AfterAssignment => print_esc(st, "afterassignment"),
        Cmd::AfterGroup => print_esc(st, "aftergroup"),
        Cmd::AssignFontDimen => print_esc(st, "fontdimen"),
        Cmd::BeginGroup => print_esc(st, "begingroup"),
        Cmd::BreakPenalty => print_esc(st, "penalty"),
        Cmd::CharNum => print_esc(st, "char"),
        Cmd::CsName => print_esc(st, "csname"),
        Cmd::DefFont => print_esc(st, "font"),
        Cmd::DelimNum => print_esc(
            st,
            DelimNumCode::from_raw(chr_code).map_or("delimiter", DelimNumCode::name),
        ),
        Cmd::Divide => print_esc(st, "divide"),
        Cmd::EndCsName => print_esc(st, "endcsname"),
        Cmd::EndGroup => print_esc(st, "endgroup"),
        Cmd::ExSpace => print_esc(st, b' '),
        Cmd::ExpandAfter => print_esc(
            st,
            if chr_code == 0 {
                "expandafter"
            } else {
                "unless"
            },
        ),
        Cmd::Halign => print_esc(st, "halign"),
        Cmd::Hrule => print_esc(st, "hrule"),
        Cmd::IgnoreSpaces => print_esc(
            st,
            if chr_code == 0 {
                "ignorespaces"
            } else {
                "primitive"
            },
        ),
        Cmd::Insert => print_esc(st, "insert"),
        Cmd::ItalCorr => print_esc(st, b'/'),
        Cmd::Mark => {
            print_esc(st, "mark");
            if chr_code > 0 {
                print_char(st, b's');
            }
        }
        Cmd::MathAccent => print_esc(
            st,
            MathAccentCode::from_raw(chr_code).map_or("mathaccent", MathAccentCode::name),
        ),
        Cmd::MathCharNum => print_esc(
            st,
            MathCharNumCode::from_raw(chr_code).map_or("mathchar", MathCharNumCode::name),
        ),
        Cmd::MathChoice => print_esc(st, "mathchoice"),
        Cmd::Multiply => print_esc(st, "multiply"),
        Cmd::NoAlign => print_esc(st, "noalign"),
        Cmd::NoBoundary => print_esc(st, "noboundary"),
        Cmd::NoExpand => print_esc(
            st,
            if chr_code == 0 {
                "noexpand"
            } else {
                "primitive"
            },
        ),
        Cmd::NonScript => print_esc(st, "nonscript"),
        Cmd::Omit => print_esc(st, "omit"),
        Cmd::Radical => print_esc(
            st,
            RadicalCode::from_raw(chr_code).map_or("radical", RadicalCode::name),
        ),
        Cmd::ReadToCs => print_esc(st, if chr_code == 0 { "read" } else { "readline" }),
        Cmd::Relax => print_esc(st, "relax"),
        Cmd::SetBox => print_esc(st, "setbox"),
        Cmd::SetPrevGraf => print_esc(st, "prevgraf"),
        Cmd::SetShape => {
            let chr_code = EqtbPtr::from_raw(chr_code);
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
        Cmd::The => print_esc(
            st,
            TheCode::from_raw(chr_code).map_or("detokenize", TheCode::name),
        ),
        Cmd::ToksRegister => {
            print_esc(st, "toks");
            if chr_code != 0 {
                if let Some(q) = NodePtr::<SaElement>::from_ptr(MemPtr::from_raw(chr_code)) {
                    print_sa_num(st, q);
                }
            }
        }
        Cmd::Vadjust => print_esc(st, "vadjust"),
        Cmd::Valign => print_esc(
            st,
            ValignCode::from_raw(chr_code).map_or("endR", ValignCode::name),
        ),
        Cmd::Vcenter => print_esc(st, "vcenter"),
        Cmd::Vrule => print_esc(st, "vrule"),
        Cmd::PartokenName => print_esc(st, "partokenname"),
        Cmd::ParEnd => print_esc(st, "par"),
        Cmd::Input => print_esc(
            st,
            InputCode::from_raw(chr_code).map_or("endinput", InputCode::name),
        ),
        Cmd::TopBotMark => {
            let mark = MarkCode::from_raw(chr_code % MarkCode::MARKS).unwrap_or(MarkCode::Top);
            print_esc(st, mark.name());
            if chr_code >= MarkCode::MARKS {
                print_char(st, b's');
            }
        }
        Cmd::Register => {
            let sa = if (0..=19).contains(&chr_code) {
                None
            } else {
                NodePtr::<SaElement>::from_ptr(MemPtr::from_raw(chr_code))
            };
            let level = sa.map_or(ValueLevel::from_raw(chr_code), |q| {
                st.mem[q].sa_type().get()
            });
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
        Cmd::SetAux => print_esc(
            st,
            if chr_code == i32::from(VMODE) {
                "prevdepth"
            } else {
                "spacefactor"
            },
        ),
        Cmd::SetPageInt => print_esc(
            st,
            PageIntCode::from_raw(chr_code).map_or("insertpenalties", PageIntCode::name),
        ),
        Cmd::SetBoxDimen => print_esc(
            st,
            match BoxDimenCode::from_raw(chr_code) {
                Some(BoxDimenCode::Width) => "wd",
                Some(BoxDimenCode::Height) => "ht",
                _ => "dp",
            },
        ),
        Cmd::LastItem => print_esc(
            st,
            LastItemCode::from_raw(chr_code).map_or("badness", LastItemCode::name),
        ),
        Cmd::Convert => print_esc(
            st,
            ConvertCode::from_raw(chr_code).map_or("jobname", ConvertCode::name),
        ),
        Cmd::IfTest => {
            if chr_code >= IfCode::UNLESS {
                print_esc(st, "unless");
            }
            let name = IfCode::from_raw(chr_code % IfCode::UNLESS).map_or("if", IfCode::name);
            print_esc(st, name);
        }
        Cmd::FiOrElse => {
            let name = if chr_code == FiOrElseCode::Fi.raw() {
                "fi"
            } else if chr_code == FiOrElseCode::Or.raw() {
                "or"
            } else {
                "else"
            };
            print_esc(st, name);
        }
        Cmd::TabMark => {
            if chr_code == SPAN_CODE {
                print_esc(st, "span");
            } else {
                char_cmd(st, "alignment tab character ");
            }
        }
        Cmd::CarRet => print_esc(st, if chr_code == CR_CODE { "cr" } else { "crcr" }),
        Cmd::SetPageDimen => {
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
        Cmd::Stop => print_esc(st, if chr_code == 1 { "dump" } else { "end" }),
        Cmd::Hskip => print_esc(
            st,
            match SkipCode::from_raw(chr_code) {
                Some(SkipCode::Skip) => "hskip",
                Some(SkipCode::Fil) => "hfil",
                Some(SkipCode::Fill) => "hfill",
                Some(SkipCode::Ss) => "hss",
                _ => "hfilneg",
            },
        ),
        Cmd::Vskip => print_esc(
            st,
            match SkipCode::from_raw(chr_code) {
                Some(SkipCode::Skip) => "vskip",
                Some(SkipCode::Fil) => "vfil",
                Some(SkipCode::Fill) => "vfill",
                Some(SkipCode::Ss) => "vss",
                _ => "vfilneg",
            },
        ),
        Cmd::Mskip => print_esc(st, "mskip"),
        Cmd::Kern => print_esc(st, "kern"),
        Cmd::Mkern => print_esc(st, "mkern"),
        Cmd::Hmove => print_esc(
            st,
            if chr_code == 1 {
                "moveleft"
            } else {
                "moveright"
            },
        ),
        Cmd::Vmove => print_esc(st, if chr_code == 1 { "raise" } else { "lower" }),
        Cmd::MakeBox => print_esc(
            st,
            BoxCode::from_raw(chr_code).map_or("hbox", BoxCode::name),
        ),
        Cmd::LeaderShip => {
            print_esc(
                st,
                LeaderCode::from_raw(chr_code).map_or("shipout", LeaderCode::name),
            );
        }
        Cmd::StartPar => print_esc(st, if chr_code == 0 { "noindent" } else { "indent" }),
        Cmd::RemoveItem => {
            let name = if chr_code == NodeType::Glue as i32 {
                "unskip"
            } else if chr_code == NodeType::Kern as i32 {
                "unkern"
            } else {
                "unpenalty"
            };
            print_esc(st, name);
        }
        Cmd::UnHbox => print_esc(
            st,
            if chr_code == BoxCode::Copy.raw() {
                "unhcopy"
            } else {
                "unhbox"
            },
        ),
        Cmd::UnVbox => print_esc(
            st,
            match BoxCode::from_raw(chr_code) {
                Some(BoxCode::Copy) => "unvcopy",
                Some(BoxCode::LastBox) => "pagediscards",
                Some(BoxCode::Vsplit) => "splitdiscards",
                _ => "unvbox",
            },
        ),
        Cmd::Discretionary => {
            if chr_code == 1 {
                print_esc(st, b'-');
            } else {
                print_esc(st, "discretionary");
            }
        }
        Cmd::EqNo => print_esc(st, if chr_code == 1 { "leqno" } else { "eqno" }),
        Cmd::MathComp => {
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
        Cmd::LimitSwitch => print_esc(
            st,
            match LimitCode::from_raw(chr_code) {
                Some(LimitCode::Limits) => "limits",
                Some(LimitCode::NoLimits) => "nolimits",
                _ => "displaylimits",
            },
        ),
        Cmd::MathStyle => print_style(st, chr_code),
        Cmd::Above => {
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
        Cmd::LeftRight => {
            let name = if chr_code == NodeType::LeftNoad as i32 {
                "left"
            } else if chr_code == LeftRightCode::Middle.raw() {
                "middle"
            } else {
                "right"
            };
            print_esc(st, name);
        }
        Cmd::Prefix => print_esc(
            st,
            match chr_code {
                1 => "long",
                2 => "outer",
                8 => "protected",
                _ => "global",
            },
        ),
        Cmd::Def => print_esc(
            st,
            match chr_code {
                0 => "def",
                1 => "gdef",
                2 => "edef",
                _ => "xdef",
            },
        ),
        Cmd::Let => print_esc(st, if chr_code != 0 { "futurelet" } else { "let" }),
        Cmd::ShorthandDef => print_esc(
            st,
            ShorthandDefCode::from_raw(chr_code).map_or("toksdef", ShorthandDefCode::name),
        ),
        Cmd::CharGiven => {
            print_esc(st, "char");
            print_hex(st, chr_code);
        }
        Cmd::MathGiven => {
            print_esc(st, "mathchar");
            print_hex(st, chr_code);
        }
        Cmd::XetexMathGiven => {
            print_esc(st, "Umathchar");
            print_hex(st, crate::scan::math_class(chr_code));
            print_hex(st, crate::scan::math_fam(chr_code));
            print_hex(st, crate::scan::math_char(chr_code));
        }
        Cmd::DefCode => {
            let chr_code = EqtbPtr::from_raw(chr_code);
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
        Cmd::XetexDefCode => {
            let chr_code = EqtbPtr::from_raw(chr_code);
            let name = match chr_code {
                SF_CODE_BASE => "XeTeXcharclass",
                MATH_CODE_BASE => "Umathcodenum",
                x if x == MATH_CODE_BASE + 1 => "Umathcode",
                DEL_CODE_BASE => "Udelcodenum",
                _ => "Udelcode",
            };
            print_esc(st, name);
        }
        Cmd::DefFamily => print_size(st, EqtbPtr::from_raw(chr_code) - MATH_FONT_BASE),
        Cmd::HyphData => print_esc(
            st,
            if chr_code == 1 {
                "patterns"
            } else {
                "hyphenation"
            },
        ),
        Cmd::AssignFontInt => {
            let name = match chr_code {
                0 => "hyphenchar",
                1 => "skewchar",
                LP_CODE_BASE => "lpcode",
                RP_CODE_BASE => "rpcode",
                _ => return,
            };
            print_esc(st, name);
        }
        Cmd::SetFont => {
            if let Ok(f) = FontId::try_from(chr_code) {
                print_font_select(st, f);
            }
        }
        Cmd::SetInteraction => {
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
        Cmd::InStream => print_esc(st, if chr_code == 0 { "closein" } else { "openin" }),
        Cmd::Message => print_esc(
            st,
            if chr_code == 0 {
                "message"
            } else {
                "errmessage"
            },
        ),
        Cmd::CaseShift => print_esc(
            st,
            if EqtbPtr::from_raw(chr_code) == LC_CODE_BASE {
                "lowercase"
            } else {
                "uppercase"
            },
        ),
        Cmd::Xray => print_esc(
            st,
            ShowCode::from_raw(chr_code).map_or("show", ShowCode::name),
        ),
        Cmd::UndefinedCs => print(st, "undefined"),
        Cmd::Call | Cmd::LongCall | Cmd::OuterCall | Cmd::LongOuterCall => {
            // Bit 0 is long, bit 1 outer, bit 2 protected.
            let mut n = i32::from(cmd.is_long_call()) + 2 * i32::from(cmd.is_outer_call());
            if st
                .mem
                .is_protected_macro(crate::token::TokenList::from_raw(chr_code))
            {
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
        Cmd::EndTemplate => print_esc(st, "outer endtemplate"),
        Cmd::Extension => match ExtensionCode::from_raw(chr_code) {
            Some(code) => print_esc(st, code.name()),
            None => print(st, "[unknown extension!]"),
        },
        _ => print(st, "[unknown command code!]"),
    }
}

pub(crate) fn print_meaning(st: &mut EngineState) {
    let (cmd, chr) = st.scanner.cur.encode();
    print_cmd_chr(st, cmd, chr);
    if cmd >= Cmd::Call {
        print_char(st, b':');
        print_ln(st);
        token_show(st, TokenList::from_raw(chr));
    } else if cmd == Cmd::TopBotMark {
        if let Some(mark) = MarkCode::from_raw(chr) {
            print_char(st, b':');
            print_ln(st);
            token_show(st, st.scanner.cur_mark[mark]);
        }
    }
}
