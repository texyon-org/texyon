//! Messages and counters shared by TeXyoN's live-preview hosts.
//!
//! [`session`] holds the requests and responses every host speaks. Sequence
//! counters serialize as canonical decimal strings so a JavaScript client
//! never routes them through an imprecise number.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

pub mod session;

/// Decoded source size limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtocolLimits {
    /// Maximum UTF-8 bytes in any one source buffer.
    pub max_source_file_bytes: usize,
    /// Maximum combined UTF-8 bytes in a source snapshot.
    pub max_source_snapshot_bytes: usize,
}

impl Default for ProtocolLimits {
    fn default() -> Self {
        const MIB: usize = 1024 * 1024;
        Self {
            max_source_file_bytes: 16 * MIB,
            max_source_snapshot_bytes: 64 * MIB,
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
    CommandCount,
    "Exact number of engine commands replayed by one operation.",
    "command count"
);

/// One complete source buffer.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct SourceFile {
    /// Logical document path.
    pub path: String,
    /// Complete UTF-8 contents.
    pub text: String,
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

/// Per-operation timing and payload metrics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct OperationMetrics {
    /// Engine duration in milliseconds.
    pub engine_ms: f64,
    /// Renderer analysis duration in milliseconds.
    pub analysis_ms: f64,
    /// Selected-page rendering duration in milliseconds.
    pub render_ms: f64,
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
