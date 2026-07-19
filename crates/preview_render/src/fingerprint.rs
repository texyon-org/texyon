//! Versioned canonical serialization for renderer-owned page identity.

use sha2::{Digest, Sha256};

use crate::render::{DrawOperation, PagePlan};

pub(crate) fn page_digest(page: &PagePlan) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(b"tectonic-page-visual\0v1");
    digest.update(page.width_bp.to_bits().to_be_bytes());
    digest.update(page.height_bp.to_bits().to_be_bytes());
    for operation in &page.operations {
        match operation {
            DrawOperation::Rule {
                x,
                y,
                height,
                width,
                color,
            } => {
                digest.update([0]);
                for value in [x, y, height, width] {
                    digest.update(value.to_be_bytes());
                }
                for value in [color.red, color.green, color.blue] {
                    digest.update(value.to_bits().to_be_bytes());
                }
            }
            DrawOperation::NativeGlyphs {
                identity,
                face_index,
                size,
                glyphs,
                color,
            } => {
                digest.update([1]);
                digest.update(identity.0);
                digest.update(face_index.to_be_bytes());
                digest.update(size.to_be_bytes());
                digest.update((glyphs.len() as u64).to_be_bytes());
                for glyph in glyphs {
                    digest.update(glyph.glyph_id.to_be_bytes());
                    digest.update(glyph.x.to_be_bytes());
                    digest.update(glyph.y.to_be_bytes());
                }
                for value in [color.red, color.green, color.blue] {
                    digest.update(value.to_bits().to_be_bytes());
                }
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
                for value in [
                    transform.a,
                    transform.b,
                    transform.c,
                    transform.d,
                    transform.e,
                    transform.f,
                ] {
                    digest.update(value.to_bits().to_be_bytes());
                }
                digest.update(page.to_be_bytes());
                digest.update(width.map(f32::to_bits).unwrap_or(u32::MAX).to_be_bytes());
                digest.update(height.map(f32::to_bits).unwrap_or(u32::MAX).to_be_bytes());
            }
        }
    }
    digest.finalize().into()
}
