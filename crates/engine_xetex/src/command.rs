//! `Command`: a meaning, TeX's `(cmd, chr)` pair as one typed value.

use crate::cmd::Cmd;
use crate::codes::{
    AboveCode, AuxCode, BoxCode, BoxDimenCode, ConvertCode, DelimNumCode, ExtensionCode,
    FiOrElseCode, FontIntCode, IfCode, InputCode, LastItemCode, LeaderCode, LeftRightCode,
    LimitCode, MarkCode, MathAccentCode, MathCharNumCode, PageDimenCode, PageIntCode, PrefixCode,
    RadicalCode, ShorthandDefCode, ShowCode, SkipCode, TheCode, ValignCode, ValueLevel,
};
use crate::consts::{BIGGEST_USV, TEX_NULL, TOO_BIG_USV};
use crate::eqtb::{DimenLoc, GlueLoc, IntLoc, ToksLoc};
use crate::errors::{Abort, OrConfusion};
use crate::main_control::{CR_CODE, CR_CR_CODE, SPAN_CODE};
use crate::math::MathStyle;
use crate::mem::NULL_LIST;
use crate::node::{NodePtr, SaElement, SaPointerNode};
use crate::scan::NO_EXPAND_FLAG;
use crate::state::InteractionMode;
use crate::token::{Token, TokenList};
use crate::units::MemIdx;
use crate::units::RawPtr;
use crate::units::{EqtbPtr, FontId, NodeType};

/// A Unicode scalar value carried by a character command.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) struct Usv(i32);

impl Usv {
    pub(crate) const SPACE: Usv = Usv(b' ' as i32);
    pub(crate) const LEFT_BRACE: Usv = Usv(b'{' as i32);

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
    Sparse(NodePtr<SaElement>),
}

/// A [`RegisterRef`] in four bytes, as TeX's `chr` packs it: a value level below
/// `Ident`, else the sparse register's node.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) struct RegisterCode(i32);

impl RegisterCode {
    pub(crate) fn new(r: RegisterRef) -> RegisterCode {
        RegisterCode(match r {
            RegisterRef::Scanned(level) => level.raw(),
            RegisterRef::Sparse(sa) => sa.ptr().raw(),
        })
    }

    #[inline]
    pub(crate) fn get(self) -> Result<RegisterRef, Abort> {
        Ok(match ValueLevel::from_raw(self.0) {
            Some(level) => RegisterRef::Scanned(level),
            None => RegisterRef::Sparse(NodePtr::at(
                MemIdx::from_raw(self.0).or_confusion("register")?,
            )),
        })
    }
}

/// A [`TabMark`] in four bytes: the character, or `SPAN_CODE`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) struct TabMarkCode(i32);

impl TabMarkCode {
    pub(crate) const SPAN: TabMarkCode = TabMarkCode(SPAN_CODE);

    pub(crate) const fn char(c: Usv) -> TabMarkCode {
        TabMarkCode(c.raw())
    }
}

/// The token register a `\toks`-style command names, in four bytes: zero for
/// `\toks` itself, else the sparse register `\toksdef` bound.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) struct ToksRef(i32);

impl ToksRef {
    pub(crate) fn sparse(sa: NodePtr<SaElement>) -> ToksRef {
        ToksRef(sa.ptr().raw())
    }

    /// The bound register, `None` for `\toks`.
    #[inline]
    pub(crate) fn get(self) -> Option<NodePtr<SaPointerNode>> {
        MemIdx::from_raw(self.0).map(NodePtr::at)
    }
}

/// Which `\relax` a relax command is. `\ifx` tells them apart.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum RelaxCode {
    /// The `\relax` primitive, and undefined names after `\csname`.
    Primitive,
    /// A token that `\noexpand` marked as not to expand.
    NoExpand,
    /// The end of a `\read` line, or the stop `scan_int` inserts.
    Zero,
}

impl RelaxCode {
    const fn from_raw(chr: i32) -> Option<RelaxCode> {
        match chr {
            TOO_BIG_USV => Some(RelaxCode::Primitive),
            NO_EXPAND_FLAG => Some(RelaxCode::NoExpand),
            0 => Some(RelaxCode::Zero),
            _ => None,
        }
    }

    const fn raw(self) -> i32 {
        match self {
            RelaxCode::Primitive => TOO_BIG_USV,
            RelaxCode::NoExpand => NO_EXPAND_FLAG,
            RelaxCode::Zero => 0,
        }
    }
}

/// A command with its `chr_code` decoded, one variant per `Cmd`, tagged by its code.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub(crate) enum Command {
    Relax(RelaxCode) = Cmd::Relax as u8,
    LeftBrace(Usv) = Cmd::LeftBrace as u8,
    RightBrace(Usv) = Cmd::RightBrace as u8,
    MathShift(Usv) = Cmd::MathShift as u8,
    TabMark(TabMarkCode) = Cmd::TabMark as u8,
    /// `\cr` or `\crcr`.
    CarRet {
        crcr: bool,
    } = Cmd::CarRet as u8,
    MacParam(Usv) = Cmd::MacParam as u8,
    SupMark(Usv) = Cmd::SupMark as u8,
    SubMark(Usv) = Cmd::SubMark as u8,
    Endv = Cmd::Endv as u8,
    Spacer(Usv) = Cmd::Spacer as u8,
    Letter(Usv) = Cmd::Letter as u8,
    OtherChar(Usv) = Cmd::OtherChar as u8,
    ParEnd = Cmd::ParEnd as u8,
    /// `\end` or `\dump`.
    Stop {
        dump: bool,
    } = Cmd::Stop as u8,
    DelimNum(DelimNumCode) = Cmd::DelimNum as u8,
    CharNum = Cmd::CharNum as u8,
    MathCharNum(MathCharNumCode) = Cmd::MathCharNum as u8,
    /// `\mark` or `\marks`.
    Mark {
        marks: bool,
    } = Cmd::Mark as u8,
    Xray(ShowCode) = Cmd::Xray as u8,
    MakeBox(BoxCode) = Cmd::MakeBox as u8,
    /// `\moveright` or `\moveleft`.
    Hmove {
        left: bool,
    } = Cmd::Hmove as u8,
    /// `\lower` or `\raise`.
    Vmove {
        raise: bool,
    } = Cmd::Vmove as u8,
    UnHbox(BoxCode) = Cmd::UnHbox as u8,
    UnVbox(BoxCode) = Cmd::UnVbox as u8,
    /// `\unskip`, `\unkern` or `\unpenalty`, by the node type removed.
    RemoveItem(NodeType) = Cmd::RemoveItem as u8,
    Hskip(SkipCode) = Cmd::Hskip as u8,
    Vskip(SkipCode) = Cmd::Vskip as u8,
    Mskip = Cmd::Mskip as u8,
    Kern = Cmd::Kern as u8,
    Mkern = Cmd::Mkern as u8,
    LeaderShip(LeaderCode) = Cmd::LeaderShip as u8,
    Halign = Cmd::Halign as u8,
    Valign(ValignCode) = Cmd::Valign as u8,
    NoAlign = Cmd::NoAlign as u8,
    Vrule = Cmd::Vrule as u8,
    Hrule = Cmd::Hrule as u8,
    Insert = Cmd::Insert as u8,
    Vadjust = Cmd::Vadjust as u8,
    /// `\ignorespaces` or `\primitive`.
    IgnoreSpaces {
        primitive: bool,
    } = Cmd::IgnoreSpaces as u8,
    AfterAssignment = Cmd::AfterAssignment as u8,
    AfterGroup = Cmd::AfterGroup as u8,
    BreakPenalty = Cmd::BreakPenalty as u8,
    /// `\noindent` or `\indent`.
    StartPar {
        indent: bool,
    } = Cmd::StartPar as u8,
    ItalCorr = Cmd::ItalCorr as u8,
    Accent = Cmd::Accent as u8,
    MathAccent(MathAccentCode) = Cmd::MathAccent as u8,
    /// `\discretionary` or `\-`.
    Discretionary {
        hyphen: bool,
    } = Cmd::Discretionary as u8,
    /// `\eqno` or `\leqno`.
    EqNo {
        left: bool,
    } = Cmd::EqNo as u8,
    LeftRight(LeftRightCode) = Cmd::LeftRight as u8,
    /// `\mathord` and friends, by the noad type they make.
    MathComp(NodeType) = Cmd::MathComp as u8,
    LimitSwitch(LimitCode) = Cmd::LimitSwitch as u8,
    Above(AboveCode) = Cmd::Above as u8,
    MathStyle(MathStyle) = Cmd::MathStyle as u8,
    MathChoice = Cmd::MathChoice as u8,
    NonScript = Cmd::NonScript as u8,
    Vcenter = Cmd::Vcenter as u8,
    /// `\lowercase` or `\uppercase`, by the code table they use.
    CaseShift(EqtbPtr) = Cmd::CaseShift as u8,
    /// `\message` or `\errmessage`.
    Message {
        err: bool,
    } = Cmd::Message as u8,
    Extension(ExtensionCode) = Cmd::Extension as u8,
    /// `\closein` or `\openin`.
    InStream {
        open: bool,
    } = Cmd::InStream as u8,
    BeginGroup = Cmd::BeginGroup as u8,
    EndGroup = Cmd::EndGroup as u8,
    Omit = Cmd::Omit as u8,
    ExSpace = Cmd::ExSpace as u8,
    NoBoundary = Cmd::NoBoundary as u8,
    Radical(RadicalCode) = Cmd::Radical as u8,
    EndCsName = Cmd::EndCsName as u8,
    CharGiven(Usv) = Cmd::CharGiven as u8,
    MathGiven(i32) = Cmd::MathGiven as u8,
    XetexMathGiven(i32) = Cmd::XetexMathGiven as u8,
    LastItem(LastItemCode) = Cmd::LastItem as u8,
    /// `\toks`, or a token register `\toksdef` bound to a name.
    ToksRegister(ToksRef) = Cmd::ToksRegister as u8,
    AssignToks(ToksLoc) = Cmd::AssignToks as u8,
    AssignInt(IntLoc) = Cmd::AssignInt as u8,
    AssignDimen(DimenLoc) = Cmd::AssignDimen as u8,
    AssignGlue(GlueLoc) = Cmd::AssignGlue as u8,
    AssignMuGlue(GlueLoc) = Cmd::AssignMuGlue as u8,
    AssignFontDimen = Cmd::AssignFontDimen as u8,
    AssignFontInt(FontIntCode) = Cmd::AssignFontInt as u8,
    SetAux(AuxCode) = Cmd::SetAux as u8,
    SetPrevGraf = Cmd::SetPrevGraf as u8,
    SetPageDimen(PageDimenCode) = Cmd::SetPageDimen as u8,
    SetPageInt(PageIntCode) = Cmd::SetPageInt as u8,
    SetBoxDimen(BoxDimenCode) = Cmd::SetBoxDimen as u8,
    SetShape(EqtbPtr) = Cmd::SetShape as u8,
    /// `\catcode` and friends, by the table base.
    DefCode(EqtbPtr) = Cmd::DefCode as u8,
    XetexDefCode(EqtbPtr) = Cmd::XetexDefCode as u8,
    /// `\textfont` and friends, by the table base.
    DefFamily(EqtbPtr) = Cmd::DefFamily as u8,
    SetFont(FontId) = Cmd::SetFont as u8,
    DefFont = Cmd::DefFont as u8,
    Register(RegisterCode) = Cmd::Register as u8,
    Advance = Cmd::Advance as u8,
    Multiply = Cmd::Multiply as u8,
    Divide = Cmd::Divide as u8,
    Prefix(PrefixCode) = Cmd::Prefix as u8,
    /// `\let` or `\futurelet`.
    Let {
        future: bool,
    } = Cmd::Let as u8,
    ShorthandDef(ShorthandDefCode) = Cmd::ShorthandDef as u8,
    /// `\read` or `\readline`.
    ReadToCs {
        line: bool,
    } = Cmd::ReadToCs as u8,
    /// `\def`, `\gdef`, `\edef` or `\xdef`.
    Def {
        global: bool,
        expand: bool,
    } = Cmd::Def as u8,
    SetBox = Cmd::SetBox as u8,
    /// `\hyphenation` or `\patterns`.
    HyphData {
        patterns: bool,
    } = Cmd::HyphData as u8,
    SetInteraction(InteractionMode) = Cmd::SetInteraction as u8,
    /// `\partokenname`.
    PartokenName = Cmd::PartokenName as u8,
    UndefinedCs = Cmd::UndefinedCs as u8,
    /// `\expandafter` or `\unless`.
    ExpandAfter {
        unless: bool,
    } = Cmd::ExpandAfter as u8,
    /// `\noexpand` or `\primitive`.
    NoExpand {
        primitive: bool,
    } = Cmd::NoExpand as u8,
    Input(InputCode) = Cmd::Input as u8,
    IfTest {
        code: IfCode,
        unless: bool,
    } = Cmd::IfTest as u8,
    FiOrElse(FiOrElseCode) = Cmd::FiOrElse as u8,
    CsName = Cmd::CsName as u8,
    Convert(ConvertCode) = Cmd::Convert as u8,
    The(TheCode) = Cmd::The as u8,
    /// `\topmark` and friends, `marks` for the `\topmarks` forms.
    TopBotMark {
        code: MarkCode,
        marks: bool,
    } = Cmd::TopBotMark as u8,
    /// A macro call, by its reference-counted definition.
    Call(TokenList) = Cmd::Call as u8,
    LongCall(TokenList) = Cmd::LongCall as u8,
    OuterCall(TokenList) = Cmd::OuterCall as u8,
    LongOuterCall(TokenList) = Cmd::LongOuterCall as u8,
    EndTemplate = Cmd::EndTemplate as u8,
    DontExpand = Cmd::DontExpand as u8,
}

impl crate::state::ScannerState {
    /// The code of the current `\fi`, `\else` or `\or`, `None` for any other command.
    #[inline]
    pub(crate) fn fi_or_else(&self) -> Option<FiOrElseCode> {
        match self.cur {
            Command::FiOrElse(code) => Some(code),
            _ => None,
        }
    }
}

const _: () = assert!(std::mem::size_of::<Command>() == 8);

/// `cur_cmd = cur_chr = 0`, the meaning before any token is read.
impl Default for Command {
    fn default() -> Self {
        Command::Relax(RelaxCode::Zero)
    }
}

impl Command {
    /// The macro call of `body` for a `\long` and/or `\outer` definition.
    pub(crate) const fn call(body: TokenList, long: bool, outer: bool) -> Command {
        match (long, outer) {
            (false, false) => Command::Call(body),
            (true, false) => Command::LongCall(body),
            (false, true) => Command::OuterCall(body),
            (true, true) => Command::LongOuterCall(body),
        }
    }

    /// The definition of a macro call.
    #[inline]
    pub(crate) const fn macro_body(self) -> Option<TokenList> {
        match self {
            Command::Call(body)
            | Command::LongCall(body)
            | Command::OuterCall(body)
            | Command::LongOuterCall(body) => Some(body),
            _ => None,
        }
    }

    /// The command of a character token of category `cat`, `None` for a category
    /// no token carries.
    #[inline(always)]
    pub(crate) fn from_char(cat: Cmd, chr: i32) -> Option<Command> {
        let c = Usv::new(chr)?;
        Some(match cat {
            Cmd::LeftBrace => Command::LeftBrace(c),
            Cmd::RightBrace => Command::RightBrace(c),
            Cmd::MathShift => Command::MathShift(c),
            Cmd::TabMark => Command::TabMark(TabMarkCode::char(c)),
            Cmd::MacParam => Command::MacParam(c),
            Cmd::SupMark => Command::SupMark(c),
            Cmd::SubMark => Command::SubMark(c),
            Cmd::Spacer => Command::Spacer(c),
            Cmd::Letter => Command::Letter(c),
            Cmd::OtherChar => Command::OtherChar(c),
            _ => return None,
        })
    }

    /// The character a letter, other character or `\chardef`'d command typesets.
    #[inline(always)]
    pub(crate) fn char_code(self) -> Option<i32> {
        match self {
            Command::Letter(c) | Command::OtherChar(c) | Command::CharGiven(c) => Some(c.raw()),
            _ => None,
        }
    }

    /// `cur_tok` for this command when no control sequence carries it, TeX's
    /// `cur_cmd * max_char_val + cur_chr`.
    #[inline(always)]
    pub(crate) fn char_token(self) -> Token {
        match self {
            Command::LeftBrace(c)
            | Command::RightBrace(c)
            | Command::MathShift(c)
            | Command::MacParam(c)
            | Command::SupMark(c)
            | Command::SubMark(c)
            | Command::Spacer(c)
            | Command::Letter(c)
            | Command::OtherChar(c) => Token::new(self.cmd(), c.raw()),
            _ => {
                let (cmd, chr) = self.encode();
                Token::new(cmd, chr)
            }
        }
    }

    /// The command code, the variant's tag.
    #[inline(always)]
    pub(crate) const fn cmd(self) -> Cmd {
        match self {
            Command::Relax(..) => Cmd::Relax,
            Command::LeftBrace(..) => Cmd::LeftBrace,
            Command::RightBrace(..) => Cmd::RightBrace,
            Command::MathShift(..) => Cmd::MathShift,
            Command::TabMark(..) => Cmd::TabMark,
            Command::CarRet { .. } => Cmd::CarRet,
            Command::MacParam(..) => Cmd::MacParam,
            Command::SupMark(..) => Cmd::SupMark,
            Command::SubMark(..) => Cmd::SubMark,
            Command::Endv => Cmd::Endv,
            Command::Spacer(..) => Cmd::Spacer,
            Command::Letter(..) => Cmd::Letter,
            Command::OtherChar(..) => Cmd::OtherChar,
            Command::ParEnd => Cmd::ParEnd,
            Command::Stop { .. } => Cmd::Stop,
            Command::DelimNum(..) => Cmd::DelimNum,
            Command::CharNum => Cmd::CharNum,
            Command::MathCharNum(..) => Cmd::MathCharNum,
            Command::Mark { .. } => Cmd::Mark,
            Command::Xray(..) => Cmd::Xray,
            Command::MakeBox(..) => Cmd::MakeBox,
            Command::Hmove { .. } => Cmd::Hmove,
            Command::Vmove { .. } => Cmd::Vmove,
            Command::UnHbox(..) => Cmd::UnHbox,
            Command::UnVbox(..) => Cmd::UnVbox,
            Command::RemoveItem(..) => Cmd::RemoveItem,
            Command::Hskip(..) => Cmd::Hskip,
            Command::Vskip(..) => Cmd::Vskip,
            Command::Mskip => Cmd::Mskip,
            Command::Kern => Cmd::Kern,
            Command::Mkern => Cmd::Mkern,
            Command::LeaderShip(..) => Cmd::LeaderShip,
            Command::Halign => Cmd::Halign,
            Command::Valign(..) => Cmd::Valign,
            Command::NoAlign => Cmd::NoAlign,
            Command::Vrule => Cmd::Vrule,
            Command::Hrule => Cmd::Hrule,
            Command::Insert => Cmd::Insert,
            Command::Vadjust => Cmd::Vadjust,
            Command::IgnoreSpaces { .. } => Cmd::IgnoreSpaces,
            Command::AfterAssignment => Cmd::AfterAssignment,
            Command::AfterGroup => Cmd::AfterGroup,
            Command::BreakPenalty => Cmd::BreakPenalty,
            Command::StartPar { .. } => Cmd::StartPar,
            Command::ItalCorr => Cmd::ItalCorr,
            Command::Accent => Cmd::Accent,
            Command::MathAccent(..) => Cmd::MathAccent,
            Command::Discretionary { .. } => Cmd::Discretionary,
            Command::EqNo { .. } => Cmd::EqNo,
            Command::LeftRight(..) => Cmd::LeftRight,
            Command::MathComp(..) => Cmd::MathComp,
            Command::LimitSwitch(..) => Cmd::LimitSwitch,
            Command::Above(..) => Cmd::Above,
            Command::MathStyle(..) => Cmd::MathStyle,
            Command::MathChoice => Cmd::MathChoice,
            Command::NonScript => Cmd::NonScript,
            Command::Vcenter => Cmd::Vcenter,
            Command::CaseShift(..) => Cmd::CaseShift,
            Command::Message { .. } => Cmd::Message,
            Command::Extension(..) => Cmd::Extension,
            Command::InStream { .. } => Cmd::InStream,
            Command::BeginGroup => Cmd::BeginGroup,
            Command::EndGroup => Cmd::EndGroup,
            Command::Omit => Cmd::Omit,
            Command::ExSpace => Cmd::ExSpace,
            Command::NoBoundary => Cmd::NoBoundary,
            Command::Radical(..) => Cmd::Radical,
            Command::EndCsName => Cmd::EndCsName,
            Command::CharGiven(..) => Cmd::CharGiven,
            Command::MathGiven(..) => Cmd::MathGiven,
            Command::XetexMathGiven(..) => Cmd::XetexMathGiven,
            Command::LastItem(..) => Cmd::LastItem,
            Command::ToksRegister(..) => Cmd::ToksRegister,
            Command::AssignToks(..) => Cmd::AssignToks,
            Command::AssignInt(..) => Cmd::AssignInt,
            Command::AssignDimen(..) => Cmd::AssignDimen,
            Command::AssignGlue(..) => Cmd::AssignGlue,
            Command::AssignMuGlue(..) => Cmd::AssignMuGlue,
            Command::AssignFontDimen => Cmd::AssignFontDimen,
            Command::AssignFontInt(..) => Cmd::AssignFontInt,
            Command::SetAux(..) => Cmd::SetAux,
            Command::SetPrevGraf => Cmd::SetPrevGraf,
            Command::SetPageDimen(..) => Cmd::SetPageDimen,
            Command::SetPageInt(..) => Cmd::SetPageInt,
            Command::SetBoxDimen(..) => Cmd::SetBoxDimen,
            Command::SetShape(..) => Cmd::SetShape,
            Command::DefCode(..) => Cmd::DefCode,
            Command::XetexDefCode(..) => Cmd::XetexDefCode,
            Command::DefFamily(..) => Cmd::DefFamily,
            Command::SetFont(..) => Cmd::SetFont,
            Command::DefFont => Cmd::DefFont,
            Command::Register(..) => Cmd::Register,
            Command::Advance => Cmd::Advance,
            Command::Multiply => Cmd::Multiply,
            Command::Divide => Cmd::Divide,
            Command::Prefix(..) => Cmd::Prefix,
            Command::Let { .. } => Cmd::Let,
            Command::ShorthandDef(..) => Cmd::ShorthandDef,
            Command::ReadToCs { .. } => Cmd::ReadToCs,
            Command::Def { .. } => Cmd::Def,
            Command::SetBox => Cmd::SetBox,
            Command::HyphData { .. } => Cmd::HyphData,
            Command::SetInteraction(..) => Cmd::SetInteraction,
            Command::PartokenName => Cmd::PartokenName,
            Command::UndefinedCs => Cmd::UndefinedCs,
            Command::ExpandAfter { .. } => Cmd::ExpandAfter,
            Command::NoExpand { .. } => Cmd::NoExpand,
            Command::Input(..) => Cmd::Input,
            Command::IfTest { .. } => Cmd::IfTest,
            Command::FiOrElse(..) => Cmd::FiOrElse,
            Command::CsName => Cmd::CsName,
            Command::Convert(..) => Cmd::Convert,
            Command::The(..) => Cmd::The,
            Command::TopBotMark { .. } => Cmd::TopBotMark,
            Command::Call(..) => Cmd::Call,
            Command::LongCall(..) => Cmd::LongCall,
            Command::OuterCall(..) => Cmd::OuterCall,
            Command::LongOuterCall(..) => Cmd::LongOuterCall,
            Command::EndTemplate => Cmd::EndTemplate,
            Command::DontExpand => Cmd::DontExpand,
        }
    }

    /// The command as TeX's `(cmd, chr)` pair, for formats, printing and the save stack.
    pub(crate) fn encode(self) -> (Cmd, i32) {
        let flag = |on: bool, code: i32| if on { code } else { 0 };
        let chr = match self {
            Command::Relax(code) => code.raw(),
            Command::LeftBrace(c)
            | Command::RightBrace(c)
            | Command::MathShift(c)
            | Command::MacParam(c)
            | Command::SupMark(c)
            | Command::SubMark(c)
            | Command::Spacer(c)
            | Command::Letter(c)
            | Command::OtherChar(c)
            | Command::CharGiven(c) => c.raw(),
            Command::TabMark(t) => t.0,
            Command::CarRet { crcr } => {
                if crcr {
                    CR_CR_CODE
                } else {
                    CR_CODE
                }
            }
            Command::Stop { dump: on }
            | Command::Hmove { left: on }
            | Command::Vmove { raise: on }
            | Command::IgnoreSpaces { primitive: on }
            | Command::StartPar { indent: on }
            | Command::Discretionary { hyphen: on }
            | Command::EqNo { left: on }
            | Command::Message { err: on }
            | Command::InStream { open: on }
            | Command::Let { future: on }
            | Command::ReadToCs { line: on }
            | Command::HyphData { patterns: on }
            | Command::ExpandAfter { unless: on }
            | Command::NoExpand { primitive: on } => flag(on, 1),
            Command::Mark { marks } => flag(marks, MarkCode::MARKS),
            Command::DelimNum(code) => code.raw(),
            Command::MathCharNum(code) => code.raw(),
            Command::Xray(code) => code.raw(),
            Command::MakeBox(code) | Command::UnHbox(code) | Command::UnVbox(code) => code.raw(),
            Command::RemoveItem(t) | Command::MathComp(t) => i32::from(t.to_u16()),
            Command::Hskip(code) | Command::Vskip(code) => code.raw(),
            Command::LeaderShip(code) => code.raw(),
            Command::Valign(code) => code.raw(),
            Command::MathAccent(code) => code.raw(),
            Command::LeftRight(code) => code.raw(),
            Command::LimitSwitch(code) => code.raw(),
            Command::Above(code) => code.raw(),
            Command::MathStyle(style) => i32::from(style.raw()),
            Command::CaseShift(loc)
            | Command::SetShape(loc)
            | Command::DefCode(loc)
            | Command::XetexDefCode(loc)
            | Command::DefFamily(loc) => loc.raw(),
            Command::AssignInt(loc) => loc.raw(),
            Command::AssignToks(loc) => loc.raw(),
            Command::AssignGlue(loc) | Command::AssignMuGlue(loc) => loc.raw(),
            Command::AssignDimen(loc) => loc.raw(),
            Command::Extension(code) => code.raw(),
            Command::Radical(code) => code.raw(),
            Command::MathGiven(v) | Command::XetexMathGiven(v) => v,
            Command::LastItem(code) => code.raw(),
            Command::ToksRegister(r) => r.0,
            Command::AssignFontInt(code) => code.raw(),
            Command::SetAux(code) => code.raw(),
            Command::SetPageDimen(code) => code.raw(),
            Command::SetPageInt(code) => code.raw(),
            Command::SetBoxDimen(code) => code.raw(),
            Command::SetFont(f) => f.raw(),
            Command::Register(r) => r.0,
            Command::Prefix(code) => code.raw(),
            Command::ShorthandDef(code) => code.raw(),
            Command::Def { global, expand } => i32::from(global) + 2 * i32::from(expand),
            Command::SetInteraction(mode) => mode as i32,
            Command::Input(code) => code.raw(),
            Command::IfTest { code, unless } => code.raw() + flag(unless, IfCode::UNLESS),
            Command::FiOrElse(code) => code.raw(),
            Command::Convert(code) => code.raw(),
            Command::The(code) => code.raw(),
            Command::TopBotMark { code, marks } => code.raw() + flag(marks, MarkCode::MARKS),
            Command::Call(body)
            | Command::LongCall(body)
            | Command::OuterCall(body)
            | Command::LongOuterCall(body) => body.raw(),
            Command::Endv
            | Command::ParEnd
            | Command::CharNum
            | Command::Mskip
            | Command::Kern
            | Command::Mkern
            | Command::Halign
            | Command::NoAlign
            | Command::Vrule
            | Command::Hrule
            | Command::Insert
            | Command::Vadjust
            | Command::AfterAssignment
            | Command::AfterGroup
            | Command::BreakPenalty
            | Command::ItalCorr
            | Command::Accent
            | Command::MathChoice
            | Command::NonScript
            | Command::Vcenter
            | Command::BeginGroup
            | Command::EndGroup
            | Command::Omit
            | Command::ExSpace
            | Command::NoBoundary
            | Command::EndCsName
            | Command::AssignFontDimen
            | Command::SetPrevGraf
            | Command::DefFont
            | Command::Advance
            | Command::Multiply
            | Command::Divide
            | Command::SetBox
            | Command::PartokenName
            | Command::UndefinedCs
            | Command::CsName
            | Command::EndTemplate
            | Command::DontExpand => unit_chr(self.cmd()),
        };
        (self.cmd(), chr)
    }

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
                .and_then(|t| NodeType::of_code(t).ok())
        };
        if is_unit(cmd) {
            return (chr == unit_chr(cmd)).then(|| unit(cmd)).flatten();
        }
        Some(match cmd {
            Cmd::Relax => Command::Relax(RelaxCode::from_raw(chr)?),
            Cmd::LeftBrace => Command::LeftBrace(usv?),
            Cmd::RightBrace => Command::RightBrace(usv?),
            Cmd::MathShift => Command::MathShift(usv?),
            Cmd::TabMark if chr == SPAN_CODE => Command::TabMark(TabMarkCode::SPAN),
            Cmd::TabMark => Command::TabMark(TabMarkCode::char(usv?)),
            Cmd::CarRet => match chr {
                CR_CODE => Command::CarRet { crcr: false },
                CR_CR_CODE => Command::CarRet { crcr: true },
                _ => return None,
            },
            Cmd::MacParam => Command::MacParam(usv?),
            Cmd::SupMark => Command::SupMark(usv?),
            Cmd::SubMark => Command::SubMark(usv?),
            Cmd::Spacer => Command::Spacer(usv?),
            Cmd::Letter => Command::Letter(usv?),
            Cmd::OtherChar => Command::OtherChar(usv?),
            Cmd::Stop => Command::Stop { dump: flag(1)? },
            Cmd::DelimNum => Command::DelimNum(DelimNumCode::from_raw(chr)?),
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
            Cmd::LeaderShip => Command::LeaderShip(LeaderCode::from_raw(chr)?),
            Cmd::Valign => Command::Valign(ValignCode::from_raw(chr)?),
            Cmd::IgnoreSpaces => Command::IgnoreSpaces {
                primitive: flag(1)?,
            },
            Cmd::StartPar => Command::StartPar { indent: flag(1)? },
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
            Cmd::CaseShift => Command::CaseShift(loc),
            Cmd::Message => Command::Message { err: flag(1)? },
            Cmd::Extension => Command::Extension(ExtensionCode::from_raw(chr)?),
            Cmd::InStream => Command::InStream { open: flag(1)? },
            Cmd::Radical => Command::Radical(RadicalCode::from_raw(chr)?),
            Cmd::CharGiven => Command::CharGiven(usv?),
            Cmd::MathGiven => Command::MathGiven(chr),
            Cmd::XetexMathGiven => Command::XetexMathGiven(chr),
            Cmd::LastItem => Command::LastItem(LastItemCode::from_raw(chr)?),
            Cmd::ToksRegister => Command::ToksRegister(ToksRef(chr)),
            Cmd::AssignToks => Command::AssignToks(ToksLoc::from_raw(chr)?),
            Cmd::AssignInt => Command::AssignInt(IntLoc::from_raw(chr)?),
            Cmd::AssignDimen => Command::AssignDimen(DimenLoc::from_raw(chr)?),
            Cmd::AssignGlue => Command::AssignGlue(GlueLoc::from_raw(chr)?),
            Cmd::AssignMuGlue => Command::AssignMuGlue(GlueLoc::from_raw(chr)?),
            Cmd::AssignFontInt => Command::AssignFontInt(FontIntCode::from_raw(chr)?),
            Cmd::SetAux => Command::SetAux(AuxCode::from_raw(chr)?),
            Cmd::SetPageDimen => Command::SetPageDimen(PageDimenCode::from_raw(chr)?),
            Cmd::SetPageInt => Command::SetPageInt(PageIntCode::from_raw(chr)?),
            Cmd::SetBoxDimen => Command::SetBoxDimen(BoxDimenCode::from_raw(chr)?),
            Cmd::SetShape => Command::SetShape(loc),
            Cmd::DefCode => Command::DefCode(loc),
            Cmd::XetexDefCode => Command::XetexDefCode(loc),
            Cmd::DefFamily => Command::DefFamily(loc),
            Cmd::SetFont => Command::SetFont(FontId::try_from(chr).ok()?),
            Cmd::Register => match ValueLevel::from_raw(chr) {
                Some(level) if level > ValueLevel::Mu => return None,
                _ => Command::Register(RegisterCode(chr)),
            },
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
            Cmd::HyphData => Command::HyphData { patterns: flag(1)? },
            Cmd::SetInteraction => Command::SetInteraction(InteractionMode::from_raw(chr)?),
            Cmd::ExpandAfter => Command::ExpandAfter { unless: flag(1)? },
            Cmd::NoExpand => Command::NoExpand {
                primitive: flag(1)?,
            },
            Cmd::Input => Command::Input(InputCode::from_raw(chr)?),
            Cmd::IfTest if !(0..2 * IfCode::UNLESS).contains(&chr) => return None,
            Cmd::IfTest => {
                let (code, unless) = IfCode::decode(chr)?;
                Command::IfTest { code, unless }
            }
            Cmd::FiOrElse => Command::FiOrElse(FiOrElseCode::from_raw(chr)?),
            Cmd::Convert => Command::Convert(ConvertCode::from_raw(chr)?),
            Cmd::The => Command::The(TheCode::from_raw(chr)?),
            Cmd::TopBotMark if !(0..2 * MarkCode::MARKS).contains(&chr) => return None,
            Cmd::TopBotMark => Command::TopBotMark {
                code: MarkCode::from_raw(chr % MarkCode::MARKS)?,
                marks: chr >= MarkCode::MARKS,
            },
            Cmd::Call | Cmd::LongCall | Cmd::OuterCall | Cmd::LongOuterCall => Command::call(
                TokenList::from_raw(chr),
                cmd.is_long_call(),
                cmd.is_outer_call(),
            ),
            Cmd::GlueRef | Cmd::ShapeRef | Cmd::BoxRef | Cmd::Data => return None,
            _ => return None,
        })
    }
}

/// Whether `cmd` has no modifier, so one `chr` stands for it.
const fn is_unit(cmd: Cmd) -> bool {
    unit(cmd).is_some()
}

/// The command of a code without a modifier.
const fn unit(cmd: Cmd) -> Option<Command> {
    Some(match cmd {
        Cmd::Endv => Command::Endv,
        Cmd::ParEnd => Command::ParEnd,
        Cmd::CharNum => Command::CharNum,
        Cmd::Mskip => Command::Mskip,
        Cmd::Kern => Command::Kern,
        Cmd::Mkern => Command::Mkern,
        Cmd::Halign => Command::Halign,
        Cmd::NoAlign => Command::NoAlign,
        Cmd::Vrule => Command::Vrule,
        Cmd::Hrule => Command::Hrule,
        Cmd::Insert => Command::Insert,
        Cmd::Vadjust => Command::Vadjust,
        Cmd::AfterAssignment => Command::AfterAssignment,
        Cmd::AfterGroup => Command::AfterGroup,
        Cmd::BreakPenalty => Command::BreakPenalty,
        Cmd::ItalCorr => Command::ItalCorr,
        Cmd::Accent => Command::Accent,
        Cmd::MathChoice => Command::MathChoice,
        Cmd::NonScript => Command::NonScript,
        Cmd::Vcenter => Command::Vcenter,
        Cmd::BeginGroup => Command::BeginGroup,
        Cmd::EndGroup => Command::EndGroup,
        Cmd::Omit => Command::Omit,
        Cmd::ExSpace => Command::ExSpace,
        Cmd::NoBoundary => Command::NoBoundary,
        Cmd::EndCsName => Command::EndCsName,
        Cmd::AssignFontDimen => Command::AssignFontDimen,
        Cmd::SetPrevGraf => Command::SetPrevGraf,
        Cmd::DefFont => Command::DefFont,
        Cmd::Advance => Command::Advance,
        Cmd::Multiply => Command::Multiply,
        Cmd::Divide => Command::Divide,
        Cmd::SetBox => Command::SetBox,
        Cmd::PartokenName => Command::PartokenName,
        Cmd::UndefinedCs => Command::UndefinedCs,
        Cmd::CsName => Command::CsName,
        Cmd::EndTemplate => Command::EndTemplate,
        Cmd::DontExpand => Command::DontExpand,
        _ => return None,
    })
}

/// The one `chr` a command without a modifier is defined with.
const fn unit_chr(cmd: Cmd) -> i32 {
    match cmd {
        Cmd::ParEnd => TOO_BIG_USV,
        Cmd::Mskip => SkipCode::Mskip.raw(),
        Cmd::Kern => KERN_EXPLICIT,
        Cmd::Mkern => MKERN_MU_GLUE,
        Cmd::Endv | Cmd::EndTemplate => NULL_LIST.raw(),
        Cmd::UndefinedCs | Cmd::DontExpand => TEX_NULL,
        _ => 0,
    }
}

/// `\kern`'s `chr`, the subtype of an explicit kern.
const KERN_EXPLICIT: i32 = 1;
/// `\mkern`'s `chr`, TeX's `mu_glue`.
const MKERN_MU_GLUE: i32 = 99;

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
            (Cmd::TabMark, SPAN_CODE, Command::TabMark(TabMarkCode::SPAN)),
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
                Command::Relax(RelaxCode::NoExpand),
            ),
            (Cmd::RemoveItem, 11, Command::RemoveItem(NodeType::Kern)),
            (
                Cmd::Register,
                2,
                Command::Register(RegisterCode::new(RegisterRef::Scanned(ValueLevel::Glue))),
            ),
        ];
        for (cmd, chr, want) in cases {
            assert_eq!(Command::decode(cmd, chr), Some(want), "{cmd:?} {chr}");
        }
    }

    #[test]
    fn every_decoded_command_encodes_back_to_its_code_and_tag() {
        let extra = [
            0,
            1,
            2,
            3,
            4,
            7,
            11,
            SPAN_CODE,
            CR_CODE,
            CR_CR_CODE,
            NO_EXPAND_FLAG,
            TOO_BIG_USV,
            BIGGEST_USV,
            MarkCode::MARKS + 3,
            IfCode::UNLESS + 5,
            100_000,
            NULL_LIST.raw(),
            TEX_NULL,
            99,
        ];
        let mut checked = 0;
        for raw in 0..=123 {
            let cmd = Cmd::from_raw(raw).unwrap();
            let chrs = PRIMITIVES.iter().map(|p| p.chr).chain(extra);
            for chr in chrs {
                if let Some(c) = Command::decode(cmd, chr) {
                    assert_eq!(c.encode(), (cmd, chr), "{c:?}");
                    checked += 1;
                }
            }
        }
        assert!(checked > 1000, "{checked}");
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
            (Cmd::Relax, 7),
            (Cmd::CarRet, 0),
        ];
        for (cmd, chr) in cases {
            assert_eq!(Command::decode(cmd, chr), None, "{cmd:?} {chr}");
        }
    }
}
