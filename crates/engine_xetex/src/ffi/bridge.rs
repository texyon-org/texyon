//! Engine-side I/O and diagnostics through the operation's [`EngineEnv`].
//!
//! Replay sources answer opens first, everything else goes to the env's host.

use std::borrow::Cow;
use std::ffi::CStr;

pub(crate) use texyon_bridge_core::{CoreBridgeState, Diagnostic, FileFormat, FoundAs};

use crate::env::EngineEnv;
use crate::files::input_file::InputSource;

/// Runs `f` with a bridge state over the env's host, for code that names
/// files by id. Files it leaves open are dropped with the state.
pub(crate) fn with_bridge_state<R>(
    env: &mut EngineEnv,
    f: impl FnOnce(&mut CoreBridgeState<'_, '_>) -> R,
) -> R {
    // C reports straight to the host, after what the terminal holds.
    env.flush_terminal();
    f(&mut CoreBridgeState::new(&mut env.host))
}

/// `bytes` as bridge-core saw them through a C string: empty when they hold a NUL.
fn c_view(bytes: &[u8]) -> Cow<'_, str> {
    if bytes.contains(&0) {
        Cow::Borrowed("")
    } else {
        String::from_utf8_lossy(bytes)
    }
}

/// An input the engine opened, with how it was found.
pub(crate) struct OpenedSource {
    pub source: InputSource,
    pub found: FoundAs,
    /// Its absolute path for SyncTeX, empty when not known or not exposed.
    pub abspath: Vec<u8>,
}

/// Longest absolute input path kept, including C's terminating NUL.
const ABSPATH_CAPACITY: usize = 1024;

fn memory_opened(env: &mut EngineEnv, source: InputSource) -> OpenedSource {
    let abspath = env
        .sources()
        .and_then(|sources| sources.last_abspath(ABSPATH_CAPACITY - 1))
        .unwrap_or_default();
    OpenedSource {
        source,
        found: FoundAs::Asked,
        abspath,
    }
}

fn disk_opened(env: &mut EngineEnv, opened: texyon_bridge_core::OpenedInput) -> OpenedSource {
    let abspath = match env.host.exposed_path(opened.path.as_deref()) {
        Some(path) if path.len() < ABSPATH_CAPACITY => {
            let end = path.find('\0').unwrap_or(path.len());
            path.as_bytes()[..end].to_vec()
        }
        _ => Vec::new(),
    };
    OpenedSource {
        source: InputSource::Disk(opened.handle),
        found: opened.found,
        abspath,
    }
}

/// Opens input file `path`, from the replay sources if they hold it.
pub(crate) fn input_open(
    env: &mut EngineEnv,
    path: &[u8],
    format: FileFormat,
    is_gz: bool,
) -> Option<OpenedSource> {
    let path = String::from_utf8_lossy(path);
    if let Some(id) = env
        .sources()
        .and_then(|sources| sources.open_named(&path, format))
    {
        return Some(memory_opened(env, InputSource::Memory(id)));
    }
    let opened = env.host.open_input(&path, format, is_gz)?;
    Some(disk_opened(env, opened))
}

/// Opens the primary input.
pub(crate) fn input_open_primary(env: &mut EngineEnv) -> Option<OpenedSource> {
    if let Some(id) = env.sources().map(|sources| sources.open_primary()) {
        return Some(memory_opened(env, InputSource::Memory(id)));
    }
    let opened = env.host.open_primary_input()?;
    Some(disk_opened(env, opened))
}

pub(crate) fn directory_exists(env: &mut EngineEnv, dir: &[u8]) -> bool {
    let dir = String::from_utf8_lossy(dir);
    env.sources()
        .is_some_and(|sources| sources.holds_directory(&dir))
        || env.host.directory_exists(&dir)
}

/// Appends `text`. A text with a NUL appends nothing.
pub(crate) fn diag_append(diag: &mut Diagnostic, text: &str) {
    if !text.contains('\0') {
        diag.append(text);
    }
}

pub(crate) fn diag_finish(env: &mut EngineEnv, diag: Diagnostic) {
    env.flush_terminal();
    env.host.finish_diagnostic(diag);
}

/// Issues `text` as a warning, empty when it holds a NUL.
pub(crate) fn issue_warning(env: &mut EngineEnv, text: &str) {
    env.flush_terminal();
    env.host.issue_warning(&c_view(text.as_bytes()));
}

/// Issues `text` as an error, empty when it holds a NUL.
pub(crate) fn issue_error(env: &mut EngineEnv, text: &str) {
    env.flush_terminal();
    env.host.issue_error(&c_view(text.as_bytes()));
}

pub(crate) fn shell_escape(env: &mut EngineEnv, cmd: &[u16]) {
    match String::from_utf16(cmd) {
        Ok(cmd) => {
            env.host.shell_escape(&cmd);
        }
        Err(err) => env
            .host
            .issue_error(&format!("command was not valid UTF-16: {err}")),
    }
}

pub(crate) fn strerror(errnum: i32) -> Vec<u8> {
    // SAFETY: strerror returns a NUL-terminated message, copied before any other call.
    unsafe { CStr::from_ptr(libc::strerror(errnum)) }
        .to_bytes()
        .to_vec()
}

pub(crate) fn get_file_md5(env: &mut EngineEnv, path: &[u8]) -> Option<[u8; 16]> {
    let mut digest = [0; 16];
    let failed = env.host.get_file_md5(&c_view(path), &mut digest);
    (!failed).then_some(digest)
}

/// Surfaces `message` as the run's abort message. A message with a NUL is dropped.
pub(crate) fn set_error_message(message: impl AsRef<[u8]>) {
    texyon_bridge_core::set_abort_message(message.as_ref());
}
