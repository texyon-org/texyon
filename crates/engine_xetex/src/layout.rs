//! Safe handles over the C libraries of the native font layer: HarfBuzz tags,
//! TECkit converters and ICU line breaking and bidi. Every raw pointer stays
//! inside this module.

use std::ffi::{c_char, c_int, c_long, c_void, CString};
use std::marker::PhantomData;
use std::ptr::{self, NonNull};

unsafe extern "C" {

    // TECkit_Status is a C `long`, which is 32-bit on wasm32 and Windows.
    fn TECkit_CreateConverter(
        mapping: *const u8,
        mapping_size: u32,
        map_forward: u8,
        source_form: u16,
        target_form: u16,
        converter: *mut *mut c_void,
    ) -> c_long;
    fn TECkit_ConvertBuffer(
        converter: *mut c_void,
        in_buffer: *const u8,
        in_length: u32,
        in_used: *mut u32,
        out_buffer: *mut u8,
        out_length: u32,
        out_used: *mut u32,
        input_is_complete: u8,
    ) -> c_long;
    fn TECkit_ResetConverter(converter: *mut c_void) -> c_long;
    fn TECkit_DisposeConverter(converter: *mut c_void) -> c_long;
}

// ICU checks the incoming UErrorCode, so each status starts at zero.
unsafe extern "C" {
    #[link_name = concat!("ubidi_open", env!("ICU_VERSION_SUFFIX"))]
    fn ubidi_open() -> *mut c_void;
    #[link_name = concat!("ubidi_close", env!("ICU_VERSION_SUFFIX"))]
    fn ubidi_close(bidi: *mut c_void);
    #[link_name = concat!("ubidi_setPara", env!("ICU_VERSION_SUFFIX"))]
    fn ubidi_setPara(
        bidi: *mut c_void,
        text: *const u16,
        length: i32,
        para_level: u8,
        embedding_levels: *mut u8,
        status: *mut i32,
    );
    #[link_name = concat!("ubidi_getDirection", env!("ICU_VERSION_SUFFIX"))]
    fn ubidi_getDirection(bidi: *mut c_void) -> c_int;
    #[link_name = concat!("ubidi_countRuns", env!("ICU_VERSION_SUFFIX"))]
    fn ubidi_countRuns(bidi: *mut c_void, status: *mut i32) -> c_int;
    #[link_name = concat!("ubidi_getVisualRun", env!("ICU_VERSION_SUFFIX"))]
    fn ubidi_getVisualRun(
        bidi: *mut c_void,
        run_index: i32,
        logical_start: *mut i32,
        length: *mut i32,
    ) -> c_int;
    #[link_name = concat!("ubrk_open", env!("ICU_VERSION_SUFFIX"))]
    fn ubrk_open(
        type_: c_int,
        locale: *const c_char,
        text: *const u16,
        text_length: i32,
        status: *mut i32,
    ) -> *mut c_void;
    #[link_name = concat!("ubrk_close", env!("ICU_VERSION_SUFFIX"))]
    fn ubrk_close(bi: *mut c_void);
    #[link_name = concat!("ubrk_setText", env!("ICU_VERSION_SUFFIX"))]
    fn ubrk_setText(bi: *mut c_void, text: *const u16, text_length: i32, status: *mut i32);
    #[link_name = concat!("ubrk_next", env!("ICU_VERSION_SUFFIX"))]
    fn ubrk_next(bi: *mut c_void) -> c_int;
}

/// Calls `f` with `bytes` as a C string, empty if `bytes` holds a NUL.
pub(crate) fn with_c_str<R>(bytes: &[u8], f: impl FnOnce(*const c_char) -> R) -> R {
    let s = CString::new(bytes).unwrap_or_default();
    f(s.as_ptr())
}

/// HarfBuzz's tag for the text `tag`.
pub(crate) fn hb_tag(tag: &[u8]) -> u32 {
    // SAFETY: `tag` is valid for the length passed.
    unsafe {
        tectonic_bridge_harfbuzz::sys::hb_tag_from_string(tag.as_ptr().cast(), tag.len() as c_int)
    }
}

/// An owned TECkit converter with exactly-once disposal.
pub(crate) struct FontMappingHandle {
    raw: NonNull<c_void>,
}

impl Drop for FontMappingHandle {
    fn drop(&mut self) {
        // SAFETY: the converter is owned here and the last owner is dropping it.
        unsafe { TECkit_DisposeConverter(self.raw.as_ptr()) };
    }
}

pub(crate) const K_STATUS_NO_ERROR: c_long = 0;
pub(crate) const K_STATUS_OUTPUT_BUFFER_FULL: c_long = 1;

impl FontMappingHandle {
    /// A converter for the compiled `mapping`, or a pure normalizer for `None`,
    /// with its creation status.
    pub(crate) fn new(
        mapping: Option<&[u8]>,
        map_forward: bool,
        source_form: u16,
        target_form: u16,
    ) -> (Option<FontMappingHandle>, c_long) {
        let mut raw: *mut c_void = ptr::null_mut();
        let (mapping_ptr, mapping_len) =
            mapping.map_or((ptr::null(), 0), |m| (m.as_ptr(), m.len()));
        // SAFETY: the mapping is valid for its length and `raw` is a valid out-parameter.
        let status = unsafe {
            TECkit_CreateConverter(
                mapping_ptr,
                mapping_len as u32,
                u8::from(map_forward),
                source_form,
                target_form,
                &mut raw,
            )
        };
        (
            NonNull::new(raw).map(|raw| FontMappingHandle { raw }),
            status,
        )
    }

    /// Converts all of `input` into `output`, then resets the converter.
    /// Returns the status and the bytes written.
    pub(crate) fn convert(&self, input: &[u8], output: &mut [u8]) -> (c_long, u32) {
        let (mut in_used, mut out_used) = (0u32, 0u32);
        // SAFETY: the converter is live and the buffers and counts are valid
        // for the lengths passed.
        unsafe {
            let status = TECkit_ConvertBuffer(
                self.raw.as_ptr(),
                input.as_ptr(),
                input.len() as u32,
                &mut in_used,
                output.as_mut_ptr(),
                output.len() as u32,
                &mut out_used,
                1,
            );
            TECkit_ResetConverter(self.raw.as_ptr());
            (status, out_used)
        }
    }
}

/// The bytes of UTF-16 `units`.
fn utf16_bytes(units: &[u16]) -> &[u8] {
    // SAFETY: `u16` has no padding, so the view covers initialised bytes.
    unsafe { std::slice::from_raw_parts(units.as_ptr().cast(), std::mem::size_of_val(units)) }
}

fn utf16_bytes_mut(units: &mut [u16]) -> &mut [u8] {
    // SAFETY: as in `utf16_bytes`, and any bytes written form valid `u16`s.
    unsafe {
        std::slice::from_raw_parts_mut(units.as_mut_ptr().cast(), std::mem::size_of_val(units))
    }
}

impl FontMappingHandle {
    /// `convert` over UTF-16 units. The byte count written comes back unchanged.
    pub(crate) fn convert_units(&self, input: &[u16], output: &mut [u16]) -> (c_long, u32) {
        self.convert(utf16_bytes(input), utf16_bytes_mut(output))
    }
}

/// An owned ICU line-break iterator. Its locale key is a TeX string-pool
/// index, so the cache belongs to one engine state and must not outlive it.
pub(crate) struct LinebreakHandle {
    raw: NonNull<c_void>,
    exhausted: bool,
}

impl Drop for LinebreakHandle {
    fn drop(&mut self) {
        // SAFETY: the iterator is owned here.
        unsafe { ubrk_close(self.raw.as_ptr()) };
    }
}

const UBRK_LINE: c_int = 2;

impl LinebreakHandle {
    /// A line-break iterator for `locale`, and ICU's status.
    pub(crate) fn open(locale: &[u8]) -> (Option<LinebreakHandle>, i32) {
        let locale = CString::new(locale).unwrap_or_default();
        let mut status = 0;
        // SAFETY: `locale` is NUL-terminated, no text is attached yet and
        // `status` is a valid out-parameter.
        let raw = unsafe { ubrk_open(UBRK_LINE, locale.as_ptr(), ptr::null(), 0, &mut status) };
        (
            NonNull::new(raw).map(|raw| LinebreakHandle {
                raw,
                exhausted: false,
            }),
            status,
        )
    }

    /// The break positions of `text`.
    pub(crate) fn breaks<'t>(&'t mut self, text: &'t [u16]) -> IcuBreaks<'t> {
        let mut status = 0;
        // SAFETY: the iterator is live, and the returned iterator keeps `text`
        // borrowed for as long as ICU may read it.
        unsafe {
            ubrk_setText(
                self.raw.as_ptr(),
                text.as_ptr(),
                text.len() as i32,
                &mut status,
            )
        };
        self.exhausted = false;
        IcuBreaks {
            iter: self,
            _text: PhantomData,
        }
    }

    /// The next break of text that ran out earlier: always the end.
    pub(crate) fn next_after_end(&self) -> c_int {
        debug_assert!(self.exhausted, "an ICU iterator reused mid-text");
        -1
    }
}

/// The break positions of the text given to [`LinebreakHandle::breaks`].
pub(crate) struct IcuBreaks<'t> {
    iter: &'t mut LinebreakHandle,
    _text: PhantomData<&'t [u16]>,
}

impl IcuBreaks<'_> {
    /// The next break offset, or -1 at the end.
    pub(crate) fn next_break(&mut self) -> c_int {
        // SAFETY: the iterator is live and its text is still borrowed.
        let offs = unsafe { ubrk_next(self.iter.raw.as_ptr()) };
        if offs < 0 {
            self.iter.exhausted = true;
        }
        offs
    }
}

/// ICU's bidi analysis of one paragraph of text.
pub(crate) struct Bidi<'t> {
    raw: NonNull<c_void>,
    /// The status of `ubidi_setPara`, which later calls see as their input.
    status: std::cell::Cell<i32>,
    _text: PhantomData<&'t [u16]>,
}

impl Drop for Bidi<'_> {
    fn drop(&mut self) {
        // SAFETY: the bidi object is owned here.
        unsafe { ubidi_close(self.raw.as_ptr()) };
    }
}

pub(crate) const UBIDI_RTL: c_int = 1;
pub(crate) const UBIDI_MIXED: c_int = 2;

impl<'t> Bidi<'t> {
    /// Analyses `text` at paragraph level `level`.
    pub(crate) fn new(text: &'t [u16], level: u8) -> Bidi<'t> {
        // SAFETY: plain allocation.
        let raw = NonNull::new(unsafe { ubidi_open() }).expect("ubidi_open failed");
        let mut status = 0;
        // SAFETY: `raw` is live, and the object keeps `text` borrowed for as long
        // as ICU may read it.
        unsafe {
            ubidi_setPara(
                raw.as_ptr(),
                text.as_ptr(),
                text.len() as i32,
                level,
                ptr::null_mut(),
                &mut status,
            )
        };
        Bidi {
            raw,
            status: std::cell::Cell::new(status),
            _text: PhantomData,
        }
    }

    pub(crate) fn direction(&self) -> c_int {
        // SAFETY: `raw` is a live bidi object.
        unsafe { ubidi_getDirection(self.raw.as_ptr()) }
    }

    pub(crate) fn count_runs(&self) -> c_int {
        let mut status = self.status.get();
        // SAFETY: `raw` is a live bidi object and `status` a valid out-parameter.
        let runs = unsafe { ubidi_countRuns(self.raw.as_ptr(), &mut status) };
        self.status.set(status);
        runs
    }

    /// The logical start, length and direction of visual run `index`.
    pub(crate) fn visual_run(&self, index: c_int) -> (i32, i32, c_int) {
        let (mut start, mut length) = (0, 0);
        // SAFETY: `raw` is a live bidi object and the outputs are valid.
        let dir = unsafe { ubidi_getVisualRun(self.raw.as_ptr(), index, &mut start, &mut length) };
        (start, length, dir)
    }
}
