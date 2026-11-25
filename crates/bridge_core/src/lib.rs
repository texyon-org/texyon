// Copyright 2016-2022 the Tectonic Project
// Licensed under the MIT License.

//! Core APIs for bridging the C and Rust portions of Tectonic’s processing
//! backends.
//!
//! This crate is used by the Tectonic "engines", which are predominantly C/C++
//! code derived from the original TeX codebase. It provides a framework so that
//! the C/C++ code can invoke the support services provided by Tectonic, such as
//! its pluggable I/O backends. The interfaces exposed to the C/C++ layers are
//! created by [cbindgen].
//!
//! [cbindgen]: https://github.com/eqrion/cbindgen
//!
//! If you change the interfaces here, rerun cbindgen as described in the README!
//!
//! In order to provide access to a C/C++ engine in Rust, you should create some
//! kind of interface that expects to be given a reference to a
//! [`CoreBridgeLauncher`] struct. You should use that struct's
//! `with_global_lock` method to obtain a [`CoreBridgeState`] reference, and
//! then pass that reference across the FFI layer. On the other side of the FFI
//! divide, your code *must* call the functions `ttbc_global_engine_enter()` and
//! `ttbc_global_engine_exit()` according to the pattern described in
//! `texyon_bridge_core.h`. If an abort is detected, the callback function
//! must return `Err(EngineAbortedError::new_abort_indicator().into())`.
//! Unfortunately, this is the cleanest and most reliable API that we can
//! provide because our abort handling uses `setjmp`/`longjmp` and those can't
//! cross FFI boundaries.
//!
//! In order to use a C/C++ engine, you need to provide something that
//! implements the [`DriverHooks`] trait. The [`MinimalDriver`] struct provides
//! a minimal implementation that only requires you to provide an [`IoProvider`]
//! implementation.
//!
//! The services themselves live in [`BridgeHost`]: hooks, status and settings,
//! with lookups that hand out owned handles. A Rust engine can keep a host for
//! as long as it likes, owning it ([`Held::Owned`]) or borrowing a launcher's
//! ([`CoreBridgeLauncher::host`]). A [`CoreBridgeState`] is a host plus the id
//! tables C code needs, and borrows its host for as long as the C code runs.

use flate2::{Compression, GzBuilder, read::GzDecoder};
use md5::{Digest, Md5};
use std::num::NonZeroUsize;
use std::{
    ffi::{CStr, CString},
    fmt::{Display, Error as FmtError, Formatter},
    io::{self, Read, SeekFrom, Write},
    marker::PhantomData,
    path::PathBuf,
    result::Result as StdResult,
    slice,
    sync::Mutex,
};
use texyon_errors::prelude::*;
use texyon_io_base::{
    InputFeatures, InputHandle, IoProvider, OpenResult, OutputHandle, digest::DigestData,
    normalize_tex_path,
};
use texyon_status_base::{MessageKind, StatusBackend, tt_error, tt_warning};

/// The C `EOF` sentinel.
///
/// The `libc` crate only defines `EOF` for platforms with a stdio, which
/// excludes the WASI targets, but the value is universal — and the engines
/// exchange it with this crate's `getc`-alikes on every target.
pub const EOF: libc::c_int = -1;

/// The ID of an InputHandle, used for Rust core state
#[derive(Copy, Clone, PartialEq)]
#[repr(transparent)]
pub struct InputId(NonZeroUsize);

impl InputId {
    fn new(idx: usize) -> Option<Self> {
        NonZeroUsize::new(idx).map(InputId)
    }

    fn idx(self) -> usize {
        self.0.get() - 1
    }

    /// Reconstitute an ID from the raw `usize` handle representation used
    /// across the engine C ABI (`rust_input_handle_t`; 0 means "no handle").
    pub fn from_raw(raw: usize) -> Option<Self> {
        NonZeroUsize::new(raw).map(InputId)
    }

    /// The raw `usize` handle representation of an optional ID (0 means "no
    /// handle") — the inverse of [`InputId::from_raw`].
    pub fn to_raw(id: Option<Self>) -> usize {
        id.map_or(0, |i| i.0.get())
    }
}

/// The ID of an OutputHandle, used for Rust core state
#[derive(Copy, Clone, PartialEq)]
#[repr(transparent)]
pub struct OutputId(NonZeroUsize);

impl OutputId {
    fn new(idx: usize) -> Option<Self> {
        NonZeroUsize::new(idx).map(OutputId)
    }

    fn idx(self) -> usize {
        self.0.get() - 1
    }

    /// Reconstitute an ID from the raw `usize` handle representation used
    /// across the engine C ABI (`rust_output_handle_t`; 0 means "no handle").
    pub fn from_raw(raw: usize) -> Option<Self> {
        NonZeroUsize::new(raw).map(OutputId)
    }

    /// The raw `usize` handle representation of an optional ID (0 means "no
    /// handle") — the inverse of [`OutputId::from_raw`].
    pub fn to_raw(id: Option<Self>) -> usize {
        id.map_or(0, |i| i.0.get())
    }
}

/// Possible failures for "system request" calls to the driver.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemRequestError {
    /// The driver does not implement this system request.
    NotImplemented,

    /// The driver implements this system request but is not allowing it
    /// to be used in this circumstance.
    NotAllowed,

    /// The driver tried to execute this system request but it failed in some
    /// fashion. There's no facility for providing more detailed information
    /// because the calling C/C++ code can't do anything useful with the
    /// details.
    Failed,
}

impl Display for SystemRequestError {
    fn fmt(&self, f: &mut Formatter) -> StdResult<(), FmtError> {
        write!(
            f,
            "{}",
            match self {
                SystemRequestError::NotImplemented => "not implemented by this driver",
                SystemRequestError::NotAllowed => "not allowed by this driver",
                SystemRequestError::Failed => "execution of the request failed",
            }
        )
    }
}

/// The DriverHooks trait allows engines to interact with the higher-level code
/// that is driving the TeX processing.
///
/// Drivers mainly manage interactions between the engines and the outside
/// world. The primary way they do this is by exposing an [`IoProvider`]
/// implementation.
///
/// Drivers can also implement handlers for additional events that help it track
/// the input and output access patterns of the engines. The CLI program needs
/// these to intelligently decide when to rerun the TeX engine, to choose which
/// files to actually save to disk, and to emit Makefile rules describing the
/// dependency of the outputs on the inputs. The relevant trait methods have
/// default implementations that do nothing.
pub trait DriverHooks {
    /// Get the main I/O implementations of this driver.
    fn io(&mut self) -> &mut dyn IoProvider;

    /// This function is called when an output file is closed. The "digest"
    /// argument specifies the cryptographic digest of the data that were
    /// written. Note that this function takes ownership of the name and
    /// digest.
    fn event_output_closed(&mut self, _name: String, _digest: DigestData) {}

    /// This function is called when an input file is closed. The "digest"
    /// argument specifies the cryptographic digest of the data that were
    /// read, if available. This digest is not always available, if the engine
    /// used seeks while reading the file. Note that this function takes
    /// ownership of the name and digest.
    fn event_input_closed(
        &mut self,
        _name: String,
        _digest: Option<DigestData>,
        _status: &mut dyn StatusBackend,
    ) {
    }

    /// The engine is requesting a "shell escape" evaluation.
    ///
    /// If the driver wishes to implement this request, it should run the
    /// specified command using the OS’s default shell. Relevant files should be
    /// available in the command's working directory, and if the command creates
    /// any files, they should be incorporated into the I/O environment. The
    /// shell-escape environment should persist across multiple invocations of
    /// this system request, because some packages run a series of commands that
    /// assume such persistence. Also note that the command text has to be
    /// evaluated through a shell, not just with `exec()`, since shell features
    /// such as redirections might be used. This is therefore a wildly insecure
    /// feature.
    ///
    /// This function can only return a limited range of error values because
    /// the C/C++ engines can't do anything useful with them. Detailed error
    /// information should be logged or stored inside the hook function.
    fn sysrq_shell_escape(
        &mut self,
        _command: &str,
        _status: &mut dyn StatusBackend,
    ) -> StdResult<(), SystemRequestError> {
        Err(SystemRequestError::NotImplemented)
    }
}

/// This type provides a minimal [`DriverHooks`] implementation.
#[derive(Clone, Debug, Default)]
pub struct MinimalDriver<T: IoProvider>(T);

impl<T: IoProvider> MinimalDriver<T> {
    /// Create a new minimal driver.
    pub fn new(io: T) -> Self {
        MinimalDriver(io)
    }
}

impl<T: IoProvider> DriverHooks for MinimalDriver<T> {
    fn io(&mut self) -> &mut dyn IoProvider {
        &mut self.0
    }
}

// Function defined in the C support code:
unsafe extern "C" {
    fn _ttbc_get_error_message() -> *const libc::c_char;
    fn _ttbc_set_error_message(message: *const libc::c_char);
    #[allow(improper_ctypes)]
    fn ttbc_global_engine_enter(api: *mut CoreBridgeState<'_, '_>) -> *mut libc::c_void;
    fn ttbc_global_engine_exit();
}

/// Sets the message the next [`EngineAbortedError`] reports, for engines that
/// abort by returning rather than through `_tt_abort`. A message with a NUL is
/// dropped, and C truncates it to its buffer.
pub fn set_abort_message(message: &[u8]) {
    if let Ok(message) = CString::new(message) {
        // SAFETY: `message` is NUL-terminated and the callee copies it.
        unsafe { _ttbc_set_error_message(message.as_ptr()) };
    }
}

/// Calculates the MD5 digest of a block of data.
pub fn data_md5(data: &[u8]) -> [u8; 16] {
    Md5::digest(data).into()
}

/// Keeps a [`CoreBridgeState`] registered with the C support code, see
/// [`CoreBridgeState::enter_global_engine`].
pub struct GlobalEngineGuard<'a>(PhantomData<&'a mut ()>);

impl Drop for GlobalEngineGuard<'_> {
    fn drop(&mut self) {
        // SAFETY: only resets the C global state slot.
        unsafe { ttbc_global_engine_exit() };
    }
}

lazy_static::lazy_static! {
    static ref ENGINE_LOCK: Mutex<u8> = Mutex::new(0u8);
}

/// An error type indicating the the FFI code aborted.
///
/// FFI bridge callbacks should return this type, which will then be filled in
/// with error text extracted from the global FFI bridge framework.
#[derive(Debug)]
pub struct EngineAbortedError {
    message: String,
}

impl EngineAbortedError {
    /// Create an error indicating that the FFI engine aborted.
    ///
    /// Upon exit, the global bridge FFI framework will report an
    /// error of this same type, but filled in with error text extracted
    /// from the global FFI bridge framework.
    pub fn new_abort_indicator() -> Self {
        EngineAbortedError {
            message: "[failed to extract detailed error message]".to_owned(),
        }
    }

    /// The error with the message the engine's abort left.
    pub fn new_with_details() -> Self {
        // SAFETY: This is always safe to call
        let ptr = unsafe { _ttbc_get_error_message() };
        // SAFETY: The pointer returned above will always have a null-terminated C-string in it
        let message = unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned();
        EngineAbortedError { message }
    }
}

impl Display for EngineAbortedError {
    fn fmt(&self, f: &mut Formatter) -> StdResult<(), FmtError> {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for EngineAbortedError {}

/// A value a [`BridgeHost`] either borrows for a run or owns for a session.
pub enum Held<'h, T: ?Sized> {
    /// Lent by the caller for the host's lifetime.
    Borrowed(&'h mut T),
    /// Owned by the host.
    Owned(Box<T>),
}

impl<T: ?Sized> std::ops::Deref for Held<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        match self {
            Held::Borrowed(value) => value,
            Held::Owned(value) => value,
        }
    }
}

impl<T: ?Sized> std::ops::DerefMut for Held<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        match self {
            Held::Borrowed(value) => value,
            Held::Owned(value) => value,
        }
    }
}

/// A mechanism for launching bridged FFI code.
pub struct CoreBridgeLauncher<'a> {
    hooks: &'a mut dyn DriverHooks,
    status: &'a mut dyn StatusBackend,
    security: SecuritySettings,
    filesystem_emulation_settings: FsEmulationSettings,
}

impl<'a> CoreBridgeLauncher<'a> {
    /// Set up a new context for launching bridged FFI code.
    ///
    /// This function uses the default security stance, which disallows all
    /// known-insecure engine features. Use [`Self::new_with_security`] to
    /// provide your own security settings that can attempt to allow the use of
    /// such features.
    pub fn new(hooks: &'a mut dyn DriverHooks, status: &'a mut dyn StatusBackend) -> Self {
        Self::new_with_security(hooks, status, SecuritySettings::default())
    }

    /// Set up a new context for launching bridged FFI code.
    pub fn new_with_security(
        hooks: &'a mut dyn DriverHooks,
        status: &'a mut dyn StatusBackend,
        security: SecuritySettings,
    ) -> Self {
        CoreBridgeLauncher {
            hooks,
            status,
            security,
            filesystem_emulation_settings: FsEmulationSettings::default(),
        }
    }

    /// While absolute paths are useful (for SyncTeX and external tools that
    /// resolve paths to TeX sources), we can disable them for reproducibility.
    pub fn with_expose_absolute_paths(&mut self, expose_absolute_paths: bool) -> &mut Self {
        self.filesystem_emulation_settings.expose_absolute_paths = expose_absolute_paths;
        self
    }

    /// Ditto for file modification timestamps. In deterministic mode, we return
    /// the configured build time (i.e. `SOURCE_DATE_EPOCH`) instead of the
    /// modification timestamp reported by the IO subsystem.
    pub fn with_mtime_override(&mut self, mtime_override: Option<i64>) -> &mut Self {
        self.filesystem_emulation_settings.mtime_override = mtime_override;
        self
    }

    /// A host over this launcher's hooks and status, for engines that keep
    /// their own state instead of running in [`Self::with_global_lock`].
    pub fn host(&mut self) -> BridgeHost<'_> {
        BridgeHost {
            hooks: Held::Borrowed(&mut *self.hooks),
            status: Held::Borrowed(&mut *self.status),
            security: self.security.clone(),
            fs_emulation_settings: self.filesystem_emulation_settings.clone(),
        }
    }

    /// Invoke a function to launch a bridged FFI engine with a global mutex
    /// held.
    ///
    /// This function *must* be used when invoking C code that makes use of the
    /// global core bridge state functions. Unfortunately, because our error
    /// reporting is based on setjmp/longjmp and it is Undefined Behavior to
    /// setjmp/longjmp across FFI boundaries, we cannot provide a more foolproof
    /// API.
    ///
    /// The invoked code *must* call the functions `ttbc_global_engine_enter`
    /// and `ttbc_global_engine_exit` according to the pattern described in
    /// `texyon_bridge_core.h`. If an abort is detected, the callback function
    /// should return `Err(EngineAbortedError::new_abort_indicator())`.
    pub fn with_global_lock<F, T>(&mut self, callback: F) -> Result<T>
    where
        F: FnOnce(&mut CoreBridgeState<'_, '_>) -> Result<T>,
    {
        let _guard = engine_lock();
        let mut host = self.host();
        let mut state = CoreBridgeState::new(&mut host);
        let result = callback(&mut state);

        if let Err(ref e) = result
            && e.downcast_ref::<EngineAbortedError>().is_some()
        {
            return Err(EngineAbortedError::new_with_details().into());
        }

        result
    }
}

/// Holds the lock every engine run takes, since the C engines keep global state.
pub fn engine_lock() -> std::sync::MutexGuard<'static, u8> {
    ENGINE_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// What engines reach the outside world through: the driver's I/O and events,
/// status reporting, and the security and filesystem-emulation settings.
pub struct BridgeHost<'h> {
    hooks: Held<'h, dyn DriverHooks + 'h>,
    status: Held<'h, dyn StatusBackend + 'h>,
    security: SecuritySettings,
    fs_emulation_settings: FsEmulationSettings,
}

/// The CoreBridgeState structure is a handle to Rust state that can be used by
/// C/C++ engine code to perform basic I/O functions.
///
/// Code that invokes a Tectonic C/C++ engine should pass a pointer to one of
/// these state structures into the C/C++ layer. It is essential that lifetimes
/// be properly managed across the Rust/C boundary.
pub struct CoreBridgeState<'a, 'h> {
    host: &'a mut BridgeHost<'h>,

    input_handles: Vec<Option<InputHandle>>,

    output_handles: Vec<Option<OutputHandle>>,

    /// How [`CoreBridgeState::input_open`] found the latest input.
    latest_input_found: FoundAs,
}

/// How an opened input's name relates to the name it was asked for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FoundAs {
    /// Under the name asked for.
    #[default]
    Asked,
    /// With this extension appended.
    WithExtension(&'static str),
    /// Under this name, which differs from the asked one in the case of its last component.
    CaseFolded(String),
}

impl FoundAs {
    /// The name an input asked as `asked` was found under, as kpathsea returns it.
    pub fn name(&self, asked: &[u8]) -> Vec<u8> {
        match self {
            FoundAs::Asked => asked.to_vec(),
            FoundAs::WithExtension(ext) => [asked, b".", ext.as_bytes()].concat(),
            FoundAs::CaseFolded(folded) => folded.as_bytes().to_vec(),
        }
    }
}

/// An input opened by a [`BridgeHost`].
pub struct OpenedInput {
    /// The open file.
    pub handle: InputHandle,
    /// Its filesystem path, if it has one.
    pub path: Option<PathBuf>,
    /// How its name relates to the one asked for.
    pub found: FoundAs,
}

impl OpenedInput {
    fn found(
        opened: OpenResult<(InputHandle, Option<PathBuf>)>,
        found: FoundAs,
    ) -> OpenResult<OpenedInput> {
        match opened {
            OpenResult::Ok((handle, path)) => OpenResult::Ok(OpenedInput {
                handle,
                path,
                found,
            }),
            OpenResult::NotAvailable => OpenResult::NotAvailable,
            OpenResult::Err(e) => OpenResult::Err(e),
        }
    }
}

impl<'h> BridgeHost<'h> {
    /// A host over `hooks` and `status` with the default filesystem emulation.
    pub fn new(
        hooks: Held<'h, dyn DriverHooks + 'h>,
        status: Held<'h, dyn StatusBackend + 'h>,
        security: SecuritySettings,
    ) -> Self {
        BridgeHost {
            hooks,
            status,
            security,
            fs_emulation_settings: FsEmulationSettings::default(),
        }
    }

    /// The driver hooks.
    pub fn hooks(&mut self) -> &mut (dyn DriverHooks + 'h) {
        &mut *self.hooks
    }

    /// The status backend.
    pub fn status(&mut self) -> &mut (dyn StatusBackend + 'h) {
        &mut *self.status
    }

    /// Issue a warning.
    pub fn issue_warning(&mut self, text: &str) {
        tt_warning!(self.status, "{}", text);
    }

    /// Issue an error.
    pub fn issue_error(&mut self, text: &str) {
        tt_error!(self.status, "{}", text);
    }

    /// Report a finished diagnostic.
    pub fn finish_diagnostic(&mut self, diag: Diagnostic) {
        self.status
            .report(diag.kind, format_args!("{}", diag.message), None);
    }

    /// `path` as engines may see it: `None` unless absolute paths are exposed
    /// and it is UTF-8.
    pub fn exposed_path<'p>(&self, path: Option<&'p std::path::Path>) -> Option<&'p str> {
        if !self.fs_emulation_settings.expose_absolute_paths {
            return None;
        }
        path?.to_str()
    }

    fn input_open_name_format(
        &mut self,
        name: &str,
        format: FileFormat,
    ) -> OpenResult<OpenedInput> {
        let io = self.hooks.io();
        let status = &mut *self.status;

        let open = |io: &mut dyn IoProvider, name: &str, status: &mut dyn StatusBackend| {
            if let FileFormat::Format = format {
                match io.input_open_format(name, status) {
                    OpenResult::Ok(h) => OpenResult::Ok((h, None)),
                    OpenResult::NotAvailable => OpenResult::NotAvailable,
                    OpenResult::Err(e) => OpenResult::Err(e),
                }
            } else {
                io.input_open_name_with_abspath(name, status)
            }
        };

        // A provider may refuse one candidate (an absolute `/dev/null.tex`) and still serve the
        // next, so an error only stands when no candidate is found.
        let mut first_error = None;
        let candidates = format.candidates(name);
        for (candidate, found) in &candidates {
            match open(io, candidate, status) {
                OpenResult::NotAvailable => {}
                OpenResult::Err(e) => {
                    first_error.get_or_insert(e);
                }
                r => return OpenedInput::found(r, found.clone()),
            }
        }

        // Like kpathsea, try the last component in another case once nothing else is found.
        for (candidate, _) in &candidates {
            let Some(folded) = io.input_casefold_name(candidate) else {
                continue;
            };
            match open(io, &folded, status) {
                OpenResult::NotAvailable => {}
                OpenResult::Err(e) => {
                    first_error.get_or_insert(e);
                }
                r => return OpenedInput::found(r, FoundAs::CaseFolded(folded)),
            }
        }

        match first_error {
            Some(e) => OpenResult::Err(e),
            None => OpenResult::NotAvailable,
        }
    }

    fn input_open_name_format_gz(
        &mut self,
        name: &str,
        format: FileFormat,
        is_gz: bool,
    ) -> OpenResult<OpenedInput> {
        let base = self.input_open_name_format(name, format);

        if !is_gz {
            return base;
        }

        match base {
            OpenResult::Ok(opened) => {
                let origin = opened.handle.origin();
                let dr = GzDecoder::new(opened.handle.into_inner());

                OpenResult::Ok(OpenedInput {
                    handle: InputHandle::new(name, dr, origin),
                    ..opened
                })
            }
            _ => base,
        }
    }

    /// Write the MD5 digest of file `name` to `dest`, returning true on failure.
    pub fn get_file_md5(&mut self, name: &str, dest: &mut [u8; 16]) -> bool {
        let name = normalize_tex_path(name);
        let mut hash = Md5::default();

        let mut ih = match self.input_open_name_format(&name, FileFormat::Tex) {
            OpenResult::Ok(opened) => opened.handle,
            // LaTeX's rerun checks ask for missing files all the time, so stay silent.
            OpenResult::NotAvailable => return true,
            OpenResult::Err(e) => {
                tt_error!(self.status, "error trying to open file \"{}\" for MD5 calculation",
                          name; e);
                return true;
            }
        };

        let mut buf = [0u8; 1024];
        let mut error_occurred = false;

        loop {
            let nread = match ih.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    tt_error!(self.status, "error reading file \"{}\" for MD5 calculation",
                              ih.name(); e.into());
                    error_occurred = true;
                    break;
                }
            };
            hash.update(&buf[..nread]);
        }

        let (name, digest_opt) = ih.into_name_digest();
        self.hooks
            .event_input_closed(name, digest_opt, &mut *self.status);

        if !error_occurred {
            let result = hash.finalize();
            dest.copy_from_slice(result.as_slice());
        }

        error_occurred
    }

    /// Whether directory `dir` of the document exists, so that a file can be created in it.
    pub fn directory_exists(&mut self, dir: &str) -> bool {
        self.hooks.io().directory_exists(dir)
    }

    /// Open a new output, provided the output name and whether it is gzipped.
    pub fn open_output(&mut self, name: &str, is_gz: bool) -> Option<OutputHandle> {
        let name = normalize_tex_path(name);

        let mut oh = match self.hooks.io().output_open_name(&name) {
            OpenResult::Ok(oh) => oh,
            OpenResult::NotAvailable => return None,
            OpenResult::Err(e) => {
                tt_warning!(self.status, "open of output {} failed", name; e);
                return None;
            }
        };

        if is_gz {
            let name = oh.name().to_owned();
            oh = OutputHandle::new(
                name,
                GzBuilder::new().write(oh.into_inner(), Compression::default()),
            );
        }

        Some(oh)
    }

    /// Open the terminal output.
    pub fn open_stdout(&mut self) -> Option<OutputHandle> {
        match self.hooks.io().output_open_stdout() {
            OpenResult::Ok(oh) => Some(oh),
            OpenResult::NotAvailable => None,
            OpenResult::Err(e) => {
                tt_warning!(self.status, "open of stdout failed"; e);
                None
            }
        }
    }

    /// Write to an output, returning true on failure.
    pub fn write_output(&mut self, handle: &mut OutputHandle, buf: &[u8]) -> bool {
        match handle.write_all(buf) {
            Ok(_) => false,
            Err(e) => {
                tt_warning!(self.status, "write failed"; e.into());
                true
            }
        }
    }

    /// Flush an output, returning true on failure.
    pub fn flush_output(&mut self, handle: &mut OutputHandle) -> bool {
        match handle.flush() {
            Ok(_) => false,
            Err(e) => {
                tt_warning!(self.status, "flush failed"; e.into());
                true
            }
        }
    }

    /// Close an output, flushing it and reporting its digest. Returns true on failure.
    pub fn close_output(&mut self, mut handle: OutputHandle) -> bool {
        let mut rv = false;
        if let Err(e) = handle.flush() {
            tt_warning!(self.status, "error when closing output {}", handle.name(); e.into());
            rv = true;
        }
        let (name, digest) = handle.into_name_digest();
        self.hooks.event_output_closed(name, digest);
        rv
    }

    /// Open an input, provided its name, the file format, and whether it is gzipped.
    pub fn open_input(
        &mut self,
        name: &str,
        format: FileFormat,
        is_gz: bool,
    ) -> Option<OpenedInput> {
        let name = normalize_tex_path(name);

        match self.input_open_name_format_gz(&name, format, is_gz) {
            OpenResult::Ok(opened) => Some(opened),
            OpenResult::NotAvailable => None,
            OpenResult::Err(e) => {
                tt_warning!(self.status, "open of input {} failed", name; e);
                None
            }
        }
    }

    /// Open the primary input.
    pub fn open_primary_input(&mut self) -> Option<OpenedInput> {
        match self
            .hooks
            .io()
            .input_open_primary_with_abspath(&mut *self.status)
        {
            OpenResult::Ok((handle, path)) => Some(OpenedInput {
                handle,
                path,
                found: FoundAs::Asked,
            }),
            OpenResult::NotAvailable => {
                tt_error!(self.status, "primary input not available (?!)");
                None
            }
            OpenResult::Err(e) => {
                tt_error!(self.status, "open of primary input failed"; e);
                None
            }
        }
    }

    /// The size of an input, or 0 after reporting a failure.
    pub fn input_size(&mut self, handle: &mut InputHandle) -> usize {
        match handle.get_size() {
            Ok(s) => s,
            Err(e) => {
                tt_warning!(self.status, "failed to get the size of an input"; e);
                0
            }
        }
    }

    /// The modification time of an input.
    pub fn input_mtime(&mut self, handle: &mut InputHandle) -> i64 {
        if let Some(mtime) = self.fs_emulation_settings.mtime_override {
            return mtime;
        }
        let maybe_time = match handle.get_unix_mtime() {
            Ok(t) => t,
            Err(e) => {
                tt_warning!(self.status, "failed to get the modification time of an input"; e);
                Some(0)
            }
        };
        maybe_time.unwrap_or(1)
    }

    /// Seek in an input, returning the new position, or 0 after reporting a failure.
    pub fn seek_input(&mut self, handle: &mut InputHandle, pos: SeekFrom) -> u64 {
        match handle.try_seek(pos) {
            Ok(pos) => pos,
            Err(e) => {
                tt_error!(self.status, "input seek failed"; e);
                0
            }
        }
    }

    /// Fill `buf` from an input, reporting a failure as a warning.
    pub fn read_input(&mut self, handle: &mut InputHandle, buf: &mut [u8]) -> Result<()> {
        let result = handle.read_exact(buf).map_err(Error::from);
        if let Err(e) = &result {
            tt_warning!(self.status, "{}-byte read failed", buf.len(); *e);
        }
        result
    }

    /// Read up to `buf.len()` bytes of an input. Returns 0 at its end or after
    /// reporting a failure.
    pub fn read_input_some(&mut self, handle: &mut InputHandle, buf: &mut [u8]) -> usize {
        match handle.read(buf) {
            Ok(n) => n,
            Err(e) => {
                tt_warning!(self.status, "read failed"; e.into());
                0
            }
        }
    }

    /// Close an input, reporting its digest. Returns true on failure.
    pub fn close_input(&mut self, mut handle: InputHandle) -> bool {
        let mut rv = false;

        if let Err(e) = handle.scan_remainder() {
            tt_warning!(self.status, "error closing out input {}", handle.name(); e);
            rv = true;
        }

        let (name, digest_opt) = handle.into_name_digest();
        self.hooks
            .event_input_closed(name, digest_opt, &mut *self.status);
        rv
    }

    /// Run a shell-escape command, returning true on failure.
    pub fn shell_escape(&mut self, command: &str) -> bool {
        if self.security.allow_shell_escape() {
            match self.hooks.sysrq_shell_escape(command, &mut *self.status) {
                Ok(_) => false,
                Err(e) => {
                    tt_error!(
                        self.status,
                        "failed to execute the shell-escape command \"{}\": {}",
                        command,
                        e
                    );
                    true
                }
            }
        } else {
            tt_error!(
                self.status,
                "forbidden to execute shell-escape command \"{}\"",
                command
            );
            true
        }
    }
}

impl<'a, 'h> CoreBridgeState<'a, 'h> {
    /// A state over `host` with no files open.
    pub fn new(host: &'a mut BridgeHost<'h>) -> Self {
        CoreBridgeState {
            host,
            output_handles: Vec::new(),
            input_handles: Vec::new(),
            latest_input_found: FoundAs::Asked,
        }
    }

    /// The host this state reaches the outside world through.
    pub fn host(&mut self) -> &mut BridgeHost<'h> {
        self.host
    }

    /// Makes this state the one C support code reaches through the `ttstub_*`
    /// functions until the guard drops. Rust engines need this only around
    /// calls into C code that does I/O.
    pub fn enter_global_engine(&mut self) -> GlobalEngineGuard<'_> {
        // SAFETY: the guard holds the exclusive borrow while C may use the pointer.
        unsafe { ttbc_global_engine_enter(self) };
        GlobalEngineGuard(PhantomData)
    }

    /// Issue a warning.
    pub fn issue_warning(&mut self, text: &str) {
        self.host.issue_warning(text);
    }

    /// Issue an error.
    pub fn issue_error(&mut self, text: &str) {
        self.host.issue_error(text);
    }

    /// Report a finished diagnostic.
    pub fn finish_diagnostic(&mut self, diag: Diagnostic) {
        self.host.finish_diagnostic(diag);
    }

    /// How the latest input was found.
    pub fn latest_input_found(&self) -> &FoundAs {
        &self.latest_input_found
    }

    /// Write the MD5 digest of file `name` to `dest`, returning true on failure.
    pub fn get_file_md5(&mut self, name: &str, dest: &mut [u8; 16]) -> bool {
        self.host.get_file_md5(name, dest)
    }

    /// Get a mutable reference to an [`OutputHandle`] associated with an [`OutputId`]
    pub fn get_output(&mut self, id: OutputId) -> &mut OutputHandle {
        self.output_handles[id.idx()].as_mut().unwrap()
    }

    /// Whether directory `dir` of the document exists, so that a file can be created in it.
    pub fn directory_exists(&mut self, dir: &str) -> bool {
        self.host.directory_exists(dir)
    }

    fn push_output(&mut self, handle: Option<OutputHandle>) -> Option<OutputId> {
        self.output_handles.push(Some(handle?));
        OutputId::new(self.output_handles.len())
    }

    /// Open a new output, provided the output name and whether it is gzipped.
    pub fn output_open(&mut self, name: &str, is_gz: bool) -> Option<OutputId> {
        let handle = self.host.open_output(name, is_gz);
        self.push_output(handle)
    }

    /// Open a new stdout output.
    pub fn output_open_stdout(&mut self) -> Option<OutputId> {
        let handle = self.host.open_stdout();
        self.push_output(handle)
    }

    /// Write to an output, returning true on failure.
    pub fn output_write(&mut self, id: OutputId, buf: &[u8]) -> bool {
        let handle = self.output_handles[id.idx()].as_mut().unwrap();
        self.host.write_output(handle, buf)
    }

    /// Flush an output, returning true on failure.
    pub fn output_flush(&mut self, id: OutputId) -> bool {
        let handle = self.output_handles[id.idx()].as_mut().unwrap();
        self.host.flush_output(handle)
    }

    /// Close the provided output, flushing it and performing any necessary handling.
    pub fn output_close(&mut self, id: OutputId) -> bool {
        let handle = self.output_handles[id.idx()].take().unwrap();
        self.host.close_output(handle)
    }

    /// Get a mutable reference to an [`InputHandle`] associated with an [`InputId`]
    pub fn get_input(&mut self, input: InputId) -> &mut InputHandle {
        self.input_handles[input.idx()].as_mut().unwrap()
    }

    fn push_input(&mut self, opened: Option<OpenedInput>) -> Option<InputId> {
        let opened = opened?;
        self.input_handles.push(Some(opened.handle));
        self.latest_input_found = opened.found;
        InputId::new(self.input_handles.len())
    }

    /// Open a new input, provided the input name, the file format, and whether it is gzipped.
    pub fn input_open(&mut self, name: &str, format: FileFormat, is_gz: bool) -> Option<InputId> {
        let opened = self.host.open_input(name, format, is_gz);
        self.push_input(opened)
    }

    /// Open the primary input.
    pub fn input_open_primary(&mut self) -> Option<InputId> {
        let opened = self.host.open_primary_input();
        self.push_input(opened)
    }

    /// Get the size of an input. Prints a warning and returns 0 on error
    pub fn input_get_size(&mut self, id: InputId) -> usize {
        let handle = self.input_handles[id.idx()].as_mut().unwrap();
        self.host.input_size(handle)
    }

    /// Get the modification time of an input.
    pub fn input_get_mtime(&mut self, id: InputId) -> i64 {
        let handle = self.input_handles[id.idx()].as_mut().unwrap();
        self.host.input_mtime(handle)
    }

    /// Seek in an input, returning the new position, or 0 after reporting a failure.
    pub fn input_seek(&mut self, id: InputId, pos: SeekFrom) -> u64 {
        let handle = self.input_handles[id.idx()].as_mut().unwrap();
        self.host.seek_input(handle, pos)
    }

    /// Fill `buf` from an input, reporting a failure as a warning.
    pub fn input_read(&mut self, id: InputId, buf: &mut [u8]) -> Result<()> {
        let handle = self.input_handles[id.idx()].as_mut().unwrap();
        self.host.read_input(handle, buf)
    }

    fn input_read_partial(&mut self, handle: InputId, buf: &mut [u8]) -> Result<usize> {
        self.get_input(handle).read(buf).map_err(Error::from)
    }

    /// Get one byte from an input, or `None` at its end or after reporting a failure.
    pub fn input_getc(&mut self, handle: InputId) -> Option<u8> {
        match self.get_input(handle).getc() {
            Ok(b) => Some(b),
            Err(e) => {
                let eof = e
                    .downcast_ref::<io::Error>()
                    .is_some_and(|ioe| ioe.kind() == io::ErrorKind::UnexpectedEof);
                if !eof {
                    tt_warning!(self.host.status, "getc failed"; e);
                }
                None
            }
        }
    }

    /// Put back a byte from [`Self::input_getc`], returning true after reporting a failure.
    pub fn input_ungetc(&mut self, handle: InputId, byte: u8) -> bool {
        match self.get_input(handle).ungetc(byte) {
            Ok(()) => false,
            Err(e) => {
                tt_warning!(self.host.status, "ungetc() failed"; e);
                true
            }
        }
    }

    /// Close the provided input, performing any necessary handling.
    pub fn input_close(&mut self, id: InputId) -> bool {
        let handle = self.input_handles[id.idx()].take().unwrap();
        self.host.close_input(handle)
    }

    /// Run a shell-escape command, returning true on failure.
    pub fn shell_escape(&mut self, command: &str) -> bool {
        self.host.shell_escape(command)
    }
}

/// A type for storing settings about potentially insecure engine features.
///
/// This type encapsulates configuration about which potentially insecure engine
/// features are enabled. Methods that configure or instantiate engines require
/// values of this type, and values of this type can only be created through
/// centralized methods that respect standard environment variables, ensuring
/// that there is some level of uniform control over the activation of any
/// known-insecure features.
///
/// The purpose of this framework is to manage the use of engine features that
/// are known to create security risks with *untrusted* input, but that trusted
/// users may wish to use due to the extra functionalities they bring. (This is
/// why these are settings and not simply security flaws!) The primary example
/// of this is the TeX engine’s shell-escape feature.
///
/// Of course, this framework is only as good as our understanding of Tectonic’s
/// security profile. Future versions might disable or restrict different pieces
/// of functionality as new risks are discovered.
#[derive(Clone, Debug)]
pub struct SecuritySettings {
    /// While we might eventually gain finer-grained enable/disable settings,
    /// there should always be a hard "disable everything known to be risky"
    /// option that supersedes everything else.
    disable_insecures: bool,
}

/// Different high-level security stances that can be adopted when creating
/// [`SecuritySettings`].
#[derive(Clone, Debug, Default)]
pub enum SecurityStance {
    /// Ensure that all known-insecure features are disabled.
    ///
    /// Use this stance if you are processing untrusted input.
    #[default]
    DisableInsecures,

    /// Request to allow the use of known-insecure features.
    ///
    /// Use this stance if you are processing trusted input *and* there is some
    /// user-level request to use such features. The request to allow insecure
    /// features might be overridden if the environment variable
    /// `TEXYON_UNTRUSTED_MODE` is set.
    MaybeAllowInsecures,
}

impl SecuritySettings {
    /// Create a new security configuration.
    ///
    /// The *stance* argument specifies the high-level security stance. If your
    /// program will be run by a trusted user, they should be able to control
    /// the setting through a command-line argument or something comparable.
    /// Even if there is a request to enable known-insecure features, however,
    /// such a request might be overridden by other mechanisms. In particular,
    /// if the environment variable `TEXYON_UNTRUSTED_MODE` is set to any
    /// value, insecure features will always be disabled regardless of the
    /// user-level setting. Other mechanisms for disable known-insecure features
    /// may be added in the future.
    pub fn new(stance: SecurityStance) -> Self {
        let disable_insecures = if std::env::var_os("TEXYON_UNTRUSTED_MODE").is_some() {
            true
        } else {
            match stance {
                SecurityStance::DisableInsecures => true,
                SecurityStance::MaybeAllowInsecures => false,
            }
        };

        SecuritySettings { disable_insecures }
    }

    /// Query whether the shell-escape TeX engine feature is allowed to be used.
    pub fn allow_shell_escape(&self) -> bool {
        !self.disable_insecures
    }

    /// Query whether we're allowed to specify extra paths to read files from.
    pub fn allow_extra_search_paths(&self) -> bool {
        !self.disable_insecures
    }
}

impl Default for SecuritySettings {
    fn default() -> Self {
        SecuritySettings::new(SecurityStance::default())
    }
}

/// A type that stores configuration knobs related to filesystem emulation.
/// These options are not security-critical, but are relevant for
/// reproducible document builds. We default to an "accurate" view of the
/// underlying IO subsystem and have options that stub the respective IO
/// functions with fake / stable values.
#[derive(Clone, Debug)]
struct FsEmulationSettings {
    /// While absolute paths are useful (for SyncTeX and external tools that
    /// resolve paths to TeX sources), we can disable them for reproducibility.
    expose_absolute_paths: bool,

    /// Ditto for file modification timestamps. In deterministic mode, we return
    /// the configured build time (i.e. `SOURCE_DATE_EPOCH`) instead of the
    /// modification timestamp reported by the IO subsystem.
    mtime_override: Option<i64>,
}

impl Default for FsEmulationSettings {
    fn default() -> Self {
        Self {
            expose_absolute_paths: true,
            mtime_override: None,
        }
    }
}

// The entry points.

/// Issue a warning.
///
/// # Safety
///
/// This function is unsafe because it accepts a raw C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ttbc_issue_warning(es: &mut CoreBridgeState, text: *const libc::c_char) {
    // SAFETY: the caller passes a valid NUL-terminated string.
    let text = unsafe { CStr::from_ptr(text) };
    es.issue_warning(&text.to_string_lossy());
}

/// Open a Tectonic file for output.
///
/// # Safety
///
/// This function is unsafe because it accepts a raw C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ttbc_output_open(
    es: &mut CoreBridgeState,
    name: *const libc::c_char,
    is_gz: libc::c_int,
) -> Option<OutputId> {
    // SAFETY: the caller passes a valid NUL-terminated string.
    let rname = unsafe { CStr::from_ptr(name) }.to_string_lossy();
    let ris_gz = is_gz != 0;

    es.output_open(&rname, ris_gz)
}

/// Open the general user output stream as a Tectonic output file.
#[unsafe(no_mangle)]
pub extern "C" fn ttbc_output_open_stdout(es: &mut CoreBridgeState) -> Option<OutputId> {
    es.output_open_stdout()
}

/// Write a single character to a Tectonic output file.
#[unsafe(no_mangle)]
pub extern "C" fn ttbc_output_putc(
    es: &mut CoreBridgeState,
    handle: Option<OutputId>,
    c: libc::c_int,
) -> libc::c_int {
    let rc = c as u8;

    if es.output_write(handle.expect("valid handle"), &[rc]) {
        EOF
    } else {
        c
    }
}

/// Write data to a Tectonic output file.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw C pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ttbc_output_write(
    es: &mut CoreBridgeState,
    handle: Option<OutputId>,
    data: *const u8,
    len: libc::size_t,
) -> libc::size_t {
    // SAFETY: the caller passes `len` readable bytes at `data`.
    let rdata = unsafe { slice::from_raw_parts(data, len) };

    // NOTE: we use f.write_all() so partial writes are not gonna be a thing.

    if es.output_write(handle.expect("valid handle"), rdata) {
        0
    } else {
        len
    }
}

/// Close a Tectonic output file.
#[unsafe(no_mangle)]
pub extern "C" fn ttbc_output_close(
    es: &mut CoreBridgeState,
    handle: Option<OutputId>,
) -> libc::c_int {
    match handle {
        Some(handle) => libc::c_int::from(es.output_close(handle)),
        None => 0, // This is/was the behavior of close_file() in C.
    }
}

/// Open a Tectonic file for input.
///
/// # Safety
///
/// This function is unsafe because it accepts a raw C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ttbc_input_open(
    es: &mut CoreBridgeState,
    name: *const libc::c_char,
    format: FileFormat,
    is_gz: libc::c_int,
) -> Option<InputId> {
    // SAFETY: the caller passes a valid NUL-terminated string.
    let rname = unsafe { CStr::from_ptr(name) }.to_string_lossy();
    let ris_gz = is_gz != 0;
    es.input_open(&rname, format, ris_gz)
}

/// Get the size of a Tectonic input file.
#[unsafe(no_mangle)]
pub extern "C" fn ttbc_input_get_size(
    es: &mut CoreBridgeState,
    handle: Option<InputId>,
) -> libc::size_t {
    es.input_get_size(handle.expect("valid handle"))
}

/// Get the modification time of a Tectonic input file.
#[unsafe(no_mangle)]
pub extern "C" fn ttbc_input_get_mtime(es: &mut CoreBridgeState, handle: Option<InputId>) -> i64 {
    es.input_get_mtime(handle.expect("valid handle"))
}

/// Seek in a Tectonic input stream.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers from C.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ttbc_input_seek(
    es: &mut CoreBridgeState,
    handle: Option<InputId>,
    offset: libc::ssize_t,
    whence: libc::c_int,
    internal_error: *mut libc::c_int,
) -> libc::size_t {
    let rwhence = match whence {
        libc::SEEK_SET => SeekFrom::Start(offset as u64),
        libc::SEEK_CUR => SeekFrom::Current(offset as i64),
        libc::SEEK_END => SeekFrom::End(offset as i64),
        _ => {
            tt_error!(
                es.host.status,
                "serious internal bug: unexpected \"whence\" parameter to fseek() wrapper: {}",
                whence
            );
            // SAFETY: the caller passes a writable error flag.
            unsafe { *internal_error = 1 };
            return 0;
        }
    };

    es.input_seek(handle.expect("valid handle"), rwhence) as libc::size_t
}

/// Get a single character from a Tectonic input file.
#[unsafe(no_mangle)]
pub extern "C" fn ttbc_input_getc(
    es: &mut CoreBridgeState,
    handle: Option<InputId>,
) -> libc::c_int {
    es.input_getc(handle.expect("valid handle"))
        .map_or(EOF, libc::c_int::from)
}

/// Put back a character that was obtained from a `getc` call.
#[unsafe(no_mangle)]
pub extern "C" fn ttbc_input_ungetc(
    es: &mut CoreBridgeState,
    handle: Option<InputId>,
    ch: libc::c_int,
) -> libc::c_int {
    if es.input_ungetc(handle.expect("valid handle"), ch as u8) {
        -1
    } else {
        0
    }
}

/// Read data from a Tectonic input handle
///
/// This read corresponds to Rust's read_exact, i.e. it will return exactly the number of requested
/// bytes or error (-1).
///
/// # Safety
///
/// This function is unsafe because it dereferences raw C pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ttbc_input_read(
    es: &mut CoreBridgeState,
    handle: Option<InputId>,
    data: *mut u8,
    len: libc::size_t,
) -> libc::ssize_t {
    // SAFETY: the caller passes `len` writable bytes at `data`.
    let rdata = unsafe { slice::from_raw_parts_mut(data, len) };

    match es.input_read(handle.expect("valid handle"), rdata) {
        Ok(()) => len as isize,
        Err(_) => -1,
    }
}

/// Read data from a Tectonic input handle
///
/// This read corresponds to Rust's read, i.e. it can return less bytes than requested (and does
/// when buffering)
///
/// # Safety
///
/// This function is unsafe because it dereferences raw C pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ttbc_input_read_partial(
    es: &mut CoreBridgeState,
    handle: Option<InputId>,
    data: *mut u8,
    len: libc::size_t,
) -> libc::ssize_t {
    // SAFETY: the caller passes `len` writable bytes at `data`.
    let rdata = unsafe { slice::from_raw_parts_mut(data, len) };

    match es.input_read_partial(handle.expect("valid handle"), rdata) {
        Ok(size) => size as isize,
        Err(e) => {
            tt_warning!(es.host.status, "{}-byte read failed", len; e);
            -1
        }
    }
}

/// Close a Tectonic input file.
#[unsafe(no_mangle)]
pub extern "C" fn ttbc_input_close(
    es: &mut CoreBridgeState,
    handle: Option<InputId>,
) -> libc::c_int {
    match handle {
        Some(handle) => libc::c_int::from(es.input_close(handle)),
        None => 0,
    }
}

/// A buffer for a diagnostic message, reported with
/// [`CoreBridgeState::finish_diagnostic`].
#[derive(Clone, Debug)]
pub struct Diagnostic {
    message: String,
    kind: MessageKind,
}

impl Diagnostic {
    /// A new diagnostic that will be reported as a warning.
    pub fn warning() -> Diagnostic {
        Diagnostic {
            message: String::new(),
            kind: MessageKind::Warning,
        }
    }

    /// A new diagnostic that will be reported as an error.
    pub fn error() -> Diagnostic {
        Diagnostic {
            message: String::new(),
            kind: MessageKind::Error,
        }
    }

    /// Append text to the message.
    pub fn append(&mut self, text: &str) {
        self.message.push_str(text);
    }
}

/// Different types of files that can be opened by TeX engines
///
/// This enumeration is used to guess filename extensions to try when looking
/// for a file to open.
///
/// cbindgen:rename-all=ScreamingSnakeCase
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum FileFormat {
    /// An Adobe Font Metrics file.
    Afm = 4,

    /// A BibTeX bibliography data file.
    Bib = 6,

    /// A BibTeX style file.
    Bst = 7,

    /// A character map data file.
    Cmap = 45,

    /// A configuration file.
    Cnf = 8,

    /// An encoding data file.
    Enc = 44,

    /// A TeX "format" file.
    Format = 10,

    /// A font-map file.
    FontMap = 11,

    /// A miscellaneous font file.
    MiscFonts = 41,

    /// An OFM font metrics file.
    Ofm = 20,

    /// An OpenType font file.
    OpenType = 47,

    /// An OVF file.
    Ovf = 23,

    /// An image file.
    Pict = 25,

    /// A PK font file.
    Pk = 1,

    /// A general program data file.
    ProgramData = 39,

    /// An SFD file.
    Sfd = 46,

    /// The Tectonic primary input file.
    TexyonPrimary = 59,

    /// A TeX language file.
    Tex = 26,

    /// A TeX PostScript header file.
    TexPsHeader = 30,

    /// A TeX Font Metrics file.
    Tfm = 3,

    /// A TrueType font file.
    TrueType = 36,

    /// A Type1 font file.
    Type1 = 32,

    /// A Virtual Font file.
    Vf = 33,
}

impl FileFormat {
    /// The names kpathsea tries for `name`, in order: as given when it already ends in
    /// one of the format's suffixes, else with each suffix added and then as given.
    fn candidates(&self, name: &str) -> Vec<(String, FoundAs)> {
        let suffixed = self
            .extensions()
            .iter()
            .chain(self.alternate_extensions())
            .any(|e| name.strip_suffix(e).is_some_and(|stem| stem.ends_with('.')));
        let mut out = Vec::new();
        if !suffixed {
            for &e in self.extensions() {
                out.push((format!("{name}.{e}"), FoundAs::WithExtension(e)));
            }
        }
        out.push((name.to_owned(), FoundAs::Asked));
        out
    }

    /// Suffixes that mark a name as complete without being added to one.
    fn alternate_extensions(&self) -> &'static [&'static str] {
        match *self {
            FileFormat::Tex => &["sty", "cls", "fd", "aux", "bbl", "def", "clo", "ldf"],
            FileFormat::Pict => &["eps", "epsi"],
            _ => &[],
        }
    }

    fn extensions(&self) -> &'static [&'static str] {
        match *self {
            FileFormat::Afm => &["afm"],
            FileFormat::Bib => &["bib"],
            FileFormat::Bst => &["bst"],
            FileFormat::Cmap => &[],
            FileFormat::Cnf => &["cnf"],
            FileFormat::Enc => &["enc"],
            FileFormat::Format => &["fmt"],
            FileFormat::FontMap => &["map"],
            FileFormat::MiscFonts => &[],
            FileFormat::Ofm => &["ofm"],
            FileFormat::OpenType => &["otf", "OTF"],
            FileFormat::Ovf => &["ovf", "vf"],
            FileFormat::Pict => &[],
            FileFormat::Pk => &["pk"],
            FileFormat::ProgramData => &[],
            FileFormat::Sfd => &["sfd"],
            FileFormat::TexyonPrimary => &[],
            FileFormat::Tex => &["tex"],
            FileFormat::TexPsHeader => &["pro"],
            FileFormat::Tfm => &["tfm"],
            FileFormat::TrueType => &["ttf", "ttc", "TTF", "TTC", "dfont"],
            FileFormat::Type1 => &["pfa", "pfb"],
            FileFormat::Vf => &["vf"],
        }
    }
}

#[test]
fn candidates_follow_kpathsea_suffix_rules() {
    let names = |format: FileFormat, name: &str| -> Vec<String> {
        format
            .candidates(name)
            .into_iter()
            .map(|(n, _)| n)
            .collect()
    };
    assert_eq!(
        names(FileFormat::Tex, "chapter"),
        ["chapter.tex", "chapter"]
    );
    assert_eq!(
        names(FileFormat::Tex, "lipsum.ltd"),
        ["lipsum.ltd.tex", "lipsum.ltd"]
    );
    assert_eq!(names(FileFormat::Tex, "article.cls"), ["article.cls"]);
    assert_eq!(names(FileFormat::Tex, "main.tex"), ["main.tex"]);
    assert_eq!(names(FileFormat::Tex, "xtex"), ["xtex.tex", "xtex"]);
    assert_eq!(names(FileFormat::Pict, "fig.png"), ["fig.png"]);
    assert_eq!(
        names(FileFormat::OpenType, "lmroman12-regular"),
        [
            "lmroman12-regular.otf",
            "lmroman12-regular.OTF",
            "lmroman12-regular"
        ]
    );
}

#[test]
fn an_error_on_one_candidate_does_not_hide_the_next() {
    use texyon_io_base::{MemoryIo, stack::IoStack};
    use texyon_status_base::NoopStatusBackend;

    struct Refuses;
    impl IoProvider for Refuses {
        fn input_open_name(
            &mut self,
            _: &str,
            _: &mut dyn StatusBackend,
        ) -> OpenResult<InputHandle> {
            OpenResult::Err(anyhow!("refused"))
        }
    }

    let mut mem = MemoryIo::new(false);
    mem.create_entry("doc", b"x".to_vec());
    let mut refuses = Refuses;
    let mut hooks = MinimalDriver::new(IoStack::new(vec![
        Box::new(&mut mem),
        Box::new(&mut refuses),
    ]));
    let mut status = NoopStatusBackend::default();
    let mut host = BridgeHost::new(
        Held::Borrowed(&mut hooks),
        Held::Borrowed(&mut status),
        SecuritySettings::default(),
    );
    let mut state = CoreBridgeState::new(&mut host);
    assert!(state.input_open("doc", FileFormat::Tex, false).is_some());
    assert_eq!(state.latest_input_found(), &FoundAs::Asked);
    assert!(state.input_open("other", FileFormat::Tex, false).is_none());
}

/// Does our resulting executable link correctly?
#[test]
fn linkage() {}
