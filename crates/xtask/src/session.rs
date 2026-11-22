//! A live-preview session in a child process, spoken to in protocol frames
//! over its stdio. The native binary and the wasm module serve the same frames.

use std::{
    fs,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{
        OnceLock,
        atomic::{AtomicU32, Ordering},
    },
};

use anyhow::{Context, Result, bail};
use texyon_preview_protocol::{
    frame,
    pack::{MANIFEST, PackManifest},
    session::{
        CompileReport, HostRequest, InlineBytes, Request, Response, SESSION_PROTOCOL_VERSION,
        SessionRequest,
    },
};

use crate::{cargo, process, project::Project, root};

/// What `\today` says in every session, so runs typeset alike.
const BUILD_DATE: u64 = 1_700_000_000;

/// What serves the session.
pub enum Host {
    /// `texyon preview`.
    Native {
        /// The binary.
        binary: PathBuf,
        /// The support pack it reads before the bundle.
        pack: PathBuf,
    },
    /// The wasm module under Node, through packages/wasm's stdio host.
    Wasm {
        /// The module.
        module: PathBuf,
        /// The support pack, which is all the module can read.
        pack: PathBuf,
    },
}

impl Host {
    /// The format name an open request gives this host.
    pub fn format(&self) -> Result<String> {
        match self {
            Self::Native { .. } => Ok("latex".into()),
            Self::Wasm { pack, .. } => {
                let manifest = fs::read_to_string(pack.join(MANIFEST))
                    .with_context(|| format!("no support pack in {}", pack.display()))?;
                Ok(serde_json::from_str::<PackManifest>(&manifest)?.format)
            }
        }
    }

    fn command(&self, profile: Option<&Path>) -> Result<Command> {
        Ok(match self {
            Self::Native { binary, pack } => {
                let mut command = Command::new(binary);
                command
                    .args([
                        "preview",
                        "--protocol",
                        &SESSION_PROTOCOL_VERSION.to_string(),
                    ])
                    .arg("--pack")
                    .arg(pack);
                if let Some(dir) = profile {
                    command.env("LLVM_PROFILE_FILE", dir.join("%m_%p.profraw"));
                }
                command
            }
            Self::Wasm { module, pack } => {
                let mut command = Command::new("node");
                command.arg(stdio_host()?).arg(module).arg(pack);
                if let Some(dir) = profile {
                    command.arg("--profile").arg(dir.join(next_profile_name()));
                }
                command
            }
        })
    }
}

/// packages/wasm's stdio host, bundled with its imports into one script Node runs.
fn stdio_host() -> Result<PathBuf> {
    static BUNDLED: OnceLock<PathBuf> = OnceLock::new();
    if let Some(script) = BUNDLED.get() {
        return Ok(script.clone());
    }
    let package = root().join("packages/wasm");
    let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
    if !package.join("node_modules").is_dir() {
        process::run(Command::new(npm).arg("ci").current_dir(&package))?;
    }
    let script = cargo::target_dir().join("stdio-host.mjs");
    process::run(
        Command::new(npm)
            .args(["run", "--silent", "build:stdio", "--"])
            .arg(format!("--outfile={}", script.display()))
            .current_dir(&package),
    )?;
    Ok(BUNDLED.get_or_init(|| script).clone())
}

fn next_profile_name() -> String {
    static SESSIONS: AtomicU32 = AtomicU32::new(0);
    format!(
        "wasm-{}-{}.profraw",
        std::process::id(),
        SESSIONS.fetch_add(1, Ordering::Relaxed)
    )
}

/// One running session.
pub struct Session {
    child: Child,
    input: BufWriter<ChildStdin>,
    output: BufReader<ChildStdout>,
}

impl Session {
    /// Start a session. An instrumented host writes its profile into `profile` when it exits.
    pub fn start(host: &Host, profile: Option<&Path>) -> Result<Self> {
        let mut command = host.command(profile)?;
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| {
                format!(
                    "could not start {}",
                    command.get_program().to_string_lossy()
                )
            })?;
        let input = BufWriter::new(child.stdin.take().context("no stdin")?);
        let output = BufReader::new(child.stdout.take().context("no stdout")?);
        Ok(Self {
            child,
            input,
            output,
        })
    }

    /// Send a request to the open session and wait for its response.
    pub fn send(&mut self, request: SessionRequest) -> Result<Response<InlineBytes>> {
        self.exchange(&Request::Session(request))
    }

    /// Open the session on `project`, drawing pixels instead of SVG when `raster`.
    pub fn open(&mut self, host: &Host, project: &Project, raster: bool) -> Result<()> {
        let root = std::path::absolute(&project.root)?;
        let request = HostRequest::Open {
            format: host.format()?,
            primary: project.primary.clone(),
            root: Some(root.to_string_lossy().into_owned()),
            raster,
            build_date: Some(BUILD_DATE),
            patches: false,
        };
        match self.exchange(&Request::Host(request))? {
            Response::Ok => Ok(()),
            other => bail!("could not open the session: {}", describe(&other)),
        }
    }

    /// Run a request that answers with a compile report.
    pub fn compile(&mut self, request: SessionRequest) -> Result<CompileReport<InlineBytes>> {
        match self.send(request)? {
            Response::Compiled(report) | Response::Passed { report, .. } => Ok(*report),
            other => bail!("expected a compile report, got {}", describe(&other)),
        }
    }

    /// Typeset what a viewport-bounded compile left, and return the page count after it.
    pub fn finish_tail(&mut self) -> Result<Option<usize>> {
        let mut pages = None;
        loop {
            match self.send(SessionRequest::ContinueTail)? {
                Response::Compiled(report) => {
                    pages = Some(report.page_count);
                    if !report.tail_pending {
                        return Ok(pages);
                    }
                }
                Response::Idle => return Ok(pages),
                other => bail!("expected the tail, got {}", describe(&other)),
            }
        }
    }

    /// The page a source line (zero-based) typeset onto, or the first page if it found none.
    pub fn page_of(&mut self, path: &str, line: usize) -> Result<usize> {
        Ok(
            match self.send(SessionRequest::ForwardSearch {
                path: path.into(),
                line: line + 1,
            })? {
                Response::Located {
                    rect: Some(rect), ..
                } => rect.page_index,
                _ => 0,
            },
        )
    }

    fn exchange(&mut self, request: &Request<InlineBytes>) -> Result<Response<InlineBytes>> {
        frame::write_message(&mut self.input, request)?;
        frame::read_message(&mut self.output)?.context("the session closed its output")
    }

    /// Shut the session down and wait for the host to exit.
    pub fn finish(mut self) -> Result<()> {
        self.exchange(&Request::Host(HostRequest::Shutdown))?;
        drop(self.input);
        let status = self.child.wait()?;
        if !status.success() {
            bail!("the session host exited with {status}");
        }
        Ok(())
    }
}

/// A response's status and message, for an error.
pub fn describe(response: &Response<InlineBytes>) -> String {
    match response {
        Response::Error { message } => format!("error: {message}"),
        other => serde_json::to_value(other)
            .ok()
            .and_then(|value| {
                value
                    .get("status")
                    .and_then(|status| status.as_str())
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| "an unexpected response".into()),
    }
}
