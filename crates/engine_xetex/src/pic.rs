//! Loading pictures and PDF pages for `\XeTeXpicfile` and `\XeTeXpdffile`.

use libc::{c_char, c_double, c_int, c_uint};

use crate::bridge::{input_open, FileFormat};
use crate::consts::INVALID_HANDLE;
use crate::errors::Abort;
use crate::ext::{d2fix, fix2d};
use crate::layout::with_c_str;
use crate::node::{PicNode, WhatsitSubtype};
use crate::state::EngineState;
use crate::units::Scaled;

#[repr(C)]
#[derive(Default)]
struct PdfRect {
    llx: c_double,
    lly: c_double,
    urx: c_double,
    ury: c_double,
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
    // Guarded dpx probes. Each arms and restores the bridge jump buffer wholly
    // within C, so a dpx abort never longjmps over a Rust frame.
    fn tt_pic_probe_pdf_box(
        ident: *const c_char,
        handle: usize,
        page_num: c_int,
        dpx_options: c_int,
        bbox: *mut PdfRect,
        rotate: *mut c_int,
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

crate::codes::code_enum! {
    /// PDF page-box selector for `\XeTeXpdffile`, stored in the picture node and
    /// echoed by `pic_out`. `Unset` (no keyword) extracts the crop box but prints
    /// nothing. A non-PDF picture stores 0, the `None` of an `Option<PageboxType>`.
    pub(crate) enum PageboxType: u16 {
        Crop = 1,
        Media = 2,
        Bleed = 3,
        Trim = 4,
        Art = 5,
        Unset = 6,
    }
}

impl PageboxType {
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

/// The bounding box of page `page_num` of a PDF, in TeX points.
fn pdf_get_rect(
    filename: &[u8],
    handle: usize,
    page_num: c_int,
    pdf_box: PageboxType,
) -> Option<RealRect> {
    let mut bbox = PdfRect::default();
    let mut rotate: c_int = 0;
    let status = with_c_str(filename, |filename| {
        crate::bridge::with_c_io(|| {
            // SAFETY: `filename` is NUL-terminated, `handle` is an open input and
            // the box and rotation are valid out-parameters.
            unsafe {
                tt_pic_probe_pdf_box(
                    filename,
                    handle,
                    page_num,
                    pdf_box.dpx_option(),
                    &mut bbox,
                    &mut rotate,
                )
            }
        })
    });
    if status != PIC_PROBE_OK {
        return None;
    }
    const BP: f64 = 72.27 / 72.0;
    let (w, h) = ((bbox.urx - bbox.llx).abs(), (bbox.ury - bbox.lly).abs());
    let (wd, ht) = match rotate.rem_euclid(360) {
        90 | 270 => (h, w),
        _ => (w, h),
    };
    Some(RealRect {
        x: (BP * bbox.llx.min(bbox.urx)) as f32,
        y: (BP * bbox.lly.min(bbox.ury)) as f32,
        wd: (BP * wd) as f32,
        ht: (BP * ht) as f32,
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

/// Measure the picture at `path`, returning the probe status, its bounds and
/// the name it was found under.
fn find_pic_file(
    path: &[u8],
    pdf_box: Option<PageboxType>,
    page: c_int,
) -> Result<(c_int, RealRect, Vec<u8>), Abort> {
    let handle = input_open(path, FileFormat::Pict, false);
    if handle == INVALID_HANDLE {
        return Ok((1, RealRect::default(), path.to_vec()));
    }
    let found = crate::io::last_found_name(path);

    let (err, bounds) = match pdf_box {
        Some(pdf_box) => match pdf_get_rect(path, handle, page, pdf_box) {
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
    Ok((err, bounds, found))
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
    let d = crate::scan::scan_dimen(st, false)?;
    if d > Scaled::ZERO {
        return Ok(Some(fix2d(d)));
    }
    crate::errors::error(
        st,
        ("Improper image ", "size (", d, "pt) will be ignored"),
        &[
            "I can't scale images to zero or negative sizes,",
            "so I'm ignoring this.",
        ],
    )?;
    Ok(None)
}

fn scan_scale_factor(st: &mut EngineState) -> Result<f64, Abort> {
    Ok(f64::from(crate::scan::scan_int(st)?) / 1000.0)
}

pub(crate) fn load_picture(st: &mut EngineState, is_pdf: bool) -> Result<(), Abort> {
    let file = crate::io::scan_file_name(st)?.file;

    let mut pdf_box_type = None;
    let mut page = 0;
    if is_pdf {
        if crate::scan::scan_keyword(st, "page")? {
            page = crate::scan::scan_int(st)?;
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

    let (result, bounds, pic_path) = find_pic_file(&file.path(&st.strings), pdf_box_type, page)?;

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
            let angle = crate::scan::scan_decimal(st)?;
            if !no_size_req {
                placement.scale_to(x_size_req, y_size_req);
                x_size_req = 0.0;
                y_size_req = 0.0;
            }
            let angle = fix2d(angle) * std::f64::consts::PI / 180.0;
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
        let subtype = if is_pdf {
            WhatsitSubtype::Pdf
        } else {
            WhatsitSubtype::Pic
        };
        let pic = st
            .mem
            .alloc_array::<PicNode>(usize::from(pic_path.len() as u16))?;
        st.input.cur_list.tail_append(&mut st.mem, pic);
        let t = &placement.t;
        let node = &mut st.mem[pic];
        node.subtype = subtype.into();
        node.page = page as u16;
        node.pagebox = pdf_box_type.map_or(0, |pb| pb as u16);
        node.width = d2fix(xmax - xmin);
        node.height = d2fix(ymax - ymin);
        node.depth = Scaled::ZERO;
        node.transform = [t.a, t.b, t.c, t.d, t.x, t.y].map(d2fix);
        let path = st.mem.entries_mut(pic);
        let stored = path.len();
        path.copy_from_slice(&pic_path[..stored]);
    } else {
        let reason = if result == FILE_NOT_FOUND {
            "the file was not found."
        } else {
            "it was not a recognized image format."
        };
        crate::errors::error(
            st,
            ("Unable to load picture or PDF file '", file, b'\''),
            &["The requested image couldn't be read because", reason],
        )?;
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
