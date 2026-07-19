use std::fmt;

use super::{RequestSeq, SequenceExhausted};

/// Result of validating and applying one announced source mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplyDisposition {
    /// Replacement was accepted and advanced the source revision.
    Accepted,
    /// Replacement was rejected without changing accepted source state.
    Rejected,
}

/// Separate announcement, application, and durable-stop state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RequestClock {
    announced: RequestSeq,
    applied: RequestSeq,
    durable_stop: bool,
}

impl RequestClock {
    /// Construct initialized steady-state accounting at sequence/revision zero.
    pub fn new() -> Self {
        Self::default()
    }

    /// Latest sequence assigned by the request side.
    pub fn announced(&self) -> RequestSeq {
        self.announced
    }

    /// Latest sequence applied or rejected in order by the worker.
    pub fn applied(&self) -> RequestSeq {
        self.applied
    }

    /// Whether durable user cancellation remains active.
    pub fn is_stopped(&self) -> bool {
        self.durable_stop
    }

    /// Assign the next sequence to a well-formed mutating request.
    pub fn announce(&mut self) -> Result<RequestSeq, RequestClockError> {
        let sequence = self
            .announced
            .checked_next()
            .map_err(RequestClockError::Sequence)?;
        self.announced = sequence;
        Ok(sequence)
    }

    /// Apply an announced request in exact sequence order.
    ///
    /// Rejections advance `applied` but not `source_revision`. Only acceptance
    /// revokes a durable user stop.
    pub fn apply(
        &mut self,
        sequence: RequestSeq,
        disposition: ApplyDisposition,
    ) -> Result<(), RequestClockError> {
        let expected = self
            .applied
            .checked_next()
            .map_err(RequestClockError::Sequence)?;
        if sequence != expected || sequence > self.announced {
            return Err(RequestClockError::OutOfOrder {
                expected,
                received: sequence,
                announced: self.announced,
            });
        }
        self.applied = sequence;
        if disposition == ApplyDisposition::Accepted {
            self.durable_stop = false;
        }
        Ok(())
    }

    /// Activate durable user cancellation.
    pub fn stop(&mut self) {
        self.durable_stop = true;
    }

    /// Whether the worker is caught up and permitted to compile.
    pub fn may_compile(&self) -> bool {
        !self.durable_stop && self.applied == self.announced
    }
}

/// Invalid or exhausted worker accounting transition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestClockError {
    /// A checked request or revision counter was exhausted.
    Sequence(SequenceExhausted),
    /// Worker attempted to apply a request other than the exact next announced request.
    OutOfOrder {
        /// Exact next sequence required.
        expected: RequestSeq,
        /// Sequence presented by the worker.
        received: RequestSeq,
        /// Latest sequence announced by the request side.
        announced: RequestSeq,
    },
}

impl fmt::Display for RequestClockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sequence(error) => error.fmt(f),
            Self::OutOfOrder { expected, received, announced } => write!(f, "cannot apply request {received}; expected {expected}, latest announced is {announced}"),
        }
    }
}

impl std::error::Error for RequestClockError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejection_advances_catchup_without_advancing_revision() {
        let mut clock = RequestClock::new();
        let first = clock.announce().unwrap();
        let second = clock.announce().unwrap();
        assert!(!clock.may_compile());

        clock.apply(first, ApplyDisposition::Rejected).unwrap();
        assert!(!clock.may_compile());
        clock.apply(second, ApplyDisposition::Accepted).unwrap();
        assert!(clock.may_compile());
    }

    #[test]
    fn rejected_traffic_does_not_revoke_durable_stop() {
        let mut clock = RequestClock::new();
        clock.stop();
        let rejected = clock.announce().unwrap();
        clock.apply(rejected, ApplyDisposition::Rejected).unwrap();
        assert!(clock.is_stopped());
        assert!(!clock.may_compile());

        let accepted = clock.announce().unwrap();
        clock.apply(accepted, ApplyDisposition::Accepted).unwrap();
        assert!(!clock.is_stopped());
        assert!(clock.may_compile());
    }

    #[test]
    fn out_of_order_application_cannot_skip_accounting() {
        let mut clock = RequestClock::new();
        clock.announce().unwrap();
        let second = clock.announce().unwrap();
        assert!(matches!(
            clock.apply(second, ApplyDisposition::Accepted),
            Err(RequestClockError::OutOfOrder { .. })
        ));
        assert_eq!(clock.applied(), RequestSeq::ZERO);
    }
}
