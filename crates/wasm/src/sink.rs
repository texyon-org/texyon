//! The publication boundary, for a host that is a function call away.
//!
//! Natively a frame is encoded as JSON Lines and written to a pipe. Here the
//! caller is the JavaScript that asked for the compile, so the frame is simply
//! kept until it is collected — but it is still kept *atomically*, because the
//! pipeline's guarantee is that a frame becomes visible only once every page it
//! names has been drawn.

use tectonic_live_preview::{FramePatch, FrameSink};
use tectonic_preview_protocol::OperationMetrics;

/// A [`FrameSink`] that retains the last frame for the caller to take.
#[derive(Default)]
pub struct CollectingSink {
    frame: Option<FramePatch>,
    metrics: OperationMetrics,
    tail_pending: bool,
}

impl CollectingSink {
    /// Construct a sink holding no frame.
    pub fn new() -> Self {
        Self::default()
    }

    /// Take the frame published since the last call, if one was.
    ///
    /// An operation that changed nothing about the document still publishes a
    /// frame; one that was cancelled or aborted publishes none, which is what
    /// keeps the last good preview on screen.
    pub fn take(&mut self) -> Option<FramePatch> {
        self.frame.take()
    }

    /// Metrics reported for the most recent frame.
    pub fn metrics(&self) -> &OperationMetrics {
        &self.metrics
    }

    /// Whether the most recent frame left tail work.
    pub fn tail_pending(&self) -> bool {
        self.tail_pending
    }
}

impl FrameSink for CollectingSink {
    // Nothing can fail: there is no transport to fail on, only a move.
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
