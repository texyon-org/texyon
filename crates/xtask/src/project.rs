//! A project's sources as an editor holds them: each file's text and revision.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use texyon_preview_protocol::session::SessionRequest;

/// Extensions an editor holds as text. Everything else is read from the project directory.
const TEXT: [&str; 7] = ["tex", "ltx", "sty", "cls", "bib", "def", "cfg"];

/// Sources and the revisions a session has seen.
#[derive(Clone)]
pub struct Project {
    /// The directory a session opens with, for files that are not sources.
    pub root: PathBuf,
    /// The file TeX starts from, relative to `root`.
    pub primary: String,
    files: BTreeMap<String, Source>,
    next_revision: u64,
}

#[derive(Clone)]
struct Source {
    /// None for a file that is not UTF-8, which the host reads from `root`.
    text: Option<String>,
    revision: u64,
    sent: bool,
}

impl Project {
    /// A project with no sources yet.
    pub fn new(root: &Path, primary: &str) -> Self {
        Self {
            root: root.to_owned(),
            primary: primary.to_owned(),
            files: BTreeMap::new(),
            next_revision: 1,
        }
    }

    /// Every text source under `root`.
    pub fn load(root: &Path, primary: &str) -> Result<Self> {
        let mut project = Self::new(root, primary);
        let mut pending = vec![root.to_owned()];
        while let Some(dir) = pending.pop() {
            for entry in
                fs::read_dir(&dir).with_context(|| format!("could not read {}", dir.display()))?
            {
                let path = entry?.path();
                if path.is_dir() {
                    pending.push(path);
                } else if path
                    .extension()
                    .is_some_and(|ext| TEXT.iter().any(|text| ext == *text))
                {
                    let relative = path
                        .strip_prefix(root)?
                        .to_string_lossy()
                        .replace('\\', "/");
                    match fs::read_to_string(&path) {
                        Ok(text) => project.set(&relative, text),
                        Err(_) => project.insert(&relative, None),
                    }
                }
            }
        }
        Ok(project)
    }

    /// One file as the whole project, the others in its directory left on disk.
    pub fn single(path: &Path) -> Result<Self> {
        let root = path.parent().context("a source file has a directory")?;
        let name = path
            .file_name()
            .context("a source file has a name")?
            .to_string_lossy();
        let mut project = Self::new(root, &name);
        let bytes = fs::read(path).with_context(|| format!("could not read {}", path.display()))?;
        match String::from_utf8(bytes) {
            Ok(text) => project.set(&name, text),
            Err(_) => project.insert(&name, None),
        }
        Ok(project)
    }

    /// Replace a file's text, which gives it a new revision when it changed.
    pub fn set(&mut self, path: &str, text: impl Into<String>) {
        let text = text.into();
        if self.files.get(path).and_then(|held| held.text.as_ref()) != Some(&text) {
            self.insert(path, Some(text));
        }
    }

    fn insert(&mut self, path: &str, text: Option<String>) {
        let revision = self.next_revision;
        self.next_revision += 1;
        self.files.insert(
            path.to_owned(),
            Source {
                text,
                revision,
                sent: false,
            },
        );
    }

    /// The same sources for a session that has seen none of them.
    pub fn for_new_session(&self) -> Self {
        let mut project = self.clone();
        for source in project.files.values_mut() {
            source.sent = false;
        }
        project
    }

    /// Drop every file not in `keep`.
    pub fn retain(&mut self, keep: &[&str]) {
        self.files.retain(|path, _| keep.contains(&path.as_str()));
    }

    /// The sources held as text, by path.
    pub fn texts(&self) -> impl Iterator<Item = (&str, &str)> {
        self.files
            .iter()
            .filter_map(|(path, source)| Some((path.as_str(), source.text.as_deref()?)))
    }

    /// A compile of the current revisions, carrying the texts the session has not seen.
    pub fn compile(
        &mut self,
        viewport: Option<(usize, usize)>,
        scale: Option<f32>,
    ) -> SessionRequest {
        let files = self
            .files
            .iter()
            .map(|(path, source)| (path.clone(), source.revision))
            .collect();
        let texts = self
            .files
            .iter_mut()
            .filter(|(_, source)| !source.sent)
            .filter_map(|(path, source)| {
                source.sent = true;
                Some((path.clone(), source.text.clone()?))
            })
            .collect();
        SessionRequest::Compile {
            files,
            edits: BTreeMap::new(),
            texts,
            viewport,
            scale,
        }
    }
}
