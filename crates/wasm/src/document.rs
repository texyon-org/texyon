//! The XDV document a resident session is building up.
//!
//! An incremental edit does not re-emit the whole document. The engine rewinds
//! to a checkpoint, and everything it writes from there on replaces the tail of
//! the previous output starting at `splice_at` — the byte offset it had already
//! flushed when that checkpoint was taken. Taking the engine's output as the
//! whole document would silently drop every page above the edit.
//!
//! This keeps the last coherent document and performs that splice. It also
//! records where each page begins, which is what lets the renderer resume its
//! analysis inside the unchanged prefix instead of walking from byte zero.

use std::sync::Arc;

/// XDV opcodes we need to find page boundaries.
mod op {
    /// `bop`, the start of a page.
    pub const BEGIN_PAGE: u8 = 139;
    /// `post`, the start of the postamble.
    pub const POSTAMBLE: u8 = 248;
}

/// A coherent XDV document, plus where its pages start.
#[derive(Default)]
pub struct Document {
    bytes: Arc<[u8]>,
    page_offsets: Vec<u32>,
}

impl Document {
    /// The complete document bytes.
    pub fn bytes(&self) -> Arc<[u8]> {
        Arc::clone(&self.bytes)
    }

    /// Absolute begin-page offsets, in page order.
    pub fn page_offsets(&self) -> &[u32] {
        &self.page_offsets
    }

    /// Whether anything has been assembled yet.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Replace the document, or its tail.
    ///
    /// `splice_at` is the engine's `splice_at`: `None` means `written` is the
    /// whole document, `Some(n)` means it replaces everything from byte `n`.
    ///
    /// Returns an error rather than a corrupt document if the retained prefix
    /// cannot be honoured — a shorter previous document than the splice point
    /// would mean the two disagree about what was already emitted.
    pub fn apply(&mut self, written: &[u8], splice_at: Option<usize>) -> Result<(), String> {
        let bytes: Arc<[u8]> = match splice_at {
            None | Some(0) => Arc::from(written),
            Some(prefix) => {
                if prefix > self.bytes.len() {
                    return Err(format!(
                        "the engine kept {prefix} bytes of a document that is only {} long",
                        self.bytes.len()
                    ));
                }

                let mut combined = Vec::with_capacity(prefix + written.len());
                combined.extend_from_slice(&self.bytes[..prefix]);
                combined.extend_from_slice(written);
                Arc::from(combined)
            }
        };

        self.page_offsets = page_offsets(&bytes);
        self.bytes = bytes;
        Ok(())
    }
}

/// Scan an XDV stream for its begin-page offsets.
///
/// Walks the backwards `bop` chain from the postamble, which is how the format
/// is meant to be read and costs one hop per page rather than a full parse.
/// Anything unexpected yields no offsets, which only costs the renderer its
/// resume optimization.
fn page_offsets(bytes: &[u8]) -> Vec<u32> {
    let Some(post) = find_postamble(bytes) else {
        return Vec::new();
    };

    // post: op(1) p(4) num(4) den(4) mag(4) l(4) u(4) s(2) t(2)
    let Some(last_page) = read_u32(bytes, post + 1) else {
        return Vec::new();
    };

    let mut offsets = Vec::new();
    let mut cursor = last_page;

    // 0xffffffff terminates the chain.
    while cursor != u32::MAX {
        let at = cursor as usize;
        if at >= bytes.len() || bytes[at] != op::BEGIN_PAGE {
            return Vec::new();
        }

        offsets.push(cursor);

        // bop: op(1) counts(40) prev(4)
        let Some(previous) = read_u32(bytes, at + 41) else {
            return Vec::new();
        };

        // A chain that does not strictly decrease is malformed, and following
        // it would not terminate.
        if previous != u32::MAX && previous >= cursor {
            return Vec::new();
        }

        cursor = previous;
    }

    offsets.reverse();
    offsets
}

/// Find the postamble by walking back over the trailing 223 padding.
fn find_postamble(bytes: &[u8]) -> Option<usize> {
    // ... post_post(1) q(4) i(1) 223*n
    let mut at = bytes.len().checked_sub(1)?;
    while bytes.get(at) == Some(&223) {
        at = at.checked_sub(1)?;
    }

    // Now at the identification byte; before it is the postamble pointer.
    let pointer = read_u32(bytes, at.checked_sub(4)?)? as usize;

    (bytes.get(pointer) == Some(&op::POSTAMBLE)).then_some(pointer)
}

/// Read a big-endian `u32`, if the stream is long enough.
fn read_u32(bytes: &[u8], at: usize) -> Option<u32> {
    bytes
        .get(at..at + 4)
        .map(|slice| u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal well-formed XDV: preamble, `pages` bops, postamble.
    fn synthetic(pages: usize) -> Vec<u8> {
        let mut bytes = vec![247, 7];
        bytes.extend_from_slice(&[0; 12]);
        bytes.push(0);

        let mut previous = u32::MAX;
        let mut starts = Vec::new();

        for _ in 0..pages {
            let at = bytes.len() as u32;
            starts.push(at);
            bytes.push(op::BEGIN_PAGE);
            bytes.extend_from_slice(&[0; 40]);
            bytes.extend_from_slice(&previous.to_be_bytes());
            bytes.push(140); // eop
            previous = at;
        }

        let post = bytes.len() as u32;
        bytes.push(op::POSTAMBLE);
        bytes.extend_from_slice(&previous.to_be_bytes());
        bytes.extend_from_slice(&[0; 24]);
        bytes.push(249); // post_post
        bytes.extend_from_slice(&post.to_be_bytes());
        bytes.push(7);
        bytes.extend_from_slice(&[223; 4]);

        (bytes, starts).0
    }

    #[test]
    fn page_offsets_follow_the_backwards_chain() {
        let bytes = synthetic(3);
        let offsets = page_offsets(&bytes);
        assert_eq!(offsets.len(), 3);
        assert!(offsets.windows(2).all(|w| w[0] < w[1]));
        for &offset in &offsets {
            assert_eq!(bytes[offset as usize], op::BEGIN_PAGE);
        }
    }

    #[test]
    fn a_complete_document_replaces_everything() {
        let mut document = Document::default();
        document.apply(&synthetic(2), None).unwrap();
        assert_eq!(document.page_offsets().len(), 2);

        document.apply(&synthetic(5), None).unwrap();
        assert_eq!(document.page_offsets().len(), 5);
    }

    #[test]
    fn a_tail_keeps_the_prefix() {
        let whole = synthetic(4);
        let mut document = Document::default();
        document.apply(&whole, None).unwrap();

        // Re-splice the document onto itself at a page boundary: the result has
        // to be the same bytes back.
        let splice = document.page_offsets()[2] as usize;
        document.apply(&whole[splice..], Some(splice)).unwrap();

        assert_eq!(&*document.bytes(), &whole[..]);
        assert_eq!(document.page_offsets().len(), 4);
    }

    #[test]
    fn a_splice_past_the_end_is_refused() {
        let mut document = Document::default();
        document.apply(&synthetic(1), None).unwrap();
        assert!(document.apply(b"tail", Some(1_000_000)).is_err());
    }

    #[test]
    fn a_truncated_document_yields_no_offsets() {
        assert!(page_offsets(b"").is_empty());
        assert!(page_offsets(&[247, 7, 0, 0]).is_empty());
    }
}
