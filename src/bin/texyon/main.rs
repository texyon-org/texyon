// Copyright 2016-2026 the Tectonic Project
// Licensed under the MIT License.

//! The `texyon` command-line program.

use clap::{Parser, Subcommand, ValueEnum};
use std::{ffi::OsString, io::IsTerminal, process::ExitCode};
use texyon::{
    config::PersistentConfig,
    errors::{Result, SyncError},
    status::{ChatterLevel, StatusBackend, termcolor::TermcolorStatusBackend},
    unstable_opts,
};
use texyon_status_base::plain::PlainStatusBackend;
use tracing::level_filters::LevelFilter;

use self::commands::bundle::BundleCommand;
#[cfg(all(
    feature = "preview-mupdf",
    any(
        target_os = "linux",
        target_os = "macos",
        all(target_os = "windows", target_env = "msvc")
    )
))]
use self::commands::preview::PreviewCommand;

mod commands;
mod compile;

#[derive(Debug, Parser)]
#[command(name = "texyon", about = "A resident LaTeX engine for live preview")]
struct Cli {
    /// How much chatter to print when running
    #[arg(long = "chatter", short, default_value = "default", global = true)]
    chatter_level: ChatterLevel,

    /// Control colorization of output
    #[arg(long = "color", default_value = "auto", global = true)]
    cli_color: CliColor,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Compile a (La)TeX document
    #[command(name = "compile")]
    Compile(compile::CompileOptions),

    /// Inspect or create TeX support bundles
    #[command(name = "bundle")]
    Bundle(BundleCommand),

    /// Serve a live-preview session over stdio
    #[cfg(all(
        feature = "preview-mupdf",
        any(
            target_os = "linux",
            target_os = "macos",
            all(target_os = "windows", target_env = "msvc")
        )
    ))]
    #[command(name = "preview")]
    Preview(PreviewCommand),
}

#[derive(ValueEnum, Clone, Debug)]
enum CliColor {
    #[value(name = "always")]
    Always,

    #[value(name = "auto")]
    Auto,

    #[value(name = "never")]
    Never,
}

impl CliColor {
    pub fn should_enable(&self) -> bool {
        match self {
            Self::Always => true,
            Self::Auto => std::io::stdout().is_terminal(),
            Self::Never => false,
        }
    }
}

/// Lets a command adjust the shared setup before it runs.
#[derive(Debug, Default)]
struct CommandCustomizations {
    always_stderr: bool,
    minimal_chatter: bool,
}

trait TexyonCommand {
    fn customize(&self, cc: &mut CommandCustomizations);
    fn execute(self, config: PersistentConfig, status: &mut dyn StatusBackend) -> Result<i32>;
}

#[derive(Parser)]
struct PeekUnstableOptions {
    #[arg(name = "option", short = 'Z')]
    unstable: Vec<unstable_opts::UnstableArg>,

    #[arg()]
    _remainder: Vec<OsString>,
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();

    // `texyon -Z help` prints its usage before a missing command could fail parsing.
    if let Ok(peek) = PeekUnstableOptions::try_parse_from(&args) {
        unstable_opts::UnstableOptions::from_unstable_args(peek.unstable.into_iter());
    }

    let cli = Cli::parse_from(&args);

    texyon::test_util::maybe_activate_test_mode();

    let config = match PersistentConfig::open(false) {
        Ok(c) => c,
        Err(ref e) => {
            e.dump_uncolorized();
            return ExitCode::FAILURE;
        }
    };

    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::INFO)
        .with_target(false)
        .without_time()
        .with_ansi(cli.cli_color.should_enable())
        .init();

    let mut customizations = CommandCustomizations::default();
    match &cli.command {
        Commands::Compile(_) => {}
        Commands::Bundle(o) => o.customize(&mut customizations),
        #[cfg(all(
            feature = "preview-mupdf",
            any(
                target_os = "linux",
                target_os = "macos",
                all(target_os = "windows", target_env = "msvc")
            )
        ))]
        Commands::Preview(o) => o.customize(&mut customizations),
    }

    let chatter_level = if customizations.minimal_chatter {
        ChatterLevel::Minimal
    } else {
        cli.chatter_level
    };
    let mut status = if cli.cli_color.should_enable() {
        let mut sb = TermcolorStatusBackend::new(chatter_level);
        sb.always_stderr(customizations.always_stderr);
        Box::new(sb) as Box<dyn StatusBackend>
    } else {
        let mut sb = PlainStatusBackend::new(chatter_level);
        sb.always_stderr(customizations.always_stderr);
        Box::new(sb) as Box<dyn StatusBackend>
    };

    let result = match cli.command {
        Commands::Compile(o) => o.execute(config, &mut *status),
        Commands::Bundle(o) => o.execute(config, &mut *status),
        #[cfg(all(
            feature = "preview-mupdf",
            any(
                target_os = "linux",
                target_os = "macos",
                all(target_os = "windows", target_env = "msvc")
            )
        ))]
        Commands::Preview(o) => o.execute(config, &mut *status),
    };

    // Returning lets atexit run, where an instrumented Windows build writes its profile.
    match result {
        Ok(code) => u8::try_from(code).map_or(ExitCode::FAILURE, ExitCode::from),
        Err(e) => {
            status.report_error(&SyncError::new(e).into());
            ExitCode::FAILURE
        }
    }
}
