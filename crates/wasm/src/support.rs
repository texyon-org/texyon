//! The I/O stack under each engine operation: its output over [`HostIo`].

use std::{cell::RefCell, rc::Rc};

use texyon_bridge_core::{CoreBridgeLauncher, MinimalDriver};
use texyon_engine_xetex::EngineFrame;
use texyon_errors::Result;
use texyon_io_base::{IoProvider, MemoryIo, stack::IoStack};
use texyon_live_preview::EngineSupport;

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
    fn with_launcher(
        &mut self,
        output: &mut MemoryIo,
        operation: &mut dyn FnMut(&mut CoreBridgeLauncher<'_>) -> Result<Option<EngineFrame>>,
    ) -> Result<Option<EngineFrame>> {
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
