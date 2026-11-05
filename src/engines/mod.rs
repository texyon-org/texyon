// Copyright 2016-2021 the Tectonic Project
// Licensed under the MIT License.

//! Access to Tectonic’s processing backends.
//!
//! These backends subsume the functionality of programs such as `bibtex`,
//! `xetex`, and `xdvipdfmx`. This module is historical — the API for each of
//! these is defined in crates with names like `texyon_engine_xetex`.

// Public sub-modules and reexports.

pub mod bibtex;
pub mod tex;
pub mod xdvipdfmx;

pub use self::{bibtex::BibtexEngine, tex::TexEngine, xdvipdfmx::XdvipdfmxEngine};
