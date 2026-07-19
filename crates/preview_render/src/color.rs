//! Canonical RGB colors passed to drawing backends.

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
}

fn canonical(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    if value == 0.0 {
        0.0
    } else {
        value
    }
}
