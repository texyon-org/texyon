// Copyright 2021 the Tectonic Project
// Licensed under the MIT License.

//! Emit the C header file for the current engine version

use std::{io, process};
use texyon_errors::prelude::*;
use texyon_xetex_format::engine::Engine;

fn inner() -> Result<()> {
    let engine = Engine::default();
    let stdout = io::stdout();
    let lock = stdout.lock();
    engine.emit_c_header(lock)?;
    Ok(())
}

fn main() {
    if let Err(e) = inner() {
        eprintln!("error: {e}");
        process::exit(1);
    }
}
