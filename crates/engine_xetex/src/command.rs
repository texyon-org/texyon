//! `Command`: a `(cur_cmd, cur_chr)` pair decoded into one typed value.

use crate::cmd::Cmd;
use crate::codes::{
    AboveCode, AuxCode, BoxCode, BoxDimenCode, ConvertCode, DelimNumCode, ExtensionCode,
    FiOrElseCode, FontIntCode, IfCode, InputCode, LastItemCode, LeaderCode, LeftRightCode,
    LimitCode, MarkCode, MathAccentCode, MathCharNumCode, PageDimenCode, PageIntCode, PrefixCode,
    RadicalCode, ShorthandDefCode, ShowCode, SkipCode, TheCode, ValignCode, ValueLevel,
};
use crate::consts::BIGGEST_USV;
use crate::main_control::{CR_CR_CODE, SPAN_CODE};
use crate::math::MathStyle;
use crate::node::{NodePtr, SaNode};
use crate::scan::NO_EXPAND_FLAG;
use crate::state::InteractionMode;
use crate::token::TokenList;
use crate::units::{EqtbPtr, FontId, MemPtr, NodeType};

/// A Unicode scalar value carried by a character command.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) struct Usv(i32);

impl Usv {
    /// `c` as a scalar value, `None` outside `0..=BIGGEST_USV`.
    #[inline]
    pub(crate) const fn new(c: i32) -> Option<Usv> {
        if c >= 0 && c <= BIGGEST_USV {
            Some(Usv(c))
        } else {
            None
        }
    }

    #[inline]
    pub(crate) const fn raw(self) -> i32 {
        self.0
    }
}

/// What a `\count`-style register command names.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum RegisterRef {
    /// `\count`, `\dimen`, `\skip` or `\muskip`, whose number follows.
    Scanned(ValueLevel),
    /// A register above 255 that `\countdef` and friends bound to a name.
    Sparse(NodePtr<SaNode>),
}

/// A `&` or `\span`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum TabMark {
    Char(Usv),
    Span,
}

/// A command with its `chr_code` decoded, one variant per `Cmd`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum Command {
    /// `\relax`, or a token that `\noexpand` marked as not to expand.
    Relax {
        no_expand: bool,
    },
    LeftBrace(Usv),
    RightBrace(Usv),
    MathShift(Usv),
    TabMark(TabMark),
    /// `\cr` or `\crcr`.
    CarRet {
        crcr: bool,
    },
    MacParam(Usv),
    SupMark(Usv),
    SubMark(Usv),
    Endv,
    Spacer(Usv),
    Letter(Usv),
    OtherChar(Usv),
    ParEnd,
    /// `\end` or `\dump`.
    Stop {
        dump: bool,
    },
    DelimNum(DelimNumCode),
    CharNum,
    MathCharNum(MathCharNumCode),
    /// `\mark` or `\marks`.
    Mark {
        marks: bool,
    },
    Xray(ShowCode),
    MakeBox(BoxCode),
    /// `\moveright` or `\moveleft`.
    Hmove {
        left: bool,
    },
    /// `\lower` or `\raise`.
    Vmove {
        raise: bool,
    },
    UnHbox(BoxCode),
    UnVbox(BoxCode),
    /// `\unskip`, `\unkern` or `\unpenalty`, by the node type removed.
    RemoveItem(NodeType),
    Hskip(SkipCode),
    Vskip(SkipCode),
    Mskip,
    Kern,
    Mkern,
    LeaderShip(LeaderCode),
    Halign,
    Valign(ValignCode),
    NoAlign,
    Vrule,
    Hrule,
    Insert,
    Vadjust,
    /// `\ignorespaces` or `\primitive`.
    IgnoreSpaces {
        primitive: bool,
    },
    AfterAssignment,
    AfterGroup,
    BreakPenalty,
    /// `\noindent` or `\indent`.
    StartPar {
        indent: bool,
    },
    ItalCorr,
    Accent,
    MathAccent(MathAccentCode),
    /// `\discretionary` or `\-`.
    Discretionary {
        hyphen: bool,
    },
    /// `\eqno` or `\leqno`.
    EqNo {
        left: bool,
    },
    LeftRight(LeftRightCode),
    /// `\mathord` and friends, by the noad type they make.
    MathComp(NodeType),
    LimitSwitch(LimitCode),
    Above(AboveCode),
    MathStyle(MathStyle),
    MathChoice,
    NonScript,
    Vcenter,
    /// `\lowercase` or `\uppercase`, by the code table they use.
    CaseShift(EqtbPtr),
    /// `\message` or `\errmessage`.
    Message {
        err: bool,
    },
    Extension(ExtensionCode),
    /// `\closein` or `\openin`.
    InStream {
        open: bool,
    },
    BeginGroup,
    EndGroup,
    Omit,
    ExSpace,
    NoBoundary,
    Radical(RadicalCode),
    EndCsName,
    CharGiven(Usv),
    MathGiven(i32),
    XetexMathGiven(i32),
    LastItem(LastItemCode),
    /// `\toks`, or a token register `\toksdef` bound to a name.
    ToksRegister(Option<NodePtr<SaNode>>),
    AssignToks(EqtbPtr),
    AssignInt(EqtbPtr),
    AssignDimen(EqtbPtr),
    AssignGlue(EqtbPtr),
    AssignMuGlue(EqtbPtr),
    AssignFontDimen,
    AssignFontInt(FontIntCode),
    SetAux(AuxCode),
    SetPrevGraf,
    SetPageDimen(PageDimenCode),
    SetPageInt(PageIntCode),
    SetBoxDimen(BoxDimenCode),
    SetShape(EqtbPtr),
    /// `\catcode` and friends, by the table base.
    DefCode(EqtbPtr),
    XetexDefCode(EqtbPtr),
    /// `\textfont` and friends, by the table base.
    DefFamily(EqtbPtr),
    SetFont(FontId),
    DefFont,
    Register(RegisterRef),
    Advance,
    Multiply,
    Divide,
    Prefix(PrefixCode),
    /// `\let` or `\futurelet`.
    Let {
        future: bool,
    },
    ShorthandDef(ShorthandDefCode),
    /// `\read` or `\readline`.
    ReadToCs {
        line: bool,
    },
    /// `\def`, `\gdef`, `\edef` or `\xdef`.
    Def {
        global: bool,
        expand: bool,
    },
    SetBox,
    /// `\hyphenation` or `\patterns`.
    HyphData {
        patterns: bool,
    },
    SetInteraction(InteractionMode),
    UndefinedCs,
    /// `\expandafter` or `\unless`.
    ExpandAfter {
        unless: bool,
    },
    /// `\noexpand` or `\primitive`.
    NoExpand {
        primitive: bool,
    },
    Input(InputCode),
    IfTest {
        code: IfCode,
        unless: bool,
    },
    FiOrElse(FiOrElseCode),
    CsName,
    Convert(ConvertCode),
    The(TheCode),
    /// `\topmark` and friends, `marks` for the `\topmarks` forms.
    TopBotMark {
        code: MarkCode,
        marks: bool,
    },
    /// A macro call, by its reference-counted definition.
    Call {
        body: TokenList,
        long: bool,
        outer: bool,
    },
    EndTemplate,
    DontExpand,
}

impl crate::state::ScannerState {
    /// The code of the current `\fi`, `\else` or `\or`, `None` for any other command.
    #[inline]
    pub(crate) fn fi_or_else(&self) -> Option<FiOrElseCode> {
        match self.try_command() {
            Some(Command::FiOrElse(code)) => Some(code),
            _ => None,
        }
    }
}

impl Command {
    /// The command `cmd` with modifier `chr`, `None` when `chr` is not one of its codes.
    pub(crate) fn decode(cmd: Cmd, chr: i32) -> Option<Command> {
        let flag = |on: i32| match chr {
            0 => Some(false),
            c if c == on => Some(true),
            _ => None,
        };
        let usv = Usv::new(chr);
        let loc = EqtbPtr::from_raw(chr);
        let node_type = || {
            u16::try_from(chr)
                .ok()
                .and_then(|t| NodeType::from_raw(t).ok())
        };
        Some(match cmd {
            Cmd::Relax => Command::Relax {
                no_expand: chr == NO_EXPAND_FLAG,
            },
            Cmd::LeftBrace => Command::LeftBrace(usv?),
            Cmd::RightBrace => Command::RightBrace(usv?),
            Cmd::MathShift => Command::MathShift(usv?),
            Cmd::TabMark if chr == SPAN_CODE => Command::TabMark(TabMark::Span),
            Cmd::TabMark => Command::TabMark(TabMark::Char(usv?)),
            Cmd::CarRet => Command::CarRet {
                crcr: chr == CR_CR_CODE,
            },
            Cmd::MacParam => Command::MacParam(usv?),
            Cmd::SupMark => Command::SupMark(usv?),
            Cmd::SubMark => Command::SubMark(usv?),
            Cmd::Endv => Command::Endv,
            Cmd::Spacer => Command::Spacer(usv?),
            Cmd::Letter => Command::Letter(usv?),
            Cmd::OtherChar => Command::OtherChar(usv?),
            Cmd::ParEnd => Command::ParEnd,
            Cmd::Stop => Command::Stop { dump: flag(1)? },
            Cmd::DelimNum => Command::DelimNum(DelimNumCode::from_raw(chr)?),
            Cmd::CharNum => Command::CharNum,
            Cmd::MathCharNum => Command::MathCharNum(MathCharNumCode::from_raw(chr)?),
            Cmd::Mark => Command::Mark {
                marks: flag(MarkCode::MARKS)?,
            },
            Cmd::Xray => Command::Xray(ShowCode::from_raw(chr)?),
            Cmd::MakeBox => Command::MakeBox(BoxCode::from_raw(chr)?),
            Cmd::Hmove => Command::Hmove { left: flag(1)? },
            Cmd::Vmove => Command::Vmove { raise: flag(1)? },
            Cmd::UnHbox => Command::UnHbox(BoxCode::from_raw(chr)?),
            Cmd::UnVbox => Command::UnVbox(BoxCode::from_raw(chr)?),
            Cmd::RemoveItem => match node_type()? {
                t @ (NodeType::Glue | NodeType::Kern | NodeType::Penalty) => Command::RemoveItem(t),
                _ => return None,
            },
            Cmd::Hskip => Command::Hskip(SkipCode::from_raw(chr)?),
            Cmd::Vskip => Command::Vskip(SkipCode::from_raw(chr)?),
            Cmd::Mskip => Command::Mskip,
            Cmd::Kern => Command::Kern,
            Cmd::Mkern => Command::Mkern,
            Cmd::LeaderShip => Command::LeaderShip(LeaderCode::from_raw(chr)?),
            Cmd::Halign => Command::Halign,
            Cmd::Valign => Command::Valign(ValignCode::from_raw(chr)?),
            Cmd::NoAlign => Command::NoAlign,
            Cmd::Vrule => Command::Vrule,
            Cmd::Hrule => Command::Hrule,
            Cmd::Insert => Command::Insert,
            Cmd::Vadjust => Command::Vadjust,
            Cmd::IgnoreSpaces => Command::IgnoreSpaces {
                primitive: flag(1)?,
            },
            Cmd::AfterAssignment => Command::AfterAssignment,
            Cmd::AfterGroup => Command::AfterGroup,
            Cmd::BreakPenalty => Command::BreakPenalty,
            Cmd::StartPar => Command::StartPar { indent: flag(1)? },
            Cmd::ItalCorr => Command::ItalCorr,
            Cmd::Accent => Command::Accent,
            Cmd::MathAccent => Command::MathAccent(MathAccentCode::from_raw(chr)?),
            Cmd::Discretionary => Command::Discretionary { hyphen: flag(1)? },
            Cmd::EqNo => Command::EqNo { left: flag(1)? },
            Cmd::LeftRight => Command::LeftRight(LeftRightCode::from_raw(chr)?),
            Cmd::MathComp => match node_type()? {
                t @ (NodeType::OrdNoad
                | NodeType::OpNoad
                | NodeType::BinNoad
                | NodeType::RelNoad
                | NodeType::OpenNoad
                | NodeType::CloseNoad
                | NodeType::PunctNoad
                | NodeType::InnerNoad
                | NodeType::UnderNoad
                | NodeType::OverNoad) => Command::MathComp(t),
                _ => return None,
            },
            Cmd::LimitSwitch => Command::LimitSwitch(LimitCode::from_raw(chr)?),
            Cmd::Above => Command::Above(AboveCode::from_raw(chr)?),
            Cmd::MathStyle => match u16::try_from(chr).ok()? {
                s @ (0 | 2 | 4 | 6) => Command::MathStyle(MathStyle::from_raw(s)),
                _ => return None,
            },
            Cmd::MathChoice => Command::MathChoice,
            Cmd::NonScript => Command::NonScript,
            Cmd::Vcenter => Command::Vcenter,
            Cmd::CaseShift => Command::CaseShift(loc),
            Cmd::Message => Command::Message { err: flag(1)? },
            Cmd::Extension => Command::Extension(ExtensionCode::from_raw(chr)?),
            Cmd::InStream => Command::InStream { open: flag(1)? },
            Cmd::BeginGroup => Command::BeginGroup,
            Cmd::EndGroup => Command::EndGroup,
            Cmd::Omit => Command::Omit,
            Cmd::ExSpace => Command::ExSpace,
            Cmd::NoBoundary => Command::NoBoundary,
            Cmd::Radical => Command::Radical(RadicalCode::from_raw(chr)?),
            Cmd::EndCsName => Command::EndCsName,
            Cmd::CharGiven => Command::CharGiven(usv?),
            Cmd::MathGiven => Command::MathGiven(chr),
            Cmd::XetexMathGiven => Command::XetexMathGiven(chr),
            Cmd::LastItem => Command::LastItem(LastItemCode::from_raw(chr)?),
            Cmd::ToksRegister => {
                Command::ToksRegister((chr != 0).then(|| NodePtr::at(MemPtr::from_raw(chr))))
            }
            Cmd::AssignToks => Command::AssignToks(loc),
            Cmd::AssignInt => Command::AssignInt(loc),
            Cmd::AssignDimen => Command::AssignDimen(loc),
            Cmd::AssignGlue => Command::AssignGlue(loc),
            Cmd::AssignMuGlue => Command::AssignMuGlue(loc),
            Cmd::AssignFontDimen => Command::AssignFontDimen,
            Cmd::AssignFontInt => Command::AssignFontInt(FontIntCode::from_raw(chr)?),
            Cmd::SetAux => Command::SetAux(AuxCode::from_raw(chr)?),
            Cmd::SetPrevGraf => Command::SetPrevGraf,
            Cmd::SetPageDimen => Command::SetPageDimen(PageDimenCode::from_raw(chr)?),
            Cmd::SetPageInt => Command::SetPageInt(PageIntCode::from_raw(chr)?),
            Cmd::SetBoxDimen => Command::SetBoxDimen(BoxDimenCode::from_raw(chr)?),
            Cmd::SetShape => Command::SetShape(loc),
            Cmd::DefCode => Command::DefCode(loc),
            Cmd::XetexDefCode => Command::XetexDefCode(loc),
            Cmd::DefFamily => Command::DefFamily(loc),
            Cmd::SetFont => Command::SetFont(FontId::from_raw(chr)),
            Cmd::DefFont => Command::DefFont,
            Cmd::Register => Command::Register(match ValueLevel::from_raw(chr) {
                Some(level) if level <= ValueLevel::Mu => RegisterRef::Scanned(level),
                Some(_) => return None,
                None => RegisterRef::Sparse(NodePtr::at(MemPtr::from_raw(chr))),
            }),
            Cmd::Advance => Command::Advance,
            Cmd::Multiply => Command::Multiply,
            Cmd::Divide => Command::Divide,
            Cmd::Prefix => Command::Prefix(PrefixCode::from_raw(chr)?),
            Cmd::Let => Command::Let { future: flag(1)? },
            Cmd::ShorthandDef => Command::ShorthandDef(ShorthandDefCode::from_raw(chr)?),
            Cmd::ReadToCs => Command::ReadToCs { line: flag(1)? },
            Cmd::Def => match chr {
                0..=3 => Command::Def {
                    global: chr & 1 != 0,
                    expand: chr >= 2,
                },
                _ => return None,
            },
            Cmd::SetBox => Command::SetBox,
            Cmd::HyphData => Command::HyphData { patterns: flag(1)? },
            Cmd::SetInteraction => Command::SetInteraction(InteractionMode::from_raw(chr)?),
            Cmd::UndefinedCs => Command::UndefinedCs,
            Cmd::ExpandAfter => Command::ExpandAfter { unless: flag(1)? },
            Cmd::NoExpand => Command::NoExpand {
                primitive: flag(1)?,
            },
            Cmd::Input => Command::Input(InputCode::from_raw(chr)?),
            Cmd::IfTest => {
                let (code, unless) = IfCode::decode(chr)?;
                Command::IfTest { code, unless }
            }
            Cmd::FiOrElse => Command::FiOrElse(FiOrElseCode::from_raw(chr)?),
            Cmd::CsName => Command::CsName,
            Cmd::Convert => Command::Convert(ConvertCode::from_raw(chr)?),
            Cmd::The => Command::The(TheCode::from_raw(chr)?),
            Cmd::TopBotMark => Command::TopBotMark {
                code: MarkCode::from_raw(chr % MarkCode::MARKS)?,
                marks: chr >= MarkCode::MARKS,
            },
            Cmd::Call | Cmd::LongCall | Cmd::OuterCall | Cmd::LongOuterCall => Command::Call {
                body: TokenList::from_ptr(MemPtr::from_raw(chr)),
                long: cmd.is_long_call(),
                outer: cmd.is_outer_call(),
            },
            Cmd::EndTemplate => Command::EndTemplate,
            Cmd::DontExpand => Command::DontExpand,
            Cmd::GlueRef | Cmd::ShapeRef | Cmd::BoxRef | Cmd::Data => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::PRIMITIVES;

    #[test]
    fn every_primitive_decodes() {
        let failed: Vec<&str> = PRIMITIVES
            .iter()
            .filter(|p| Command::decode(p.cmd, p.chr).is_none())
            .map(|p| p.name)
            .collect();
        assert_eq!(failed, Vec::<&str>::new());
    }

    #[test]
    fn flags_and_payloads_decode_from_their_codes() {
        let cases = [
            (
                Cmd::Def,
                3,
                Command::Def {
                    global: true,
                    expand: true,
                },
            ),
            (
                Cmd::Def,
                2,
                Command::Def {
                    global: false,
                    expand: true,
                },
            ),
            (Cmd::TabMark, SPAN_CODE, Command::TabMark(TabMark::Span)),
            (Cmd::CarRet, CR_CR_CODE, Command::CarRet { crcr: true }),
            (Cmd::Mark, MarkCode::MARKS, Command::Mark { marks: true }),
            (
                Cmd::TopBotMark,
                MarkCode::MARKS + 2,
                Command::TopBotMark {
                    code: MarkCode::Bot,
                    marks: true,
                },
            ),
            (
                Cmd::IfTest,
                IfCode::UNLESS + 2,
                Command::IfTest {
                    code: IfCode::Num,
                    unless: true,
                },
            ),
            (
                Cmd::Relax,
                NO_EXPAND_FLAG,
                Command::Relax { no_expand: true },
            ),
            (Cmd::RemoveItem, 11, Command::RemoveItem(NodeType::Kern)),
            (
                Cmd::Register,
                2,
                Command::Register(RegisterRef::Scanned(ValueLevel::Glue)),
            ),
        ];
        for (cmd, chr, want) in cases {
            assert_eq!(Command::decode(cmd, chr), Some(want), "{cmd:?} {chr}");
        }
    }

    #[test]
    fn codes_outside_a_command_do_not_decode() {
        let cases = [
            (Cmd::Def, 4),
            (Cmd::Stop, 2),
            (Cmd::Letter, BIGGEST_USV + 1),
            (Cmd::RemoveItem, 13),
            (Cmd::MathStyle, 1),
            (Cmd::Register, 4),
            (Cmd::Data, 0),
        ];
        for (cmd, chr) in cases {
            assert_eq!(Command::decode(cmd, chr), None, "{cmd:?} {chr}");
        }
    }
}
