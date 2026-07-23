//! Sound byte-span facts recorded at XDV shipout boundaries.

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
}

/// Operation-local output observer. Batch runs leave it disabled.
///
/// Only byte positions are tracked; the manifest never retains a second copy
/// of the emitted XDV bytes.
#[derive(Clone, Default)]
pub(crate) struct PageManifest {
    enabled: bool,
    span_start: u64,
    span_len: u64,
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
        self.span_len = 0;
        self.page_bop = None;
        self.pages.clear();
    }

    pub(crate) fn record_byte(&mut self, _byte: u8) {
        if self.enabled {
            self.span_len += 1;
        }
    }

    pub(crate) fn pop_byte(&mut self) {
        if self.enabled {
            self.span_len = self
                .span_len
                .checked_sub(1)
                .expect("DVI pop before manifest span start");
        }
    }

    pub(crate) fn patch_byte(&mut self, offset: u64, _delta: u8) {
        if !self.enabled {
            return;
        }
        let index = offset
            .checked_sub(self.span_start)
            .expect("DVI backpatch before manifest span start");
        assert!(index < self.span_len, "DVI backpatch beyond manifest span");
    }

    pub(crate) fn begin_page(&mut self, bop_offset: u64) {
        if !self.enabled {
            return;
        }
        assert!(self.page_bop.is_none(), "nested XDV page manifest");
        assert_eq!(
            bop_offset,
            self.span_start + self.span_len,
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
        let span_end = self.span_start + self.span_len;
        self.pages.push(PageExecution {
            logical_clock,
            bop_offset,
            span_start: self.span_start,
            span_end,
        });
        self.span_start = span_end;
        self.span_len = 0;
    }

    pub(crate) fn take_pages(&mut self) -> Vec<PageExecution> {
        std::mem::take(&mut self.pages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_reflect_recorded_and_backpatched_bytes() {
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
    }
}
