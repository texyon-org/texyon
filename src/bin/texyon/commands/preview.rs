//! `preview`: a live-preview session over length-prefixed MessagePack frames
//! on stdin and stdout.

use std::{
    io::{BufReader, BufWriter},
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime},
};

use clap::Parser;
use texyon::{
    config::PersistentConfig,
    driver::{OutputFormat, ProcessingSessionBuilder},
    engines::tex::EnginePreviewConfig,
    errors::{ErrorKind, Result},
    io::{format_cache::FormatCache, IoProvider, OpenResult},
    live_preview::{
        serve, BundleOpener, BundleResourceProvider, DiskSources, DocumentRoot, FrozenBundle,
        OpenRequest, PackedBundle, PreviewSession, ResidentEngineSupport, SessionConfig,
        SupportPack,
    },
    status::{NoopStatusBackend, StatusBackend},
    FORMAT_SERIAL,
};
use texyon_bundles::Bundle;
use texyon_errors::anyhow::{anyhow, bail};
use texyon_preview_protocol::{session::SESSION_PROTOCOL_VERSION, ProtocolLimits};
use texyon_preview_render::MupdfBackend;

use crate::{CommandCustomizations, TexyonCommand};

/// Serve one live-preview session over stdio.
#[derive(Debug, Eq, PartialEq, Parser)]
pub struct PreviewCommand {
    /// The session protocol version the client speaks.
    #[arg(long)]
    protocol: u32,

    /// Use only resource files cached locally.
    #[arg(short = 'C', long)]
    only_cached: bool,

    /// Soft resident checkpoint-ladder budget in MiB.
    #[arg(long, default_value_t = 64)]
    ladder_mib: usize,

    /// Verify named engine rewinds with the debug oracle.
    #[arg(long)]
    verify_rewinds: bool,

    /// Prebuilt support pack to read before the bundle.
    #[arg(long, value_name = "DIR")]
    pack: Option<PathBuf>,
}

impl TexyonCommand for PreviewCommand {
    fn customize(&self, customizations: &mut CommandCustomizations) {
        // Protocol stdout must contain frames and nothing else.
        customizations.always_stderr = true;
        customizations.minimal_chatter = true;
    }

    fn execute(self, config: PersistentConfig, _status: &mut dyn StatusBackend) -> Result<i32> {
        if self.protocol != SESSION_PROTOCOL_VERSION {
            return Err(ErrorKind::Msg(format!(
                "the client speaks preview protocol {}, this build speaks {SESSION_PROTOCOL_VERSION}",
                self.protocol
            ))
            .into());
        }
        let ladder_bytes = self
            .ladder_mib
            .checked_mul(1024 * 1024)
            .ok_or_else(|| ErrorKind::Msg("preview ladder budget overflows usize".into()))?;
        let tuning = Tuning {
            ladder_bytes,
            only_cached: self.only_cached,
            verify_rewinds: self.verify_rewinds,
            pack: self.pack,
        };

        serve(
            BufReader::new(std::io::stdin()),
            BufWriter::new(std::io::stdout()),
            move |request| open(&config, &tuning, request),
        )
        .map_err(|error| ErrorKind::Msg(error.to_string()))?;
        Ok(0)
    }
}

struct Tuning {
    ladder_bytes: usize,
    only_cached: bool,
    verify_rewinds: bool,
    pack: Option<PathBuf>,
}

/// Where support files come from, before the format is known to be cached.
enum Support {
    Pack(SupportPack),
    Bundle(Box<dyn Bundle>),
}

type NativeSession = PreviewSession<BundleResourceProvider, MupdfBackend, DiskSources>;

fn open(
    config: &PersistentConfig,
    tuning: &Tuning,
    request: OpenRequest,
) -> texyon_errors::Result<NativeSession> {
    let format_name = if request.format.ends_with(".fmt") {
        request.format
    } else {
        format!("{}.fmt", request.format)
    };
    let mut status = NoopStatusBackend::default();
    let mut support = match &tuning.pack {
        Some(dir) => Support::Pack(SupportPack::open(dir).map_err(|error| anyhow!("{error}"))?),
        None => Support::Bundle(
            config
                .default_bundle(tuning.only_cached)
                .map_err(|error| anyhow!("{error}"))?,
        ),
    };
    let digest = match &mut support {
        Support::Pack(pack) => pack.bundle_digest(),
        Support::Bundle(bundle) => bundle.get_digest().map_err(|error| anyhow!("{error}"))?,
    };
    let mut format_cache = FormatCache::new(
        digest,
        config
            .format_cache_path()
            .map_err(|error| anyhow!("{error}"))?,
        FORMAT_SERIAL,
    );
    if let Support::Pack(pack) = &support {
        pack.seed_format(&mut format_cache, &format_name, FORMAT_SERIAL)
            .map_err(|error| anyhow!("{error}"))?;
    }
    ensure_format(
        config,
        &mut format_cache,
        &format_name,
        tuning.only_cached,
        &mut status,
    )
    .map_err(|error| anyhow!("could not prepare the format: {error}"))?;
    if let OpenResult::NotAvailable = format_cache.input_open_format(&format_name, &mut status) {
        bail!("format `{format_name}` is still not cached after generating it");
    }

    // Built only now, since its index download would race a format generation.
    let bundle: Box<dyn Bundle> = match support {
        Support::Pack(pack) => {
            let config = config.clone();
            let only_cached = tuning.only_cached;
            let open: BundleOpener = Arc::new(move || {
                config
                    .default_bundle(only_cached)
                    .map_err(|error| anyhow!("{error}"))
            });
            Box::new(PackedBundle::new(pack, open))
        }
        Support::Bundle(bundle) => bundle,
    };

    let root = DocumentRoot::new(&request.root)
        .map_err(|error| anyhow!("invalid project directory: {error}"))?;
    let bundle = FrozenBundle::new(bundle);
    let provider = BundleResourceProvider::new(bundle.clone(), Some(root.path().to_owned()))
        .map_err(|error| anyhow!("{error}"))?;
    let support = ResidentEngineSupport::Bundle {
        bundle,
        format_cache,
    };

    let mut engine = EnginePreviewConfig::new(format_name);
    engine.build_date = SystemTime::UNIX_EPOCH
        + request
            .build_date
            .map_or(Duration::ZERO, Duration::from_secs);
    engine.deterministic = true;
    engine.verify_rewinds = tuning.verify_rewinds;
    engine.ladder_target_bytes = tuning.ladder_bytes;
    engine.ladder_hard_bytes = tuning
        .ladder_bytes
        .checked_add(tuning.ladder_bytes / 2)
        .ok_or_else(|| anyhow!("preview hard ladder budget overflows usize"))?;

    let backend = MupdfBackend::new().map_err(|error| anyhow!("{error}"))?;
    Ok(PreviewSession::new(
        SessionConfig {
            engine,
            root: root.clone(),
            primary: request.primary,
            raster: request.raster,
            patches: request.patches,
            limits: ProtocolLimits::default(),
        },
        support,
        provider,
        backend,
        DiskSources::new(root),
    ))
}

/// Generate the format on first use, as a normal compile would.
fn ensure_format(
    config: &PersistentConfig,
    format_cache: &mut FormatCache,
    format_name: &str,
    only_cached: bool,
    status: &mut dyn StatusBackend,
) -> Result<()> {
    if let OpenResult::Ok(_) = format_cache.input_open_format(format_name, status) {
        return Ok(());
    }
    let mut builder = ProcessingSessionBuilder::default();
    builder
        .bundle(config.default_bundle(only_cached)?)
        .format_name(format_name)
        .format_cache_path(config.format_cache_path()?)
        .primary_input_buffer(b"\\documentclass{article}\\begin{document}x\\end{document}")
        .output_format(OutputFormat::Xdv)
        .tex_input_name("texput.tex")
        .output_dir(std::env::temp_dir())
        .do_not_write_output_files();
    builder.create(status)?.run(status)
}
