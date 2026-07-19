//! Feature-gated MuPDF shim compilation and symbol localization.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var_os("CARGO_FEATURE_BACKEND_MUPDF").is_none() {
        return;
    }
    assert_eq!(std::env::var("CARGO_CFG_TARGET_OS").as_deref(), Ok("linux"));
    println!("cargo:rerun-if-changed=src/backend_mupdf/shim.c");
    println!("cargo:rerun-if-env-changed=TECTONIC_MUPDF_INCLUDE");
    println!("cargo:rerun-if-env-changed=TECTONIC_MUPDF_LIB");
    let include = std::env::var_os("TECTONIC_MUPDF_INCLUDE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr/include"));
    cc::Build::new()
        .file("src/backend_mupdf/shim.c")
        .include(include)
        .compile("preview_render_mupdf_shim");

    let archive = std::env::var_os("TECTONIC_MUPDF_LIB")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr/lib/libmupdf.a"));
    assert!(
        archive.exists(),
        "MuPDF archive not found at {}",
        archive.display()
    );
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let localized = out.join("libmupdf_preview.a");
    localize_pdf_symbols(&archive, &localized, &out);
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=mupdf_preview");
    println!("cargo:rustc-link-search=native=/usr/lib");
    println!("cargo:rustc-link-lib=static=mupdf-third");
    // Tectonic also builds a private, feature-reduced `libharfbuzz.a`. Request
    // the system shared object by its exact soname so that MuPDF's FreeType
    // integration cannot accidentally bind to that private archive.
    println!("cargo:rustc-link-lib=dylib:+verbatim=libharfbuzz.so.0");
    for library in [
        "freetype", "jbig2dec", "openjp2", "jpeg", "z", "m", "gumbo", "mujs",
    ] {
        println!("cargo:rustc-link-lib=dylib={library}");
    }
}

fn localize_pdf_symbols(source: &Path, destination: &Path, out: &Path) {
    std::fs::copy(source, destination).unwrap();
    let output = Command::new("nm")
        .args(["-g", "--defined-only"])
        .arg(destination)
        .output()
        .expect("run nm for MuPDF symbol localization");
    assert!(output.status.success());
    let mut symbols = BTreeSet::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let mut fields = line.split_whitespace();
        let name = match (fields.next(), fields.next(), fields.next()) {
            (Some(_), Some(_), Some(name)) => name,
            _ => continue,
        };
        if name.starts_with("pdf_") {
            symbols.insert(name.to_owned());
        }
    }
    assert!(!symbols.is_empty(), "MuPDF archive has no pdf_* symbols");
    let map = symbols
        .into_iter()
        .map(|symbol| format!("{symbol} tectonic_preview_{symbol}\n"))
        .collect::<String>();
    let map_path = out.join("mupdf-symbols.txt");
    std::fs::write(&map_path, map).unwrap();
    let status = Command::new("objcopy")
        .arg(format!("--redefine-syms={}", map_path.display()))
        .arg(destination)
        .status()
        .expect("localize MuPDF symbols");
    assert!(status.success());
}
