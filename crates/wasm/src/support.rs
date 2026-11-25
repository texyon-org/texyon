//! The I/O stack under each engine operation: its output over [`HostIo`].

use std::{cell::RefCell, rc::Rc};

use texyon_bridge_core::{CoreBridgeLauncher, MinimalDriver};
use texyon_errors::Result;
use texyon_io_base::{IoProvider, MemoryIo, stack::IoStack};
use texyon_live_preview::EngineSupport;
use texyon_status_base::StatusBackend;

use crate::io::{HostIo, HostStatus};

/// Stacks the host's support files under one engine operation.
pub struct HostEngineSupport {
    support: HostIo,
    messages: Rc<RefCell<Vec<String>>>,
}

impl HostEngineSupport {
    pub fn new(support: HostIo, messages: Rc<RefCell<Vec<String>>>) -> Self {
        Self { support, messages }
    }
}

impl EngineSupport for HostEngineSupport {
    fn session_io(&mut self, output: MemoryIo) -> Result<IoStack<'static>> {
        Ok(IoStack::new(vec![
            Box::new(output),
            Box::new(self.support.clone()),
        ]))
    }

    fn status(&mut self) -> Box<dyn StatusBackend> {
        Box::new(HostStatus::new(Rc::clone(&self.messages)))
    }

    fn with_launcher(
        &mut self,
        files: &mut MemoryIo,
        operation: &mut dyn FnMut(&mut CoreBridgeLauncher<'_>) -> Result<()>,
    ) -> Result<()> {
        let mut status = HostStatus::new(Rc::clone(&self.messages));
        let io = IoStack::new(vec![
            Box::new(files) as Box<dyn IoProvider + '_>,
            Box::new(&mut self.support),
        ]);
        let mut hooks = MinimalDriver::new(io);
        operation(&mut CoreBridgeLauncher::new(&mut hooks, &mut status))
    }
}
