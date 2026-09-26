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
