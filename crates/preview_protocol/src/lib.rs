//! Versioned, bounded JSON Lines messages for Tectonic live preview.
//!
//! Sequence counters are encoded as canonical decimal strings so JavaScript
//! clients never route them through an imprecise JSON number.

use std::{collections::BTreeMap, fmt, io::BufRead, str::FromStr};

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

/// Initial clean protocol version.
pub const PROTOCOL_VERSION: u32 = 1;

/// Explicit decoded-source, wire-line, and frame limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtocolLimits {
    /// Maximum UTF-8 bytes in any one source buffer.
    pub max_source_file_bytes: usize,
    /// Maximum combined UTF-8 bytes in an initialized source snapshot.
    pub max_source_snapshot_bytes: usize,
    /// Maximum encoded bytes in one physical JSON line, excluding newline.
    pub max_json_line_bytes: usize,
    /// Maximum encoded bytes in one server frame, excluding newline.
    pub max_frame_bytes: usize,
}

impl Default for ProtocolLimits {
    fn default() -> Self {
        const MIB: usize = 1024 * 1024;
        let max_source_snapshot_bytes = 64 * MIB;
        Self {
            max_source_file_bytes: 16 * MIB,
            max_source_snapshot_bytes,
            // JSON may encode each control byte as six ASCII bytes. Leave a
            // separate envelope allowance for paths and child descriptors.
            max_json_line_bytes: max_source_snapshot_bytes * 6 + 16 * MIB,
            max_frame_bytes: 64 * MIB,
        }
    }
}

/// A preview sequence counter cannot advance beyond its exact `u64` range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SequenceExhausted {
    kind: &'static str,
}

impl SequenceExhausted {
    /// Name of the exhausted counter.
    pub fn kind(self) -> &'static str {
        self.kind
    }
}

impl fmt::Display for SequenceExhausted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} counter exhausted", self.kind)
    }
}

impl std::error::Error for SequenceExhausted {}

macro_rules! sequence_type {
    ($name:ident, $description:literal, $kind:literal) => {
        #[doc = $description]
        #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(u64);

        impl $name {
            /// The initial counter value.
            pub const ZERO: Self = Self(0);

            /// Construct a counter from its exact integer value.
            pub const fn new(value: u64) -> Self {
                Self(value)
            }

            /// Return the exact integer value.
            pub const fn get(self) -> u64 {
                self.0
            }

            /// Advance exactly once, failing instead of wrapping.
            pub fn checked_next(self) -> Result<Self, SequenceExhausted> {
                self.0
                    .checked_add(1)
                    .map(Self)
                    .ok_or(SequenceExhausted { kind: $kind })
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl FromStr for $name {
            type Err = ParseSequenceError;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                if text.is_empty()
                    || (text.len() > 1 && text.starts_with('0'))
                    || !text.bytes().all(|byte| byte.is_ascii_digit())
                {
                    return Err(ParseSequenceError);
                }
                text.parse::<u64>()
                    .map(Self)
                    .map_err(|_| ParseSequenceError)
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.collect_str(self)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let text = <&str>::deserialize(deserializer)?;
                text.parse().map_err(|_| {
                    de::Error::custom(concat!("expected canonical decimal string for ", $kind))
                })
            }
        }
    };
}

/// A sequence string was non-canonical or outside the exact `u64` range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParseSequenceError;

impl fmt::Display for ParseSequenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid canonical decimal sequence")
    }
}

impl std::error::Error for ParseSequenceError {}

sequence_type!(
    RequestSeq,
    "Sequence assigned to every well-formed mutating request.",
    "request"
);
sequence_type!(
    SourceRevision,
    "Revision assigned to every accepted source replacement.",
    "source revision"
);
sequence_type!(
    FrameId,
    "Process-local identity assigned to each coherent published frame.",
    "frame"
);
sequence_type!(
    QueryId,
    "Client-assigned identity for one source-navigation query.",
    "query"
);
sequence_type!(
    CommandCount,
    "Exact number of engine commands replayed by one operation.",
    "command count"
);

/// Complete source buffer sent during initialization.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct SourceFile {
    /// Logical document path.
    pub path: String,
    /// Complete UTF-8 contents.
    pub text: String,
}

/// Requests accepted on protocol stdin.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientRequest {
    /// Establish the initial immutable source snapshot.
    Initialize {
        /// Protocol version echoed from `ready`.
        protocol_version: u32,
        /// Logical primary document path.
        primary_path: String,
        /// Complete primary document contents.
        primary_text: String,
        /// Complete already-dirty eligible child buffers.
        children: Vec<SourceFile>,
    },
    /// Replace one complete source buffer.
    Replace {
        /// Logical child path, or primary when omitted.
        path: Option<String>,
        /// Complete replacement contents.
        text: String,
    },
    /// Report the true visible page range.
    Viewport {
        /// First visible zero-based page.
        first: usize,
        /// Last visible zero-based page, inclusive.
        last: usize,
    },
    /// Request a full replay of retained server belief.
    Resync {},
    /// Durably stop compilation until an accepted mutation.
    Cancel {},
    /// Report viewer paint telemetry.
    Painted {
        /// Frame retained and painted by the viewer.
        frame_id: FrameId,
        /// Measured paint duration in milliseconds.
        paint_ms: f64,
    },
    /// Locate preview geometry for one source line.
    ForwardSearch {
        /// Client-assigned query identity.
        query_id: QueryId,
        /// Primary or child source path.
        path: String,
        /// One-based source line.
        line: usize,
    },
    /// Locate the source nearest one preview point.
    BackwardSearch {
        /// Client-assigned query identity.
        query_id: QueryId,
        /// Zero-based output page.
        page_index: usize,
        /// Horizontal page coordinate in PostScript big points.
        h_bp: f64,
        /// Vertical page coordinate in PostScript big points.
        v_bp: f64,
    },
    /// Stop the worker and process cleanly.
    Shutdown {},
}

/// Rendering phase represented by a coherent frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FramePhase {
    /// Immediate foreground response to a source operation.
    Foreground,
    /// Later completion at the same source revision.
    TailSettle,
    /// Full replay of retained belief after a client reload.
    Resync,
}

/// High-level result carried by an update.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateOutcome {
    /// Compilation and rendering completed coherently.
    Complete,
    /// A coherent last-good frame accompanies a recoverable source error.
    Error,
}

/// SVG payload for one page.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct PageSvgPayload {
    /// Zero-based page index.
    pub page_index: usize,
    /// Self-contained SVG markup.
    pub svg: String,
}

/// Page dimensions in PostScript big points.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub struct PageSize {
    /// Width in big points.
    pub width_bp: f32,
    /// Height in big points.
    pub height_bp: f32,
}

/// Unsupported renderer operations observed on one page.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct PageUnsupportedPayload {
    /// Zero-based page index.
    pub page_index: usize,
    /// Stable operation name to occurrence count.
    pub operations: BTreeMap<String, u64>,
}

/// Per-operation timing and payload metrics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct OperationMetrics {
    /// Engine duration in milliseconds.
    pub engine_ms: f64,
    /// Renderer analysis duration in milliseconds.
    pub analysis_ms: f64,
    /// Selected-page rendering duration in milliseconds.
    pub render_ms: f64,
    /// Encoded frame bytes before the JSONL newline.
    pub frame_bytes: usize,
    /// Retained checkpoint rungs after the engine operation.
    pub checkpoint_rungs: usize,
    /// Accounted retained checkpoint bytes after the engine operation.
    pub checkpoint_bytes: usize,
    /// Fresh pages shipped by the engine operation.
    pub shipouts: usize,
    /// Whether the operation reused an old XDV prefix from a replay fence.
    pub reused_prefix: bool,
    /// Checkpoint rung selected as the operation's replay origin.
    pub replay_rung: usize,
    /// Engine commands executed after the selected replay origin.
    pub replay_commands: CommandCount,
    /// Checkpoint candidates admitted during the operation.
    pub checkpoint_rungs_admitted: usize,
    /// Older checkpoint rungs dropped during admission.
    pub checkpoint_rungs_dropped: usize,
    /// Checkpoint candidates rejected by the hard byte cap.
    pub checkpoint_rungs_rejected: usize,
    /// Pages fingerprinted during renderer analysis.
    pub pages_fingerprinted: usize,
    /// Pages rendered to new SVG payloads.
    pub pages_rendered: usize,
    /// Pages reporting one or more unsupported renderer operations.
    pub unsupported_pages: usize,
}

/// Preview rectangle returned by a forward SyncTeX query.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub struct SyncRectPayload {
    /// Zero-based output page.
    pub page_index: usize,
    /// Left edge in PostScript big points.
    pub h_bp: f64,
    /// Baseline in PostScript big points.
    pub v_bp: f64,
    /// Width in PostScript big points.
    pub width_bp: f64,
    /// Height above the baseline in PostScript big points.
    pub height_bp: f64,
    /// Depth below the baseline in PostScript big points.
    pub depth_bp: f64,
}

/// Source location returned by a backward SyncTeX query.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct SourceLocation {
    /// Primary or child source name.
    pub path: String,
    /// One-based source line.
    pub line: usize,
}

/// Events emitted on protocol stdout.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
// `Update` owns one complete atomic viewer patch. Boxing its fields would add
// public API indirection without reducing the serialized or retained payload.
#[allow(clippy::large_enum_variant)]
pub enum ServerEvent {
    /// Bootstrap completed and initialization may begin.
    Ready {
        /// Supported protocol version.
        protocol_version: u32,
        /// Stable capability names.
        capabilities: Vec<String>,
        /// Digest of the frozen engine/renderer bundle universe.
        bundle_digest: String,
    },
    /// One coherent atomic viewer patch.
    Update {
        /// Strictly increasing process-local frame identity.
        frame_id: FrameId,
        /// Accepted source revision rendered by this frame.
        source_revision: SourceRevision,
        /// Latest request applied before compilation.
        applied_request_seq: RequestSeq,
        /// Foreground, tail-settle, or resync phase.
        phase: FramePhase,
        /// High-level operation result.
        outcome: UpdateOutcome,
        /// Complete page count after applying this patch.
        page_count: usize,
        /// Added or changed page indices.
        changed: Vec<usize>,
        /// Removed page indices.
        removed: Vec<usize>,
        /// SVG payloads corresponding exactly to `changed`.
        pages_svg: Vec<PageSvgPayload>,
        /// Complete page sizes in document order.
        page_sizes: Vec<PageSize>,
        /// Pages whose rendering encountered unsupported operations.
        unsupported_pages: Vec<PageUnsupportedPayload>,
        /// Operation timing and payload measurements.
        metrics: OperationMetrics,
        /// Whether a later tail-settle operation remains scheduled.
        tail_pending: bool,
    },
    /// Current compile stopped without replacing last-good belief.
    Cancelled {
        /// Latest request applied by the worker.
        applied_request_seq: RequestSeq,
        /// Latest accepted source revision.
        source_revision: SourceRevision,
    },
    /// Durable user cancellation is active.
    Stopped {
        /// Latest request applied by the worker.
        applied_request_seq: RequestSeq,
        /// Latest accepted source revision.
        source_revision: SourceRevision,
    },
    /// Compilation failed while last-good belief was retained.
    Aborted {
        /// Latest request applied by the worker.
        applied_request_seq: RequestSeq,
        /// Latest accepted source revision.
        source_revision: SourceRevision,
        /// Bounded diagnostic tail.
        log_tail: String,
    },
    /// A well-formed mutating request failed source validation.
    Rejected {
        /// Sequence assigned to the rejected request.
        request_seq: RequestSeq,
        /// Unchanged accepted source revision.
        source_revision: SourceRevision,
        /// Stable human-readable rejection reason.
        reason: String,
    },
    /// Result of a forward source-to-preview query.
    ForwardSearchResult {
        /// Client-assigned query identity.
        query_id: QueryId,
        /// Best geometry in the last published belief, when available.
        result: Option<SyncRectPayload>,
        /// The boxes to flash, on the page of `result`, best first.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        boxes: Vec<SyncRectPayload>,
    },
    /// Result of a backward preview-to-source query.
    BackwardSearchResult {
        /// Client-assigned query identity.
        query_id: QueryId,
        /// Best source location in the last published belief, when available.
        result: Option<SourceLocation>,
    },
    /// Unrecoverable session or transport failure.
    Fatal {
        /// Stable human-readable fatal error.
        reason: String,
    },
}

/// Failure while reading, validating, or encoding one protocol line.
#[derive(Debug)]
pub enum ProtocolError {
    /// Underlying stream read failed.
    Io(std::io::Error),
    /// Physical line exceeded the encoded wire limit.
    LineTooLong {
        /// Configured encoded byte limit.
        limit: usize,
    },
    /// Input ended before any bytes for another request were read.
    EndOfStream,
    /// Line was not valid UTF-8 JSON matching the request schema.
    InvalidJson(serde_json::Error),
    /// Parsed request violates a semantic protocol constraint.
    InvalidRequest(String),
    /// Initialization protocol version is incompatible.
    IncompatibleVersion {
        /// Version sent by the client.
        received: u32,
        /// Version implemented by the server.
        supported: u32,
    },
    /// One decoded source file exceeds its limit.
    SourceFileTooLarge {
        /// Decoded UTF-8 byte count.
        bytes: usize,
        /// Configured decoded byte limit.
        limit: usize,
    },
    /// Combined initialized snapshot exceeds its limit.
    SourceSnapshotTooLarge {
        /// Combined decoded UTF-8 byte count.
        bytes: usize,
        /// Configured decoded byte limit.
        limit: usize,
    },
    /// Encoded server frame exceeds its atomic limit.
    FrameTooLarge {
        /// Encoded frame byte count.
        bytes: usize,
        /// Configured encoded byte limit.
        limit: usize,
    },
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "protocol input failed: {error}"),
            Self::LineTooLong { limit } => write!(f, "JSON line exceeds {limit} bytes"),
            Self::EndOfStream => write!(f, "protocol input ended"),
            Self::InvalidJson(error) => write!(f, "invalid protocol JSON: {error}"),
            Self::InvalidRequest(error) => write!(f, "invalid protocol request: {error}"),
            Self::IncompatibleVersion {
                received,
                supported,
            } => write!(
                f,
                "protocol version {received} is incompatible with supported version {supported}"
            ),
            Self::SourceFileTooLarge { bytes, limit } => {
                write!(f, "source file has {bytes} bytes, exceeding {limit}")
            }
            Self::SourceSnapshotTooLarge { bytes, limit } => {
                write!(f, "source snapshot has {bytes} bytes, exceeding {limit}")
            }
            Self::FrameTooLarge { bytes, limit } => {
                write!(f, "encoded frame has {bytes} bytes, exceeding {limit}")
            }
        }
    }
}

impl std::error::Error for ProtocolError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::InvalidJson(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ProtocolError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// Read and decode exactly one bounded physical JSON line.
pub fn read_request<R: BufRead>(
    reader: &mut R,
    limits: ProtocolLimits,
) -> Result<ClientRequest, ProtocolError> {
    let mut encoded = Vec::new();
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            if encoded.is_empty() {
                return Err(ProtocolError::EndOfStream);
            }
            break;
        }
        let take = available
            .iter()
            .position(|&byte| byte == b'\n')
            .map_or(available.len(), |index| index + 1);
        let ended = available[take - 1] == b'\n';
        let content = if ended {
            &available[..take - 1]
        } else {
            &available[..take]
        };
        if encoded.len().saturating_add(content.len()) > limits.max_json_line_bytes {
            return Err(ProtocolError::LineTooLong {
                limit: limits.max_json_line_bytes,
            });
        }
        encoded.extend_from_slice(content);
        reader.consume(take);
        if ended {
            break;
        }
    }
    if encoded.last() == Some(&b'\r') {
        encoded.pop();
    }
    let request: ClientRequest =
        serde_json::from_slice(&encoded).map_err(ProtocolError::InvalidJson)?;
    validate_request(&request, limits)?;
    Ok(request)
}

/// Encode one complete event and enforce the atomic frame limit.
pub fn encode_event(event: &ServerEvent, limits: ProtocolLimits) -> Result<Vec<u8>, ProtocolError> {
    let mut encoded = serde_json::to_vec(event).map_err(ProtocolError::InvalidJson)?;
    if encoded.len() > limits.max_frame_bytes {
        return Err(ProtocolError::FrameTooLarge {
            bytes: encoded.len(),
            limit: limits.max_frame_bytes,
        });
    }
    encoded.push(b'\n');
    Ok(encoded)
}

fn validate_request(request: &ClientRequest, limits: ProtocolLimits) -> Result<(), ProtocolError> {
    match request {
        ClientRequest::Initialize {
            protocol_version,
            primary_text,
            children,
            ..
        } => {
            if *protocol_version != PROTOCOL_VERSION {
                return Err(ProtocolError::IncompatibleVersion {
                    received: *protocol_version,
                    supported: PROTOCOL_VERSION,
                });
            }
            validate_file(primary_text, limits)?;
            let mut total = primary_text.len();
            for child in children {
                validate_file(&child.text, limits)?;
                total = total.saturating_add(child.text.len());
            }
            if total > limits.max_source_snapshot_bytes {
                return Err(ProtocolError::SourceSnapshotTooLarge {
                    bytes: total,
                    limit: limits.max_source_snapshot_bytes,
                });
            }
        }
        ClientRequest::Replace { text, .. } => validate_file(text, limits)?,
        ClientRequest::Viewport { first, last } if first > last => {
            return Err(ProtocolError::InvalidRequest(
                "viewport begins after it ends".into(),
            ));
        }
        ClientRequest::Painted { paint_ms, .. } if !paint_ms.is_finite() || *paint_ms < 0.0 => {
            return Err(ProtocolError::InvalidRequest(
                "paint_ms must be finite and non-negative".into(),
            ));
        }
        ClientRequest::ForwardSearch { path, line, .. } if path.is_empty() || *line == 0 => {
            return Err(ProtocolError::InvalidRequest(
                "forward search requires a path and one-based line".into(),
            ));
        }
        ClientRequest::BackwardSearch { h_bp, v_bp, .. }
            if !h_bp.is_finite() || !v_bp.is_finite() || *h_bp < 0.0 || *v_bp < 0.0 =>
        {
            return Err(ProtocolError::InvalidRequest(
                "backward search coordinates must be finite and non-negative".into(),
            ));
        }
        _ => {}
    }
    Ok(())
}

fn validate_file(text: &str, limits: ProtocolLimits) -> Result<(), ProtocolError> {
    if text.len() > limits.max_source_file_bytes {
        return Err(ProtocolError::SourceFileTooLarge {
            bytes: text.len(),
            limit: limits.max_source_file_bytes,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::{BufReader, Cursor};

    use super::*;

    fn small_limits() -> ProtocolLimits {
        ProtocolLimits {
            max_source_file_bytes: 16,
            max_source_snapshot_bytes: 24,
            max_json_line_bytes: 256,
            max_frame_bytes: 256,
        }
    }

    #[test]
    fn sequence_counters_are_decimal_strings_above_two_to_the_53() {
        let id = FrameId::new((1_u64 << 53) + 1);
        let encoded = serde_json::to_string(&id).unwrap();
        assert_eq!(encoded, "\"9007199254740993\"");
        assert_eq!(serde_json::from_str::<FrameId>(&encoded).unwrap(), id);
        assert!(serde_json::from_str::<FrameId>("9007199254740993").is_err());
        assert!(serde_json::from_str::<FrameId>("\"01\"").is_err());
    }

    #[test]
    fn counters_fail_instead_of_wrapping() {
        let error = SourceRevision::new(u64::MAX).checked_next().unwrap_err();
        assert_eq!(error.kind(), "source revision");
        assert_eq!(RequestSeq::ZERO.get(), 0);
    }

    #[test]
    fn reads_one_request_without_consuming_the_next() {
        let wire =
            b"{\"type\":\"replace\",\"path\":null,\"text\":\"a\\nb\"}\n{\"type\":\"shutdown\"}\n";
        let mut reader = BufReader::with_capacity(7, Cursor::new(wire));
        let first = read_request(&mut reader, small_limits()).unwrap();
        let second = read_request(&mut reader, small_limits()).unwrap();
        assert_eq!(
            first,
            ClientRequest::Replace {
                path: None,
                text: "a\nb".into()
            }
        );
        assert_eq!(second, ClientRequest::Shutdown {});
    }

    #[test]
    fn wire_limit_allows_worst_case_escaped_source() {
        let text = "\u{0001}".repeat(8);
        let wire = format!(
            "{{\"type\":\"replace\",\"path\":null,\"text\":{}}}\n",
            serde_json::to_string(&text).unwrap()
        );
        let mut limits = small_limits();
        limits.max_json_line_bytes = wire.len() - 1;
        let mut reader = Cursor::new(wire.as_bytes());
        assert!(
            matches!(read_request(&mut reader, limits), Ok(ClientRequest::Replace { text: decoded, .. }) if decoded == text)
        );
    }

    #[test]
    fn rejects_wire_and_decoded_snapshot_limits_separately() {
        let mut reader = Cursor::new(b"{\"type\":\"shutdown\"}\n");
        let mut limits = small_limits();
        limits.max_json_line_bytes = 5;
        assert!(matches!(
            read_request(&mut reader, limits),
            Err(ProtocolError::LineTooLong { .. })
        ));

        let wire = b"{\"type\":\"initialize\",\"protocol_version\":1,\"primary_path\":\"x.tex\",\"primary_text\":\"0123456789abcdef\",\"children\":[{\"path\":\"y.tex\",\"text\":\"0123456789\"}]}\n";
        let mut reader = Cursor::new(wire);
        assert!(matches!(
            read_request(&mut reader, small_limits()),
            Err(ProtocolError::SourceSnapshotTooLarge { .. })
        ));
    }

    #[test]
    fn rejects_incompatible_versions_and_unknown_schema_fields() {
        let mut version = Cursor::new(
            b"{\"type\":\"initialize\",\"protocol_version\":2,\"primary_path\":\"x.tex\",\"primary_text\":\"x\",\"children\":[]}\n",
        );
        assert!(matches!(
            read_request(&mut version, small_limits()),
            Err(ProtocolError::IncompatibleVersion { .. })
        ));

        let mut unknown = Cursor::new(b"{\"type\":\"shutdown\",\"surprise\":true}\n");
        assert!(matches!(
            read_request(&mut unknown, small_limits()),
            Err(ProtocolError::InvalidJson(_))
        ));
    }

    #[test]
    fn frame_limit_never_truncates_output() {
        let event = ServerEvent::Fatal {
            reason: "too large".repeat(20),
        };
        let mut limits = small_limits();
        limits.max_frame_bytes = 16;
        assert!(matches!(
            encode_event(&event, limits),
            Err(ProtocolError::FrameTooLarge { .. })
        ));
    }

    #[test]
    fn rejects_invalid_nonmutating_telemetry() {
        let mut viewport = Cursor::new(b"{\"type\":\"viewport\",\"first\":4,\"last\":2}\n");
        assert!(matches!(
            read_request(&mut viewport, small_limits()),
            Err(ProtocolError::InvalidRequest(_))
        ));

        let mut painted =
            Cursor::new(b"{\"type\":\"painted\",\"frame_id\":\"1\",\"paint_ms\":-1.0}\n");
        assert!(matches!(
            read_request(&mut painted, small_limits()),
            Err(ProtocolError::InvalidRequest(_))
        ));

        let mut forward = Cursor::new(
            b"{\"type\":\"forward_search\",\"query_id\":\"1\",\"path\":\"\",\"line\":0}\n",
        );
        assert!(matches!(
            read_request(&mut forward, small_limits()),
            Err(ProtocolError::InvalidRequest(_))
        ));

        let mut backward = Cursor::new(
            b"{\"type\":\"backward_search\",\"query_id\":\"2\",\"page_index\":0,\"h_bp\":-1.0,\"v_bp\":0.0}\n",
        );
        assert!(matches!(
            read_request(&mut backward, small_limits()),
            Err(ProtocolError::InvalidRequest(_))
        ));
    }

    #[test]
    fn navigation_queries_preserve_exact_query_ids() {
        let request = ClientRequest::ForwardSearch {
            query_id: QueryId::new((1_u64 << 53) + 7),
            path: "chapters/one.tex".into(),
            line: 42,
        };
        let value = serde_json::to_value(request).unwrap();
        assert_eq!(value["type"], "forward_search");
        assert_eq!(value["query_id"], "9007199254740999");

        let event = ServerEvent::BackwardSearchResult {
            query_id: QueryId::new(9),
            result: Some(SourceLocation {
                path: "main.tex".into(),
                line: 3,
            }),
        };
        let value = serde_json::to_value(event).unwrap();
        assert_eq!(value["type"], "backward_search_result");
        assert_eq!(value["query_id"], "9");
        assert_eq!(value["result"]["line"], 3);
    }

    #[test]
    fn update_wire_shape_uses_string_counters() {
        let event = ServerEvent::Update {
            frame_id: FrameId::new(3),
            source_revision: SourceRevision::new(2),
            applied_request_seq: RequestSeq::new(4),
            phase: FramePhase::Foreground,
            outcome: UpdateOutcome::Complete,
            page_count: 0,
            changed: vec![],
            removed: vec![],
            pages_svg: vec![],
            page_sizes: vec![],
            unsupported_pages: vec![],
            metrics: OperationMetrics::default(),
            tail_pending: false,
        };
        let value = serde_json::to_value(event).unwrap();
        assert_eq!(value["type"], "update");
        assert_eq!(value["frame_id"], "3");
        assert_eq!(value["source_revision"], "2");
        assert_eq!(value["applied_request_seq"], "4");
        assert_eq!(value["metrics"]["replay_commands"], "0");
    }
}
