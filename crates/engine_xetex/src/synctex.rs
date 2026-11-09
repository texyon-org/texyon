/* Copyright (c) 2008-2017 jerome DOT laurens AT u-bourgogne DOT fr, MIT License. */

use crate::params::IntPar;
use std::fmt::Write as _;

use crate::bridge::{issue_error, issue_warning, output_close, output_open, output_write};
use crate::consts::INVALID_HANDLE;
use crate::node::{AnyNode, BoxNode, KernNode, MathNode, NodePtr};
use crate::shipout::ONE_INCH;
use crate::state::{EngineState, EqtbState, ShipoutState, SynctexCtxt};
use crate::sync_index::RecordKind;
use crate::units::MemPtr;

const SYNCTEX_VERSION: i32 = 1;
/// The pseudo file of a resident compile, which only feeds the preview index.
const PREVIEW_HANDLE: usize = usize::MAX;

impl ShipoutState {
    fn synctex_curh(&self) -> i32 {
        (self.cur_h + ONE_INCH).raw()
    }

    fn synctex_curv(&self) -> i32 {
        (self.cur_v + ONE_INCH).raw()
    }
}

impl EqtbState {
    fn synctex_enabled(&self) -> bool {
        self.int_par(IntPar::Synctex) != 0
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
    if output_write(handle, bytes) {
        bytes.len() as i32
    } else {
        -1
    }
}

impl SynctexCtxt {
    fn abort(&mut self) {
        if self.file != INVALID_HANDLE && self.file != PREVIEW_HANDLE {
            output_close(self.file);
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
            self.abort();
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
            self.abort();
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

    fn record_input(&mut self, tag: i32, name: &[u8]) -> bool {
        let name = String::from_utf8_lossy(name);
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
            self.abort();
        }
        ok
    }

    /// Write a sheet boundary, `{` opening and `}` closing page `sheet`.
    fn record_sheet(&mut self, brace: char, sheet: i32) {
        if self.record_anchor() && self.record(&format!("{brace}{sheet}\n")) {
            self.count += 1;
        } else {
            self.abort();
        }
    }

    /// Record a node as `<kind><tag>,<line>:<h>,<v>`, followed by its dimensions.
    fn record_node(&mut self, kind: char, tag: i32, line: i32, dims: &[i32]) {
        if let (Some(index), Some(kind)) = (&mut self.preview_index, RecordKind::from_prefix(kind))
        {
            index.record(kind, [tag, line, self.curh, self.curv], dims);
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
            self.abort();
        }
    }

    fn record_box(&mut self, mem: &crate::state::MemState, kind: char, b: NodePtr<BoxNode>) {
        self.record_node(
            kind,
            mem[b].sync_tag,
            mem[b].sync_line,
            &[mem[b].width.raw(), mem[b].height.raw(), mem[b].depth.raw()],
        );
    }

    fn record_kern(&mut self, mem: &crate::state::MemState, kern: NodePtr<KernNode>) {
        self.record_node(
            'k',
            mem[kern].sync_tag,
            mem[kern].sync_line,
            &[mem[kern].width.raw()],
        );
    }

    /// Open the `.synctex.gz` file (or the preview sink) and write the preamble.
    fn synctex_dot_open(&mut self, eqtb: &EqtbState, job_name: &[u8]) -> usize {
        if self.flags.off || !eqtb.synctex_enabled() {
            return INVALID_HANDLE;
        }
        if self.file != INVALID_HANDLE {
            return self.file;
        }

        if self.preview_index.is_some() {
            self.file = PREVIEW_HANDLE;
        } else {
            let mut name = job_name.to_vec();
            if name.is_empty() {
                self.abort();
                return INVALID_HANDLE;
            }
            name.extend_from_slice(b".synctex.gz");
            self.file = output_open(&name, true);
            if self.file == INVALID_HANDLE {
                self.abort();
                return INVALID_HANDLE;
            }
        }

        if !self.record_restart(&format!("SyncTeX Version:{SYNCTEX_VERSION}\n")) {
            self.abort();
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

    fn synctex_prepare_content(&mut self, eqtb: &EqtbState, job_name: &[u8]) -> usize {
        if self.flags.content_ready {
            return self.file;
        }
        if self.synctex_dot_open(eqtb, job_name) != INVALID_HANDLE
            && self.record_settings()
            && self.record("Content:\n")
        {
            self.flags.content_ready = true;
            return self.file;
        }
        self.abort();
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
    synctex.node = None;
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
        .set_int_par(IntPar::Synctex, i32::from(synctex_enabled));
}

pub fn synctex_terminate(st: &mut EngineState) {
    let synctex = &mut st.synctex;
    if synctex.file != INVALID_HANDLE {
        synctex.record_postamble();
        if synctex.file != PREVIEW_HANDLE {
            output_close(synctex.file);
        }
        synctex.file = INVALID_HANDLE;
    }
    synctex.abort();
}

/// Tags the input file just opened at absolute path `current_name`.
pub fn synctex_start_input(st: &mut EngineState, job_name: &[u8], current_name: &[u8]) {
    let synctex = &mut st.synctex;
    if synctex.flags.off {
        return;
    }
    if synctex.synctex_tag_counter == u32::MAX {
        synctex.synctex_tag_counter = 0;
        return;
    }
    synctex.synctex_tag_counter += 1;
    let tag = synctex.synctex_tag_counter as i32;
    st.input.cur_input.synctex_tag = tag;
    st.mem.cur_synctex_tag = tag;
    if let Some(index) = &mut synctex.preview_index {
        index.register_name(tag, String::from_utf8_lossy(current_name).into_owned());
    }

    if synctex.synctex_tag_counter == 1 {
        synctex.root_name = Some(if current_name.is_empty() {
            b"texput".to_vec()
        } else {
            current_name.to_vec()
        });
        return;
    }

    if synctex.file != INVALID_HANDLE
        || synctex.synctex_dot_open(&st.eqtb, job_name) != INVALID_HANDLE
    {
        synctex.record_input(tag, current_name);
    }
}

pub fn synctex_sheet(st: &mut EngineState, mag: i32, job_name: &[u8]) {
    let total_pages = st.shipout.total_pages;
    let synctex = &mut st.synctex;
    if synctex.flags.off {
        if st.eqtb.synctex_enabled() && !synctex.flags.warn {
            synctex.flags.warn = true;
            issue_warning("SyncTeX was disabled -- changing the value of \\synctex has no effect");
        }
        return;
    }
    if let Some(index) = &mut synctex.preview_index {
        index.begin_page(total_pages.max(0) as usize);
    }
    if total_pages == 0 && mag > 0 {
        synctex.magnification = mag;
    }
    if synctex.synctex_prepare_content(&st.eqtb, job_name) != INVALID_HANDLE {
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
    synctex.node = Some(node);
    synctex.tag = tag;
    synctex.line = line;
    synctex.curh = st.shipout.synctex_curh();
    synctex.curv = st.shipout.synctex_curv();
    synctex.deferred_kern = None;
}

fn enter_box(st: &mut EngineState, b: NodePtr<BoxNode>) {
    let (tag, line) = (st.mem[b].sync_tag, st.mem[b].sync_line);
    enter_node(st, b.ptr(), tag, line);
}

pub fn synctex_vlist(st: &mut EngineState, this_box: NodePtr<BoxNode>) {
    if !synctex_active(st) {
        return;
    }
    enter_box(st, this_box);
    st.synctex.flags.not_void = true;
    st.synctex.record_box(&st.mem, '[', this_box);
}

pub fn synctex_tsilv(st: &mut EngineState, this_box: NodePtr<BoxNode>) {
    if !synctex_active(st) {
        return;
    }
    enter_box(st, this_box);
    if let Some(index) = &mut st.synctex.preview_index {
        index.close(RecordKind::VBoxEnd);
    }
    st.synctex.record("]\n");
}

pub fn synctex_void_vlist(st: &mut EngineState, p: NodePtr<BoxNode>) {
    if !synctex_active(st) {
        return;
    }
    enter_box(st, p);
    st.synctex.record_box(&st.mem, 'v', p);
}

pub fn synctex_hlist(st: &mut EngineState, this_box: NodePtr<BoxNode>) {
    if !synctex_active(st) {
        return;
    }
    enter_box(st, this_box);
    st.synctex.flags.not_void = true;
    st.synctex.record_box(&st.mem, '(', this_box);
}

pub fn synctex_tsilh(st: &mut EngineState, this_box: NodePtr<BoxNode>) {
    if !synctex_active(st) {
        return;
    }
    enter_box(st, this_box);
    if let Some(index) = &mut st.synctex.preview_index {
        index.close(RecordKind::HBoxEnd);
    }
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
        self.node.is_none() || tag != self.tag || line != self.line
    }
}

pub fn synctex_void_hlist(st: &mut EngineState, p: NodePtr<BoxNode>) {
    let (tag, line) = (st.mem[p].sync_tag, st.mem[p].sync_line);
    if synctex_ignored(st, tag, line) {
        return;
    }
    flush_deferred_kern(st);
    enter_node(st, p.ptr(), tag, line);
    st.synctex.record_box(&st.mem, 'h', p);
}

pub fn synctex_math(st: &mut EngineState, p: NodePtr<MathNode>) {
    if !synctex_active(st) {
        return;
    }
    let (tag, line) = (st.mem[p].sync_tag, st.mem[p].sync_line);
    // A kern deferred in an unchanged context is dropped.
    if st.synctex.context_changed(tag, line) {
        flush_deferred_kern(st);
    }
    enter_node(st, p.ptr(), tag, line);
    st.synctex.record_node('$', tag, line, &[]);
}

/// The one polymorphic entry: `hlist_out` may have retyped a glue node to a
/// kern in place before this call, so the node is classified here.
/// Record rule or glue `p` as shipped, with `rule` the resolved size of a rule.
pub fn synctex_horizontal_rule_or_glue(
    st: &mut EngineState,
    p: MemPtr,
    rule: Option<crate::shipout::RuleDims>,
) {
    let node = st.mem.classify(p);
    let (tag, line) = match node {
        Ok(AnyNode::Rule(r)) => (st.mem[r].sync_tag, st.mem[r].sync_line),
        Ok(AnyNode::Glue(g)) => (st.mem[g].sync_tag, st.mem[g].sync_line),
        Ok(AnyNode::Kern(k)) => (st.mem[k].sync_tag, st.mem[k].sync_line),
        _ => {
            issue_error(&format!(
                "unknown node type {} in SyncTeX",
                st.mem.type_code(p)
            ));
            return;
        }
    };
    if synctex_ignored(st, tag, line) {
        return;
    }
    enter_node(st, p, tag, line);
    let synctex = &mut st.synctex;
    match (node, rule) {
        (Ok(AnyNode::Rule(_)), Some(rule)) => synctex.record_node(
            'r',
            tag,
            line,
            &[rule.wd.raw(), rule.ht.raw(), rule.dp.raw()],
        ),
        (Ok(AnyNode::Glue(_)), _) => synctex.record_node('g', tag, line, &[]),
        (Ok(AnyNode::Kern(k)), _) => synctex.record_kern(&st.mem, k),
        _ => {}
    }
}

pub fn synctex_kern(st: &mut EngineState, p: NodePtr<KernNode>, this_box: NodePtr<BoxNode>) {
    let (tag, line) = (st.mem[p].sync_tag, st.mem[p].sync_line);
    if synctex_ignored(st, tag, line) {
        return;
    }
    let changed = st.synctex.context_changed(tag, line);
    if changed {
        flush_deferred_kern(st);
    }
    let synctex = &mut st.synctex;
    let first_in_list = synctex.node == Some(this_box.ptr());
    synctex.node = Some(p.ptr());
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
        index.record(
            RecordKind::Boundary,
            [synctex.tag, synctex.line, curh, curv],
            &[],
        );
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
        synctex.abort();
    }
}
