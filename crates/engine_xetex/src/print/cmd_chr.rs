//! Symbolic printing of commands: `print_command` and `print_meaning`.

use super::errors::InteractionMode;
use super::{
    TexPrint, print, print_char, print_code, print_esc, print_font_select, print_hex, print_int,
    print_length_param, print_ln, print_param, print_size, print_skip_param, print_style,
    token_show,
};
use crate::base::codes::{BoxCode, FiOrElseCode, LimitCode, SkipCode, ValueLevel};
use crate::base::command::{Command, Usv};
use crate::base::params::TokPar;
use crate::base::token::TokenList;
use crate::base::units::NodeType;
use crate::env::EngineEnv;
use crate::scanning::scan::{math_char, math_class, math_fam};
use crate::state::EngineState;
use crate::tables::eqtb::{DimenSlot, GlueSlot, IntSlot, ToksSlot};

impl TexPrint for Command {
    fn print_to(&self, st: &mut EngineState, env: &mut EngineEnv) {
        print_command(st, env, *self);
    }
}

pub(crate) fn print_command(st: &mut EngineState, env: &mut EngineEnv, c: Command) {
    let char_cmd = |st: &mut EngineState, env: &mut EngineEnv, s: &str, c: Usv| {
        print(st, env, s);
        print_code(st, env, c.raw());
    };
    let esc_if = |st: &mut EngineState, env: &mut EngineEnv, on: bool, yes: &str, no: &str| {
        print_esc(st, env, if on { yes } else { no });
    };

    match c {
        Command::LeftBrace(c) => char_cmd(st, env, "begin-group character ", c),
        Command::RightBrace(c) => char_cmd(st, env, "end-group character ", c),
        Command::MathShift(c) => char_cmd(st, env, "math shift character ", c),
        Command::MacParam(c) => char_cmd(st, env, "macro parameter character ", c),
        Command::SupMark(c) => char_cmd(st, env, "superscript character ", c),
        Command::SubMark(c) => char_cmd(st, env, "subscript character ", c),
        Command::Endv => print(st, env, "end of alignment template"),
        Command::Spacer(c) => char_cmd(st, env, "blank space ", c),
        Command::Letter(c) => char_cmd(st, env, "the letter ", c),
        Command::OtherChar(c) => char_cmd(st, env, "the character ", c),
        Command::TabMark(t) => match t.get() {
            Some(c) => char_cmd(st, env, "alignment tab character ", c),
            None => print_esc(st, env, "span"),
        },
        Command::AssignGlue(loc) | Command::AssignMuGlue(loc) => match loc.get() {
            GlueSlot::Skip(n) => {
                print_esc(st, env, "skip");
                print_int(st, env, i32::from(n));
            }
            GlueSlot::MuSkip(n) => {
                print_esc(st, env, "muskip");
                print_int(st, env, i32::from(n));
            }
            GlueSlot::Par(n) => print_skip_param(st, env, i32::from(n)),
        },
        Command::AssignToks(loc) => match loc.get() {
            ToksSlot::Register(n) => {
                print_esc(st, env, "toks");
                print_int(st, env, i32::from(n));
            }
            ToksSlot::Par(n) => {
                let par = TokPar::from_raw(i32::from(n)).unwrap_or(TokPar::ErrHelp);
                print_esc(st, env, par.name());
            }
        },
        Command::AssignInt(loc) => match loc.get() {
            IntSlot::Count(n) => {
                print_esc(st, env, "count");
                print_int(st, env, i32::from(n));
            }
            IntSlot::Par(n) => print_param(st, env, i32::from(n)),
        },
        Command::AssignDimen(loc) => match loc.get() {
            DimenSlot::Dimen(n) => {
                print_esc(st, env, "dimen");
                print_int(st, env, i32::from(n));
            }
            DimenSlot::Par(n) => print_length_param(st, env, i32::from(n)),
        },
        Command::Accent => print_esc(st, env, "accent"),
        Command::Advance => print_esc(st, env, "advance"),
        Command::AfterAssignment => print_esc(st, env, "afterassignment"),
        Command::AfterGroup => print_esc(st, env, "aftergroup"),
        Command::AssignFontDimen => print_esc(st, env, "fontdimen"),
        Command::BeginGroup => print_esc(st, env, "begingroup"),
        Command::BreakPenalty => print_esc(st, env, "penalty"),
        Command::CharNum => print_esc(st, env, "char"),
        Command::CsName => print_esc(st, env, "csname"),
        Command::DefFont => print_esc(st, env, "font"),
        Command::DelimNum(code) => print_esc(st, env, code.name()),
        Command::Divide => print_esc(st, env, "divide"),
        Command::EndCsName => print_esc(st, env, "endcsname"),
        Command::EndGroup => print_esc(st, env, "endgroup"),
        Command::ExSpace => print_esc(st, env, b' '),
        Command::ExpandAfter { unless } => esc_if(st, env, unless, "unless", "expandafter"),
        Command::Halign => print_esc(st, env, "halign"),
        Command::Hrule => print_esc(st, env, "hrule"),
        Command::IgnoreSpaces { primitive } => {
            esc_if(st, env, primitive, "primitive", "ignorespaces");
        }
        Command::Insert => print_esc(st, env, "insert"),
        Command::ItalCorr => print_esc(st, env, b'/'),
        Command::Mark { marks } => {
            print_esc(st, env, "mark");
            if marks {
                print_char(st, env, b's');
            }
        }
        Command::MathAccent(code) => print_esc(st, env, code.name()),
        Command::MathCharNum(code) => print_esc(st, env, code.name()),
        Command::MathChoice => print_esc(st, env, "mathchoice"),
        Command::Multiply => print_esc(st, env, "multiply"),
        Command::NoAlign => print_esc(st, env, "noalign"),
        Command::NoBoundary => print_esc(st, env, "noboundary"),
        Command::NoExpand { primitive } => esc_if(st, env, primitive, "primitive", "noexpand"),
        Command::NonScript => print_esc(st, env, "nonscript"),
        Command::Omit => print_esc(st, env, "omit"),
        Command::Radical(code) => print_esc(st, env, code.name()),
        Command::ReadToCs { line } => esc_if(st, env, line, "readline", "read"),
        Command::Relax(_) => print_esc(st, env, "relax"),
        Command::SetBox => print_esc(st, env, "setbox"),
        Command::SetPrevGraf => print_esc(st, env, "prevgraf"),
        Command::SetShape(loc) => print_esc(st, env, loc.name()),
        Command::The(code) => print_esc(st, env, code.name()),
        Command::ToksRegister => print_esc(st, env, "toks"),
        Command::Vadjust => print_esc(st, env, "vadjust"),
        Command::Valign(code) => print_esc(st, env, code.name()),
        Command::Vcenter => print_esc(st, env, "vcenter"),
        Command::Vrule => print_esc(st, env, "vrule"),
        Command::PartokenName => print_esc(st, env, "partokenname"),
        Command::ParEnd => print_esc(st, env, "par"),
        Command::Input(code) => print_esc(st, env, code.name()),
        Command::TopBotMark { code, marks } => {
            print_esc(st, env, code.name());
            if marks {
                print_char(st, env, b's');
            }
        }
        Command::Register(level) => print_esc(
            st,
            env,
            match level {
                ValueLevel::Int => "count",
                ValueLevel::Dimen => "dimen",
                ValueLevel::Glue => "skip",
                _ => "muskip",
            },
        ),
        Command::SetAux(code) => print_esc(st, env, code.name()),
        Command::SetPageInt(code) => print_esc(st, env, code.name()),
        Command::SetBoxDimen(code) => print_esc(st, env, code.name()),
        Command::LastItem(code) => print_esc(st, env, code.name()),
        Command::Convert(code) => print_esc(st, env, code.name()),
        Command::IfTest(test) => {
            if test.unless {
                print_esc(st, env, "unless");
            }
            print_esc(st, env, test.code.name());
        }
        Command::FiOrElse(code) => print_esc(
            st,
            env,
            match code {
                FiOrElseCode::Fi => "fi",
                FiOrElseCode::Or => "or",
                _ => "else",
            },
        ),
        Command::CarRet { crcr } => esc_if(st, env, crcr, "crcr", "cr"),
        Command::SetPageDimen(code) => print_esc(st, env, code.name()),
        Command::Stop { dump } => esc_if(st, env, dump, "dump", "end"),
        Command::Hskip(code) => print_esc(
            st,
            env,
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
            env,
            match code {
                SkipCode::Skip => "vskip",
                SkipCode::Fil => "vfil",
                SkipCode::Fill => "vfill",
                SkipCode::Ss => "vss",
                SkipCode::FilNeg | SkipCode::Mskip => "vfilneg",
            },
        ),
        Command::Mskip => print_esc(st, env, "mskip"),
        Command::Kern => print_esc(st, env, "kern"),
        Command::Mkern => print_esc(st, env, "mkern"),
        Command::Hmove { left } => esc_if(st, env, left, "moveleft", "moveright"),
        Command::Vmove { raise } => esc_if(st, env, raise, "raise", "lower"),
        Command::MakeBox(code) => print_esc(st, env, code.name()),
        Command::LeaderShip(code) => print_esc(st, env, code.name()),
        Command::StartPar { indent } => esc_if(st, env, indent, "indent", "noindent"),
        Command::RemoveItem(t) => print_esc(
            st,
            env,
            match t {
                NodeType::Glue => "unskip",
                NodeType::Kern => "unkern",
                _ => "unpenalty",
            },
        ),
        Command::UnHbox(code) => esc_if(st, env, code == BoxCode::Copy, "unhcopy", "unhbox"),
        Command::UnVbox(code) => print_esc(
            st,
            env,
            match code {
                BoxCode::Copy => "unvcopy",
                BoxCode::LastBox => "pagediscards",
                BoxCode::Vsplit => "splitdiscards",
                _ => "unvbox",
            },
        ),
        Command::Discretionary { hyphen } => {
            if hyphen {
                print_esc(st, env, b'-');
            } else {
                print_esc(st, env, "discretionary");
            }
        }
        Command::EqNo { left } => esc_if(st, env, left, "leqno", "eqno"),
        Command::MathComp(t) => print_esc(
            st,
            env,
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
            env,
            match code {
                LimitCode::Limits => "limits",
                LimitCode::NoLimits => "nolimits",
                LimitCode::Normal => "displaylimits",
            },
        ),
        Command::MathStyle(style) => print_style(st, env, i32::from(style.raw())),
        Command::Above(code) => print_esc(st, env, code.name()),
        Command::LeftRight(code) => print_esc(st, env, code.name()),
        Command::Prefix(code) => print_esc(st, env, code.name()),
        Command::Def { global, expand } => print_esc(
            st,
            env,
            match (expand, global) {
                (false, false) => "def",
                (false, true) => "gdef",
                (true, false) => "edef",
                (true, true) => "xdef",
            },
        ),
        Command::Let { future } => esc_if(st, env, future, "futurelet", "let"),
        Command::ShorthandDef(code) => print_esc(st, env, code.name()),
        Command::CharGiven(c) => {
            print_esc(st, env, "char");
            print_hex(st, env, c.raw());
        }
        Command::MathGiven(v) => {
            print_esc(st, env, "mathchar");
            print_hex(st, env, v);
        }
        Command::XetexMathGiven(v) => {
            print_esc(st, env, "Umathchar");
            print_hex(st, env, math_class(v));
            print_hex(st, env, math_fam(v));
            print_hex(st, env, math_char(v));
        }
        Command::DefCode(table) => print_esc(st, env, table.name()),
        Command::XetexDefCode(code) => print_esc(st, env, code.name()),
        Command::DefFamily(loc) => print_size(st, env, loc.math_offset()),
        Command::HyphData { patterns } => esc_if(st, env, patterns, "patterns", "hyphenation"),
        Command::AssignFontInt(code) => print_esc(st, env, code.name()),
        Command::SetFont(f) => print_font_select(st, env, f),
        Command::SetInteraction(mode) => print_esc(
            st,
            env,
            match mode {
                InteractionMode::Batch => "batchmode",
                InteractionMode::Nonstop => "nonstopmode",
                InteractionMode::Scroll => "scrollmode",
                InteractionMode::ErrorStop => "errorstopmode",
            },
        ),
        Command::InStream { open } => esc_if(st, env, open, "openin", "closein"),
        Command::Message { err } => esc_if(st, env, err, "errmessage", "message"),
        Command::CaseShift(table) => print_esc(st, env, table.name()),
        Command::Xray(code) => print_esc(st, env, code.name()),
        Command::UndefinedCs => print(st, env, "undefined"),
        Command::Call(body)
        | Command::LongCall(body)
        | Command::OuterCall(body)
        | Command::LongOuterCall(body) => {
            let cmd = c.cmd();
            let mut prefixed = false;
            if st.mem.is_protected_macro(body) {
                print_esc(st, env, "protected");
                prefixed = true;
            }
            if cmd.is_long_call() {
                print_esc(st, env, "long");
                prefixed = true;
            }
            if cmd.is_outer_call() {
                print_esc(st, env, "outer");
                prefixed = true;
            }
            if prefixed {
                print_char(st, env, b' ');
            }
            print(st, env, "macro");
        }
        Command::EndTemplate => print_esc(st, env, "outer endtemplate"),
        Command::Extension(code) => print_esc(st, env, code.name()),
        Command::DontExpand => print(st, env, "[unknown command code!]"),
    }
}

pub(crate) fn print_meaning(st: &mut EngineState, env: &mut EngineEnv) {
    let cur = st.scanner.cur;
    print_command(st, env, cur);
    let list = match cur {
        Command::Call(body)
        | Command::LongCall(body)
        | Command::OuterCall(body)
        | Command::LongOuterCall(body) => body,
        Command::EndTemplate => TokenList::NULL,
        Command::TopBotMark { code, marks: false } => st.scanner.cur_mark[code],
        _ => return,
    };
    print_char(st, env, b':');
    print_ln(st, env);
    token_show(st, env, list);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::params::IntPar;
    use crate::base::units::PoolPos;
    use crate::env::test_env;
    use crate::print::Selector;
    use crate::tables::eqtb::{IntLoc, Table};
    use crate::tables::primitives::PRIMITIVES;

    fn printed(st: &mut EngineState, env: &mut EngineEnv, c: Command) -> String {
        let start = st.strings.pool_ptr;
        print_command(st, env, c);
        let text = String::from_utf16_lossy(
            &st.strings.str_pool[start.index()..st.strings.pool_ptr.index()],
        );
        st.strings.pool_ptr = start;
        text
    }

    #[test]
    fn every_primitive_prints_as_its_name() {
        let env = &mut test_env();
        let mut st = EngineState::default();
        st.eqtb.ints = Table::new(IntLoc::LEN, 0);
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
            let got = printed(&mut st, env, c);
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
