use std::{
    cell::{RefCell, RefMut},
    fmt, fs,
    io::Read,
    path::{Path, PathBuf},
    rc::Rc,
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

/// Exact-byte renderer resource lookup over document root then support bundle.
#[derive(Clone, Debug)]
pub struct BundleResourceProvider {
    bundle: FrozenBundle,
}

impl BundleResourceProvider {
    /// Construct a provider and canonicalize its optional document root.
    pub fn new(bundle: FrozenBundle, document_root: Option<PathBuf>) -> Result<Self, RenderError> {
        if let Some(root) = document_root {
            bundle.set_document_root(&root)?;
        }
        Ok(Self { bundle })
    }

    fn document_bytes(&self, name: &str) -> Result<Option<Vec<u8>>, RenderError> {
        let Some(root) = self.bundle.document_root() else {
            return Ok(None);
        };
        let candidate = root.join(name);
        let canonical = match fs::canonicalize(&candidate) {
            Ok(path) => path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(RenderError::Resource(error.to_string())),
        };
        if !canonical.starts_with(&root) || !canonical.is_file() {
            return Ok(None);
        }
        fs::read(canonical)
            .map(Some)
            .map_err(|error| RenderError::Resource(error.to_string()))
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
            if let Some(bytes) = self.document_bytes(&candidate)? {
                return Ok(Some(Resource::from_bytes(bytes)));
            }
            if let Some(bytes) = self.bundle_bytes(&candidate)? {
                return Ok(Some(Resource::from_bytes(bytes)));
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
        crate::test_util::activate_test_mode_augmented(env!("CARGO_MANIFEST_DIR"));
        let bundle = FrozenBundle::new(Box::new(crate::test_util::TestBundle::default()));
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
            include_bytes!("../../tests/assets/cmr10.tfm")
        );
    }
}
