//! Builds, tests and profiles TeXyoN's release artifacts: `cargo xtask --help`.

mod bench;
mod cargo;
mod corpus;
mod deps;
mod differential;
mod fetch;
mod mupdf;
mod native;
mod pack;
mod pgo;
mod process;
mod project;
mod scenario;
mod session;
mod test;
mod wasm;

use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::session::Host;

#[derive(Parser)]
#[command(about = "Build, test and profile TeXyoN's release artifacts")]
struct Cli {
    #[command(subcommand)]
    command: Task,
}

#[derive(Subcommand)]
enum Task {
    /// Fetch wasi-sdk and the C sources the wasm build compiles into deps/.
    Deps {
        /// Also build MuPDF for the wasm module's renderer.
        #[arg(long)]
        mupdf: bool,
        /// A MuPDF source archive to use instead of downloading it.
        #[arg(long, value_name = "TARBALL")]
        mupdf_source: Option<PathBuf>,
    },
    /// Build MuPDF for the native preview into deps/mupdf.
    Mupdf {
        /// A MuPDF source archive to use instead of downloading it.
        #[arg(long, value_name = "TARBALL")]
        source: Option<PathBuf>,
    },
    /// Build the wasm module.
    Wasm {
        /// An unoptimized build.
        #[arg(long, conflicts_with = "pgo")]
        debug: bool,
        /// Optimize with a profile of the training sessions.
        #[arg(long)]
        pgo: bool,
        /// Where the module goes, target/wasm-assets by default.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Build the native `texyon` binary.
    Native {
        /// The target triple, the host's by default.
        #[arg(long)]
        target: Option<String>,
        /// Cargo features, such as preview-mupdf.
        #[arg(long, value_delimiter = ',')]
        features: Vec<String>,
        /// Optimize with a profile of the training sessions, which needs preview-mupdf.
        #[arg(long)]
        pgo: bool,
    },
    /// Build the support pack from the standard seed documents.
    Pack {
        /// Where the pack goes, target/wasm-assets by default.
        #[arg(long)]
        out: Option<PathBuf>,
        /// More documents to seed from: project directories around a main.tex, or .tex files.
        #[arg(long = "seed", value_name = "DIR")]
        seeds: Vec<PathBuf>,
    },
    /// Test a built host end to end through a live-preview session.
    Test(HostArgs),
    /// Time the editing session on documents.
    Bench {
        #[command(flatten)]
        host: HostArgs,
        /// Runs per document, the fastest counts.
        #[arg(long, default_value_t = 3)]
        rounds: usize,
        /// Project directories around a main.tex, or .tex files. The corpus fixtures by default.
        documents: Vec<PathBuf>,
    },
    /// Check that incremental typesetting matches fresh sessions on documents.
    Differential {
        #[command(flatten)]
        host: HostArgs,
        /// Project directories around a main.tex, or .tex files. The corpus fixtures by default.
        documents: Vec<PathBuf>,
    },
}

/// Which host serves the sessions, and where its files are.
#[derive(Args)]
struct HostArgs {
    /// The host.
    kind: HostKind,
    /// The support pack, with the wasm module beside it. target/wasm-assets by default.
    #[arg(long)]
    assets: Option<PathBuf>,
    /// A native binary to use instead of building one with preview-mupdf.
    #[arg(long)]
    binary: Option<PathBuf>,
}

#[derive(Clone, Copy, ValueEnum)]
enum HostKind {
    /// `texyon -X preview`.
    Native,
    /// The wasm module under Node.
    Wasm,
}

impl HostArgs {
    fn host(self) -> Result<Host> {
        let pack = assets(self.assets);
        Ok(match self.kind {
            HostKind::Native => Host::Native {
                binary: match self.binary {
                    Some(binary) => binary,
                    None => native::build(
                        &cargo::host()?,
                        &["preview-mupdf".into()],
                        &native::Variant::Release,
                    )?,
                },
                pack,
            },
            HostKind::Wasm => Host::Wasm {
                module: pack.join(wasm::MODULE),
                pack,
            },
        })
    }
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Task::Deps {
            mupdf,
            mupdf_source,
        } => deps::fetch_wasm(mupdf, mupdf_source.as_deref()),
        Task::Mupdf { source } => mupdf::build_native(source.as_deref()).map(drop),
        Task::Wasm { debug, pgo, out } => {
            let module = match (debug, pgo) {
                (true, _) => wasm::build(&wasm::Variant::Debug)?,
                (false, true) => pgo::wasm()?,
                (false, false) => wasm::build(&wasm::Variant::Release)?,
            };
            wasm::install(&module, &assets(out)).map(drop)
        }
        Task::Native {
            target,
            features,
            pgo,
        } => {
            let target = target.map_or_else(cargo::host, Ok)?;
            let binary = if pgo {
                pgo::native(&target, &features)?
            } else {
                native::build(&target, &features, &native::Variant::Release)?
            };
            println!("binary: {}", binary.display());
            Ok(())
        }
        Task::Pack { out, seeds } => {
            let mut all = pack::standard_seeds()?;
            for seed in &seeds {
                all.push(corpus::open(seed)?.into());
            }
            let texyon = native::build(&cargo::host()?, &[], &native::Variant::Release)?;
            pack::build(&texyon, None, &all, &assets(out), pack::Archive::Write)
        }
        Task::Test(args) => test::run(&args.host()?),
        Task::Bench {
            host,
            rounds,
            documents,
        } => bench::run(&host.host()?, &or_defaults(documents), rounds),
        Task::Differential { host, documents } => {
            differential::run(&host.host()?, &or_defaults(documents))
        }
    }
}

fn or_defaults(documents: Vec<PathBuf>) -> Vec<PathBuf> {
    if documents.is_empty() {
        corpus::defaults()
    } else {
        documents
    }
}

/// Where the module and the pack go: `out`, or the workspace's `target/wasm-assets`.
fn assets(out: Option<PathBuf>) -> PathBuf {
    out.unwrap_or_else(|| cargo::target_dir().join("wasm-assets"))
}

/// The repository checkout.
fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap_or_else(|| Path::new("."))
}
