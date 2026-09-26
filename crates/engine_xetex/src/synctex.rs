/* Copyright (c) 2008-2017 jerome DOT laurens AT u-bourgogne DOT fr, MIT License. */

use std::fmt::Write as _;

use crate::bridge::{
    issue_error_cstr, issue_warning_str, ttstub_output_close, ttstub_output_open,
    ttstub_output_write,
};
use crate::consts::{INT_PAR_SYNCTEX, INVALID_HANDLE};
use crate::node::{AnyBox, AnyNode, HListNode, KernNode, MathNode, NodePtr, VListNode};
use crate::shipout::ONE_INCH;
use crate::state::{EngineState, EqtbState, ShipoutState, SynctexCtxt};
use crate::units::{MemIdx, MemPtr};
use libc::c_char;

const SYNCTEX_VERSION: i32 = 1;
/// The pseudo file of a resident compile, which only feeds the preview index.
const PREVIEW_HANDLE: usize = usize::MAX;

impl ShipoutState {
    fn synctex_curh(&self) -> i32 {
        self.cur_h + ONE_INCH
    }

    fn synctex_curv(&self) -> i32 {
        self.cur_v + ONE_INCH
    }
}

impl EqtbState {
    fn synctex_enabled(&self) -> bool {
        self.int_par(INT_PAR_SYNCTEX) != 0
    }
}

/// Bytes written (or that would be for the preview sink), -1 on failure.
fn synctex_write(handle: usize, s: &str) -> i32 {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return 0;
    }
    if handle == PREVIEW_HANDLE {
        return i32::try_from(bytes.len()).unwrap_or(i32::MAX);
    }
    // SAFETY: FFI write of a live byte slice.
    let written =
        unsafe { ttstub_output_write(handle, bytes.as_ptr() as *const c_char, bytes.len()) };
    if written > 0 {
        written as i32
    } else {
        -1
    }
}

impl SynctexCtxt {
    fn synctexabort(&mut self) {
        if self.file != INVALID_HANDLE && self.file != PREVIEW_HANDLE {
            // SAFETY: FFI close of the open SyncTeX file.
            unsafe { ttstub_output_close(self.file) };
            self.file = INVALID_HANDLE;
        }
        self.root_name = None;
        self.flags.off = true;
    }

    /// Write `s` and add it to the running length, aborting SyncTeX on failure.
    fn record(&mut self, s: &str) -> bool {
        let len = synctex_write(self.file, s);
        if len > 0 {
            self.total_length += len;
            true
        } else {
            self.synctexabort();
            false
        }
    }

    /// Like [`record`] but restarting the running length (anchors and the preamble).
    fn record_restart(&mut self, s: &str) -> bool {
        let len = synctex_write(self.file, s);
        if len > 0 {
            self.total_length = len;
            true
        } else {
            self.synctexabort();
            false
        }
    }

    fn record_anchor(&mut self) -> bool {
        let ok = self.record_restart(&format!("!{}\n", self.total_length));
        if ok {
            self.count += 1;
        }
        ok
    }

    fn record_input(&mut self, tag: i32, name: &std::ffi::CStr) -> bool {
        let name = name.to_string_lossy();
        if let Some(index) = &mut self.preview_index {
            index.register_name(tag, name.to_string());
        }
        self.record(&format!("Input:{tag}:{name}\n"))
    }

    fn record_postamble(&mut self) -> bool {
        let ok = self.record_anchor()
            && self.record("Postamble:\n")
            && self.record(&format!("Count:{}\n", self.count))
            && self.record_anchor()
            && self.record("Post scriptum:\n");
        if !ok {
            self.synctexabort();
        }
        ok
    }

    /// Write a sheet boundary, `{` opening and `}` closing page `sheet`.
    fn record_sheet(&mut self, brace: char, sheet: i32) {
        if self.record_anchor() && self.record(&format!("{brace}{sheet}\n")) {
            self.count += 1;
        } else {
            self.synctexabort();
        }
    }

    /// Record a node as `<kind><tag>,<line>:<h>,<v>`, followed by its dimensions.
    fn record_node(&mut self, kind: char, tag: i32, line: i32, dims: &[i32]) {
        if let Some(index) = &mut self.preview_index {
            let dim = |i: usize| dims.get(i).copied().unwrap_or(0);
            index.record(tag, line, self.curh, self.curv, dim(0), dim(1), dim(2));
        }
        if self.file == PREVIEW_HANDLE {
            self.lastv = self.curv;
            return;
        }
        let unit = self.unit;
        let mut s = format!(
            "{kind}{tag},{line}:{},{}",
            self.curh / unit,
            self.curv / unit
        );
        for (i, d) in dims.iter().enumerate() {
            let sep = if i == 0 { ':' } else { ',' };
            let _ = write!(s, "{sep}{}", d / unit);
        }
        s.push('\n');
        let len = synctex_write(self.file, &s);
        self.lastv = self.curv;
        if len > 0 {
            self.total_length += len;
            self.count += 1;
        } else {
            self.synctexabort();
        }
    }

    fn record_box(&mut self, mem: &crate::state::MemState, kind: char, b: AnyBox) {
        self.record_node(
            kind,
            b.sync_tag(mem),
            b.sync_line(mem),
            &[b.width(mem).0, b.height(mem).0, b.depth(mem).0],
        );
    }

    fn record_kern(&mut self, mem: &crate::state::MemState, kern: NodePtr<KernNode>) {
        self.record_node(
            'k',
            kern.sync_tag(mem),
            kern.sync_line(mem),
            &[kern.width(mem).0],
        );
    }

    /// Open the `.synctex.gz` file (or the preview sink) and write the preamble.
    /// Takes ownership of the malloc'ed job name `tmp`.
    unsafe fn synctex_dot_open(&mut self, eqtb: &EqtbState, tmp: *mut c_char) -> usize {
        if self.flags.off || !eqtb.synctex_enabled() {
            libc::free(tmp as *mut libc::c_void);
            return INVALID_HANDLE;
        }
        if self.file != INVALID_HANDLE {
            libc::free(tmp as *mut libc::c_void);
            return self.file;
        }

        if self.preview_index.is_some() {
            libc::free(tmp as *mut libc::c_void);
            self.file = PREVIEW_HANDLE;
        } else {
            let mut name = std::ffi::CStr::from_ptr(tmp).to_bytes().to_vec();
            libc::free(tmp as *mut libc::c_void);
            if name.is_empty() {
                self.synctexabort();
                return INVALID_HANDLE;
            }
            name.extend_from_slice(b".synctex.gz\0");
            self.file = ttstub_output_open(name.as_ptr() as *const c_char, 1);
            if self.file == INVALID_HANDLE {
                self.synctexabort();
                return INVALID_HANDLE;
            }
        }

        if !self.record_restart(&format!("SyncTeX Version:{SYNCTEX_VERSION}\n")) {
            self.synctexabort();
            return INVALID_HANDLE;
        }
        if self.magnification == 0 {
            self.magnification = 1000;
        }
        self.unit = 1;
        if let Some(root_name) = self.root_name.take() {
            self.record_input(1, &root_name);
        }
        self.count = 0;
        self.file
    }

    unsafe fn synctex_prepare_content(&mut self, eqtb: &EqtbState, tmp: *mut c_char) -> usize {
        if self.flags.content_ready {
            libc::free(tmp as *mut libc::c_void);
            return self.file;
        }
        if self.synctex_dot_open(eqtb, tmp) != INVALID_HANDLE
            && self.record_settings()
            && self.record("Content:\n")
        {
            self.flags.content_ready = true;
            return self.file;
        }
        self.synctexabort();
        INVALID_HANDLE
    }

    fn record_settings(&mut self) -> bool {
        if self.file == INVALID_HANDLE {
            return true;
        }
        let s = format!(
            "Output:pdf\nMagnification:{}\nUnit:{}\nX Offset:0\nY Offset:0\n",
            self.magnification, self.unit
        );
        self.record(&s)
    }
}

pub fn synctex_init_command(st: &mut EngineState) {
    let synctex = &mut st.synctex;
    synctex.file = INVALID_HANDLE;
    synctex.root_name = None;
    synctex.count = 0;
    synctex.node = MemPtr(0);
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
    synctex.preview_index = (st.config.execution_mode
        == crate::state::EngineExecutionMode::Resident)
        .then(Default::default);

    let synctex_enabled = st.config.synctex_enabled != 0;
    st.eqtb
        .set_int_par(INT_PAR_SYNCTEX, i32::from(synctex_enabled));
}

pub fn synctex_terminate(st: &mut EngineState) {
    let synctex = &mut st.synctex;
    if synctex.file != INVALID_HANDLE {
        synctex.record_postamble();
        if synctex.file != PREVIEW_HANDLE {
            // SAFETY: FFI close of the open SyncTeX file.
            unsafe { ttstub_output_close(synctex.file) };
        }
        synctex.file = INVALID_HANDLE;
    }
    synctex.synctexabort();
}

/// Takes ownership of the malloc'ed job name `tmp`.
pub unsafe fn synctex_start_input(st: &mut EngineState, tmp: *mut c_char) {
    let synctex = &mut st.synctex;
    if synctex.flags.off {
        libc::free(tmp as *mut libc::c_void);
        return;
    }
    if synctex.synctex_tag_counter == u32::MAX {
        synctex.synctex_tag_counter = 0;
        libc::free(tmp as *mut libc::c_void);
        return;
    }
    synctex.synctex_tag_counter += 1;
    let tag = synctex.synctex_tag_counter as i32;
    st.input.cur_input.synctex_tag = tag;
    st.mem.cur_synctex_tag = tag;
    let current_name = st.input.get_current_name();
    if let Some(index) = &mut synctex.preview_index {
        index.register_name(tag, current_name.to_string_lossy().into_owned());
    }

    if synctex.synctex_tag_counter == 1 {
        synctex.root_name = Some(if current_name.is_empty() {
            c"texput".to_owned()
        } else {
            current_name.to_owned()
        });
        libc::free(tmp as *mut libc::c_void);
        return;
    }

    if synctex.file != INVALID_HANDLE || synctex.synctex_dot_open(&st.eqtb, tmp) != INVALID_HANDLE {
        synctex.record_input(tag, st.input.get_current_name());
    }
}

/// Takes ownership of the malloc'ed job name `tmp`.
pub unsafe fn synctex_sheet(st: &mut EngineState, mag: i32, tmp: *mut c_char) {
    let total_pages = st.shipout.total_pages;
    let synctex = &mut st.synctex;
    if synctex.flags.off {
        libc::free(tmp as *mut libc::c_void);
        if st.eqtb.synctex_enabled() && !synctex.flags.warn {
            synctex.flags.warn = true;
            issue_warning_str(
                "SyncTeX was disabled -- changing the value of \\synctex has no effect",
            );
        }
        return;
    }
    if let Some(index) = &mut synctex.preview_index {
        index.begin_page(total_pages.max(0) as usize);
    }
    if total_pages == 0 && mag > 0 {
        synctex.magnification = mag;
    }
    if synctex.synctex_prepare_content(&st.eqtb, tmp) != INVALID_HANDLE {
        synctex.record_sheet('{', total_pages + 1);
    }
}

pub fn synctex_teehs(st: &mut EngineState) {
    let synctex = &mut st.synctex;
    if synctex.flags.off || synctex.file == INVALID_HANDLE {
        return;
    }
    synctex.record_sheet('}', st.shipout.total_pages);
    if let Some(index) = &mut synctex.preview_index {
        index.end_page();
    }
}

/// Records are written: SyncTeX is on and its file is open.
fn synctex_active(st: &EngineState) -> bool {
    !st.synctex.flags.off && st.eqtb.synctex_enabled() && st.synctex.file != INVALID_HANDLE
}

/// Make `node` the current SyncTeX context at the current position.
fn enter_node(st: &mut EngineState, node: MemPtr, tag: i32, line: i32) {
    let synctex = &mut st.synctex;
    synctex.node = node;
    synctex.tag = tag;
    synctex.line = line;
    synctex.curh = st.shipout.synctex_curh();
    synctex.curv = st.shipout.synctex_curv();
    synctex.deferred_kern = None;
}

fn enter_box(st: &mut EngineState, b: AnyBox) {
    let (tag, line) = (b.sync_tag(&st.mem), b.sync_line(&st.mem));
    enter_node(st, b.ptr(), tag, line);
}

pub fn synctex_vlist(st: &mut EngineState, this_box: AnyBox) {
    if !synctex_active(st) {
        return;
    }
    enter_box(st, this_box);
    st.synctex.flags.not_void = true;
    st.synctex.record_box(&st.mem, '[', this_box);
}

pub fn synctex_tsilv(st: &mut EngineState, this_box: AnyBox) {
    if !synctex_active(st) {
        return;
    }
    enter_box(st, this_box);
    st.synctex.record("]\n");
}

pub fn synctex_void_vlist(st: &mut EngineState, p: NodePtr<VListNode>) {
    if !synctex_active(st) {
        return;
    }
    enter_box(st, AnyBox::VList(p));
    st.synctex.record_box(&st.mem, 'v', AnyBox::VList(p));
}

pub fn synctex_hlist(st: &mut EngineState, this_box: AnyBox) {
    if !synctex_active(st) {
        return;
    }
    enter_box(st, this_box);
    st.synctex.flags.not_void = true;
    st.synctex.record_box(&st.mem, '(', this_box);
}

pub fn synctex_tsilh(st: &mut EngineState, this_box: AnyBox) {
    if !synctex_active(st) {
        return;
    }
    enter_box(st, this_box);
    if st.synctex.record(")\n") {
        st.synctex.count += 1;
    }
}

/// Whether a record is dropped: SyncTeX is off or the node has no source position.
fn synctex_ignored(st: &EngineState, tag: i32, line: i32) -> bool {
    st.synctex.flags.off || !st.eqtb.synctex_enabled() || tag <= 0 || line <= 0
}

/// Flush the deferred kern record before leaving its context.
fn flush_deferred_kern(st: &mut EngineState) {
    if let Some(kern) = st.synctex.deferred_kern {
        st.synctex.record_kern(&st.mem, kern);
    }
}

impl SynctexCtxt {
    fn context_changed(&self, tag: i32, line: i32) -> bool {
        self.node.0 == 0 || tag != self.tag || line != self.line
    }
}

pub fn synctex_void_hlist(st: &mut EngineState, p: NodePtr<HListNode>) {
    let (tag, line) = (p.sync_tag(&st.mem), p.sync_line(&st.mem));
    if synctex_ignored(st, tag, line) {
        return;
    }
    flush_deferred_kern(st);
    enter_node(st, p.ptr(), tag, line);
    st.synctex.record_box(&st.mem, 'h', AnyBox::HList(p));
}

pub fn synctex_math(st: &mut EngineState, p: NodePtr<MathNode>) {
    if !synctex_active(st) {
        return;
    }
    let (tag, line) = (p.sync_tag(&st.mem), p.sync_line(&st.mem));
    // A kern deferred in an unchanged context is dropped.
    if st.synctex.context_changed(tag, line) {
        flush_deferred_kern(st);
    }
    enter_node(st, p.ptr(), tag, line);
    st.synctex.record_node('$', tag, line, &[]);
}

/// The one polymorphic entry: `hlist_out` may have retyped a glue node to a
/// kern in place before this call, so the node is classified here.
pub fn synctex_horizontal_rule_or_glue(st: &mut EngineState, p: MemPtr) {
    let node = st.mem.classify(MemIdx::new(p.0));
    let (tag, line) = match node {
        Ok(AnyNode::Rule(r)) => (r.sync_tag(&st.mem), r.sync_line(&st.mem)),
        Ok(AnyNode::Glue(g)) => (g.sync_tag(&st.mem), g.sync_line(&st.mem)),
        Ok(AnyNode::Kern(k)) => (k.sync_tag(&st.mem), k.sync_line(&st.mem)),
        _ => {
            let s = format!(
                "unknown node type {} in SyncTeX\0",
                st.mem.type_code(MemIdx::new(p))
            );
            // SAFETY: `s` is NUL-terminated and outlives the call.
            unsafe { issue_error_cstr(s.as_ptr() as *const c_char) };
            return;
        }
    };
    if synctex_ignored(st, tag, line) {
        return;
    }
    enter_node(st, p, tag, line);
    let shipout = &st.shipout;
    let synctex = &mut st.synctex;
    match node {
        Ok(AnyNode::Rule(_)) => synctex.record_node(
            'r',
            tag,
            line,
            &[shipout.rule_wd, shipout.rule_ht, shipout.rule_dp],
        ),
        Ok(AnyNode::Glue(_)) => synctex.record_node('g', tag, line, &[]),
        Ok(AnyNode::Kern(k)) => synctex.record_kern(&st.mem, k),
        _ => {}
    }
}

pub fn synctex_kern(st: &mut EngineState, p: NodePtr<KernNode>, this_box: AnyBox) {
    let (tag, line) = (p.sync_tag(&st.mem), p.sync_line(&st.mem));
    if synctex_ignored(st, tag, line) {
        return;
    }
    let changed = st.synctex.context_changed(tag, line);
    if changed {
        flush_deferred_kern(st);
    }
    let synctex = &mut st.synctex;
    let first_in_list = synctex.node == this_box.ptr();
    synctex.node = p.ptr();
    synctex.tag = tag;
    synctex.line = line;
    // The first kern of an hlist, and any kern in an unchanged context, is deferred.
    if !changed || first_in_list {
        synctex.deferred_kern = Some(p);
    } else {
        synctex.deferred_kern = None;
        synctex.record_kern(&st.mem, p);
    }
}

pub fn synctex_current(st: &mut EngineState) {
    if !synctex_active(st) {
        return;
    }
    let (curh, curv) = (st.shipout.synctex_curh(), st.shipout.synctex_curv());
    let synctex = &mut st.synctex;
    if let Some(index) = &mut synctex.preview_index {
        index.record(synctex.tag, synctex.line, curh, curv, 0, 0, 0);
    }
    if synctex.file == PREVIEW_HANDLE {
        synctex.lastv = curv;
        return;
    }
    let unit = synctex.unit;
    let s = format!(
        "x{},{}:{},{}\n",
        synctex.tag,
        synctex.line,
        curh / unit,
        curv / unit
    );
    let len = synctex_write(synctex.file, &s);
    synctex.lastv = curv;
    if len > 0 {
        synctex.total_length += len;
    } else {
        synctex.synctexabort();
    }
}
