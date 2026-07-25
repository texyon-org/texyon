use std::{
    cell::{RefCell, RefMut},
    collections::HashMap,
    fmt, fs,
    io::Read,
    path::{Path, PathBuf},
    rc::Rc,
    time::SystemTime,
};

use tectonic_bundles::Bundle;
use tectonic_io_base::{IoProvider, OpenResult};
use tectonic_preview_render::{
    RenderError, Resource, ResourceKind, ResourceProvider, ResourceRequest,
};
use tectonic_status_base::NoopStatusBackend;

/// Worker-local shared ownership of one support bundle.
#[derive(Clone)]
pub struct FrozenBundle {
    bundle: Rc<RefCell<Box<dyn Bundle>>>,
    document_root: Rc<RefCell<Option<PathBuf>>>,
}

impl FrozenBundle {
    /// Freeze ownership of a fully bootstrapped bundle.
    pub fn new(bundle: Box<dyn Bundle>) -> Self {
        Self {
            bundle: Rc::new(RefCell::new(bundle)),
            document_root: Rc::new(RefCell::new(None)),
        }
    }

    pub(crate) fn try_borrow_mut(&self) -> Result<RefMut<'_, Box<dyn Bundle>>, RenderError> {
        self.bundle
            .try_borrow_mut()
            .map_err(|_| RenderError::Resource("support bundle is already borrowed".into()))
    }

    pub(crate) fn borrow_mut_io(&self) -> RefMut<'_, Box<dyn Bundle>> {
        self.bundle.borrow_mut()
    }

    /// Bind the canonical document root once initialization establishes it.
    pub fn set_document_root(&self, root: &Path) -> Result<(), RenderError> {
        let canonical =
            fs::canonicalize(root).map_err(|error| RenderError::Resource(error.to_string()))?;
        let mut current = self
            .document_root
            .try_borrow_mut()
            .map_err(|_| RenderError::Resource("document root is already borrowed".into()))?;
        match current.as_ref() {
            Some(existing) if existing != &canonical => Err(RenderError::Resource(format!(
                "document root changed from {} to {}",
                existing.display(),
                canonical.display()
            ))),
            Some(_) => Ok(()),
            None => {
                *current = Some(canonical);
                Ok(())
            }
        }
    }

    pub(crate) fn document_root(&self) -> Option<PathBuf> {
        self.document_root.borrow().clone()
    }
}

impl fmt::Debug for FrozenBundle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FrozenBundle").finish_non_exhaustive()
    }
}

/// Disk identity of one document-root resource, used to reuse frozen bytes
/// only while the file behind them is unchanged.
type DiskStamp = (SystemTime, u64);

/// Memoized resolution of one candidate name.
///
/// Support-bundle bytes are immutable for the session, so a bundle hit or miss
/// is reusable forever. Document-root bytes are editable on disk, so they are
/// reusable only while the file's modification time and length are unchanged,
/// and a bundle answer is reusable only while no document-root file shadows it.
#[derive(Debug)]
enum CachedCandidate {
    Document { stamp: DiskStamp, resource: Resource },
    Bundle(Option<Resource>),
}

/// Exact-byte renderer resource lookup over document root then support bundle.
///
/// Resolution is memoized because analysis re-resolves every font of the whole
/// document on every keystroke, and each miss otherwise costs a bundle read
/// plus a SHA-256 over the complete resource bytes.
#[derive(Clone, Debug)]
pub struct BundleResourceProvider {
    bundle: FrozenBundle,
    cache: Rc<RefCell<HashMap<String, CachedCandidate>>>,
}

impl BundleResourceProvider {
    /// Construct a provider and canonicalize its optional document root.
    pub fn new(bundle: FrozenBundle, document_root: Option<PathBuf>) -> Result<Self, RenderError> {
        if let Some(root) = document_root {
            bundle.set_document_root(&root)?;
        }
        Ok(Self {
            bundle,
            cache: Rc::new(RefCell::new(HashMap::new())),
        })
    }

    /// Canonical path and disk stamp of a document-root candidate, if it is a
    /// regular file confined to the root.
    fn document_stamp(&self, name: &str) -> Result<Option<(PathBuf, DiskStamp)>, RenderError> {
        let Some(root) = self.bundle.document_root() else {
            return Ok(None);
        };
        let candidate = root.join(name);
        // Most candidate names never exist in the document root, so take the
        // cheapest possible negative before paying for canonicalization.
        let metadata = match fs::metadata(&candidate) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(RenderError::Resource(error.to_string())),
        };
        if !metadata.is_file() {
            return Ok(None);
        }
        let canonical = match fs::canonicalize(&candidate) {
            Ok(path) => path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(RenderError::Resource(error.to_string())),
        };
        if !canonical.starts_with(&root) {
            return Ok(None);
        }
        let modified = metadata
            .modified()
            .map_err(|error| RenderError::Resource(error.to_string()))?;
        Ok(Some((canonical, (modified, metadata.len()))))
    }

    fn bundle_bytes(&self, name: &str) -> Result<Option<Vec<u8>>, RenderError> {
        let mut bundle = self.bundle.try_borrow_mut()?;
        let mut status = NoopStatusBackend::default();
        let mut handle = match bundle.input_open_name(name, &mut status) {
            OpenResult::Ok(handle) => handle,
            OpenResult::NotAvailable => return Ok(None),
            OpenResult::Err(error) => return Err(RenderError::Resource(error.to_string())),
        };
        let mut bytes = Vec::new();
        handle
            .read_to_end(&mut bytes)
            .map_err(|error| RenderError::Resource(error.to_string()))?;
        Ok(Some(bytes))
    }
}

impl ResourceProvider for BundleResourceProvider {
    fn resolve(&self, request: &ResourceRequest) -> Result<Option<Resource>, RenderError> {
        for candidate in candidates(request) {
            // Stat the document root first on every call: it decides both which
            // source wins and whether a cached answer is still current.
            let stamp = self.document_stamp(&candidate)?;
            if let Some(cached) = self.cache.borrow().get(&candidate) {
                match (cached, &stamp) {
                    (CachedCandidate::Document { stamp: seen, resource }, Some((_, now)))
                        if seen == now =>
                    {
                        return Ok(Some(resource.clone()));
                    }
                    (CachedCandidate::Bundle(resource), None) => {
                        if let Some(resource) = resource {
                            return Ok(Some(resource.clone()));
                        }
                        continue;
                    }
                    _ => {}
                }
            }
            if let Some((path, stamp)) = stamp {
                let bytes =
                    fs::read(path).map_err(|error| RenderError::Resource(error.to_string()))?;
                let resource = Resource::from_bytes(bytes);
                self.cache.borrow_mut().insert(
                    candidate,
                    CachedCandidate::Document {
                        stamp,
                        resource: resource.clone(),
                    },
                );
                return Ok(Some(resource));
            }
            let resource = self.bundle_bytes(&candidate)?.map(Resource::from_bytes);
            self.cache
                .borrow_mut()
                .insert(candidate, CachedCandidate::Bundle(resource.clone()));
            if let Some(resource) = resource {
                return Ok(Some(resource));
            }
        }
        Ok(None)
    }
}

fn candidates(request: &ResourceRequest) -> Vec<String> {
    let extension = match request.kind {
        ResourceKind::Tfm => Some("tfm"),
        ResourceKind::VirtualFont => Some("vf"),
        ResourceKind::Encoding => Some("enc"),
        ResourceKind::FontMap | ResourceKind::Image | ResourceKind::Pdf => None,
        ResourceKind::FontProgram => None,
    };
    let mut candidates = vec![request.name.clone()];
    if let Some(extension) = extension {
        if Path::new(&request.name).extension().is_none() {
            candidates.push(format!("{}.{}", request.name, extension));
        }
    }
    if request.kind == ResourceKind::FontProgram && Path::new(&request.name).extension().is_none() {
        for extension in ["pfb", "pfa", "otf", "ttf"] {
            candidates.push(format!("{}.{}", request.name, extension));
        }
    }
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_candidates_are_kind_specific_and_stable() {
        assert_eq!(
            candidates(&ResourceRequest {
                kind: ResourceKind::Tfm,
                name: "cmr10".into(),
            }),
            vec!["cmr10", "cmr10.tfm"]
        );
        assert_eq!(
            candidates(&ResourceRequest {
                kind: ResourceKind::FontProgram,
                name: "font.otf".into(),
            }),
            vec!["font.otf"]
        );
    }

    #[test]
    fn bundle_provider_returns_exact_resource_bytes() {
        let bundle = FrozenBundle::new(Box::new(crate::test_support::AssetBundle::default()));
        let provider = BundleResourceProvider::new(bundle, None).unwrap();
        let resource = provider
            .resolve(&ResourceRequest {
                kind: ResourceKind::Tfm,
                name: "cmr10".into(),
            })
            .unwrap()
            .unwrap();

        assert_eq!(
            resource.bytes.as_ref(),
            include_bytes!("../../../tests/assets/cmr10.tfm")
        );
    }
}
