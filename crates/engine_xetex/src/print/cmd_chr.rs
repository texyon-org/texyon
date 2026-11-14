//! Symbolic printing of commands: `print_command` and `print_meaning`.

use super::{
    print, print_char, print_code, print_esc, print_font_select, print_hex, print_int,
    print_length_param, print_ln, print_param, print_size, print_skip_param, print_style,
    token_show,
};
use crate::base::codes::{BoxCode, FiOrElseCode, LimitCode, SkipCode, ValueLevel};
use crate::base::command::Command;
use crate::base::params::TokPar;
use crate::base::token::TokenList;
use crate::base::units::NodeType;
use crate::print::errors::InteractionMode;
use crate::state::EngineState;
use crate::tables::eqtb::{DimenSlot, GlueSlot, IntSlot, ToksSlot};

impl crate::print::TexPrint for Command {
    fn print_to(&self, st: &mut EngineState) {
        print_command(st, *self);
    }
}

/// Prints `c` as its primitive name, or describes a character command.
pub(crate) fn print_command(st: &mut EngineState, c: Command) {
    let char_cmd = |st: &mut EngineState, s: &str, c: crate::base::command::Usv| {
        print(st, s);
        print_code(st, c.raw());
    };
    let esc_if = |st: &mut EngineState, on: bool, yes: &str, no: &str| {
        print_esc(st, if on { yes } else { no });
    };

    match c {
        Command::LeftBrace(c) => char_cmd(st, "begin-group character ", c),
        Command::RightBrace(c) => char_cmd(st, "end-group character ", c),
        Command::MathShift(c) => char_cmd(st, "math shift character ", c),
        Command::MacParam(c) => char_cmd(st, "macro parameter character ", c),
        Command::SupMark(c) => char_cmd(st, "superscript character ", c),
        Command::SubMark(c) => char_cmd(st, "subscript character ", c),
        Command::Endv => print(st, "end of alignment template"),
        Command::Spacer(c) => char_cmd(st, "blank space ", c),
        Command::Letter(c) => char_cmd(st, "the letter ", c),
        Command::OtherChar(c) => char_cmd(st, "the character ", c),
        Command::TabMark(t) => match t.get() {
            Some(c) => char_cmd(st, "alignment tab character ", c),
            None => print_esc(st, "span"),
        },
        Command::AssignGlue(loc) | Command::AssignMuGlue(loc) => match loc.get() {
            GlueSlot::Skip(n) => {
                print_esc(st, "skip");
                print_int(st, i32::from(n));
            }
            GlueSlot::MuSkip(n) => {
                print_esc(st, "muskip");
                print_int(st, i32::from(n));
            }
            GlueSlot::Par(n) => print_skip_param(st, i32::from(n)),
        },
        Command::AssignToks(loc) => match loc.get() {
            ToksSlot::Register(n) => {
                print_esc(st, "toks");
                print_int(st, i32::from(n));
            }
            ToksSlot::Par(n) => {
                let par = TokPar::from_raw(i32::from(n)).unwrap_or(TokPar::ErrHelp);
                print_esc(st, par.name());
            }
        },
        Command::AssignInt(loc) => match loc.get() {
            IntSlot::Count(n) => {
                print_esc(st, "count");
                print_int(st, i32::from(n));
            }
            IntSlot::Par(n) => print_param(st, i32::from(n)),
        },
        Command::AssignDimen(loc) => match loc.get() {
            DimenSlot::Dimen(n) => {
                print_esc(st, "dimen");
                print_int(st, i32::from(n));
            }
            DimenSlot::Par(n) => print_length_param(st, i32::from(n)),
        },
        Command::Accent => print_esc(st, "accent"),
        Command::Advance => print_esc(st, "advance"),
        Command::AfterAssignment => print_esc(st, "afterassignment"),
        Command::AfterGroup => print_esc(st, "aftergroup"),
        Command::AssignFontDimen => print_esc(st, "fontdimen"),
        Command::BeginGroup => print_esc(st, "begingroup"),
        Command::BreakPenalty => print_esc(st, "penalty"),
        Command::CharNum => print_esc(st, "char"),
        Command::CsName => print_esc(st, "csname"),
        Command::DefFont => print_esc(st, "font"),
        Command::DelimNum(code) => print_esc(st, code.name()),
        Command::Divide => print_esc(st, "divide"),
        Command::EndCsName => print_esc(st, "endcsname"),
        Command::EndGroup => print_esc(st, "endgroup"),
        Command::ExSpace => print_esc(st, b' '),
        Command::ExpandAfter { unless } => esc_if(st, unless, "unless", "expandafter"),
        Command::Halign => print_esc(st, "halign"),
        Command::Hrule => print_esc(st, "hrule"),
        Command::IgnoreSpaces { primitive } => {
            esc_if(st, primitive, "primitive", "ignorespaces");
        }
        Command::Insert => print_esc(st, "insert"),
        Command::ItalCorr => print_esc(st, b'/'),
        Command::Mark { marks } => {
            print_esc(st, "mark");
            if marks {
                print_char(st, b's');
            }
        }
        Command::MathAccent(code) => print_esc(st, code.name()),
        Command::MathCharNum(code) => print_esc(st, code.name()),
        Command::MathChoice => print_esc(st, "mathchoice"),
        Command::Multiply => print_esc(st, "multiply"),
        Command::NoAlign => print_esc(st, "noalign"),
        Command::NoBoundary => print_esc(st, "noboundary"),
        Command::NoExpand { primitive } => esc_if(st, primitive, "primitive", "noexpand"),
        Command::NonScript => print_esc(st, "nonscript"),
        Command::Omit => print_esc(st, "omit"),
        Command::Radical(code) => print_esc(st, code.name()),
        Command::ReadToCs { line } => esc_if(st, line, "readline", "read"),
        Command::Relax(_) => print_esc(st, "relax"),
        Command::SetBox => print_esc(st, "setbox"),
        Command::SetPrevGraf => print_esc(st, "prevgraf"),
        Command::SetShape(loc) => print_esc(st, loc.name()),
        Command::The(code) => print_esc(st, code.name()),
        Command::ToksRegister => print_esc(st, "toks"),
        Command::Vadjust => print_esc(st, "vadjust"),
        Command::Valign(code) => print_esc(st, code.name()),
        Command::Vcenter => print_esc(st, "vcenter"),
        Command::Vrule => print_esc(st, "vrule"),
        Command::PartokenName => print_esc(st, "partokenname"),
        Command::ParEnd => print_esc(st, "par"),
        Command::Input(code) => print_esc(st, code.name()),
        Command::TopBotMark { code, marks } => {
            print_esc(st, code.name());
            if marks {
                print_char(st, b's');
            }
        }
        Command::Register(level) => print_esc(
            st,
            match level {
                ValueLevel::Int => "count",
                ValueLevel::Dimen => "dimen",
                ValueLevel::Glue => "skip",
                _ => "muskip",
            },
        ),
        Command::SetAux(code) => print_esc(st, code.name()),
        Command::SetPageInt(code) => print_esc(st, code.name()),
        Command::SetBoxDimen(code) => print_esc(st, code.name()),
        Command::LastItem(code) => print_esc(st, code.name()),
        Command::Convert(code) => print_esc(st, code.name()),
        Command::IfTest(test) => {
            if test.unless {
                print_esc(st, "unless");
            }
            print_esc(st, test.code.name());
        }
        Command::FiOrElse(code) => print_esc(
            st,
            match code {
                FiOrElseCode::Fi => "fi",
                FiOrElseCode::Or => "or",
                _ => "else",
            },
        ),
        Command::CarRet { crcr } => esc_if(st, crcr, "crcr", "cr"),
        Command::SetPageDimen(code) => print_esc(st, code.name()),
        Command::Stop { dump } => esc_if(st, dump, "dump", "end"),
        Command::Hskip(code) => print_esc(
            st,
            match code {
                SkipCode::Skip => "hskip",
                SkipCode::Fil => "hfil",
                SkipCode::Fill => "hfill",
                SkipCode::Ss => "hss",
                SkipCode::FilNeg | SkipCode::Mskip => "hfilneg",
            },
        ),
        Command::Vskip(code) => print_esc(
            st,
            match code {
                SkipCode::Skip => "vskip",
                SkipCode::Fil => "vfil",
                SkipCode::Fill => "vfill",
                SkipCode::Ss => "vss",
                SkipCode::FilNeg | SkipCode::Mskip => "vfilneg",
            },
        ),
        Command::Mskip => print_esc(st, "mskip"),
        Command::Kern => print_esc(st, "kern"),
        Command::Mkern => print_esc(st, "mkern"),
        Command::Hmove { left } => esc_if(st, left, "moveleft", "moveright"),
        Command::Vmove { raise } => esc_if(st, raise, "raise", "lower"),
        Command::MakeBox(code) => print_esc(st, code.name()),
        Command::LeaderShip(code) => print_esc(st, code.name()),
        Command::StartPar { indent } => esc_if(st, indent, "indent", "noindent"),
        Command::RemoveItem(t) => print_esc(
            st,
            match t {
                NodeType::Glue => "unskip",
                NodeType::Kern => "unkern",
                _ => "unpenalty",
            },
        ),
        Command::UnHbox(code) => esc_if(st, code == BoxCode::Copy, "unhcopy", "unhbox"),
        Command::UnVbox(code) => print_esc(
            st,
            match code {
                BoxCode::Copy => "unvcopy",
                BoxCode::LastBox => "pagediscards",
                BoxCode::Vsplit => "splitdiscards",
                _ => "unvbox",
            },
        ),
        Command::Discretionary { hyphen } => {
            if hyphen {
                print_esc(st, b'-');
            } else {
                print_esc(st, "discretionary");
            }
        }
        Command::EqNo { left } => esc_if(st, left, "leqno", "eqno"),
        Command::MathComp(t) => print_esc(
            st,
            match t {
                NodeType::OrdNoad => "mathord",
                NodeType::OpNoad => "mathop",
                NodeType::BinNoad => "mathbin",
                NodeType::RelNoad => "mathrel",
                NodeType::OpenNoad => "mathopen",
                NodeType::CloseNoad => "mathclose",
                NodeType::PunctNoad => "mathpunct",
                NodeType::InnerNoad => "mathinner",
                NodeType::UnderNoad => "underline",
                _ => "overline",
            },
        ),
        Command::LimitSwitch(code) => print_esc(
            st,
            match code {
                LimitCode::Limits => "limits",
                LimitCode::NoLimits => "nolimits",
                LimitCode::Normal => "displaylimits",
            },
        ),
        Command::MathStyle(style) => print_style(st, i32::from(style.raw())),
        Command::Above(code) => print_esc(st, code.name()),
        Command::LeftRight(code) => print_esc(st, code.name()),
        Command::Prefix(code) => print_esc(st, code.name()),
        Command::Def { global, expand } => print_esc(
            st,
            match (expand, global) {
                (false, false) => "def",
                (false, true) => "gdef",
                (true, false) => "edef",
                (true, true) => "xdef",
            },
        ),
        Command::Let { future } => esc_if(st, future, "futurelet", "let"),
        Command::ShorthandDef(code) => print_esc(st, code.name()),
        Command::CharGiven(c) => {
            print_esc(st, "char");
            print_hex(st, c.raw());
        }
        Command::MathGiven(v) => {
            print_esc(st, "mathchar");
            print_hex(st, v);
        }
        Command::XetexMathGiven(v) => {
            print_esc(st, "Umathchar");
            print_hex(st, crate::scanning::scan::math_class(v));
            print_hex(st, crate::scanning::scan::math_fam(v));
            print_hex(st, crate::scanning::scan::math_char(v));
        }
        Command::DefCode(table) => print_esc(st, table.name()),
        Command::XetexDefCode(code) => print_esc(st, code.name()),
        Command::DefFamily(loc) => print_size(st, loc.math_offset()),
        Command::HyphData { patterns } => esc_if(st, patterns, "patterns", "hyphenation"),
        Command::AssignFontInt(code) => print_esc(st, code.name()),
        Command::SetFont(f) => print_font_select(st, f),
        Command::SetInteraction(mode) => print_esc(
            st,
            match mode {
                InteractionMode::Batch => "batchmode",
                InteractionMode::Nonstop => "nonstopmode",
                InteractionMode::Scroll => "scrollmode",
                InteractionMode::ErrorStop => "errorstopmode",
            },
        ),
        Command::InStream { open } => esc_if(st, open, "openin", "closein"),
        Command::Message { err } => esc_if(st, err, "errmessage", "message"),
        Command::CaseShift(table) => print_esc(st, table.name()),
        Command::Xray(code) => print_esc(st, code.name()),
        Command::UndefinedCs => print(st, "undefined"),
        Command::Call(body)
        | Command::LongCall(body)
        | Command::OuterCall(body)
        | Command::LongOuterCall(body) => {
            let cmd = c.cmd();
            let mut prefixed = false;
            if st.mem.is_protected_macro(body) {
                print_esc(st, "protected");
                prefixed = true;
            }
            if cmd.is_long_call() {
                print_esc(st, "long");
                prefixed = true;
            }
            if cmd.is_outer_call() {
                print_esc(st, "outer");
                prefixed = true;
            }
            if prefixed {
                print_char(st, b' ');
            }
            print(st, "macro");
        }
        Command::EndTemplate => print_esc(st, "outer endtemplate"),
        Command::Extension(code) => print_esc(st, code.name()),
        Command::DontExpand => print(st, "[unknown command code!]"),
    }
}

/// `\meaning` of the current command: its name, and a macro's or mark's text.
pub(crate) fn print_meaning(st: &mut EngineState) {
    let cur = st.scanner.cur;
    print_command(st, cur);
    let list = match cur {
        Command::Call(body)
        | Command::LongCall(body)
        | Command::OuterCall(body)
        | Command::LongOuterCall(body) => body,
        Command::EndTemplate => TokenList::NULL,
        Command::TopBotMark { code, marks: false } => st.scanner.cur_mark[code],
        _ => return,
    };
    print_char(st, b':');
    print_ln(st);
    token_show(st, list);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::params::IntPar;
    use crate::base::units::PoolPos;
    use crate::print::Selector;
    use crate::tables::primitives::PRIMITIVES;

    fn printed(st: &mut EngineState, c: Command) -> String {
        let start = st.strings.pool_ptr;
        print_command(st, c);
        let text = String::from_utf16_lossy(
            &st.strings.str_pool[start.index()..st.strings.pool_ptr.index()],
        );
        st.strings.pool_ptr = start;
        text
    }

    #[test]
    fn every_primitive_prints_as_its_name() {
        let mut st = EngineState::default();
        st.eqtb.ints = crate::tables::eqtb::Table::new(crate::tables::eqtb::IntLoc::LEN, 0);
        st.strings.str_pool = vec![0; 256];
        st.strings.pool_size = PoolPos::from_raw(256);
        st.eqtb.set_int_par(IntPar::EscapeChar, i32::from(b'\\'));
        st.shipout.selector = Selector::NewString;
        let mut wrong = Vec::new();
        for p in PRIMITIVES {
            // A font prints through its identifier and `\/` through its pool
            // string, neither of which this bare state has.
            let Some(c) = Command::decode(p.cmd, p.chr)
                .filter(|c| !matches!(c, Command::SetFont(_)) && p.name.len() > 1)
            else {
                continue;
            };
            let got = printed(&mut st, c);
            if got != format!("\\{}", p.name) {
                wrong.push(format!("{} -> {got}", p.name));
            }
        }
        let aliases = [
            "XeTeXdelimiter -> \\Udelimiter",
            "XeTeXmathcharnum -> \\Umathcharnum",
            "XeTeXmathchar -> \\Umathchar",
            "XeTeXmathaccent -> \\Umathaccent",
            "XeTeXradical -> \\Uradical",
            "XeTeXmathcodenum -> \\Umathcodenum",
            "XeTeXmathcode -> \\Umathcode",
            "XeTeXdelcodenum -> \\Udelcodenum",
            "XeTeXdelcode -> \\Udelcode",
            "XeTeXmathcharnumdef -> \\Umathcharnumdef",
            "XeTeXmathchardef -> \\Umathchardef",
        ];
        assert_eq!(wrong, aliases);
    }
}
