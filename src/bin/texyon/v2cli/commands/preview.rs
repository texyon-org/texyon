//! `preview`: SVG live preview over JSON Lines stdin/stdout.

use std::{
    io::{BufReader, BufWriter},
    time::SystemTime,
};

use clap::Parser;
use texyon::{
    config::PersistentConfig,
    driver::{OutputFormat, ProcessingSessionBuilder},
    engines::tex::EnginePreviewConfig,
    errors::{ErrorKind, Result},
    io::{format_cache::FormatCache, IoProvider, OpenResult},
    live_preview::{
        serve_preview_jsonl, BundleResourceProvider, FrozenBundle, JsonlFrameSink,
        PreviewBootstrap, PreviewTuning, ResidentEngineSupport, ResidentPreviewProcessor,
        ResidentXeTeX,
    },
    status::{NoopStatusBackend, StatusBackend},
    FORMAT_SERIAL,
};
use texyon_preview_protocol::ProtocolLimits;
use texyon_preview_render::MupdfBackend;

use crate::v2cli::{CommandCustomizations, TexyonCommand};

/// Serve one live-preview process over stdio.
#[derive(Debug, Eq, PartialEq, Parser)]
pub struct PreviewCommand {
    /// XeTeX format name, with or without the `.fmt` suffix.
    #[arg(long, default_value = "latex")]
    format: String,

    /// Use only resource files cached locally.
    #[arg(short = 'C', long)]
    only_cached: bool,

    /// Grace before a newer replacement cancels stale instrumented work.
    #[arg(long, default_value_t = 500)]
    supersede_grace_ms: u64,

    /// Idle delay before resuming a viewport-bounded document tail.
    #[arg(long, default_value_t = 200)]
    tail_idle_ms: u64,

    /// Soft resident checkpoint-ladder budget in MiB.
    #[arg(long, default_value_t = 64)]
    ladder_mib: usize,

    /// Verify named engine rewinds with the debug oracle.
    #[arg(long)]
    verify_rewinds: bool,
}

impl TexyonCommand for PreviewCommand {
    fn customize(&self, customizations: &mut CommandCustomizations) {
        // Protocol stdout must contain JSON Lines and nothing else.
        customizations.always_stderr = true;
        customizations.minimal_chatter = true;
    }

    fn execute(self, config: PersistentConfig, status: &mut dyn StatusBackend) -> Result<i32> {
        let ladder_bytes = self
            .ladder_mib
            .checked_mul(1024 * 1024)
            .ok_or_else(|| ErrorKind::Msg("preview ladder budget overflows usize".into()))?;
        let tuning = PreviewTuning {
            supersede_grace_ms: self.supersede_grace_ms,
            tail_idle_ms: self.tail_idle_ms,
        };
        let limits = ProtocolLimits::default();
        let format_name = if self.format.ends_with(".fmt") {
            self.format
        } else {
            format!("{}.fmt", self.format)
        };
        let verify_rewinds = self.verify_rewinds;
        let only_cached = self.only_cached;
        ensure_format(&config, &format_name, only_cached, status)?;

        let result = serve_preview_jsonl(
            BufReader::new(std::io::stdin()),
            BufWriter::new(std::io::stdout()),
            limits,
            tuning,
            move |writer| -> std::result::Result<_, String> {
                let mut bundle = config
                    .default_bundle(only_cached)
                    .map_err(|error| error.to_string())?;
                let bundle_digest = bundle.get_digest().map_err(|error| error.to_string())?;
                let digest_text = bundle_digest.to_string();
                let mut format_cache = FormatCache::new(
                    bundle_digest,
                    config
                        .format_cache_path()
                        .map_err(|error| error.to_string())?,
                    FORMAT_SERIAL,
                );
                let mut status = NoopStatusBackend::default();
                match format_cache.input_open_format(&format_name, &mut status) {
                    OpenResult::Ok(_) => {}
                    OpenResult::NotAvailable => {
                        return Err(format!(
                            "format `{format_name}` is still not cached after generating it"
                        ));
                    }
                    OpenResult::Err(error) => return Err(error.to_string()),
                }
                let bundle = FrozenBundle::new(bundle);
                let provider = BundleResourceProvider::new(bundle.clone(), None)
                    .map_err(|error| error.to_string())?;
                let support = ResidentEngineSupport::Bundle {
                    bundle,
                    format_cache,
                };
                let mut engine_config = EnginePreviewConfig::new(format_name);
                engine_config.build_date = SystemTime::UNIX_EPOCH;
                engine_config.deterministic = true;
                engine_config.verify_rewinds = verify_rewinds;
                engine_config.ladder_target_bytes = ladder_bytes;
                engine_config.ladder_hard_bytes = ladder_bytes
                    .checked_add(ladder_bytes / 2)
                    .ok_or_else(|| "preview hard ladder budget overflows usize".to_owned())?;
                let engine = ResidentXeTeX::new(support, engine_config);
                let backend = MupdfBackend::new().map_err(|error| error.to_string())?;
                let sink = JsonlFrameSink::new(writer, limits);
                let processor = ResidentPreviewProcessor::new(engine, provider, backend, sink);
                Ok((
                    processor,
                    PreviewBootstrap {
                        capabilities: vec![
                            "svg".into(),
                            "whole_buffer_replace".into(),
                            "child_sources".into(),
                            "viewport".into(),
                            "resync".into(),
                            "painted_telemetry".into(),
                            "synctex".into(),
                            "synctex_boxes".into(),
                        ],
                        bundle_digest: digest_text,
                    },
                ))
            },
        );
        match result {
            Ok(_) => Ok(0),
            Err(error) => Err(ErrorKind::Msg(error.to_string()).into()),
        }
    }
}

/// Generate the format on first use, as a normal compile would.
fn ensure_format(
    config: &PersistentConfig,
    format_name: &str,
    only_cached: bool,
    status: &mut dyn StatusBackend,
) -> Result<()> {
    let mut bundle = config.default_bundle(only_cached)?;
    let mut format_cache = FormatCache::new(
        bundle.get_digest()?,
        config.format_cache_path()?,
        FORMAT_SERIAL,
    );
    if let OpenResult::Ok(_) = format_cache.input_open_format(format_name, status) {
        return Ok(());
    }
    let mut builder = ProcessingSessionBuilder::default();
    builder
        .bundle(bundle)
        .format_name(format_name)
        .format_cache_path(config.format_cache_path()?)
        .primary_input_buffer(b"\\documentclass{article}\\begin{document}x\\end{document}")
        .output_format(OutputFormat::Xdv)
        .tex_input_name("texput.tex")
        .output_dir(std::env::temp_dir())
        .do_not_write_output_files();
    builder.create(status)?.run(status)
}
