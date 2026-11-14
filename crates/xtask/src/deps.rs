//! Third-party toolchains and C sources the builds need, kept in `deps/`.

use std::{
    env::consts,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Result, bail};

use crate::{fetch::Archive, mupdf, root};

const WASI_SDK_RELEASE: u32 = 33;
const LIBPNG: &str = "1.6.50";
const FREETYPE: &str = "2.13.3";
const GRAPHITE2: &str = "1.3.14";
/// Shared with the native build in [`mupdf`].
pub const MUPDF: &str = "1.24.9";

/// The `deps/` directory.
pub fn dir() -> PathBuf {
    root().join("deps")
}

/// MuPDF's release source archive.
pub fn mupdf_archive() -> Archive {
    Archive {
        name: "MuPDF",
        url: format!("https://mupdf.com/downloads/archive/mupdf-{MUPDF}-source.tar.gz"),
    }
}

/// Fetch everything a wasm build compiles against, and with `with_mupdf` build MuPDF for it.
pub fn fetch_wasm(with_mupdf: bool, mupdf_source: Option<&Path>) -> Result<()> {
    let sdk = WasiSdk::path();
    WasiSdk::archive()?.install(&sdk, None)?;

    let vendor = wasm_vendor();
    let freetype_tag = FREETYPE.replace('.', "-");
    let sources = [
        Archive {
            name: "libpng",
            url: format!("https://github.com/pnggroup/libpng/archive/refs/tags/v{LIBPNG}.tar.gz"),
        },
        // GitHub, since savannah's mirror redirector sometimes serves HTML with status 200.
        Archive {
            name: "freetype",
            url: format!(
                "https://github.com/freetype/freetype/archive/refs/tags/VER-{freetype_tag}.tar.gz"
            ),
        },
        Archive {
            name: "graphite2",
            url: format!(
                "https://github.com/silnrsi/graphite/archive/refs/tags/{GRAPHITE2}.tar.gz"
            ),
        },
    ];
    fs::create_dir_all(&vendor)?;
    for archive in &sources {
        archive.install(&vendor.join(archive.name), None)?;
    }

    // The template every distribution installs as libpng's build configuration.
    let png_config = vendor.join("libpng/pnglibconf.h");
    if !png_config.exists() {
        fs::copy(
            vendor.join("libpng/scripts/pnglibconf.h.prebuilt"),
            png_config,
        )?;
    }

    if with_mupdf {
        let source = vendor.join("mupdf");
        mupdf_archive().install(&source, mupdf_source)?;
        mupdf::build(&source, &mupdf::Target::Wasm(WasiSdk::locate()?))?;
    }
    Ok(())
}

/// Where the wasm build's C library sources live.
pub fn wasm_vendor() -> PathBuf {
    dir().join("wasm")
}

/// The wasi-sdk C toolchain the wasm build compiles its C and C++ with.
pub struct WasiSdk {
    root: PathBuf,
}

impl WasiSdk {
    fn path() -> PathBuf {
        dir().join(format!("wasi-sdk-{WASI_SDK_RELEASE}"))
    }

    fn archive() -> Result<Archive> {
        let host = match (consts::OS, consts::ARCH) {
            ("linux", "x86_64") => "x86_64-linux",
            ("linux", "aarch64") => "arm64-linux",
            ("macos", "x86_64") => "x86_64-macos",
            ("macos", "aarch64") => "arm64-macos",
            ("windows", "x86_64") => "x86_64-windows",
            (os, arch) => bail!("wasi-sdk has no release for {arch} {os}"),
        };
        Ok(Archive {
            name: "wasi-sdk",
            url: format!(
                "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-{WASI_SDK_RELEASE}/\
                 wasi-sdk-{WASI_SDK_RELEASE}.0-{host}.tar.gz"
            ),
        })
    }

    /// The fetched toolchain, or an error saying how to fetch it.
    pub fn locate() -> Result<Self> {
        let root = Self::path();
        if !root.is_dir() {
            bail!("no wasi-sdk at {}, run: cargo xtask deps", root.display());
        }
        Ok(Self { root })
    }

    /// A tool from the toolchain's `bin/`.
    pub fn tool(&self, name: &str) -> PathBuf {
        self.root.join("bin").join(name)
    }

    /// The C sysroot.
    pub fn sysroot(&self) -> PathBuf {
        self.root.join("share/wasi-sysroot")
    }

    /// C flags for every C and C++ file in the module.
    ///
    /// TeX reports fatal errors by longjmp'ing out of the engine, which wasm can
    /// only do through the exception-handling proposal.
    pub fn cflags(&self) -> String {
        format!(
            "--sysroot={} --target=wasm32-wasip1 -mllvm -wasm-enable-sjlj",
            self.sysroot().display()
        )
    }

    /// rustc flags for linking the module.
    pub fn rustflags(&self) -> Vec<String> {
        let libs = self.sysroot().join("lib/wasm32-wasip1");
        vec![
            format!("-Lnative={}", libs.display()),
            // HarfBuzz and Graphite2 are C++ and need wasi-sdk's libc++, the
            // no-exceptions build to match cc-rs's -fno-exceptions.
            format!("-Lnative={}", libs.join("noeh").display()),
            // Without it rust-lld turns __wasm_setjmp and __c_longjmp into
            // imports no host can supply, and the module fails to instantiate.
            "-Clink-arg=-lsetjmp".into(),
            // Lets crates/wasm/src/alloc_shim.rs replace wasi-libc's malloc.
            "-Clink-arg=--allow-multiple-definition".into(),
            "-Clink-arg=--initial-memory=67108864".into(),
            "-Clink-arg=--max-memory=2147483648".into(),
        ]
    }

    /// Point the C and C++ compiles of a `cargo` build at this toolchain.
    pub fn configure_cc(&self, cargo: &mut Command) {
        cargo
            // cc links libc++abi, where operator new lives, only when it knows the sysroot.
            .env("WASI_SYSROOT", self.sysroot())
            .env("CC_wasm32_wasip1", self.tool("clang"))
            .env("CXX_wasm32_wasip1", self.tool("clang++"))
            .env("AR_wasm32_wasip1", self.tool("llvm-ar"))
            .env("CFLAGS_wasm32_wasip1", self.cflags())
            .env("CXXFLAGS_wasm32_wasip1", self.cflags());
    }
}
