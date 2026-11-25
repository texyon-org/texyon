//! What the engine reaches the outside world through, lent for each operation.

use texyon_bridge_core::BridgeHost;
use texyon_io_base::OutputHandle;

/// The engine's environment: never part of [`crate::state::EngineState`], so
/// checkpoints and rewinds leave it alone.
pub struct EngineEnv<'h> {
    pub(crate) host: BridgeHost<'h>,
    /// Open outputs, named by index plus one so that 0 stays invalid.
    outputs: Vec<Option<OutputHandle>>,
}

impl<'h> EngineEnv<'h> {
    /// An environment over `host`.
    pub fn new(host: BridgeHost<'h>) -> Self {
        EngineEnv {
            host,
            outputs: Vec::new(),
        }
    }

    /// Keeps `handle` open under a new id, or returns 0 for none.
    pub(crate) fn push_output(&mut self, handle: Option<OutputHandle>) -> usize {
        match handle {
            Some(handle) => {
                self.outputs.push(Some(handle));
                self.outputs.len()
            }
            None => 0,
        }
    }

    fn output_slot(&mut self, id: usize) -> Option<&mut Option<OutputHandle>> {
        self.outputs.get_mut(id.checked_sub(1)?)
    }

    /// Writes `data` to output `id`, returning true on failure.
    pub(crate) fn output_write(&mut self, id: usize, data: &[u8]) -> bool {
        let Some(handle) = id
            .checked_sub(1)
            .and_then(|i| self.outputs.get_mut(i))
            .and_then(Option::as_mut)
        else {
            return true;
        };
        self.host.write_output(handle, data)
    }

    /// Flushes output `id`.
    pub(crate) fn output_flush(&mut self, id: usize) {
        if let Some(handle) = id
            .checked_sub(1)
            .and_then(|i| self.outputs.get_mut(i))
            .and_then(Option::as_mut)
        {
            self.host.flush_output(handle);
        }
    }

    /// Closes output `id`, returning true on failure.
    pub(crate) fn output_close(&mut self, id: usize) -> bool {
        match self.output_slot(id).and_then(Option::take) {
            Some(handle) => self.host.close_output(handle),
            None => false,
        }
    }
}

/// An env over empty in-memory I/O that reports nothing, for unit tests.
#[cfg(test)]
pub(crate) fn test_env() -> EngineEnv<'static> {
    use texyon_bridge_core::{Held, MinimalDriver, SecuritySettings};
    use texyon_io_base::memory::MemoryIo;
    use texyon_status_base::NoopStatusBackend;

    EngineEnv::new(BridgeHost::new(
        Held::Owned(Box::new(MinimalDriver::new(MemoryIo::new(false)))),
        Held::Owned(Box::new(NoopStatusBackend::default())),
        SecuritySettings::default(),
    ))
}
