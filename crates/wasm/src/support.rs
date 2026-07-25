//! The I/O environment the engine runs in when its host is a browser.
//!
//! Natively the preview pipeline stacks a support bundle and a format cache
//! under each operation. Here there is neither: every support file arrives
//! through a blocking call into JavaScript, and the only thing underneath the
//! engine is [`HostIo`] and whatever the operation writes.

use std::{cell::RefCell, rc::Rc};

use tectonic_bridge_core::{CoreBridgeLauncher, MinimalDriver};
use tectonic_engine_xetex::EngineFrame;
use tectonic_errors::Result;
use tectonic_io_base::{memory::MemoryFileCollection, stack::IoStack, IoProvider, MemoryIo};
use tectonic_live_preview::EngineSupport;

use crate::io::{HostIo, HostStatus};

/// A view of what the most recent engine operation wrote.
///
/// The pipeline builds an output provider per operation and keeps only the XDV
/// from it, but a browser editor wants the whole `.log` as well. The provider's
/// file map is reference-counted, so retaining it here costs a pointer rather
/// than a copy of the transcript.
#[derive(Clone, Default)]
pub struct WrittenFiles(Rc<RefCell<Option<Rc<RefCell<MemoryFileCollection>>>>>);

impl WrittenFiles {
    /// Contents of a file the last operation wrote, if it wrote one.
    pub fn get(&self, name: &str) -> Option<Vec<u8>> {
        let outer = self.0.borrow();
        let files = outer.as_ref()?.borrow();
        files.get(name).map(|file| file.data.clone())
    }

    fn observe(&self, files: &Rc<RefCell<MemoryFileCollection>>) {
        *self.0.borrow_mut() = Some(Rc::clone(files));
    }
}

/// Stacks the host's support files under one engine operation.
pub struct HostEngineSupport {
    support: HostIo,
    /// Diagnostics the engine reported, shared with the session.
    messages: Rc<RefCell<Vec<String>>>,
    /// Where each operation's output is published for the session to read.
    written: WrittenFiles,
}

impl HostEngineSupport {
    /// Construct a support layer over the host's file cache.
    pub fn new(support: HostIo, messages: Rc<RefCell<Vec<String>>>, written: WrittenFiles) -> Self {
        Self {
            support,
            messages,
            written,
        }
    }
}

impl EngineSupport for HostEngineSupport {
    fn with_launcher(
        &mut self,
        output: &mut MemoryIo,
        operation: &mut dyn FnMut(&mut CoreBridgeLauncher<'_>) -> Result<Option<EngineFrame>>,
    ) -> Result<Option<EngineFrame>> {
        // Published before the operation rather than after, so a run that
        // aborts still leaves its transcript readable — which is exactly the
        // run whose transcript someone wants.
        self.written.observe(&output.files);

        let mut status = HostStatus::new(Rc::clone(&self.messages));
        let io = IoStack::new(vec![
            output as &mut dyn IoProvider,
            &mut self.support as &mut dyn IoProvider,
        ]);
        let mut hooks = MinimalDriver::new(io);
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        operation(&mut launcher)
    }
}
