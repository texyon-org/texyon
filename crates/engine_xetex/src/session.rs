//! Synchronous rung-zero resident XeTeX session.

use std::ffi::CString;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

use tectonic_bridge_core::{CoreBridgeLauncher, CoreBridgeState};
use tectonic_errors::prelude::*;

use crate::checkpoint::{
    BudgetError, CheckpointLadder, EngineBundle, LadderBudget, StringWatermark,
};
use crate::state::{EngineExecutionMode, EngineState};
use crate::{PageExecution, SourceRevision, SourceSnapshot, TexOutcome};

static RESIDENT_ACTIVE: AtomicBool = AtomicBool::new(false);

pub(crate) fn resident_active() -> bool {
    RESIDENT_ACTIVE.load(Ordering::Acquire)
}

/// Page interval requested by a preview client. Rung-zero replay currently
/// compiles the complete document; horizon handling consumes this in M8.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageRange {
    /// First visible page, zero-based and inclusive.
    pub first: usize,
    /// Last visible page, zero-based and inclusive.
    pub last: usize,
}

/// Configuration fixed for the lifetime of one resident engine.
#[derive(Clone, Debug)]
pub struct EnginePreviewConfig {
    /// Format filename resolved through the caller's support I/O stack.
    pub format_file_name: String,
    /// Engine-visible reproducible build date.
    pub build_date: SystemTime,
    /// Pin clock, timezone, and random-seed inputs.
    pub deterministic: bool,
    /// Enable expensive named rewind verification.
    pub verify_rewinds: bool,
    /// Soft and hard checkpoint memory limits.
    pub ladder_target_bytes: usize,
    /// Absolute checkpoint memory limit.
    pub ladder_hard_bytes: usize,
    /// Recent rung count protected while satisfying the soft target.
    pub min_recent_rungs: usize,
}

impl EnginePreviewConfig {
    /// Construct a deterministic preview configuration for `format_file_name`.
    pub fn new(format_file_name: impl Into<String>) -> Self {
        Self {
            format_file_name: format_file_name.into(),
            build_date: SystemTime::UNIX_EPOCH,
            deterministic: true,
            verify_rewinds: false,
            ladder_target_bytes: 64 * 1024 * 1024,
            ladder_hard_bytes: 96 * 1024 * 1024,
            min_recent_rungs: 8,
        }
    }
}

/// Result of one complete synchronous resident operation.
#[derive(Clone, Debug)]
pub struct EngineFrame {
    /// Engine warning/error outcome.
    pub outcome: TexOutcome,
    /// Exact immutable sources consumed by this operation.
    pub source_revision: SourceRevision,
    /// Recoverable fatal-class TeX abort; prior good output remains authoritative.
    pub aborted: Option<String>,
    /// Page-boundary execution facts for exact XDV suffix comparison.
    pub pages: Vec<PageExecution>,
}

/// One initialized XeTeX state repeatedly replayed from rung zero.
///
/// XeTeX still owns process-global native state, so at most one resident
/// session may be open in a process. Call [`finish`](Self::finish) before
/// dropping the session so that those native resources are released.
pub struct IncrementalXeTeXSession {
    config: EnginePreviewConfig,
    state: Box<EngineState>,
    ladder: Option<CheckpointLadder<EngineBundle>>,
    strings_base: Option<StringWatermark>,
    opened: bool,
    dead: bool,
    owns_global_lease: bool,
    _not_send: PhantomData<Rc<()>>,
}

impl IncrementalXeTeXSession {
    /// Create an unopened resident session. Format I/O is deferred until
    /// [`open`](Self::open), where it runs under the process-global lock.
    pub fn new(config: EnginePreviewConfig) -> Result<Self> {
        CString::new(config.format_file_name.as_str())?;
        if config.ladder_target_bytes > config.ladder_hard_bytes {
            bail!("checkpoint target exceeds hard cap");
        }
        Ok(Self {
            config,
            state: Box::new(EngineState::default()),
            ladder: None,
            strings_base: None,
            opened: false,
            dead: false,
            owns_global_lease: false,
            _not_send: PhantomData,
        })
    }

    /// Initialize the format and compile the first immutable source snapshot.
    pub fn open(
        &mut self,
        launcher: &mut CoreBridgeLauncher,
        sources: SourceSnapshot,
    ) -> Result<EngineFrame> {
        if self.dead {
            bail!("resident XeTeX session is dead");
        }
        if self.opened {
            bail!("resident XeTeX session is already open");
        }
        let format = CString::new(self.config.format_file_name.as_str())?;
        let input = CString::new(sources.primary().as_str())?;
        let build_date = self
            .config
            .build_date
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_| anyhow!("preview build date precedes Unix epoch"))?
            .as_secs() as libc::time_t;
        if RESIDENT_ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            bail!("another resident XeTeX session is already open");
        }
        self.owns_global_lease = true;
        let budget = self.budget();
        let verify = self.config.verify_rewinds;
        let config = self.config.clone();
        let state = &mut *self.state;
        let ladder = &mut self.ladder;
        let strings_base = &mut self.strings_base;
        let revision = sources.revision();
        let result = launcher.with_global_lock(|api| {
            Ok(catch_operation(api, state, |state| unsafe {
                configure(state, &config);
                let _sources = crate::replay::adapter::install(sources);
                match crate::engine::initialize_engine(state, format.as_ptr(), build_date) {
                    crate::engine::InitOutcome::Ready => {}
                    crate::engine::InitOutcome::Terminate(history) => {
                        return OperationResult::Terminal(history)
                    }
                }
                state.config.execution_mode = EngineExecutionMode::Resident;
                crate::journal::arm(state);
                let base = StringWatermark::capture(state);
                let mut new_ladder = match CheckpointLadder::from_engine_with_budget(
                    state, base, budget,
                ) {
                    Ok(ladder) => ladder,
                    Err(error) => return OperationResult::Budget(error),
                };
                if verify {
                    new_ladder.enable_verification(state);
                }
                *strings_base = Some(base);
                *ladder = Some(new_ladder);
                run_document(state, ladder.as_mut().unwrap(), input.as_ptr())
            }))
        });
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                self.release_global_lease();
                return Err(error);
            }
        };
        self.opened = self.ladder.is_some();
        self.finish_operation(result, revision)
    }

    /// Replace the immutable source set and fully replay from rung zero.
    /// `viewport` is accepted now so the API does not change when M8 adds a
    /// foreground horizon; it does not shorten execution yet.
    pub fn edit(
        &mut self,
        launcher: &mut CoreBridgeLauncher,
        sources: SourceSnapshot,
        _viewport: Option<PageRange>,
    ) -> Result<EngineFrame> {
        if self.dead {
            bail!("resident XeTeX session is dead");
        }
        if !self.opened {
            bail!("resident XeTeX session was edited before open");
        }
        let input = CString::new(sources.primary().as_str())?;
        let revision = sources.revision();
        let config = self.config.clone();
        let state = &mut *self.state;
        let ladder = self.ladder.as_mut().expect("opened session without ladder");
        let result = launcher.with_global_lock(|api| {
            Ok(catch_operation(api, state, |state| unsafe {
                let _sources = crate::replay::adapter::install(sources);
                ladder.rewind_engine(state, 0);
                configure(state, &config);
                state.shipout.rust_stdout = crate::bridge::ttstub_output_open_stdout();
                run_document(state, ladder, input.as_ptr())
            }))
        })?;
        self.finish_operation(result, revision)
    }

    /// Release process-global native resources and all initialized arenas.
    pub fn finish(&mut self, launcher: &mut CoreBridgeLauncher) -> Result<()> {
        if self.dead {
            return Ok(());
        }
        if !self.opened {
            self.dead = true;
            return Ok(());
        }
        let state = &mut *self.state;
        let ladder = self.ladder.as_mut().expect("opened session without ladder");
        let teardown = launcher.with_global_lock(|api| {
            Ok(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run_locked(api, state, |state| unsafe {
                    crate::engine::release_operation_inputs(state);
                    ladder.rewind_engine(state, 0);
                    crate::engine::tt_cleanup_in(state);
                })
            }))
            .is_err())
        });
        self.dead = true;
        self.release_global_lease();
        let panicked = teardown?;
        if panicked {
            bail!("resident XeTeX session panicked during teardown");
        }
        Ok(())
    }

    /// Whether a panic or explicit finish made the session unusable.
    pub fn is_dead(&self) -> bool {
        self.dead
    }

    fn budget(&self) -> LadderBudget {
        LadderBudget {
            target_bytes: self.config.ladder_target_bytes,
            hard_bytes: self.config.ladder_hard_bytes,
            min_recent_rungs: self.config.min_recent_rungs,
        }
    }

    fn finish_operation(
        &mut self,
        result: OperationResult,
        revision: SourceRevision,
    ) -> Result<EngineFrame> {
        match result {
            OperationResult::Complete(history) => Ok(EngineFrame {
                outcome: history_to_outcome(history)?,
                source_revision: revision,
                aborted: None,
                pages: self.state.shipout.page_manifest.take_pages(),
            }),
            OperationResult::Aborted => Ok(EngineFrame {
                outcome: TexOutcome::Errors,
                source_revision: revision,
                aborted: Some("XeTeX aborted; resident state rewound to rung zero".into()),
                pages: Vec::new(),
            }),
            OperationResult::Budget(error) => {
                self.dead = true;
                self.release_global_lease();
                Err(anyhow!("invalid checkpoint budget: {error:?}"))
            }
            OperationResult::Terminal(history) => {
                self.dead = true;
                self.release_global_lease();
                Err(anyhow!("XeTeX initialization terminated with history {history}"))
            }
            OperationResult::Panicked => {
                self.dead = true;
                self.release_global_lease();
                Err(anyhow!("resident XeTeX operation panicked"))
            }
        }
    }

    fn release_global_lease(&mut self) {
        if self.owns_global_lease {
            self.owns_global_lease = false;
            RESIDENT_ACTIVE.store(false, Ordering::Release);
        }
    }
}

impl Drop for IncrementalXeTeXSession {
    fn drop(&mut self) {
        self.release_global_lease();
    }
}

enum OperationResult {
    Complete(i32),
    Aborted,
    Budget(BudgetError),
    Terminal(i32),
    Panicked,
}

unsafe fn run_document(
    state: &mut EngineState,
    ladder: &mut CheckpointLadder<EngineBundle>,
    input: *const libc::c_char,
) -> OperationResult {
    state.shipout.page_manifest.begin_operation();
    match crate::engine::run_resident_document(state, input) {
        Ok(history) => OperationResult::Complete(history),
        Err(_) => {
            crate::engine::release_operation_inputs(state);
            ladder.rewind_engine(state, 0);
            OperationResult::Aborted
        }
    }
}

fn configure(state: &mut EngineState, config: &EnginePreviewConfig) {
    state.config.halt_on_error_p = 0;
    state.config.in_initex_mode = false;
    state.config.synctex_enabled = 0;
    state.config.semantic_pagination_enabled = false;
    state.config.shell_escape_enabled = false;
    state.config.deterministic_mode = config.deterministic;
    state.config.execution_mode = EngineExecutionMode::Resident;
}

fn history_to_outcome(history: i32) -> Result<TexOutcome> {
    match history {
        0 => Ok(TexOutcome::Spotless),
        1 => Ok(TexOutcome::Warnings),
        2 => Ok(TexOutcome::Errors),
        value => Err(anyhow!("unexpected XeTeX history value {value}")),
    }
}

fn catch_operation(
    api: &mut CoreBridgeState<'_>,
    state: &mut EngineState,
    operation: impl FnOnce(&mut EngineState) -> OperationResult,
) -> OperationResult {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_locked(api, state, operation)
    }))
    .unwrap_or_else(|_| {
        crate::engine::reset_layout_font_caches();
        OperationResult::Panicked
    })
}

fn run_locked<R>(
    api: &mut CoreBridgeState<'_>,
    state: &mut EngineState,
    operation: impl FnOnce(&mut EngineState) -> R,
) -> R {
    let api = api as *mut CoreBridgeState<'_> as *mut libc::c_void;
    struct EngineGuard;
    impl Drop for EngineGuard {
        fn drop(&mut self) {
            unsafe { crate::bridge::exit_global_engine() };
        }
    }
    struct BridgeGuard;
    impl Drop for BridgeGuard {
        fn drop(&mut self) {
            unsafe { crate::bridge::set_bridge_state(std::ptr::null_mut()) };
        }
    }
    struct StateGuard;
    impl Drop for StateGuard {
        fn drop(&mut self) {
            crate::state::set_engine_state_ptr(std::ptr::null_mut());
        }
    }
    unsafe {
        crate::bridge::enter_global_engine(api);
        let _engine = EngineGuard;
        crate::bridge::set_bridge_state(api);
        let _bridge = BridgeGuard;
        crate::state::set_engine_state_ptr(state as *mut EngineState);
        let _state = StateGuard;
        operation(state)
    }
}
