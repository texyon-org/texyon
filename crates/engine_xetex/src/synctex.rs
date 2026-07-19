/* Copyright (c) 2008-2017 jerome DOT laurens AT u-bourgogne DOT fr, MIT License.
 * Translated to Rust from xetex-synctex.c. */

use crate::consts::{INT_BASE, INVALID_HANDLE};
use crate::node::{AnyBox, AnyNode, HListNode, KernNode, MathNode, NodePtr, VListNode};
use crate::units::{MemIdx, MemPtr};
use libc::c_char;


// INT_BASE (save.rs) and INT_PAR__synctex (xetex_format.h:116).
const INT_PAR_SYNCTEX: i32 = 81;

const SYNCTEX_VERSION: i32 = 1;

use crate::bridge::{
    issue_error_cstr, issue_warning_cstr, ttstub_output_close, ttstub_output_open,
    ttstub_output_write,
};

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

unsafe fn synctexabort(synctex: &mut crate::state::SynctexCtxt) {
    if synctex.file != INVALID_HANDLE {
        ttstub_output_close(synctex.file);
        synctex.file = INVALID_HANDLE;
    }
    synctex.root_name = None;
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
    name: *const c_char,
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
    p: AnyBox,
) {
    synctex.flags.not_void = true;
    let (file, curh, curv, unit) = (synctex.file, synctex.curh, synctex.curv, synctex.unit);
    let s = format!(
        "[{},{}:{},{}:{},{},{}\n",
        p.sync_tag(mem),
        p.sync_line(mem),
        curh / unit,
        curv / unit,
        p.width(mem).0 / unit,
        p.height(mem).0 / unit,
        p.depth(mem).0 / unit
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

unsafe fn synctex_record_node_tsilv(synctex: &mut crate::state::SynctexCtxt) {
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
    p: NodePtr<VListNode>,
) {
    let (file, curh, curv, unit) = (synctex.file, synctex.curh, synctex.curv, synctex.unit);
    let s = format!(
        "v{},{}:{},{}:{},{},{}\n",
        p.sync_tag(mem),
        p.sync_line(mem),
        curh / unit,
        curv / unit,
        p.width(mem).0 / unit,
        p.height(mem).0 / unit,
        p.depth(mem).0 / unit
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
    p: AnyBox,
) {
    synctex.flags.not_void = true;
    let (file, curh, curv, unit) = (synctex.file, synctex.curh, synctex.curv, synctex.unit);
    let s = format!(
        "({},{}:{},{}:{},{},{}\n",
        p.sync_tag(mem),
        p.sync_line(mem),
        curh / unit,
        curv / unit,
        p.width(mem).0 / unit,
        p.height(mem).0 / unit,
        p.depth(mem).0 / unit
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

unsafe fn synctex_record_node_tsilh(synctex: &mut crate::state::SynctexCtxt) {
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
    p: NodePtr<HListNode>,
) {
    let (file, curh, curv, unit) = (synctex.file, synctex.curh, synctex.curv, synctex.unit);
    let s = format!(
        "h{},{}:{},{}:{},{},{}\n",
        p.sync_tag(mem),
        p.sync_line(mem),
        curh / unit,
        curv / unit,
        p.width(mem).0 / unit,
        p.height(mem).0 / unit,
        p.depth(mem).0 / unit
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
    p: NodePtr<MathNode>,
) {
    let (file, curh, curv, unit) = (synctex.file, synctex.curh, synctex.curv, synctex.unit);
    let s = format!(
        "${},{}:{},{}\n",
        p.sync_tag(mem),
        p.sync_line(mem),
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
    p: NodePtr<crate::node::GlueNode>,
) {
    let (file, curh, curv, unit) = (synctex.file, synctex.curh, synctex.curv, synctex.unit);
    let s = format!(
        "g{},{}:{},{}\n",
        p.sync_tag(mem),
        p.sync_line(mem),
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
    p: NodePtr<KernNode>,
) {
    let (file, curh, curv, unit) = (synctex.file, synctex.curh, synctex.curv, synctex.unit);
    let s = format!(
        "k{},{}:{},{}:{}\n",
        p.sync_tag(mem),
        p.sync_line(mem),
        curh / unit,
        curv / unit,
        p.width(mem).0 / unit
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
    p: NodePtr<crate::node::RuleNode>,
) {
    let (file, curh, curv, unit) = (synctex.file, synctex.curh, synctex.curv, synctex.unit);
    let s = format!(
        "r{},{}:{},{}:{},{},{}\n",
        p.sync_tag(mem),
        p.sync_line(mem),
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

    if let Some(root_name) = synctex.root_name.take() {
        synctex_record_input(synctex, 1, root_name.as_ptr());
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
    synctex.root_name = None;
    synctex.count = 0;
    synctex.node.0 = 0;
    synctex.deferred_kern = None;
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
    st.input.cur_input.synctex_tag = tag;
    st.mem.cur_synctex_tag = st.input.cur_input.synctex_tag;

    if synctex.synctex_tag_counter == 1 {
        let name = st.input.get_current_name();
        synctex.root_name = Some(if name.is_empty() {
            c"texput".to_owned()
        } else {
            name.to_owned()
        });
        libc::free(tmp as *mut libc::c_void);
        return;
    }

    if synctex.file != INVALID_HANDLE || synctex_dot_open(synctex, &st.eqtb, tmp) != INVALID_HANDLE
    {
        let ci = st.input.cur_input;
        let tmp_name = st.input.get_current_name();
        synctex_record_input(synctex, ci.synctex_tag, tmp_name.as_ptr());
    }
}

pub unsafe fn synctex_sheet(st: &mut crate::state::EngineState, mag: i32, tmp: *mut c_char) {
    let (is_off, total_pages) = (st.synctex.flags.off, st.shipout.total_pages);

    if is_off {
        libc::free(tmp as *mut libc::c_void);
        if intpar_synctex(&st.eqtb) != 0 && !st.synctex.flags.warn {
            st.synctex.flags.warn = true;
            issue_warning_cstr(
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

pub unsafe fn synctex_vlist(st: &mut crate::state::EngineState, this_box: AnyBox) {
    let synctex = &mut st.synctex;
    if synctex.flags.off || intpar_synctex(&st.eqtb) == 0 || synctex.file == INVALID_HANDLE {
        return;
    }
    synctex.node = this_box.ptr();
    synctex.deferred_kern = None;
    synctex.tag = this_box.sync_tag(&st.mem);
    synctex.line = this_box.sync_line(&st.mem);
    synctex.curh = synctex_curh(&st.shipout);
    synctex.curv = synctex_curv(&st.shipout);
    synctex_record_node_vlist(synctex, &st.mem, this_box);
}

pub unsafe fn synctex_tsilv(st: &mut crate::state::EngineState, this_box: AnyBox) {
    let synctex = &mut st.synctex;
    if synctex.flags.off || intpar_synctex(&st.eqtb) == 0 || synctex.file == INVALID_HANDLE {
        return;
    }
    synctex.node = this_box.ptr();
    synctex.tag = this_box.sync_tag(&st.mem);
    synctex.line = this_box.sync_line(&st.mem);
    synctex.curh = synctex_curh(&st.shipout);
    synctex.curv = synctex_curv(&st.shipout);
    synctex.deferred_kern = None;
    synctex_record_node_tsilv(synctex);
}

pub unsafe fn synctex_void_vlist(st: &mut crate::state::EngineState, p: NodePtr<VListNode>) {
    let synctex = &mut st.synctex;
    if synctex.flags.off || intpar_synctex(&st.eqtb) == 0 || synctex.file == INVALID_HANDLE {
        return;
    }
    synctex.node = p.ptr();
    synctex.tag = p.sync_tag(&st.mem);
    synctex.line = p.sync_line(&st.mem);
    synctex.curh = synctex_curh(&st.shipout);
    synctex.curv = synctex_curv(&st.shipout);
    synctex.deferred_kern = None;
    synctex_record_node_void_vlist(synctex, &st.mem, p);
}

pub unsafe fn synctex_hlist(st: &mut crate::state::EngineState, this_box: AnyBox) {
    let synctex = &mut st.synctex;
    if synctex.flags.off || intpar_synctex(&st.eqtb) == 0 || synctex.file == INVALID_HANDLE {
        return;
    }
    synctex.node = this_box.ptr();
    synctex.tag = this_box.sync_tag(&st.mem);
    synctex.line = this_box.sync_line(&st.mem);
    synctex.curh = synctex_curh(&st.shipout);
    synctex.curv = synctex_curv(&st.shipout);
    synctex.deferred_kern = None;
    synctex_record_node_hlist(synctex, &st.mem, this_box);
}

pub unsafe fn synctex_tsilh(st: &mut crate::state::EngineState, this_box: AnyBox) {
    let synctex = &mut st.synctex;
    if synctex.flags.off || intpar_synctex(&st.eqtb) == 0 || synctex.file == INVALID_HANDLE {
        return;
    }
    synctex.node = this_box.ptr();
    synctex.tag = this_box.sync_tag(&st.mem);
    synctex.line = this_box.sync_line(&st.mem);
    synctex.curh = synctex_curh(&st.shipout);
    synctex.curv = synctex_curv(&st.shipout);
    synctex.deferred_kern = None;
    synctex_record_node_tsilh(synctex);
}

pub unsafe fn synctex_void_hlist(st: &mut crate::state::EngineState, p: NodePtr<HListNode>) {
    let synctex = &mut st.synctex;
    let tag = p.sync_tag(&st.mem);
    let line = p.sync_line(&st.mem);
    if synctex.flags.off || intpar_synctex(&st.eqtb) == 0 || tag <= 0 || line <= 0 {
        return;
    }
    // Flush a pending kern record before the context change.
    if let Some(kern) = synctex.deferred_kern {
        synctex_record_node_kern(synctex, &st.mem, kern);
    }
    synctex.node = p.ptr();
    synctex.tag = tag;
    synctex.line = line;
    synctex.curh = synctex_curh(&st.shipout);
    synctex.curv = synctex_curv(&st.shipout);
    synctex.deferred_kern = None;
    synctex_record_node_void_hlist(synctex, &st.mem, p);
}

pub unsafe fn synctex_math(st: &mut crate::state::EngineState, p: NodePtr<MathNode>) {
    let synctex = &mut st.synctex;
    if synctex.flags.off || intpar_synctex(&st.eqtb) == 0 || synctex.file == INVALID_HANDLE {
        return;
    }
    let tag = p.sync_tag(&st.mem);
    let line = p.sync_line(&st.mem);
    // SYNCTEX_CONTEXT_DID_CHANGE: node == 0 (the C sentinel, not TEX_NULL) or
    // tag/line differ.
    let ctx_changed = synctex.node.0 == 0 || tag != synctex.tag || line != synctex.line;
    if ctx_changed {
        // Flush a pending kern record; an armed record in an UNCHANGED context
        // is silently dropped below — faithful C.
        if let Some(kern) = synctex.deferred_kern {
            synctex_record_node_kern(synctex, &st.mem, kern);
        }
    }
    synctex.node = p.ptr();
    synctex.tag = tag;
    synctex.line = line;
    synctex.curh = synctex_curh(&st.shipout);
    synctex.curv = synctex_curv(&st.shipout);
    synctex.deferred_kern = None;
    synctex_record_node_math(synctex, &st.mem, p);
}

/// The one genuinely polymorphic SyncTeX entry: `hlist_out`'s glue path
/// re-stamps its glue node to `KERN_NODE` in place *before* this call, so the
/// runtime input set is Rule | Glue | Kern and the node must be re-derived
/// here — the caller's typed handle (if it ever had one) is stale.
pub unsafe fn synctex_horizontal_rule_or_glue(st: &mut crate::state::EngineState, p: MemPtr) {
    let synctex = &mut st.synctex;
    let node = st.mem.classify(MemIdx::new(p.0));
    // Second SYNCTEX_IGNORE definition: also checks tag/line > 0.
    let off = synctex.flags.off || intpar_synctex(&st.eqtb) == 0;
    let ignore = |tag: i32, line: i32| off || tag <= 0 || line <= 0;

    match node {
        Ok(AnyNode::Rule(r)) => {
            if ignore(r.sync_tag(&st.mem), r.sync_line(&st.mem)) {
                return;
            }
        }
        Ok(AnyNode::Glue(g)) => {
            if ignore(g.sync_tag(&st.mem), g.sync_line(&st.mem)) {
                return;
            }
        }
        Ok(AnyNode::Kern(k)) => {
            if ignore(k.sync_tag(&st.mem), k.sync_line(&st.mem)) {
                return;
            }
        }
        _ => {
            let s = format!("unknown node type {} in SyncTeX\0", st.mem.b16s1(p.0));
            issue_error_cstr(s.as_ptr() as *const c_char);
            return;
        }
    }

    synctex.node = p;
    synctex.curh = synctex_curh(&st.shipout);
    synctex.curv = synctex_curv(&st.shipout);
    synctex.deferred_kern = None;
    match node {
        Ok(AnyNode::Rule(r)) => {
            synctex.tag = r.sync_tag(&st.mem);
            synctex.line = r.sync_line(&st.mem);
            synctex_record_node_rule(synctex, &st.mem, &st.shipout, r);
        }
        Ok(AnyNode::Glue(g)) => {
            synctex.tag = g.sync_tag(&st.mem);
            synctex.line = g.sync_line(&st.mem);
            synctex_record_node_glue(synctex, &st.mem, g);
        }
        Ok(AnyNode::Kern(k)) => {
            synctex.tag = k.sync_tag(&st.mem);
            synctex.line = k.sync_line(&st.mem);
            synctex_record_node_kern(synctex, &st.mem, k);
        }
        _ => {
            // Already handled above — unreachable in practice.
        }
    }
}

pub unsafe fn synctex_kern(
    st: &mut crate::state::EngineState,
    p: NodePtr<KernNode>,
    this_box: AnyBox,
) {
    let synctex = &mut st.synctex;
    let tag = p.sync_tag(&st.mem);
    let line = p.sync_line(&st.mem);
    // Second SYNCTEX_IGNORE definition.
    if synctex.flags.off || intpar_synctex(&st.eqtb) == 0 || tag <= 0 || line <= 0 {
        return;
    }
    // SYNCTEX_CONTEXT_DID_CHANGE
    let ctx_changed = synctex.node.0 == 0 || tag != synctex.tag || line != synctex.line;
    if ctx_changed {
        // Flush pending kern record.
        if let Some(kern) = synctex.deferred_kern {
            synctex_record_node_kern(synctex, &st.mem, kern);
        }
        synctex.node = p.ptr();
        synctex.tag = tag;
        synctex.line = line;
        if synctex.node == this_box.ptr() {
            // First node in hlist.
            synctex.deferred_kern = Some(p);
        } else {
            synctex.deferred_kern = None;
            synctex_record_node_kern(synctex, &st.mem, p);
        }
    } else {
        // Same context — defer recording.
        synctex.node = p.ptr();
        synctex.tag = tag;
        synctex.line = line;
        synctex.deferred_kern = Some(p);
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
