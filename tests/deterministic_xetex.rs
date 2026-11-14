//! Integration coverage for XeTeX's deterministic hidden-input contract.

use std::collections::HashSet;
use std::time::{Duration, SystemTime};

use texyon::TexEngine;
use texyon::io::testing::SingleInputFileIo;
use texyon::io::{FilesystemIo, FilesystemPrimaryInputIo, IoProvider, IoStack, MemoryIo};
use texyon_bridge_core::{CoreBridgeLauncher, MinimalDriver};
use texyon_status_base::NoopStatusBackend;

#[path = "util/mod.rs"]
mod util;
use util::{ensure_plain_format, test_path};

const SOURCE: &str = r#"\immediate\write16{CLOCK=\the\year/\the\month/\the\day/\the\time}
\immediate\write16{SEED=\the\randomseed}
\immediate\write16{ELAPSED=\the\elapsedtime}
\resettimer
\immediate\write16{RESET=\the\elapsedtime}
\setrandomseed 1234
\immediate\write16{OVERRIDE=\the\randomseed}
\bye
"#;

struct TimezoneGuard(Option<std::ffi::OsString>);

impl TimezoneGuard {
    fn set(tz: &str) -> Self {
        let previous = std::env::var_os("TZ");
        // SAFETY: this binary has one test, so no other thread reads the environment.
        unsafe { std::env::set_var("TZ", tz) };
        refresh_timezone();
        Self(previous)
    }
}

impl Drop for TimezoneGuard {
    fn drop(&mut self) {
        // SAFETY: this binary has one test, so no other thread reads the environment.
        unsafe {
            match self.0.take() {
                Some(value) => std::env::set_var("TZ", value),
                None => std::env::remove_var("TZ"),
            }
        }
        refresh_timezone();
    }
}

#[cfg(unix)]
fn refresh_timezone() {
    unsafe extern "C" {
        fn tzset();
    }

    // SAFETY: tzset rereads TZ, and this binary has one test, so no thread changes it meanwhile.
    unsafe { tzset() };
}

#[cfg(not(unix))]
fn refresh_timezone() {}

fn run_clock_fixture(tz: &str, epoch: u64, deterministic: bool) -> Vec<String> {
    util::set_test_root();
    let _timezone = TimezoneGuard::set(tz);
    let tempdir = tempfile::tempdir().unwrap();
    let primary_path = tempdir.path().join("clock.tex");
    std::fs::write(&primary_path, SOURCE).unwrap();

    let mut output = MemoryIo::new(true);
    let mut primary = FilesystemPrimaryInputIo::new(primary_path);
    let mut format = SingleInputFileIo::new(&ensure_plain_format().unwrap());
    let mut support = FilesystemIo::new(&test_path(&["assets"]), false, false, HashSet::new());

    {
        let providers: Vec<&mut dyn IoProvider> =
            vec![&mut output, &mut primary, &mut format, &mut support];
        let mut hooks = MinimalDriver::new(IoStack::new(providers));
        let mut status = NoopStatusBackend::default();
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);

        TexEngine::default()
            .deterministic_mode(deterministic)
            .build_date(SystemTime::UNIX_EPOCH + Duration::from_secs(epoch))
            .process(&mut launcher, "plain.fmt", "clock.tex")
            .unwrap();
    }

    let files = output.files.borrow();
    let log = String::from_utf8(files["clock.log"].data.clone()).unwrap();
    log.lines()
        .filter(|line| {
            ["CLOCK=", "SEED=", "ELAPSED=", "RESET=", "OVERRIDE="]
                .iter()
                .any(|prefix| line.starts_with(prefix))
        })
        .map(str::to_owned)
        .collect()
}

#[test]
fn deterministic_mode_pins_timezone_seed_and_timers() {
    for epoch in [1_456_304_492, 1_700_000_000] {
        let utc = run_clock_fixture("UTC0", epoch, true);
        let pacific = run_clock_fixture("PST8PDT", epoch, true);

        assert_eq!(utc, pacific);
        assert!(utc.iter().any(|line| line == "ELAPSED=0"));
        assert!(utc.iter().any(|line| line == "RESET=0"));
        assert!(utc.iter().any(|line| line == "OVERRIDE=1234"));

        let live_utc = run_clock_fixture("UTC0", epoch, false);
        let live_pacific = run_clock_fixture("PST8PDT", epoch, false);
        assert_ne!(live_utc[0], live_pacific[0]);
        assert!(live_utc.iter().any(|line| line == "OVERRIDE=1234"));
        assert!(live_pacific.iter().any(|line| line == "OVERRIDE=1234"));
    }
}
