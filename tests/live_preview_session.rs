//! Differential coverage for the synchronous resident XeTeX session.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use sha2::{Digest, Sha256};
use tectonic::engines::tex::{
    EngineCancellation, EnginePreviewConfig, IncrementalXeTeXSession, SourceSnapshot, TexEngine,
    TexOutcome,
};
use tectonic::io::testing::SingleInputFileIo;
use tectonic::io::{IoProvider, IoStack, MemoryIo};
use tectonic::live_preview::{
    AssembledDocument, DocumentAssembler, DocumentFragment, DocumentRoot, JsonlFrameSink,
    PreviewWorkerProcessor, ResidentEngineOutput, ResidentEngineSupport, ResidentPreviewProcessor,
    ResidentXeTeX, SourceSet, WorkerRun,
};
use tectonic_bridge_core::{CoreBridgeLauncher, MinimalDriver};
use tectonic_io_base::stdstreams::BufferedPrimaryIo;
use tectonic_preview_protocol::{FramePhase, ProtocolLimits, RequestSeq, ServerEvent, SourceFile};
use tectonic_preview_render::{
    NullBackend, RenderError, Resource, ResourceKind, ResourceProvider, ResourceRequest,
};
use tectonic_status_base::NoopStatusBackend;

#[path = "util/mod.rs"]
mod util;

static SESSION_TEST_LOCK: Mutex<()> = Mutex::new(());

fn snapshot(bytes: &[u8]) -> SourceSnapshot {
    snapshot_with_child(bytes, None)
}

fn snapshot_with_child(bytes: &[u8], child: Option<&[u8]>) -> SourceSnapshot {
    let mut entries = vec![(PathBuf::from("session.tex"), Arc::from(bytes))];
    if let Some(child) = child {
        entries.push((PathBuf::from("child.tex"), Arc::from(child)));
    }
    SourceSnapshot::new(
        PathBuf::from("/workspace"),
        PathBuf::from("session.tex"),
        entries,
    )
    .unwrap()
}

fn output_bytes(memory: &MemoryIo, name: &str) -> Vec<u8> {
    memory
        .files
        .borrow()
        .get(name)
        .unwrap_or_else(|| panic!("missing output {name}"))
        .data
        .clone()
}

fn cold_xdv(bytes: &[u8]) -> Vec<u8> {
    cold_xdv_with_child(bytes, None)
}

fn cold_xdv_with_child(bytes: &[u8], child: Option<&[u8]>) -> Vec<u8> {
    cold_output_with_child(bytes, child, "session.xdv")
}

fn cold_output_with_child(bytes: &[u8], child: Option<&[u8]>, name: &str) -> Vec<u8> {
    let children = child
        .map(|bytes| vec![("child.tex", bytes)])
        .unwrap_or_default();
    cold_output_with_children(bytes, &children, name)
}

fn cold_output_with_children(bytes: &[u8], children: &[(&str, &[u8])], name: &str) -> Vec<u8> {
    let mut format =
        SingleInputFileIo::new(&util::ensure_plain_format().expect("could not build plain format"));
    let mut primary = BufferedPrimaryIo::from_buffer(bytes.to_vec());
    let mut output = MemoryIo::new(true);
    for (path, child) in children {
        output.create_entry(path, child.to_vec());
    }
    {
        let io = IoStack::new(vec![
            &mut output as &mut dyn IoProvider,
            &mut primary as &mut dyn IoProvider,
            &mut format as &mut dyn IoProvider,
        ]);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        let outcome = TexEngine::default()
            .deterministic_mode(true)
            .process(&mut launcher, "plain.fmt", "session.tex")
            .unwrap();
        assert_eq!(outcome, TexOutcome::Spotless);
    }
    output_bytes(&output, name)
}

fn assemble_resident_output(
    assembler: &DocumentAssembler,
    base: Option<&AssembledDocument>,
    output: ResidentEngineOutput,
) -> AssembledDocument {
    let xdv = output
        .xdv
        .expect("successful resident operation omitted XDV");
    let fragment =
        output
            .frame
            .splice_at
            .map_or(DocumentFragment::Complete(xdv.as_ref()), |prefix_len| {
                DocumentFragment::Tail {
                    prefix_len,
                    bytes: xdv.as_ref(),
                }
            });
    assembler.assemble(base, fragment).unwrap()
}

#[test]
fn session_open_edit_and_revert_are_byte_stable() {
    let _serial = SESSION_TEST_LOCK.lock().unwrap();
    util::set_test_root();
    let cold_original = cold_xdv(b"A\n\\bye\n");
    let cold_edited = cold_xdv(b"B\n\\bye\n");
    let mut format =
        SingleInputFileIo::new(&util::ensure_plain_format().expect("could not build plain format"));
    let mut open_output = MemoryIo::new(true);
    let mut session = IncrementalXeTeXSession::new(EnginePreviewConfig::new("plain.fmt")).unwrap();

    let original = snapshot(b"A\n\\bye\n");
    {
        let io = IoStack::new(vec![
            &mut open_output as &mut dyn IoProvider,
            &mut format as &mut dyn IoProvider,
        ]);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        let frame = session.open(&mut launcher, original.clone()).unwrap();
        assert_eq!(frame.outcome, TexOutcome::Spotless);
    }
    let original_xdv = output_bytes(&open_output, "session.xdv");
    assert_eq!(original_xdv, cold_original);

    let edited = original
        .replace("session", Arc::from(&b"B\n\\bye\n"[..]))
        .unwrap();
    let mut edit_output = MemoryIo::new(true);
    {
        let io = IoStack::new(vec![&mut edit_output as &mut dyn IoProvider]);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        let frame = session
            .edit(&mut launcher, edited.snapshot.clone(), None)
            .unwrap();
        assert_eq!(frame.outcome, TexOutcome::Spotless);
    }
    assert_eq!(output_bytes(&edit_output, "session.xdv"), cold_edited);

    let reverted = edited
        .snapshot
        .replace("session.tex", Arc::from(&b"A\n\\bye\n"[..]))
        .unwrap();
    let mut revert_output = MemoryIo::new(true);
    {
        let io = IoStack::new(vec![&mut revert_output as &mut dyn IoProvider]);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        let frame = session
            .edit(&mut launcher, reverted.snapshot, None)
            .unwrap();
        assert_eq!(frame.outcome, TexOutcome::Spotless);
        session.finish(&mut launcher).unwrap();
    }
    assert_eq!(output_bytes(&revert_output, "session.xdv"), original_xdv);
}

#[test]
fn late_edit_uses_a_nonzero_fence_and_assembles_to_cold_output() {
    let _serial = SESSION_TEST_LOCK.lock().unwrap();
    util::set_test_root();
    let mut original = String::from("\\hsize=220pt\n");
    for index in 0..2400 {
        if index == 2100 {
            original.push_str("ORIGINAL TAIL MARKER\\par\n");
        } else {
            original.push_str(&format!(
                "line {index} has enough words for typesetting\\par\n"
            ));
        }
    }
    original.push_str("\\bye\n");
    let edited = original.replace("ORIGINAL TAIL MARKER", "EDITED TAIL MARKER");
    let edited_again = edited.replace("EDITED TAIL MARKER", "SECOND TAIL MARKER");
    let cold_original = cold_xdv(original.as_bytes());
    let cold_edited = cold_xdv(edited.as_bytes());
    let cold_edited_again = cold_xdv(edited_again.as_bytes());

    let mut format =
        SingleInputFileIo::new(&util::ensure_plain_format().expect("could not build plain format"));
    let mut session = IncrementalXeTeXSession::new(EnginePreviewConfig::new("plain.fmt")).unwrap();
    let source = snapshot(original.as_bytes());
    let mut first_output = MemoryIo::new(true);
    let opened = {
        let io = IoStack::new(vec![
            &mut first_output as &mut dyn IoProvider,
            &mut format as &mut dyn IoProvider,
        ]);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        session.open(&mut launcher, source.clone()).unwrap()
    };
    assert!(opened.checkpoint_rungs > 1);
    assert_eq!(output_bytes(&first_output, "session.xdv"), cold_original);

    let changed = source
        .replace("session.tex", Arc::from(edited.as_bytes()))
        .unwrap();
    let changed_snapshot = changed.snapshot.clone();
    let mut edit_output = MemoryIo::new(true);
    let frame = {
        let io = IoStack::new(vec![&mut edit_output as &mut dyn IoProvider]);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        session.edit(&mut launcher, changed.snapshot, None).unwrap()
    };
    let splice_at = frame.splice_at.expect("late edit fell back to rung zero");
    assert!(splice_at > 0);
    let tail = output_bytes(&edit_output, "session.xdv");
    let assembler = DocumentAssembler::new();
    let base = assembler
        .assemble(None, DocumentFragment::Complete(&cold_original))
        .unwrap();
    let assembled = assembler
        .assemble(
            Some(&base),
            DocumentFragment::Tail {
                prefix_len: splice_at,
                bytes: &tail,
            },
        )
        .unwrap();
    assert_eq!(assembled.bytes().as_ref(), cold_edited);

    let changed_again = changed_snapshot
        .replace("session.tex", Arc::from(edited_again.as_bytes()))
        .unwrap();
    let mut second_output = MemoryIo::new(true);
    let second_frame = {
        let io = IoStack::new(vec![&mut second_output as &mut dyn IoProvider]);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        let frame = session
            .edit(&mut launcher, changed_again.snapshot, None)
            .unwrap();
        session.finish(&mut launcher).unwrap();
        frame
    };
    let second_splice = second_frame
        .splice_at
        .expect("repeat edit did not use its protected local fence");
    assert!(second_splice >= splice_at);
    let second_tail = output_bytes(&second_output, "session.xdv");
    let assembled_again = assembler
        .assemble(
            Some(&assembled),
            DocumentFragment::Tail {
                prefix_len: second_splice,
                bytes: &second_tail,
            },
        )
        .unwrap();
    assert_eq!(assembled_again.bytes().as_ref(), cold_edited_again);
}

#[test]
fn late_child_edit_resumes_with_the_nested_input_open() {
    let _serial = SESSION_TEST_LOCK.lock().unwrap();
    util::set_test_root();
    let primary = b"\\input child\nAFTER CHILD\\par\n\\bye\n";
    let mut child = String::new();
    for index in 0..2400 {
        if index == 2100 {
            child.push_str("ORIGINAL CHILD MARKER\\par\n");
        } else {
            child.push_str(&format!("child line {index} has several words\\par\n"));
        }
    }
    let edited_child = child.replace("ORIGINAL CHILD MARKER", "EDITED CHILD MARKER");
    let cold_original = cold_xdv_with_child(primary, Some(child.as_bytes()));
    let cold_edited = cold_xdv_with_child(primary, Some(edited_child.as_bytes()));

    let mut format =
        SingleInputFileIo::new(&util::ensure_plain_format().expect("could not build plain format"));
    let mut session = IncrementalXeTeXSession::new(EnginePreviewConfig::new("plain.fmt")).unwrap();
    let source = snapshot_with_child(primary, Some(child.as_bytes()));
    let mut first_output = MemoryIo::new(true);
    {
        let io = IoStack::new(vec![
            &mut first_output as &mut dyn IoProvider,
            &mut format as &mut dyn IoProvider,
        ]);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        session.open(&mut launcher, source.clone()).unwrap();
    }

    let changed = source
        .replace("child.tex", Arc::from(edited_child.as_bytes()))
        .unwrap();
    let mut edit_output = MemoryIo::new(true);
    let frame = {
        let io = IoStack::new(vec![&mut edit_output as &mut dyn IoProvider]);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        let frame = session.edit(&mut launcher, changed.snapshot, None).unwrap();
        session.finish(&mut launcher).unwrap();
        frame
    };
    let splice_at = frame
        .splice_at
        .expect("late child edit did not use an open-child fence");
    assert!(splice_at > 0);
    let tail = output_bytes(&edit_output, "session.xdv");
    let assembler = DocumentAssembler::new();
    let base = assembler
        .assemble(None, DocumentFragment::Complete(&cold_original))
        .unwrap();
    let assembled = assembler
        .assemble(
            Some(&base),
            DocumentFragment::Tail {
                prefix_len: splice_at,
                bytes: &tail,
            },
        )
        .unwrap();
    assert_eq!(assembled.bytes().as_ref(), cold_edited);
}

#[test]
fn changing_a_consumed_child_falls_back_to_rung_zero() {
    let _serial = SESSION_TEST_LOCK.lock().unwrap();
    util::set_test_root();
    let mut primary = String::from("\\input child\n");
    for index in 0..1200 {
        primary.push_str(&format!("primary line {index} has several words\\par\n"));
    }
    primary.push_str("\\bye\n");
    let original_child = b"ORIGINAL CHILD\\par\n";
    let edited_child = b"EDITED CHILD\\par\n";
    let cold_edited = cold_xdv_with_child(primary.as_bytes(), Some(edited_child));

    let mut format =
        SingleInputFileIo::new(&util::ensure_plain_format().expect("could not build plain format"));
    let mut session = IncrementalXeTeXSession::new(EnginePreviewConfig::new("plain.fmt")).unwrap();
    let source = snapshot_with_child(primary.as_bytes(), Some(original_child));
    let mut first_output = MemoryIo::new(true);
    let opened = {
        let io = IoStack::new(vec![
            &mut first_output as &mut dyn IoProvider,
            &mut format as &mut dyn IoProvider,
        ]);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        session.open(&mut launcher, source.clone()).unwrap()
    };
    assert!(opened.checkpoint_rungs > 1);

    let changed = source
        .replace("child.tex", Arc::from(&edited_child[..]))
        .unwrap();
    let mut edit_output = MemoryIo::new(true);
    let frame = {
        let io = IoStack::new(vec![&mut edit_output as &mut dyn IoProvider]);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        let frame = session.edit(&mut launcher, changed.snapshot, None).unwrap();
        session.finish(&mut launcher).unwrap();
        frame
    };
    assert_eq!(frame.splice_at, None);
    assert_eq!(output_bytes(&edit_output, "session.xdv"), cold_edited);
}

#[test]
fn same_size_same_mtime_disk_child_invalidates_replay() {
    let _serial = SESSION_TEST_LOCK.lock().unwrap();
    util::set_test_root();
    let original_primary = "\\input disk\nPRIMARY\n\\bye\n";
    let edited_primary = "\\input disk\nPRIMARY EDITED\n\\bye\n";
    let expected = cold_output_with_children(
        edited_primary.as_bytes(),
        &[("disk.tex", &b"B\n"[..])],
        "session.xdv",
    );
    let temp = tempfile::tempdir().unwrap();
    let disk_path = temp.path().join("disk.tex");
    std::fs::write(&disk_path, b"A\n").unwrap();
    let fixed_time = filetime::FileTime::from_unix_time(1_700_000_000, 0);
    filetime::set_file_mtime(&disk_path, fixed_time).unwrap();
    let root = DocumentRoot::new(temp.path()).unwrap();
    let mut sources = SourceSet::initialize(
        root,
        "main.tex",
        original_primary.into(),
        vec![],
        ProtocolLimits::default(),
    )
    .unwrap();
    let support = ResidentEngineSupport::Fixture {
        format_path: util::ensure_plain_format().unwrap(),
        assets_dir: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/assets"),
    };
    let cancellation = EngineCancellation::new();
    let mut runner = ResidentXeTeX::new(support, EnginePreviewConfig::new("plain.fmt"));
    runner
        .compile(sources.snapshot(), None, &cancellation)
        .unwrap();

    std::fs::write(&disk_path, b"B\n").unwrap();
    filetime::set_file_mtime(&disk_path, fixed_time).unwrap();
    sources.replace(None, edited_primary.into()).unwrap();
    let edited = runner
        .compile(sources.snapshot(), None, &cancellation)
        .unwrap();
    assert_eq!(edited.frame.splice_at, None);
    assert_eq!(edited.xdv.unwrap().as_ref(), expected);
    runner.finish().unwrap();
}

#[test]
fn live_write_stream_is_reopened_with_its_checkpointed_prefix() {
    let _serial = SESSION_TEST_LOCK.lock().unwrap();
    util::set_test_root();
    let mut original = String::from(
        "\\newwrite\\previewaux\n\\immediate\\openout\\previewaux=state.aux\n\\immediate\\write\\previewaux{prefix record}\n",
    );
    for index in 0..1400 {
        if index == 1200 {
            original.push_str("ORIGINAL WRITE MARKER\\par\n");
        } else {
            original.push_str(&format!("write test line {index} has words\\par\n"));
        }
    }
    original.push_str(
        "\\immediate\\write\\previewaux{tail record}\n\\immediate\\closeout\\previewaux\n\\bye\n",
    );
    let edited = original.replace("ORIGINAL WRITE MARKER", "EDITED WRITE MARKER");
    let cold_original = cold_xdv(original.as_bytes());
    let cold_edited = cold_xdv(edited.as_bytes());
    let cold_aux = cold_output_with_child(edited.as_bytes(), None, "state.aux");

    let mut format =
        SingleInputFileIo::new(&util::ensure_plain_format().expect("could not build plain format"));
    let mut session = IncrementalXeTeXSession::new(EnginePreviewConfig::new("plain.fmt")).unwrap();
    let source = snapshot(original.as_bytes());
    let mut first_output = MemoryIo::new(true);
    {
        let io = IoStack::new(vec![
            &mut first_output as &mut dyn IoProvider,
            &mut format as &mut dyn IoProvider,
        ]);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        session.open(&mut launcher, source.clone()).unwrap();
    }
    assert_eq!(output_bytes(&first_output, "session.xdv"), cold_original);

    let changed = source
        .replace("session.tex", Arc::from(edited.as_bytes()))
        .unwrap();
    let mut edit_output = MemoryIo::new(true);
    let frame = {
        let io = IoStack::new(vec![&mut edit_output as &mut dyn IoProvider]);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        let frame = session.edit(&mut launcher, changed.snapshot, None).unwrap();
        session.finish(&mut launcher).unwrap();
        frame
    };
    let splice_at = frame
        .splice_at
        .expect("live write stream made the edit fall back to rung zero");
    let assembler = DocumentAssembler::new();
    let base = assembler
        .assemble(None, DocumentFragment::Complete(&cold_original))
        .unwrap();
    let tail = output_bytes(&edit_output, "session.xdv");
    let assembled = assembler
        .assemble(
            Some(&base),
            DocumentFragment::Tail {
                prefix_len: splice_at,
                bytes: &tail,
            },
        )
        .unwrap();
    assert_eq!(assembled.bytes().as_ref(), cold_edited);
    assert_eq!(output_bytes(&edit_output, "state.aux"), cold_aux);
}

#[test]
fn fatal_class_edit_abort_rewinds_and_next_edit_recovers() {
    let _serial = SESSION_TEST_LOCK.lock().unwrap();
    util::set_test_root();
    let mut format =
        SingleInputFileIo::new(&util::ensure_plain_format().expect("could not build plain format"));
    let mut first_output = MemoryIo::new(true);
    let mut session = IncrementalXeTeXSession::new(EnginePreviewConfig::new("plain.fmt")).unwrap();
    let valid = snapshot(b"A\n\\bye\n");
    {
        let io = IoStack::new(vec![
            &mut first_output as &mut dyn IoProvider,
            &mut format as &mut dyn IoProvider,
        ]);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        session.open(&mut launcher, valid.clone()).unwrap();
    }
    let expected = output_bytes(&first_output, "session.xdv");

    let broken = valid.replace("session", Arc::from(&b"A\n"[..])).unwrap();
    let mut broken_output = MemoryIo::new(true);
    {
        let io = IoStack::new(vec![&mut broken_output as &mut dyn IoProvider]);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        let frame = session.edit(&mut launcher, broken.snapshot, None).unwrap();
        assert!(frame.aborted.is_some());
        assert!(!session.is_dead());
    }

    let mut recovered_output = MemoryIo::new(true);
    {
        let io = IoStack::new(vec![&mut recovered_output as &mut dyn IoProvider]);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        let frame = session.edit(&mut launcher, valid, None).unwrap();
        assert_eq!(frame.outcome, TexOutcome::Spotless);
        assert!(frame.aborted.is_none());
        session.finish(&mut launcher).unwrap();
    }
    assert_eq!(output_bytes(&recovered_output, "session.xdv"), expected);
}

#[test]
fn cooperative_cancel_rewinds_and_next_edit_recovers() {
    let _serial = SESSION_TEST_LOCK.lock().unwrap();
    util::set_test_root();
    let mut format =
        SingleInputFileIo::new(&util::ensure_plain_format().expect("could not build plain format"));
    let cancellation = EngineCancellation::new();
    let mut config = EnginePreviewConfig::new("plain.fmt");
    config.cancellation = cancellation.clone();
    let mut session = IncrementalXeTeXSession::new(config).unwrap();
    let source = snapshot(b"A\n\\bye\n");

    let mut first_output = MemoryIo::new(true);
    {
        let io = IoStack::new(vec![
            &mut first_output as &mut dyn IoProvider,
            &mut format as &mut dyn IoProvider,
        ]);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        assert!(
            !session
                .open(&mut launcher, source.clone())
                .unwrap()
                .cancelled
        );
    }

    cancellation.request();
    let mut cancelled_output = MemoryIo::new(true);
    {
        let io = IoStack::new(vec![&mut cancelled_output as &mut dyn IoProvider]);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        let frame = session.edit(&mut launcher, source.clone(), None).unwrap();
        assert!(frame.cancelled);
        assert!(frame.aborted.is_none());
        assert!(!session.is_dead());
    }

    cancellation.rearm();
    let mut recovered_output = MemoryIo::new(true);
    {
        let io = IoStack::new(vec![&mut recovered_output as &mut dyn IoProvider]);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        let frame = session.edit(&mut launcher, source, None).unwrap();
        assert!(!frame.cancelled);
        assert_eq!(frame.outcome, TexOutcome::Spotless);
        session.finish(&mut launcher).unwrap();
    }
}

#[test]
fn resident_engine_driver_captures_complete_xdv_and_revision() {
    let _serial = SESSION_TEST_LOCK.lock().unwrap();
    util::set_test_root();
    let expected = cold_xdv(b"A\n\\bye\n");
    let temp = tempfile::tempdir().unwrap();
    let root = DocumentRoot::new(temp.path()).unwrap();
    let mut sources = SourceSet::initialize(
        root,
        "main.tex",
        "A\n\\bye\n".into(),
        vec![],
        ProtocolLimits::default(),
    )
    .unwrap();
    let support = ResidentEngineSupport::Fixture {
        format_path: util::ensure_plain_format().unwrap(),
        assets_dir: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/assets"),
    };
    let cancellation = EngineCancellation::new();
    let mut runner = ResidentXeTeX::new(support, EnginePreviewConfig::new("plain.fmt"));

    let first = runner
        .compile(sources.snapshot(), None, &cancellation)
        .unwrap();
    assert_eq!(first.xdv.unwrap().as_ref(), expected);
    assert_eq!(first.frame.source_revision.get(), 0);

    sources.replace(None, "B\n\\bye\n".into()).unwrap();
    let edited = runner
        .compile(sources.snapshot(), Some((0, 0)), &cancellation)
        .unwrap();
    assert!(edited.xdv.is_some());
    assert_eq!(edited.frame.source_revision.get(), 1);

    cancellation.request();
    let cancelled = runner
        .compile(sources.snapshot(), None, &cancellation)
        .unwrap();
    assert!(cancelled.frame.cancelled);
    assert!(cancelled.xdv.is_none());
    cancellation.rearm();
    assert!(runner
        .compile(sources.snapshot(), None, &cancellation)
        .unwrap()
        .xdv
        .is_some());
    runner.finish().unwrap();
}

#[test]
fn viewport_foreground_and_tail_reassemble_to_cold_xdv() {
    let _serial = SESSION_TEST_LOCK.lock().unwrap();
    util::set_test_root();
    let original = b"A\\vfill\\eject\nB\\vfill\\eject\nC\\vfill\\eject\nD\\bye\n";
    let edited = b"Z\\vfill\\eject\nB\\vfill\\eject\nC\\vfill\\eject\nD\\bye\n";
    let expected = cold_xdv(edited);
    let temp = tempfile::tempdir().unwrap();
    let root = DocumentRoot::new(temp.path()).unwrap();
    let mut sources = SourceSet::initialize(
        root,
        "main.tex",
        String::from_utf8(original.to_vec()).unwrap(),
        vec![],
        ProtocolLimits::default(),
    )
    .unwrap();
    let support = ResidentEngineSupport::Fixture {
        format_path: util::ensure_plain_format().unwrap(),
        assets_dir: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/assets"),
    };
    let cancellation = EngineCancellation::new();
    let mut runner = ResidentXeTeX::new(support, EnginePreviewConfig::new("plain.fmt"));
    let assembler = DocumentAssembler::new();

    let initial = runner
        .compile(sources.snapshot(), None, &cancellation)
        .unwrap();
    let initial_xdv = initial.xdv.unwrap();
    let initial_document = assembler
        .assemble(None, DocumentFragment::Complete(&initial_xdv))
        .unwrap();

    sources
        .replace(None, String::from_utf8(edited.to_vec()).unwrap())
        .unwrap();
    let foreground = runner
        .compile(sources.snapshot(), Some((0, 0)), &cancellation)
        .unwrap();
    assert!(foreground.frame.tail_pending);
    assert!(runner.has_pending_tail());
    assert_eq!(foreground.frame.pages.len(), 1);
    let foreground_xdv = foreground.xdv.unwrap();
    let foreground_fragment = foreground.frame.splice_at.map_or(
        DocumentFragment::Complete(foreground_xdv.as_ref()),
        |prefix_len| DocumentFragment::Tail {
            prefix_len,
            bytes: foreground_xdv.as_ref(),
        },
    );
    let foreground_document = assembler
        .assemble(Some(&initial_document), foreground_fragment)
        .unwrap();

    let tail = runner
        .continue_tail(&cancellation)
        .unwrap()
        .expect("foreground advertised a pending tail");
    assert!(!tail.frame.tail_pending);
    assert!(!runner.has_pending_tail());
    let tail_xdv = tail.xdv.unwrap();
    let tail_fragment = tail.frame.splice_at.map_or(
        DocumentFragment::Complete(tail_xdv.as_ref()),
        |prefix_len| DocumentFragment::Tail {
            prefix_len,
            bytes: tail_xdv.as_ref(),
        },
    );
    let complete = assembler
        .assemble(Some(&foreground_document), tail_fragment)
        .unwrap();
    assert_eq!(complete.bytes().as_ref(), expected);
    runner.finish().unwrap();
}

#[test]
fn cancelled_tail_preserves_its_horizon_fence() {
    let _serial = SESSION_TEST_LOCK.lock().unwrap();
    util::set_test_root();
    let original = "A\\vfill\\eject\nB\\vfill\\eject\nC\\vfill\\eject\nD\\bye\n";
    let first_edit = "Z\\vfill\\eject\nB\\vfill\\eject\nC\\vfill\\eject\nD\\bye\n";
    let second_edit = "Z\\vfill\\eject\nB\\vfill\\eject\nC\\vfill\\eject\nY\\bye\n";
    let expected = cold_xdv(second_edit.as_bytes());
    let temp = tempfile::tempdir().unwrap();
    let root = DocumentRoot::new(temp.path()).unwrap();
    let mut sources = SourceSet::initialize(
        root,
        "main.tex",
        original.into(),
        vec![],
        ProtocolLimits::default(),
    )
    .unwrap();
    let support = ResidentEngineSupport::Fixture {
        format_path: util::ensure_plain_format().unwrap(),
        assets_dir: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/assets"),
    };
    let cancellation = EngineCancellation::new();
    let mut runner = ResidentXeTeX::new(support, EnginePreviewConfig::new("plain.fmt"));
    let assembler = DocumentAssembler::new();
    let initial = runner
        .compile(sources.snapshot(), None, &cancellation)
        .unwrap();
    let initial_xdv = initial.xdv.unwrap();
    let initial_document = assembler
        .assemble(None, DocumentFragment::Complete(&initial_xdv))
        .unwrap();

    sources.replace(None, first_edit.into()).unwrap();
    let foreground = runner
        .compile(sources.snapshot(), Some((0, 0)), &cancellation)
        .unwrap();
    let foreground_xdv = foreground.xdv.unwrap();
    let foreground_fragment = foreground.frame.splice_at.map_or(
        DocumentFragment::Complete(foreground_xdv.as_ref()),
        |prefix_len| DocumentFragment::Tail {
            prefix_len,
            bytes: foreground_xdv.as_ref(),
        },
    );
    let foreground_document = assembler
        .assemble(Some(&initial_document), foreground_fragment)
        .unwrap();

    cancellation.request();
    let cancelled = runner
        .continue_tail(&cancellation)
        .unwrap()
        .expect("foreground advertised a pending tail");
    assert!(cancelled.frame.cancelled);
    assert!(cancelled.frame.checkpoint_rungs > 1);
    cancellation.rearm();

    sources.replace(None, second_edit.into()).unwrap();
    let replay = runner
        .compile(sources.snapshot(), None, &cancellation)
        .unwrap();
    assert!(!replay.frame.tail_pending);
    let splice_at = replay
        .frame
        .splice_at
        .expect("cancelled tail discarded its horizon fence");
    let replay_xdv = replay.xdv.unwrap();
    let complete = assembler
        .assemble(
            Some(&foreground_document),
            DocumentFragment::Tail {
                prefix_len: splice_at,
                bytes: replay_xdv.as_ref(),
            },
        )
        .unwrap();
    assert_eq!(complete.bytes().as_ref(), expected);
    runner.finish().unwrap();
}

#[test]
fn length_changing_child_horizon_resumes_exactly() {
    let _serial = SESSION_TEST_LOCK.lock().unwrap();
    util::set_test_root();
    let primary = b"\\input child\n\\bye\n";
    let original_child = "A\\vfill\\eject\nB\\vfill\\eject\nC\\bye\n";
    let edited_child = "Z LONGER EDITED TEXT\\vfill\\eject\nB\\vfill\\eject\nC\\bye\n";
    let expected = cold_xdv_with_child(primary, Some(edited_child.as_bytes()));
    let temp = tempfile::tempdir().unwrap();
    let root = DocumentRoot::new(temp.path()).unwrap();
    let mut sources = SourceSet::initialize(
        root,
        "main.tex",
        String::from_utf8(primary.to_vec()).unwrap(),
        vec![SourceFile {
            path: "child.tex".into(),
            text: original_child.into(),
        }],
        ProtocolLimits::default(),
    )
    .unwrap();
    let support = ResidentEngineSupport::Fixture {
        format_path: util::ensure_plain_format().unwrap(),
        assets_dir: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/assets"),
    };
    let cancellation = EngineCancellation::new();
    let mut runner = ResidentXeTeX::new(support, EnginePreviewConfig::new("plain.fmt"));
    let assembler = DocumentAssembler::new();
    let initial = runner
        .compile(sources.snapshot(), None, &cancellation)
        .unwrap();
    let initial_xdv = initial.xdv.unwrap();
    let initial_document = assembler
        .assemble(None, DocumentFragment::Complete(&initial_xdv))
        .unwrap();

    sources
        .replace(Some("child.tex"), edited_child.into())
        .unwrap();
    let foreground = runner
        .compile(sources.snapshot(), Some((0, 0)), &cancellation)
        .unwrap();
    assert!(foreground.frame.tail_pending);
    assert_eq!(foreground.frame.pages.len(), 1);
    let foreground_hit = foreground
        .frame
        .sync_index
        .forward("child", 1)
        .expect("fenced child foreground lost source geometry");
    assert_eq!(foreground_hit.page_index, 0);
    let inverse = foreground
        .frame
        .sync_index
        .backward(
            foreground_hit.page_index,
            foreground_hit.h,
            foreground_hit.v,
        )
        .expect("fenced child foreground lost inverse geometry");
    assert!(inverse.path.ends_with("child.tex"));
    assert_eq!(inverse.line, 1);
    let foreground_xdv = foreground.xdv.unwrap();
    let foreground_fragment = foreground.frame.splice_at.map_or(
        DocumentFragment::Complete(foreground_xdv.as_ref()),
        |prefix_len| DocumentFragment::Tail {
            prefix_len,
            bytes: foreground_xdv.as_ref(),
        },
    );
    let foreground_document = assembler
        .assemble(Some(&initial_document), foreground_fragment)
        .unwrap();

    let tail = runner
        .continue_tail(&cancellation)
        .unwrap()
        .expect("child horizon did not retain a continuation");
    assert_eq!(
        tail.frame
            .sync_index
            .forward("child.tex", 3)
            .expect("child continuation lost source geometry")
            .page_index,
        2
    );
    let tail_xdv = tail.xdv.unwrap();
    let splice_at = tail
        .frame
        .splice_at
        .expect("child continuation lost its replay splice");
    let complete = assembler
        .assemble(
            Some(&foreground_document),
            DocumentFragment::Tail {
                prefix_len: splice_at,
                bytes: tail_xdv.as_ref(),
            },
        )
        .unwrap();
    assert_eq!(complete.bytes().as_ref(), expected);
    runner.finish().unwrap();
}

#[test]
fn alternating_editor_children_and_primary_share_one_snapshot_revision() {
    let _serial = SESSION_TEST_LOCK.lock().unwrap();
    util::set_test_root();
    let original_primary = "\\input one\n\\input two\nPRIMARY\n\\bye\n";
    let edited_primary = "\\input one\n\\input two\nPRIMARY EDITED\n\\bye\n";
    let one = "ONE\n";
    let one_edited = "ONE EDITED\n";
    let two = "TWO\n";
    let two_edited = "TWO LONGER EDITED\n";
    let expected_one = cold_output_with_children(
        original_primary.as_bytes(),
        &[
            ("one.tex", one_edited.as_bytes()),
            ("two.tex", two.as_bytes()),
        ],
        "session.xdv",
    );
    let expected_two = cold_output_with_children(
        original_primary.as_bytes(),
        &[
            ("one.tex", one_edited.as_bytes()),
            ("two.tex", two_edited.as_bytes()),
        ],
        "session.xdv",
    );
    let expected_primary = cold_output_with_children(
        edited_primary.as_bytes(),
        &[
            ("one.tex", one_edited.as_bytes()),
            ("two.tex", two_edited.as_bytes()),
        ],
        "session.xdv",
    );
    let temp = tempfile::tempdir().unwrap();
    let root = DocumentRoot::new(temp.path()).unwrap();
    let mut sources = SourceSet::initialize(
        root,
        "main.tex",
        original_primary.into(),
        vec![
            SourceFile {
                path: "one.tex".into(),
                text: one.into(),
            },
            SourceFile {
                path: "two.tex".into(),
                text: two.into(),
            },
        ],
        ProtocolLimits::default(),
    )
    .unwrap();
    let support = ResidentEngineSupport::Fixture {
        format_path: util::ensure_plain_format().unwrap(),
        assets_dir: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/assets"),
    };
    let cancellation = EngineCancellation::new();
    let mut runner = ResidentXeTeX::new(support, EnginePreviewConfig::new("plain.fmt"));
    let assembler = DocumentAssembler::new();
    let mut document = assemble_resident_output(
        &assembler,
        None,
        runner
            .compile(sources.snapshot(), None, &cancellation)
            .unwrap(),
    );

    sources.replace(Some("one"), one_edited.into()).unwrap();
    let output = runner
        .compile(sources.snapshot(), None, &cancellation)
        .unwrap();
    assert_eq!(output.frame.source_revision.get(), 1);
    document = assemble_resident_output(&assembler, Some(&document), output);
    assert_eq!(document.bytes().as_ref(), expected_one);

    sources.replace(Some("two.tex"), two_edited.into()).unwrap();
    let output = runner
        .compile(sources.snapshot(), None, &cancellation)
        .unwrap();
    assert_eq!(output.frame.source_revision.get(), 2);
    document = assemble_resident_output(&assembler, Some(&document), output);
    assert_eq!(document.bytes().as_ref(), expected_two);

    sources.replace(None, edited_primary.into()).unwrap();
    let output = runner
        .compile(sources.snapshot(), None, &cancellation)
        .unwrap();
    assert_eq!(output.frame.source_revision.get(), 3);
    document = assemble_resident_output(&assembler, Some(&document), output);
    assert_eq!(document.bytes().as_ref(), expected_primary);
    runner.finish().unwrap();
}

#[test]
fn resident_processor_publishes_engine_output_as_svg_frame() {
    struct ClassicResources;

    impl ResourceProvider for ClassicResources {
        fn resolve(&self, request: &ResourceRequest) -> Result<Option<Resource>, RenderError> {
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let path = match request.kind {
                ResourceKind::Tfm => root.join("tests/assets/cmr10.tfm"),
                ResourceKind::FontProgram => root.join("tests/assets/cmr10.pfb"),
                ResourceKind::Encoding => return Ok(None),
                _ => return Ok(None),
            };
            Ok(Some(Resource::from_bytes(std::fs::read(path).unwrap())))
        }
    }

    let _serial = SESSION_TEST_LOCK.lock().unwrap();
    util::set_test_root();
    let temp = tempfile::tempdir().unwrap();
    let root = DocumentRoot::new(temp.path()).unwrap();
    let mut sources = SourceSet::initialize(
        root,
        "main.tex",
        "A\\vfill\\eject\nB\\vfill\\eject\nC\\vfill\\eject\nD\\bye\n".into(),
        vec![],
        ProtocolLimits::default(),
    )
    .unwrap();
    let support = ResidentEngineSupport::Fixture {
        format_path: util::ensure_plain_format().unwrap(),
        assets_dir: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/assets"),
    };
    let cancellation = EngineCancellation::new();
    let engine = ResidentXeTeX::new(support, EnginePreviewConfig::new("plain.fmt"));
    let sink = JsonlFrameSink::new(Vec::new(), ProtocolLimits::default());
    let mut processor = ResidentPreviewProcessor::new(engine, ClassicResources, NullBackend, sink);

    let first = processor.compile(sources.snapshot(), RequestSeq::ZERO, None, &cancellation);
    assert!(matches!(
        first,
        WorkerRun::Complete(frame) if frame.frame_id.get() == 1
    ));
    sources
        .replace(
            None,
            "Z\\vfill\\eject\nB\\vfill\\eject\nC\\vfill\\eject\nD\\bye\n".into(),
        )
        .unwrap();
    let second = processor.compile(
        sources.snapshot(),
        RequestSeq::new(1),
        Some((0, 0)),
        &cancellation,
    );
    assert!(matches!(
        second,
        WorkerRun::Complete(frame) if frame.frame_id.get() == 2 && frame.tail_pending
    ));
    let fresh = processor
        .forward_search("main", 1)
        .expect("foreground source geometry was not published");
    assert_eq!(fresh.page_index, 0);
    let preserved = processor
        .forward_search("main.tex", 4)
        .expect("old tail source geometry was not preserved");
    assert_eq!(preserved.page_index, 3);
    assert_eq!(
        processor
            .backward_search(preserved.page_index, preserved.h_bp, preserved.v_bp)
            .expect("preserved tail geometry was not backward searchable")
            .line,
        4
    );
    assert!(processor.has_pending_tail());
    let settled = processor.continue_tail(RequestSeq::new(1), &cancellation);
    assert!(matches!(
        settled,
        WorkerRun::Complete(Some(frame))
            if frame.frame_id.get() == 3 && !frame.tail_pending
    ));
    assert_eq!(
        processor
            .forward_search("main.tex", 4)
            .expect("settled source geometry was not published")
            .page_index,
        3
    );
    sources
        .replace(
            None,
            "% harmless\nZ\\vfill\\eject\nB\\vfill\\eject\nC\\vfill\\eject\nD\\bye\n".into(),
        )
        .unwrap();
    assert!(matches!(
        processor.compile(
            sources.snapshot(),
            RequestSeq::new(2),
            Some((0, 0)),
            &cancellation,
        ),
        WorkerRun::Complete(frame) if frame.frame_id.get() == 4 && frame.tail_pending
    ));
    assert!(matches!(
        processor.continue_tail(RequestSeq::new(2), &cancellation),
        WorkerRun::Complete(Some(frame))
            if frame.frame_id.get() == 5 && !frame.tail_pending
    ));
    processor
        .resync(sources.snapshot(), RequestSeq::new(2))
        .unwrap();

    let wire = processor.sink().writer();
    let events = wire
        .split(|&byte| byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice::<ServerEvent>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(events.len(), 6);
    assert!(matches!(
        &events[1],
        ServerEvent::Update {
            phase: FramePhase::Foreground,
            tail_pending: true,
            page_count: 4,
            changed,
            removed,
            ..
        } if changed == &[0] && removed.is_empty()
    ));
    assert!(matches!(
        &events[2],
        ServerEvent::Update {
            phase: FramePhase::TailSettle,
            tail_pending: false,
            page_count: 4,
            ..
        }
    ));
    assert!(matches!(
        &events[3],
        ServerEvent::Update {
            phase: FramePhase::Foreground,
            tail_pending: true,
            changed,
            pages_svg,
            ..
        } if changed.is_empty() && pages_svg.is_empty()
    ));
    assert!(matches!(
        &events[4],
        ServerEvent::Update {
            phase: FramePhase::TailSettle,
            tail_pending: false,
            changed,
            pages_svg,
            ..
        } if changed.is_empty() && pages_svg.is_empty()
    ));
    processor.shutdown().unwrap();
}

#[test]
fn editor_owned_child_edits_are_read_from_the_snapshot() {
    let _serial = SESSION_TEST_LOCK.lock().unwrap();
    util::set_test_root();
    let primary = b"\\input child\n\\bye\n";
    let cold_first = cold_xdv_with_child(primary, Some(b"A\n"));
    let cold_second = cold_xdv_with_child(primary, Some(b"B\n"));
    let sources = SourceSnapshot::new(
        PathBuf::from("/workspace"),
        PathBuf::from("session.tex"),
        vec![
            (PathBuf::from("session.tex"), Arc::from(&primary[..])),
            (PathBuf::from("child.tex"), Arc::from(&b"A\n"[..])),
        ],
    )
    .unwrap();
    let mut format =
        SingleInputFileIo::new(&util::ensure_plain_format().expect("could not build plain format"));
    let mut session = IncrementalXeTeXSession::new(EnginePreviewConfig::new("plain.fmt")).unwrap();
    let mut first_output = MemoryIo::new(true);
    {
        let io = IoStack::new(vec![
            &mut first_output as &mut dyn IoProvider,
            &mut format as &mut dyn IoProvider,
        ]);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        session.open(&mut launcher, sources.clone()).unwrap();
    }
    assert_eq!(output_bytes(&first_output, "session.xdv"), cold_first);

    let update = sources.replace("child", Arc::from(&b"B\n"[..])).unwrap();
    let mut second_output = MemoryIo::new(true);
    {
        let io = IoStack::new(vec![&mut second_output as &mut dyn IoProvider]);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        session.edit(&mut launcher, update.snapshot, None).unwrap();
        session.finish(&mut launcher).unwrap();
    }
    assert_eq!(output_bytes(&second_output, "session.xdv"), cold_second);
}

#[test]
fn resident_synctex_index_stays_in_memory_and_tracks_children() {
    let _serial = SESSION_TEST_LOCK.lock().unwrap();
    util::set_test_root();
    let primary = b"\\input child\nMAIN SOURCE LINE\\par\n\\bye\n";
    let child = b"CHILD SOURCE LINE\\par\n";
    let sources = snapshot_with_child(primary, Some(child));
    let mut format =
        SingleInputFileIo::new(&util::ensure_plain_format().expect("could not build plain format"));
    let mut session = IncrementalXeTeXSession::new(EnginePreviewConfig::new("plain.fmt")).unwrap();
    let mut output = MemoryIo::new(true);
    let frame = {
        let io = IoStack::new(vec![
            &mut output as &mut dyn IoProvider,
            &mut format as &mut dyn IoProvider,
        ]);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        let frame = session.open(&mut launcher, sources).unwrap();
        session.finish(&mut launcher).unwrap();
        frame
    };

    let child_hit = frame
        .sync_index
        .forward("child", 1)
        .expect("child source line was not indexed");
    assert_eq!(child_hit.page_index, 0);
    assert_eq!(child_hit.line, 1);
    assert!(frame.sync_index.forward("session.tex", 2).is_some());
    let backward = frame
        .sync_index
        .backward(child_hit.page_index, child_hit.h, child_hit.v)
        .expect("indexed child geometry was not searchable");
    assert!(backward.path.ends_with("child.tex"));
    assert_eq!(backward.line, 1);
    assert!(
        output
            .files
            .borrow()
            .keys()
            .all(|name| !name.ends_with(".synctex.gz")),
        "resident SyncTeX must not create a serialized sidecar"
    );
}

#[test]
fn bom_crlf_and_utf16_inputs_match_cold_decoding() {
    let _serial = SESSION_TEST_LOCK.lock().unwrap();
    util::set_test_root();
    let cases: Vec<Vec<u8>> = vec![
        b"\xEF\xBB\xBFA\r\n\\bye\r\n".to_vec(),
        "A\r\n\\bye\r\n"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect(),
    ];
    for (index, bytes) in cases.into_iter().enumerate() {
        let bytes = if index == 1 {
            [vec![0xFF, 0xFE], bytes].concat()
        } else {
            bytes
        };
        let expected = cold_xdv(&bytes);
        let sources = snapshot(&bytes);
        let mut format = SingleInputFileIo::new(
            &util::ensure_plain_format().expect("could not build plain format"),
        );
        let mut output = MemoryIo::new(true);
        let mut session =
            IncrementalXeTeXSession::new(EnginePreviewConfig::new("plain.fmt")).unwrap();
        {
            let io = IoStack::new(vec![
                &mut output as &mut dyn IoProvider,
                &mut format as &mut dyn IoProvider,
            ]);
            let mut hooks = MinimalDriver::new(io);
            let mut status = NoopStatusBackend::default();
            let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
            session.open(&mut launcher, sources).unwrap();
            session.finish(&mut launcher).unwrap();
        }
        assert_eq!(output_bytes(&output, "session.xdv"), expected);
    }
}

#[test]
fn page_manifest_matches_final_multi_page_xdv_spans() {
    let _serial = SESSION_TEST_LOCK.lock().unwrap();
    util::set_test_root();
    let input = b"A\\vfill\\eject\nB\\vfill\\eject\nC\\bye\n";
    let mut format =
        SingleInputFileIo::new(&util::ensure_plain_format().expect("could not build plain format"));
    let mut output = MemoryIo::new(true);
    let mut session = IncrementalXeTeXSession::new(EnginePreviewConfig::new("plain.fmt")).unwrap();
    let frame;
    {
        let io = IoStack::new(vec![
            &mut output as &mut dyn IoProvider,
            &mut format as &mut dyn IoProvider,
        ]);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        frame = session.open(&mut launcher, snapshot(input)).unwrap();
        session.finish(&mut launcher).unwrap();
    }

    let xdv = output_bytes(&output, "session.xdv");
    assert_eq!(frame.pages.len(), 3);
    let mut expected_start = 0usize;
    let mut previous_clock = 0;
    for page in frame.pages {
        let start = page.span_start as usize;
        let end = page.span_end as usize;
        assert_eq!(start, expected_start);
        assert!(start <= page.bop_offset as usize);
        assert_eq!(xdv[page.bop_offset as usize], 139);
        let expected_digest: [u8; 32] = Sha256::digest(&xdv[start..end]).into();
        assert_eq!(page.byte_digest, expected_digest);
        assert!(page.logical_clock > previous_clock);
        assert!(page.synctex_range.is_none());
        expected_start = end;
        previous_clock = page.logical_clock;
    }
}
