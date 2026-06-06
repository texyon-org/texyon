/* Copyright (c) 2008-2017 jerome DOT laurens AT u-bourgogne DOT fr, MIT License.
 * Translated to Rust from xetex-synctex.c. */

use crate::consts::{
    BOX_NODE_SIZE, GLUE_NODE, INT_BASE, KERN_NODE, MEDIUM_NODE_SIZE, RULE_NODE, RULE_NODE_SIZE,
};
use libc::{c_char, c_int, size_t};
use std::ptr;

use crate::globals::{cur_input, set_cur_input_synctex_tag};

// INT_BASE (save.rs) and INT_PAR__synctex (xetex_format.h:116).
const INT_PAR_SYNCTEX: i32 = 81;

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

use crate::state::{EqtbState, ShipoutState};

// ---------------------------------------------------------------------------
// Accessors for TeX memory / parameters
// ---------------------------------------------------------------------------

// SYNCTEX_CURH / SYNCTEX_CURV (xetex-synctex.c:19-20).
#[inline]
fn synctex_curh(shipout: &ShipoutState) -> i32 {
    shipout.cur_h + 4736287
}
#[inline]
fn synctex_curv(shipout: &ShipoutState) -> i32 {
    shipout.cur_v + 4736287
}

// Phase-C: the synctex node-field readers below are pure `mem`-arena leaves.
// Each is threaded onto a narrow `&MemState` borrow (direct, safe slice
// indexing). Public recording entry points borrow the thread-local once at the
// boundary and pass disjoint state fields through.

// SYNCTEX_TAG_MODEL / SYNCTEX_LINE_MODEL for BOX (BOX_NODE_SIZE = 8, field = offset 7).
#[inline]
fn synctex_tag_box(mem: &crate::state::MemState, p: i32) -> i32 {
    mem.b32s0(p + BOX_NODE_SIZE - SYNCTEX_FIELD_SIZE)
}
#[inline]
fn synctex_line_box(mem: &crate::state::MemState, p: i32) -> i32 {
    mem.b32s1(p + BOX_NODE_SIZE - SYNCTEX_FIELD_SIZE)
}

// For RULE (RULE_NODE_SIZE = 5, field at offset 4).
#[inline]
fn synctex_tag_rule(mem: &crate::state::MemState, p: i32) -> i32 {
    mem.b32s0(p + RULE_NODE_SIZE - SYNCTEX_FIELD_SIZE)
}
#[inline]
fn synctex_line_rule(mem: &crate::state::MemState, p: i32) -> i32 {
    mem.b32s1(p + RULE_NODE_SIZE - SYNCTEX_FIELD_SIZE)
}

// For GLUE / KERN / MATH (MEDIUM_NODE_SIZE = 3, field at offset 2).
#[inline]
fn synctex_tag_medium(mem: &crate::state::MemState, p: i32) -> i32 {
    mem.b32s0(p + MEDIUM_NODE_SIZE - SYNCTEX_FIELD_SIZE)
}
#[inline]
fn synctex_line_medium(mem: &crate::state::MemState, p: i32) -> i32 {
    mem.b32s1(p + MEDIUM_NODE_SIZE - SYNCTEX_FIELD_SIZE)
}

// SYNCTEX_TYPE: mem[p].b16.s1
#[inline]
fn synctex_type(mem: &crate::state::MemState, p: i32) -> u16 {
    mem.b16s1(p)
}

// SYNCTEX_WIDTH/DEPTH/HEIGHT (width_offset=1, depth_offset=2, height_offset=3).
#[inline]
fn synctex_width(mem: &crate::state::MemState, p: i32) -> i32 {
    mem.b32s1(p + 1)
}
#[inline]
fn synctex_depth(mem: &crate::state::MemState, p: i32) -> i32 {
    mem.b32s1(p + 2)
}
#[inline]
fn synctex_height(mem: &crate::state::MemState, p: i32) -> i32 {
    mem.b32s1(p + 3)
}

// INTPAR(synctex) = eqtb[INT_BASE + INT_PAR__synctex].b32.s1
#[inline]
fn intpar_synctex(eqtb: &EqtbState) -> i32 {
    eqtb.b32s1(INT_BASE + INT_PAR_SYNCTEX)
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

unsafe fn synctexabort(synctex: &mut crate::state::SynctexCtxt) {
    if synctex.file != INVALID_HANDLE {
        ttstub_output_close(synctex.file);
        synctex.file = INVALID_HANDLE;
    }
    synctex.root_name = mfree_ptr(synctex.root_name);
    synctex.flags.off = true;
}

unsafe fn synctex_record_anchor(synctex: &mut crate::state::SynctexCtxt) -> i32 {
    let total = synctex.total_length;
    let file = synctex.file;
    let s = format!("!{}\n", total);
    let len = synctex_write(file, &s);
    if len > 0 {
        synctex.total_length = len; // XXX: same quirk as original C (assignment, not +=)
        synctex.count += 1;
        return 0;
    }
    synctexabort(synctex);
    -1
}

unsafe fn synctex_record_preamble(synctex: &mut crate::state::SynctexCtxt) -> i32 {
    let file = synctex.file;
    let s = format!("SyncTeX Version:{}\n", SYNCTEX_VERSION);
    let len = synctex_write(file, &s);
    if len > 0 {
        synctex.total_length = len; // XXX: assignment, not +=, as in C
        return 0;
    }
    synctexabort(synctex);
    -1
}

unsafe fn synctex_record_input(
    synctex: &mut crate::state::SynctexCtxt,
    tag: i32,
    name: *mut c_char,
) -> i32 {
    let file = synctex.file;
    let name_str = std::ffi::CStr::from_ptr(name).to_string_lossy();
    let s = format!("Input:{}:{}\n", tag, name_str);
    let len = synctex_write(file, &s);
    if len > 0 {
        synctex.total_length += len;
        return 0;
    }
    synctexabort(synctex);
    -1
}

unsafe fn synctex_record_settings(synctex: &mut crate::state::SynctexCtxt) -> i32 {
    if synctex.file == INVALID_HANDLE {
        return 0;
    }
    let (file, mag, unit) = (synctex.file, synctex.magnification, synctex.unit);
    let s = format!(
        "Output:pdf\nMagnification:{}\nUnit:{}\nX Offset:0\nY Offset:0\n",
        mag, unit
    );
    let len = synctex_write(file, &s);
    if len > 0 {
        synctex.total_length += len;
        return 0;
    }
    synctexabort(synctex);
    -1
}

unsafe fn synctex_record_content(synctex: &mut crate::state::SynctexCtxt) -> i32 {
    let file = synctex.file;
    let len = synctex_write(file, "Content:\n");
    if len > 0 {
        synctex.total_length += len;
        return 0;
    }
    synctexabort(synctex);
    -1
}

unsafe fn synctex_record_count(synctex: &mut crate::state::SynctexCtxt) -> i32 {
    let (file, count) = (synctex.file, synctex.count);
    let s = format!("Count:{}\n", count);
    let len = synctex_write(file, &s);
    if len > 0 {
        synctex.total_length += len;
        return 0;
    }
    synctexabort(synctex);
    -1
}

unsafe fn synctex_record_postamble(synctex: &mut crate::state::SynctexCtxt) -> i32 {
    if synctex_record_anchor(synctex) == 0 {
        let file = synctex.file;
        let len = synctex_write(file, "Postamble:\n");
        if len > 0 {
            synctex.total_length += len;
            if synctex_record_count(synctex) == 0 && synctex_record_anchor(synctex) == 0 {
                let file2 = synctex.file;
                let len2 = synctex_write(file2, "Post scriptum:\n");
                if len2 > 0 {
                    synctex.total_length += len2;
                    return 0;
                }
            }
        }
    }
    synctexabort(synctex);
    -1
}

unsafe fn synctex_record_sheet(synctex: &mut crate::state::SynctexCtxt, sheet: i32) -> i32 {
    if synctex_record_anchor(synctex) == 0 {
        let file = synctex.file;
        let s = format!("{{{}\n", sheet);
        let len = synctex_write(file, &s);
        if len > 0 {
            synctex.total_length += len;
            synctex.count += 1;
            return 0;
        }
    }
    synctexabort(synctex);
    -1
}

unsafe fn synctex_record_teehs(synctex: &mut crate::state::SynctexCtxt, sheet: i32) -> i32 {
    if synctex_record_anchor(synctex) == 0 {
        let file = synctex.file;
        let s = format!("}}{}\n", sheet);
        let len = synctex_write(file, &s);
        if len > 0 {
            synctex.total_length += len;
            synctex.count += 1;
            return 0;
        }
    }
    synctexabort(synctex);
    -1
}

unsafe fn synctex_record_node_vlist(
    synctex: &mut crate::state::SynctexCtxt,
    mem: &crate::state::MemState,
    p: i32,
) {
    synctex.flags.not_void = true;
    let (file, curh, curv, unit) = (synctex.file, synctex.curh, synctex.curv, synctex.unit);
    let s = format!(
        "[{},{}:{},{}:{},{},{}\n",
        synctex_tag_box(mem, p),
        synctex_line_box(mem, p),
        curh / unit,
        curv / unit,
        synctex_width(mem, p) / unit,
        synctex_height(mem, p) / unit,
        synctex_depth(mem, p) / unit
    );
    let len = synctex_write(file, &s);
    synctex.lastv = synctex.curv;
    if len > 0 {
        synctex.total_length += len;
        synctex.count += 1;
    } else {
        synctexabort(synctex);
    }
}

unsafe fn synctex_record_node_tsilv(synctex: &mut crate::state::SynctexCtxt, _p: i32) {
    let file = synctex.file;
    let len = synctex_write(file, "]\n");
    if len > 0 {
        synctex.total_length += len;
        // count NOT incremented here (matches C comment)
    } else {
        synctexabort(synctex);
    }
}

unsafe fn synctex_record_node_void_vlist(
    synctex: &mut crate::state::SynctexCtxt,
    mem: &crate::state::MemState,
    p: i32,
) {
    let (file, curh, curv, unit) = (synctex.file, synctex.curh, synctex.curv, synctex.unit);
    let s = format!(
        "v{},{}:{},{}:{},{},{}\n",
        synctex_tag_box(mem, p),
        synctex_line_box(mem, p),
        curh / unit,
        curv / unit,
        synctex_width(mem, p) / unit,
        synctex_height(mem, p) / unit,
        synctex_depth(mem, p) / unit
    );
    let len = synctex_write(file, &s);
    synctex.lastv = synctex.curv;
    if len > 0 {
        synctex.total_length += len;
        synctex.count += 1;
    } else {
        synctexabort(synctex);
    }
}

unsafe fn synctex_record_node_hlist(
    synctex: &mut crate::state::SynctexCtxt,
    mem: &crate::state::MemState,
    p: i32,
) {
    synctex.flags.not_void = true;
    let (file, curh, curv, unit) = (synctex.file, synctex.curh, synctex.curv, synctex.unit);
    let s = format!(
        "({},{}:{},{}:{},{},{}\n",
        synctex_tag_box(mem, p),
        synctex_line_box(mem, p),
        curh / unit,
        curv / unit,
        synctex_width(mem, p) / unit,
        synctex_height(mem, p) / unit,
        synctex_depth(mem, p) / unit
    );
    let len = synctex_write(file, &s);
    synctex.lastv = synctex.curv;
    if len > 0 {
        synctex.total_length += len;
        synctex.count += 1;
    } else {
        synctexabort(synctex);
    }
}

unsafe fn synctex_record_node_tsilh(synctex: &mut crate::state::SynctexCtxt, _p: i32) {
    let file = synctex.file;
    let len = synctex_write(file, ")\n");
    if len > 0 {
        synctex.total_length += len;
        synctex.count += 1;
    } else {
        synctexabort(synctex);
    }
}

unsafe fn synctex_record_node_void_hlist(
    synctex: &mut crate::state::SynctexCtxt,
    mem: &crate::state::MemState,
    p: i32,
) {
    let (file, curh, curv, unit) = (synctex.file, synctex.curh, synctex.curv, synctex.unit);
    let s = format!(
        "h{},{}:{},{}:{},{},{}\n",
        synctex_tag_box(mem, p),
        synctex_line_box(mem, p),
        curh / unit,
        curv / unit,
        synctex_width(mem, p) / unit,
        synctex_height(mem, p) / unit,
        synctex_depth(mem, p) / unit
    );
    let len = synctex_write(file, &s);
    synctex.lastv = synctex.curv;
    if len > 0 {
        synctex.total_length += len;
        synctex.count += 1;
    } else {
        synctexabort(synctex);
    }
}

unsafe fn synctex_record_node_math(
    synctex: &mut crate::state::SynctexCtxt,
    mem: &crate::state::MemState,
    p: i32,
) {
    let (file, curh, curv, unit) = (synctex.file, synctex.curh, synctex.curv, synctex.unit);
    let s = format!(
        "${},{}:{},{}\n",
        synctex_tag_medium(mem, p),
        synctex_line_medium(mem, p),
        curh / unit,
        curv / unit
    );
    let len = synctex_write(file, &s);
    synctex.lastv = synctex.curv;
    if len > 0 {
        synctex.total_length += len;
        synctex.count += 1;
    } else {
        synctexabort(synctex);
    }
}

unsafe fn synctex_record_node_glue(
    synctex: &mut crate::state::SynctexCtxt,
    mem: &crate::state::MemState,
    p: i32,
) {
    let (file, curh, curv, unit) = (synctex.file, synctex.curh, synctex.curv, synctex.unit);
    let s = format!(
        "g{},{}:{},{}\n",
        synctex_tag_medium(mem, p),
        synctex_line_medium(mem, p),
        curh / unit,
        curv / unit
    );
    let len = synctex_write(file, &s);
    synctex.lastv = synctex.curv;
    if len > 0 {
        synctex.total_length += len;
        synctex.count += 1;
    } else {
        synctexabort(synctex);
    }
}

unsafe fn synctex_record_node_kern(
    synctex: &mut crate::state::SynctexCtxt,
    mem: &crate::state::MemState,
    p: i32,
) {
    let (file, curh, curv, unit) = (synctex.file, synctex.curh, synctex.curv, synctex.unit);
    let s = format!(
        "k{},{}:{},{}:{}\n",
        synctex_tag_medium(mem, p),
        synctex_line_medium(mem, p),
        curh / unit,
        curv / unit,
        synctex_width(mem, p) / unit
    );
    let len = synctex_write(file, &s);
    synctex.lastv = synctex.curv;
    if len > 0 {
        synctex.total_length += len;
        synctex.count += 1;
    } else {
        synctexabort(synctex);
    }
}

unsafe fn synctex_record_node_rule(
    synctex: &mut crate::state::SynctexCtxt,
    mem: &crate::state::MemState,
    shipout: &crate::state::ShipoutState,
    p: i32,
) {
    let (file, curh, curv, unit) = (synctex.file, synctex.curh, synctex.curv, synctex.unit);
    let s = format!(
        "r{},{}:{},{}:{},{},{}\n",
        synctex_tag_rule(mem, p),
        synctex_line_rule(mem, p),
        curh / unit,
        curv / unit,
        shipout.rule_wd / unit,
        shipout.rule_ht / unit,
        shipout.rule_dp / unit
    );
    let len = synctex_write(file, &s);
    synctex.lastv = synctex.curv;
    if len > 0 {
        synctex.total_length += len;
        synctex.count += 1;
    } else {
        synctexabort(synctex);
    }
}

unsafe fn synctex_dot_open(
    synctex: &mut crate::state::SynctexCtxt,
    eqtb: &crate::state::EqtbState,
    tmp: *mut c_char,
) -> usize {
    if synctex.flags.off || intpar_synctex(eqtb) == 0 {
        libc::free(tmp as *mut libc::c_void);
        return INVALID_HANDLE;
    }
    if synctex.file != INVALID_HANDLE {
        libc::free(tmp as *mut libc::c_void);
        return synctex.file;
    }

    let job_len = libc::strlen(tmp);
    if job_len == 0 {
        libc::free(tmp as *mut libc::c_void);
        synctexabort(synctex);
        return INVALID_HANDLE;
    }

    // Build "<jobname>.synctex.gz\0"
    let suffix = b".synctex.gz";
    let total = job_len + suffix.len() + 1; // +1 for NUL
    let the_name = libc::malloc(total) as *mut c_char;
    if the_name.is_null() {
        libc::free(tmp as *mut libc::c_void);
        synctexabort(synctex);
        return INVALID_HANDLE;
    }
    libc::strcpy(the_name, tmp);
    libc::strcat(the_name, c".synctex.gz".as_ptr());
    libc::free(tmp as *mut libc::c_void);

    synctex.file = ttstub_output_open(the_name, 1);
    libc::free(the_name as *mut libc::c_void);

    if synctex.file == INVALID_HANDLE {
        synctexabort(synctex);
        return INVALID_HANDLE;
    }

    if synctex_record_preamble(synctex) != 0 {
        synctexabort(synctex);
        return INVALID_HANDLE;
    }

    if synctex.magnification == 0 {
        synctex.magnification = 1000;
    }
    synctex.unit = 1;

    if !synctex.root_name.is_null() {
        let rn = synctex.root_name;
        synctex_record_input(synctex, 1, rn);
        synctex.root_name = mfree_ptr(synctex.root_name);
    }

    synctex.count = 0;
    synctex.file
}

unsafe fn synctex_prepare_content(
    synctex: &mut crate::state::SynctexCtxt,
    eqtb: &crate::state::EqtbState,
    tmp: *mut c_char,
) -> usize {
    if synctex.flags.content_ready {
        libc::free(tmp as *mut libc::c_void);
        return synctex.file;
    }
    if synctex_dot_open(synctex, eqtb, tmp) != INVALID_HANDLE
        && synctex_record_settings(synctex) == 0
        && synctex_record_content(synctex) == 0
    {
        synctex.flags.content_ready = true;
        return synctex.file;
    }
    synctexabort(synctex);
    INVALID_HANDLE
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn synctex_init_command(st: &mut crate::state::EngineState) {
    let synctex = &mut st.synctex;
    synctex.file = INVALID_HANDLE;
    synctex.root_name = ptr::null_mut();
    synctex.count = 0;
    synctex.node = 0;
    synctex.recorder = None;
    synctex.tag = 0;
    synctex.line = 0;
    synctex.curh = 0;
    synctex.curv = 0;
    synctex.magnification = 0;
    synctex.unit = 0;
    synctex.total_length = 0;
    synctex.lastv = -1;
    synctex.form_depth = 0;
    synctex.synctex_tag_counter = 0;
    synctex.flags.content_ready = false;
    synctex.flags.off = false;
    synctex.flags.not_void = false;
    synctex.flags.warn = false;
    synctex.flags.output_p = false;

    let synctex_enabled = st.config.synctex_enabled != 0;
    st.eqtb
        .set_b32s1(INT_BASE + INT_PAR_SYNCTEX, i32::from(synctex_enabled));
}

pub unsafe fn synctex_terminate(st: &mut crate::state::EngineState, _log_opened: bool) {
    let synctex = &mut st.synctex;
    if synctex.file != INVALID_HANDLE {
        synctex_record_postamble(synctex);
        ttstub_output_close(synctex.file);
        synctex.file = INVALID_HANDLE;
    }
    synctexabort(synctex);
}

pub unsafe fn synctex_start_input(st: &mut crate::state::EngineState, tmp: *mut c_char) {
    let synctex = &mut st.synctex;
    if synctex.flags.off {
        libc::free(tmp as *mut libc::c_void);
        return;
    }
    if synctex.synctex_tag_counter != u32::MAX {
        synctex.synctex_tag_counter += 1;
    } else {
        synctex.synctex_tag_counter = 0;
        libc::free(tmp as *mut libc::c_void);
        return;
    }
    let tag = synctex.synctex_tag_counter as i32;
    set_cur_input_synctex_tag(tag);

    if synctex.synctex_tag_counter == 1 {
        synctex.root_name = get_current_name();
        libc::free(tmp as *mut libc::c_void);
        if libc::strlen(synctex.root_name) == 0 {
            let texput = b"texput\0";
            synctex.root_name =
                xrealloc(synctex.root_name as *mut libc::c_void, texput.len()) as *mut c_char;
            libc::strcpy(synctex.root_name, texput.as_ptr() as *const c_char);
        }
        return;
    }

    if synctex.file != INVALID_HANDLE || synctex_dot_open(synctex, &st.eqtb, tmp) != INVALID_HANDLE
    {
        let ci = cur_input();
        let tmp_name = get_current_name();
        synctex_record_input(synctex, ci.synctex_tag, tmp_name);
        libc::free(tmp_name as *mut libc::c_void);
    }
}

pub unsafe fn synctex_sheet(st: &mut crate::state::EngineState, mag: i32, tmp: *mut c_char) {
    let (is_off, total_pages) = (st.synctex.flags.off, st.shipout.total_pages);

    if is_off {
        libc::free(tmp as *mut libc::c_void);
        if intpar_synctex(&st.eqtb) != 0 && !st.synctex.flags.warn {
            st.synctex.flags.warn = true;
            ttstub_issue_warning(
                c"SyncTeX was disabled -- changing the value of \\synctex has no effect".as_ptr(),
            );
        }
        return;
    }

    let synctex = &mut st.synctex;
    if total_pages == 0 && mag > 0 {
        synctex.magnification = mag;
    }
    if synctex_prepare_content(synctex, &st.eqtb, tmp) != INVALID_HANDLE {
        synctex_record_sheet(synctex, total_pages + 1);
    }
}

pub unsafe fn synctex_teehs(st: &mut crate::state::EngineState) {
    let synctex = &mut st.synctex;
    if synctex.flags.off || synctex.file == INVALID_HANDLE {
        return;
    }
    synctex_record_teehs(synctex, st.shipout.total_pages);
}

pub unsafe fn synctex_vlist(st: &mut crate::state::EngineState, this_box: i32) {
    let synctex = &mut st.synctex;
    if synctex.flags.off || intpar_synctex(&st.eqtb) == 0 || synctex.file == INVALID_HANDLE {
        return;
    }
    synctex.node = this_box;
    synctex.recorder = None;
    synctex.tag = synctex_tag_box(&st.mem, this_box);
    synctex.line = synctex_line_box(&st.mem, this_box);
    synctex.curh = synctex_curh(&st.shipout);
    synctex.curv = synctex_curv(&st.shipout);
    synctex_record_node_vlist(synctex, &st.mem, this_box);
}

pub unsafe fn synctex_tsilv(st: &mut crate::state::EngineState, this_box: i32) {
    let synctex = &mut st.synctex;
    if synctex.flags.off || intpar_synctex(&st.eqtb) == 0 || synctex.file == INVALID_HANDLE {
        return;
    }
    synctex.node = this_box;
    synctex.tag = synctex_tag_box(&st.mem, this_box);
    synctex.line = synctex_line_box(&st.mem, this_box);
    synctex.curh = synctex_curh(&st.shipout);
    synctex.curv = synctex_curv(&st.shipout);
    synctex.recorder = None;
    synctex_record_node_tsilv(synctex, this_box);
}

pub unsafe fn synctex_void_vlist(st: &mut crate::state::EngineState, p: i32, _this_box: i32) {
    let synctex = &mut st.synctex;
    if synctex.flags.off || intpar_synctex(&st.eqtb) == 0 || synctex.file == INVALID_HANDLE {
        return;
    }
    synctex.node = p;
    synctex.tag = synctex_tag_box(&st.mem, p);
    synctex.line = synctex_line_box(&st.mem, p);
    synctex.curh = synctex_curh(&st.shipout);
    synctex.curv = synctex_curv(&st.shipout);
    synctex.recorder = None;
    synctex_record_node_void_vlist(synctex, &st.mem, p);
}

pub unsafe fn synctex_hlist(st: &mut crate::state::EngineState, this_box: i32) {
    let synctex = &mut st.synctex;
    if synctex.flags.off || intpar_synctex(&st.eqtb) == 0 || synctex.file == INVALID_HANDLE {
        return;
    }
    synctex.node = this_box;
    synctex.tag = synctex_tag_box(&st.mem, this_box);
    synctex.line = synctex_line_box(&st.mem, this_box);
    synctex.curh = synctex_curh(&st.shipout);
    synctex.curv = synctex_curv(&st.shipout);
    synctex.recorder = None;
    synctex_record_node_hlist(synctex, &st.mem, this_box);
}

pub unsafe fn synctex_tsilh(st: &mut crate::state::EngineState, this_box: i32) {
    let synctex = &mut st.synctex;
    if synctex.flags.off || intpar_synctex(&st.eqtb) == 0 || synctex.file == INVALID_HANDLE {
        return;
    }
    synctex.node = this_box;
    synctex.tag = synctex_tag_box(&st.mem, this_box);
    synctex.line = synctex_line_box(&st.mem, this_box);
    synctex.curh = synctex_curh(&st.shipout);
    synctex.curv = synctex_curv(&st.shipout);
    synctex.recorder = None;
    synctex_record_node_tsilh(synctex, this_box);
}

pub unsafe fn synctex_void_hlist(st: &mut crate::state::EngineState, p: i32, _this_box: i32) {
    let synctex = &mut st.synctex;
    let tag = synctex_tag_box(&st.mem, p);
    let line = synctex_line_box(&st.mem, p);
    if synctex.flags.off || intpar_synctex(&st.eqtb) == 0 || tag <= 0 || line <= 0 {
        return;
    }
    // Flush pending recorder before context change.
    if let Some(rec) = synctex.recorder {
        let node = synctex.node;
        rec(synctex, &st.mem, node);
    }
    synctex.node = p;
    synctex.tag = tag;
    synctex.line = line;
    synctex.curh = synctex_curh(&st.shipout);
    synctex.curv = synctex_curv(&st.shipout);
    synctex.recorder = None;
    synctex_record_node_void_hlist(synctex, &st.mem, p);
}

pub unsafe fn synctex_math(st: &mut crate::state::EngineState, p: i32, _this_box: i32) {
    let synctex = &mut st.synctex;
    if synctex.flags.off || intpar_synctex(&st.eqtb) == 0 || synctex.file == INVALID_HANDLE {
        return;
    }
    let tag = synctex_tag_medium(&st.mem, p);
    let line = synctex_line_medium(&st.mem, p);
    // SYNCTEX_CONTEXT_DID_CHANGE: node == 0 or tag/line differ.
    let ctx_changed = synctex.node == 0 || tag != synctex.tag || line != synctex.line;
    if let Some(rec) = synctex.recorder {
        if ctx_changed {
            let node = synctex.node;
            rec(synctex, &st.mem, node);
        }
    }
    synctex.node = p;
    synctex.tag = tag;
    synctex.line = line;
    synctex.curh = synctex_curh(&st.shipout);
    synctex.curv = synctex_curv(&st.shipout);
    synctex.recorder = None;
    synctex_record_node_math(synctex, &st.mem, p);
}

pub unsafe fn synctex_horizontal_rule_or_glue(
    st: &mut crate::state::EngineState,
    p: i32,
    _this_box: i32,
) {
    let synctex = &mut st.synctex;
    let node_type = synctex_type(&st.mem, p);
    // Second SYNCTEX_IGNORE definition: also checks tag/line > 0.
    let off = synctex.flags.off || intpar_synctex(&st.eqtb) == 0;
    let ignore = |tag: i32, line: i32| off || tag <= 0 || line <= 0;

    match node_type {
        RULE_NODE => {
            if ignore(synctex_tag_rule(&st.mem, p), synctex_line_rule(&st.mem, p)) {
                return;
            }
        }
        GLUE_NODE => {
            if ignore(
                synctex_tag_medium(&st.mem, p),
                synctex_line_medium(&st.mem, p),
            ) {
                return;
            }
        }
        KERN_NODE => {
            if ignore(
                synctex_tag_medium(&st.mem, p),
                synctex_line_medium(&st.mem, p),
            ) {
                return;
            }
        }
        _ => {
            let s = format!("unknown node type {} in SyncTeX\0", node_type);
            ttstub_issue_error(s.as_ptr() as *const c_char);
            return;
        }
    }

    synctex.node = p;
    synctex.curh = synctex_curh(&st.shipout);
    synctex.curv = synctex_curv(&st.shipout);
    synctex.recorder = None;
    match node_type {
        RULE_NODE => {
            synctex.tag = synctex_tag_rule(&st.mem, p);
            synctex.line = synctex_line_rule(&st.mem, p);
            synctex_record_node_rule(synctex, &st.mem, &st.shipout, p);
        }
        GLUE_NODE => {
            synctex.tag = synctex_tag_medium(&st.mem, p);
            synctex.line = synctex_line_medium(&st.mem, p);
            synctex_record_node_glue(synctex, &st.mem, p);
        }
        KERN_NODE => {
            synctex.tag = synctex_tag_medium(&st.mem, p);
            synctex.line = synctex_line_medium(&st.mem, p);
            synctex_record_node_kern(synctex, &st.mem, p);
        }
        _ => {
            // Already handled above — unreachable in practice.
        }
    }
}

pub unsafe fn synctex_kern(st: &mut crate::state::EngineState, p: i32, this_box: i32) {
    let synctex = &mut st.synctex;
    let tag = synctex_tag_medium(&st.mem, p);
    let line = synctex_line_medium(&st.mem, p);
    // Second SYNCTEX_IGNORE definition.
    if synctex.flags.off || intpar_synctex(&st.eqtb) == 0 || tag <= 0 || line <= 0 {
        return;
    }
    // SYNCTEX_CONTEXT_DID_CHANGE
    let ctx_changed = synctex.node == 0 || tag != synctex.tag || line != synctex.line;
    if ctx_changed {
        // Flush pending recorder.
        if let Some(rec) = synctex.recorder {
            let node = synctex.node;
            rec(synctex, &st.mem, node);
        }
        synctex.node = p;
        synctex.tag = tag;
        synctex.line = line;
        if synctex.node == this_box {
            // First node in hlist.
            synctex.recorder = Some(synctex_record_node_kern);
        } else {
            synctex.recorder = None;
            synctex_record_node_kern(synctex, &st.mem, p);
        }
    } else {
        // Same context — defer recording.
        synctex.node = p;
        synctex.tag = tag;
        synctex.line = line;
        synctex.recorder = Some(synctex_record_node_kern);
    }
}

pub unsafe fn synctex_current(st: &mut crate::state::EngineState) {
    let synctex = &mut st.synctex;
    if synctex.flags.off || intpar_synctex(&st.eqtb) == 0 || synctex.file == INVALID_HANDLE {
        return;
    }
    let (file, tag, line, unit) = (synctex.file, synctex.tag, synctex.line, synctex.unit);
    let s = format!(
        "x{},{}:{},{}\n",
        tag,
        line,
        synctex_curh(&st.shipout) / unit,
        synctex_curv(&st.shipout) / unit
    );
    let len = synctex_write(file, &s);
    synctex.lastv = synctex_curv(&st.shipout);
    if len > 0 {
        synctex.total_length += len;
    } else {
        synctexabort(synctex);
    }
}
