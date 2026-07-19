use std::fmt;

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
    };
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_remain_exact_above_javascript_integer_range() {
        let boundary = 1_u64 << 53;
        let before = FrameId::new(boundary - 1);
        let after = before.checked_next().unwrap();

        assert_eq!(after.get(), boundary);
        assert_ne!(after, after.checked_next().unwrap());
        assert_eq!(after.to_string(), "9007199254740992");
    }

    #[test]
    fn counters_fail_instead_of_wrapping() {
        let error = SourceRevision::new(u64::MAX).checked_next().unwrap_err();
        assert_eq!(error.kind(), "source revision");
        assert_eq!(RequestSeq::ZERO.get(), 0);
    }
}
