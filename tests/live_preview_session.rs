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
use tectonic_bridge_core::{CoreBridgeLauncher, MinimalDriver};
use tectonic_io_base::stdstreams::BufferedPrimaryIo;
use tectonic_status_base::NoopStatusBackend;

#[path = "util/mod.rs"]
mod util;

static SESSION_TEST_LOCK: Mutex<()> = Mutex::new(());

fn snapshot(bytes: &[u8]) -> SourceSnapshot {
    SourceSnapshot::new(
        PathBuf::from("/workspace"),
        PathBuf::from("session.tex"),
        vec![(PathBuf::from("session.tex"), Arc::from(bytes))],
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
    let mut format =
        SingleInputFileIo::new(&util::ensure_plain_format().expect("could not build plain format"));
    let mut primary = BufferedPrimaryIo::from_buffer(bytes.to_vec());
    let mut output = MemoryIo::new(true);
    if let Some(child) = child {
        output.create_entry("child.tex", child.to_vec());
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
    output_bytes(&output, "session.xdv")
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
