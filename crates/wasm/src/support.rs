//! The I/O stack under each engine operation: its output over [`HostIo`].

use std::{cell::RefCell, rc::Rc};

use texyon_bridge_core::{CoreBridgeLauncher, MinimalDriver};
use texyon_engine_xetex::EngineFrame;
use texyon_errors::Result;
use texyon_io_base::{memory::MemoryFileCollection, stack::IoStack, IoProvider, MemoryIo};
use texyon_live_preview::EngineSupport;

use crate::io::{HostIo, HostStatus};

/// A shared handle on the files the last operation wrote, so the session can
/// read its `.log` without copying the map.
#[derive(Clone, Default)]
pub struct WrittenFiles(Rc<RefCell<Option<Rc<RefCell<MemoryFileCollection>>>>>);

impl WrittenFiles {
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
    messages: Rc<RefCell<Vec<String>>>,
    written: WrittenFiles,
}

impl HostEngineSupport {
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
        // Observed up front so an aborted run still leaves its transcript readable.
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
