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
            } => {
                digest.update([0]);
                for value in [x, y, height, width] {
                    digest.update(value.to_be_bytes());
                }
            }
            DrawOperation::NativeGlyphs {
                identity,
                face_index,
                size,
                glyphs,
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
            }
        }
    }
    digest.finalize().into()
}
