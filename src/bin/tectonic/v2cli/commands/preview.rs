//! `preview`: cache-only SVG live preview over JSON Lines stdin/stdout.

use std::{
    io::{BufReader, BufWriter},
    time::SystemTime,
};

use clap::Parser;
use tectonic::{
    config::PersistentConfig,
    engines::tex::EnginePreviewConfig,
    errors::{ErrorKind, Result},
    io::{format_cache::FormatCache, IoProvider, OpenResult},
    live_preview::{
        serve_preview_jsonl, BundleResourceProvider, FrozenBundle, JsonlFrameSink,
        PreviewBootstrap, PreviewTuning, ResidentEngineSupport, ResidentPreviewProcessor,
        ResidentXeTeX,
    },
    status::{NoopStatusBackend, StatusBackend},
};
use tectonic_preview_protocol::ProtocolLimits;
use tectonic_preview_render::MupdfBackend;

use crate::v2cli::{CommandCustomizations, TectonicCommand};

/// Serve one cache-only live-preview process over stdio.
#[derive(Debug, Eq, PartialEq, Parser)]
pub struct PreviewCommand {
    /// XeTeX format name, with or without the `.fmt` suffix.
    #[arg(long, default_value = "latex")]
    format: String,

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

impl TectonicCommand for PreviewCommand {
    fn customize(&self, customizations: &mut CommandCustomizations) {
        // Protocol stdout must contain JSON Lines and nothing else.
        customizations.always_stderr = true;
        customizations.minimal_chatter = true;
    }

    fn execute(self, config: PersistentConfig, _status: &mut dyn StatusBackend) -> Result<i32> {
        let ladder_bytes = self
            .ladder_mib
            .checked_mul(1024 * 1024)
            .ok_or_else(|| ErrorKind::Msg("preview ladder budget overflows usize".into()))?;
        let tuning = PreviewTuning {
            supersede_grace_ms: self.supersede_grace_ms,
            tail_idle_ms: self.tail_idle_ms,
            ladder_bytes,
            verify_rewinds: self.verify_rewinds,
        };
        let limits = ProtocolLimits::default();
        let format_name = if self.format.ends_with(".fmt") {
            self.format
        } else {
            format!("{}.fmt", self.format)
        };
        let verify_rewinds = self.verify_rewinds;

        let result = serve_preview_jsonl(
            BufReader::new(std::io::stdin()),
            BufWriter::new(std::io::stdout()),
            limits,
            tuning,
            move |writer| -> std::result::Result<_, String> {
                let mut bundle = config
                    .default_bundle(true)
                    .map_err(|error| error.to_string())?;
                let bundle_digest = bundle.get_digest().map_err(|error| error.to_string())?;
                let digest_text = bundle_digest.to_string();
                let mut format_cache = FormatCache::new(
                    bundle_digest,
                    config
                        .format_cache_path()
                        .map_err(|error| error.to_string())?,
                );
                let mut status = NoopStatusBackend::default();
                match format_cache.input_open_format(&format_name, &mut status) {
                    OpenResult::Ok(_) => {}
                    OpenResult::NotAvailable => {
                        return Err(format!(
                            "format `{format_name}` is not cached; run a normal compile before starting preview"
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
