//! Error type shared by analysis, resources and backends.

use std::fmt;

use texyon_xdv::XdvError;

/// Failure from parsing, resource resolution, or a rendering backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderError {
    /// Malformed or unsupported XDV structure.
    Xdv(String),
    /// Resource-provider failure.
    Resource(String),
    /// Backend failure.
    Backend(String),
    /// Invalid selected-page request.
    InvalidSelection(String),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Xdv(message) => write!(f, "XDV analysis failed: {message}"),
            Self::Resource(message) => write!(f, "resource resolution failed: {message}"),
            Self::Backend(message) => write!(f, "render backend failed: {message}"),
            Self::InvalidSelection(message) => write!(f, "invalid page selection: {message}"),
        }
    }
}

impl std::error::Error for RenderError {}

impl From<XdvError> for RenderError {
    fn from(value: XdvError) -> Self {
        Self::Xdv(value.to_string())
    }
}

impl From<std::io::Error> for RenderError {
    fn from(value: std::io::Error) -> Self {
        Self::Xdv(value.to_string())
    }
}
