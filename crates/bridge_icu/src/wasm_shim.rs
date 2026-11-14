//! The slice of ICU that XeTeX uses, reimplemented on pure-Rust crates.
//!
//! Tectonic normally links the system ICU and calls it through version-suffixed
//! symbols (`ubidi_open_76` and friends). A browser has no system ICU, and
//! shipping ICU4C's ~30 MB of locale data to one is not a serious option, so on
//! wasm the engine gets these symbols from here instead — unsuffixed, because
//! `texyon_engine_xetex`'s build script leaves `ICU_VERSION_SUFFIX` empty for
//! wasm targets.
//!
//! Three families are needed, all consumed from `texyon_engine_xetex`:
//!
//! - `ubidi_*` — the Unicode bidirectional algorithm, run for every native-font
//!   word node. Backed by [`unicode_bidi`], which implements the same UAX #9.
//! - `ubrk_*` — locale-aware line breaking, reached only through
//!   `\XeTeXlinebreaklocale`. Backed by [`icu_segmenter`], i.e. ICU4X: the same
//!   project as ICU4C, with the data compiled in.
//! - `ucnv_*` — legacy input encodings for `\XeTeXinputencoding`. Backed by
//!   [`encoding_rs`], which speaks the WHATWG labels; those cover the encodings
//!   ICU converter names denote in practice, but this is the one family where a
//!   name ICU would accept may be rejected here. The engine already treats a
//!   failed converter open as "read the file as raw bytes".
//!
//! Everything here is called from a single-threaded engine holding the global
//! engine lock, and the handles never leave that thread.

#![allow(nonstandard_style)]

use encoding_rs::Encoding;
use icu_segmenter::{LineSegmenter, LineSegmenterBorrowed};
use std::{ffi::CStr, ops::Range, ptr, slice};
use unicode_bidi::{Level, utf16::BidiInfo};

/// `U_ZERO_ERROR`. ICU reports success as zero and failures as positives; the
/// engine only ever tests `status > 0`.
const U_ZERO_ERROR: i32 = 0;
/// `U_ILLEGAL_ARGUMENT_ERROR`.
const U_ILLEGAL_ARGUMENT_ERROR: i32 = 1;
/// `U_BUFFER_OVERFLOW_ERROR`.
const U_BUFFER_OVERFLOW_ERROR: i32 = 15;

/// `UBIDI_LTR`: every character is left-to-right.
const UBIDI_LTR: libc::c_int = 0;
/// `UBIDI_RTL`: every character is right-to-left.
const UBIDI_RTL: libc::c_int = 1;
/// `UBIDI_MIXED`: the paragraph mixes both directions.
const UBIDI_MIXED: libc::c_int = 2;

/// `UBIDI_DEFAULT_LTR`: detect the paragraph direction, falling back to LTR.
const UBIDI_DEFAULT_LTR: u8 = 0xfe;
/// `UBIDI_DEFAULT_RTL`: detect the paragraph direction, falling back to RTL.
const UBIDI_DEFAULT_RTL: u8 = 0xff;

/// `UBRK_DONE`: the break iterator has passed the last boundary.
const UBRK_DONE: libc::c_int = -1;

/// Write through an ICU `UErrorCode*` out-parameter, which is allowed to be null.
///
/// # Safety
///
/// `status` must be null or point to a writable `i32`.
unsafe fn set_status(status: *mut i32, value: i32) {
    // SAFETY: forwarded from the caller.
    unsafe {
        if !status.is_null() {
            *status = value;
        }
    }
}

// ---------------------------------------------------------------------------
// ubidi_* — UAX #9, via unicode-bidi
// ---------------------------------------------------------------------------

/// The allocation behind an opaque `UBiDi`.
#[derive(Default)]
struct Bidi {
    /// The paragraph text, kept because `visual_runs` borrows the analysis.
    text: Vec<u16>,
    /// Embedding level per code unit, empty until `ubidi_setPara`.
    levels: Vec<Level>,
    /// Logical ranges in visual order.
    runs: Vec<Range<usize>>,
}

/// Allocate a bidi analysis object. Its text is set by [`ubidi_setPara`].
#[unsafe(no_mangle)]
pub extern "C" fn ubidi_open() -> *mut libc::c_void {
    Box::into_raw(Box::<Bidi>::default()) as *mut libc::c_void
}

/// Release an object from [`ubidi_open`].
#[unsafe(no_mangle)]
pub extern "C" fn ubidi_close(bidi: *mut libc::c_void) {
    if !bidi.is_null() {
        // SAFETY: the caller owns `bidi`, so it came from ubidi_open.
        drop(unsafe { Box::from_raw(bidi as *mut Bidi) });
    }
}

/// # Safety
///
/// `text` must point to `length` UTF-16 code units, and `bidi` must have come
/// from [`ubidi_open`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ubidi_setPara(
    bidi: *mut libc::c_void,
    text: *const u16,
    length: i32,
    para_level: u8,
    _embedding_levels: *mut u8,
    status: *mut i32,
) {
    // SAFETY: forwarded from the caller.
    unsafe {
        let Some(bidi) = (bidi as *mut Bidi).as_mut() else {
            set_status(status, U_ILLEGAL_ARGUMENT_ERROR);
            return;
        };

        if text.is_null() || length < 0 {
            set_status(status, U_ILLEGAL_ARGUMENT_ERROR);
            return;
        }

        // ICU's two "default" levels mean "run rule P2/P3 and fall back to this
        // direction if the paragraph has no strong character". unicode-bidi does
        // exactly that detection when given no level, but always falls back to LTR,
        // so DEFAULT_RTL needs the fallback applied by hand.
        let requested = match para_level {
            UBIDI_DEFAULT_LTR | UBIDI_DEFAULT_RTL => None,
            level => Some(Level::new(level & 1).unwrap()),
        };

        bidi.text = slice::from_raw_parts(text, length as usize).to_vec();

        let info = BidiInfo::new(&bidi.text, requested);
        let Some(para) = info.paragraphs.first() else {
            // An empty paragraph has no runs; leave the analysis cleared.
            bidi.levels.clear();
            bidi.runs.clear();
            set_status(status, U_ZERO_ERROR);
            return;
        };

        let para =
            if para_level == UBIDI_DEFAULT_RTL && para.level.is_ltr() && !has_strong(&bidi.text) {
                let mut para = para.clone();
                para.level = Level::rtl();
                para
            } else {
                para.clone()
            };

        let (levels, runs) = info.visual_runs(&para, para.range.clone());
        bidi.levels = levels;
        bidi.runs = runs;

        set_status(status, U_ZERO_ERROR);
    }
}

/// Whether the text contains a strong directional character, i.e. whether rule
/// P2 found anything and the `UBIDI_DEFAULT_*` fallback does not apply.
fn has_strong(text: &[u16]) -> bool {
    char::decode_utf16(text.iter().copied())
        .flatten()
        .any(|c| !matches!(unicode_bidi::bidi_class(c), unicode_bidi::BidiClass::ON))
}

/// # Safety
///
/// `bidi` must have come from [`ubidi_open`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ubidi_getDirection(bidi: *mut libc::c_void) -> libc::c_int {
    // SAFETY: forwarded from the caller.
    unsafe {
        let Some(bidi) = (bidi as *const Bidi).as_ref() else {
            return UBIDI_LTR;
        };

        let mut any_ltr = false;
        let mut any_rtl = false;

        for level in &bidi.levels {
            if level.is_rtl() {
                any_rtl = true;
            } else {
                any_ltr = true;
            }
        }

        match (any_ltr, any_rtl) {
            (true, true) => UBIDI_MIXED,
            (false, true) => UBIDI_RTL,
            // All-LTR and the empty text both read as LTR, matching ICU for the
            // paragraph levels XeTeX asks for.
            _ => UBIDI_LTR,
        }
    }
}

/// # Safety
///
/// `bidi` must have come from [`ubidi_open`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ubidi_countRuns(bidi: *mut libc::c_void, status: *mut i32) -> libc::c_int {
    // SAFETY: forwarded from the caller.
    unsafe {
        let Some(bidi) = (bidi as *const Bidi).as_ref() else {
            set_status(status, U_ILLEGAL_ARGUMENT_ERROR);
            return 0;
        };

        set_status(status, U_ZERO_ERROR);
        bidi.runs.len() as libc::c_int
    }
}

/// # Safety
///
/// `bidi` must have come from [`ubidi_open`], and the out-parameters must be
/// writable or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ubidi_getVisualRun(
    bidi: *mut libc::c_void,
    run_index: i32,
    logical_start: *mut i32,
    length: *mut i32,
) -> libc::c_int {
    // SAFETY: forwarded from the caller.
    unsafe {
        let Some(bidi) = (bidi as *const Bidi).as_ref() else {
            return UBIDI_LTR;
        };

        let Some(run) = usize::try_from(run_index)
            .ok()
            .and_then(|i| bidi.runs.get(i))
        else {
            return UBIDI_LTR;
        };

        if !logical_start.is_null() {
            *logical_start = run.start as i32;
        }
        if !length.is_null() {
            *length = run.len() as i32;
        }

        // Every code unit in a run shares its level, so the first one names the
        // run's direction.
        match bidi.levels.get(run.start) {
            Some(level) if level.is_rtl() => UBIDI_RTL,
            _ => UBIDI_LTR,
        }
    }
}

// ---------------------------------------------------------------------------
// ubrk_* — line breaking, via ICU4X
// ---------------------------------------------------------------------------

/// The allocation behind an opaque `UBreakIterator`.
struct BreakIterator {
    segmenter: LineSegmenterBorrowed<'static>,
    /// Boundaries after the current position, in ascending order.
    remaining: std::vec::IntoIter<usize>,
}

/// # Safety
///
/// `text` must point to `text_length` UTF-16 code units, or be null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ubrk_open(
    _type_: libc::c_int,
    _locale: *const libc::c_char,
    text: *const u16,
    text_length: i32,
    status: *mut i32,
) -> *mut libc::c_void {
    // SAFETY: forwarded from the caller.
    unsafe {
        // ICU4X's line segmenter is driven by the Unicode line breaking algorithm
        // and the compiled-in dictionary/LSTM data for the languages that need one
        // (Thai, Lao, Khmer, Burmese, Chinese, Japanese). It takes no locale, so
        // every locale XeTeX may name is accepted rather than rejected — which is
        // why this never reports the error that would send the engine to `en_us`.
        let mut iter = Box::new(BreakIterator {
            segmenter: LineSegmenter::new_auto(Default::default()),
            remaining: Vec::new().into_iter(),
        });

        if !text.is_null() && text_length > 0 {
            iter.set_text(slice::from_raw_parts(text, text_length as usize));
        }

        set_status(status, U_ZERO_ERROR);
        Box::into_raw(iter) as *mut libc::c_void
    }
}

impl BreakIterator {
    /// Recompute the boundary list and rewind to the start of the text.
    fn set_text(&mut self, text: &[u16]) {
        // ICU positions a fresh iterator *before* the first boundary and has
        // `next` return the ones after it, so the leading zero is dropped.
        let breaks: Vec<usize> = self
            .segmenter
            .segment_utf16(text)
            .filter(|&offset| offset != 0)
            .collect();

        self.remaining = breaks.into_iter();
    }
}

/// Release an iterator from [`ubrk_open`].
#[unsafe(no_mangle)]
pub extern "C" fn ubrk_close(bi: *mut libc::c_void) {
    if !bi.is_null() {
        // SAFETY: the caller owns `bi`, so it came from ubrk_open.
        drop(unsafe { Box::from_raw(bi as *mut BreakIterator) });
    }
}

/// # Safety
///
/// `bi` must have come from [`ubrk_open`] and `text` must point to
/// `text_length` UTF-16 code units.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ubrk_setText(
    bi: *mut libc::c_void,
    text: *const u16,
    text_length: i32,
    status: *mut i32,
) {
    // SAFETY: forwarded from the caller.
    unsafe {
        let Some(iter) = (bi as *mut BreakIterator).as_mut() else {
            set_status(status, U_ILLEGAL_ARGUMENT_ERROR);
            return;
        };

        if text.is_null() || text_length < 0 {
            set_status(status, U_ILLEGAL_ARGUMENT_ERROR);
            return;
        }

        iter.set_text(slice::from_raw_parts(text, text_length as usize));
        set_status(status, U_ZERO_ERROR);
    }
}

/// # Safety
///
/// `bi` must have come from [`ubrk_open`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ubrk_next(bi: *mut libc::c_void) -> libc::c_int {
    // SAFETY: forwarded from the caller.
    unsafe {
        let Some(iter) = (bi as *mut BreakIterator).as_mut() else {
            return UBRK_DONE;
        };

        match iter.remaining.next() {
            Some(offset) => offset as libc::c_int,
            None => UBRK_DONE,
        }
    }
}

// ---------------------------------------------------------------------------
// ucnv_* — legacy input encodings, via encoding_rs
// ---------------------------------------------------------------------------

/// The allocation behind an opaque `UConverter`.
struct Converter {
    encoding: &'static Encoding,
}

/// # Safety
///
/// `converter_name` must be a valid NUL-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ucnv_open(
    converter_name: *const libc::c_char,
    err: *mut i32,
) -> *mut libc::c_void {
    // SAFETY: forwarded from the caller.
    unsafe {
        if converter_name.is_null() {
            set_status(err, U_ILLEGAL_ARGUMENT_ERROR);
            return ptr::null_mut();
        }

        let Ok(name) = CStr::from_ptr(converter_name).to_str() else {
            set_status(err, U_ILLEGAL_ARGUMENT_ERROR);
            return ptr::null_mut();
        };

        let Some(encoding) = Encoding::for_label(name.as_bytes()) else {
            // The engine prints a diagnostic and falls back to reading raw bytes.
            set_status(err, U_ILLEGAL_ARGUMENT_ERROR);
            return ptr::null_mut();
        };

        set_status(err, U_ZERO_ERROR);
        Box::into_raw(Box::new(Converter { encoding })) as *mut libc::c_void
    }
}

/// Release a converter from [`ucnv_open`].
#[unsafe(no_mangle)]
pub extern "C" fn ucnv_close(converter: *mut libc::c_void) {
    if !converter.is_null() {
        // SAFETY: the caller owns `converter`, so it came from ucnv_open.
        drop(unsafe { Box::from_raw(converter as *mut Converter) });
    }
}

/// Convert `source` from the converter's encoding into native-endian UTF-32.
///
/// Only `UCNV_UTF32_NativeEndian` is implemented, because that is the one
/// algorithmic target XeTeX asks for. Returns the number of *bytes* written.
///
/// # Safety
///
/// `cnv` must have come from [`ucnv_open`]; `source` must point to
/// `source_length` bytes and `target` to `target_capacity` writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ucnv_toAlgorithmic(
    _algorithmic_type: libc::c_int,
    cnv: *mut libc::c_void,
    target: *mut libc::c_char,
    target_capacity: i32,
    source: *const libc::c_char,
    source_length: i32,
    err: *mut i32,
) -> i32 {
    // SAFETY: forwarded from the caller.
    unsafe {
        let Some(cnv) = (cnv as *const Converter).as_ref() else {
            set_status(err, U_ILLEGAL_ARGUMENT_ERROR);
            return 0;
        };

        if source.is_null() || target.is_null() || source_length < 0 || target_capacity < 0 {
            set_status(err, U_ILLEGAL_ARGUMENT_ERROR);
            return 0;
        }

        let source = slice::from_raw_parts(source as *const u8, source_length as usize);

        // Malformed input decodes to U+FFFD, which is what ICU's default callback
        // substitutes too.
        let (decoded, _, _) = cnv.encoding.decode(source);

        let capacity = target_capacity as usize / 4;
        let mut written = 0usize;

        for c in decoded.chars() {
            if written == capacity {
                set_status(err, U_BUFFER_OVERFLOW_ERROR);
                return 0;
            }

            // The engine's buffer is a `u32` array it hands us as bytes, so it is
            // aligned for this write.
            (target as *mut u32).add(written).write(c as u32);
            written += 1;
        }

        set_status(err, U_ZERO_ERROR);
        (written * 4) as i32
    }
}
