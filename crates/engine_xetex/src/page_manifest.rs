//! Sound byte-span facts recorded at XDV shipout boundaries.

use sha2::{Digest, Sha256};

/// Facts about one page emitted by a resident XeTeX operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageExecution {
    /// Token-fetch clock observed after the page's EOP was emitted.
    pub logical_clock: u64,
    /// Absolute byte offset of the page's BOP opcode.
    pub bop_offset: u64,
    /// Start of the independently comparable XDV span containing this page.
    pub span_start: u64,
    /// Exclusive end of the independently comparable XDV span.
    pub span_end: u64,
    /// SHA-256 of `xdv[span_start..span_end]` after DVI backpatching.
    pub byte_digest: [u8; 32],
    /// Reserved for the in-memory SyncTeX index added by the mapping milestone.
    pub synctex_range: Option<std::ops::Range<usize>>,
}

/// Operation-local output observer. Batch runs leave it disabled.
#[derive(Clone, Default)]
pub(crate) struct PageManifest {
    enabled: bool,
    span_start: u64,
    span: Vec<u8>,
    page_bop: Option<u64>,
    pages: Vec<PageExecution>,
}

impl PageManifest {
    pub(crate) fn begin_operation(&mut self) {
        self.begin_operation_at(0);
    }

    /// Start observing newly emitted bytes at an existing absolute XDV
    /// position, as happens when a resident compile resumes from a fence.
    pub(crate) fn begin_operation_at(&mut self, span_start: u64) {
        self.enabled = true;
        self.span_start = span_start;
        self.span.clear();
        self.page_bop = None;
        self.pages.clear();
    }

    pub(crate) fn record_byte(&mut self, byte: u8) {
        if self.enabled {
            self.span.push(byte);
        }
    }

    pub(crate) fn pop_byte(&mut self) {
        if self.enabled {
            self.span.pop().expect("DVI pop before manifest span start");
        }
    }

    pub(crate) fn patch_byte(&mut self, offset: u64, delta: u8) {
        if !self.enabled {
            return;
        }
        let index = offset
            .checked_sub(self.span_start)
            .and_then(|value| usize::try_from(value).ok())
            .expect("DVI backpatch before manifest span start");
        self.span[index] = self.span[index].wrapping_add(delta);
    }

    pub(crate) fn begin_page(&mut self, bop_offset: u64) {
        if !self.enabled {
            return;
        }
        assert!(self.page_bop.is_none(), "nested XDV page manifest");
        assert_eq!(
            bop_offset,
            self.span_start + self.span.len() as u64,
            "XDV position diverged from page manifest"
        );
        self.page_bop = Some(bop_offset);
    }

    pub(crate) fn finish_page(&mut self, logical_clock: u64) {
        if !self.enabled {
            return;
        }
        let Some(bop_offset) = self.page_bop.take() else {
            return;
        };
        let span_end = self.span_start + self.span.len() as u64;
        let byte_digest = Sha256::digest(&self.span).into();
        self.pages.push(PageExecution {
            logical_clock,
            bop_offset,
            span_start: self.span_start,
            span_end,
            byte_digest,
            synctex_range: None,
        });
        self.span_start = span_end;
        self.span.clear();
    }

    pub(crate) fn take_pages(&mut self) -> Vec<PageExecution> {
        std::mem::take(&mut self.pages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_reflects_final_backpatched_bytes() {
        let mut manifest = PageManifest::default();
        manifest.begin_operation();
        manifest.record_byte(1);
        manifest.begin_page(1);
        manifest.record_byte(2);
        manifest.record_byte(3);
        manifest.patch_byte(1, 5);
        manifest.finish_page(9);

        let pages = manifest.take_pages();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].logical_clock, 9);
        assert_eq!(pages[0].bop_offset, 1);
        assert_eq!(pages[0].span_start, 0);
        assert_eq!(pages[0].span_end, 3);
        let expected: [u8; 32] = Sha256::digest([1, 7, 3]).into();
        assert_eq!(pages[0].byte_digest, expected);
    }
}
