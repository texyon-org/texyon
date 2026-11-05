//! Answer protocol requests on an open session, the same way for every host.

use std::time::{Duration, SystemTime};

use texyon_engine_xetex::introspect::{self, DefinedCommand};
use texyon_preview_protocol::session::{
    Command, CommandKind, CompileReport, Destination, Link, PageOverlay, RasterPage, RenderedPage,
    Response, SessionRequest,
};
use texyon_preview_render::{Backend, LinkAction, ResourceProvider};

use crate::{session, CompileOutput, PreviewSession, SessionHost};

/// How a host carries bulk bytes in its messages.
pub trait Payload: Sized {
    /// Wrap bytes the session produced.
    fn hand_over(bytes: Vec<u8>) -> Self;
}

/// Answer one request on an open session.
pub fn handle<P, Bk, H, B>(
    session: &mut PreviewSession<P, Bk, H>,
    request: SessionRequest,
) -> Response<B>
where
    P: ResourceProvider + 'static,
    Bk: Backend + 'static,
    H: SessionHost,
    B: Payload,
{
    match request {
        SessionRequest::Compile {
            files,
            edits,
            texts,
            viewport,
            scale,
        } => match session.compile(files, edits, texts, viewport, scale) {
            Ok(output) => Response::Compiled(Box::new(report(session.host(), output))),
            Err(error) => Response::error(error),
        },
        SessionRequest::ContinueTail => match session.continue_tail() {
            Ok(Some(output)) => Response::Compiled(Box::new(report(session.host(), output))),
            Ok(None) => Response::Idle,
            Err(error) => Response::error(error),
        },
        SessionRequest::Raster { pages, scale } => match session.rasterize(&pages, scale) {
            Ok(pages) => Response::Rastered {
                pages: pages.into_iter().map(raster).collect(),
            },
            Err(error) => Response::error(error),
        },
        SessionRequest::FullPass => match session.full_pass() {
            Ok((output, rerun)) => Response::Passed {
                report: Box::new(report(session.host(), output)),
                rerun,
            },
            Err(error) => Response::error(error),
        },
        SessionRequest::Export { now } => {
            match session.export_pdf(now.map_or_else(SystemTime::now, |seconds| {
                SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)
            })) {
                Ok(pdf) => Response::Exported(B::hand_over(pdf)),
                Err(error) => Response::error(error),
            }
        }
        SessionRequest::ForwardSearch { path, line } => {
            let found = session.forward_search(&path, line);
            Response::Located {
                rect: found.result.map(Into::into),
                boxes: found.boxes.into_iter().map(Into::into).collect(),
                source: None,
            }
        }
        SessionRequest::BackwardSearch {
            page_index,
            h_bp,
            v_bp,
        } => Response::Located {
            rect: None,
            boxes: Vec::new(),
            source: session
                .backward_search(page_index, h_bp, v_bp)
                .map(Into::into),
        },
        SessionRequest::Commands => Response::Commands {
            commands: session
                .defined_commands()
                .into_iter()
                .map(command)
                .collect(),
        },
        SessionRequest::Cancel => {
            session.cancel();
            Response::Ok
        }
    }
}

fn raster<B: Payload>(page: session::RasterPage) -> RasterPage<B> {
    RasterPage {
        index: page.index,
        region: page.region,
        pixels: B::hand_over(page.pixels),
    }
}

fn report<H: SessionHost, B: Payload>(host: &H, output: CompileOutput) -> CompileReport<B> {
    let (support_files, support_bytes) = host.support_stats();
    let metrics = output.metrics;

    CompileReport {
        published: output.published,
        pages: output
            .pages
            .into_iter()
            .map(|page| RenderedPage {
                index: page.index,
                svg: B::hand_over(page.svg.into_bytes()),
            })
            .collect(),
        rasters: output.rasters.into_iter().map(raster).collect(),
        changed: output.changed,
        removed: output.removed,
        page_sizes: output.page_sizes,
        page_overlays: output.page_overlays.into_iter().map(overlay).collect(),
        page_count: output.page_count,
        log: String::from_utf8_lossy(&output.log).into_owned(),
        outcome: output.outcome,
        aborted: output.aborted,
        tail_pending: output.tail_pending,
        messages: output.messages,
        diagnostics: output.diagnostics,
        checkpoint_rungs: metrics.checkpoint_rungs,
        checkpoint_bytes: metrics.checkpoint_bytes,
        reused_prefix: metrics.reused_prefix,
        replay_rung: metrics.replay_rung,
        replay_commands: metrics.replay_commands.get(),
        tex_ms: metrics.engine_ms,
        analyze_ms: metrics.analysis_ms,
        render_ms: metrics.render_ms,
        shipouts: metrics.shipouts,
        pages_fingerprinted: metrics.pages_fingerprinted,
        pages_rendered: metrics.pages_rendered,
        unsupported_pages: metrics.unsupported_pages,
        support_files,
        support_bytes,
    }
}

fn overlay(overlay: session::PageOverlay) -> PageOverlay {
    PageOverlay {
        links: overlay.links.into_iter().map(link).collect(),
        destinations: overlay
            .destinations
            .into_iter()
            .map(|destination| Destination {
                name: destination.name,
                x_bp: destination.x_bp,
                y_bp: destination.y_bp,
            })
            .collect(),
    }
}

fn link(link: texyon_preview_render::Link) -> Link {
    let (uri, destination) = match link.action {
        LinkAction::Uri(uri) => (Some(uri), None),
        LinkAction::Destination(name) => (None, Some(name)),
    };
    Link {
        left: link.rect.left,
        top: link.rect.top,
        right: link.rect.right,
        bottom: link.rect.bottom,
        uri,
        destination,
    }
}

fn command(found: DefinedCommand) -> Command {
    Command {
        name: found.name,
        arity: found.arity,
        optional: found.optional,
        kind: match found.kind {
            introspect::CommandKind::Macro => CommandKind::Macro,
            introspect::CommandKind::ProtectedMacro => CommandKind::Protected,
            introspect::CommandKind::Builtin => CommandKind::Builtin,
        },
    }
}
