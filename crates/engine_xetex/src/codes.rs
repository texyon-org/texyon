//! Enums for TeX's small numeric code sets: `chr_code` modifiers and the like.

/// Declares a fieldless enum with explicit discriminants plus `from_raw`
/// (a real `match`, so dispatch compiles to a jump table) and `raw`. With
/// `Variant = n => "name"` it also gets `name`, the primitive's name.
macro_rules! code_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident: $repr:ty {
            $($(#[$vmeta:meta])* $var:ident = $val:expr => $prim:literal),+ $(,)?
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
            $($(#[$vmeta:meta])* $var:ident = $val:expr),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Copy, Clone, PartialEq, Eq, Debug)]
        #[repr($repr)]
        $vis enum $name {
            $($(#[$vmeta])* $var = $val),+
        }

        #[allow(dead_code)]
        impl $name {
            #[inline]
            pub(crate) const fn from_raw(v: i32) -> Option<Self> {
                #![allow(non_upper_case_globals)]
                $(const $var: i32 = $val;)+
                match v {
                    $($var => Some(Self::$var),)+
                    _ => None,
                }
            }

            #[inline]
            pub(crate) const fn raw(self) -> i32 {
                self as i32
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

    /// TeX's `decr(cur_val_level)` when coercing: glue to dimen to int.
    pub(crate) const fn lower(self) -> ValueLevel {
        match self {
            ValueLevel::Int | ValueLevel::Dimen => ValueLevel::Int,
            ValueLevel::Glue => ValueLevel::Dimen,
            ValueLevel::Mu => ValueLevel::Glue,
            ValueLevel::Ident => ValueLevel::Mu,
            ValueLevel::Tok => ValueLevel::Ident,
            ValueLevel::InterChar => ValueLevel::Tok,
            ValueLevel::Mark => ValueLevel::InterChar,
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
