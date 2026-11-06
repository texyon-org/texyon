//! The `eqtb` parameters read through `EqtbState::int_par`, `dimen_par` and
//! `glue_par`, numbered as offsets from their region base.

use crate::codes::code_enum;
use crate::consts::{DIMEN_BASE, GLUE_BASE, INT_BASE, LOCAL_BASE};
use crate::units::EqtbPtr;

code_enum! {
    pub(crate) enum IntPar: u8 {
        Pretolerance = 0,
        Tolerance = 1,
        LinePenalty = 2,
        HyphenPenalty = 3,
        ExHyphenPenalty = 4,
        ClubPenalty = 5,
        WidowPenalty = 6,
        DisplayWidowPenalty = 7,
        BrokenPenalty = 8,
        BinOpPenalty = 9,
        RelPenalty = 10,
        PreDisplayPenalty = 11,
        PostDisplayPenalty = 12,
        InterLinePenalty = 13,
        DoubleHyphenDemerits = 14,
        FinalHyphenDemerits = 15,
        AdjDemerits = 16,
        Mag = 17,
        DelimiterFactor = 18,
        Looseness = 19,
        Time = 20,
        Day = 21,
        Month = 22,
        Year = 23,
        ShowBoxBreadth = 24,
        ShowBoxDepth = 25,
        Hbadness = 26,
        Vbadness = 27,
        Pausing = 28,
        TracingOnline = 29,
        TracingMacros = 30,
        TracingStats = 31,
        TracingParagraphs = 32,
        TracingPages = 33,
        TracingOutput = 34,
        TracingLostChars = 35,
        TracingCommands = 36,
        TracingRestores = 37,
        UcHyph = 38,
        OutputPenalty = 39,
        MaxDeadCycles = 40,
        HangAfter = 41,
        FloatingPenalty = 42,
        GlobalDefs = 43,
        CurFam = 44,
        EscapeChar = 45,
        DefaultHyphenChar = 46,
        DefaultSkewChar = 47,
        EndLineChar = 48,
        NewLineChar = 49,
        Language = 50,
        LeftHyphenMin = 51,
        RightHyphenMin = 52,
        HoldingInserts = 53,
        ErrorContextLines = 54,
        TracingStackLevels = 55,
        TracingAssigns = 56,
        TracingGroups = 57,
        TracingIfs = 58,
        TracingScanTokens = 59,
        TracingNesting = 60,
        PreDisplayDirection = 61,
        LastLineFit = 62,
        SavingVdiscards = 63,
        SavingHyphCodes = 64,
        SuppressFontnotfoundError = 65,
        XetexLinebreakLocale = 66,
        XetexLinebreakPenalty = 67,
        XetexProtrudeChars = 68,
        Texxet = 69,
        XetexDashBreak = 70,
        XetexUpwards = 71,
        XetexUseGlyphMetrics = 72,
        XetexInterCharTokens = 73,
        XetexInputNormalization = 74,
        XetexDefaultInputMode = 75,
        XetexDefaultInputEncoding = 76,
        XetexTracingFonts = 77,
        XetexInterwordSpaceShaping = 78,
        XetexGenerateActualText = 79,
        XetexHyphenatableLength = 80,
        Synctex = 81,
        PdfOutput = 82,
    }
}

impl IntPar {
    /// The primitive name, `None` for a parameter without one.
    pub(crate) const fn name(self) -> Option<&'static str> {
        Some(match self {
            IntPar::Pretolerance => "pretolerance",
            IntPar::Tolerance => "tolerance",
            IntPar::LinePenalty => "linepenalty",
            IntPar::HyphenPenalty => "hyphenpenalty",
            IntPar::ExHyphenPenalty => "exhyphenpenalty",
            IntPar::ClubPenalty => "clubpenalty",
            IntPar::WidowPenalty => "widowpenalty",
            IntPar::DisplayWidowPenalty => "displaywidowpenalty",
            IntPar::BrokenPenalty => "brokenpenalty",
            IntPar::BinOpPenalty => "binoppenalty",
            IntPar::RelPenalty => "relpenalty",
            IntPar::PreDisplayPenalty => "predisplaypenalty",
            IntPar::PostDisplayPenalty => "postdisplaypenalty",
            IntPar::InterLinePenalty => "interlinepenalty",
            IntPar::DoubleHyphenDemerits => "doublehyphendemerits",
            IntPar::FinalHyphenDemerits => "finalhyphendemerits",
            IntPar::AdjDemerits => "adjdemerits",
            IntPar::Mag => "mag",
            IntPar::DelimiterFactor => "delimiterfactor",
            IntPar::Looseness => "looseness",
            IntPar::Time => "time",
            IntPar::Day => "day",
            IntPar::Month => "month",
            IntPar::Year => "year",
            IntPar::ShowBoxBreadth => "showboxbreadth",
            IntPar::ShowBoxDepth => "showboxdepth",
            IntPar::Hbadness => "hbadness",
            IntPar::Vbadness => "vbadness",
            IntPar::Pausing => "pausing",
            IntPar::TracingOnline => "tracingonline",
            IntPar::TracingMacros => "tracingmacros",
            IntPar::TracingStats => "tracingstats",
            IntPar::TracingParagraphs => "tracingparagraphs",
            IntPar::TracingPages => "tracingpages",
            IntPar::TracingOutput => "tracingoutput",
            IntPar::TracingLostChars => "tracinglostchars",
            IntPar::TracingCommands => "tracingcommands",
            IntPar::TracingRestores => "tracingrestores",
            IntPar::UcHyph => "uchyph",
            IntPar::OutputPenalty => "outputpenalty",
            IntPar::MaxDeadCycles => "maxdeadcycles",
            IntPar::HangAfter => "hangafter",
            IntPar::FloatingPenalty => "floatingpenalty",
            IntPar::GlobalDefs => "globaldefs",
            IntPar::CurFam => "fam",
            IntPar::EscapeChar => "escapechar",
            IntPar::DefaultHyphenChar => "defaulthyphenchar",
            IntPar::DefaultSkewChar => "defaultskewchar",
            IntPar::EndLineChar => "endlinechar",
            IntPar::NewLineChar => "newlinechar",
            IntPar::Language => "language",
            IntPar::LeftHyphenMin => "lefthyphenmin",
            IntPar::RightHyphenMin => "righthyphenmin",
            IntPar::HoldingInserts => "holdinginserts",
            IntPar::ErrorContextLines => "errorcontextlines",
            IntPar::TracingStackLevels => "tracingstacklevels",
            IntPar::TracingAssigns => "tracingassigns",
            IntPar::TracingGroups => "tracinggroups",
            IntPar::TracingIfs => "tracingifs",
            IntPar::TracingScanTokens => "tracingscantokens",
            IntPar::TracingNesting => "tracingnesting",
            IntPar::PreDisplayDirection => "predisplaydirection",
            IntPar::LastLineFit => "lastlinefit",
            IntPar::SavingVdiscards => "savingvdiscards",
            IntPar::SavingHyphCodes => "savinghyphcodes",
            IntPar::SuppressFontnotfoundError => "suppressfontnotfounderror",
            IntPar::XetexLinebreakPenalty => "XeTeXlinebreakpenalty",
            IntPar::XetexProtrudeChars => "XeTeXprotrudechars",
            IntPar::Texxet => "TeXXeTstate",
            IntPar::XetexDashBreak => "XeTeXdashbreakstate",
            IntPar::XetexUpwards => "XeTeXupwardsmode",
            IntPar::XetexUseGlyphMetrics => "XeTeXuseglyphmetrics",
            IntPar::XetexInterCharTokens => "XeTeXinterchartokenstate",
            IntPar::XetexInputNormalization => "XeTeXinputnormalization",
            IntPar::XetexTracingFonts => "XeTeXtracingfonts",
            IntPar::XetexInterwordSpaceShaping => "XeTeXinterwordspaceshaping",
            IntPar::XetexGenerateActualText => "XeTeXgenerateactualtext",
            IntPar::XetexHyphenatableLength => "XeTeXhyphenatablelength",
            IntPar::Synctex => "synctex",
            IntPar::PdfOutput => "pdfoutput",
            _ => return None,
        })
    }

    /// The `eqtb` location.
    #[inline]
    pub(crate) const fn loc(self) -> EqtbPtr {
        INT_BASE.plus(self as i32)
    }
}

code_enum! {
    pub(crate) enum DimenPar: u8 {
        ParIndent = 0,
        MathSurround = 1,
        LineSkipLimit = 2,
        Hsize = 3,
        Vsize = 4,
        MaxDepth = 5,
        SplitMaxDepth = 6,
        BoxMaxDepth = 7,
        Hfuzz = 8,
        Vfuzz = 9,
        DelimiterShortfall = 10,
        NullDelimiterSpace = 11,
        ScriptSpace = 12,
        PreDisplaySize = 13,
        DisplayWidth = 14,
        DisplayIndent = 15,
        OverfullRule = 16,
        HangIndent = 17,
        HOffset = 18,
        VOffset = 19,
        EmergencyStretch = 20,
        PdfPageWidth = 21,
        PdfPageHeight = 22,
    }
}

impl DimenPar {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            DimenPar::ParIndent => "parindent",
            DimenPar::MathSurround => "mathsurround",
            DimenPar::LineSkipLimit => "lineskiplimit",
            DimenPar::Hsize => "hsize",
            DimenPar::Vsize => "vsize",
            DimenPar::MaxDepth => "maxdepth",
            DimenPar::SplitMaxDepth => "splitmaxdepth",
            DimenPar::BoxMaxDepth => "boxmaxdepth",
            DimenPar::Hfuzz => "hfuzz",
            DimenPar::Vfuzz => "vfuzz",
            DimenPar::DelimiterShortfall => "delimitershortfall",
            DimenPar::NullDelimiterSpace => "nulldelimiterspace",
            DimenPar::ScriptSpace => "scriptspace",
            DimenPar::PreDisplaySize => "predisplaysize",
            DimenPar::DisplayWidth => "displaywidth",
            DimenPar::DisplayIndent => "displayindent",
            DimenPar::OverfullRule => "overfullrule",
            DimenPar::HangIndent => "hangindent",
            DimenPar::HOffset => "hoffset",
            DimenPar::VOffset => "voffset",
            DimenPar::EmergencyStretch => "emergencystretch",
            DimenPar::PdfPageWidth => "pdfpagewidth",
            DimenPar::PdfPageHeight => "pdfpageheight",
        }
    }

    /// The `eqtb` location.
    #[inline]
    pub(crate) const fn loc(self) -> EqtbPtr {
        DIMEN_BASE.plus(self as i32)
    }
}

code_enum! {
    #[allow(clippy::enum_variant_names)] // named after \lineskip..\thickmuskip
    pub(crate) enum GluePar: u8 {
        LineSkip = 0,
        BaselineSkip = 1,
        ParSkip = 2,
        AboveDisplaySkip = 3,
        BelowDisplaySkip = 4,
        AboveDisplayShortSkip = 5,
        BelowDisplayShortSkip = 6,
        LeftSkip = 7,
        RightSkip = 8,
        TopSkip = 9,
        SplitTopSkip = 10,
        TabSkip = 11,
        SpaceSkip = 12,
        XspaceSkip = 13,
        ParFillSkip = 14,
        XetexLinebreakSkip = 15,
        ThinMuSkip = 16,
        MedMuSkip = 17,
        ThickMuSkip = 18,
    }
}

impl GluePar {
    /// The subtype of a glue node made from this parameter.
    #[inline]
    pub(crate) const fn subtype(self) -> u16 {
        self as u16 + 1
    }

    pub(crate) const fn name(self) -> &'static str {
        match self {
            GluePar::LineSkip => "lineskip",
            GluePar::BaselineSkip => "baselineskip",
            GluePar::ParSkip => "parskip",
            GluePar::AboveDisplaySkip => "abovedisplayskip",
            GluePar::BelowDisplaySkip => "belowdisplayskip",
            GluePar::AboveDisplayShortSkip => "abovedisplayshortskip",
            GluePar::BelowDisplayShortSkip => "belowdisplayshortskip",
            GluePar::LeftSkip => "leftskip",
            GluePar::RightSkip => "rightskip",
            GluePar::TopSkip => "topskip",
            GluePar::SplitTopSkip => "splittopskip",
            GluePar::TabSkip => "tabskip",
            GluePar::SpaceSkip => "spaceskip",
            GluePar::XspaceSkip => "xspaceskip",
            GluePar::ParFillSkip => "parfillskip",
            GluePar::XetexLinebreakSkip => "XeTeXlinebreakskip",
            GluePar::ThinMuSkip => "thinmuskip",
            GluePar::MedMuSkip => "medmuskip",
            GluePar::ThickMuSkip => "thickmuskip",
        }
    }

    /// The `eqtb` location.
    #[inline]
    pub(crate) const fn loc(self) -> EqtbPtr {
        GLUE_BASE.plus(self as i32)
    }
}

code_enum! {
    /// The token list parameters, after `\parshape` in the local region.
    pub(crate) enum TokPar: u8 {
        OutputRoutine = 1 => "output",
        EveryPar = 2 => "everypar",
        EveryMath = 3 => "everymath",
        EveryDisplay = 4 => "everydisplay",
        EveryHbox = 5 => "everyhbox",
        EveryVbox = 6 => "everyvbox",
        EveryJob = 7 => "everyjob",
        EveryCr = 8 => "everycr",
        ErrHelp = 9 => "errhelp",
        EveryEof = 10 => "everyeof",
        XetexInterCharToks = 11 => "XeTeXinterchartoks",
        TectonicCoda = 12 => "TectonicCodaTokens",
    }
}

impl TokPar {
    /// The `eqtb` location.
    #[inline]
    pub(crate) const fn loc(self) -> EqtbPtr {
        LOCAL_BASE.plus(self as i32)
    }
}
