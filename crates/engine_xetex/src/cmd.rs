//! Command codes (`cur_cmd`, `eq_type`), ordered as in xetex.web.

use crate::codes::code_enum;

code_enum! {
    /// A command code (`cur_cmd`, `eq_type`).
    #[derive(PartialOrd, Ord, Default)]
    pub(crate) enum Cmd: u8 {
        #[default]
        Relax = 0,
        LeftBrace = 1,
        RightBrace = 2,
        MathShift = 3,
        TabMark = 4,
        CarRet = 5,
        MacParam = 6,
        SupMark = 7,
        SubMark = 8,
        Endv = 9,
        Spacer = 10,
        Letter = 11,
        OtherChar = 12,
        ParEnd = 13,
        Stop = 14,
        DelimNum = 15,
        CharNum = 16,
        MathCharNum = 17,
        Mark = 18,
        Xray = 19,
        MakeBox = 20,
        Hmove = 21,
        Vmove = 22,
        UnHbox = 23,
        UnVbox = 24,
        RemoveItem = 25,
        Hskip = 26,
        Vskip = 27,
        Mskip = 28,
        Kern = 29,
        Mkern = 30,
        LeaderShip = 31,
        Halign = 32,
        Valign = 33,
        NoAlign = 34,
        Vrule = 35,
        Hrule = 36,
        Insert = 37,
        Vadjust = 38,
        IgnoreSpaces = 39,
        AfterAssignment = 40,
        AfterGroup = 41,
        BreakPenalty = 42,
        StartPar = 43,
        ItalCorr = 44,
        Accent = 45,
        MathAccent = 46,
        Discretionary = 47,
        EqNo = 48,
        LeftRight = 49,
        MathComp = 50,
        LimitSwitch = 51,
        Above = 52,
        MathStyle = 53,
        MathChoice = 54,
        NonScript = 55,
        Vcenter = 56,
        CaseShift = 57,
        Message = 58,
        Extension = 59,
        InStream = 60,
        BeginGroup = 61,
        EndGroup = 62,
        Omit = 63,
        ExSpace = 64,
        NoBoundary = 65,
        Radical = 66,
        EndCsName = 67,
        CharGiven = 68,
        MathGiven = 69,
        XetexMathGiven = 70,
        LastItem = 71,
        ToksRegister = 72,
        AssignToks = 73,
        AssignInt = 74,
        AssignDimen = 75,
        AssignGlue = 76,
        AssignMuGlue = 77,
        AssignFontDimen = 78,
        AssignFontInt = 79,
        SetAux = 80,
        SetPrevGraf = 81,
        SetPageDimen = 82,
        SetPageInt = 83,
        SetBoxDimen = 84,
        SetShape = 85,
        DefCode = 86,
        XetexDefCode = 87,
        DefFamily = 88,
        SetFont = 89,
        DefFont = 90,
        Register = 91,
        Advance = 92,
        Multiply = 93,
        Divide = 94,
        Prefix = 95,
        Let = 96,
        ShorthandDef = 97,
        ReadToCs = 98,
        Def = 99,
        SetBox = 100,
        HyphData = 101,
        SetInteraction = 102,
        PartokenName = 103,
        UndefinedCs = 104,
        ExpandAfter = 105,
        NoExpand = 106,
        Input = 107,
        IfTest = 108,
        FiOrElse = 109,
        CsName = 110,
        Convert = 111,
        The = 112,
        TopBotMark = 113,
        Call = 114,
        LongCall = 115,
        OuterCall = 116,
        LongOuterCall = 117,
        EndTemplate = 118,
        DontExpand = 119,
        GlueRef = 120,
        ShapeRef = 121,
        BoxRef = 122,
        Data = 123,
    }
}

impl Cmd {
    pub(crate) const ESCAPE: Cmd = Cmd::Relax;
    pub(crate) const OUT_PARAM: Cmd = Cmd::CarRet;
    pub(crate) const IGNORE: Cmd = Cmd::Endv;
    pub(crate) const ACTIVE_CHAR: Cmd = Cmd::ParEnd;
    pub(crate) const MATCH: Cmd = Cmd::ParEnd;
    pub(crate) const COMMENT: Cmd = Cmd::Stop;
    pub(crate) const END_MATCH: Cmd = Cmd::Stop;
    pub(crate) const INVALID_CHAR: Cmd = Cmd::DelimNum;
    pub(crate) const MAX_CHAR_CODE: Cmd = Cmd::DelimNum;
    pub(crate) const MIN_INTERNAL: Cmd = Cmd::CharGiven;
    pub(crate) const MAX_NON_PREFIXED_COMMAND: Cmd = Cmd::LastItem;
    pub(crate) const MAX_INTERNAL: Cmd = Cmd::Register;
    pub(crate) const MAX_COMMAND: Cmd = Cmd::PartokenName;
}

impl Cmd {
    /// The category of a character token. A character token's quotient by
    /// `max_char_val` is below 16, and every code below 16 is a category.
    #[inline(always)]
    pub(crate) const fn from_catcode_bits(v: i32) -> Cmd {
        const CATEGORIES: [Cmd; 16] = [
            Cmd::Relax,
            Cmd::LeftBrace,
            Cmd::RightBrace,
            Cmd::MathShift,
            Cmd::TabMark,
            Cmd::CarRet,
            Cmd::MacParam,
            Cmd::SupMark,
            Cmd::SubMark,
            Cmd::Endv,
            Cmd::Spacer,
            Cmd::Letter,
            Cmd::OtherChar,
            Cmd::ParEnd,
            Cmd::Stop,
            Cmd::DelimNum,
        ];
        CATEGORIES[(v & 0xF) as usize]
    }

    /// A command read back from a memory word that holds one. Only the engine
    /// writes those words, and the format loader checks every `eq_type`.
    #[inline(always)]
    pub(crate) fn from_stored(v: impl Into<i32>) -> Cmd {
        let v = v.into();
        let cmd = Cmd::from_raw(v);
        debug_assert!(cmd.is_some(), "stored command {v} is not a command");
        cmd.unwrap_or(Cmd::UndefinedCs)
    }
}

impl From<Cmd> for u16 {
    #[inline(always)]
    fn from(cmd: Cmd) -> u16 {
        cmd as u16
    }
}

impl From<Cmd> for i32 {
    #[inline(always)]
    fn from(cmd: Cmd) -> i32 {
        cmd.raw()
    }
}

impl Cmd {
    /// The macro call command for a `\long` and/or `\outer` definition.
    pub(crate) const fn call(long: bool, outer: bool) -> Cmd {
        match (long, outer) {
            (false, false) => Cmd::Call,
            (true, false) => Cmd::LongCall,
            (false, true) => Cmd::OuterCall,
            (true, true) => Cmd::LongOuterCall,
        }
    }

    pub(crate) const fn is_long_call(self) -> bool {
        matches!(self, Cmd::LongCall | Cmd::LongOuterCall)
    }

    pub(crate) const fn is_outer_call(self) -> bool {
        matches!(self, Cmd::OuterCall | Cmd::LongOuterCall)
    }

    /// The same macro call without `\outer`.
    pub(crate) const fn without_outer(self) -> Cmd {
        match self {
            Cmd::OuterCall => Cmd::Call,
            Cmd::LongOuterCall => Cmd::LongCall,
            other => other,
        }
    }

    /// A command that may follow `\global`, `\long` or `\outer`.
    pub(crate) fn is_prefixed(self) -> bool {
        self > Cmd::MAX_NON_PREFIXED_COMMAND && self <= Cmd::MAX_COMMAND
    }
}
