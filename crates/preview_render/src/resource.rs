//! Immutable resource identities at the portable-core boundary.

use std::sync::Arc;

use sha2::{Digest, Sha256};

use crate::RenderError;

/// Kind of resource requested by the XDV interpreter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResourceKind {
    /// TeX font metrics.
    Tfm,
    /// A virtual font program.
    VirtualFont,
    /// A font map.
    FontMap,
    /// A font encoding vector.
    Encoding,
    /// Type1, OpenType, or TrueType font bytes.
    FontProgram,
    /// Encoded raster image bytes.
    Image,
    /// Embedded PDF bytes.
    Pdf,
}

/// One normalized resource lookup.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceRequest {
    /// Requested resource kind.
    pub kind: ResourceKind,
    /// Engine-visible resource name.
    pub name: String,
}

/// SHA-256 identity of the exact bytes supplied to analysis and rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceIdentity(pub [u8; 32]);

/// Immutable resource bytes paired with their strong identity.
#[derive(Clone, Debug)]
pub struct Resource {
    /// Exact bytes consumed by the interpreter or backend.
    pub bytes: Arc<[u8]>,
    /// Digest of `bytes`.
    pub identity: ResourceIdentity,
}

impl Resource {
    /// Freeze bytes and compute their correctness identity.
    pub fn from_bytes(bytes: impl Into<Arc<[u8]>>) -> Self {
        let bytes = bytes.into();
        let identity = ResourceIdentity(Sha256::digest(&bytes).into());
        Self { bytes, identity }
    }
}

/// Supplies resource bytes without exposing paths or filesystem behavior.
pub trait ResourceProvider {
    /// Resolve `request`, returning `None` when it is unavailable.
    fn resolve(&self, request: &ResourceRequest) -> Result<Option<Resource>, RenderError>;
}

impl ResourceProvider for () {
    fn resolve(&self, _request: &ResourceRequest) -> Result<Option<Resource>, RenderError> {
        Ok(None)
    }
}
