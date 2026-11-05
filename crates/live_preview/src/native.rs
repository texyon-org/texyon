//! A preview session served over a byte stream, for a native host.
//!
//! Each frame is a little-endian `u32` length and then a MessagePack map. The
//! server answers requests in order, one response each. `cancel` is the
//! exception: it stops the running operation at once and gets no reply, so a
//! client sends it only while waiting on the operation it means to stop.

use std::{
    fmt,
    io::{self, Read, Write},
    sync::{mpsc, Arc, Mutex, PoisonError},
    thread,
};

use texyon_engine_xetex::EngineCancellation;
use texyon_errors::{anyhow::anyhow, Result};
use texyon_preview_protocol::session::{
    HostRequest, InlineBytes, Request, Response, SessionRequest,
};
use texyon_preview_render::{Backend, ResourceProvider};

use crate::{handle, DocumentRoot, Payload, PreviewSession, SessionHost};

const WORKER_STACK_BYTES: usize = 32 * 1024 * 1024;

/// Refused before allocating, so a corrupt length cannot exhaust memory.
const MAX_FRAME_BYTES: u32 = 256 * 1024 * 1024;

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
    Io(io::Error),
    /// A frame was longer than [`MAX_FRAME_BYTES`].
    FrameTooLarge(u32),
    /// A response could not be encoded.
    Encode(rmp_serde::encode::Error),
    /// The session thread could not start or panicked.
    Worker,
}

impl fmt::Display for ServeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "preview stream failed: {error}"),
            Self::FrameTooLarge(len) => write!(f, "preview frame of {len} bytes is too large"),
            Self::Encode(error) => write!(f, "could not encode a preview response: {error}"),
            Self::Worker => write!(f, "the preview session thread failed"),
        }
    }
}

impl std::error::Error for ServeError {}

impl From<io::Error> for ServeError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
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
    while let Some(frame) = read_frame(input)? {
        let request = rmp_serde::from_slice::<Request<InlineBytes>>(&frame)
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
        let (response, done) = match request {
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

        write_frame(&mut output, &response)?;
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

fn read_frame<R: Read>(input: &mut R) -> Result<Option<Vec<u8>>, ServeError> {
    let mut header = [0; 4];
    let mut filled = 0;
    while filled < header.len() {
        match input.read(&mut header[filled..]) {
            // End of stream is clean only between frames.
            Ok(0) if filled == 0 => return Ok(None),
            Ok(0) => return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into()),
            Ok(read) => filled += read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error.into()),
        }
    }
    let len = u32::from_le_bytes(header);
    if len > MAX_FRAME_BYTES {
        return Err(ServeError::FrameTooLarge(len));
    }
    let mut frame = vec![0; usize::try_from(len).map_err(|_| ServeError::FrameTooLarge(len))?];
    input.read_exact(&mut frame)?;
    Ok(Some(frame))
}

fn write_frame<W: Write>(
    output: &mut W,
    response: &Response<InlineBytes>,
) -> Result<(), ServeError> {
    let body = rmp_serde::to_vec_named(response).map_err(ServeError::Encode)?;
    let len = u32::try_from(body.len())
        .ok()
        .filter(|len| *len <= MAX_FRAME_BYTES)
        .ok_or(ServeError::FrameTooLarge(u32::MAX))?;
    output.write_all(&len.to_le_bytes())?;
    output.write_all(&body)?;
    output.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use texyon_preview_protocol::session::{HostRequest, InlineBytes, Request, Response};

    use super::{read_frame, write_frame, MAX_FRAME_BYTES};

    #[test]
    fn frames_round_trip_with_inline_bytes() {
        let mut wire = Vec::new();
        let sent = Response::Exported(InlineBytes {
            data: vec![0, 159, 255],
        });
        write_frame(&mut wire, &sent).unwrap();
        assert_eq!(
            u32::from_le_bytes(wire[..4].try_into().unwrap()),
            u32::try_from(wire.len() - 4).unwrap()
        );

        let frame = read_frame(&mut Cursor::new(wire)).unwrap().unwrap();
        let received: Response<InlineBytes> = rmp_serde::from_slice(&frame).unwrap();
        assert_eq!(received, sent);
    }

    #[test]
    fn requests_route_by_op() {
        let open = Request::<InlineBytes>::Host(HostRequest::Open {
            format: "latex".into(),
            primary: "main.tex".into(),
            root: Some("/doc".into()),
            raster: true,
            patches: true,
            build_date: Some(1_700_000_000),
        });
        let encoded = rmp_serde::to_vec_named(&open).unwrap();
        assert_eq!(
            rmp_serde::from_slice::<Request<InlineBytes>>(&encoded).unwrap(),
            open
        );

        let compile = br#"{"op":"compile","files":{"main.tex":3},"viewport":[0,1]}"#;
        let parsed: Request<InlineBytes> = serde_json::from_slice(compile).unwrap();
        assert!(matches!(parsed, Request::Session(_)));
    }

    #[test]
    fn end_of_stream_between_frames_is_clean() {
        assert_eq!(read_frame(&mut Cursor::new(Vec::new())).unwrap(), None);
        assert!(read_frame(&mut Cursor::new(vec![1, 0])).is_err());
    }

    #[test]
    fn oversized_lengths_are_refused_before_allocating() {
        let header = (MAX_FRAME_BYTES + 1).to_le_bytes();
        assert!(read_frame(&mut Cursor::new(header.to_vec())).is_err());
    }
}
