//! A preview session served over a byte stream, for a native host.
//!
//! Each frame is a little-endian `u32` length and then a MessagePack map. The
//! server answers requests in order, one response each. `cancel` is the
//! exception: it stops the running operation at once and gets no reply, so a
//! client sends it only while waiting on the operation it means to stop.

use std::{
    fmt,
    io::{Read, Write},
    sync::{Arc, Mutex, PoisonError, mpsc},
    thread,
};

use texyon_engine_xetex::EngineCancellation;
use texyon_errors::{Result, anyhow::anyhow};
use texyon_preview_protocol::{
    frame::{self, FrameError},
    session::{HostRequest, InlineBytes, Request, Response, SessionRequest},
};
use texyon_preview_render::{Backend, ResourceProvider};

use crate::{DocumentRoot, Payload, PreviewSession, SessionHost, handle};

const WORKER_STACK_BYTES: usize = 32 * 1024 * 1024;

impl Payload for InlineBytes {
    fn hand_over(bytes: Vec<u8>) -> Self {
        InlineBytes { data: bytes }
    }
}

/// What a client asked for when it opened the session.
pub struct OpenRequest {
    /// Format name.
    pub format: String,
    /// The primary source, relative to `root`.
    pub primary: String,
    /// The project directory.
    pub root: String,
    /// Whether to draw pages to pixels.
    pub raster: bool,
    /// Whether a compile may send only the changed part of a page.
    pub patches: bool,
    /// Wall-clock seconds since the epoch for `\today`, or `None` for 1970.
    pub build_date: Option<u64>,
}

/// Sources the session does not hold, read from the project directory.
pub struct DiskSources {
    root: DocumentRoot,
}

impl DiskSources {
    /// Read sources from inside `root`.
    pub fn new(root: DocumentRoot) -> Self {
        Self { root }
    }
}

impl SessionHost for DiskSources {
    fn read_source(&mut self, path: &str) -> Result<Option<Vec<u8>>> {
        self.root
            .read(path)
            .map_err(|error| anyhow!("could not read {path}: {error}"))
    }
}

/// Failure of the stream itself, after which no reply can be trusted.
#[derive(Debug)]
pub enum ServeError {
    /// Reading or writing a frame failed.
    Frame(FrameError),
    /// The session thread could not start or panicked.
    Worker,
}

impl fmt::Display for ServeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Frame(error) => error.fmt(f),
            Self::Worker => write!(f, "the preview session thread failed"),
        }
    }
}

impl std::error::Error for ServeError {}

impl From<FrameError> for ServeError {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}

type Incoming = std::result::Result<Request<InlineBytes>, String>;

/// Serve one session until the client shuts it down or closes `input`.
///
/// `open` runs on the session's own thread, since the engine keeps
/// thread-bound state.
pub fn serve<R, W, F, P, B, H>(mut input: R, output: W, open: F) -> Result<(), ServeError>
where
    R: Read,
    W: Write + Send + 'static,
    F: FnMut(OpenRequest) -> Result<PreviewSession<P, B, H>> + Send + 'static,
    P: ResourceProvider + 'static,
    B: Backend + 'static,
    H: SessionHost,
{
    let cancellation: Arc<Mutex<Option<EngineCancellation>>> = Arc::default();
    let (requests, incoming) = mpsc::channel::<Incoming>();

    let worker_cancellation = Arc::clone(&cancellation);
    let worker = thread::Builder::new()
        .name("texyon-preview".into())
        .stack_size(WORKER_STACK_BYTES)
        .spawn(move || run_session(incoming, output, open, &worker_cancellation))
        .map_err(|_| ServeError::Worker)?;

    let read = read_requests(&mut input, &requests, &cancellation);
    drop(requests);
    let served = worker.join().map_err(|_| ServeError::Worker)?;
    read.and(served)
}

fn read_requests<R: Read>(
    input: &mut R,
    requests: &mpsc::Sender<Incoming>,
    cancellation: &Mutex<Option<EngineCancellation>>,
) -> Result<(), ServeError> {
    while let Some(frame) = frame::read_frame(input)? {
        let request = frame::decode::<Request<InlineBytes>>(&frame)
            .map_err(|error| format!("malformed request: {error}"));
        if let Ok(Request::Session(SessionRequest::Cancel)) = request {
            let held = cancellation.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(cancellation) = held.as_ref() {
                cancellation.request();
            }
            continue;
        }
        let shutdown = matches!(request, Ok(Request::Host(HostRequest::Shutdown)));
        if requests.send(request).is_err() || shutdown {
            break;
        }
    }
    Ok(())
}

fn run_session<W, F, P, B, H>(
    incoming: mpsc::Receiver<Incoming>,
    mut output: W,
    mut open: F,
    cancellation: &Mutex<Option<EngineCancellation>>,
) -> Result<(), ServeError>
where
    W: Write,
    F: FnMut(OpenRequest) -> Result<PreviewSession<P, B, H>>,
    P: ResourceProvider + 'static,
    B: Backend + 'static,
    H: SessionHost,
{
    let mut session: Option<PreviewSession<P, B, H>> = None;

    for request in incoming {
        let (response, done): (Response<InlineBytes>, bool) = match request {
            Err(message) => (Response::error(message), false),
            Ok(Request::Host(HostRequest::Open {
                format,
                primary,
                root,
                raster,
                patches,
                build_date,
            })) => match (&session, root) {
                (Some(_), _) => (Response::error("a session is already open"), false),
                (None, None) => (Response::error("a native session needs a root"), false),
                (None, Some(root)) => match open(OpenRequest {
                    format,
                    primary,
                    root,
                    raster,
                    patches,
                    build_date,
                }) {
                    Ok(opened) => {
                        *cancellation.lock().unwrap_or_else(PoisonError::into_inner) =
                            Some(opened.cancellation());
                        session = Some(opened);
                        (Response::Ok, false)
                    }
                    Err(error) => (Response::error(format!("{error:#}")), false),
                },
            },
            Ok(Request::Host(HostRequest::Preload { .. })) => (
                Response::error("a native session reads its own support files"),
                false,
            ),
            Ok(Request::Host(HostRequest::Shutdown)) => {
                let finished = match session.take().map(|mut open| open.finish()) {
                    Some(Err(error)) => Response::error(error),
                    Some(Ok(())) | None => Response::Ok,
                };
                (finished, true)
            }
            Ok(Request::Session(request)) => match session.as_mut() {
                Some(open) => (handle(open, request), false),
                None => (Response::error("no session is open"), false),
            },
        };

        frame::write_message(&mut output, &response)?;
        if done {
            return Ok(());
        }
    }

    if let Some(mut open) = session {
        // The client is gone, so there is nobody to report a failure to.
        let _ = open.finish();
    }
    Ok(())
}
