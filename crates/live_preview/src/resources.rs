use std::{
    cell::{RefCell, RefMut},
    collections::HashMap,
    fmt, fs,
    io::Read,
    path::{Path, PathBuf},
    rc::Rc,
    time::SystemTime,
};

use texyon_bundles::Bundle;
use texyon_io_base::{InputHandle, IoProvider, OpenResult, OutputHandle};
use texyon_preview_render::{
    RenderError, Resource, ResourceKind, ResourceProvider, ResourceRequest,
};
use texyon_status_base::{NoopStatusBackend, StatusBackend};

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

/// Serves the bundle to an engine, borrowing it for each request only.
impl IoProvider for FrozenBundle {
    fn output_open_name(&mut self, name: &str) -> OpenResult<OutputHandle> {
        self.with_bundle(|bundle| bundle.output_open_name(name))
    }

    fn output_open_stdout(&mut self) -> OpenResult<OutputHandle> {
        self.with_bundle(|bundle| bundle.output_open_stdout())
    }

    fn input_open_name(
        &mut self,
        name: &str,
        status: &mut dyn StatusBackend,
    ) -> OpenResult<InputHandle> {
        self.with_bundle(|bundle| bundle.input_open_name(name, status))
    }

    fn input_open_name_with_abspath(
        &mut self,
        name: &str,
        status: &mut dyn StatusBackend,
    ) -> OpenResult<(InputHandle, Option<PathBuf>)> {
        self.with_bundle(|bundle| bundle.input_open_name_with_abspath(name, status))
    }

    fn input_open_primary(&mut self, status: &mut dyn StatusBackend) -> OpenResult<InputHandle> {
        self.with_bundle(|bundle| bundle.input_open_primary(status))
    }

    fn input_open_primary_with_abspath(
        &mut self,
        status: &mut dyn StatusBackend,
    ) -> OpenResult<(InputHandle, Option<PathBuf>)> {
        self.with_bundle(|bundle| bundle.input_open_primary_with_abspath(status))
    }

    fn input_casefold_name(&mut self, name: &str) -> Option<String> {
        self.bundle.try_borrow_mut().ok()?.input_casefold_name(name)
    }

    fn directory_exists(&mut self, dir: &str) -> bool {
        self.bundle
            .try_borrow_mut()
            .is_ok_and(|mut bundle| bundle.directory_exists(dir))
    }

    fn input_open_format(
        &mut self,
        name: &str,
        status: &mut dyn StatusBackend,
    ) -> OpenResult<InputHandle> {
        self.with_bundle(|bundle| bundle.input_open_format(name, status))
    }

    fn write_format(
        &mut self,
        name: &str,
        data: &[u8],
        status: &mut dyn StatusBackend,
    ) -> texyon_errors::Result<()> {
        let mut bundle = self.try_borrow_mut()?;
        bundle.write_format(name, data, status)
    }
}

impl FrozenBundle {
    fn with_bundle<T>(
        &self,
        f: impl FnOnce(&mut Box<dyn Bundle>) -> OpenResult<T>,
    ) -> OpenResult<T> {
        match self.try_borrow_mut() {
            Ok(mut bundle) => f(&mut bundle),
            Err(error) => OpenResult::Err(error.into()),
        }
    }
}

impl fmt::Debug for FrozenBundle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FrozenBundle").finish_non_exhaustive()
    }
}

/// Modification time and length of a document-root file.
type DiskStamp = (SystemTime, u64);

/// Memoized resolution of one candidate name.
///
/// Bundle answers hold while no document-root file shadows them. Document-root
/// bytes hold while their [`DiskStamp`] is unchanged.
#[derive(Debug)]
enum CachedCandidate {
    Document {
        stamp: DiskStamp,
        resource: Resource,
    },
    Bundle(Option<Resource>),
}

/// Exact-byte renderer resource lookup over document root then support bundle.
///
/// Memoized because analysis re-resolves every font on every keystroke.
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

    /// Canonical path and stamp of a regular file on disk: an absolute path, as
    /// the engine reads installed fonts, or one confined to the document root.
    fn document_stamp(&self, name: &str) -> Result<Option<(PathBuf, DiskStamp)>, RenderError> {
        let absolute = Path::new(name).is_absolute();
        let root = self.bundle.document_root();
        let candidate = match &root {
            _ if absolute => PathBuf::from(name),
            Some(root) => root.join(name),
            None => return Ok(None),
        };
        // Most candidates are absent, so stat before paying for canonicalization.
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
        if !absolute && root.is_some_and(|root| !canonical.starts_with(root)) {
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
            // The stamp decides both which source wins and whether the cache is current.
            let stamp = self.document_stamp(&candidate)?;
            if let Some(cached) = self.cache.borrow().get(&candidate) {
                match (cached, &stamp) {
                    (
                        CachedCandidate::Document {
                            stamp: seen,
                            resource,
                        },
                        Some((_, now)),
                    ) if seen == now => {
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
    let extensions: &[&str] = match request.kind {
        ResourceKind::Tfm => &["tfm"],
        ResourceKind::VirtualFont => &["vf"],
        ResourceKind::Encoding => &["enc"],
        ResourceKind::FontProgram => &["pfb", "pfa", "otf", "ttf"],
        ResourceKind::FontMap | ResourceKind::Image | ResourceKind::Pdf => &[],
    };
    let name = &request.name;
    let mut candidates = vec![name.clone()];
    if Path::new(name).extension().is_none() {
        candidates.extend(
            extensions
                .iter()
                .map(|extension| format!("{name}.{extension}")),
        );
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

    #[test]
    fn installed_fonts_resolve_by_absolute_path_but_relative_names_stay_in_the_root() {
        let root = tempfile::tempdir().unwrap();
        let installed = tempfile::tempdir().unwrap();
        let font = installed.path().join("installed.otf");
        fs::write(&font, b"installed font").unwrap();
        let bundle = FrozenBundle::new(Box::new(crate::test_support::AssetBundle::default()));
        let provider = BundleResourceProvider::new(bundle, Some(root.path().into())).unwrap();
        let request = |name: String| ResourceRequest {
            kind: ResourceKind::FontProgram,
            name,
        };

        let resource = provider
            .resolve(&request(font.to_str().unwrap().into()))
            .unwrap()
            .unwrap();
        assert_eq!(resource.bytes.as_ref(), b"installed font");

        let escape = format!(
            "../{}/installed.otf",
            installed.path().file_name().unwrap().to_str().unwrap()
        );
        assert!(provider.resolve(&request(escape)).unwrap().is_none());
    }
}
