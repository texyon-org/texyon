//! Rust-owned engine state.

use crate::build::hyphenation::HyphState;
use crate::build::main_control::InterCharState;
use crate::fonts::FontState;
use crate::fonts::ext::SourceSpecials;
use crate::memory::mem::MemState;
use crate::print::errors::ErrorState;
use crate::scanning::input::InputClusterState;
use crate::scanning::scan::ScannerState;
use crate::shipout::ShipoutState;
use crate::shipout::synctex::SynctexCtxt;
use crate::tables::eqtb::EqtbState;
use crate::tables::save::SaveStackState;
use crate::tables::string_pool::StringsState;

#[derive(Default)]
pub(crate) struct EngineState {
    pub config: ConfigState,
    pub cancellation: crate::preview::cancellation::CancellationState,
    pub errors: ErrorState,
    pub random: crate::base::random::Random,
    pub shipout: ShipoutState,
    pub scanner: ScannerState,
    pub save_stack: SaveStackState,
    pub eqtb: EqtbState,
    pub mem: MemState,
    pub input: InputClusterState,
    pub strings: StringsState,
    pub hyph: HyphState,
    pub fonts: FontState,
    pub src_specials: SourceSpecials,
    pub synctex: SynctexCtxt,
    pub interchar: InterCharState,
    /// Not checkpointed.
    pub pgfmath: crate::scanning::pgfmath::Diagnostics,
    /// Not checkpointed: a restore keeps the operation's list.
    pub diagnostics: crate::print::diagnostic::DiagnosticsState,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EngineExecutionMode {
    #[default]
    Batch,
    Resident,
}

#[derive(Default, Clone, Copy)]
pub(crate) struct ConfigState {
    pub file_line_error_style_p: i32,
    pub halt_on_error_p: i32,
    pub in_initex_mode: bool,
    pub synctex_enabled: i32,
    pub semantic_pagination_enabled: bool,
    pub shell_escape_enabled: bool,
    /// Remove live clock and timezone inputs from engine-visible values.
    pub deterministic_mode: bool,
    /// Selects operation-lifetime behavior that differs for a resident engine,
    /// such as guarded picture probes.
    pub execution_mode: EngineExecutionMode,
    pub max_print_line: i32,
    pub pgfmath: crate::scanning::pgfmath::Mode,
}
