//! Enums for TeX's small numeric code sets: `chr_code` modifiers and the like.

/// Declares a fieldless enum for one of TeX's numeric code sets, with
/// `from_raw` (a real `match`, so dispatch compiles to a jump table), `raw`
/// and the [`StoredCode`](crate::node::StoredCode) conversions. With
/// `Variant = n => "name"` it also gets `name`, the primitive's name.
macro_rules! code_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident: $repr:ty {
            $($(#[$vmeta:meta])* $var:ident = $val:literal => $prim:literal),+ $(,)?
        }
    ) => {
        code_enum! {
            $(#[$meta])*
            $vis enum $name: $repr {
                $($(#[$vmeta])* $var = $val),+
            }
        }

        impl $name {
            pub(crate) const fn name(self) -> &'static str {
                match self {
                    $(Self::$var => $prim,)+
                }
            }
        }
    };
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident: $repr:ty {
            $($(#[$vmeta:meta])* $var:ident = $val:literal),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Copy, Clone, PartialEq, Eq, Debug)]
        #[repr($repr)]
        $vis enum $name {
            $($(#[$vmeta])* $var = $val),+
        }

        impl $name {
            #[inline]
            pub(crate) const fn from_raw(v: i32) -> Option<Self> {
                match v {
                    $($val => Some(Self::$var),)+
                    _ => None,
                }
            }
        }

        impl $name {
            #[inline]
            pub(crate) const fn raw(self) -> i32 {
                self as i32
            }
        }

        impl From<$name> for i32 {
            #[inline]
            fn from(code: $name) -> i32 {
                code.raw()
            }
        }

        impl $name {
            /// The code a node field or save word stores as `raw`.
            #[inline]
            pub(crate) const fn from_u16(raw: u16) -> Option<Self> {
                Self::from_raw(raw as i32)
            }

            /// The code as a node field or save word stores it.
            #[inline]
            pub(crate) const fn to_u16(self) -> u16 {
                self as u16
            }
        }

        impl crate::node::StoredCode for $name {
            #[inline]
            fn from_u16(raw: u16) -> Option<Self> {
                $name::from_u16(raw)
            }
            #[inline]
            fn to_u16(self) -> u16 {
                $name::to_u16(self)
            }
        }
    };
}

pub(crate) use code_enum;

code_enum! {
    /// The type of a scanned value (`cur_val_level`), ordered as TeX compares
    /// them. `Int`..`Mark` also name the sparse-array register types.
    #[derive(PartialOrd, Ord, Default)]
    pub(crate) enum ValueLevel: u8 {
        #[default]
        Int = 0,
        Dimen = 1,
        Glue = 2,
        Mu = 3,
        Ident = 4,
        Tok = 5,
        InterChar = 6,
        Mark = 7,
    }
}

impl ValueLevel {
    /// `box_val`, the sparse-array type of box registers.
    pub(crate) const BOX: ValueLevel = ValueLevel::Ident;

    /// Where glue registers 0..=255 of this type live in `eqtb`.
    pub(crate) const fn register_base(self) -> Option<crate::units::EqtbPtr> {
        match self {
            ValueLevel::Glue => Some(crate::consts::SKIP_BASE),
            ValueLevel::Mu => Some(crate::consts::MU_SKIP_BASE),
            _ => None,
        }
    }
}

code_enum! {
    /// The `chr_code` of `\lastpenalty` and the other `last_item` primitives.
    #[derive(PartialOrd, Ord)]
    pub(crate) enum LastItemCode: u8 {
        LastPenalty = 0 => "lastpenalty",
        LastKern = 1 => "lastkern",
        LastSkip = 2 => "lastskip",
        LastNodeType = 3 => "lastnodetype",
        InputLineNo = 4 => "inputlineno",
        Badness = 5 => "badness",
        PdfLastXPos = 12 => "pdflastxpos",
        PdfLastYPos = 13 => "pdflastypos",
        ElapsedTime = 16 => "elapsedtime",
        PdfShellEscape = 17 => "shellescape",
        RandomSeed = 18 => "randomseed",
        EtexVersion = 19 => "eTeXversion",
        CurrentGroupLevel = 20 => "currentgrouplevel",
        CurrentGroupType = 21 => "currentgrouptype",
        CurrentIfLevel = 22 => "currentiflevel",
        CurrentIfType = 23 => "currentiftype",
        CurrentIfBranch = 24 => "currentifbranch",
        GlueStretchOrder = 25 => "gluestretchorder",
        GlueShrinkOrder = 26 => "glueshrinkorder",
        XetexVersion = 27 => "XeTeXversion",
        XetexCountGlyphs = 28 => "XeTeXcountglyphs",
        XetexCountVariations = 29 => "XeTeXcountvariations",
        XetexVariation = 30 => "XeTeXvariation",
        XetexFindVariationByName = 31 => "XeTeXfindvariationbyname",
        XetexVariationMin = 32 => "XeTeXvariationmin",
        XetexVariationMax = 33 => "XeTeXvariationmax",
        XetexVariationDefault = 34 => "XeTeXvariationdefault",
        XetexCountFeatures = 35 => "XeTeXcountfeatures",
        XetexFeatureCode = 36 => "XeTeXfeaturecode",
        XetexFindFeatureByName = 37 => "XeTeXfindfeaturebyname",
        XetexIsExclusiveFeature = 38 => "XeTeXisexclusivefeature",
        XetexCountSelectors = 39 => "XeTeXcountselectors",
        XetexSelectorCode = 40 => "XeTeXselectorcode",
        XetexFindSelectorByName = 41 => "XeTeXfindselectorbyname",
        XetexIsDefaultSelector = 42 => "XeTeXisdefaultselector",
        XetexOtCountScripts = 43 => "XeTeXOTcountscripts",
        XetexOtCountLanguages = 44 => "XeTeXOTcountlanguages",
        XetexOtCountFeatures = 45 => "XeTeXOTcountfeatures",
        XetexOtScript = 46 => "XeTeXOTscripttag",
        XetexOtLanguage = 47 => "XeTeXOTlanguagetag",
        XetexOtFeature = 48 => "XeTeXOTfeaturetag",
        XetexMapCharToGlyph = 49 => "XeTeXcharglyph",
        XetexGlyphIndex = 50 => "XeTeXglyphindex",
        XetexFontType = 51 => "XeTeXfonttype",
        XetexFirstChar = 52 => "XeTeXfirstfontchar",
        XetexLastChar = 53 => "XeTeXlastfontchar",
        XetexPdfPageCount = 54 => "XeTeXpdfpagecount",
        XetexGlyphBounds = 55 => "XeTeXglyphbounds",
        FontCharWd = 56 => "fontcharwd",
        FontCharHt = 57 => "fontcharht",
        FontCharDp = 58 => "fontchardp",
        FontCharIc = 59 => "fontcharic",
        ParShapeLength = 60 => "parshapelength",
        ParShapeIndent = 61 => "parshapeindent",
        ParShapeDimen = 62 => "parshapedimen",
        GlueStretch = 63 => "gluestretch",
        GlueShrink = 64 => "glueshrink",
        MuToGlue = 65 => "mutoglue",
        GlueToMu = 66 => "gluetomu",
        NumExpr = 67 => "numexpr",
        DimExpr = 68 => "dimexpr",
        GlueExpr = 69 => "glueexpr",
        MuExpr = 70 => "muexpr",
    }
}

code_enum! {
    /// The `chr_code` of `\number`, `\string` and the other `convert` primitives.
    pub(crate) enum ConvertCode: u8 {
        Number = 0 => "number",
        RomanNumeral = 1 => "romannumeral",
        String = 2 => "string",
        Meaning = 3 => "meaning",
        FontName = 4 => "fontname",
        EtexRevision = 5 => "eTeXrevision",
        Expanded = 6 => "expanded",
        LeftMarginKern = 16 => "leftmarginkern",
        RightMarginKern = 17 => "rightmarginkern",
        Strcmp = 18 => "strcmp",
        CreationDate = 22 => "creationdate",
        FileModDate = 23 => "filemoddate",
        FileSize = 24 => "filesize",
        MdfiveSum = 25 => "mdfivesum",
        FileDump = 26 => "filedump",
        UniformDeviate = 29 => "uniformdeviate",
        NormalDeviate = 30 => "normaldeviate",
        XetexRevision = 33 => "XeTeXrevision",
        XetexVariationName = 34 => "XeTeXvariationname",
        XetexFeatureName = 35 => "XeTeXfeaturename",
        XetexSelectorName = 36 => "XeTeXselectorname",
        XetexGlyphName = 37 => "XeTeXglyphname",
        Uchar = 38 => "Uchar",
        Ucharcat = 39 => "Ucharcat",
        JobName = 40 => "jobname",
    }
}

code_enum! {
    /// The `chr_code` of `\if`, `\ifx` and the other `if_test` primitives,
    /// without the `\unless` offset.
    pub(crate) enum IfCode: u8 {
        Char = 0 => "if",
        Cat = 1 => "ifcat",
        Num = 2 => "ifnum",
        Dim = 3 => "ifdim",
        Odd = 4 => "ifodd",
        VMode = 5 => "ifvmode",
        HMode = 6 => "ifhmode",
        MMode = 7 => "ifmmode",
        Inner = 8 => "ifinner",
        Void = 9 => "ifvoid",
        HBox = 10 => "ifhbox",
        VBox = 11 => "ifvbox",
        X = 12 => "ifx",
        Eof = 13 => "ifeof",
        True = 14 => "iftrue",
        False = 15 => "iffalse",
        Case = 16 => "ifcase",
        Defined = 17 => "ifdefined",
        CsName = 18 => "ifcsname",
        FontChar = 19 => "iffontchar",
        InCsName = 20 => "ifincsname",
        Primitive = 21 => "ifprimitive",
    }
}

impl IfCode {
    /// Added to the `chr_code` of an `\if` preceded by `\unless`.
    pub(crate) const UNLESS: i32 = 32;

    /// Splits an `if_test` `chr_code` into its test and whether it is negated.
    #[inline]
    pub(crate) const fn decode(chr: i32) -> Option<(IfCode, bool)> {
        match IfCode::from_raw(chr % IfCode::UNLESS) {
            Some(code) => Some((code, chr >= IfCode::UNLESS)),
            None => None,
        }
    }
}

code_enum! {
    /// `if_limit`, and the `chr_code` of `\fi`, `\else` and `\or`.
    #[derive(PartialOrd, Ord, Default)]
    pub(crate) enum FiOrElseCode: u8 {
        #[default]
        Normal = 0,
        If = 1,
        Fi = 2,
        Else = 3,
        Or = 4,
    }
}

code_enum! {
    /// The `chr_code` of `\box`, `\copy`, `\lastbox`, `\vsplit` and the box
    /// makers (`make_box`). `Copy`, `LastBox` and `Vsplit` also select
    /// `\unvcopy`, `\pagediscards` and `\splitdiscards`, and index `disc_ptr`.
    pub(crate) enum BoxCode: u8 {
        Box = 0 => "box",
        Copy = 1 => "copy",
        LastBox = 2 => "lastbox",
        Vsplit = 3 => "vsplit",
        Vtop = 4 => "vtop",
        Vbox = 5 => "vbox",
        Hbox = 109 => "hbox",
    }
}

const _: () = {
    assert!(BoxCode::Vbox.raw() == BoxCode::Vtop.raw() + crate::consts::VMODE as i32);
    assert!(BoxCode::Hbox.raw() == BoxCode::Vtop.raw() + crate::consts::HMODE as i32);
};

code_enum! {
    /// The `chr_code` of `\hfil`..`\hskip`, their vertical twins and `\mskip`.
    pub(crate) enum SkipCode: u8 {
        Fil = 0,
        Fill = 1,
        Ss = 2,
        FilNeg = 3,
        Skip = 4,
        Mskip = 5,
    }
}

code_enum! {
    /// The `chr_code` of `\chardef` and the other `shorthand_def` primitives.
    pub(crate) enum ShorthandDefCode: u8 {
        Char = 0 => "chardef",
        MathChar = 1 => "mathchardef",
        Count = 2 => "countdef",
        Dimen = 3 => "dimendef",
        Skip = 4 => "skipdef",
        MuSkip = 5 => "muskipdef",
        Toks = 6 => "toksdef",
        CharSub = 7 => "charsubdef",
        XetexMathCharNum = 8 => "Umathcharnumdef",
        XetexMathChar = 9 => "Umathchardef",
    }
}

code_enum! {
    /// The `chr_code` of `\wd`, `\dp` and `\ht`, the box field offsets.
    pub(crate) enum BoxDimenCode: u8 {
        Width = 1 => "wd",
        Depth = 2 => "dp",
        Height = 3 => "ht",
    }
}

code_enum! {
    /// The `chr_code` of `\openout` and the other `extension` primitives. The
    /// whatsit-making ones use their whatsit subtype.
    pub(crate) enum ExtensionCode: u8 {
        Open = 0 => "openout",
        Write = 1 => "write",
        Close = 2 => "closeout",
        Special = 3 => "special",
        Immediate = 5 => "immediate",
        SetLanguage = 6 => "setlanguage",
        PdfSavePos = 23 => "pdfsavepos",
        ResetTimer = 33 => "resettimer",
        SetRandomSeed = 35 => "setrandomseed",
        PicFile = 41 => "XeTeXpicfile",
        PdfFile = 42 => "XeTeXpdffile",
        Glyph = 43 => "XeTeXglyph",
        InputEncoding = 44 => "XeTeXinputencoding",
        DefaultEncoding = 45 => "XeTeXdefaultencoding",
        LinebreakLocale = 46 => "XeTeXlinebreaklocale",
    }
}

const _: () = {
    use crate::node::WhatsitSubtype;
    assert!(ExtensionCode::Open.raw() == WhatsitSubtype::Open as i32);
    assert!(ExtensionCode::Write.raw() == WhatsitSubtype::Write as i32);
    assert!(ExtensionCode::Close.raw() == WhatsitSubtype::Close as i32);
    assert!(ExtensionCode::Special.raw() == WhatsitSubtype::Special as i32);
    assert!(ExtensionCode::PdfSavePos.raw() == WhatsitSubtype::PdfSavePos as i32);
};

code_enum! {
    /// The `chr_code` of `\show` and the other `xray` primitives.
    pub(crate) enum ShowCode: u8 {
        Show = 0 => "show",
        Box = 1 => "showbox",
        The = 2 => "showthe",
        Lists = 3 => "showlists",
        Groups = 4 => "showgroups",
        Tokens = 5 => "showtokens",
        Ifs = 6 => "showifs",
    }
}

code_enum! {
    /// What the scanner is absorbing (`scanner_status`), for runaway and
    /// `\outer` checks. Ordered as TeX compares it.
    #[derive(PartialOrd, Ord, Default)]
    pub(crate) enum ScannerStatus: u8 {
        #[default]
        Normal = 0,
        Skipping = 1,
        Defining = 2,
        Matching = 3,
        Aligning = 4,
        Absorbing = 5,
    }
}

code_enum! {
    /// The `chr_code` of `\input`, `\endinput` and `\scantokens`.
    pub(crate) enum InputCode: u8 {
        Input = 0 => "input",
        EndInput = 1 => "endinput",
        ScanTokens = 2 => "scantokens",
    }
}

code_enum! {
    /// The `chr_code` of `\deadcycles`, `\insertpenalties` and `\interactionmode`.
    pub(crate) enum PageIntCode: u8 {
        DeadCycles = 0 => "deadcycles",
        InsertPenalties = 1 => "insertpenalties",
        InteractionMode = 2 => "interactionmode",
    }
}

code_enum! {
    /// A mark of the current page or split (`\topmark`..`\splitbotmark`),
    /// the index into `cur_mark` and into a mark class record. The `\marks`
    /// forms add `MarkCode::MARKS`.
    pub(crate) enum MarkCode: u8 {
        Top = 0 => "topmark",
        First = 1 => "firstmark",
        Bot = 2 => "botmark",
        SplitFirst = 3 => "splitfirstmark",
        SplitBot = 4 => "splitbotmark",
    }
}

impl MarkCode {
    /// `marks_code`, added to the `chr_code` of the `\marks` forms.
    pub(crate) const MARKS: i32 = 5;

    pub(crate) const ALL: [MarkCode; 5] = [
        MarkCode::Top,
        MarkCode::First,
        MarkCode::Bot,
        MarkCode::SplitFirst,
        MarkCode::SplitBot,
    ];
}

code_enum! {
    /// The `chr_code` of `\the`, `\unexpanded` and `\detokenize`.
    pub(crate) enum TheCode: u8 {
        The = 0 => "the",
        Unexpanded = 1 => "unexpanded",
        Detokenize = 5 => "detokenize",
    }
}

const _: () = assert!(TheCode::Detokenize.raw() == ShowCode::Tokens.raw());

code_enum! {
    /// The `chr_code` of `\mathchar` and its XeTeX forms.
    pub(crate) enum MathCharNumCode: u8 {
        MathChar = 0 => "mathchar",
        UmathCharNum = 1 => "Umathcharnum",
        UmathChar = 2 => "Umathchar",
    }
}

code_enum! {
    /// The `chr_code` of `\delimiter` and `\Udelimiter`.
    pub(crate) enum DelimNumCode: u8 {
        Delimiter = 0 => "delimiter",
        Udelimiter = 1 => "Udelimiter",
    }
}

code_enum! {
    /// The `chr_code` of `\mathaccent` and `\Umathaccent` (`\accent` in math is 0).
    pub(crate) enum MathAccentCode: u8 {
        MathAccent = 0 => "mathaccent",
        UmathAccent = 1 => "Umathaccent",
    }
}

code_enum! {
    /// The `chr_code` of `\radical` and `\Uradical`.
    pub(crate) enum RadicalCode: u8 {
        Radical = 0 => "radical",
        Uradical = 1 => "Uradical",
    }
}

code_enum! {
    /// The `chr_code` of `\shipout` and the leaders primitives.
    pub(crate) enum LeaderCode: u8 {
        Shipout = 99 => "shipout",
        ALeaders = 100 => "leaders",
        CLeaders = 101 => "cleaders",
        XLeaders = 102 => "xleaders",
    }
}

impl LeaderCode {
    /// The glue kind `\leaders`, `\cleaders` and `\xleaders` make, `None` for `\shipout`.
    pub(crate) fn glue_leaders(self) -> Option<crate::node::LeaderKind> {
        use crate::node::LeaderKind;
        match self {
            LeaderCode::Shipout => None,
            LeaderCode::ALeaders => Some(LeaderKind::Aligned),
            LeaderCode::CLeaders => Some(LeaderKind::Centered),
            LeaderCode::XLeaders => Some(LeaderKind::Expanded),
        }
    }
}

code_enum! {
    /// The `chr_code` of `\valign` and the TeX--XeT direction primitives.
    pub(crate) enum ValignCode: u8 {
        Valign = 0 => "valign",
        BeginL = 6 => "beginL",
        EndL = 7 => "endL",
        BeginR = 10 => "beginR",
        EndR = 11 => "endR",
    }
}

impl ValignCode {
    /// The math node `\beginL`..`\endR` append, `None` for `\valign`.
    pub(crate) fn math_code(self) -> Option<crate::node::MathCode> {
        use crate::node::MathCode;
        match self {
            ValignCode::Valign => None,
            ValignCode::BeginL => Some(MathCode::BeginL),
            ValignCode::EndL => Some(MathCode::EndL),
            ValignCode::BeginR => Some(MathCode::BeginR),
            ValignCode::EndR => Some(MathCode::EndR),
        }
    }
}

code_enum! {
    /// The `chr_code` of `\middle`, `\left` and `\right`: the noad they make.
    pub(crate) enum LeftRightCode: u8 {
        Middle = 1 => "middle",
        Left = 30 => "left",
        Right = 31 => "right",
    }
}

code_enum! {
    /// The `chr_code` of `\displaylimits`, `\limits` and `\nolimits`.
    pub(crate) enum LimitCode: u8 {
        Normal = 0 => "displaylimits",
        Limits = 1 => "limits",
        NoLimits = 2 => "nolimits",
    }
}

code_enum! {
    /// The `chr_code` of `\above` and the other generalized fractions.
    pub(crate) enum AboveCode: u8 {
        Above = 0 => "above",
        Over = 1 => "over",
        Atop = 2 => "atop",
        AboveWithDelims = 3 => "abovewithdelims",
        OverWithDelims = 4 => "overwithdelims",
        AtopWithDelims = 5 => "atopwithdelims",
    }
}

code_enum! {
    /// The `chr_code` of `\hyphenchar`, `\skewchar`, `\lpcode` and `\rpcode`.
    pub(crate) enum FontIntCode: u8 {
        HyphenChar = 0 => "hyphenchar",
        SkewChar = 1 => "skewchar",
        LpCode = 2 => "lpcode",
        RpCode = 3 => "rpcode",
    }
}

code_enum! {
    /// The `chr_code` of `\prevdepth` and `\spacefactor`: the mode they belong to.
    pub(crate) enum AuxCode: u8 {
        PrevDepth = 1 => "prevdepth",
        SpaceFactor = 105 => "spacefactor",
    }
}

impl AuxCode {
    /// The mode in which the quantity exists.
    pub(crate) const fn mode(self) -> i16 {
        self as i16
    }
}

code_enum! {
    /// The `chr_code` of `\pagegoal` and the other `page_so_far` entries.
    pub(crate) enum PageDimenCode: u8 {
        Goal = 0 => "pagegoal",
        Total = 1 => "pagetotal",
        Stretch = 2 => "pagestretch",
        FilStretch = 3 => "pagefilstretch",
        FillStretch = 4 => "pagefillstretch",
        FilllStretch = 5 => "pagefilllstretch",
        Shrink = 6 => "pageshrink",
        Depth = 7 => "pagedepth",
    }
}

impl PageDimenCode {
    /// The index into `page_so_far`.
    pub(crate) const fn index(self) -> usize {
        self as usize
    }
}

code_enum! {
    /// The `chr_code` of `\long`, `\outer`, `\global` and `\protected`, a prefix bit.
    pub(crate) enum PrefixCode: u8 {
        Long = 1 => "long",
        Outer = 2 => "outer",
        Global = 4 => "global",
        Protected = 8 => "protected",
    }
}

const _: () = {
    assert!(AuxCode::PrevDepth.mode() == crate::consts::VMODE);
    assert!(AuxCode::SpaceFactor.mode() == crate::consts::HMODE);
    assert!(LeftRightCode::Left.raw() == crate::units::NodeType::LeftNoad as i32);
    assert!(LeftRightCode::Right.raw() == crate::units::NodeType::RightNoad as i32);
    assert!(FontIntCode::LpCode.raw() == crate::main_control::LP_CODE_BASE);
    assert!(FontIntCode::RpCode.raw() == crate::main_control::RP_CODE_BASE);
};
