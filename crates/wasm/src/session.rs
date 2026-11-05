//! The shared [`PreviewSession`] with the JavaScript host behind it.

use std::{cell::RefCell, rc::Rc, time::Duration};

use texyon_engine_xetex::EnginePreviewConfig;
use texyon_errors::{anyhow::anyhow, Result};
use texyon_live_preview::{DocumentRoot, PreviewSession, SessionConfig, SessionHost};
use texyon_preview_protocol::ProtocolLimits;
use texyon_preview_render::MupdfBackend;

use crate::{host, io::HostIo, resources::HostResources, support::HostEngineSupport};

/// The virtual absolute root every source path is anchored under.
const DOCUMENT_ROOT: &str = "/doc";

// Browser tabs have far less memory than a desktop editor.
const LADDER_TARGET_BYTES: usize = 48 * 1024 * 1024;
const LADDER_HARD_BYTES: usize = 72 * 1024 * 1024;

// The Worker cannot interrupt a call, so a keystroke waits out at most this
// much of a tail settle.
const TAIL_CHUNK: Duration = Duration::from_millis(150);

/// Sources pulled through the host import, and the status lines the engine
/// support collected.
pub struct WasmHost {
    messages: Rc<RefCell<Vec<String>>>,
    support: HostIo,
}

impl SessionHost for WasmHost {
    fn read_source(&mut self, path: &str) -> Result<Option<Vec<u8>>> {
        host::read_source(path).map_err(|error| anyhow!(error))
    }

    fn take_messages(&mut self) -> Vec<String> {
        std::mem::take(&mut *self.messages.borrow_mut())
    }

    fn support_stats(&self) -> (usize, usize) {
        (self.support.cached_count(), self.support.cached_bytes())
    }
}

pub type Session = PreviewSession<HostResources, MupdfBackend, WasmHost>;

/// Create a session. The format is read on the first compile.
pub fn open(format_name: &str, primary_name: &str, raster: bool) -> Result<Session> {
    let mut engine = EnginePreviewConfig::new(format_name);
    engine.ladder_target_bytes = LADDER_TARGET_BYTES;
    engine.ladder_hard_bytes = LADDER_HARD_BYTES;
    engine.tail_chunk = Some(TAIL_CHUNK);

    let root = DocumentRoot::detached(DOCUMENT_ROOT)
        .map_err(|error| anyhow!("invalid document root: {error}"))?;
    let support = HostIo::default();
    let messages = Rc::new(RefCell::new(Vec::new()));
    let backend =
        MupdfBackend::new().map_err(|error| anyhow!("could not start the renderer: {error}"))?;

    Ok(PreviewSession::new(
        SessionConfig {
            engine,
            root,
            primary: primary_name.to_owned(),
            raster,
            limits: ProtocolLimits::default(),
        },
        HostEngineSupport::new(support.clone(), Rc::clone(&messages)),
        HostResources::new(support.clone()),
        backend,
        WasmHost { messages, support },
    ))
}

/// Seed the support cache with bytes the host already holds.
pub fn preload(session: &Session, name: &str, bytes: Vec<u8>) {
    session.host().support.preload(name, bytes);
}
