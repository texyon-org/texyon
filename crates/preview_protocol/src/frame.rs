//! Messages over a byte stream: each frame is a little-endian `u32` length,
//! then a MessagePack map.

use std::{
    fmt,
    io::{self, Read, Write},
};

use serde::{Serialize, de::DeserializeOwned};

/// Refused before allocating, so a corrupt length cannot exhaust memory.
pub const MAX_FRAME_BYTES: u32 = 256 * 1024 * 1024;

/// Failure of the stream, after which no frame can be trusted.
#[derive(Debug)]
pub enum FrameError {
    /// Reading or writing failed.
    Io(io::Error),
    /// A frame was longer than [`MAX_FRAME_BYTES`].
    TooLarge(u64),
    /// A message could not be encoded.
    Encode(rmp_serde::encode::Error),
    /// A frame did not hold the expected message.
    Decode(rmp_serde::decode::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "preview stream failed: {error}"),
            Self::TooLarge(len) => write!(f, "preview frame of {len} bytes is too large"),
            Self::Encode(error) => write!(f, "could not encode a preview message: {error}"),
            Self::Decode(error) => write!(f, "malformed preview message: {error}"),
        }
    }
}

impl std::error::Error for FrameError {}

impl From<io::Error> for FrameError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Read one frame's bytes, or `None` when the stream ends between frames.
pub fn read_frame<R: Read>(input: &mut R) -> Result<Option<Vec<u8>>, FrameError> {
    let mut header = [0; 4];
    let mut filled = 0;
    while filled < header.len() {
        match input.read(&mut header[filled..]) {
            Ok(0) if filled == 0 => return Ok(None),
            Ok(0) => return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into()),
            Ok(read) => filled += read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error.into()),
        }
    }
    let len = u32::from_le_bytes(header);
    if len > MAX_FRAME_BYTES {
        return Err(FrameError::TooLarge(len.into()));
    }
    let mut frame = vec![0; usize::try_from(len).map_err(|_| FrameError::TooLarge(len.into()))?];
    input.read_exact(&mut frame)?;
    Ok(Some(frame))
}

/// Decode one frame's bytes.
pub fn decode<T: DeserializeOwned>(frame: &[u8]) -> Result<T, FrameError> {
    rmp_serde::from_slice(frame).map_err(FrameError::Decode)
}

/// Read and decode one message, or `None` when the stream ends between frames.
pub fn read_message<R: Read, T: DeserializeOwned>(input: &mut R) -> Result<Option<T>, FrameError> {
    read_frame(input)?.map(|frame| decode(&frame)).transpose()
}

/// Write `message` as one frame and flush it.
pub fn write_message<W: Write, T: Serialize>(
    output: &mut W,
    message: &T,
) -> Result<(), FrameError> {
    let body = rmp_serde::to_vec_named(message).map_err(FrameError::Encode)?;
    let len = u32::try_from(body.len())
        .ok()
        .filter(|len| *len <= MAX_FRAME_BYTES)
        .ok_or(FrameError::TooLarge(
            body.len().try_into().unwrap_or(u64::MAX),
        ))?;
    output.write_all(&len.to_le_bytes())?;
    output.write_all(&body)?;
    output.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{MAX_FRAME_BYTES, read_frame, read_message, write_message};
    use crate::session::{HostRequest, InlineBytes, Request, Response};

    #[test]
    fn frames_round_trip_with_inline_bytes() {
        let mut wire = Vec::new();
        let sent = Response::Exported(InlineBytes {
            data: vec![0, 159, 255],
        });
        write_message(&mut wire, &sent).unwrap();
        assert_eq!(
            u32::from_le_bytes(wire[..4].try_into().unwrap()),
            u32::try_from(wire.len() - 4).unwrap()
        );

        let received: Response<InlineBytes> =
            read_message(&mut Cursor::new(wire)).unwrap().unwrap();
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
        let mut wire = Vec::new();
        write_message(&mut wire, &open).unwrap();
        assert_eq!(
            read_message::<_, Request<InlineBytes>>(&mut Cursor::new(wire)).unwrap(),
            Some(open)
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
