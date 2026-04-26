//! Test suite targeting constructs actively translated in the engine_xetex
//! Rustification project.
//!
//! # Golden-file workflow
//!
//! Each test compares the engine's `.log` and `.xdv` outputs byte-for-byte
//! against golden files stored in `tests/engine_rust/`.
//!
//! ## Generating / updating goldens
//!
//! Run with the environment variable `TECTONIC_BLESS=1` to write the actual
//! engine output as the new golden files instead of comparing:
//!
//! ```text
//! TECTONIC_BLESS=1 cargo test -p tectonic --test engine_rust
//! ```
//!
//! After blessing, verify the diff looks reasonable, then commit.

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::PathBuf;

use tectonic::engines::tex::TexOutcome;
use tectonic::io::testing::SingleInputFileIo;
use tectonic::io::{FilesystemIo, FilesystemPrimaryInputIo, IoProvider, IoStack, MemoryIo};
use tectonic::TexEngine;
use tectonic_bridge_core::{CoreBridgeLauncher, MinimalDriver};
use tectonic_status_base::NoopStatusBackend;

#[path = "util/mod.rs"]
mod util;
use crate::util::{ensure_plain_format, test_path};

/// Directory for the `.tex` source files.
fn src_dir() -> PathBuf {
    test_path(&["engine_rust"])
}

/// Directory for the golden `.log` / `.xdv` files.
fn golden_dir() -> PathBuf {
    test_path(&["engine_rust", "goldens"])
}

fn bless_mode() -> bool {
    env::var("TECTONIC_BLESS").map(|v| v == "1").unwrap_or(false)
}

/// Run the plain TeX engine on `<stem>.tex` found in `src_dir()`, then either
/// bless the outputs as new goldens or compare against existing ones.
fn run_case(stem: &str, expect: TexOutcome) {
    util::set_test_root();

    let mut src = src_dir();
    src.push(format!("{stem}.tex"));

    let texname = format!("{stem}.tex");
    let logname = format!("{stem}.log");
    let xdvname = format!("{stem}.xdv");

    let mut fmt =
        SingleInputFileIo::new(&ensure_plain_format().expect("couldn't write format file"));

    let mut tex = FilesystemPrimaryInputIo::new(&src);
    let mut assets = FilesystemIo::new(&test_path(&["assets"]), false, false, HashSet::new());
    let mut mem = MemoryIo::new(true);

    let res = {
        let io = IoStack::new(vec![
            &mut mem as &mut dyn IoProvider,
            &mut tex,
            &mut fmt,
            &mut assets,
        ]);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);

        TexEngine::default()
            .process(&mut launcher, "plain.fmt", &texname)
    };

    // Check engine outcome.
    match (&res, expect) {
        (Ok(got), exp) if *got == exp => {}
        (Ok(got), exp) => panic!("{stem}: expected {exp:?}, got {got:?}"),
        (Err(e), _) => panic!("{stem}: engine error: {e}"),
    }

    let files = mem.files.borrow();
    let gdir = golden_dir();

    for output_name in [&logname, &xdvname] {
        let actual = files
            .get(output_name.as_str())
            .unwrap_or_else(|| panic!("{stem}: output file {output_name} not produced"))
            .data
            .clone();

        let golden_path = gdir.join(output_name);

        if bless_mode() {
            fs::create_dir_all(&gdir).expect("creating golden dir");
            fs::write(&golden_path, &actual)
                .unwrap_or_else(|e| panic!("writing {golden_path:?}: {e}"));
            eprintln!("  blessed {output_name}");
        } else {
            let golden = fs::read(&golden_path).unwrap_or_else(|e| {
                panic!(
                    "{stem}: golden file {golden_path:?} not found ({e}). \
                     Run with TECTONIC_BLESS=1 to generate it."
                )
            });
            if actual != golden {
                // Write observed files next to the goldens for easy diffing.
                let obs_path = gdir.join(format!("{output_name}.observed"));
                let _ = fs::write(&obs_path, &actual);
                panic!(
                    "{stem}: {output_name} differs from golden.\n  \
                     Expected: {golden_path:?}\n  \
                     Observed: {obs_path:?}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests: one function per test case
// ---------------------------------------------------------------------------

/// Basic \halign: exercises init_align, fin_col, fin_row, fin_align.
#[test]
fn align_basic() {
    run_case("align_basic", TexOutcome::Spotless);
}

/// \halign with && loop template: exercises preamble extension in fin_col §822.
#[test]
fn align_loop() {
    run_case("align_loop", TexOutcome::Spotless);
}

/// \halign with \span / \multispan: exercises SPAN_CODE path in fin_col and
/// span width accumulation.
#[test]
fn align_span() {
    run_case("align_span", TexOutcome::Spotless);
}

/// Display math ($$): exercises the MMODE branch in init_align and the
/// full math-mode dispatcher path.
#[test]
fn display_math() {
    run_case("display_math", TexOutcome::Spotless);
}

/// Italic correction (\/): exercises append_italic_correction with a
/// NATIVE_WORD_NODE tail — specifically the mem_node_addr fix from session 50b.
#[test]
fn italic_correction() {
    run_case("italic_correction", TexOutcome::Spotless);
}

/// Various \if conditionals: exercises the conditional function translated in
/// session 39.
#[test]
fn conditionals() {
    run_case("conditionals", TexOutcome::Spotless);
}

/// Save/restore stack with alignment: exercises new_save_level, unsave, and
/// eq_define interactions (sessions 17, 21).
#[test]
fn save_restore() {
    run_case("save_restore", TexOutcome::Spotless);
}
