//! A frame sink for a host that is a function call away.

use texyon_live_preview::{FramePatch, FrameSink};
use texyon_preview_protocol::OperationMetrics;

/// A [`FrameSink`] that retains the last frame for the caller to take.
#[derive(Default)]
pub struct CollectingSink {
    frame: Option<FramePatch>,
    metrics: OperationMetrics,
    tail_pending: bool,
}

impl CollectingSink {
    /// Take the frame published since the last call.
    ///
    /// Cancelled or aborted operations publish none, which keeps the last
    /// good preview on screen.
    pub fn take(&mut self) -> Option<FramePatch> {
        self.frame.take()
    }

    pub fn metrics(&self) -> OperationMetrics {
        self.metrics
    }

    pub fn tail_pending(&self) -> bool {
        self.tail_pending
    }
}

impl FrameSink for CollectingSink {
    type Error = std::convert::Infallible;

    fn set_operation_metrics(&mut self, metrics: OperationMetrics) {
        self.metrics = metrics;
    }

    fn set_tail_pending(&mut self, tail_pending: bool) {
        self.tail_pending = tail_pending;
    }

    fn publish(&mut self, frame: &FramePatch) -> Result<(), Self::Error> {
        self.frame = Some(frame.clone());
        Ok(())
    }
}
