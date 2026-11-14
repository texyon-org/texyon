//! The WebAssembly module the browser and the VS Code fallback run.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};

use crate::{
    cargo::{self, Build},
    deps::WasiSdk,
    process,
};

/// The module's target triple.
pub const TARGET: &str = "wasm32-wasip1";
/// The module's file name, in the build and in an assets directory.
pub const MODULE: &str = "texyon_wasm.wasm";

/// How the module is compiled.
pub enum Variant {
    /// Unoptimized.
    Debug,
    /// Optimized without a profile.
    Release,
    /// Counting for profile-guided optimization, with the `pgo` feature's profile export.
    Instrumented,
    /// Optimized with the profile at this path.
    Optimized(PathBuf),
}

/// Build the module and return the path cargo wrote it to.
pub fn build(variant: &Variant) -> Result<PathBuf> {
    let sdk = WasiSdk::locate()?;
    let mut build = Build::release("texyon_wasm", TARGET);
    build.rustflags = sdk.rustflags();
    match variant {
        Variant::Debug => build.release = false,
        Variant::Release => {}
        Variant::Instrumented => {
            // rustup ships no profiler runtime for wasm, the `pgo` feature links
            // minicov instead, and -Zno-profiler-runtime keeps rustc from asking for one.
            build
                .rustflags
                .extend(["-Cprofile-generate".into(), "-Zno-profiler-runtime".into()]);
            build.unstable = true;
            build.features.push("pgo".into());
            build.target_dir = cargo::target_dir().join("pgo-instrumented");
        }
        Variant::Optimized(profile) => {
            build
                .rustflags
                .push(format!("-Cprofile-use={}", profile.display()));
        }
    }

    let mut cargo = build.command();
    sdk.configure_cc(&mut cargo);
    process::run(&mut cargo)?;

    let module = build.artifact(MODULE);
    let bytes =
        fs::read(&module).with_context(|| format!("could not read {}", module.display()))?;
    check_imports(&bytes)?;
    Ok(module)
}

/// Copy `module` into `out`, through wasm-opt when it is installed.
pub fn install(module: &Path, out: &Path) -> Result<PathBuf> {
    fs::create_dir_all(out)?;
    let dest = out.join(MODULE);
    if Command::new("wasm-opt").arg("--version").output().is_ok() {
        // The engine's setjmp needs exception handling and its arena clears bulk memory.
        process::run(
            Command::new("wasm-opt")
                .args(["-O2", "--enable-exception-handling", "--enable-bulk-memory"])
                .arg(module)
                .arg("-o")
                .arg(&dest),
        )?;
    } else {
        println!("wasm-opt is not installed, copying the module as built");
        fs::copy(module, &dest)?;
    }
    println!("module: {}", dest.display());
    Ok(dest)
}

/// Refuse a module importing anything a host cannot supply.
///
/// The linker turns an unresolved `__wasm_setjmp` or `__c_longjmp` into an
/// import instead of an error, and an imported tag is something no host can
/// provide, so such a module only fails at instantiation.
fn check_imports(module: &[u8]) -> Result<()> {
    let unexpected: Vec<String> = imports(module)?
        .into_iter()
        .filter(|(module, name)| match module.as_str() {
            "wasi_snapshot_preview1" => false,
            "env" => !name.starts_with("texyon_host_"),
            _ => true,
        })
        .map(|(module, name)| format!("{module}::{name}"))
        .collect();
    if !unexpected.is_empty() {
        bail!(
            "the module imports what no host provides: {}\n\
             An imported __c_longjmp means -lsetjmp did not reach the linker.",
            unexpected.join(", ")
        );
    }
    Ok(())
}

/// The `(module, name)` of every import, read from the module's import section.
fn imports(module: &[u8]) -> Result<Vec<(String, String)>> {
    const IMPORT_SECTION: u8 = 2;
    let mut reader = Reader {
        bytes: module,
        at: 8,
    };
    if module.get(..4) != Some(b"\0asm") {
        bail!("not a wasm module");
    }
    while reader.at < module.len() {
        let id = reader.byte()?;
        let size = reader.uleb()?;
        if id != IMPORT_SECTION {
            reader.at += size;
            continue;
        }
        let count = reader.uleb()?;
        let mut found = Vec::with_capacity(count);
        for _ in 0..count {
            let module = reader.name()?;
            let name = reader.name()?;
            reader.skip_import_kind()?;
            found.push((module, name));
        }
        return Ok(found);
    }
    Ok(Vec::new())
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn byte(&mut self) -> Result<u8> {
        let byte = *self.bytes.get(self.at).context("the module is truncated")?;
        self.at += 1;
        Ok(byte)
    }

    fn uleb(&mut self) -> Result<usize> {
        let mut value = 0usize;
        for shift in (0..64).step_by(7) {
            let byte = self.byte()?;
            value |= usize::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        bail!("an integer in the module is too long")
    }

    fn name(&mut self) -> Result<String> {
        let len = self.uleb()?;
        let bytes = self
            .bytes
            .get(self.at..self.at + len)
            .context("the module is truncated")?;
        self.at += len;
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }

    fn skip_import_kind(&mut self) -> Result<()> {
        match self.byte()? {
            // function: type index
            0 => {
                self.uleb()?;
            }
            // table: element type, limits
            1 => {
                self.byte()?;
                self.skip_limits()?;
            }
            // memory: limits
            2 => self.skip_limits()?,
            // global: value type, mutability
            3 => self.at += 2,
            // tag: attribute, type index
            4 => {
                self.byte()?;
                self.uleb()?;
            }
            kind => bail!("unknown import kind {kind}"),
        }
        Ok(())
    }

    fn skip_limits(&mut self) -> Result<()> {
        let flags = self.byte()?;
        self.uleb()?;
        if flags & 1 != 0 {
            self.uleb()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::check_imports;

    /// A module whose import section holds `imports`, each `(module, name, kind byte, kind payload)`.
    fn module(imports: &[(&str, &str, &[u8])]) -> Vec<u8> {
        let mut section = vec![u8::try_from(imports.len()).unwrap()];
        for (module, name, kind) in imports {
            for text in [module, name] {
                section.push(u8::try_from(text.len()).unwrap());
                section.extend_from_slice(text.as_bytes());
            }
            section.extend_from_slice(kind);
        }
        let mut bytes = b"\0asm\x01\0\0\0".to_vec();
        bytes.push(2);
        bytes.push(u8::try_from(section.len()).unwrap());
        bytes.extend(section);
        bytes
    }

    const FUNCTION: &[u8] = &[0, 0];
    const TAG: &[u8] = &[4, 0, 0];

    #[test]
    fn host_imports_pass() {
        let bytes = module(&[
            ("wasi_snapshot_preview1", "fd_write", FUNCTION),
            ("env", "texyon_host_read_file", FUNCTION),
        ]);
        assert!(check_imports(&bytes).is_ok());
    }

    #[test]
    fn an_imported_longjmp_tag_is_refused() {
        let bytes = module(&[
            ("wasi_snapshot_preview1", "fd_write", FUNCTION),
            ("env", "__c_longjmp", TAG),
        ]);
        let error = check_imports(&bytes).unwrap_err().to_string();
        assert!(error.contains("env::__c_longjmp"), "{error}");
    }
}
