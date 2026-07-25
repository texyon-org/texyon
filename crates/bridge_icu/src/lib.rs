// Copyright 2020 the Tectonic Project
// Licensed under the MIT License.

//! On most targets, no Rust code: this crate exists to export the ICU *C* API
//! into the Cargo framework.
//!
//! wasm is the exception. There is no system ICU to export there, so the crate
//! supplies the pieces XeTeX uses from pure-Rust crates instead — see
//! [`wasm_shim`].

#[cfg(target_family = "wasm")]
pub mod wasm_shim;
