use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// Cross-thread cooperative cancellation signal for resident XeTeX work.
#[derive(Clone, Debug, Default)]
pub struct EngineCancellation(Arc<AtomicBool>);

impl EngineCancellation {
    /// Construct a re-armed cancellation signal.
    pub fn new() -> Self {
        Self::default()
    }

    /// Request cancellation of current instrumented engine work.
    pub fn request(&self) {
        self.0.store(true, Ordering::Release);
    }

    /// Re-arm the signal before starting a newer operation.
    pub fn rearm(&self) {
        self.0.store(false, Ordering::Release);
    }

    /// Whether cancellation has been requested.
    pub fn is_requested(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Default)]
pub(crate) struct CancellationState {
    signal: EngineCancellation,
    expansion_tokens: u16,
}

impl CancellationState {
    pub(crate) fn install(&mut self, signal: EngineCancellation) {
        self.signal = signal;
        self.expansion_tokens = 0;
    }

    pub(crate) fn at_command_boundary(&self) -> bool {
        self.signal.is_requested()
    }

    pub(crate) fn at_expansion_token(&mut self) -> bool {
        const POLL_INTERVAL: u16 = 1024;
        self.expansion_tokens = self.expansion_tokens.wrapping_add(1);
        if self.expansion_tokens < POLL_INTERVAL {
            false
        } else {
            self.expansion_tokens = 0;
            self.signal.is_requested()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_is_shared_and_rearmable() {
        let signal = EngineCancellation::new();
        let other = signal.clone();
        other.request();
        assert!(signal.is_requested());
        signal.rearm();
        assert!(!other.is_requested());
    }

    #[test]
    fn expansion_poll_is_bounded() {
        let signal = EngineCancellation::new();
        signal.request();
        let mut state = CancellationState::default();
        state.install(signal);
        for _ in 0..1023 {
            assert!(!state.at_expansion_token());
        }
        assert!(state.at_expansion_token());
    }
}
