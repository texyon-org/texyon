// translated from xetex-pic.c

use libc::{c_char, c_double, c_int, c_uint, c_void};

// --- dpx opaque types ---

#[repr(C)]
pub struct PdfObj {
    _opaque: [u8; 0],
}

#[repr(C)]
pub struct PdfFile {
    _opaque: [u8; 0],
}

// dpx layout types — must match C struct layout exactly (dpx-pdfdev.h)
use crate::ext::D2Fix;
use crate::ext::Fix2D;

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
    // dpx PDF functions
    fn pdf_open(ident: *const c_char, handle: usize) -> *mut PdfFile;
    fn pdf_close(pf: *mut PdfFile);
    fn pdf_doc_get_page_count(pf: *mut PdfFile) -> c_int;
    fn pdf_doc_get_page(
        pf: *mut PdfFile,
        page_no: c_int,
        opt_bbox: c_int,
        bbox: *mut PdfRect,
        matrix: *mut PdfTMatrix,
        resources_p: *mut *mut PdfObj,
    ) -> *mut PdfObj;
    fn pdf_dev_transform(p: *mut PdfCoord, m: *const PdfTMatrix);
    fn pdf_release_obj(object: *mut PdfObj);
    // dpx image probers
    fn check_for_jpeg(handle: usize) -> c_int;
    fn jpeg_get_bbox(
        handle: usize,
        width: *mut c_uint,
        height: *mut c_uint,
        xdensity: *mut c_double,
        ydensity: *mut c_double,
    ) -> c_int;
    fn check_for_bmp(handle: usize) -> c_int;
    fn bmp_get_bbox(
        handle: usize,
        width: *mut c_uint,
        height: *mut c_uint,
        xdensity: *mut c_double,
        ydensity: *mut c_double,
    ) -> c_int;
    fn check_for_png(handle: usize) -> c_int;
    fn png_get_bbox(
        handle: usize,
        width: *mut c_uint,
        height: *mut c_uint,
        xdensity: *mut c_double,
        ydensity: *mut c_double,
    ) -> c_int;
    fn min4(v1: c_double, v2: c_double, v3: c_double, v4: c_double) -> c_double;
    fn max4(v1: c_double, v2: c_double, v3: c_double, v4: c_double) -> c_double;
    // xetex-ext.c conversion helpers (still in C)

    // bridge I/O
    fn ttstub_input_open(path: *const c_char, format: c_int, mag: c_int) -> usize;
    fn ttstub_input_close(handle: usize) -> c_int;
    // C string utilities
    fn xstrdup(s: *const c_char) -> *mut c_char;
}

const INVALID_HANDLE: usize = 0;
const TTBC_FILE_FORMAT_PICT: c_int = 25; // tectonic_bridge_core_generated.h:74

// pdfbox constants from xetex-ext.h:53–58
const PDFBOX_CROP: c_int = 1;
const PDFBOX_MEDIA: c_int = 2;
const PDFBOX_BLEED: c_int = 3;
const PDFBOX_TRIM: c_int = 4;
const PDFBOX_ART: c_int = 5;
const PDFBOX_NONE: c_int = 6;

// node constants from xetex_format.h:466–467 and xetex-constants.h:133
const PIC_NODE: i16 = 43;
const PDF_NODE: u16 = 44;
const PIC_NODE_SIZE: i32 = 9;

// translated from xetex-pic.c:99 (pdf_get_rect)
unsafe fn pdf_get_rect(
    filename: *const c_char,
    handle: usize,
    mut page_num: c_int,
    pdf_box: c_int,
    box_out: &mut RealRect,
) -> c_int {
    let pf = pdf_open(filename, handle);
    if pf.is_null() {
        return -1;
    }

    let pages = pdf_doc_get_page_count(pf);
    if page_num > pages {
        page_num = pages;
    }
    if page_num < 0 {
        page_num += pages + 1;
    }
    if page_num < 1 {
        page_num = 1;
    }

    let dpx_options: c_int = match pdf_box {
        PDFBOX_MEDIA => 2,
        PDFBOX_BLEED => 5,
        PDFBOX_TRIM => 4,
        PDFBOX_ART => 3,
        _ => 1, // PDFBOX_CROP and default
    };

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
    let page = pdf_doc_get_page(
        pf,
        page_num,
        dpx_options,
        &mut bbox,
        &mut matrix,
        std::ptr::null_mut(),
    );
    pdf_close(pf);

    if page.is_null() {
        return -1;
    }
    pdf_release_obj(page);

    // Transform all four corners by the page matrix to account for /Rotate.
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
    pdf_dev_transform(&mut p1, &matrix);
    pdf_dev_transform(&mut p2, &matrix);
    pdf_dev_transform(&mut p3, &matrix);
    pdf_dev_transform(&mut p4, &matrix);

    bbox.llx = min4(p1.x, p2.x, p3.x, p4.x);
    bbox.lly = min4(p1.y, p2.y, p3.y, p4.y);
    bbox.urx = max4(p1.x, p2.x, p3.x, p4.x);
    bbox.ury = max4(p1.y, p2.y, p3.y, p4.y);

    box_out.x = (72.27 / 72.0 * bbox.llx) as f32;
    box_out.y = (72.27 / 72.0 * bbox.lly) as f32;
    box_out.wd = (72.27 / 72.0 * (bbox.urx - bbox.llx)) as f32;
    box_out.ht = (72.27 / 72.0 * (bbox.ury - bbox.lly)) as f32;
    0
}

// translated from xetex-pic.c:191 (get_image_size_in_inches)
unsafe fn get_image_size_in_inches(handle: usize, width: &mut f64, height: &mut f64) -> c_int {
    let mut err: c_int = 1;
    let mut width_pix: c_uint = 0;
    let mut height_pix: c_uint = 0;
    let mut xdensity: f64 = 0.0;
    let mut ydensity: f64 = 0.0;

    if check_for_jpeg(handle) != 0 {
        err = jpeg_get_bbox(
            handle,
            &mut width_pix,
            &mut height_pix,
            &mut xdensity,
            &mut ydensity,
        );
    } else if check_for_bmp(handle) != 0 {
        err = bmp_get_bbox(
            handle,
            &mut width_pix,
            &mut height_pix,
            &mut xdensity,
            &mut ydensity,
        );
    } else if check_for_png(handle) != 0 {
        err = png_get_bbox(
            handle,
            &mut width_pix,
            &mut height_pix,
            &mut xdensity,
            &mut ydensity,
        );
    }

    if err != 0 {
        *width = -1.0;
        *height = -1.0;
        return err;
    }
    // xdvipdfmx: density = 72 / dpi, so width_inches = pixels * density / 72.
    *width = width_pix as f64 * xdensity / 72.0;
    *height = height_pix as f64 * ydensity / 72.0;
    0
}

// translated from xetex-pic.c:224 (find_pic_file)
unsafe fn find_pic_file(
    path: &mut *mut c_char,
    bounds: &mut RealRect,
    pdf_box_type: c_int,
    page: c_int,
) -> c_int {
    let handle = ttstub_input_open(crate::globals::name_of_file(), TTBC_FILE_FORMAT_PICT, 0);
    bounds.x = 0.0;
    bounds.y = 0.0;
    bounds.wd = 0.0;
    bounds.ht = 0.0;

    if handle == INVALID_HANDLE {
        return 1;
    }

    let err = if pdf_box_type != 0 {
        pdf_get_rect(
            crate::globals::name_of_file(),
            handle,
            page,
            pdf_box_type,
            bounds,
        )
    } else {
        let mut wd: f64 = 0.0;
        let mut ht: f64 = 0.0;
        let e = get_image_size_in_inches(handle, &mut wd, &mut ht);
        bounds.wd = (wd * 72.27) as f32;
        bounds.ht = (ht * 72.27) as f32;
        e
    };

    if err == 0 {
        *path = xstrdup(crate::globals::name_of_file());
    }
    ttstub_input_close(handle);
    err
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

// translated from xetex-pic.c:336 (load_picture)
pub unsafe fn load_picture(is_pdf: bool) {
    use crate::errors::error;
    use crate::globals::{
        cur_area, cur_ext, cur_list_tail, cur_name, cur_val, mem_node_addr, set_mem_b16s0,
        set_mem_b16s1, set_mem_b32s0, set_mem_b32s1,
    };
    use crate::mem::new_whatsit_global;
    use crate::output::{print_cstr, print_file_name, print_scaled};
    use crate::scan::{
        pack_file_name, scan_decimal, scan_dimen, scan_file_name, scan_int, scan_keyword,
    };

    scan_file_name();
    pack_file_name(cur_name(), cur_area(), cur_ext());

    let mut pdf_box_type: c_int = 0;
    let mut page: i32 = 0;

    if is_pdf {
        if scan_keyword(c"page".as_ptr()) {
            scan_int();
            page = cur_val();
        }
        pdf_box_type = PDFBOX_NONE;
        if scan_keyword(c"crop".as_ptr()) {
            pdf_box_type = PDFBOX_CROP;
        } else if scan_keyword(c"media".as_ptr()) {
            pdf_box_type = PDFBOX_MEDIA;
        } else if scan_keyword(c"bleed".as_ptr()) {
            pdf_box_type = PDFBOX_BLEED;
        } else if scan_keyword(c"trim".as_ptr()) {
            pdf_box_type = PDFBOX_TRIM;
        } else if scan_keyword(c"art".as_ptr()) {
            pdf_box_type = PDFBOX_ART;
        }
    }

    let mut bounds = RealRect {
        x: 0.0,
        y: 0.0,
        wd: 0.0,
        ht: 0.0,
    };
    let mut pic_path: *mut c_char = std::ptr::null_mut();
    let result = if pdf_box_type == PDFBOX_NONE {
        find_pic_file(&mut pic_path, &mut bounds, PDFBOX_CROP, page)
    } else {
        find_pic_file(&mut pic_path, &mut bounds, pdf_box_type, page)
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
        if scan_keyword(c"scaled".as_ptr()) {
            scan_int();
            if x_size_req == 0.0 && y_size_req == 0.0 {
                let s = cur_val() as f64 / 1000.0;
                make_scale(&mut t2, s, s);
                for c in corners.iter_mut() {
                    transform_point(c, &t2);
                }
                transform_concat(&mut t, &t2);
            }
        } else if scan_keyword(c"xscaled".as_ptr()) {
            scan_int();
            if x_size_req == 0.0 && y_size_req == 0.0 {
                make_scale(&mut t2, cur_val() as f64 / 1000.0, 1.0);
                for c in corners.iter_mut() {
                    transform_point(c, &t2);
                }
                transform_concat(&mut t, &t2);
            }
        } else if scan_keyword(c"yscaled".as_ptr()) {
            scan_int();
            if x_size_req == 0.0 && y_size_req == 0.0 {
                make_scale(&mut t2, 1.0, cur_val() as f64 / 1000.0);
                for c in corners.iter_mut() {
                    transform_point(c, &t2);
                }
                transform_concat(&mut t, &t2);
            }
        } else if scan_keyword(c"width".as_ptr()) {
            scan_dimen(false, false, false);
            if cur_val() <= 0 {
                crate::output::error_here_with_diagnostic(c"Improper image ".as_ptr());
                print_cstr(c"size (".as_ptr());
                print_scaled(cur_val());
                print_cstr(c"pt) will be ignored".as_ptr());
                crate::output::capture_to_diagnostic(std::ptr::null_mut());
                crate::globals::set_help_ptr(2);
                crate::globals::set_help_line(
                    1,
                    c"I can't scale images to zero or negative sizes,".as_ptr(),
                );
                crate::globals::set_help_line(0, c"so I'm ignoring this.".as_ptr());
                error();
            } else {
                x_size_req = Fix2D(cur_val());
            }
        } else if scan_keyword(c"height".as_ptr()) {
            scan_dimen(false, false, false);
            if cur_val() <= 0 {
                crate::output::error_here_with_diagnostic(c"Improper image ".as_ptr());
                print_cstr(c"size (".as_ptr());
                print_scaled(cur_val());
                print_cstr(c"pt) will be ignored".as_ptr());
                crate::output::capture_to_diagnostic(std::ptr::null_mut());
                crate::globals::set_help_ptr(2);
                crate::globals::set_help_line(
                    1,
                    c"I can't scale images to zero or negative sizes,".as_ptr(),
                );
                crate::globals::set_help_line(0, c"so I'm ignoring this.".as_ptr());
                error();
            } else {
                y_size_req = Fix2D(cur_val());
            }
        } else if scan_keyword(c"rotated".as_ptr()) {
            scan_decimal();
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
            make_rotation(&mut t2, Fix2D(cur_val()) * std::f64::consts::PI / 180.0);
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
        let path_len = libc::strlen(pic_path);
        let mem_word_bytes = std::mem::size_of::<crate::mem_types::MemoryWord>() as i32;
        let extra_words = (path_len as i32 + mem_word_bytes - 1) / mem_word_bytes;
        new_whatsit_global(PIC_NODE, (PIC_NODE_SIZE + extra_words) as i16);

        let tail = cur_list_tail();
        if is_pdf {
            set_mem_b16s0(tail, PDF_NODE);
        }
        // PIC_NODE_path_len: mem[p+4].b16.s1
        set_mem_b16s1(tail + 4, path_len as u16);
        // page: mem[p+4].b16.s0
        set_mem_b16s0(tail + 4, page as u16);
        // pdf_box_type: mem[p+8].b16.s1
        set_mem_b16s1(tail + 8, pdf_box_type as u16);
        // width: mem[p+1].b32.s1
        set_mem_b32s1(tail + 1, D2Fix(xmax - xmin));
        // height: mem[p+3].b32.s1
        set_mem_b32s1(tail + 3, D2Fix(ymax - ymin));
        // depth: mem[p+2].b32.s1
        set_mem_b32s1(tail + 2, 0);
        // transform: mem[p+5..7]
        set_mem_b32s0(tail + 5, D2Fix(t.a));
        set_mem_b32s1(tail + 5, D2Fix(t.b));
        set_mem_b32s0(tail + 6, D2Fix(t.c));
        set_mem_b32s1(tail + 6, D2Fix(t.d));
        set_mem_b32s0(tail + 7, D2Fix(t.x));
        set_mem_b32s1(tail + 7, D2Fix(t.y));
        // path bytes: mem[p + PIC_NODE_SIZE]
        let dst = mem_node_addr(tail + PIC_NODE_SIZE) as *mut u8;
        std::ptr::copy_nonoverlapping(pic_path as *const u8, dst, path_len);
        libc::free(pic_path as *mut c_void);
    } else {
        crate::output::error_here_with_diagnostic(c"Unable to load picture or PDF file '".as_ptr());
        print_file_name(cur_name(), cur_area(), cur_ext());
        crate::output::print('\'' as i32);
        crate::output::capture_to_diagnostic(std::ptr::null_mut());
        if result == -43 {
            crate::globals::set_help_ptr(2);
            crate::globals::set_help_line(
                1,
                c"The requested image couldn't be read because".as_ptr(),
            );
            crate::globals::set_help_line(0, c"the file was not found.".as_ptr());
        } else {
            crate::globals::set_help_ptr(2);
            crate::globals::set_help_line(
                1,
                c"The requested image couldn't be read because".as_ptr(),
            );
            crate::globals::set_help_line(0, c"it was not a recognized image format.".as_ptr());
        }
        error();
    }
}

// translated from xetex-pic.c:76 (count_pdf_file_pages)
pub unsafe fn count_pdf_file_pages() -> c_int {
    let handle = ttstub_input_open(crate::globals::name_of_file(), TTBC_FILE_FORMAT_PICT, 0);
    if handle == INVALID_HANDLE {
        return 0;
    }
    let pf = pdf_open(crate::globals::name_of_file(), handle);
    if pf.is_null() {
        ttstub_input_close(handle);
        return 0;
    }
    let pages = pdf_doc_get_page_count(pf);
    pdf_close(pf);
    ttstub_input_close(handle);
    pages
}
