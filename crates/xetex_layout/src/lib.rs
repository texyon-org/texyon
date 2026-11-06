// Copyright 2020-2021 the Tectonic Project
// Licensed under the MIT License.

//! XeTeX's font loading and layout code.
//!
//! [Tectonic]: https://tectonic-typesetting.github.io/

#[cfg(not(target_os = "macos"))]
use texyon_bridge_fontconfig as fc;

pub mod engine;
pub mod font;
pub mod manager;
mod utils;

pub use engine::{EngineOptions, GraphiteBreaks, LayoutEngine};
pub use font::Font;
pub use manager::{Engine, FontManager};

/// A point in TeX points.
#[derive(Copy, Clone, Default, PartialEq, Debug)]
pub struct FloatPoint {
    /// Horizontal coordinate.
    pub x: f32,
    /// Vertical coordinate.
    pub y: f32,
}

/// A glyph bounding box in TeX points.
#[derive(Copy, Clone, Default, PartialEq, Debug)]
pub struct GlyphBBox {
    /// Left edge.
    pub x_min: f32,
    /// Bottom edge.
    pub y_min: f32,
    /// Right edge.
    pub x_max: f32,
    /// Top edge.
    pub y_max: f32,
}

/// A 16.16 fixed-point number.
pub type Fixed = i32;
/// An OpenType tag.
pub type OTTag = u32;
/// A glyph index.
pub type GlyphID = u16;

/// A font reference from the platform's font manager.
#[cfg(not(target_os = "macos"))]
pub type PlatformFontRef = fc::Pattern;
/// A font reference from the platform's font manager.
#[cfg(target_os = "macos")]
pub type PlatformFontRef = texyon_mac_core::CTFontDescriptor;

/// Does our resulting executable link correctly?
#[test]
fn linkage() {}
