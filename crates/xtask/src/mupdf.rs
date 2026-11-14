//! MuPDF for the preview's renderer.
//!
//! It is built against TeXyoN's own FreeType, HarfBuzz and zlib, since two
//! copies of one library in a wasm module link silently under
//! `--allow-multiple-definition` and hand one copy's structs to the other's code.
//! MuPDF and pdf_io both define `pdf_*` symbols, so MuPDF's are renamed in the
//! preprocessor: llvm-objcopy cannot rename wasm symbols and macOS has no objcopy.

use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    thread,
};

use anyhow::{Context, Result, bail};

use crate::{
    deps::{self, WasiSdk},
    process, root,
};

/// The file of `#define`s that renames MuPDF's `pdf_*` symbols.
pub const RENAMES: &str = "texyon-pdf-rename.h";

/// What MuPDF is compiled for.
pub enum Target {
    /// The machine running the build.
    Native,
    /// The wasm module.
    Wasm(WasiSdk),
}

/// Fetch and build MuPDF for the native preview into `deps/mupdf`.
pub fn build_native(source_archive: Option<&Path>) -> Result<PathBuf> {
    let source = deps::dir().join("mupdf");
    deps::mupdf_archive().install(&source, source_archive)?;
    build(&source, &Target::Native)?;
    Ok(source)
}

/// Build MuPDF's static libraries in `source`, unless a renamed build is already there.
pub fn build(source: &Path, target: &Target) -> Result<()> {
    let library = source.join("build/release/libmupdf.a");
    let renames = source.join(RENAMES);
    // A restored cache can hold an archive whose renames were lost. Rebuilding
    // from it would find no pdf_* symbols and ship every collision.
    if library.exists() && renames.exists() && raw_pdf_symbols(&library, target)?.is_empty() {
        println!("MuPDF is already built in {}", source.display());
        return Ok(());
    }
    if let Target::Wasm(_) = target {
        allow_missing_mkstemp(source)?;
    }

    println!(
        "building MuPDF in {} to find its pdf_* symbols",
        source.display()
    );
    clean(source)?;
    process::run(&mut make(source, target, None)?)?;
    let symbols = raw_pdf_symbols(&library, target)?;
    if symbols.is_empty() {
        bail!("found no pdf_* symbols in {} to rename", library.display());
    }
    let header: String = symbols
        .iter()
        .map(|name| format!("#define {name} texyon_mupdf_{name}\n"))
        .collect();
    fs::write(&renames, header)?;

    println!(
        "rebuilding MuPDF with {} pdf_* symbols renamed",
        symbols.len()
    );
    clean(source)?;
    process::run(&mut make(source, target, Some(&renames))?)?;
    if !raw_pdf_symbols(&library, target)?.is_empty() {
        bail!(
            "{} still defines pdf_* symbols after renaming",
            library.display()
        );
    }
    Ok(())
}

fn make(source: &Path, target: &Target, renames: Option<&Path>) -> Result<Command> {
    let harfbuzz = root().join("crates/bridge_harfbuzz/harfbuzz/src");
    let jobs = thread::available_parallelism().map_or(1, usize::from);

    let mut cflags = match target {
        Target::Native => "-fPIC".to_owned(),
        Target::Wasm(sdk) => sdk.cflags(),
    };
    cflags.push_str(" -DTOFU_CJK -DFZ_ENABLE_JS=0 -DFZ_ENABLE_OCR_OUTPUT=0");
    if let Some(renames) = renames {
        cflags.push_str(&format!(" -include {}", renames.display()));
    }

    let (freetype, zlib) = match target {
        Target::Native => (
            flags_or_pkg_config("FREETYPE_CFLAGS", "freetype2")?,
            flags_or_pkg_config("ZLIB_CFLAGS", "zlib").unwrap_or_default(),
        ),
        Target::Wasm(_) => (
            format!(
                "-I{}",
                deps::wasm_vendor().join("freetype/include").display()
            ),
            format!("-I{}", source.join("thirdparty/zlib").display()),
        ),
    };

    let mut make = Command::new("make");
    make.arg("-C")
        .arg(source)
        .arg(format!("-j{jobs}"))
        .arg("build=release")
        .arg(format!("XCFLAGS={cflags}"))
        .args(["USE_SYSTEM_FREETYPE=yes", "SYS_FREETYPE_LIBS="])
        .arg(format!("SYS_FREETYPE_CFLAGS={freetype}"))
        .args(["USE_SYSTEM_ZLIB=yes", "SYS_ZLIB_LIBS="])
        .arg(format!("SYS_ZLIB_CFLAGS={zlib}"))
        .args(["USE_SYSTEM_HARFBUZZ=yes", "SYS_HARFBUZZ_LIBS="])
        .arg(format!("SYS_HARFBUZZ_CFLAGS=-I{}", harfbuzz.display()))
        .args([
            "HAVE_X11=no",
            "HAVE_GLUT=no",
            "HAVE_CURL=no",
            "HAVE_LIBCRYPTO=no",
            "HAVE_TESSERACT=no",
            "HAVE_LEPTONICA=no",
            "HAVE_OBJCOPY=no",
        ]);
    if let Target::Wasm(sdk) = target {
        make.arg("OS=wasi")
            .arg(format!("CC={}", sdk.tool("clang").display()))
            .arg(format!("CXX={}", sdk.tool("clang++").display()))
            .arg(format!("AR={}", sdk.tool("llvm-ar").display()))
            .arg(format!("RANLIB={}", sdk.tool("llvm-ranlib").display()));
    }
    make.arg("libs");
    Ok(make)
}

fn clean(source: &Path) -> Result<()> {
    let build = source.join("build/release");
    if build.exists() {
        fs::remove_dir_all(&build)
            .with_context(|| format!("could not clean {}", build.display()))?;
    }
    Ok(())
}

fn flags_or_pkg_config(variable: &str, package: &str) -> Result<String> {
    if let Ok(flags) = env::var(variable) {
        return Ok(flags);
    }
    process::stdout(Command::new("pkg-config").args(["--cflags", package]))
        .map(|flags| flags.trim().to_owned())
        .with_context(|| format!("set {variable} or install {package} for pkg-config"))
}

/// The `pdf_*` names `library` still defines under their own name.
fn raw_pdf_symbols(library: &Path, target: &Target) -> Result<BTreeSet<String>> {
    let nm = match target {
        Target::Native => PathBuf::from("nm"),
        Target::Wasm(sdk) => sdk.tool("llvm-nm"),
    };
    // Mach-O prefixes every C symbol with an underscore.
    let prefix = if cfg!(target_os = "macos") && matches!(target, Target::Native) {
        "_"
    } else {
        ""
    };
    let listing = process::stdout(Command::new(nm).args(["-g", "--defined-only"]).arg(library))?;
    Ok(listing
        .lines()
        .filter_map(
            |line| match line.split_whitespace().collect::<Vec<_>>()[..] {
                [_, kind, name] if "TDBRSW".contains(kind) => name.strip_prefix(prefix),
                _ => None,
            },
        )
        .filter(|name| name.starts_with("pdf_"))
        .map(str::to_owned)
        .collect())
}

/// WASI has no `mkstemp`. Every caller of MuPDF's temp-file helper is an output
/// path that can report failure, so it throws instead.
fn allow_missing_mkstemp(source: &Path) -> Result<()> {
    let path = source.join("source/fitz/document.c");
    let text = fs::read_to_string(&path)?;
    if text.contains("__wasi__") {
        return Ok(());
    }
    let anchor = "\twhile (out == NULL);\n#else";
    if text.matches(anchor).count() != 1 {
        bail!(
            "{} changed, the WASI mkstemp guard no longer applies",
            path.display()
        );
    }
    let guarded = "\twhile (out == NULL);\n#elif defined(__wasi__)\n\
                   \tfz_throw(ctx, FZ_ERROR_UNSUPPORTED, \"no temporary files under WASI\");\n#else";
    fs::write(&path, text.replace(anchor, guarded))?;
    Ok(())
}
