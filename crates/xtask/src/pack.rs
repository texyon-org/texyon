//! The support pack: the format dump and every support file the seed
//! documents read, so a session starts without waiting on the network.

use std::{
    collections::{BTreeMap, btree_map::Entry},
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};
use flate2::{Compression, GzBuilder};
use texyon_preview_protocol::pack::{MANIFEST, PackManifest};

use crate::{cargo, project::Project, root};

/// The bundle a session reads at runtime. Seeding from another one bakes an
/// older LaTeX kernel into the format, and later fetches then fail.
pub const BUNDLE_URL: &str = "https://bundles.texyon.org/texlive2026.ttb";
/// The pack as one file, for the browser.
pub const ARCHIVE: &str = "texyon-pack.tar.gz";
/// Static hosts such as Cloudflare Pages refuse larger files.
const MAX_ARCHIVE_BYTES: u64 = 25 * 1024 * 1024;
/// Every pack has these. A missing one means a seed compile failed, likely on the network.
const REQUIRED: [&str; 4] = ["article.cls", "amsmath.sty", "cmr5.pfb", "cmr10.tfm"];

/// A document the pack is seeded from.
pub struct Seed {
    /// Its directory, which is also where TeX runs.
    pub dir: PathBuf,
    /// The file to compile, relative to `dir`.
    pub primary: String,
}

impl From<Project> for Seed {
    fn from(project: Project) -> Self {
        Self {
            dir: project.root,
            primary: project.primary,
        }
    }
}

/// The standard seeds, which cover what a general-purpose editor reaches offline.
pub fn standard_seeds() -> Result<Vec<Seed>> {
    let dir = root().join("crates/xtask/seeds");
    let mut seeds = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        if name.ends_with(".tex") {
            seeds.push(Seed {
                dir: dir.clone(),
                primary: name,
            });
        }
    }
    seeds.sort_by(|a, b| a.primary.cmp(&b.primary));
    Ok(seeds)
}

/// Whether a pack gets the single-file archive the browser downloads.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Archive {
    /// For a release.
    Write,
    /// For a session host, which reads `files/`.
    Skip,
}

/// Compile `seeds` with the native binary `texyon` and pack every support file they read into `out`.
///
/// An instrumented `texyon` writes its counts into `profile`.
pub fn build(
    texyon: &Path,
    profile: Option<&Path>,
    seeds: &[Seed],
    out: &Path,
    archive: Archive,
) -> Result<()> {
    let work = cargo::target_dir().join("pack-work");
    if work.exists() {
        fs::remove_dir_all(&work)?;
    }
    let cache = work.join("cache");
    let output = work.join("output");
    fs::create_dir_all(&output)?;

    for seed in seeds {
        println!("seeding from {}", seed.dir.join(&seed.primary).display());
        let mut compile = Command::new(texyon);
        compile
            .args([
                "-X",
                "compile",
                &seed.primary,
                "--bundle",
                BUNDLE_URL,
                "--outfmt",
                "pdf",
            ])
            .arg("-o")
            .arg(&output)
            .current_dir(&seed.dir)
            .env("TEXYON_CACHE_DIR", &cache);
        if let Some(dir) = profile {
            compile.env("LLVM_PROFILE_FILE", dir.join("%m_%p.profraw"));
        }
        // TeX errors are fine here, what matters is which files a seed read.
        let _ = compile
            .output()
            .with_context(|| format!("could not run {}", texyon.display()))?;
    }

    let format = only_entry(&cache.join("formats"), "a format dump")?;
    let bundle_files = only_entry(&cache.join("bundles/data"), "cached bundle files")?;
    let manifest = write(out, &format, &bundle_files)?;
    if archive == Archive::Write {
        write_archive(out, &manifest)?;
    }
    Ok(())
}

fn only_entry(dir: &Path, what: &str) -> Result<PathBuf> {
    let entries = fs::read_dir(dir)
        .with_context(|| format!("no {what} in {}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_none_or(|ext| ext != "index"))
        .collect::<Vec<_>>();
    match entries.as_slice() {
        [one] => Ok(one.clone()),
        _ => bail!(
            "expected one {what} in {}, found {}",
            dir.display(),
            entries.len()
        ),
    }
}

/// Lay out `files/` and `manifest.json` in `out`.
fn write(out: &Path, format: &Path, bundle_files: &Path) -> Result<PackManifest> {
    // The cached format is named <bundle digest>-latex-<serial>.fmt.
    let stem = format.file_stem().unwrap_or_default().to_string_lossy();
    let (Some((digest, _)), Some((_, serial))) = (stem.split_once('-'), stem.rsplit_once('-'))
    else {
        bail!("unexpected format dump name {stem}");
    };

    let files_dir = out.join("files");
    if files_dir.exists() {
        fs::remove_dir_all(&files_dir)?;
    }
    fs::create_dir_all(&files_dir)?;

    // The engine asks for bare names, so the TeX tree flattens to basenames.
    let mut sources = BTreeMap::new();
    sources.insert("latex.fmt".to_owned(), format.to_owned());
    for path in files_under(bundle_files)? {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        match sources.entry(name) {
            Entry::Vacant(slot) => {
                slot.insert(path);
            }
            Entry::Occupied(kept) => {
                if fs::read(kept.get())? != fs::read(&path)? {
                    println!(
                        "warning: two different files are named {}, keeping {}",
                        kept.key(),
                        kept.get().display()
                    );
                }
            }
        }
    }

    let mut files = BTreeMap::new();
    for (name, source) in &sources {
        files.insert(name.clone(), fs::copy(source, files_dir.join(name))?);
    }
    if let Some(missing) = REQUIRED.iter().find(|name| !files.contains_key(**name)) {
        bail!("the pack is missing {missing}, a seed compile probably failed");
    }

    let manifest = PackManifest {
        version: 1,
        format: "latex.fmt".into(),
        bundle_digest: digest.into(),
        format_serial: serial
            .parse()
            .with_context(|| format!("unexpected format serial in {stem}"))?,
        total_bytes: files.values().sum(),
        files,
    };
    let mut text = serde_json::to_string_pretty(&manifest)?;
    text.push('\n');
    fs::write(out.join(MANIFEST), text)?;
    println!(
        "{} files, {:.1} MB",
        manifest.files.len(),
        mb(manifest.total_bytes)
    );

    Ok(manifest)
}

/// One deterministic `.tar.gz`, so an unchanged pack stays byte-identical in caches.
fn write_archive(out: &Path, manifest: &PackManifest) -> Result<()> {
    let path = out.join(ARCHIVE);
    let file = fs::File::create(&path)?;
    let gzip = GzBuilder::new().mtime(0).write(file, Compression::best());
    let mut archive = tar::Builder::new(gzip);
    let entries = std::iter::once((MANIFEST.to_owned(), out.join(MANIFEST))).chain(
        manifest
            .files
            .keys()
            .map(|name| (name.clone(), out.join("files").join(name))),
    );
    for (name, source) in entries {
        let bytes = fs::read(&source)?;
        let mut header = tar::Header::new_ustar();
        header.set_size(bytes.len().try_into()?);
        header.set_mode(0o644);
        header.set_mtime(0);
        archive.append_data(&mut header, &name, bytes.as_slice())?;
    }
    archive.into_inner()?.finish()?.flush()?;

    let size = fs::metadata(&path)?.len();
    if size > MAX_ARCHIVE_BYTES {
        bail!(
            "{ARCHIVE} is {:.1} MB, over the 25 MiB static hosting limit",
            mb(size)
        );
    }
    println!("{}: {:.1} MB", path.display(), mb(size));
    Ok(())
}

fn files_under(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_owned()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                pending.push(entry.path());
            } else {
                files.push(entry.path());
            }
        }
    }
    files.sort();
    Ok(files)
}

fn mb(bytes: u64) -> f64 {
    bytes as f64 / 1e6
}
