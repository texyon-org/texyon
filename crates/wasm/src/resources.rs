//! Support files for the renderer, named the way it asks for them.

use std::{cell::RefCell, collections::HashMap, path::Path};

use tectonic_preview_render::{
    RenderError, Resource, ResourceKind, ResourceProvider, ResourceRequest,
};

use crate::io::HostIo;

/// A [`ResourceProvider`] sharing [`HostIo`]'s byte cache.
pub struct HostResources {
    support: HostIo,
    // Building a Resource copies and hashes the file, too slow to repeat per
    // keystroke for the 4.4 MB pdftex.map.
    resolved: RefCell<HashMap<String, Option<Resource>>>,
}

impl HostResources {
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
                    None => continue,
                }
            }

            let resource = self
                .support
                .read(&candidate)
                .map_err(RenderError::Resource)?
                .map(|bytes| Resource::from_bytes(bytes.to_vec()));

            self.resolved
                .borrow_mut()
                .insert(candidate, resource.clone());

            if resource.is_some() {
                return Ok(resource);
            }
        }

        Ok(None)
    }
}

/// File names a request could mean, in preference order.
///
/// Mirrors the native provider in `src/live_preview/resources.rs`.
fn candidates(request: &ResourceRequest) -> Vec<String> {
    let name = &request.name;
    let mut candidates = vec![name.clone()];

    if Path::new(name).extension().is_some() {
        return candidates;
    }

    let extensions: &[&str] = match request.kind {
        ResourceKind::Tfm => &["tfm"],
        ResourceKind::VirtualFont => &["vf"],
        ResourceKind::Encoding => &["enc"],
        ResourceKind::FontProgram => &["pfb", "pfa", "otf", "ttf"],
        ResourceKind::FontMap | ResourceKind::Image | ResourceKind::Pdf => &[],
    };
    candidates.extend(extensions.iter().map(|ext| format!("{name}.{ext}")));

    candidates
}
