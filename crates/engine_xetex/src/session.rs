//! Synchronous rung-zero resident XeTeX session.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

use tectonic_bridge_core::{CoreBridgeLauncher, CoreBridgeState};
use tectonic_errors::prelude::*;

use crate::checkpoint::{
    BudgetError, CheckpointLadder, EngineBundle, LadderBudget, RungAdmission, StringWatermark,
};
use crate::errors::Abort;
use crate::replay::source::SourcePath;
use crate::state::{EngineExecutionMode, EngineState};
use crate::EngineCancellation;
use crate::{PageExecution, SourceRevision, SourceSnapshot, TexOutcome};

static RESIDENT_ACTIVE: AtomicBool = AtomicBool::new(false);

// Command-count fallback for documents without edit or page-horizon fences.
const PERIODIC_FENCE_TOKENS: u64 = 500_000;
// An early first rung by source progress, then coarser ones to bound capture work.
const FIRST_PERIODIC_FENCE_SOURCE_BYTES: usize = 32 * 1024;
const PERIODIC_FENCE_SOURCE_BYTES: usize = 256 * 1024;
const MAX_EDIT_FENCES: usize = 16;

#[derive(Clone, Copy, Debug, Default)]
struct CheckpointActivity {
    admitted: usize,
    dropped: usize,
    rejected: usize,
}

struct FenceController {
    ladder: NonNull<CheckpointLadder<EngineBundle>>,
    snapshot: NonNull<SourceSnapshot>,
    strings_base: StringWatermark,
    budget: LadderBudget,
    next_clock: u64,
    next_source_bytes: usize,
    pending_fences: Vec<(SourcePath, Vec<usize>)>,
    horizon_last: Option<usize>,
    horizon_rung: NonNull<Option<usize>>,
    activity: NonNull<CheckpointActivity>,
}

/// Where an operation may place checkpoint fences.
struct FencePlan {
    strings_base: StringWatermark,
    budget: LadderBudget,
    pending_fences: Vec<(SourcePath, Vec<usize>)>,
    horizon_last: Option<usize>,
}

thread_local! {
    static FENCE_CONTROLLER: RefCell<Option<FenceController>> = const { RefCell::new(None) };
}

struct FenceControllerGuard;

impl Drop for FenceControllerGuard {
    fn drop(&mut self) {
        FENCE_CONTROLLER.with(|controller| {
            controller.borrow_mut().take();
        });
    }
}

fn install_fence_controller(
    ladder: &mut CheckpointLadder<EngineBundle>,
    snapshot: &SourceSnapshot,
    plan: FencePlan,
    current_clock: u64,
    horizon_rung: &mut Option<usize>,
    activity: &mut CheckpointActivity,
) -> FenceControllerGuard {
    let FencePlan {
        strings_base,
        budget,
        pending_fences,
        horizon_last,
    } = plan;
    FENCE_CONTROLLER.with(|controller| {
        let previous = controller.borrow_mut().replace(FenceController {
            ladder: NonNull::from(ladder),
            snapshot: NonNull::from(snapshot),
            strings_base,
            budget,
            next_clock: current_clock.saturating_add(PERIODIC_FENCE_TOKENS),
            next_source_bytes: crate::replay::adapter::observed_bytes()
                .saturating_add(FIRST_PERIODIC_FENCE_SOURCE_BYTES),
            pending_fences,
            horizon_last,
            horizon_rung: NonNull::from(horizon_rung),
            activity: NonNull::from(activity),
        });
        assert!(previous.is_none(), "nested resident fence controller");
    });
    FenceControllerGuard
}

/// Capture a protected edit fence before fetching the next command.
///
/// At this point the source adapter still points to the start of the next TeX
/// line, so a subsequent edit within that line can reuse the fence.
pub(crate) fn capture_prefetch_fence(state: &mut EngineState) {
    let _ = capture_fence(state, false, false);
}

/// Capture an undispatched-command checkpoint when the active cadence fires.
pub(crate) fn capture_command_fence(state: &mut EngineState) -> bool {
    capture_fence(state, true, true)
}

fn capture_fence(
    state: &mut EngineState,
    resume_pending_command: bool,
    allow_periodic_or_horizon: bool,
) -> bool {
    FENCE_CONTROLLER.with(|controller| {
        let mut controller = controller.borrow_mut();
        let Some(controller) = controller.as_mut() else {
            return false;
        };
        let horizon_due = allow_periodic_or_horizon
            && controller
                .horizon_last
                .is_some_and(|last| state.shipout.total_pages.max(0) as usize > last);
        let mut protected = false;
        for (path, fences) in &mut controller.pending_fences {
            let Some(position) = crate::replay::adapter::open_position(path) else {
                continue;
            };
            if fences.first().is_some_and(|offset| position >= *offset) {
                fences.retain(|offset| *offset > position);
                protected = true;
            }
        }
        controller
            .pending_fences
            .retain(|(_, fences)| !fences.is_empty());
        let observed_bytes = crate::replay::adapter::observed_bytes();
        let periodic_due = allow_periodic_or_horizon
            && (state.scanner.logical_clock >= controller.next_clock
                || observed_bytes >= controller.next_source_bytes);
        if !horizon_due && !protected && !periodic_due {
            return false;
        }
        controller.next_clock = state
            .scanner
            .logical_clock
            .saturating_add(PERIODIC_FENCE_TOKENS);
        controller.next_source_bytes = observed_bytes.saturating_add(PERIODIC_FENCE_SOURCE_BYTES);
        // SAFETY: the controller is installed around one synchronous engine
        // call on this thread. Both pointees outlive the guard and are not
        // accessed through another reference while this hook runs.
        let snapshot = unsafe { controller.snapshot.as_ref() };
        let Some(fence) =
            crate::replay::fence::ReplayFence::capture(state, snapshot, resume_pending_command)
        else {
            return false;
        };
        // SAFETY: as for `snapshot` above.
        let ladder = unsafe { controller.ladder.as_mut() };
        let admission = ladder.admit_engine_fence(
            state,
            controller.strings_base,
            fence,
            protected || horizon_due,
            controller.budget,
        );
        match admission {
            RungAdmission::Admitted { index, dropped, .. } => {
                // SAFETY: the pointer targets run_document's local activity
                // counters and cannot outlive the controller guard.
                let activity = unsafe { controller.activity.as_mut() };
                activity.admitted += 1;
                activity.dropped += dropped;
                if !horizon_due {
                    return false;
                }
                // SAFETY: the pointer targets run_document's local slot and the
                // controller cannot outlive its guard.
                unsafe { *controller.horizon_rung.as_mut() = Some(index) };
                true
            }
            RungAdmission::RungNotAdmitted { .. } => {
                // SAFETY: see the admitted branch above.
                unsafe { controller.activity.as_mut() }.rejected += 1;
                false
            }
        }
    })
}

pub(crate) fn resident_active() -> bool {
    RESIDENT_ACTIVE.load(Ordering::Acquire)
}

/// Page interval requested by a preview client for foreground horizon work.
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
    /// Evaluate pgfmath natively in floating point. Output may differ from
    /// pgf's fixed-point macros in the last digits, as with luamath.
    pub native_pgfmath: bool,
    /// Soft and hard checkpoint memory limits.
    pub ladder_target_bytes: usize,
    /// Absolute checkpoint memory limit.
    pub ladder_hard_bytes: usize,
    /// Recent rung count protected while satisfying the soft target.
    pub min_recent_rungs: usize,
    /// Cooperative cancellation signal polled by instrumented engine loops.
    pub cancellation: EngineCancellation,
}

impl EnginePreviewConfig {
    /// Construct a deterministic preview configuration for `format_file_name`.
    pub fn new(format_file_name: impl Into<String>) -> Self {
        Self {
            format_file_name: format_file_name.into(),
            build_date: SystemTime::UNIX_EPOCH,
            deterministic: true,
            verify_rewinds: false,
            native_pgfmath: true,
            ladder_target_bytes: 64 * 1024 * 1024,
            ladder_hard_bytes: 96 * 1024 * 1024,
            min_recent_rungs: 8,
            cancellation: EngineCancellation::new(),
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
    /// Recoverable fatal-class TeX abort. Prior good output remains authoritative.
    pub aborted: Option<String>,
    /// Operation was cooperatively cancelled and a safe replay state restored.
    pub cancelled: bool,
    /// Page-boundary execution facts for exact XDV suffix comparison.
    pub pages: Vec<PageExecution>,
    /// In-memory source geometry produced for the operation's coherent coverage.
    pub sync_index: crate::sync_index::SyncIndex,
    /// Old-document byte prefix retained by a fence-local replay.
    pub splice_at: Option<usize>,
    /// Number of checkpoint rungs retained after this operation.
    pub checkpoint_rungs: usize,
    /// Accounted checkpoint-ladder bytes retained after this operation.
    pub checkpoint_bytes: usize,
    /// Checkpoint rung selected as the operation's replay origin.
    pub replay_rung: usize,
    /// Commands executed after the selected replay origin.
    pub replay_commands: u64,
    /// Checkpoint candidates admitted during the operation.
    pub checkpoint_rungs_admitted: usize,
    /// Older checkpoint rungs dropped while admitting candidates.
    pub checkpoint_rungs_dropped: usize,
    /// Checkpoint candidates rejected by the hard byte cap.
    pub checkpoint_rungs_rejected: usize,
    /// A resumable tail remains after a viewport-bounded foreground edit.
    pub tail_pending: bool,
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
    last_sources: Option<SourceSnapshot>,
    pending_continuation: Option<(u64, SourceSnapshot)>,
    opened: bool,
    dead: bool,
    owns_global_lease: bool,
    _not_send: PhantomData<Rc<()>>,
}

impl IncrementalXeTeXSession {
    /// Create an unopened resident session. Format I/O is deferred until
    /// [`open`](Self::open), where it runs under the process-global lock.
    pub fn new(config: EnginePreviewConfig) -> Result<Self> {
        if let Some(pos) = config.format_file_name.find('\0') {
            bail!("nul byte found in provided data at position: {pos}");
        }
        if config.ladder_target_bytes > config.ladder_hard_bytes {
            bail!("checkpoint target exceeds hard cap");
        }
        Ok(Self {
            config,
            state: Box::new(EngineState::default()),
            ladder: None,
            strings_base: None,
            last_sources: None,
            pending_continuation: None,
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
        let format = self.config.format_file_name.clone();
        let input = sources.primary().as_str().to_owned();
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
        let accepted_sources = sources.clone();
        let mut checkpoint_activity = CheckpointActivity::default();
        let result = launcher.with_global_lock(|api| {
            Ok(catch_operation(api, state, |state| {
                configure(state, &config);
                let fence_sources = sources.clone();
                let _sources = crate::replay::adapter::install(sources);
                match crate::engine::initialize_engine(state, &format, build_date) {
                    crate::engine::InitOutcome::Ready => {}
                    crate::engine::InitOutcome::Terminate(history) => {
                        return OperationResult::Terminal(history)
                    }
                }
                state.config.execution_mode = EngineExecutionMode::Resident;
                crate::journal::arm(state);
                let base = StringWatermark::capture(state);
                let mut new_ladder =
                    match CheckpointLadder::from_engine_with_budget(state, base, budget) {
                        Ok(ladder) => ladder,
                        Err(error) => return OperationResult::Budget(error),
                    };
                if verify {
                    new_ladder.enable_verification(state);
                }
                *strings_base = Some(base);
                run_document(
                    state,
                    ladder.insert(new_ladder),
                    DocumentRun {
                        input: &input,
                        snapshot: &fence_sources,
                        splice: None,
                        recovery_rung_clock: None,
                        fences: FencePlan {
                            strings_base: base,
                            budget,
                            pending_fences: Vec::new(),
                            horizon_last: None,
                        },
                    },
                    &mut checkpoint_activity,
                )
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
        self.last_sources = Some(accepted_sources);
        self.finish_operation(result, revision, 0, 0, checkpoint_activity)
    }

    /// Replace the immutable source set and resume from the deepest safe fence.
    /// When a viewport is supplied, return after its last required fresh page
    /// and retain an exact command fence for [`continue_tail`](Self::continue_tail).
    pub fn edit(
        &mut self,
        launcher: &mut CoreBridgeLauncher,
        sources: SourceSnapshot,
        viewport: Option<PageRange>,
    ) -> Result<EngineFrame> {
        if self.dead {
            bail!("resident XeTeX session is dead");
        }
        if !self.opened {
            bail!("resident XeTeX session was edited before open");
        }
        self.pending_continuation = None;
        let input = sources.primary().as_str().to_owned();
        let revision = sources.revision();
        let pending_fences = self
            .last_sources
            .as_ref()
            .map(|previous| edit_fence_clusters(&sources, previous))
            .unwrap_or_default();
        let accepted_sources = sources.clone();
        let config = self.config.clone();
        let budget = self.budget();
        let strings_base = self
            .strings_base
            .expect("opened session without string watermark");
        let state = &mut *self.state;
        let ladder = self.ladder.as_mut().expect("opened session without ladder");
        let selection = ladder.select_replay_fence(&sources);
        let replay_rung = selection.as_ref().map_or(0, |(target, _)| *target);
        let replay_clock = ladder.rung_clock(replay_rung).unwrap_or(0);
        let mut checkpoint_activity = CheckpointActivity::default();
        let result = launcher.with_global_lock(|api| {
            Ok(catch_operation(api, state, |state| {
                let fence_sources = sources.clone();
                let _sources = crate::replay::adapter::install(sources);
                let splice = if let Some((target, fence)) = selection {
                    crate::engine::release_operation_inputs(state);
                    ladder.rewind_engine(state, target);
                    assert!(
                        fence.restore_inputs(state),
                        "eligible replay fence failed input reconstruction"
                    );
                    Some(Splice {
                        at: fence.splice_at(),
                        resume_pending_command: fence.resume_pending_command(),
                    })
                } else {
                    ladder.rewind_engine(state, 0);
                    None
                };
                let first_shipped = state.shipout.total_pages.max(0) as usize;
                let horizon_last = viewport.map(|range| range.last.max(first_shipped));
                ladder.clear_protections();
                configure(state, &config);
                state.shipout.rust_stdout = crate::bridge::output_open_stdout();
                run_document(
                    state,
                    ladder,
                    DocumentRun {
                        input: &input,
                        snapshot: &fence_sources,
                        splice,
                        recovery_rung_clock: Some(replay_clock),
                        fences: FencePlan {
                            strings_base,
                            budget,
                            pending_fences,
                            horizon_last,
                        },
                    },
                    &mut checkpoint_activity,
                )
            }))
        })?;
        if let OperationResult::Horizoned { rung_clock, .. } = &result {
            self.pending_continuation = Some((*rung_clock, accepted_sources.clone()));
        }
        self.last_sources = Some(accepted_sources);
        self.finish_operation(
            result,
            revision,
            replay_rung,
            replay_clock,
            checkpoint_activity,
        )
    }

    /// Resume a viewport-bounded edit from its exact command fence to `\end`.
    pub fn continue_tail(
        &mut self,
        launcher: &mut CoreBridgeLauncher,
    ) -> Result<Option<EngineFrame>> {
        let Some((rung_clock, sources)) = self.pending_continuation.take() else {
            return Ok(None);
        };
        let revision = sources.revision();
        let input = sources.primary().as_str().to_owned();
        let config = self.config.clone();
        let budget = self.budget();
        let strings_base = self
            .strings_base
            .expect("opened session without string watermark");
        let state = &mut *self.state;
        let ladder = self.ladder.as_mut().expect("opened session without ladder");
        let target = ladder
            .rung_at_clock(rung_clock)
            .ok_or_else(|| anyhow!("pending tail fence disappeared"))?;
        let fence = ladder
            .replay_fence(target)
            .ok_or_else(|| anyhow!("pending tail rung is not resumable"))?;
        let splice = Splice {
            at: fence.splice_at(),
            resume_pending_command: fence.resume_pending_command(),
        };
        let fence_sources = sources.clone();
        let replay_clock = ladder.rung_clock(target).unwrap_or(rung_clock);
        let mut checkpoint_activity = CheckpointActivity::default();
        let result = launcher.with_global_lock(|api| {
            Ok(catch_operation(api, state, |state| {
                let _sources = crate::replay::adapter::install(sources);
                crate::engine::release_operation_inputs(state);
                ladder.rewind_engine(state, target);
                if !fence.restore_inputs(state) {
                    ladder.rewind_engine(state, 0);
                    return OperationResult::Aborted;
                }
                // Pin the foreground ladder so the tail's own rungs cannot evict
                // the fences the next edit needs.
                assert!(ladder.protect_rungs_through(target));
                configure(state, &config);
                state.shipout.rust_stdout = crate::bridge::output_open_stdout();
                run_document(
                    state,
                    ladder,
                    DocumentRun {
                        input: &input,
                        snapshot: &fence_sources,
                        splice: Some(splice),
                        recovery_rung_clock: Some(rung_clock),
                        fences: FencePlan {
                            strings_base,
                            budget,
                            pending_fences: Vec::new(),
                            horizon_last: None,
                        },
                    },
                    &mut checkpoint_activity,
                )
            }))
        })?;
        self.finish_operation(result, revision, target, replay_clock, checkpoint_activity)
            .map(Some)
    }

    /// Every control sequence the engine currently has defined.
    ///
    /// Meaningful once a document has been opened: before that the table holds
    /// only the format's own definitions, and after a compile it holds those
    /// plus everything the preamble and the document added. Reading it changes
    /// nothing, so it is safe to call between operations.
    pub fn defined_commands(&self) -> Vec<crate::introspect::DefinedCommand> {
        crate::introspect::defined_commands(&self.state)
    }

    /// Whether a viewport-bounded foreground operation left resumable tail work.
    pub fn has_pending_tail(&self) -> bool {
        self.pending_continuation.is_some()
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
                run_locked(api, state, |state| {
                    crate::engine::release_operation_inputs(state);
                    ladder.rewind_engine(state, 0);
                    crate::engine::cleanup(state);
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
        replay_rung: usize,
        replay_clock: u64,
        checkpoint_activity: CheckpointActivity,
    ) -> Result<EngineFrame> {
        let mut frame = EngineFrame {
            outcome: TexOutcome::Errors,
            source_revision: revision,
            aborted: None,
            cancelled: false,
            pages: Vec::new(),
            sync_index: Default::default(),
            splice_at: None,
            checkpoint_rungs: self.ladder.as_ref().map_or(0, CheckpointLadder::len),
            checkpoint_bytes: self
                .ladder
                .as_ref()
                .map_or(0, CheckpointLadder::accounted_bytes),
            replay_rung,
            replay_commands: self
                .state
                .scanner
                .logical_clock
                .saturating_sub(replay_clock),
            checkpoint_rungs_admitted: checkpoint_activity.admitted,
            checkpoint_rungs_dropped: checkpoint_activity.dropped,
            checkpoint_rungs_rejected: checkpoint_activity.rejected,
            tail_pending: false,
        };
        let (history, splice_at, tail_pending) = match result {
            OperationResult::Complete { history, splice_at } => (history, splice_at, false),
            OperationResult::Horizoned {
                history, splice_at, ..
            } => (history, splice_at, true),
            OperationResult::Aborted => {
                frame.aborted = Some("XeTeX aborted; resident state rewound to rung zero".into());
                return Ok(frame);
            }
            OperationResult::Cancelled => {
                frame.cancelled = true;
                return Ok(frame);
            }
            OperationResult::Budget(error) => {
                return Err(self.kill(anyhow!("invalid checkpoint budget: {error:?}")))
            }
            OperationResult::Terminal(history) => {
                return Err(self.kill(anyhow!(
                    "XeTeX initialization terminated with history {history}"
                )))
            }
            OperationResult::Panicked => {
                return Err(self.kill(anyhow!("resident XeTeX operation panicked")))
            }
        };
        frame.outcome = history_to_outcome(history)?;
        frame.pages = self.state.shipout.dvi.manifest.take_pages();
        frame.sync_index = self
            .state
            .synctex
            .preview_index
            .as_ref()
            .map(crate::sync_index::PreviewSyncIndex::snapshot)
            .unwrap_or_default();
        frame.splice_at = splice_at;
        frame.tail_pending = tail_pending;
        Ok(frame)
    }

    /// Marks the session unusable and returns `error`.
    fn kill(&mut self, error: Error) -> Error {
        self.dead = true;
        self.release_global_lease();
        error
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
    Complete {
        history: i32,
        splice_at: Option<usize>,
    },
    Horizoned {
        history: i32,
        splice_at: Option<usize>,
        rung_clock: u64,
    },
    Aborted,
    Cancelled,
    Budget(BudgetError),
    Terminal(i32),
    Panicked,
}

/// The replay fence an operation resumes from: its splice offset and whether
/// it holds an undispatched command.
#[derive(Clone, Copy)]
struct Splice {
    at: usize,
    resume_pending_command: bool,
}

struct DocumentRun<'a> {
    input: &'a str,
    snapshot: &'a SourceSnapshot,
    splice: Option<Splice>,
    recovery_rung_clock: Option<u64>,
    fences: FencePlan,
}

fn run_document(
    state: &mut EngineState,
    ladder: &mut CheckpointLadder<EngineBundle>,
    run: DocumentRun<'_>,
    checkpoint_activity: &mut CheckpointActivity,
) -> OperationResult {
    let DocumentRun {
        input,
        snapshot,
        splice,
        recovery_rung_clock,
        fences,
    } = run;
    let splice_at = splice.map(|splice| splice.at);
    let mut horizon_rung = None;
    let _fences = install_fence_controller(
        ladder,
        snapshot,
        fences,
        state.scanner.logical_clock,
        &mut horizon_rung,
        checkpoint_activity,
    );
    if splice.is_some() {
        let manifest_start = state.shipout.dvi.pos() as u64;
        state
            .shipout
            .dvi
            .manifest
            .begin_operation_at(manifest_start);
    } else {
        state.shipout.dvi.manifest.begin_operation();
    }
    let execution = match splice {
        Some(splice) => {
            crate::engine::resume_resident_document(state, splice.resume_pending_command)
        }
        None => crate::engine::run_resident_document(state, input),
    };
    match execution {
        Ok(history) => OperationResult::Complete { history, splice_at },
        Err(Abort::Horizoned) => {
            let Some(rung) = horizon_rung else {
                crate::engine::release_operation_inputs(state);
                ladder.rewind_engine(state, 0);
                return OperationResult::Aborted;
            };
            crate::engine::release_operation_inputs(state);
            if crate::io::close_files_and_terminate(state).is_err() {
                ladder.rewind_engine(state, 0);
                return OperationResult::Aborted;
            }
            OperationResult::Horizoned {
                history: state.errors.history,
                splice_at,
                rung_clock: ladder
                    .rung_clock(rung)
                    .expect("admitted horizon rung disappeared"),
            }
        }
        Err(error) => {
            crate::engine::release_operation_inputs(state);
            // Both cancellation and fatal aborts stop at a journal-coherent
            // point, so rewind to the replay origin rather than rung zero.
            let recovery_rung = recovery_rung_clock
                .and_then(|clock| ladder.rung_at_clock(clock))
                .unwrap_or(0);
            ladder.rewind_engine(state, recovery_rung);
            if matches!(error, Abort::Cancelled) {
                OperationResult::Cancelled
            } else {
                OperationResult::Aborted
            }
        }
    }
}

fn edit_fence_clusters(
    snapshot: &SourceSnapshot,
    previous: &SourceSnapshot,
) -> Vec<(SourcePath, Vec<usize>)> {
    snapshot
        .changes_from(previous)
        .into_iter()
        .filter_map(|(path, offset)| {
            let fences = edit_fence_offsets(offset);
            (!fences.is_empty()).then_some((path, fences))
        })
        .collect()
}

/// Sparse exponential cluster immediately before an edit, in source bytes.
fn edit_fence_offsets(offset: usize) -> Vec<usize> {
    if offset == 0 {
        return Vec::new();
    }
    let base = offset.saturating_sub(64) & !63;
    if base == 0 {
        return Vec::new();
    }
    let mut offsets = vec![base];
    let mut position = base;
    let mut step = 50usize;
    while offsets.len() < MAX_EDIT_FENCES {
        position = position.saturating_sub(step);
        if position == 0 {
            break;
        }
        offsets.push(position);
        step = step.saturating_mul(2);
    }
    offsets.sort_unstable();
    offsets.dedup();
    offsets
}

fn configure(state: &mut EngineState, config: &EnginePreviewConfig) {
    state.config.halt_on_error_p = 0;
    state.config.in_initex_mode = false;
    state.config.synctex_enabled = 1;
    state.config.semantic_pagination_enabled = false;
    state.config.shell_escape_enabled = false;
    state.config.deterministic_mode = config.deterministic;
    state.config.native_pgfmath = config.native_pgfmath;
    state.config.execution_mode = EngineExecutionMode::Resident;
    state.cancellation.install(config.cancellation.clone());
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
        tectonic_xetex_layout::reset_font_caches();
        OperationResult::Panicked
    })
}

fn run_locked<R>(
    api: &mut CoreBridgeState<'_>,
    state: &mut EngineState,
    operation: impl FnOnce(&mut EngineState) -> R,
) -> R {
    let _bridge = crate::bridge::enter(api);
    operation(state)
}

#[cfg(test)]
mod tests {
    use super::edit_fence_offsets;

    #[test]
    fn edit_fences_are_dense_near_the_change_and_bounded() {
        assert!(edit_fence_offsets(0).is_empty());
        assert!(edit_fence_offsets(63).is_empty());
        let offsets = edit_fence_offsets(10_000);
        assert!(offsets.len() <= super::MAX_EDIT_FENCES);
        assert!(offsets.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(offsets.last().copied(), Some(9_920));
    }
}
