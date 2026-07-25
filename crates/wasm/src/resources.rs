//! Support files, as the renderer wants them.
//!
//! The engine and the renderer ask for the same files by different names: TeX
//! asks for `cmr10.tfm`, the renderer asks for a `Tfm` called `cmr10`. This maps
//! one onto the other and shares a single byte cache with [`HostIo`], so a font
//! the engine has already pulled over the network is not fetched again to draw
//! it.

use std::{cell::RefCell, collections::HashMap, path::Path};

use tectonic_preview_render::{
    RenderError, Resource, ResourceKind, ResourceProvider, ResourceRequest,
};

use crate::io::HostIo;

/// A [`ResourceProvider`] backed by the host's support files.
pub struct HostResources {
    support: HostIo,
    /// Resolved resources, by the name that resolved them.
    ///
    /// Not redundant with `HostIo`'s byte cache. A [`Resource`] carries the
    /// SHA-256 of its bytes, which is how the renderer decides whether its
    /// parsed artifacts are still valid — so building one means copying and
    /// hashing the whole file. `pdftex.map` is 4.4 MB and is resolved on every
    /// analysis; doing that work per keystroke cost more than everything else
    /// in the pipeline combined.
    resolved: RefCell<HashMap<String, Option<Resource>>>,
}

impl HostResources {
    /// Draw resources from the same cache the engine reads through.
    pub fn new(support: HostIo) -> Self {
        Self {
            support,
            resolved: RefCell::new(HashMap::new()),
        }
    }
}

impl ResourceProvider for HostResources {
    fn resolve(&self, request: &ResourceRequest) -> Result<Option<Resource>, RenderError> {
        for candidate in candidates(request) {
            if let Some(hit) = self.resolved.borrow().get(&candidate) {
                match hit {
                    Some(resource) => return Ok(Some(resource.clone())),
                    // A name already known to be absent: try the next spelling.
                    None => continue,
                }
            }

            let resource = match self.support.read(&candidate) {
                Ok(Some(bytes)) => Some(Resource::from_bytes(bytes.to_vec())),
                Ok(None) => None,
                Err(error) => return Err(RenderError::Resource(error)),
            };

            self.resolved
                .borrow_mut()
                .insert(candidate, resource.clone());

            if let Some(resource) = resource {
                return Ok(Some(resource));
            }
        }

        Ok(None)
    }
}

/// The file names one resource request could mean, in preference order.
///
/// Mirrors the native provider in `src/live_preview/resources.rs`: the renderer
/// names fonts the way a font map does, without an extension, and which
/// extension applies depends on the kind.
fn candidates(request: &ResourceRequest) -> Vec<String> {
    let mut candidates = vec![request.name.clone()];

    if Path::new(&request.name).extension().is_some() {
        return candidates;
    }

    match request.kind {
        ResourceKind::Tfm => candidates.push(format!("{}.tfm", request.name)),
        ResourceKind::VirtualFont => candidates.push(format!("{}.vf", request.name)),
        ResourceKind::Encoding => candidates.push(format!("{}.enc", request.name)),
        ResourceKind::FontProgram => {
            for extension in ["pfb", "pfa", "otf", "ttf"] {
                candidates.push(format!("{}.{}", request.name, extension));
            }
        }
        ResourceKind::FontMap | ResourceKind::Image | ResourceKind::Pdf => {}
    }

    candidates
}
