//! Loading pictures and PDF pages for `\XeTeXpicfile` and `\XeTeXpdffile`.

use libc::{c_char, c_double, c_int, c_uint};

use crate::bridge::{input_open, FileFormat};
use crate::consts::{INVALID_HANDLE, PIC_NODE_SIZE};
use crate::errors::Abort;
use crate::ext::{d2fix, fix2d};
use crate::layout::with_c_str;
use crate::node::WhatsitSubtype;
use crate::state::EngineState;
use crate::units::Scaled;

#[repr(C)]
#[derive(Default)]
struct PdfTMatrix {
    a: c_double,
    b: c_double,
    c: c_double,
    d: c_double,
    e: c_double,
    f: c_double,
}

#[repr(C)]
#[derive(Default)]
struct PdfRect {
    llx: c_double,
    lly: c_double,
    urx: c_double,
    ury: c_double,
}

#[repr(C)]
struct PdfCoord {
    x: c_double,
    y: c_double,
}

#[derive(Clone, Copy)]
struct RealPoint {
    x: f32,
    y: f32,
}

struct Transform {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    x: f64,
    y: f64,
}

#[derive(Default)]
struct RealRect {
    x: f32,
    y: f32,
    wd: f32,
    ht: f32,
}

extern "C" {
    fn pdf_dev_transform(p: *mut PdfCoord, m: *const PdfTMatrix);

    // Guarded dpx probes. Each arms and restores the bridge jump buffer wholly
    // within C, so a dpx abort never longjmps over a Rust frame.
    fn tt_pic_probe_pdf_box(
        ident: *const c_char,
        handle: usize,
        page_num: c_int,
        dpx_options: c_int,
        bbox: *mut PdfRect,
        matrix: *mut PdfTMatrix,
    ) -> c_int;
    fn tt_pic_probe_image_box(
        handle: usize,
        width: *mut c_uint,
        height: *mut c_uint,
        xdensity: *mut c_double,
        ydensity: *mut c_double,
    ) -> c_int;
    fn tt_pic_probe_pdf_page_count(
        ident: *const c_char,
        handle: usize,
        pages_out: *mut c_int,
    ) -> c_int;
}

const PIC_PROBE_OK: c_int = 0;
/// The probe error for a missing file.
const FILE_NOT_FOUND: c_int = -43;

/// PDF page-box selector for `\XeTeXpdffile`, stored in the picture node and
/// echoed by `pic_out`. `Unset` (no keyword) extracts the crop box but prints
/// nothing. A non-PDF picture stores 0, the `None` of an `Option<PageboxType>`.
#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(u16)]
pub(crate) enum PageboxType {
    Crop = 1,
    Media = 2,
    Bleed = 3,
    Trim = 4,
    Art = 5,
    Unset = 6,
}

impl PageboxType {
    pub(crate) fn from_raw(v: u16) -> Option<PageboxType> {
        match v {
            1 => Some(PageboxType::Crop),
            2 => Some(PageboxType::Media),
            3 => Some(PageboxType::Bleed),
            4 => Some(PageboxType::Trim),
            5 => Some(PageboxType::Art),
            6 => Some(PageboxType::Unset),
            _ => None,
        }
    }

    pub(crate) fn label(self) -> Option<&'static str> {
        match self {
            PageboxType::Crop => Some("pagebox cropbox "),
            PageboxType::Media => Some("pagebox mediabox "),
            PageboxType::Bleed => Some("pagebox bleedbox "),
            PageboxType::Trim => Some("pagebox trimbox "),
            PageboxType::Art => Some("pagebox artbox "),
            PageboxType::Unset => None,
        }
    }

    fn dpx_option(self) -> c_int {
        match self {
            PageboxType::Media => 2,
            PageboxType::Bleed => 5,
            PageboxType::Trim => 4,
            PageboxType::Art => 3,
            PageboxType::Crop | PageboxType::Unset => 1,
        }
    }
}

fn min4(v: [f64; 4]) -> f64 {
    v[1..].iter().fold(v[0], |m, &x| if x < m { x } else { m })
}

fn max4(v: [f64; 4]) -> f64 {
    v[1..].iter().fold(v[0], |m, &x| if x > m { x } else { m })
}

/// The bounding box of page `page_num` of a PDF, in TeX points.
fn pdf_get_rect(
    filename: &[u8],
    handle: usize,
    page_num: c_int,
    pdf_box: PageboxType,
) -> Option<RealRect> {
    let mut bbox = PdfRect::default();
    let mut matrix = PdfTMatrix::default();
    let status = with_c_str(filename, |filename| {
        crate::bridge::with_c_io(|| {
            // SAFETY: `filename` is NUL-terminated, `handle` is an open input and
            // the box and matrix are valid out-parameters.
            unsafe {
                tt_pic_probe_pdf_box(
                    filename,
                    handle,
                    page_num,
                    pdf_box.dpx_option(),
                    &mut bbox,
                    &mut matrix,
                )
            }
        })
    });
    if status != PIC_PROBE_OK {
        return None;
    }
    let mut corners = [
        PdfCoord {
            x: bbox.llx,
            y: bbox.lly,
        },
        PdfCoord {
            x: bbox.urx,
            y: bbox.lly,
        },
        PdfCoord {
            x: bbox.urx,
            y: bbox.ury,
        },
        PdfCoord {
            x: bbox.llx,
            y: bbox.ury,
        },
    ];
    for p in &mut corners {
        // SAFETY: both arguments are valid for the call.
        unsafe { pdf_dev_transform(p, &matrix) };
    }
    let xs = corners.each_ref().map(|p| p.x);
    let ys = corners.each_ref().map(|p| p.y);
    let (llx, lly, urx, ury) = (min4(xs), min4(ys), max4(xs), max4(ys));
    const BP: f64 = 72.27 / 72.0;
    Some(RealRect {
        x: (BP * llx) as f32,
        y: (BP * lly) as f32,
        wd: (BP * (urx - llx)) as f32,
        ht: (BP * (ury - lly)) as f32,
    })
}

/// Width and height of a raster image in inches, or the probe error.
fn get_image_size_in_inches(handle: usize) -> Result<(f64, f64), c_int> {
    let (mut width_pix, mut height_pix) = (0, 0);
    let (mut xdensity, mut ydensity) = (0.0, 0.0);
    let err = crate::bridge::with_c_io(|| {
        // SAFETY: `handle` is an open input and the sizes are valid out-parameters.
        unsafe {
            tt_pic_probe_image_box(
                handle,
                &mut width_pix,
                &mut height_pix,
                &mut xdensity,
                &mut ydensity,
            )
        }
    });
    if err != PIC_PROBE_OK {
        return Err(err);
    }
    Ok((
        f64::from(width_pix) * xdensity / 72.0,
        f64::from(height_pix) * ydensity / 72.0,
    ))
}

/// Measure the picture `name_of_file`, returning the probe status and its bounds.
fn find_pic_file(
    name_of_file: &[u8],
    pdf_box: Option<PageboxType>,
    page: c_int,
) -> Result<(c_int, RealRect), Abort> {
    let handle = input_open(name_of_file, FileFormat::Pict, false);
    if handle == INVALID_HANDLE {
        return Ok((1, RealRect::default()));
    }

    let (err, bounds) = match pdf_box {
        Some(pdf_box) => match pdf_get_rect(name_of_file, handle, page, pdf_box) {
            Some(bounds) => (0, bounds),
            None => (-1, RealRect::default()),
        },
        None => {
            let (err, (wd, ht)) = match get_image_size_in_inches(handle) {
                Ok(size) => (0, size),
                Err(e) => (e, (-1.0, -1.0)),
            };
            let bounds = RealRect {
                wd: (wd * 72.27) as f32,
                ht: (ht * 72.27) as f32,
                ..RealRect::default()
            };
            (err, bounds)
        }
    };
    if crate::io::input_close(handle) {
        return Err(Abort::InputClose);
    }
    Ok((err, bounds))
}

impl Transform {
    const IDENTITY: Transform = Transform::scale(1.0, 1.0);

    const fn scale(xscale: f64, yscale: f64) -> Transform {
        Transform {
            a: xscale,
            b: 0.0,
            c: 0.0,
            d: yscale,
            x: 0.0,
            y: 0.0,
        }
    }

    fn translation(dx: f64, dy: f64) -> Transform {
        Transform {
            x: dx,
            y: dy,
            ..Transform::IDENTITY
        }
    }

    fn rotation(a: f64) -> Transform {
        Transform {
            a: a.cos(),
            b: a.sin(),
            c: -a.sin(),
            d: a.cos(),
            x: 0.0,
            y: 0.0,
        }
    }

    fn apply(&self, p: &mut RealPoint) {
        let rx = self.a * f64::from(p.x) + self.c * f64::from(p.y) + self.x;
        let ry = self.b * f64::from(p.x) + self.d * f64::from(p.y) + self.y;
        p.x = rx as f32;
        p.y = ry as f32;
    }

    /// Follow this transform by `t2`.
    fn concat(&mut self, t2: &Transform) {
        *self = Transform {
            a: self.a * t2.a + self.b * t2.c,
            b: self.a * t2.b + self.b * t2.d,
            c: self.c * t2.a + self.d * t2.c,
            d: self.c * t2.b + self.d * t2.d,
            x: self.x * t2.a + self.y * t2.c + t2.x,
            y: self.x * t2.b + self.y * t2.d + t2.y,
        };
    }
}

/// The picture corners and the transform that produced them.
struct Placement {
    corners: [RealPoint; 4],
    t: Transform,
}

impl Placement {
    fn transform(&mut self, t2: &Transform) {
        for c in &mut self.corners {
            t2.apply(c);
        }
        self.t.concat(t2);
    }

    /// `(xmin, xmax, ymin, ymax)` of the corners.
    fn bbox(&self) -> (f64, f64, f64, f64) {
        let xs = self.corners.map(|p| p.x);
        let ys = self.corners.map(|p| p.y);
        let min = |v: [f32; 4]| f64::from(v.into_iter().fold(f32::INFINITY, f32::min));
        let max = |v: [f32; 4]| f64::from(v.into_iter().fold(f32::NEG_INFINITY, f32::max));
        (min(xs), max(xs), min(ys), max(ys))
    }

    /// Scale to a requested width and/or height, keeping the aspect ratio
    /// when only one is given.
    fn scale_to(&mut self, x_size_req: f64, y_size_req: f64) {
        let (xmin, xmax, ymin, ymax) = self.bbox();
        let (sx, sy) = if x_size_req == 0.0 {
            let s = y_size_req / (ymax - ymin);
            (s, s)
        } else if y_size_req == 0.0 {
            let s = x_size_req / (xmax - xmin);
            (s, s)
        } else {
            (x_size_req / (xmax - xmin), y_size_req / (ymax - ymin))
        };
        self.transform(&Transform::scale(sx, sy));
    }
}

/// Scan an image dimension, reporting and ignoring non-positive values.
fn scan_image_size(st: &mut EngineState) -> Result<Option<f64>, Abort> {
    crate::scan::scan_dimen(st, false, false, false)?;
    if st.scanner.cur_val > 0 {
        return Ok(Some(fix2d(st.scanner.cur_dimen())));
    }
    crate::output::error_here_with_diagnostic(st, "Improper image ");
    crate::output::print(st, "size (");
    crate::output::print_scaled(st, st.scanner.cur_dimen());
    crate::output::print(st, "pt) will be ignored");
    crate::output::capture_to_diagnostic(st, None);
    st.errors.help(&[
        "I can't scale images to zero or negative sizes,",
        "so I'm ignoring this.",
    ]);
    crate::errors::error(st)?;
    Ok(None)
}

fn scan_scale_factor(st: &mut EngineState) -> Result<f64, Abort> {
    crate::scan::scan_int(st)?;
    Ok(f64::from(st.scanner.cur_val) / 1000.0)
}

pub(crate) fn load_picture(st: &mut EngineState, is_pdf: bool) -> Result<(), Abort> {
    crate::io::scan_and_pack_name(st)?;

    let mut pdf_box_type = None;
    let mut page = 0;
    if is_pdf {
        if crate::scan::scan_keyword(st, "page")? {
            crate::scan::scan_int(st)?;
            page = st.scanner.cur_val;
        }
        let keywords = [
            ("crop", PageboxType::Crop),
            ("media", PageboxType::Media),
            ("bleed", PageboxType::Bleed),
            ("trim", PageboxType::Trim),
            ("art", PageboxType::Art),
        ];
        let mut pdf_box = PageboxType::Unset;
        for (keyword, pb) in keywords {
            if crate::scan::scan_keyword(st, keyword)? {
                pdf_box = pb;
                break;
            }
        }
        pdf_box_type = Some(pdf_box);
    }

    let pic_path = st.input.name_of_file_bytes().to_vec();
    let (result, bounds) = find_pic_file(&pic_path, pdf_box_type, page)?;

    let (x, y, wd, ht) = (bounds.x, bounds.y, bounds.wd, bounds.ht);
    let mut placement = Placement {
        corners: [
            RealPoint { x, y },
            RealPoint { x, y: y + ht },
            RealPoint {
                x: x + wd,
                y: y + ht,
            },
            RealPoint { x: x + wd, y },
        ],
        t: Transform::IDENTITY,
    };
    let mut x_size_req = 0.0;
    let mut y_size_req = 0.0;
    loop {
        let no_size_req = x_size_req == 0.0 && y_size_req == 0.0;
        if crate::scan::scan_keyword(st, "scaled")? {
            let s = scan_scale_factor(st)?;
            if no_size_req {
                placement.transform(&Transform::scale(s, s));
            }
        } else if crate::scan::scan_keyword(st, "xscaled")? {
            let s = scan_scale_factor(st)?;
            if no_size_req {
                placement.transform(&Transform::scale(s, 1.0));
            }
        } else if crate::scan::scan_keyword(st, "yscaled")? {
            let s = scan_scale_factor(st)?;
            if no_size_req {
                placement.transform(&Transform::scale(1.0, s));
            }
        } else if crate::scan::scan_keyword(st, "width")? {
            if let Some(size) = scan_image_size(st)? {
                x_size_req = size;
            }
        } else if crate::scan::scan_keyword(st, "height")? {
            if let Some(size) = scan_image_size(st)? {
                y_size_req = size;
            }
        } else if crate::scan::scan_keyword(st, "rotated")? {
            crate::scan::xetex_scan_dimen(st, false, false, false, false)?;
            if !no_size_req {
                placement.scale_to(x_size_req, y_size_req);
                x_size_req = 0.0;
                y_size_req = 0.0;
            }
            let angle = fix2d(st.scanner.cur_dimen()) * std::f64::consts::PI / 180.0;
            let rotation = Transform::rotation(angle);
            for c in &mut placement.corners {
                rotation.apply(c);
            }
            let (xmin, xmax, ymin, ymax) = placement.bbox();
            let (xmin, xmax, ymin, ymax) = (xmin as f32, xmax as f32, ymin as f32, ymax as f32);
            placement.corners = [
                RealPoint { x: xmin, y: ymin },
                RealPoint { x: xmin, y: ymax },
                RealPoint { x: xmax, y: ymax },
                RealPoint { x: xmax, y: ymin },
            ];
            placement.t.concat(&rotation);
        } else {
            break;
        }
    }
    if x_size_req != 0.0 || y_size_req != 0.0 {
        placement.scale_to(x_size_req, y_size_req);
    }

    let (xmin, xmax, ymin, ymax) = placement.bbox();
    placement.t.concat(&Transform::translation(
        -f64::from(xmin as i32) * 72.0 / 72.27,
        -f64::from(ymin as i32) * 72.0 / 72.27,
    ));

    if result == 0 {
        let word_bytes = std::mem::size_of::<crate::mem_types::MemoryWord>();
        let extra_words = pic_path.len().div_ceil(word_bytes) as i32;
        let pic = st.input.cur_list.new_whatsit(
            &mut st.mem,
            WhatsitSubtype::Pic,
            PIC_NODE_SIZE + extra_words,
        )?;
        let mem = &mut st.mem;
        if is_pdf {
            pic.set_subtype(mem, WhatsitSubtype::Pdf as u16);
        }
        pic.set_pic_path_len(mem, pic_path.len() as u16);
        pic.set_pic_page(mem, page as u16);
        pic.set_pic_pagebox(mem, pdf_box_type.map_or(0, |pb| pb as u16));
        pic.set_width(mem, d2fix(xmax - xmin));
        pic.set_height(mem, d2fix(ymax - ymin));
        pic.set_depth(mem, Scaled::ZERO);
        let t = &placement.t;
        pic.set_pic_transform(mem, [t.a, t.b, t.c, t.d, t.x, t.y].map(d2fix));
        pic.set_pic_path(mem, &pic_path);
    } else {
        crate::output::error_here_with_diagnostic(st, "Unable to load picture or PDF file '");
        let (n, a, e) = (st.scanner.cur_name, st.scanner.cur_area, st.scanner.cur_ext);
        crate::output::print_file_name(st, n, a, e);
        crate::output::print(st, i32::from(b'\''));
        crate::output::capture_to_diagnostic(st, None);
        let reason = if result == FILE_NOT_FOUND {
            "the file was not found."
        } else {
            "it was not a recognized image format."
        };
        st.errors
            .help(&["The requested image couldn't be read because", reason]);
        crate::errors::error(st)?;
    }
    Ok(())
}

pub(crate) fn count_pdf_file_pages_named(name: &[u8]) -> Result<c_int, Abort> {
    let handle = input_open(name, FileFormat::Pict, false);
    if handle == INVALID_HANDLE {
        return Ok(0);
    }
    let mut pages = 0;
    let status = with_c_str(name, |name| {
        // SAFETY: `name` is NUL-terminated, `handle` is an open input and
        // `pages` is a valid out-parameter.
        crate::bridge::with_c_io(|| unsafe {
            tt_pic_probe_pdf_page_count(name, handle, &mut pages)
        })
    });
    if status != PIC_PROBE_OK {
        pages = 0;
    }
    if crate::io::input_close(handle) {
        return Err(Abort::InputClose);
    }
    Ok(pages)
}
