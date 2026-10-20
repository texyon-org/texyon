//! Feature-gated MuPDF shim compilation and symbol localization.

use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var_os("CARGO_FEATURE_BACKEND_MUPDF").is_none() {
        return;
    }
    if texyon_dep_support::target_is_wasm() {
        link_prebuilt(
            &texyon_dep_support::wasm_vendor_dir("mupdf"),
            "tools/wasm/fetch-deps.sh --with-mupdf",
        );
        return;
    }

    println!("cargo:rerun-if-env-changed=TEXYON_MUPDF_DIR");
    let os = std::env::var("CARGO_CFG_TARGET_OS");
    let env = std::env::var("CARGO_CFG_TARGET_ENV");
    match (os.as_deref(), env.as_deref()) {
        (Ok("windows"), Ok("msvc")) => build_windows_msvc(),
        (Ok("linux" | "macos"), _) if std::env::var_os("TEXYON_MUPDF_DIR").is_some() => {
            link_prebuilt(
                &PathBuf::from(std::env::var_os("TEXYON_MUPDF_DIR").unwrap()),
                "tools/mupdf/build.sh",
            );
            println!("cargo:rustc-link-lib=dylib=z");
        }
        (Ok("linux"), _) => build_linux(),
        // Workspace builds unify the feature in from the wasm crate without linking it.
        _ => println!(
            "cargo:warning=backend-mupdf is not linked: run tools/mupdf/build.sh \
             and set TEXYON_MUPDF_DIR to its tools/mupdf/vendor/mupdf"
        ),
    }
}

/// Link a MuPDF built by `tools/wasm/fetch-deps.sh` or `tools/mupdf/build.sh`.
///
/// Both already move `pdf_*` aside in the preprocessor (`llvm-objcopy` cannot rename wasm
/// symbols and macOS has no objcopy).
fn link_prebuilt(mupdf: &Path, script: &str) {
    println!("cargo:rerun-if-changed=src/backend_mupdf/shim.c");

    let build = mupdf.join("build/release");
    let renames = mupdf.join("tectonic-pdf-rename.h");

    for path in [
        &build.join("libmupdf.a"),
        &build.join("libmupdf-third.a"),
        &renames,
    ] {
        assert!(
            path.exists(),
            "no MuPDF build at {}\nThe SVG backend needs it; run: {script}",
            path.display()
        );
    }

    // A stale link keeps MuPDF's own HarfBuzz and traps deep inside font setup.
    for path in [&build.join("libmupdf.a"), &build.join("libmupdf-third.a")] {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    println!("cargo:rerun-if-changed={}", renames.display());

    cc::Build::new()
        .file("src/backend_mupdf/shim.c")
        .include(mupdf.join("include"))
        // The shim must see the same renames MuPDF was compiled with.
        .flag(format!("-include{}", renames.display()))
        .warnings(false)
        .compile("preview_render_mupdf_shim");

    println!("cargo:rustc-link-search=native={}", build.display());
    println!("cargo:rustc-link-lib=static=mupdf");
    println!("cargo:rustc-link-lib=static=mupdf-third");
}

fn build_linux() {
    println!("cargo:rerun-if-changed=src/backend_mupdf/shim.c");
    println!("cargo:rerun-if-env-changed=TEXYON_MUPDF_INCLUDE");
    println!("cargo:rerun-if-env-changed=TEXYON_MUPDF_LIB");
    let include = std::env::var_os("TEXYON_MUPDF_INCLUDE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr/include"));
    cc::Build::new()
        .file("src/backend_mupdf/shim.c")
        .include(include)
        .compile("preview_render_mupdf_shim");

    let archive = std::env::var_os("TEXYON_MUPDF_LIB")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr/lib/libmupdf.a"));
    assert!(
        archive.exists(),
        "MuPDF archive not found at {}",
        archive.display()
    );
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let localized = out.join("libmupdf_preview.a");
    localize_pdf_symbols(&archive, &localized, &out, "nm", "objcopy");
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=mupdf_preview");
    println!("cargo:rustc-link-search=native=/usr/lib");
    println!("cargo:rustc-link-lib=static=mupdf-third");
    // By soname, so MuPDF's FreeType cannot bind Tectonic's private `libharfbuzz.a`.
    println!("cargo:rustc-link-lib=dylib:+verbatim=libharfbuzz.so.0");
    for library in [
        "freetype", "jbig2dec", "openjp2", "jpeg", "z", "m", "gumbo", "mujs",
    ] {
        println!("cargo:rustc-link-lib=dylib={library}");
    }
}

fn build_windows_msvc() {
    println!("cargo:rerun-if-changed=src/backend_mupdf/shim.c");
    println!("cargo:rerun-if-env-changed=TEXYON_MUPDF_NM");
    println!("cargo:rerun-if-env-changed=TEXYON_MUPDF_OBJCOPY");

    let library = vcpkg::Config::new()
        .cargo_metadata(false)
        .find_package("libmupdf")
        .expect("find static MuPDF and its dependencies with vcpkg");
    cc::Build::new()
        .file("src/backend_mupdf/shim.c")
        .includes(&library.include_paths)
        .compile("preview_render_mupdf_shim");

    let archive = library
        .found_libs
        .iter()
        .find(|path| {
            path.file_stem()
                .and_then(OsStr::to_str)
                .is_some_and(|stem| stem.eq_ignore_ascii_case("libmupdf"))
        })
        .expect("vcpkg libmupdf package did not contain libmupdf.lib");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let localized = out.join("mupdf_preview.lib");
    localize_pdf_symbols(
        archive,
        &localized,
        &out,
        llvm_tool("TEXYON_MUPDF_NM", "llvm-nm"),
        llvm_tool("TEXYON_MUPDF_OBJCOPY", "llvm-objcopy"),
    );

    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=mupdf_preview");
    for path in &library.link_paths {
        println!("cargo:rustc-link-search=native={}", path.display());
    }
    for name in &library.found_names {
        if !name.eq_ignore_ascii_case("libmupdf") {
            println!("cargo:rustc-link-lib={name}");
        }
    }
}

fn llvm_tool(variable: &str, name: &str) -> OsString {
    if let Some(tool) = std::env::var_os(variable) {
        return tool;
    }
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"));
    let output = Command::new(rustc)
        .args(["--print", "sysroot"])
        .output()
        .expect("locate the Rust sysroot");
    if output.status.success() {
        let sysroot = String::from_utf8_lossy(&output.stdout);
        let bundled = PathBuf::from(sysroot.trim())
            .join("lib")
            .join("rustlib")
            .join(std::env::var("HOST").unwrap())
            .join("bin")
            .join(format!("{name}.exe"));
        if bundled.exists() {
            return bundled.into_os_string();
        }
    }
    OsString::from(name)
}

fn localize_pdf_symbols(
    source: &Path,
    destination: &Path,
    out: &Path,
    nm: impl AsRef<OsStr>,
    objcopy: impl AsRef<OsStr>,
) {
    std::fs::copy(source, destination).unwrap();
    let output = Command::new(nm)
        .args(["-g", "--defined-only"])
        .arg(destination)
        .output()
        .expect("run nm for MuPDF symbol localization");
    assert!(output.status.success());
    let mut symbols = BTreeSet::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Some(name) = line.split_whitespace().last() else {
            continue;
        };
        if name.starts_with("pdf_") {
            symbols.insert(name.to_owned());
        }
    }
    assert!(!symbols.is_empty(), "MuPDF archive has no pdf_* symbols");
    let map = symbols
        .into_iter()
        .map(|symbol| format!("{symbol} texyon_preview_{symbol}\n"))
        .collect::<String>();
    let map_path = out.join("mupdf-symbols.txt");
    std::fs::write(&map_path, map).unwrap();
    let status = Command::new(objcopy)
        .arg(format!("--redefine-syms={}", map_path.display()))
        .arg(destination)
        .status()
        .expect("localize MuPDF symbols");
    assert!(status.success());
}
