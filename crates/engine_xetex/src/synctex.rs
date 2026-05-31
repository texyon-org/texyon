/* Copyright (c) 2008-2017 jerome DOT laurens AT u-bourgogne DOT fr, MIT License.
 * Translated to Rust from xetex-synctex.c. */

use crate::consts::{
    BOX_NODE_SIZE, GLUE_NODE, INT_BASE, KERN_NODE, MEDIUM_NODE_SIZE, RULE_NODE, RULE_NODE_SIZE,
};
use libc::{c_char, c_int, size_t};
use std::ptr;

use crate::globals::{
    cur_h, cur_input, cur_v, eqtb_b32s1, job_name, mem_b16s1, mem_b32s0, mem_b32s1, rule_dp,
    rule_ht, rule_wd, set_cur_input_synctex_tag, set_eqtb_b32s1, synctex_enabled, total_pages,
};

// INT_BASE (save.rs) and INT_PAR__synctex (xetex_format.h:116).
const INT_PAR_SYNCTEX: i32 = 81;

// Node-size constants (xetex-constants.h).
use crate::texmfmp::gettexstring;

const SYNCTEX_FIELD_SIZE: i32 = 1;

// Node-type constants matching xetex-synctex.c.

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

    // abspath_of_input_file: #[no_mangle] pub static in io.rs, char[1024].
    static abspath_of_input_file: [c_char; 1024];
}

// ---------------------------------------------------------------------------
// SynctexCtxt bridge accessors
// ---------------------------------------------------------------------------
//
// The SyncTeX context (formerly the `SYNCTEX_CTXT` file-static) now lives on
// `EngineState.synctex` (session 46). These thin module-private accessors
// bridge each field through the thread-local; each closure touches one field
// and never holds the borrow across engine calls. The functions below read
// fields into locals before doing I/O and write scalars back afterward, so the
// per-field bridge is a faithful drop-in for the former direct field access.

use crate::state::Recorder;

macro_rules! sc_scalar {
    ($get:ident, $set:ident, $field:ident, $ty:ty) => {
        #[inline]
        fn $get() -> $ty {
            crate::state::with_state(|s| s.synctex.$field)
        }
        #[inline]
        fn $set(v: $ty) {
            crate::state::with_state_mut(|s| s.synctex.$field = v);
        }
    };
}

macro_rules! sc_flag {
    ($get:ident, $set:ident, $field:ident) => {
        #[inline]
        fn $get() -> bool {
            crate::state::with_state(|s| s.synctex.flags.$field)
        }
        #[inline]
        fn $set(v: bool) {
            crate::state::with_state_mut(|s| s.synctex.flags.$field = v);
        }
    };
}

sc_scalar!(sc_file, set_sc_file, file, usize);
sc_scalar!(sc_root_name, set_sc_root_name, root_name, *mut c_char);
sc_scalar!(sc_count, set_sc_count, count, i32);
sc_scalar!(sc_node, set_sc_node, node, i32);
sc_scalar!(sc_recorder, set_sc_recorder, recorder, Option<Recorder>);
sc_scalar!(sc_tag, set_sc_tag, tag, i32);
sc_scalar!(sc_line, set_sc_line, line, i32);
sc_scalar!(sc_curh, set_sc_curh, curh, i32);
sc_scalar!(sc_curv, set_sc_curv, curv, i32);
sc_scalar!(sc_magnification, set_sc_magnification, magnification, i32);
sc_scalar!(sc_unit, set_sc_unit, unit, i32);
sc_scalar!(sc_total_length, set_sc_total_length, total_length, i32);
sc_scalar!(sc_lastv, set_sc_lastv, lastv, i32);
sc_scalar!(sc_form_depth, set_sc_form_depth, form_depth, i32);
sc_scalar!(
    sc_synctex_tag_counter,
    set_sc_synctex_tag_counter,
    synctex_tag_counter,
    u32
);
sc_flag!(
    sc_flags_content_ready,
    set_sc_flags_content_ready,
    content_ready
);
sc_flag!(sc_flags_off, set_sc_flags_off, off);
sc_flag!(sc_flags_not_void, set_sc_flags_not_void, not_void);
sc_flag!(sc_flags_warn, set_sc_flags_warn, warn);
#[allow(dead_code)]
sc_flag!(sc_flags_output_p, set_sc_flags_output_p, output_p);

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
    if sc_file() != INVALID_HANDLE {
        ttstub_output_close(sc_file());
        set_sc_file(INVALID_HANDLE);
    }
    set_sc_root_name(mfree_ptr(sc_root_name()));
    set_sc_flags_off(true);
}

unsafe fn synctex_record_anchor() -> i32 {
    let total = sc_total_length();
    let file = sc_file();
    let s = format!("!{}\n", total);
    let len = synctex_write(file, &s);
    if len > 0 {
        set_sc_total_length(len); // XXX: same quirk as original C (assignment, not +=)
        set_sc_count(sc_count() + 1);
        return 0;
    }
    synctexabort();
    -1
}

unsafe fn synctex_record_preamble() -> i32 {
    let file = sc_file();
    let s = format!("SyncTeX Version:{}\n", SYNCTEX_VERSION);
    let len = synctex_write(file, &s);
    if len > 0 {
        set_sc_total_length(len); // XXX: assignment, not +=, as in C
        return 0;
    }
    synctexabort();
    -1
}

unsafe fn synctex_record_input(tag: i32, name: *mut c_char) -> i32 {
    let file = sc_file();
    let name_str = std::ffi::CStr::from_ptr(name).to_string_lossy();
    let s = format!("Input:{}:{}\n", tag, name_str);
    let len = synctex_write(file, &s);
    if len > 0 {
        set_sc_total_length(sc_total_length() + len);
        return 0;
    }
    synctexabort();
    -1
}

unsafe fn synctex_record_settings() -> i32 {
    if sc_file() == INVALID_HANDLE {
        return 0;
    }
    let (file, mag, unit) = (sc_file(), sc_magnification(), sc_unit());
    let s = format!(
        "Output:pdf\nMagnification:{}\nUnit:{}\nX Offset:0\nY Offset:0\n",
        mag, unit
    );
    let len = synctex_write(file, &s);
    if len > 0 {
        set_sc_total_length(sc_total_length() + len);
        return 0;
    }
    synctexabort();
    -1
}

unsafe fn synctex_record_content() -> i32 {
    let file = sc_file();
    let len = synctex_write(file, "Content:\n");
    if len > 0 {
        set_sc_total_length(sc_total_length() + len);
        return 0;
    }
    synctexabort();
    -1
}

unsafe fn synctex_record_count() -> i32 {
    let (file, count) = (sc_file(), sc_count());
    let s = format!("Count:{}\n", count);
    let len = synctex_write(file, &s);
    if len > 0 {
        set_sc_total_length(sc_total_length() + len);
        return 0;
    }
    synctexabort();
    -1
}

unsafe fn synctex_record_postamble() -> i32 {
    if synctex_record_anchor() == 0 {
        let file = sc_file();
        let len = synctex_write(file, "Postamble:\n");
        if len > 0 {
            set_sc_total_length(sc_total_length() + len);
            if synctex_record_count() == 0 && synctex_record_anchor() == 0 {
                let file2 = sc_file();
                let len2 = synctex_write(file2, "Post scriptum:\n");
                if len2 > 0 {
                    set_sc_total_length(sc_total_length() + len2);
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
        let file = sc_file();
        let s = format!("{{{}\n", sheet);
        let len = synctex_write(file, &s);
        if len > 0 {
            set_sc_total_length(sc_total_length() + len);
            set_sc_count(sc_count() + 1);
            return 0;
        }
    }
    synctexabort();
    -1
}

unsafe fn synctex_record_teehs(sheet: i32) -> i32 {
    if synctex_record_anchor() == 0 {
        let file = sc_file();
        let s = format!("}}{}\n", sheet);
        let len = synctex_write(file, &s);
        if len > 0 {
            set_sc_total_length(sc_total_length() + len);
            set_sc_count(sc_count() + 1);
            return 0;
        }
    }
    synctexabort();
    -1
}

unsafe fn synctex_record_pdfxform(_form: i32) -> i32 {
    if sc_flags_off() || intpar_synctex() == 0 || sc_file() == INVALID_HANDLE {
        return 0;
    }
    set_sc_form_depth(sc_form_depth() + 1);
    let (file, depth) = (sc_file(), sc_form_depth());
    let s = format!("<{}\n", depth);
    let len = synctex_write(file, &s);
    if len > 0 {
        set_sc_total_length(sc_total_length() + len);
        set_sc_count(sc_count() + 1);
        return 0;
    }
    synctexabort();
    -1
}

unsafe fn synctex_record_mrofxfdp() -> i32 {
    if synctex_record_anchor() == 0 {
        set_sc_form_depth(sc_form_depth() - 1);
        let file = sc_file();
        let len = synctex_write(file, ">\n");
        if len > 0 {
            set_sc_total_length(sc_total_length() + len);
            set_sc_count(sc_count() + 1);
            return 0;
        }
    }
    synctexabort();
    -1
}

unsafe fn synctex_record_node_pdfrefxform(objnum: c_int) -> i32 {
    set_sc_curh(synctex_curh());
    set_sc_curv(synctex_curv());
    if sc_flags_off() || intpar_synctex() == 0 || sc_file() == INVALID_HANDLE {
        return 0;
    }
    let (file, curh, curv, unit) = (sc_file(), sc_curh(), sc_curv(), sc_unit());
    let s = format!("f{}:{},{}\n", objnum, curh / unit, curv / unit);
    let len = synctex_write(file, &s);
    set_sc_lastv(sc_curv());
    if len > 0 {
        set_sc_total_length(sc_total_length() + len);
        set_sc_count(sc_count() + 1);
        return 0;
    }
    synctexabort();
    -1
}

unsafe fn synctex_record_node_vlist(p: i32) {
    set_sc_flags_not_void(true);
    let (file, curh, curv, unit) = (sc_file(), sc_curh(), sc_curv(), sc_unit());
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
    set_sc_lastv(sc_curv());
    if len > 0 {
        set_sc_total_length(sc_total_length() + len);
        set_sc_count(sc_count() + 1);
    } else {
        synctexabort();
    }
}

unsafe fn synctex_record_node_tsilv(_p: i32) {
    let file = sc_file();
    let len = synctex_write(file, "]\n");
    if len > 0 {
        set_sc_total_length(sc_total_length() + len);
        // count NOT incremented here (matches C comment)
    } else {
        synctexabort();
    }
}

unsafe fn synctex_record_node_void_vlist(p: i32) {
    let (file, curh, curv, unit) = (sc_file(), sc_curh(), sc_curv(), sc_unit());
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
    set_sc_lastv(sc_curv());
    if len > 0 {
        set_sc_total_length(sc_total_length() + len);
        set_sc_count(sc_count() + 1);
    } else {
        synctexabort();
    }
}

unsafe fn synctex_record_node_hlist(p: i32) {
    set_sc_flags_not_void(true);
    let (file, curh, curv, unit) = (sc_file(), sc_curh(), sc_curv(), sc_unit());
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
    set_sc_lastv(sc_curv());
    if len > 0 {
        set_sc_total_length(sc_total_length() + len);
        set_sc_count(sc_count() + 1);
    } else {
        synctexabort();
    }
}

unsafe fn synctex_record_node_tsilh(_p: i32) {
    let file = sc_file();
    let len = synctex_write(file, ")\n");
    if len > 0 {
        set_sc_total_length(sc_total_length() + len);
        set_sc_count(sc_count() + 1);
    } else {
        synctexabort();
    }
}

unsafe fn synctex_record_node_void_hlist(p: i32) {
    let (file, curh, curv, unit) = (sc_file(), sc_curh(), sc_curv(), sc_unit());
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
    set_sc_lastv(sc_curv());
    if len > 0 {
        set_sc_total_length(sc_total_length() + len);
        set_sc_count(sc_count() + 1);
    } else {
        synctexabort();
    }
}

unsafe fn synctex_record_node_math(p: i32) {
    let (file, curh, curv, unit) = (sc_file(), sc_curh(), sc_curv(), sc_unit());
    let s = format!(
        "${},{}:{},{}\n",
        synctex_tag_medium(p),
        synctex_line_medium(p),
        curh / unit,
        curv / unit
    );
    let len = synctex_write(file, &s);
    set_sc_lastv(sc_curv());
    if len > 0 {
        set_sc_total_length(sc_total_length() + len);
        set_sc_count(sc_count() + 1);
    } else {
        synctexabort();
    }
}

unsafe fn synctex_record_node_glue(p: i32) {
    let (file, curh, curv, unit) = (sc_file(), sc_curh(), sc_curv(), sc_unit());
    let s = format!(
        "g{},{}:{},{}\n",
        synctex_tag_medium(p),
        synctex_line_medium(p),
        curh / unit,
        curv / unit
    );
    let len = synctex_write(file, &s);
    set_sc_lastv(sc_curv());
    if len > 0 {
        set_sc_total_length(sc_total_length() + len);
        set_sc_count(sc_count() + 1);
    } else {
        synctexabort();
    }
}

// Note: kern record uses synctex_tag_medium / synctex_line_medium (GLUE node size),
// matching `SYNCTEX_TAG_MODEL(p,GLUE)` in the C source.
unsafe fn synctex_record_node_kern(p: i32) {
    let (file, curh, curv, unit) = (sc_file(), sc_curh(), sc_curv(), sc_unit());
    let s = format!(
        "k{},{}:{},{}:{}\n",
        synctex_tag_medium(p),
        synctex_line_medium(p),
        curh / unit,
        curv / unit,
        synctex_width(p) / unit
    );
    let len = synctex_write(file, &s);
    set_sc_lastv(sc_curv());
    if len > 0 {
        set_sc_total_length(sc_total_length() + len);
        set_sc_count(sc_count() + 1);
    } else {
        synctexabort();
    }
}

unsafe fn synctex_record_node_rule(p: i32) {
    let (file, curh, curv, unit) = (sc_file(), sc_curh(), sc_curv(), sc_unit());
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
    set_sc_lastv(sc_curv());
    if len > 0 {
        set_sc_total_length(sc_total_length() + len);
        set_sc_count(sc_count() + 1);
    } else {
        synctexabort();
    }
}

// Opens the .synctex.gz output file and writes the preamble.
unsafe fn synctex_dot_open() -> usize {
    if sc_flags_off() || intpar_synctex() == 0 {
        return INVALID_HANDLE;
    }
    if sc_file() != INVALID_HANDLE {
        return sc_file();
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
    libc::strcat(the_name, c".synctex.gz".as_ptr());
    libc::free(tmp as *mut libc::c_void);

    set_sc_file(ttstub_output_open(the_name, 1));
    libc::free(the_name as *mut libc::c_void);

    if sc_file() == INVALID_HANDLE {
        synctexabort();
        return INVALID_HANDLE;
    }

    if synctex_record_preamble() != 0 {
        synctexabort();
        return INVALID_HANDLE;
    }

    if sc_magnification() == 0 {
        set_sc_magnification(1000);
    }
    set_sc_unit(1);

    if !sc_root_name().is_null() {
        let rn = sc_root_name();
        synctex_record_input(1, rn);
        set_sc_root_name(mfree_ptr(sc_root_name()));
    }

    set_sc_count(0);
    sc_file()
}

// Ensures the synctex file is open and settings + content header are written.
unsafe fn synctex_prepare_content() -> usize {
    if sc_flags_content_ready() {
        return sc_file();
    }
    if synctex_dot_open() != INVALID_HANDLE
        && synctex_record_settings() == 0
        && synctex_record_content() == 0
    {
        set_sc_flags_content_ready(true);
        return sc_file();
    }
    synctexabort();
    INVALID_HANDLE
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn synctex_init_command() {
    set_sc_file(INVALID_HANDLE);
    set_sc_root_name(ptr::null_mut());
    set_sc_count(0);
    set_sc_node(0);
    set_sc_recorder(None);
    set_sc_tag(0);
    set_sc_line(0);
    set_sc_curh(0);
    set_sc_curv(0);
    set_sc_magnification(0);
    set_sc_unit(0);
    set_sc_total_length(0);
    set_sc_lastv(-1);
    set_sc_form_depth(0);
    set_sc_synctex_tag_counter(0);
    set_sc_flags_content_ready(false);
    set_sc_flags_off(false);
    set_sc_flags_not_void(false);
    set_sc_flags_warn(false);
    set_sc_flags_output_p(false);

    if synctex_enabled() != 0 {
        set_eqtb_b32s1(INT_BASE + INT_PAR_SYNCTEX, 1);
    } else {
        set_eqtb_b32s1(INT_BASE + INT_PAR_SYNCTEX, 0);
    }
}

pub unsafe fn synctex_terminate(_log_opened: bool) {
    if sc_file() != INVALID_HANDLE {
        synctex_record_postamble();
        ttstub_output_close(sc_file());
        set_sc_file(INVALID_HANDLE);
    }
    synctexabort();
}

// synctex_abort declared in xetex-synctex.h but not implemented in original C.
#[no_mangle]
pub unsafe extern "C" fn synctex_abort(_log_opened: bool) {
    synctexabort();
}

pub unsafe fn synctex_start_input() {
    if sc_flags_off() {
        return;
    }
    // ~counter > 0 ⟺ counter != u32::MAX (bitwise NOT of max is 0, not > 0)
    if sc_synctex_tag_counter() != u32::MAX {
        set_sc_synctex_tag_counter(sc_synctex_tag_counter() + 1);
    } else {
        set_sc_synctex_tag_counter(0);
        return;
    }
    let tag = sc_synctex_tag_counter() as i32;
    set_cur_input_synctex_tag(tag);

    if sc_synctex_tag_counter() == 1 {
        // First file: store root name for later.
        set_sc_root_name(get_current_name());
        if libc::strlen(sc_root_name()) == 0 {
            let texput = b"texput\0";
            set_sc_root_name(
                xrealloc(sc_root_name() as *mut libc::c_void, texput.len()) as *mut c_char
            );
            libc::strcpy(sc_root_name(), texput.as_ptr() as *const c_char);
        }
        return;
    }

    if sc_file() != INVALID_HANDLE || synctex_dot_open() != INVALID_HANDLE {
        let ci = cur_input();
        let tmp = get_current_name();
        synctex_record_input(ci.synctex_tag, tmp);
        libc::free(tmp as *mut libc::c_void);
    }
}

#[no_mangle]
pub unsafe extern "C" fn synctex_sheet(mag: i32) {
    if sc_flags_off() {
        if intpar_synctex() != 0 && !sc_flags_warn() {
            set_sc_flags_warn(true);
            ttstub_issue_warning(
                c"SyncTeX was disabled -- changing the value of \\synctex has no effect".as_ptr(),
            );
        }
        return;
    }
    if total_pages() == 0 && mag > 0 {
        set_sc_magnification(mag);
    }
    if synctex_prepare_content() != INVALID_HANDLE {
        synctex_record_sheet(total_pages() + 1);
    }
}

#[no_mangle]
pub unsafe extern "C" fn synctex_teehs() {
    if sc_flags_off() || sc_file() == INVALID_HANDLE {
        return;
    }
    synctex_record_teehs(total_pages());
}

#[no_mangle]
pub unsafe extern "C" fn synctex_pdfxform(p: i32) {
    if sc_flags_off() {
        if intpar_synctex() != 0 && !sc_flags_warn() {
            set_sc_flags_warn(true);
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
    if sc_file() != INVALID_HANDLE {
        synctex_record_mrofxfdp();
    }
}

#[no_mangle]
pub unsafe extern "C" fn synctex_pdfrefxform(objnum: c_int) {
    if sc_file() != INVALID_HANDLE {
        synctex_record_node_pdfrefxform(objnum);
    }
}

#[no_mangle]
pub unsafe extern "C" fn synctex_vlist(this_box: i32) {
    if sc_flags_off() || intpar_synctex() == 0 || sc_file() == INVALID_HANDLE {
        return;
    }
    set_sc_node(this_box);
    set_sc_recorder(None);
    set_sc_tag(synctex_tag_box(this_box));
    set_sc_line(synctex_line_box(this_box));
    set_sc_curh(synctex_curh());
    set_sc_curv(synctex_curv());
    synctex_record_node_vlist(this_box);
}

#[no_mangle]
pub unsafe extern "C" fn synctex_tsilv(this_box: i32) {
    if sc_flags_off() || intpar_synctex() == 0 || sc_file() == INVALID_HANDLE {
        return;
    }
    set_sc_node(this_box);
    set_sc_tag(synctex_tag_box(this_box));
    set_sc_line(synctex_line_box(this_box));
    set_sc_curh(synctex_curh());
    set_sc_curv(synctex_curv());
    set_sc_recorder(None);
    synctex_record_node_tsilv(this_box);
}

#[no_mangle]
pub unsafe extern "C" fn synctex_void_vlist(p: i32, _this_box: i32) {
    if sc_flags_off() || intpar_synctex() == 0 || sc_file() == INVALID_HANDLE {
        return;
    }
    set_sc_node(p);
    set_sc_tag(synctex_tag_box(p));
    set_sc_line(synctex_line_box(p));
    set_sc_curh(synctex_curh());
    set_sc_curv(synctex_curv());
    set_sc_recorder(None);
    synctex_record_node_void_vlist(p);
}

#[no_mangle]
pub unsafe extern "C" fn synctex_hlist(this_box: i32) {
    if sc_flags_off() || intpar_synctex() == 0 || sc_file() == INVALID_HANDLE {
        return;
    }
    set_sc_node(this_box);
    set_sc_tag(synctex_tag_box(this_box));
    set_sc_line(synctex_line_box(this_box));
    set_sc_curh(synctex_curh());
    set_sc_curv(synctex_curv());
    set_sc_recorder(None);
    synctex_record_node_hlist(this_box);
}

#[no_mangle]
pub unsafe extern "C" fn synctex_tsilh(this_box: i32) {
    if sc_flags_off() || intpar_synctex() == 0 || sc_file() == INVALID_HANDLE {
        return;
    }
    set_sc_node(this_box);
    set_sc_tag(synctex_tag_box(this_box));
    set_sc_line(synctex_line_box(this_box));
    set_sc_curh(synctex_curh());
    set_sc_curv(synctex_curv());
    set_sc_recorder(None);
    synctex_record_node_tsilh(this_box);
}

#[no_mangle]
pub unsafe extern "C" fn synctex_void_hlist(p: i32, _this_box: i32) {
    // Uses the second SYNCTEX_IGNORE definition: checks tag/line too.
    if sc_flags_off()
        || intpar_synctex() == 0
        || synctex_tag_box(p) <= 0
        || synctex_line_box(p) <= 0
    {
        return;
    }
    // Flush pending recorder before context change.
    let (rec, node) = (sc_recorder(), sc_node());
    if let Some(rec) = rec {
        rec(node);
    }
    set_sc_node(p);
    set_sc_tag(synctex_tag_box(p));
    set_sc_line(synctex_line_box(p));
    set_sc_curh(synctex_curh());
    set_sc_curv(synctex_curv());
    set_sc_recorder(None);
    synctex_record_node_void_hlist(p);
}

#[no_mangle]
pub unsafe extern "C" fn synctex_math(p: i32, _this_box: i32) {
    if sc_flags_off() || intpar_synctex() == 0 || sc_file() == INVALID_HANDLE {
        return;
    }
    // SYNCTEX_CONTEXT_DID_CHANGE: node == 0 or tag/line differ.
    let ctx_changed =
        sc_node() == 0 || synctex_tag_medium(p) != sc_tag() || synctex_line_medium(p) != sc_line();
    let (rec, node) = (sc_recorder(), sc_node());
    if let Some(rec) = rec {
        if ctx_changed {
            rec(node);
        }
    }
    set_sc_node(p);
    set_sc_tag(synctex_tag_medium(p));
    set_sc_line(synctex_line_medium(p));
    set_sc_curh(synctex_curh());
    set_sc_curv(synctex_curv());
    set_sc_recorder(None);
    synctex_record_node_math(p);
}

#[no_mangle]
pub unsafe extern "C" fn synctex_horizontal_rule_or_glue(p: i32, _this_box: i32) {
    let node_type = synctex_type(p);
    // Second SYNCTEX_IGNORE definition: also checks tag/line > 0.
    let off = sc_flags_off() || intpar_synctex() == 0;
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

    set_sc_node(p);
    set_sc_curh(synctex_curh());
    set_sc_curv(synctex_curv());
    set_sc_recorder(None);
    match node_type {
        RULE_NODE => {
            set_sc_tag(synctex_tag_rule(p));
            set_sc_line(synctex_line_rule(p));
            synctex_record_node_rule(p);
        }
        GLUE_NODE => {
            set_sc_tag(synctex_tag_medium(p));
            set_sc_line(synctex_line_medium(p));
            synctex_record_node_glue(p);
        }
        KERN_NODE => {
            set_sc_tag(synctex_tag_medium(p));
            set_sc_line(synctex_line_medium(p));
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
    if sc_flags_off()
        || intpar_synctex() == 0
        || synctex_tag_medium(p) <= 0
        || synctex_line_medium(p) <= 0
    {
        return;
    }
    // SYNCTEX_CONTEXT_DID_CHANGE
    let ctx_changed =
        sc_node() == 0 || synctex_tag_medium(p) != sc_tag() || synctex_line_medium(p) != sc_line();
    if ctx_changed {
        // Flush pending recorder.
        let (rec, node) = (sc_recorder(), sc_node());
        if let Some(rec) = rec {
            rec(node);
        }
        set_sc_node(p);
        set_sc_tag(synctex_tag_medium(p));
        set_sc_line(synctex_line_medium(p));
        if sc_node() == this_box {
            // First node in hlist.
            set_sc_recorder(Some(synctex_record_node_kern));
        } else {
            set_sc_recorder(None);
            synctex_record_node_kern(p);
        }
    } else {
        // Same context — defer recording.
        set_sc_node(p);
        set_sc_tag(synctex_tag_medium(p));
        set_sc_line(synctex_line_medium(p));
        set_sc_recorder(Some(synctex_record_node_kern));
    }
}

#[no_mangle]
pub unsafe extern "C" fn synctex_current() {
    if sc_flags_off() || intpar_synctex() == 0 || sc_file() == INVALID_HANDLE {
        return;
    }
    let (file, tag, line, unit) = (sc_file(), sc_tag(), sc_line(), sc_unit());
    let s = format!(
        "x{},{}:{},{}\n",
        tag,
        line,
        synctex_curh() / unit,
        synctex_curv() / unit
    );
    let len = synctex_write(file, &s);
    set_sc_lastv(synctex_curv());
    if len > 0 {
        set_sc_total_length(sc_total_length() + len);
    } else {
        synctexabort();
    }
}
