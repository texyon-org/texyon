/* Copyright (c) 2008-2017 jerome DOT laurens AT u-bourgogne DOT fr, MIT License.
 * Translated to Rust from xetex-synctex.c. */

use libc::{c_char, c_int, size_t};
use std::ptr;

use crate::globals::{
    cur_h, cur_input, cur_v, eqtb_b32s1, job_name, mem_b16s1, mem_b32s0, mem_b32s1, rule_dp,
    rule_ht, rule_wd, set_cur_input_synctex_tag, set_eqtb_b32s1, synctex_enabled, total_pages,
};

// INT_BASE (save.rs) and INT_PAR__synctex (xetex_format.h:116).
const INT_BASE: i32 = 7_826_729;
const INT_PAR_SYNCTEX: i32 = 81;

// Node-size constants (xetex-constants.h).
const SYNCTEX_FIELD_SIZE: i32 = 1;
const BOX_NODE_SIZE: i32 = 8;
const RULE_NODE_SIZE: i32 = 5;
const MEDIUM_NODE_SIZE: i32 = 3; // GLUE, KERN, MATH

// Node-type constants matching xetex-synctex.c.
const RULE_NODE: u16 = 2;
const GLUE_NODE: u16 = 10;
const KERN_NODE: u16 = 11;

const SYNCTEX_VERSION: i32 = 1;
const INVALID_HANDLE: usize = 0;

extern "C" {
    fn ttstub_output_open(path: *const c_char, is_gz: c_int) -> usize;
    fn ttstub_output_write(handle: usize, data: *const c_char, len: size_t) -> size_t;
    fn ttstub_output_close(handle: usize) -> c_int;
    fn ttstub_issue_warning(fmt: *const c_char, ...);
    fn ttstub_issue_error(fmt: *const c_char, ...);

    fn xstrdup(s: *const c_char) -> *mut c_char;
    fn xrealloc(ptr: *mut libc::c_void, size: size_t) -> *mut libc::c_void;
    fn gettexstring(s: i32) -> *mut c_char;

    // abspath_of_input_file: #[no_mangle] pub static in io.rs, char[1024].
    static abspath_of_input_file: [c_char; 1024];
}

// ---------------------------------------------------------------------------
// SynctexCtxt
// ---------------------------------------------------------------------------

type Recorder = unsafe fn(i32);

struct SynctexFlags {
    content_ready: bool,
    off: bool,
    not_void: bool,
    warn: bool,
    #[allow(dead_code)]
    output_p: bool,
}

struct SynctexCtxt {
    file: usize,
    root_name: *mut c_char,
    count: i32,
    node: i32,
    recorder: Option<Recorder>,
    tag: i32,
    line: i32,
    curh: i32,
    curv: i32,
    magnification: i32,
    unit: i32,
    total_length: i32,
    lastv: i32,
    form_depth: i32,
    synctex_tag_counter: u32,
    flags: SynctexFlags,
}

static mut SYNCTEX_CTXT: SynctexCtxt = SynctexCtxt {
    file: INVALID_HANDLE,
    root_name: ptr::null_mut(),
    count: 0,
    node: 0,
    recorder: None,
    tag: 0,
    line: 0,
    curh: 0,
    curv: 0,
    magnification: 0,
    unit: 0,
    total_length: 0,
    lastv: -1,
    form_depth: 0,
    synctex_tag_counter: 0,
    flags: SynctexFlags {
        content_ready: false,
        off: false,
        not_void: false,
        warn: false,
        output_p: false,
    },
};

// ---------------------------------------------------------------------------
// Accessors for TeX memory / parameters
// ---------------------------------------------------------------------------

// SYNCTEX_CURH / SYNCTEX_CURV (xetex-synctex.c:19-20).
#[inline]
unsafe fn synctex_curh() -> i32 {
    cur_h() + 4736287
}
#[inline]
unsafe fn synctex_curv() -> i32 {
    cur_v() + 4736287
}

// SYNCTEX_TAG_MODEL / SYNCTEX_LINE_MODEL for BOX (BOX_NODE_SIZE = 8, field = offset 7).
#[inline]
unsafe fn synctex_tag_box(p: i32) -> i32 {
    mem_b32s0(p + BOX_NODE_SIZE - SYNCTEX_FIELD_SIZE)
}
#[inline]
unsafe fn synctex_line_box(p: i32) -> i32 {
    mem_b32s1(p + BOX_NODE_SIZE - SYNCTEX_FIELD_SIZE)
}

// For RULE (RULE_NODE_SIZE = 5, field at offset 4).
#[inline]
unsafe fn synctex_tag_rule(p: i32) -> i32 {
    mem_b32s0(p + RULE_NODE_SIZE - SYNCTEX_FIELD_SIZE)
}
#[inline]
unsafe fn synctex_line_rule(p: i32) -> i32 {
    mem_b32s1(p + RULE_NODE_SIZE - SYNCTEX_FIELD_SIZE)
}

// For GLUE / KERN / MATH (MEDIUM_NODE_SIZE = 3, field at offset 2).
#[inline]
unsafe fn synctex_tag_medium(p: i32) -> i32 {
    mem_b32s0(p + MEDIUM_NODE_SIZE - SYNCTEX_FIELD_SIZE)
}
#[inline]
unsafe fn synctex_line_medium(p: i32) -> i32 {
    mem_b32s1(p + MEDIUM_NODE_SIZE - SYNCTEX_FIELD_SIZE)
}

// SYNCTEX_TYPE: mem[p].b16.s1
#[inline]
unsafe fn synctex_type(p: i32) -> u16 {
    mem_b16s1(p)
}

// SYNCTEX_WIDTH/DEPTH/HEIGHT (width_offset=1, depth_offset=2, height_offset=3).
#[inline]
unsafe fn synctex_width(p: i32) -> i32 {
    mem_b32s1(p + 1)
}
#[inline]
unsafe fn synctex_depth(p: i32) -> i32 {
    mem_b32s1(p + 2)
}
#[inline]
unsafe fn synctex_height(p: i32) -> i32 {
    mem_b32s1(p + 3)
}

// INTPAR(synctex) = eqtb[INT_BASE + INT_PAR__synctex].b32.s1
#[inline]
unsafe fn intpar_synctex() -> i32 {
    eqtb_b32s1(INT_BASE + INT_PAR_SYNCTEX)
}

// ---------------------------------------------------------------------------
// Write helper: format! + ttstub_output_write
// Replaces ttstub_fprintf(handle, "...", ...).
// Returns bytes written (> 0) on success, -1 on failure.
// ---------------------------------------------------------------------------
unsafe fn synctex_write(handle: usize, s: &str) -> i32 {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return 0;
    }
    let written = ttstub_output_write(handle, bytes.as_ptr() as *const c_char, bytes.len());
    if written > 0 {
        written as i32
    } else {
        -1
    }
}

// Free a C pointer and return null (replaces `mfree(ptr)`).
unsafe fn mfree_ptr<T>(ptr: *mut T) -> *mut T {
    libc::free(ptr as *mut libc::c_void);
    ptr::null_mut()
}

// ---------------------------------------------------------------------------
// Private helpers (static linkage in C)
// ---------------------------------------------------------------------------

unsafe fn get_current_name() -> *mut c_char {
    xstrdup(abspath_of_input_file.as_ptr())
}

unsafe fn synctexabort() {
    if SYNCTEX_CTXT.file != INVALID_HANDLE {
        ttstub_output_close(SYNCTEX_CTXT.file);
        SYNCTEX_CTXT.file = INVALID_HANDLE;
    }
    SYNCTEX_CTXT.root_name = mfree_ptr(SYNCTEX_CTXT.root_name);
    SYNCTEX_CTXT.flags.off = true;
}

unsafe fn synctex_record_anchor() -> i32 {
    let total = SYNCTEX_CTXT.total_length;
    let file = SYNCTEX_CTXT.file;
    let s = format!("!{}\n", total);
    let len = synctex_write(file, &s);
    if len > 0 {
        SYNCTEX_CTXT.total_length = len; // XXX: same quirk as original C (assignment, not +=)
        SYNCTEX_CTXT.count += 1;
        return 0;
    }
    synctexabort();
    -1
}

unsafe fn synctex_record_preamble() -> i32 {
    let file = SYNCTEX_CTXT.file;
    let s = format!("SyncTeX Version:{}\n", SYNCTEX_VERSION);
    let len = synctex_write(file, &s);
    if len > 0 {
        SYNCTEX_CTXT.total_length = len; // XXX: assignment, not +=, as in C
        return 0;
    }
    synctexabort();
    -1
}

unsafe fn synctex_record_input(tag: i32, name: *mut c_char) -> i32 {
    let file = SYNCTEX_CTXT.file;
    let name_str = std::ffi::CStr::from_ptr(name).to_string_lossy();
    let s = format!("Input:{}:{}\n", tag, name_str);
    let len = synctex_write(file, &s);
    if len > 0 {
        SYNCTEX_CTXT.total_length += len;
        return 0;
    }
    synctexabort();
    -1
}

unsafe fn synctex_record_settings() -> i32 {
    if SYNCTEX_CTXT.file == INVALID_HANDLE {
        return 0;
    }
    let (file, mag, unit) = (
        SYNCTEX_CTXT.file,
        SYNCTEX_CTXT.magnification,
        SYNCTEX_CTXT.unit,
    );
    let s = format!(
        "Output:pdf\nMagnification:{}\nUnit:{}\nX Offset:0\nY Offset:0\n",
        mag, unit
    );
    let len = synctex_write(file, &s);
    if len > 0 {
        SYNCTEX_CTXT.total_length += len;
        return 0;
    }
    synctexabort();
    -1
}

unsafe fn synctex_record_content() -> i32 {
    let file = SYNCTEX_CTXT.file;
    let len = synctex_write(file, "Content:\n");
    if len > 0 {
        SYNCTEX_CTXT.total_length += len;
        return 0;
    }
    synctexabort();
    -1
}

unsafe fn synctex_record_count() -> i32 {
    let (file, count) = (SYNCTEX_CTXT.file, SYNCTEX_CTXT.count);
    let s = format!("Count:{}\n", count);
    let len = synctex_write(file, &s);
    if len > 0 {
        SYNCTEX_CTXT.total_length += len;
        return 0;
    }
    synctexabort();
    -1
}

unsafe fn synctex_record_postamble() -> i32 {
    if synctex_record_anchor() == 0 {
        let file = SYNCTEX_CTXT.file;
        let len = synctex_write(file, "Postamble:\n");
        if len > 0 {
            SYNCTEX_CTXT.total_length += len;
            if synctex_record_count() == 0 && synctex_record_anchor() == 0 {
                let file2 = SYNCTEX_CTXT.file;
                let len2 = synctex_write(file2, "Post scriptum:\n");
                if len2 > 0 {
                    SYNCTEX_CTXT.total_length += len2;
                    return 0;
                }
            }
        }
    }
    synctexabort();
    -1
}

unsafe fn synctex_record_sheet(sheet: i32) -> i32 {
    if synctex_record_anchor() == 0 {
        let file = SYNCTEX_CTXT.file;
        let s = format!("{{{}\n", sheet);
        let len = synctex_write(file, &s);
        if len > 0 {
            SYNCTEX_CTXT.total_length += len;
            SYNCTEX_CTXT.count += 1;
            return 0;
        }
    }
    synctexabort();
    -1
}

unsafe fn synctex_record_teehs(sheet: i32) -> i32 {
    if synctex_record_anchor() == 0 {
        let file = SYNCTEX_CTXT.file;
        let s = format!("}}{}\n", sheet);
        let len = synctex_write(file, &s);
        if len > 0 {
            SYNCTEX_CTXT.total_length += len;
            SYNCTEX_CTXT.count += 1;
            return 0;
        }
    }
    synctexabort();
    -1
}

unsafe fn synctex_record_pdfxform(_form: i32) -> i32 {
    if SYNCTEX_CTXT.flags.off || intpar_synctex() == 0 || SYNCTEX_CTXT.file == INVALID_HANDLE {
        return 0;
    }
    SYNCTEX_CTXT.form_depth += 1;
    let (file, depth) = (SYNCTEX_CTXT.file, SYNCTEX_CTXT.form_depth);
    let s = format!("<{}\n", depth);
    let len = synctex_write(file, &s);
    if len > 0 {
        SYNCTEX_CTXT.total_length += len;
        SYNCTEX_CTXT.count += 1;
        return 0;
    }
    synctexabort();
    -1
}

unsafe fn synctex_record_mrofxfdp() -> i32 {
    if synctex_record_anchor() == 0 {
        SYNCTEX_CTXT.form_depth -= 1;
        let file = SYNCTEX_CTXT.file;
        let len = synctex_write(file, ">\n");
        if len > 0 {
            SYNCTEX_CTXT.total_length += len;
            SYNCTEX_CTXT.count += 1;
            return 0;
        }
    }
    synctexabort();
    -1
}

unsafe fn synctex_record_node_pdfrefxform(objnum: c_int) -> i32 {
    SYNCTEX_CTXT.curh = synctex_curh();
    SYNCTEX_CTXT.curv = synctex_curv();
    if SYNCTEX_CTXT.flags.off || intpar_synctex() == 0 || SYNCTEX_CTXT.file == INVALID_HANDLE {
        return 0;
    }
    let (file, curh, curv, unit) = (
        SYNCTEX_CTXT.file,
        SYNCTEX_CTXT.curh,
        SYNCTEX_CTXT.curv,
        SYNCTEX_CTXT.unit,
    );
    let s = format!("f{}:{},{}\n", objnum, curh / unit, curv / unit);
    let len = synctex_write(file, &s);
    SYNCTEX_CTXT.lastv = SYNCTEX_CTXT.curv;
    if len > 0 {
        SYNCTEX_CTXT.total_length += len;
        SYNCTEX_CTXT.count += 1;
        return 0;
    }
    synctexabort();
    -1
}

unsafe fn synctex_record_node_vlist(p: i32) {
    SYNCTEX_CTXT.flags.not_void = true;
    let (file, curh, curv, unit) = (
        SYNCTEX_CTXT.file,
        SYNCTEX_CTXT.curh,
        SYNCTEX_CTXT.curv,
        SYNCTEX_CTXT.unit,
    );
    let s = format!(
        "[{},{}:{},{}:{},{},{}\n",
        synctex_tag_box(p),
        synctex_line_box(p),
        curh / unit,
        curv / unit,
        synctex_width(p) / unit,
        synctex_height(p) / unit,
        synctex_depth(p) / unit
    );
    let len = synctex_write(file, &s);
    SYNCTEX_CTXT.lastv = SYNCTEX_CTXT.curv;
    if len > 0 {
        SYNCTEX_CTXT.total_length += len;
        SYNCTEX_CTXT.count += 1;
    } else {
        synctexabort();
    }
}

unsafe fn synctex_record_node_tsilv(_p: i32) {
    let file = SYNCTEX_CTXT.file;
    let len = synctex_write(file, "]\n");
    if len > 0 {
        SYNCTEX_CTXT.total_length += len;
        // count NOT incremented here (matches C comment)
    } else {
        synctexabort();
    }
}

unsafe fn synctex_record_node_void_vlist(p: i32) {
    let (file, curh, curv, unit) = (
        SYNCTEX_CTXT.file,
        SYNCTEX_CTXT.curh,
        SYNCTEX_CTXT.curv,
        SYNCTEX_CTXT.unit,
    );
    let s = format!(
        "v{},{}:{},{}:{},{},{}\n",
        synctex_tag_box(p),
        synctex_line_box(p),
        curh / unit,
        curv / unit,
        synctex_width(p) / unit,
        synctex_height(p) / unit,
        synctex_depth(p) / unit
    );
    let len = synctex_write(file, &s);
    SYNCTEX_CTXT.lastv = SYNCTEX_CTXT.curv;
    if len > 0 {
        SYNCTEX_CTXT.total_length += len;
        SYNCTEX_CTXT.count += 1;
    } else {
        synctexabort();
    }
}

unsafe fn synctex_record_node_hlist(p: i32) {
    SYNCTEX_CTXT.flags.not_void = true;
    let (file, curh, curv, unit) = (
        SYNCTEX_CTXT.file,
        SYNCTEX_CTXT.curh,
        SYNCTEX_CTXT.curv,
        SYNCTEX_CTXT.unit,
    );
    let s = format!(
        "({},{}:{},{}:{},{},{}\n",
        synctex_tag_box(p),
        synctex_line_box(p),
        curh / unit,
        curv / unit,
        synctex_width(p) / unit,
        synctex_height(p) / unit,
        synctex_depth(p) / unit
    );
    let len = synctex_write(file, &s);
    SYNCTEX_CTXT.lastv = SYNCTEX_CTXT.curv;
    if len > 0 {
        SYNCTEX_CTXT.total_length += len;
        SYNCTEX_CTXT.count += 1;
    } else {
        synctexabort();
    }
}

unsafe fn synctex_record_node_tsilh(_p: i32) {
    let file = SYNCTEX_CTXT.file;
    let len = synctex_write(file, ")\n");
    if len > 0 {
        SYNCTEX_CTXT.total_length += len;
        SYNCTEX_CTXT.count += 1;
    } else {
        synctexabort();
    }
}

unsafe fn synctex_record_node_void_hlist(p: i32) {
    let (file, curh, curv, unit) = (
        SYNCTEX_CTXT.file,
        SYNCTEX_CTXT.curh,
        SYNCTEX_CTXT.curv,
        SYNCTEX_CTXT.unit,
    );
    let s = format!(
        "h{},{}:{},{}:{},{},{}\n",
        synctex_tag_box(p),
        synctex_line_box(p),
        curh / unit,
        curv / unit,
        synctex_width(p) / unit,
        synctex_height(p) / unit,
        synctex_depth(p) / unit
    );
    let len = synctex_write(file, &s);
    SYNCTEX_CTXT.lastv = SYNCTEX_CTXT.curv;
    if len > 0 {
        SYNCTEX_CTXT.total_length += len;
        SYNCTEX_CTXT.count += 1;
    } else {
        synctexabort();
    }
}

unsafe fn synctex_record_node_math(p: i32) {
    let (file, curh, curv, unit) = (
        SYNCTEX_CTXT.file,
        SYNCTEX_CTXT.curh,
        SYNCTEX_CTXT.curv,
        SYNCTEX_CTXT.unit,
    );
    let s = format!(
        "${},{}:{},{}\n",
        synctex_tag_medium(p),
        synctex_line_medium(p),
        curh / unit,
        curv / unit
    );
    let len = synctex_write(file, &s);
    SYNCTEX_CTXT.lastv = SYNCTEX_CTXT.curv;
    if len > 0 {
        SYNCTEX_CTXT.total_length += len;
        SYNCTEX_CTXT.count += 1;
    } else {
        synctexabort();
    }
}

unsafe fn synctex_record_node_glue(p: i32) {
    let (file, curh, curv, unit) = (
        SYNCTEX_CTXT.file,
        SYNCTEX_CTXT.curh,
        SYNCTEX_CTXT.curv,
        SYNCTEX_CTXT.unit,
    );
    let s = format!(
        "g{},{}:{},{}\n",
        synctex_tag_medium(p),
        synctex_line_medium(p),
        curh / unit,
        curv / unit
    );
    let len = synctex_write(file, &s);
    SYNCTEX_CTXT.lastv = SYNCTEX_CTXT.curv;
    if len > 0 {
        SYNCTEX_CTXT.total_length += len;
        SYNCTEX_CTXT.count += 1;
    } else {
        synctexabort();
    }
}

// Note: kern record uses synctex_tag_medium / synctex_line_medium (GLUE node size),
// matching `SYNCTEX_TAG_MODEL(p,GLUE)` in the C source.
unsafe fn synctex_record_node_kern(p: i32) {
    let (file, curh, curv, unit) = (
        SYNCTEX_CTXT.file,
        SYNCTEX_CTXT.curh,
        SYNCTEX_CTXT.curv,
        SYNCTEX_CTXT.unit,
    );
    let s = format!(
        "k{},{}:{},{}:{}\n",
        synctex_tag_medium(p),
        synctex_line_medium(p),
        curh / unit,
        curv / unit,
        synctex_width(p) / unit
    );
    let len = synctex_write(file, &s);
    SYNCTEX_CTXT.lastv = SYNCTEX_CTXT.curv;
    if len > 0 {
        SYNCTEX_CTXT.total_length += len;
        SYNCTEX_CTXT.count += 1;
    } else {
        synctexabort();
    }
}

unsafe fn synctex_record_node_rule(p: i32) {
    let (file, curh, curv, unit) = (
        SYNCTEX_CTXT.file,
        SYNCTEX_CTXT.curh,
        SYNCTEX_CTXT.curv,
        SYNCTEX_CTXT.unit,
    );
    let s = format!(
        "r{},{}:{},{}:{},{},{}\n",
        synctex_tag_rule(p),
        synctex_line_rule(p),
        curh / unit,
        curv / unit,
        rule_wd() / unit,
        rule_ht() / unit,
        rule_dp() / unit
    );
    let len = synctex_write(file, &s);
    SYNCTEX_CTXT.lastv = SYNCTEX_CTXT.curv;
    if len > 0 {
        SYNCTEX_CTXT.total_length += len;
        SYNCTEX_CTXT.count += 1;
    } else {
        synctexabort();
    }
}

// Opens the .synctex.gz output file and writes the preamble.
unsafe fn synctex_dot_open() -> usize {
    if SYNCTEX_CTXT.flags.off || intpar_synctex() == 0 {
        return INVALID_HANDLE;
    }
    if SYNCTEX_CTXT.file != INVALID_HANDLE {
        return SYNCTEX_CTXT.file;
    }

    let tmp = gettexstring(job_name());
    let job_len = libc::strlen(tmp);
    if job_len == 0 {
        libc::free(tmp as *mut libc::c_void);
        synctexabort();
        return INVALID_HANDLE;
    }

    // Build "<jobname>.synctex.gz\0"
    let suffix = b".synctex.gz";
    let total = job_len + suffix.len() + 1; // +1 for NUL
    let the_name = libc::malloc(total) as *mut c_char;
    if the_name.is_null() {
        libc::free(tmp as *mut libc::c_void);
        synctexabort();
        return INVALID_HANDLE;
    }
    libc::strcpy(the_name, tmp);
    libc::strcat(the_name, b".synctex.gz\0".as_ptr() as *const c_char);
    libc::free(tmp as *mut libc::c_void);

    SYNCTEX_CTXT.file = ttstub_output_open(the_name, 1);
    libc::free(the_name as *mut libc::c_void);

    if SYNCTEX_CTXT.file == INVALID_HANDLE {
        synctexabort();
        return INVALID_HANDLE;
    }

    if synctex_record_preamble() != 0 {
        synctexabort();
        return INVALID_HANDLE;
    }

    if SYNCTEX_CTXT.magnification == 0 {
        SYNCTEX_CTXT.magnification = 1000;
    }
    SYNCTEX_CTXT.unit = 1;

    if !SYNCTEX_CTXT.root_name.is_null() {
        let rn = SYNCTEX_CTXT.root_name;
        synctex_record_input(1, rn);
        SYNCTEX_CTXT.root_name = mfree_ptr(SYNCTEX_CTXT.root_name);
    }

    SYNCTEX_CTXT.count = 0;
    SYNCTEX_CTXT.file
}

// Ensures the synctex file is open and settings + content header are written.
unsafe fn synctex_prepare_content() -> usize {
    if SYNCTEX_CTXT.flags.content_ready {
        return SYNCTEX_CTXT.file;
    }
    if synctex_dot_open() != INVALID_HANDLE
        && synctex_record_settings() == 0
        && synctex_record_content() == 0
    {
        SYNCTEX_CTXT.flags.content_ready = true;
        return SYNCTEX_CTXT.file;
    }
    synctexabort();
    INVALID_HANDLE
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn synctex_init_command() {
    SYNCTEX_CTXT.file = INVALID_HANDLE;
    SYNCTEX_CTXT.root_name = ptr::null_mut();
    SYNCTEX_CTXT.count = 0;
    SYNCTEX_CTXT.node = 0;
    SYNCTEX_CTXT.recorder = None;
    SYNCTEX_CTXT.tag = 0;
    SYNCTEX_CTXT.line = 0;
    SYNCTEX_CTXT.curh = 0;
    SYNCTEX_CTXT.curv = 0;
    SYNCTEX_CTXT.magnification = 0;
    SYNCTEX_CTXT.unit = 0;
    SYNCTEX_CTXT.total_length = 0;
    SYNCTEX_CTXT.lastv = -1;
    SYNCTEX_CTXT.form_depth = 0;
    SYNCTEX_CTXT.synctex_tag_counter = 0;
    SYNCTEX_CTXT.flags.content_ready = false;
    SYNCTEX_CTXT.flags.off = false;
    SYNCTEX_CTXT.flags.not_void = false;
    SYNCTEX_CTXT.flags.warn = false;
    SYNCTEX_CTXT.flags.output_p = false;

    if synctex_enabled() != 0 {
        set_eqtb_b32s1(INT_BASE + INT_PAR_SYNCTEX, 1);
    } else {
        set_eqtb_b32s1(INT_BASE + INT_PAR_SYNCTEX, 0);
    }
}

#[no_mangle]
pub unsafe extern "C" fn synctex_terminate(_log_opened: bool) {
    if SYNCTEX_CTXT.file != INVALID_HANDLE {
        synctex_record_postamble();
        ttstub_output_close(SYNCTEX_CTXT.file);
        SYNCTEX_CTXT.file = INVALID_HANDLE;
    }
    synctexabort();
}

// synctex_abort declared in xetex-synctex.h but not implemented in original C.
#[no_mangle]
pub unsafe extern "C" fn synctex_abort(_log_opened: bool) {
    synctexabort();
}

#[no_mangle]
pub unsafe extern "C" fn synctex_start_input() {
    if SYNCTEX_CTXT.flags.off {
        return;
    }
    // ~counter > 0 ⟺ counter != u32::MAX (bitwise NOT of max is 0, not > 0)
    if SYNCTEX_CTXT.synctex_tag_counter != u32::MAX {
        SYNCTEX_CTXT.synctex_tag_counter += 1;
    } else {
        SYNCTEX_CTXT.synctex_tag_counter = 0;
        return;
    }
    let tag = SYNCTEX_CTXT.synctex_tag_counter as i32;
    set_cur_input_synctex_tag(tag);

    if SYNCTEX_CTXT.synctex_tag_counter == 1 {
        // First file: store root name for later.
        SYNCTEX_CTXT.root_name = get_current_name();
        if libc::strlen(SYNCTEX_CTXT.root_name) == 0 {
            let texput = b"texput\0";
            SYNCTEX_CTXT.root_name =
                xrealloc(SYNCTEX_CTXT.root_name as *mut libc::c_void, texput.len()) as *mut c_char;
            libc::strcpy(SYNCTEX_CTXT.root_name, texput.as_ptr() as *const c_char);
        }
        return;
    }

    if SYNCTEX_CTXT.file != INVALID_HANDLE || synctex_dot_open() != INVALID_HANDLE {
        let ci = cur_input();
        let tmp = get_current_name();
        synctex_record_input(ci.synctex_tag, tmp);
        libc::free(tmp as *mut libc::c_void);
    }
}

#[no_mangle]
pub unsafe extern "C" fn synctex_sheet(mag: i32) {
    if SYNCTEX_CTXT.flags.off {
        if intpar_synctex() != 0 && !SYNCTEX_CTXT.flags.warn {
            SYNCTEX_CTXT.flags.warn = true;
            ttstub_issue_warning(
                c"SyncTeX was disabled -- changing the value of \\synctex has no effect".as_ptr(),
            );
        }
        return;
    }
    if total_pages() == 0 && mag > 0 {
        SYNCTEX_CTXT.magnification = mag;
    }
    if synctex_prepare_content() != INVALID_HANDLE {
        synctex_record_sheet(total_pages() + 1);
    }
}

#[no_mangle]
pub unsafe extern "C" fn synctex_teehs() {
    if SYNCTEX_CTXT.flags.off || SYNCTEX_CTXT.file == INVALID_HANDLE {
        return;
    }
    synctex_record_teehs(total_pages());
}

#[no_mangle]
pub unsafe extern "C" fn synctex_pdfxform(p: i32) {
    if SYNCTEX_CTXT.flags.off {
        if intpar_synctex() != 0 && !SYNCTEX_CTXT.flags.warn {
            SYNCTEX_CTXT.flags.warn = true;
            ttstub_issue_warning(
                c"SyncTeX was disabled - changing the value of \\synctex has no effect".as_ptr(),
            );
        }
        return;
    }
    if synctex_prepare_content() != INVALID_HANDLE {
        synctex_record_pdfxform(p);
    }
}

#[no_mangle]
pub unsafe extern "C" fn synctex_mrofxfdp() {
    if SYNCTEX_CTXT.file != INVALID_HANDLE {
        synctex_record_mrofxfdp();
    }
}

#[no_mangle]
pub unsafe extern "C" fn synctex_pdfrefxform(objnum: c_int) {
    if SYNCTEX_CTXT.file != INVALID_HANDLE {
        synctex_record_node_pdfrefxform(objnum);
    }
}

#[no_mangle]
pub unsafe extern "C" fn synctex_vlist(this_box: i32) {
    if SYNCTEX_CTXT.flags.off || intpar_synctex() == 0 || SYNCTEX_CTXT.file == INVALID_HANDLE {
        return;
    }
    SYNCTEX_CTXT.node = this_box;
    SYNCTEX_CTXT.recorder = None;
    SYNCTEX_CTXT.tag = synctex_tag_box(this_box);
    SYNCTEX_CTXT.line = synctex_line_box(this_box);
    SYNCTEX_CTXT.curh = synctex_curh();
    SYNCTEX_CTXT.curv = synctex_curv();
    synctex_record_node_vlist(this_box);
}

#[no_mangle]
pub unsafe extern "C" fn synctex_tsilv(this_box: i32) {
    if SYNCTEX_CTXT.flags.off || intpar_synctex() == 0 || SYNCTEX_CTXT.file == INVALID_HANDLE {
        return;
    }
    SYNCTEX_CTXT.node = this_box;
    SYNCTEX_CTXT.tag = synctex_tag_box(this_box);
    SYNCTEX_CTXT.line = synctex_line_box(this_box);
    SYNCTEX_CTXT.curh = synctex_curh();
    SYNCTEX_CTXT.curv = synctex_curv();
    SYNCTEX_CTXT.recorder = None;
    synctex_record_node_tsilv(this_box);
}

#[no_mangle]
pub unsafe extern "C" fn synctex_void_vlist(p: i32, _this_box: i32) {
    if SYNCTEX_CTXT.flags.off || intpar_synctex() == 0 || SYNCTEX_CTXT.file == INVALID_HANDLE {
        return;
    }
    SYNCTEX_CTXT.node = p;
    SYNCTEX_CTXT.tag = synctex_tag_box(p);
    SYNCTEX_CTXT.line = synctex_line_box(p);
    SYNCTEX_CTXT.curh = synctex_curh();
    SYNCTEX_CTXT.curv = synctex_curv();
    SYNCTEX_CTXT.recorder = None;
    synctex_record_node_void_vlist(p);
}

#[no_mangle]
pub unsafe extern "C" fn synctex_hlist(this_box: i32) {
    if SYNCTEX_CTXT.flags.off || intpar_synctex() == 0 || SYNCTEX_CTXT.file == INVALID_HANDLE {
        return;
    }
    SYNCTEX_CTXT.node = this_box;
    SYNCTEX_CTXT.tag = synctex_tag_box(this_box);
    SYNCTEX_CTXT.line = synctex_line_box(this_box);
    SYNCTEX_CTXT.curh = synctex_curh();
    SYNCTEX_CTXT.curv = synctex_curv();
    SYNCTEX_CTXT.recorder = None;
    synctex_record_node_hlist(this_box);
}

#[no_mangle]
pub unsafe extern "C" fn synctex_tsilh(this_box: i32) {
    if SYNCTEX_CTXT.flags.off || intpar_synctex() == 0 || SYNCTEX_CTXT.file == INVALID_HANDLE {
        return;
    }
    SYNCTEX_CTXT.node = this_box;
    SYNCTEX_CTXT.tag = synctex_tag_box(this_box);
    SYNCTEX_CTXT.line = synctex_line_box(this_box);
    SYNCTEX_CTXT.curh = synctex_curh();
    SYNCTEX_CTXT.curv = synctex_curv();
    SYNCTEX_CTXT.recorder = None;
    synctex_record_node_tsilh(this_box);
}

#[no_mangle]
pub unsafe extern "C" fn synctex_void_hlist(p: i32, _this_box: i32) {
    // Uses the second SYNCTEX_IGNORE definition: checks tag/line too.
    if SYNCTEX_CTXT.flags.off
        || intpar_synctex() == 0
        || synctex_tag_box(p) <= 0
        || synctex_line_box(p) <= 0
    {
        return;
    }
    // Flush pending recorder before context change.
    let (rec, node) = (SYNCTEX_CTXT.recorder, SYNCTEX_CTXT.node);
    if let Some(rec) = rec {
        rec(node);
    }
    SYNCTEX_CTXT.node = p;
    SYNCTEX_CTXT.tag = synctex_tag_box(p);
    SYNCTEX_CTXT.line = synctex_line_box(p);
    SYNCTEX_CTXT.curh = synctex_curh();
    SYNCTEX_CTXT.curv = synctex_curv();
    SYNCTEX_CTXT.recorder = None;
    synctex_record_node_void_hlist(p);
}

#[no_mangle]
pub unsafe extern "C" fn synctex_math(p: i32, _this_box: i32) {
    if SYNCTEX_CTXT.flags.off || intpar_synctex() == 0 || SYNCTEX_CTXT.file == INVALID_HANDLE {
        return;
    }
    // SYNCTEX_CONTEXT_DID_CHANGE: node == 0 or tag/line differ.
    let ctx_changed = SYNCTEX_CTXT.node == 0
        || synctex_tag_medium(p) != SYNCTEX_CTXT.tag
        || synctex_line_medium(p) != SYNCTEX_CTXT.line;
    let (rec, node) = (SYNCTEX_CTXT.recorder, SYNCTEX_CTXT.node);
    if let Some(rec) = rec {
        if ctx_changed {
            rec(node);
        }
    }
    SYNCTEX_CTXT.node = p;
    SYNCTEX_CTXT.tag = synctex_tag_medium(p);
    SYNCTEX_CTXT.line = synctex_line_medium(p);
    SYNCTEX_CTXT.curh = synctex_curh();
    SYNCTEX_CTXT.curv = synctex_curv();
    SYNCTEX_CTXT.recorder = None;
    synctex_record_node_math(p);
}

#[no_mangle]
pub unsafe extern "C" fn synctex_horizontal_rule_or_glue(p: i32, _this_box: i32) {
    let node_type = synctex_type(p);
    // Second SYNCTEX_IGNORE definition: also checks tag/line > 0.
    let off = SYNCTEX_CTXT.flags.off || intpar_synctex() == 0;
    let ignore = |tag: i32, line: i32| off || tag <= 0 || line <= 0;

    match node_type {
        RULE_NODE => {
            if ignore(synctex_tag_rule(p), synctex_line_rule(p)) {
                return;
            }
        }
        GLUE_NODE => {
            if ignore(synctex_tag_medium(p), synctex_line_medium(p)) {
                return;
            }
        }
        KERN_NODE => {
            if ignore(synctex_tag_medium(p), synctex_line_medium(p)) {
                return;
            }
        }
        _ => {
            let s = format!("unknown node type {} in SyncTeX\0", node_type);
            ttstub_issue_error(s.as_ptr() as *const c_char);
            return;
        }
    }

    SYNCTEX_CTXT.node = p;
    SYNCTEX_CTXT.curh = synctex_curh();
    SYNCTEX_CTXT.curv = synctex_curv();
    SYNCTEX_CTXT.recorder = None;
    match node_type {
        RULE_NODE => {
            SYNCTEX_CTXT.tag = synctex_tag_rule(p);
            SYNCTEX_CTXT.line = synctex_line_rule(p);
            synctex_record_node_rule(p);
        }
        GLUE_NODE => {
            SYNCTEX_CTXT.tag = synctex_tag_medium(p);
            SYNCTEX_CTXT.line = synctex_line_medium(p);
            synctex_record_node_glue(p);
        }
        KERN_NODE => {
            SYNCTEX_CTXT.tag = synctex_tag_medium(p);
            SYNCTEX_CTXT.line = synctex_line_medium(p);
            synctex_record_node_kern(p);
        }
        _ => {
            // Already handled above — unreachable in practice.
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn synctex_kern(p: i32, this_box: i32) {
    // Second SYNCTEX_IGNORE definition.
    if SYNCTEX_CTXT.flags.off
        || intpar_synctex() == 0
        || synctex_tag_medium(p) <= 0
        || synctex_line_medium(p) <= 0
    {
        return;
    }
    // SYNCTEX_CONTEXT_DID_CHANGE
    let ctx_changed = SYNCTEX_CTXT.node == 0
        || synctex_tag_medium(p) != SYNCTEX_CTXT.tag
        || synctex_line_medium(p) != SYNCTEX_CTXT.line;
    if ctx_changed {
        // Flush pending recorder.
        let (rec, node) = (SYNCTEX_CTXT.recorder, SYNCTEX_CTXT.node);
        if let Some(rec) = rec {
            rec(node);
        }
        SYNCTEX_CTXT.node = p;
        SYNCTEX_CTXT.tag = synctex_tag_medium(p);
        SYNCTEX_CTXT.line = synctex_line_medium(p);
        if SYNCTEX_CTXT.node == this_box {
            // First node in hlist.
            SYNCTEX_CTXT.recorder = Some(synctex_record_node_kern);
        } else {
            SYNCTEX_CTXT.recorder = None;
            synctex_record_node_kern(p);
        }
    } else {
        // Same context — defer recording.
        SYNCTEX_CTXT.node = p;
        SYNCTEX_CTXT.tag = synctex_tag_medium(p);
        SYNCTEX_CTXT.line = synctex_line_medium(p);
        SYNCTEX_CTXT.recorder = Some(synctex_record_node_kern);
    }
}

#[no_mangle]
pub unsafe extern "C" fn synctex_current() {
    if SYNCTEX_CTXT.flags.off || intpar_synctex() == 0 || SYNCTEX_CTXT.file == INVALID_HANDLE {
        return;
    }
    let (file, tag, line, unit) = (
        SYNCTEX_CTXT.file,
        SYNCTEX_CTXT.tag,
        SYNCTEX_CTXT.line,
        SYNCTEX_CTXT.unit,
    );
    let s = format!(
        "x{},{}:{},{}\n",
        tag,
        line,
        synctex_curh() / unit,
        synctex_curv() / unit
    );
    let len = synctex_write(file, &s);
    SYNCTEX_CTXT.lastv = synctex_curv();
    if len > 0 {
        SYNCTEX_CTXT.total_length += len;
    } else {
        synctexabort();
    }
}
