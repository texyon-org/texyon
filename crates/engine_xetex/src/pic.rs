// translated from xetex-pic.c

use libc::{c_char, c_double, c_int, c_uint, c_void};

// dpx layout types — must match C struct layout exactly (dpx-pdfdev.h)
use crate::bridge::ttstub_input_open;
use crate::consts::{INVALID_HANDLE, PIC_NODE_SIZE};
use crate::ext::D2Fix;
use crate::ext::Fix2D;
use crate::main_control::{mc_propagate_error, MainControlAbort};
use crate::state::EngineState;

#[repr(C)]
struct PdfTMatrix {
    a: c_double,
    b: c_double,
    c: c_double,
    d: c_double,
    e: c_double,
    f: c_double,
}

#[repr(C)]
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

// --- local helper types (real_point, transform_t, real_rect from xetex-pic.c) ---

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

struct RealRect {
    x: f32,
    y: f32,
    wd: f32,
    ht: f32,
}

// --- extern C ---

extern "C" {
    fn pdf_dev_transform(p: *mut PdfCoord, m: *const PdfTMatrix);
    fn min4(v1: c_double, v2: c_double, v3: c_double, v4: c_double) -> c_double;
    fn max4(v1: c_double, v2: c_double, v3: c_double, v4: c_double) -> c_double;
    // xetex-ext.c conversion helpers (still in C)

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
const TTBC_FILE_FORMAT_PICT: c_int = 25; // tectonic_bridge_core_generated.h:74

// pdfbox constants from xetex-ext.h:53–58
/// PDF page-box selector for `\XeTeXpdffile` (xetex-pic.c). The `\media`/`\crop`/…
/// keyword the user gave, stored in the pic node's `b16.s1(+8)` field and echoed
/// by the shipout `show` path. `Unset` is TeX's default (no keyword → treated as
/// crop for extraction, printed as nothing). A raw i32 of 0 (non-PDF image) is the
/// `None` of an `Option<PageboxType>`.
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
    /// Decode the stored `b16.s1(+8)` selector; `None` for 0 (non-PDF) or unknown.
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

    /// The `show`-box label, or `None` for `Unset` (no keyword ⇒ no echo).
    pub(crate) fn label(self) -> Option<&'static std::ffi::CStr> {
        match self {
            PageboxType::Crop => Some(c"pagebox cropbox "),
            PageboxType::Media => Some(c"pagebox mediabox "),
            PageboxType::Bleed => Some(c"pagebox bleedbox "),
            PageboxType::Trim => Some(c"pagebox trimbox "),
            PageboxType::Art => Some(c"pagebox artbox "),
            PageboxType::Unset => None,
        }
    }
}

fn dpx_box_option(pdf_box: PageboxType) -> c_int {
    match pdf_box {
        PageboxType::Media => 2,
        PageboxType::Bleed => 5,
        PageboxType::Trim => 4,
        PageboxType::Art => 3,
        PageboxType::Crop | PageboxType::Unset => 1,
    }
}

unsafe fn finish_pdf_bbox(bbox: &mut PdfRect, matrix: &PdfTMatrix, box_out: &mut RealRect) {
    let mut p1 = PdfCoord {
        x: bbox.llx,
        y: bbox.lly,
    };
    let mut p2 = PdfCoord {
        x: bbox.urx,
        y: bbox.lly,
    };
    let mut p3 = PdfCoord {
        x: bbox.urx,
        y: bbox.ury,
    };
    let mut p4 = PdfCoord {
        x: bbox.llx,
        y: bbox.ury,
    };
    pdf_dev_transform(&mut p1, matrix);
    pdf_dev_transform(&mut p2, matrix);
    pdf_dev_transform(&mut p3, matrix);
    pdf_dev_transform(&mut p4, matrix);

    bbox.llx = min4(p1.x, p2.x, p3.x, p4.x);
    bbox.lly = min4(p1.y, p2.y, p3.y, p4.y);
    bbox.urx = max4(p1.x, p2.x, p3.x, p4.x);
    bbox.ury = max4(p1.y, p2.y, p3.y, p4.y);

    box_out.x = (72.27 / 72.0 * bbox.llx) as f32;
    box_out.y = (72.27 / 72.0 * bbox.lly) as f32;
    box_out.wd = (72.27 / 72.0 * (bbox.urx - bbox.llx)) as f32;
    box_out.ht = (72.27 / 72.0 * (bbox.ury - bbox.lly)) as f32;
}

// translated from xetex-pic.c:99 (pdf_get_rect)
unsafe fn pdf_get_rect(
    filename: *const c_char,
    handle: usize,
    page_num: c_int,
    dpx_options: c_int,
    box_out: &mut RealRect,
) -> c_int {
    let mut bbox = PdfRect {
        llx: 0.0,
        lly: 0.0,
        urx: 0.0,
        ury: 0.0,
    };
    let mut matrix = PdfTMatrix {
        a: 0.0,
        b: 0.0,
        c: 0.0,
        d: 0.0,
        e: 0.0,
        f: 0.0,
    };
    if tt_pic_probe_pdf_box(
        filename,
        handle,
        page_num,
        dpx_options,
        &mut bbox,
        &mut matrix,
    ) != PIC_PROBE_OK
    {
        return -1;
    }
    finish_pdf_bbox(&mut bbox, &matrix, box_out);
    0
}

// translated from xetex-pic.c:191 (get_image_size_in_inches)
unsafe fn get_image_size_in_inches(handle: usize, width: &mut f64, height: &mut f64) -> c_int {
    let mut width_pix = 0;
    let mut height_pix = 0;
    let mut xdensity = 0.0;
    let mut ydensity = 0.0;
    let err = tt_pic_probe_image_box(
        handle,
        &mut width_pix,
        &mut height_pix,
        &mut xdensity,
        &mut ydensity,
    );
    if err != PIC_PROBE_OK {
        *width = -1.0;
        *height = -1.0;
        return err;
    }
    *width = width_pix as f64 * xdensity / 72.0;
    *height = height_pix as f64 * ydensity / 72.0;
    0
}

// translated from xetex-pic.c:224 (find_pic_file)
unsafe fn find_pic_file(
    name_of_file: *const c_char,
    path: &mut *mut c_char,
    bounds: &mut RealRect,
    pdf_box_type: Option<PageboxType>,
    page: c_int,
) -> Result<c_int, MainControlAbort> {
    let handle = ttstub_input_open(name_of_file, TTBC_FILE_FORMAT_PICT, 0);
    bounds.x = 0.0;
    bounds.y = 0.0;
    bounds.wd = 0.0;
    bounds.ht = 0.0;

    if handle == INVALID_HANDLE {
        return Ok(1);
    }

    let err = if let Some(pdf_box) = pdf_box_type {
        let dpx_options = dpx_box_option(pdf_box);
        pdf_get_rect(name_of_file, handle, page, dpx_options, bounds)
    } else {
        let mut wd: f64 = 0.0;
        let mut ht: f64 = 0.0;
        let e = get_image_size_in_inches(handle, &mut wd, &mut ht);
        bounds.wd = (wd * 72.27) as f32;
        bounds.ht = (ht * 72.27) as f32;
        e
    };

    if err == 0 {
        let len = libc::strlen(name_of_file) + 1;
        *path = crate::mem::xmalloc(len) as *mut c_char;
        std::ptr::copy_nonoverlapping(name_of_file, *path, len);
    }
    if crate::io::input_close(handle) {
        // The C ttstub_input_close wrapper longjmp'd on this serious internal
        // close error; propagate it as a fatal abort.
        return Err(MainControlAbort::InputClose);
    }
    Ok(err)
}

// translated from xetex-pic.c:258 (transform_point)
fn transform_point(p: &mut RealPoint, t: &Transform) {
    let rx = t.a * (p.x as f64) + t.c * (p.y as f64) + t.x;
    let ry = t.b * (p.x as f64) + t.d * (p.y as f64) + t.y;
    p.x = rx as f32;
    p.y = ry as f32;
}

// translated from xetex-pic.c:270 (make_identity)
fn make_identity(t: &mut Transform) {
    *t = Transform {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        x: 0.0,
        y: 0.0,
    };
}

// translated from xetex-pic.c:282 (make_scale)
fn make_scale(t: &mut Transform, xscale: f64, yscale: f64) {
    *t = Transform {
        a: xscale,
        b: 0.0,
        c: 0.0,
        d: yscale,
        x: 0.0,
        y: 0.0,
    };
}

// translated from xetex-pic.c:294 (make_translation)
fn make_translation(t: &mut Transform, dx: f64, dy: f64) {
    *t = Transform {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        x: dx,
        y: dy,
    };
}

// translated from xetex-pic.c:306 (make_rotation)
fn make_rotation(t: &mut Transform, a: f64) {
    *t = Transform {
        a: a.cos(),
        b: a.sin(),
        c: -a.sin(),
        d: a.cos(),
        x: 0.0,
        y: 0.0,
    };
}

// translated from xetex-pic.c:318 (transform_concat)
fn transform_concat(t1: &mut Transform, t2: &Transform) {
    let r = Transform {
        a: t1.a * t2.a + t1.b * t2.c,
        b: t1.a * t2.b + t1.b * t2.d,
        c: t1.c * t2.a + t1.d * t2.c,
        d: t1.c * t2.b + t1.d * t2.d,
        x: t1.x * t2.a + t1.y * t2.c + t2.x,
        y: t1.x * t2.b + t1.y * t2.d + t2.y,
    };
    *t1 = r;
}

fn corners_bbox(corners: &[RealPoint; 4]) -> (f64, f64, f64, f64) {
    let xmin = corners.iter().map(|p| p.x).fold(f32::INFINITY, f32::min) as f64;
    let xmax = corners
        .iter()
        .map(|p| p.x)
        .fold(f32::NEG_INFINITY, f32::max) as f64;
    let ymin = corners.iter().map(|p| p.y).fold(f32::INFINITY, f32::min) as f64;
    let ymax = corners
        .iter()
        .map(|p| p.y)
        .fold(f32::NEG_INFINITY, f32::max) as f64;
    (xmin, xmax, ymin, ymax)
}

pub(crate) fn load_picture_in(st: &mut EngineState, is_pdf: bool) -> Result<(), MainControlAbort> {
    crate::io::scan_and_pack_name_in(st)?;

    // `None` = non-PDF image (no page box); `Some(..)` once we know it is a PDF.
    let mut pdf_box_type: Option<PageboxType> = None;
    let mut page: i32 = 0;

    if is_pdf {
        if crate::scan::scan_keyword_in(st, c"page".as_ptr())? {
            crate::scan::scan_int_in(st)?;
            page = st.scanner.cur_val;
        }
        pdf_box_type = Some(if crate::scan::scan_keyword_in(st, c"crop".as_ptr())? {
            PageboxType::Crop
        } else if crate::scan::scan_keyword_in(st, c"media".as_ptr())? {
            PageboxType::Media
        } else if crate::scan::scan_keyword_in(st, c"bleed".as_ptr())? {
            PageboxType::Bleed
        } else if crate::scan::scan_keyword_in(st, c"trim".as_ptr())? {
            PageboxType::Trim
        } else if crate::scan::scan_keyword_in(st, c"art".as_ptr())? {
            PageboxType::Art
        } else {
            PageboxType::Unset
        });
    }

    let mut bounds = RealRect {
        x: 0.0,
        y: 0.0,
        wd: 0.0,
        ht: 0.0,
    };
    let mut pic_path: *mut c_char = std::ptr::null_mut();
    let name_of_file = st.input.name_of_file.as_mut_ptr();
    let result = unsafe {
        // `Unset` (PDF with no explicit keyword) extracts the crop box.
        let effective = match pdf_box_type {
            Some(PageboxType::Unset) => Some(PageboxType::Crop),
            other => other,
        };
        find_pic_file(name_of_file, &mut pic_path, &mut bounds, effective, page)?
    };

    let mut corners: [RealPoint; 4] = [
        RealPoint {
            x: bounds.x,
            y: bounds.y,
        },
        RealPoint {
            x: bounds.x,
            y: bounds.y + bounds.ht,
        },
        RealPoint {
            x: bounds.x + bounds.wd,
            y: bounds.y + bounds.ht,
        },
        RealPoint {
            x: bounds.x + bounds.wd,
            y: bounds.y,
        },
    ];

    let mut x_size_req: f64 = 0.0;
    let mut y_size_req: f64 = 0.0;
    let mut t = Transform {
        a: 0.0,
        b: 0.0,
        c: 0.0,
        d: 0.0,
        x: 0.0,
        y: 0.0,
    };
    make_identity(&mut t);

    let mut check_keywords = true;
    while check_keywords {
        let mut t2 = Transform {
            a: 0.0,
            b: 0.0,
            c: 0.0,
            d: 0.0,
            x: 0.0,
            y: 0.0,
        };
        if crate::scan::scan_keyword_in(st, c"scaled".as_ptr())? {
            crate::scan::scan_int_in(st)?;
            if x_size_req == 0.0 && y_size_req == 0.0 {
                let s = st.scanner.cur_val as f64 / 1000.0;
                make_scale(&mut t2, s, s);
                for c in corners.iter_mut() {
                    transform_point(c, &t2);
                }
                transform_concat(&mut t, &t2);
            }
        } else if crate::scan::scan_keyword_in(st, c"xscaled".as_ptr())? {
            crate::scan::scan_int_in(st)?;
            if x_size_req == 0.0 && y_size_req == 0.0 {
                make_scale(&mut t2, st.scanner.cur_val as f64 / 1000.0, 1.0);
                for c in corners.iter_mut() {
                    transform_point(c, &t2);
                }
                transform_concat(&mut t, &t2);
            }
        } else if crate::scan::scan_keyword_in(st, c"yscaled".as_ptr())? {
            crate::scan::scan_int_in(st)?;
            if x_size_req == 0.0 && y_size_req == 0.0 {
                make_scale(&mut t2, 1.0, st.scanner.cur_val as f64 / 1000.0);
                for c in corners.iter_mut() {
                    transform_point(c, &t2);
                }
                transform_concat(&mut t, &t2);
            }
        } else if crate::scan::scan_keyword_in(st, c"width".as_ptr())? {
            crate::scan::scan_dimen_in(st, false, false, false)?;
            if st.scanner.cur_val <= 0 {
                crate::output::error_here_with_diagnostic_in(st, c"Improper image ".as_ptr());
                crate::output::print_cstr_in(st, c"size (".as_ptr());
                crate::output::print_scaled_in(st, st.scanner.cur_val);
                crate::output::print_cstr_in(st, c"pt) will be ignored".as_ptr());
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                st.errors.help_ptr = 2;
                st.errors.help_line[1] =
                    c"I can't scale images to zero or negative sizes,".as_ptr();
                st.errors.help_line[0] = c"so I'm ignoring this.".as_ptr();
                mc_propagate_error(crate::errors::error_in(st))?;
            } else {
                x_size_req = Fix2D(st.scanner.cur_val);
            }
        } else if crate::scan::scan_keyword_in(st, c"height".as_ptr())? {
            crate::scan::scan_dimen_in(st, false, false, false)?;
            if st.scanner.cur_val <= 0 {
                crate::output::error_here_with_diagnostic_in(st, c"Improper image ".as_ptr());
                crate::output::print_cstr_in(st, c"size (".as_ptr());
                crate::output::print_scaled_in(st, st.scanner.cur_val);
                crate::output::print_cstr_in(st, c"pt) will be ignored".as_ptr());
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                st.errors.help_ptr = 2;
                st.errors.help_line[1] =
                    c"I can't scale images to zero or negative sizes,".as_ptr();
                st.errors.help_line[0] = c"so I'm ignoring this.".as_ptr();
                mc_propagate_error(crate::errors::error_in(st))?;
            } else {
                y_size_req = Fix2D(st.scanner.cur_val);
            }
        } else if crate::scan::scan_keyword_in(st, c"rotated".as_ptr())? {
            crate::scan::xetex_scan_dimen_in(st, false, false, false, false)?;
            if x_size_req != 0.0 || y_size_req != 0.0 {
                let (xmin, xmax, ymin, ymax) = corners_bbox(&corners);
                let (sx, sy) = if x_size_req == 0.0 {
                    let s = y_size_req / (ymax - ymin);
                    (s, s)
                } else if y_size_req == 0.0 {
                    let s = x_size_req / (xmax - xmin);
                    (s, s)
                } else {
                    (x_size_req / (xmax - xmin), y_size_req / (ymax - ymin))
                };
                make_scale(&mut t2, sx, sy);
                for c in corners.iter_mut() {
                    transform_point(c, &t2);
                }
                x_size_req = 0.0;
                y_size_req = 0.0;
                transform_concat(&mut t, &t2);
            }
            make_rotation(
                &mut t2,
                Fix2D(st.scanner.cur_val) * std::f64::consts::PI / 180.0,
            );
            for c in corners.iter_mut() {
                transform_point(c, &t2);
            }
            let (xmin, xmax, ymin, ymax) = corners_bbox(&corners);
            corners = [
                RealPoint {
                    x: xmin as f32,
                    y: ymin as f32,
                },
                RealPoint {
                    x: xmin as f32,
                    y: ymax as f32,
                },
                RealPoint {
                    x: xmax as f32,
                    y: ymax as f32,
                },
                RealPoint {
                    x: xmax as f32,
                    y: ymin as f32,
                },
            ];
            transform_concat(&mut t, &t2);
        } else {
            check_keywords = false;
        }
    }

    if x_size_req != 0.0 || y_size_req != 0.0 {
        let (xmin, xmax, ymin, ymax) = corners_bbox(&corners);
        let (sx, sy) = if x_size_req == 0.0 {
            let s = y_size_req / (ymax - ymin);
            (s, s)
        } else if y_size_req == 0.0 {
            let s = x_size_req / (xmax - xmin);
            (s, s)
        } else {
            (x_size_req / (xmax - xmin), y_size_req / (ymax - ymin))
        };
        let mut t2 = Transform {
            a: 0.0,
            b: 0.0,
            c: 0.0,
            d: 0.0,
            x: 0.0,
            y: 0.0,
        };
        make_scale(&mut t2, sx, sy);
        for c in corners.iter_mut() {
            transform_point(c, &t2);
        }
        x_size_req = 0.0;
        y_size_req = 0.0;
        transform_concat(&mut t, &t2);
    }
    let _ = (x_size_req, y_size_req); // consumed

    let (xmin, xmax, ymin, ymax) = corners_bbox(&corners);
    let mut t2 = Transform {
        a: 0.0,
        b: 0.0,
        c: 0.0,
        d: 0.0,
        x: 0.0,
        y: 0.0,
    };
    make_translation(
        &mut t2,
        -(xmin as i32 as f64) * 72.0 / 72.27,
        -(ymin as i32 as f64) * 72.0 / 72.27,
    );
    transform_concat(&mut t, &t2);

    if result == 0 {
        let path_len = unsafe { libc::strlen(pic_path) };
        let mem_word_bytes = std::mem::size_of::<crate::mem_types::MemoryWord>() as i32;
        let extra_words = (path_len as i32 + mem_word_bytes - 1) / mem_word_bytes;
        crate::mem::new_whatsit(
            &mut st.input.cur_list,
            &mut st.mem,
            crate::node::WhatsitSubtype::Pic as i16,
            (PIC_NODE_SIZE + extra_words) as i16,
        )
        .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;

        let tail = st.input.cur_list.tail.0;
        if is_pdf {
            st.mem
                .set_b16s0(tail, crate::node::WhatsitSubtype::Pdf as u16);
        }
        // PIC_NODE_path_len: mem[p+4].b16.s1
        st.mem.set_b16s1(tail + 4, path_len as u16);
        // page: mem[p+4].b16.s0
        st.mem.set_b16s0(tail + 4, page as u16);
        // pdf_box_type: mem[p+8].b16.s1
        st.mem
            .set_b16s1(tail + 8, pdf_box_type.map_or(0, |pb| pb as u16));
        // width: mem[p+1].b32.s1
        st.mem.set_b32s1(tail + 1, D2Fix(xmax - xmin));
        // height: mem[p+3].b32.s1
        st.mem.set_b32s1(tail + 3, D2Fix(ymax - ymin));
        // depth: mem[p+2].b32.s1
        st.mem.set_b32s1(tail + 2, 0);
        // transform: mem[p+5..7]
        st.mem.set_b32s0(tail + 5, D2Fix(t.a));
        st.mem.set_b32s1(tail + 5, D2Fix(t.b));
        st.mem.set_b32s0(tail + 6, D2Fix(t.c));
        st.mem.set_b32s1(tail + 6, D2Fix(t.d));
        st.mem.set_b32s0(tail + 7, D2Fix(t.x));
        st.mem.set_b32s1(tail + 7, D2Fix(t.y));
        // path bytes: mem[p + PIC_NODE_SIZE]
        st.mem
            .journal_mark_range(tail + PIC_NODE_SIZE, path_len.div_ceil(8));
        let dst = st.mem.node_addr(tail + PIC_NODE_SIZE) as *mut u8;
        unsafe {
            std::ptr::copy_nonoverlapping(pic_path as *const u8, dst, path_len);
            libc::free(pic_path as *mut c_void);
        }
    } else {
        crate::output::error_here_with_diagnostic_in(
            st,
            c"Unable to load picture or PDF file '".as_ptr(),
        );
        crate::output::print_file_name_in(
            st,
            st.scanner.cur_name,
            st.scanner.cur_area,
            st.scanner.cur_ext,
        );
        crate::output::print_in(st, '\'' as i32);
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        if result == -43 {
            st.errors.help_ptr = 2;
            st.errors.help_line[1] = c"The requested image couldn't be read because".as_ptr();
            st.errors.help_line[0] = c"the file was not found.".as_ptr();
        } else {
            st.errors.help_ptr = 2;
            st.errors.help_line[1] = c"The requested image couldn't be read because".as_ptr();
            st.errors.help_line[0] = c"it was not a recognized image format.".as_ptr();
        }
        mc_propagate_error(crate::errors::error_in(st))?;
    }
    Ok(())
}

pub(crate) unsafe fn count_pdf_file_pages_named(
    name: *const libc::c_char,
) -> Result<c_int, crate::scan::SccAbort> {
    let handle = ttstub_input_open(name, TTBC_FILE_FORMAT_PICT, 0);
    if handle == INVALID_HANDLE {
        return Ok(0);
    }
    let mut pages = 0;
    if tt_pic_probe_pdf_page_count(name, handle, &mut pages) != PIC_PROBE_OK {
        pages = 0;
    }
    if crate::io::input_close(handle) {
        // The C ttstub_input_close wrapper longjmp'd on this serious internal
        // close error; propagate it as a fatal abort.
        return Err(crate::scan::SccAbort::InputClose);
    }
    Ok(pages)
}
