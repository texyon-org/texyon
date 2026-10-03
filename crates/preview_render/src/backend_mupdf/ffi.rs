//! Safe handle over `shim.c`, which catches every MuPDF exception itself.

use std::ffi::{c_char, c_int, c_void, CStr};
use std::ptr::NonNull;

use crate::{Color, FontId, FontStyle, Matrix, Path, PathCommand, PositionedGlyph, StrokeStyle};

mod sys {
    use std::ffi::{c_char, c_int, c_void};

    use crate::{Matrix, StrokeStyle};

    extern "C" {
        pub(super) fn pr_backend_new() -> *mut c_void;
        pub(super) fn pr_backend_drop(backend: *mut c_void);
        pub(super) fn pr_backend_begin(backend: *mut c_void, width: f32, height: f32) -> c_int;
        pub(super) fn pr_backend_font(
            backend: *mut c_void,
            data: *const u8,
            length: usize,
            face: c_int,
        ) -> c_int;
        pub(super) fn pr_backend_gid_by_name(
            backend: *mut c_void,
            font: c_int,
            name: *const c_char,
        ) -> c_int;
        pub(super) fn pr_backend_glyphs(
            backend: *mut c_void,
            font: c_int,
            size: f32,
            gids: *const u16,
            transforms: *const Matrix,
            count: c_int,
            rgba: *const f32,
            extend: f32,
            slant: f32,
            embolden: f32,
            vertical: c_int,
        ) -> c_int;
        pub(super) fn pr_backend_rule(
            backend: *mut c_void,
            width: f32,
            height: f32,
            transform: Matrix,
            rgba: *const f32,
        ) -> c_int;
        pub(super) fn pr_backend_fill_path(
            backend: *mut c_void,
            kinds: *const u8,
            coordinates: *const f32,
            count: c_int,
            even_odd: c_int,
            transform: Matrix,
            rgba: *const f32,
        ) -> c_int;
        pub(super) fn pr_backend_stroke_path(
            backend: *mut c_void,
            kinds: *const u8,
            coordinates: *const f32,
            count: c_int,
            style: StrokeStyle,
            transform: Matrix,
            rgba: *const f32,
        ) -> c_int;
        pub(super) fn pr_backend_clip_path(
            backend: *mut c_void,
            kinds: *const u8,
            coordinates: *const f32,
            count: c_int,
            even_odd: c_int,
            transform: Matrix,
        ) -> c_int;
        pub(super) fn pr_backend_pop_clip(backend: *mut c_void) -> c_int;
        pub(super) fn pr_backend_image(
            backend: *mut c_void,
            data: *const u8,
            length: usize,
            transform: Matrix,
            width: f32,
            height: f32,
        ) -> c_int;
        pub(super) fn pr_backend_pdf(
            backend: *mut c_void,
            data: *const u8,
            length: usize,
            page: c_int,
            transform: Matrix,
        ) -> c_int;
        pub(super) fn pr_backend_begin_raster(
            backend: *mut c_void,
            width: c_int,
            height: c_int,
            scale: f32,
            samples: *mut u8,
        ) -> c_int;
        pub(super) fn pr_backend_finish_raster(backend: *mut c_void) -> c_int;
        pub(super) fn pr_backend_finish(backend: *mut c_void, length: *mut usize) -> *mut u8;
        #[cfg(test)]
        pub(super) fn pr_backend_raster(
            backend: *mut c_void,
            data: *const u8,
            length: usize,
            mime: *const c_char,
            page: c_int,
            dpi: f32,
            width: *mut c_int,
            height: *mut c_int,
            components: *mut c_int,
            sample_length: *mut usize,
        ) -> *mut u8;
        pub(super) fn pr_backend_free(data: *mut u8);
    }
}

/// A shim call failed, or its arguments could not be represented in C.
pub(super) struct ShimFailed;

type ShimResult<T> = Result<T, ShimFailed>;

fn check(ok: c_int) -> ShimResult<()> {
    if ok == 0 {
        Err(ShimFailed)
    } else {
        Ok(())
    }
}

fn to_c_int<T: TryInto<c_int>>(value: T) -> ShimResult<c_int> {
    value.try_into().map_err(|_| ShimFailed)
}

fn rgba(color: Color) -> [f32; 4] {
    [color.red, color.green, color.blue, color.alpha]
}

/// Buffer the shim allocated with `malloc`, freed on drop.
struct ShimBuffer {
    data: NonNull<u8>,
    length: usize,
}

impl ShimBuffer {
    /// # Safety
    ///
    /// `data` must be null or a shim allocation of `length` initialized bytes
    /// that nothing else frees.
    unsafe fn from_raw(data: *mut u8, length: usize) -> ShimResult<Self> {
        let data = NonNull::new(data).ok_or(ShimFailed)?;
        Ok(Self { data, length })
    }

    fn as_slice(&self) -> &[u8] {
        // SAFETY: `from_raw` requires `length` initialized bytes at `data`.
        unsafe { std::slice::from_raw_parts(self.data.as_ptr(), self.length) }
    }
}

impl Drop for ShimBuffer {
    fn drop(&mut self) {
        // SAFETY: the buffer is shim-owned and this is its only free.
        unsafe { sys::pr_backend_free(self.data.as_ptr()) };
    }
}

/// Path in the shim's encoding: one kind byte per command plus its coordinates.
struct FlatPath {
    kinds: Vec<u8>,
    coordinates: Vec<f32>,
    count: c_int,
}

impl FlatPath {
    fn new(path: &Path) -> ShimResult<Self> {
        let mut kinds = Vec::with_capacity(path.commands.len());
        let mut coordinates = Vec::new();
        for command in &path.commands {
            match command {
                PathCommand::MoveTo(x, y) => {
                    kinds.push(0);
                    coordinates.extend([x, y]);
                }
                PathCommand::LineTo(x, y) => {
                    kinds.push(1);
                    coordinates.extend([x, y]);
                }
                PathCommand::CurveTo(x1, y1, x2, y2, x3, y3) => {
                    kinds.push(2);
                    coordinates.extend([x1, y1, x2, y2, x3, y3]);
                }
                PathCommand::Rect(x0, y0, x1, y1) => {
                    kinds.push(3);
                    coordinates.extend([x0, y0, x1, y1]);
                }
                PathCommand::Close => kinds.push(4),
            }
        }
        let count = to_c_int(kinds.len())?;
        Ok(Self {
            kinds,
            coordinates,
            count,
        })
    }
}

/// Owned shim backend holding one MuPDF context and its registered fonts.
pub(super) struct Shim {
    raw: NonNull<c_void>,
    page_open: bool,
    /// The open raster page: its header, then room MuPDF draws the pixels into, and its length.
    raster: Option<(Vec<u8>, usize)>,
}

impl Shim {
    pub(super) fn new() -> Option<Self> {
        // SAFETY: the shim returns an owned backend or null.
        let raw = NonNull::new(unsafe { sys::pr_backend_new() })?;
        Some(Self {
            raw,
            page_open: false,
            raster: None,
        })
    }

    /// The raw handle, only while an SVG page device exists.
    fn page(&self) -> ShimResult<*mut c_void> {
        if self.page_open {
            Ok(self.raw.as_ptr())
        } else {
            Err(ShimFailed)
        }
    }

    pub(super) fn begin_page(&mut self, width: f32, height: f32) -> ShimResult<()> {
        // SAFETY: `raw` is live and the shim closes any previous page itself.
        let ok = unsafe { sys::pr_backend_begin(self.raw.as_ptr(), width, height) };
        // Only now: closing a raster page still draws into its pixels.
        self.raster = None;
        self.page_open = ok != 0;
        check(ok)
    }

    /// Open a `width` by `height` pixel page drawn at `scale` pixels per bp.
    pub(super) fn begin_raster(&mut self, width: u32, height: u32, scale: f32) -> ShimResult<()> {
        let pixels = (width as usize)
            .checked_mul(height as usize)
            .and_then(|count| count.checked_mul(4))
            .ok_or(ShimFailed)?;
        let length = 8 + pixels;
        let mut page = Vec::with_capacity(length);
        page.extend_from_slice(&width.to_le_bytes());
        page.extend_from_slice(&height.to_le_bytes());
        // SAFETY: `raw` is live, and the samples pointer has `pixels` bytes of
        // capacity that stay put until `finish_page` takes the vector.
        let ok = unsafe {
            sys::pr_backend_begin_raster(
                self.raw.as_ptr(),
                to_c_int(width)?,
                to_c_int(height)?,
                scale,
                page.as_mut_ptr().add(8),
            )
        };
        // Replaced only now: closing a previous raster page still draws into its pixels.
        self.raster = (ok != 0).then_some((page, length));
        self.page_open = ok != 0;
        check(ok)
    }

    pub(super) fn finish_page(&mut self) -> ShimResult<Vec<u8>> {
        let raw = self.page()?;
        self.page_open = false;
        if let Some((mut page, length)) = self.raster.take() {
            // SAFETY: a raster page is open on `raw`.
            check(unsafe { sys::pr_backend_finish_raster(raw) })?;
            // SAFETY: the header was written and MuPDF cleared every pixel when
            // the page opened, so all `length` bytes of capacity are initialized.
            unsafe { page.set_len(length) };
            return Ok(page);
        }
        let mut length = 0;
        // SAFETY: a page is open and `length` outlives the call.
        let data = unsafe { sys::pr_backend_finish(raw, &mut length) };
        // SAFETY: the shim returned `length` bytes of its own allocation, or null.
        let buffer = unsafe { ShimBuffer::from_raw(data, length) }?;
        Ok(buffer.as_slice().to_vec())
    }

    pub(super) fn register_font(&mut self, bytes: &[u8], face_index: u32) -> ShimResult<FontId> {
        let face = to_c_int(face_index)?;
        // SAFETY: `bytes` is live for the call and the shim copies it.
        let font =
            unsafe { sys::pr_backend_font(self.raw.as_ptr(), bytes.as_ptr(), bytes.len(), face) };
        u32::try_from(font).map(FontId).map_err(|_| ShimFailed)
    }

    /// Glyph for a PostScript name, or `None` when the font lacks it.
    pub(super) fn glyph_by_name(&mut self, font: FontId, name: &CStr) -> Option<u16> {
        let font = to_c_int(font.0).ok()?;
        let name: *const c_char = name.as_ptr();
        // SAFETY: `name` is NUL-terminated and live, and the shim bounds-checks `font`.
        let glyph = unsafe { sys::pr_backend_gid_by_name(self.raw.as_ptr(), font, name) };
        u16::try_from(glyph).ok()
    }

    pub(super) fn fill_rule(
        &mut self,
        width: f32,
        height: f32,
        transform: Matrix,
        color: Color,
    ) -> ShimResult<()> {
        let raw = self.page()?;
        let rgba = rgba(color);
        // SAFETY: a page is open and `rgba` holds the four floats the shim reads.
        check(unsafe { sys::pr_backend_rule(raw, width, height, transform, rgba.as_ptr()) })
    }

    pub(super) fn fill_path(
        &mut self,
        path: &Path,
        even_odd: bool,
        transform: Matrix,
        color: Color,
    ) -> ShimResult<()> {
        let raw = self.page()?;
        let path = FlatPath::new(path)?;
        let rgba = rgba(color);
        // SAFETY: a page is open and `FlatPath` matches its kinds to its coordinates.
        check(unsafe {
            sys::pr_backend_fill_path(
                raw,
                path.kinds.as_ptr(),
                path.coordinates.as_ptr(),
                path.count,
                c_int::from(even_odd),
                transform,
                rgba.as_ptr(),
            )
        })
    }

    pub(super) fn stroke_path(
        &mut self,
        path: &Path,
        style: StrokeStyle,
        transform: Matrix,
        color: Color,
    ) -> ShimResult<()> {
        let raw = self.page()?;
        let path = FlatPath::new(path)?;
        let rgba = rgba(color);
        // SAFETY: a page is open and `FlatPath` matches its kinds to its coordinates.
        check(unsafe {
            sys::pr_backend_stroke_path(
                raw,
                path.kinds.as_ptr(),
                path.coordinates.as_ptr(),
                path.count,
                style,
                transform,
                rgba.as_ptr(),
            )
        })
    }

    pub(super) fn clip_path(
        &mut self,
        path: &Path,
        even_odd: bool,
        transform: Matrix,
    ) -> ShimResult<()> {
        let raw = self.page()?;
        let path = FlatPath::new(path)?;
        // SAFETY: a page is open and `FlatPath` matches its kinds to its coordinates.
        check(unsafe {
            sys::pr_backend_clip_path(
                raw,
                path.kinds.as_ptr(),
                path.coordinates.as_ptr(),
                path.count,
                c_int::from(even_odd),
                transform,
            )
        })
    }

    pub(super) fn pop_clip(&mut self) -> ShimResult<()> {
        let raw = self.page()?;
        // SAFETY: a page is open.
        check(unsafe { sys::pr_backend_pop_clip(raw) })
    }

    pub(super) fn fill_glyphs(
        &mut self,
        font: FontId,
        size: f32,
        glyphs: &[PositionedGlyph],
        color: Color,
        style: FontStyle,
    ) -> ShimResult<()> {
        let raw = self.page()?;
        let font = to_c_int(font.0)?;
        let count = to_c_int(glyphs.len())?;
        let gids: Vec<_> = glyphs.iter().map(|glyph| glyph.glyph_id).collect();
        let transforms: Vec<_> = glyphs.iter().map(|glyph| glyph.transform).collect();
        let rgba = rgba(color);
        // SAFETY: a page is open and `gids` and `transforms` both hold `count` entries.
        check(unsafe {
            sys::pr_backend_glyphs(
                raw,
                font,
                size,
                gids.as_ptr(),
                transforms.as_ptr(),
                count,
                rgba.as_ptr(),
                style.extend,
                style.slant,
                style.embolden,
                c_int::from(style.vertical),
            )
        })
    }

    pub(super) fn draw_image(
        &mut self,
        bytes: &[u8],
        transform: Matrix,
        width: Option<f32>,
        height: Option<f32>,
    ) -> ShimResult<()> {
        let raw = self.page()?;
        // SAFETY: a page is open and `bytes` is live for the call.
        check(unsafe {
            sys::pr_backend_image(
                raw,
                bytes.as_ptr(),
                bytes.len(),
                transform,
                width.unwrap_or(f32::NAN),
                height.unwrap_or(f32::NAN),
            )
        })
    }

    pub(super) fn draw_pdf(
        &mut self,
        bytes: &[u8],
        page: u32,
        transform: Matrix,
    ) -> ShimResult<()> {
        let raw = self.page()?;
        let page = to_c_int(page)?;
        // SAFETY: a page is open and `bytes` is live for the call.
        check(unsafe { sys::pr_backend_pdf(raw, bytes.as_ptr(), bytes.len(), page, transform) })
    }

    /// Rasterize one page of a document in memory (fidelity-oracle tests only).
    #[cfg(test)]
    pub(super) fn rasterize(
        &self,
        bytes: &[u8],
        mime: &CStr,
        page: u32,
        dpi: f32,
    ) -> ShimResult<super::Pixmap> {
        let page = to_c_int(page)?;
        let (mut width, mut height, mut components, mut length) = (0, 0, 0, 0);
        // SAFETY: every pointer is live for the call and the outputs are plain ints.
        let data = unsafe {
            sys::pr_backend_raster(
                self.raw.as_ptr(),
                bytes.as_ptr(),
                bytes.len(),
                mime.as_ptr(),
                page,
                dpi,
                &mut width,
                &mut height,
                &mut components,
                &mut length,
            )
        };
        // SAFETY: the shim returned `length` bytes of its own allocation, or null.
        let buffer = unsafe { ShimBuffer::from_raw(data, length) }?;
        let dimension = |value: c_int| usize::try_from(value).map_err(|_| ShimFailed);
        Ok(super::Pixmap {
            width: dimension(width)?,
            height: dimension(height)?,
            components: dimension(components)?,
            samples: buffer.as_slice().to_vec(),
        })
    }
}

impl Drop for Shim {
    fn drop(&mut self) {
        // SAFETY: `raw` is uniquely owned and dropped exactly once.
        unsafe { sys::pr_backend_drop(self.raw.as_ptr()) };
    }
}
