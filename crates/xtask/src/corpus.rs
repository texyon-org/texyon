//! The documents sessions run on.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;

use crate::{pack::Seed, project::Project, root};

/// Whole projects in crates/xtask/corpus that profiles are trained on.
const TRAINING_PROJECTS: [&str; 3] = ["article", "beamer", "pgfplots"];

/// The documents profile-guided builds are trained on: the training projects,
/// and the engine's tier1 corpus one file at a time.
pub fn training() -> Result<Vec<Project>> {
    let mut projects = TRAINING_PROJECTS
        .iter()
        .map(|name| Project::load(&dir().join(name), "main.tex"))
        .collect::<Result<Vec<_>>>()?;
    for path in tier1()? {
        projects.push(Project::single(&path)?);
    }
    Ok(projects)
}

/// The training documents as pack seeds, so a training pack holds what they read.
pub fn training_seeds() -> Result<Vec<Seed>> {
    Ok(training()?.into_iter().map(Seed::from).collect())
}

/// The documents `bench` and `differential` run when none are named.
pub fn defaults() -> Vec<PathBuf> {
    ["report", "nested"]
        .iter()
        .map(|name| dir().join(name))
        .collect()
}

/// A project directory around its `main.tex`, or one `.tex` file.
pub fn open(path: &Path) -> Result<Project> {
    if path.is_dir() {
        Project::load(path, "main.tex")
    } else {
        Project::single(path)
    }
}

fn dir() -> PathBuf {
    root().join("crates/xtask/corpus")
}

fn tier1() -> Result<Vec<PathBuf>> {
    let dir = root().join("crates/engine_xetex/tests/corpus/tier1");
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "tex") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}
