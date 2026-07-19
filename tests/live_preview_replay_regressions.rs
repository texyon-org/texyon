//! Regression coverage for live-preview checkpoint cadence.

use std::{path::PathBuf, sync::Mutex};

use tectonic::{
    engines::tex::{EngineCancellation, EnginePreviewConfig},
    live_preview::{DocumentRoot, ResidentEngineSupport, ResidentXeTeX, SourceSet},
};
use tectonic_preview_protocol::ProtocolLimits;

#[path = "util/mod.rs"]
mod util;

static SESSION_LOCK: Mutex<()> = Mutex::new(());

fn runner() -> ResidentXeTeX {
    let support = ResidentEngineSupport::Fixture {
        format_path: util::ensure_plain_format().unwrap(),
        assets_dir: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/assets"),
    };
    ResidentXeTeX::new(support, EnginePreviewConfig::new("plain.fmt"))
}

fn sources(text: &str) -> (tempfile::TempDir, SourceSet) {
    let temp = tempfile::tempdir().unwrap();
    let root = DocumentRoot::new(temp.path()).unwrap();
    let sources = SourceSet::initialize(
        root,
        "session.tex",
        text.into(),
        vec![],
        ProtocolLimits::default(),
    )
    .unwrap();
    (temp, sources)
}

#[test]
fn command_heavy_source_position_does_not_flood_the_ladder() {
    let _serial = SESSION_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    util::set_test_root();
    let text = "\\newcount\\n \\n=0 \\loop\\advance\\n by1 \\ifnum\\n<100000 \\repeat\nDONE\\bye\n";
    let cancellation = EngineCancellation::new();
    let (_temp, sources) = sources(text);
    let mut runner = runner();
    let output = runner
        .compile(sources.snapshot(), None, &cancellation)
        .unwrap();
    assert!(
        output.frame.checkpoint_rungs_admitted < 32,
        "command-heavy input admitted {} checkpoints",
        output.frame.checkpoint_rungs_admitted
    );
    runner.finish().unwrap();
}
