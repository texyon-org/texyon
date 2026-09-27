//! Canonical RGB colors passed to drawing backends.

use crate::units::normalize_zero;

/// RGB color in the backend's normalized component space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    /// Red component.
    pub red: f32,
    /// Green component.
    pub green: f32,
    /// Blue component.
    pub blue: f32,
    /// Alpha component.
    pub alpha: f32,
}

impl Color {
    /// Opaque black.
    pub const BLACK: Self = Self {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
        alpha: 1.0,
    };

    pub(crate) fn gray(value: f32) -> Self {
        Self::rgb(value, value, value)
    }

    pub(crate) fn rgb(red: f32, green: f32, blue: f32) -> Self {
        Self::rgba(red, green, blue, 1.0)
    }

    pub(crate) fn rgba(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self {
            red: canonical(red),
            green: canonical(green),
            blue: canonical(blue),
            alpha: canonical(alpha),
        }
    }

    pub(crate) fn cmyk(cyan: f32, magenta: f32, yellow: f32, black: f32) -> Self {
        Self::rgb(
            (1.0 - cyan) * (1.0 - black),
            (1.0 - magenta) * (1.0 - black),
            (1.0 - yellow) * (1.0 - black),
        )
    }
    /// A dvips color name, as `dvipsnam.def` and dvipdfmx define them.
    pub(crate) fn dvips_named(name: &str) -> Option<Self> {
        let [cyan, magenta, yellow, black] = match name {
            "GreenYellow" => [0.15, 0.0, 0.69, 0.0],
            "Yellow" => [0.0, 0.0, 1.0, 0.0],
            "Goldenrod" => [0.0, 0.10, 0.84, 0.0],
            "Dandelion" => [0.0, 0.29, 0.84, 0.0],
            "Apricot" => [0.0, 0.32, 0.52, 0.0],
            "Peach" => [0.0, 0.50, 0.70, 0.0],
            "Melon" => [0.0, 0.46, 0.50, 0.0],
            "YellowOrange" => [0.0, 0.42, 1.0, 0.0],
            "Orange" => [0.0, 0.61, 0.87, 0.0],
            "BurntOrange" => [0.0, 0.51, 1.0, 0.0],
            "Bittersweet" => [0.0, 0.75, 1.0, 0.24],
            "RedOrange" => [0.0, 0.77, 0.87, 0.0],
            "Mahogany" => [0.0, 0.85, 0.87, 0.35],
            "Maroon" => [0.0, 0.87, 0.68, 0.32],
            "BrickRed" => [0.0, 0.89, 0.94, 0.28],
            "Red" => [0.0, 1.0, 1.0, 0.0],
            "OrangeRed" => [0.0, 1.0, 0.50, 0.0],
            "RubineRed" => [0.0, 1.0, 0.13, 0.0],
            "WildStrawberry" => [0.0, 0.96, 0.39, 0.0],
            "Salmon" => [0.0, 0.53, 0.38, 0.0],
            "CarnationPink" => [0.0, 0.63, 0.0, 0.0],
            "Magenta" => [0.0, 1.0, 0.0, 0.0],
            "VioletRed" => [0.0, 0.81, 0.0, 0.0],
            "Rhodamine" => [0.0, 0.82, 0.0, 0.0],
            "Mulberry" => [0.34, 0.90, 0.0, 0.02],
            "RedViolet" => [0.07, 0.90, 0.0, 0.34],
            "Fuchsia" => [0.47, 0.91, 0.0, 0.08],
            "Lavender" => [0.0, 0.48, 0.0, 0.0],
            "Thistle" => [0.12, 0.59, 0.0, 0.0],
            "Orchid" => [0.32, 0.64, 0.0, 0.0],
            "DarkOrchid" => [0.40, 0.80, 0.20, 0.0],
            "Purple" => [0.45, 0.86, 0.0, 0.0],
            "Plum" => [0.50, 1.0, 0.0, 0.0],
            "Violet" => [0.79, 0.88, 0.0, 0.0],
            "RoyalPurple" => [0.75, 0.90, 0.0, 0.0],
            "BlueViolet" => [0.86, 0.91, 0.0, 0.04],
            "Periwinkle" => [0.57, 0.55, 0.0, 0.0],
            "CadetBlue" => [0.62, 0.57, 0.23, 0.0],
            "CornflowerBlue" => [0.65, 0.13, 0.0, 0.0],
            "MidnightBlue" => [0.98, 0.13, 0.0, 0.43],
            "NavyBlue" => [0.94, 0.54, 0.0, 0.0],
            "RoyalBlue" => [1.0, 0.50, 0.0, 0.0],
            "Blue" => [1.0, 1.0, 0.0, 0.0],
            "Cerulean" => [0.94, 0.11, 0.0, 0.0],
            "Cyan" => [1.0, 0.0, 0.0, 0.0],
            "ProcessBlue" => [0.96, 0.0, 0.0, 0.0],
            "SkyBlue" => [0.62, 0.0, 0.12, 0.0],
            "Turquoise" => [0.85, 0.0, 0.20, 0.0],
            "TealBlue" => [0.86, 0.0, 0.34, 0.02],
            "Aquamarine" => [0.82, 0.0, 0.30, 0.0],
            "BlueGreen" => [0.85, 0.0, 0.33, 0.0],
            "Emerald" => [1.0, 0.0, 0.50, 0.0],
            "JungleGreen" => [0.99, 0.0, 0.52, 0.0],
            "SeaGreen" => [0.69, 0.0, 0.50, 0.0],
            "Green" => [1.0, 0.0, 1.0, 0.0],
            "ForestGreen" => [0.91, 0.0, 0.88, 0.12],
            "PineGreen" => [0.92, 0.0, 0.59, 0.25],
            "LimeGreen" => [0.50, 0.0, 1.0, 0.0],
            "YellowGreen" => [0.44, 0.0, 0.74, 0.0],
            "SpringGreen" => [0.26, 0.0, 0.76, 0.0],
            "OliveGreen" => [0.64, 0.0, 0.95, 0.40],
            "RawSienna" => [0.0, 0.72, 1.0, 0.45],
            "Sepia" => [0.0, 0.83, 1.0, 0.70],
            "Brown" => [0.0, 0.81, 1.0, 0.60],
            "Tan" => [0.14, 0.42, 0.56, 0.0],
            "Gray" => [0.0, 0.0, 0.0, 0.50],
            "Black" => [0.0, 0.0, 0.0, 1.0],
            "White" => [0.0, 0.0, 0.0, 0.0],
            _ => return None,
        };
        Some(Self::cmyk(cyan, magenta, yellow, black))
    }
}

fn canonical(value: f32) -> f32 {
    normalize_zero(value.clamp(0.0, 1.0))
}
