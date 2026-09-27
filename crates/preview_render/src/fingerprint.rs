//! Versioned canonical serialization for renderer-owned page identity.

use sha2::{Digest, Sha256};

use crate::render::{DrawOperation, PagePlan};
use crate::{Color, LinkAction, Matrix, Path, PathCommand};

pub(crate) fn page_digest(page: &PagePlan) -> [u8; 32] {
    let mut digest = Sha256::new();
    // Bumped from v4 when links and destinations joined a page's identity.
    digest.update(b"tectonic-page-visual\0v5");
    digest.update(page.width_bp.to_bits().to_be_bytes());
    digest.update(page.height_bp.to_bits().to_be_bytes());
    for operation in &page.operations {
        match operation {
            DrawOperation::Rule {
                height,
                width,
                transform,
                color,
            } => {
                digest.update([0]);
                for value in [height, width] {
                    digest.update(value.to_be_bytes());
                }
                hash_matrix(&mut digest, *transform);
                hash_color(&mut digest, *color);
            }
            DrawOperation::NativeGlyphs {
                identity,
                face_index,
                size,
                glyphs,
                color,
                extend,
                slant,
                embolden,
                vertical,
            } => {
                digest.update([1]);
                digest.update(identity.0);
                digest.update(face_index.to_be_bytes());
                digest.update(size.to_be_bytes());
                digest.update((glyphs.len() as u64).to_be_bytes());
                for glyph in glyphs {
                    digest.update(glyph.glyph_id.to_be_bytes());
                    hash_matrix(&mut digest, glyph.transform);
                }
                hash_color(&mut digest, *color);
                digest.update(extend.to_be_bytes());
                digest.update(slant.to_be_bytes());
                digest.update(embolden.to_be_bytes());
                digest.update([u8::from(*vertical)]);
            }
            DrawOperation::Image {
                identity,
                kind,
                transform,
                page,
                width,
                height,
            } => {
                digest.update([2]);
                digest.update(identity.0);
                digest.update([*kind as u8]);
                hash_matrix(&mut digest, *transform);
                digest.update(page.to_be_bytes());
                digest.update(width.map(f32::to_bits).unwrap_or(u32::MAX).to_be_bytes());
                digest.update(height.map(f32::to_bits).unwrap_or(u32::MAX).to_be_bytes());
            }
            DrawOperation::ClassicChars {
                tfm_identity,
                program_identity,
                encoding_identity,
                scale,
                chars,
                color,
            } => {
                digest.update([3]);
                digest.update(tfm_identity.0);
                digest.update(program_identity.0);
                digest.update(encoding_identity.0);
                digest.update(scale.to_be_bytes());
                for character in chars {
                    digest.update(character.character.to_be_bytes());
                    hash_matrix(&mut digest, character.transform);
                }
                hash_color(&mut digest, *color);
            }
            DrawOperation::PathFill {
                path,
                even_odd,
                transform,
                color,
            } => {
                digest.update([4]);
                hash_path(&mut digest, path);
                digest.update([u8::from(*even_odd)]);
                hash_matrix(&mut digest, *transform);
                hash_color(&mut digest, *color);
            }
            DrawOperation::PathStroke {
                path,
                style,
                transform,
                color,
            } => {
                digest.update([5]);
                hash_path(&mut digest, path);
                digest.update(style.width.to_bits().to_be_bytes());
                digest.update(style.join.to_be_bytes());
                digest.update(style.cap.to_be_bytes());
                digest.update(style.miter_limit.to_bits().to_be_bytes());
                digest.update((style.dash_len as u64).to_be_bytes());
                for value in &style.dash[..style.dash_len] {
                    digest.update(value.to_bits().to_be_bytes());
                }
                digest.update(style.dash_phase.to_bits().to_be_bytes());
                hash_matrix(&mut digest, *transform);
                hash_color(&mut digest, *color);
            }
            DrawOperation::ClipPath {
                path,
                even_odd,
                transform,
            } => {
                digest.update([6]);
                hash_path(&mut digest, path);
                digest.update([u8::from(*even_odd)]);
                hash_matrix(&mut digest, *transform);
            }
            DrawOperation::PopClip => digest.update([7]),
        }
    }
    hash_overlay(&mut digest, page);
    digest.finalize().into()
}

fn hash_path(digest: &mut Sha256, path: &Path) {
    digest.update((path.commands.len() as u64).to_be_bytes());
    for command in &path.commands {
        match command {
            PathCommand::MoveTo(x, y) => {
                digest.update([0]);
                hash_floats(digest, &[*x, *y]);
            }
            PathCommand::LineTo(x, y) => {
                digest.update([1]);
                hash_floats(digest, &[*x, *y]);
            }
            PathCommand::CurveTo(x1, y1, x2, y2, x3, y3) => {
                digest.update([2]);
                hash_floats(digest, &[*x1, *y1, *x2, *y2, *x3, *y3]);
            }
            PathCommand::Rect(x0, y0, x1, y1) => {
                digest.update([3]);
                hash_floats(digest, &[*x0, *y0, *x1, *y1]);
            }
            PathCommand::Close => digest.update([4]),
        }
    }
}

/// Fold a page's overlay into its identity.
///
/// Links and destinations are shipped with the pages a frame says changed, so a
/// page whose drawing is identical but whose links now point elsewhere has to
/// count as changed — otherwise the viewer keeps an overlay that silently goes
/// to the wrong place.
fn hash_overlay(digest: &mut Sha256, page: &PagePlan) {
    digest.update(b"links");
    digest.update((page.links.len() as u64).to_be_bytes());
    for link in &page.links {
        hash_floats(
            digest,
            &[
                link.rect.left,
                link.rect.top,
                link.rect.right,
                link.rect.bottom,
            ],
        );
        let (tag, target) = match &link.action {
            LinkAction::Uri(uri) => (0, uri),
            LinkAction::Destination(name) => (1, name),
        };
        digest.update([tag]);
        hash_str(digest, target);
    }

    digest.update(b"destinations");
    digest.update((page.destinations.len() as u64).to_be_bytes());
    for destination in &page.destinations {
        hash_str(digest, &destination.name);
        hash_floats(digest, &[destination.x_bp, destination.y_bp]);
    }
}

fn hash_str(digest: &mut Sha256, text: &str) {
    digest.update((text.len() as u64).to_be_bytes());
    digest.update(text.as_bytes());
}

fn hash_matrix(digest: &mut Sha256, matrix: Matrix) {
    hash_floats(
        digest,
        &[matrix.a, matrix.b, matrix.c, matrix.d, matrix.e, matrix.f],
    );
}

fn hash_color(digest: &mut Sha256, color: Color) {
    hash_floats(digest, &[color.red, color.green, color.blue, color.alpha]);
}

fn hash_floats(digest: &mut Sha256, values: &[f32]) {
    for value in values {
        digest.update(value.to_bits().to_be_bytes());
    }
}
