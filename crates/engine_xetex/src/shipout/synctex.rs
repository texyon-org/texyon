/* Copyright (c) 2008-2017 jerome DOT laurens AT u-bourgogne DOT fr, MIT License. */

use std::fmt::Write as _;

use super::{ONE_INCH, RuleDims, ShipoutState};
use crate::base::consts::INVALID_HANDLE;
use crate::base::params::IntPar;
use crate::base::units::{MemAddr, MemPtr};
use crate::env::EngineEnv;
use crate::ffi::bridge::{issue_error, issue_warning, output_close, output_open, output_write};
use crate::memory::mem::MemState;
use crate::memory::node::{AnyNode, BoxNode, KernNode, MathNode, NodePtr};
use crate::preview::sync_index::{PreviewSyncIndex, RecordKind};
use crate::state::{EngineExecutionMode, EngineState};
use crate::tables::eqtb::EqtbState;

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
fn synctex_write(env: &mut EngineEnv, handle: usize, s: &str) -> i32 {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return 0;
    }
    if handle == PREVIEW_HANDLE {
        return i32::try_from(bytes.len()).unwrap_or(i32::MAX);
    }
    if output_write(env, handle, bytes) {
        bytes.len() as i32
    } else {
        -1
    }
}

impl SynctexCtxt {
    fn abort(&mut self, env: &mut EngineEnv) {
        if self.file != INVALID_HANDLE && self.file != PREVIEW_HANDLE {
            output_close(env, self.file);
            self.file = INVALID_HANDLE;
        }
        self.root_name = None;
        self.flags.off = true;
    }

    fn record(&mut self, env: &mut EngineEnv, s: &str) -> bool {
        let len = synctex_write(env, self.file, s);
        if len > 0 {
            self.total_length += len;
            true
        } else {
            self.abort(env);
            false
        }
    }

    /// Like [`record`] but restarting the running length (anchors and the preamble).
    fn record_restart(&mut self, env: &mut EngineEnv, s: &str) -> bool {
        let len = synctex_write(env, self.file, s);
        if len > 0 {
            self.total_length = len;
            true
        } else {
            self.abort(env);
            false
        }
    }

    fn record_anchor(&mut self, env: &mut EngineEnv) -> bool {
        let ok = self.record_restart(env, &format!("!{}\n", self.total_length));
        if ok {
            self.count += 1;
        }
        ok
    }

    fn record_input(&mut self, env: &mut EngineEnv, tag: i32, name: &[u8]) -> bool {
        let name = String::from_utf8_lossy(name);
        if let Some(index) = &mut self.preview_index {
            index.register_name(tag, name.to_string());
        }
        self.record(env, &format!("Input:{tag}:{name}\n"))
    }

    fn record_postamble(&mut self, env: &mut EngineEnv) -> bool {
        let ok = self.record_anchor(env)
            && self.record(env, "Postamble:\n")
            && self.record(env, &format!("Count:{}\n", self.count))
            && self.record_anchor(env)
            && self.record(env, "Post scriptum:\n");
        if !ok {
            self.abort(env);
        }
        ok
    }

    fn record_sheet(&mut self, env: &mut EngineEnv, brace: char, sheet: i32) {
        if self.record_anchor(env) && self.record(env, &format!("{brace}{sheet}\n")) {
            self.count += 1;
        } else {
            self.abort(env);
        }
    }

    fn record_node(&mut self, env: &mut EngineEnv, kind: char, tag: i32, line: i32, dims: &[i32]) {
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
        let len = synctex_write(env, self.file, &s);
        self.lastv = self.curv;
        if len > 0 {
            self.total_length += len;
            self.count += 1;
        } else {
            self.abort(env);
        }
    }

    fn record_box(&mut self, env: &mut EngineEnv, mem: &MemState, kind: char, b: NodePtr<BoxNode>) {
        self.record_node(
            env,
            kind,
            mem[b].sync.tag,
            mem[b].sync.line,
            &[mem[b].width.raw(), mem[b].height.raw(), mem[b].depth.raw()],
        );
    }

    fn record_kern(&mut self, env: &mut EngineEnv, mem: &MemState, kern: NodePtr<KernNode>) {
        self.record_node(
            env,
            'k',
            mem[kern].sync.tag,
            mem[kern].sync.line,
            &[mem[kern].width.raw()],
        );
    }

    fn synctex_dot_open(
        &mut self,
        env: &mut EngineEnv,
        eqtb: &EqtbState,
        job_name: &[u8],
    ) -> usize {
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
                self.abort(env);
                return INVALID_HANDLE;
            }
            name.extend_from_slice(b".synctex.gz");
            self.file = output_open(env, &name, true);
            if self.file == INVALID_HANDLE {
                self.abort(env);
                return INVALID_HANDLE;
            }
        }

        if !self.record_restart(env, &format!("SyncTeX Version:{SYNCTEX_VERSION}\n")) {
            self.abort(env);
            return INVALID_HANDLE;
        }
        if self.magnification == 0 {
            self.magnification = 1000;
        }
        self.unit = 1;
        if let Some(root_name) = self.root_name.take() {
            self.record_input(env, 1, &root_name);
        }
        self.count = 0;
        self.file
    }

    fn synctex_prepare_content(
        &mut self,
        env: &mut EngineEnv,
        eqtb: &EqtbState,
        job_name: &[u8],
    ) -> usize {
        if self.flags.content_ready {
            return self.file;
        }
        if self.synctex_dot_open(env, eqtb, job_name) != INVALID_HANDLE
            && self.record_settings(env)
            && self.record(env, "Content:\n")
        {
            self.flags.content_ready = true;
            return self.file;
        }
        self.abort(env);
        INVALID_HANDLE
    }

    fn record_settings(&mut self, env: &mut EngineEnv) -> bool {
        if self.file == INVALID_HANDLE {
            return true;
        }
        let s = format!(
            "Output:pdf\nMagnification:{}\nUnit:{}\nX Offset:0\nY Offset:0\n",
            self.magnification, self.unit
        );
        self.record(env, &s)
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
    synctex.preview_index =
        (st.config.execution_mode == EngineExecutionMode::Resident).then(Default::default);

    let synctex_enabled = st.config.synctex_enabled != 0;
    st.eqtb
        .set_int_par(IntPar::Synctex, i32::from(synctex_enabled));
}

pub fn synctex_terminate(st: &mut EngineState, env: &mut EngineEnv) {
    let synctex = &mut st.synctex;
    if synctex.file != INVALID_HANDLE {
        synctex.record_postamble(env);
        if synctex.file != PREVIEW_HANDLE {
            output_close(env, synctex.file);
        }
        synctex.file = INVALID_HANDLE;
    }
    synctex.abort(env);
}

pub fn synctex_start_input(
    st: &mut EngineState,
    env: &mut EngineEnv,
    job_name: &[u8],
    current_name: &[u8],
) {
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
        || synctex.synctex_dot_open(env, &st.eqtb, job_name) != INVALID_HANDLE
    {
        synctex.record_input(env, tag, current_name);
    }
}

pub fn synctex_sheet(st: &mut EngineState, env: &mut EngineEnv, mag: i32, job_name: &[u8]) {
    let total_pages = st.shipout.total_pages;
    let synctex = &mut st.synctex;
    if synctex.flags.off {
        if st.eqtb.synctex_enabled() && !synctex.flags.warn {
            synctex.flags.warn = true;
            issue_warning(
                env,
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
    if synctex.synctex_prepare_content(env, &st.eqtb, job_name) != INVALID_HANDLE {
        synctex.record_sheet(env, '{', total_pages + 1);
    }
}

pub fn synctex_teehs(st: &mut EngineState, env: &mut EngineEnv) {
    let synctex = &mut st.synctex;
    if synctex.flags.off || synctex.file == INVALID_HANDLE {
        return;
    }
    synctex.record_sheet(env, '}', st.shipout.total_pages);
    if let Some(index) = &mut synctex.preview_index {
        index.end_page();
    }
}

fn synctex_active(st: &EngineState) -> bool {
    !st.synctex.flags.off && st.eqtb.synctex_enabled() && st.synctex.file != INVALID_HANDLE
}

fn enter_node(st: &mut EngineState, node: impl MemAddr, tag: i32, line: i32) {
    let synctex = &mut st.synctex;
    synctex.node = Some(node.addr());
    synctex.tag = tag;
    synctex.line = line;
    synctex.curh = st.shipout.synctex_curh();
    synctex.curv = st.shipout.synctex_curv();
    synctex.deferred_kern = None;
}

fn enter_box(st: &mut EngineState, b: NodePtr<BoxNode>) {
    let (tag, line) = (st.mem[b].sync.tag, st.mem[b].sync.line);
    enter_node(st, b, tag, line);
}

pub fn synctex_vlist(st: &mut EngineState, env: &mut EngineEnv, this_box: NodePtr<BoxNode>) {
    if !synctex_active(st) {
        return;
    }
    enter_box(st, this_box);
    st.synctex.flags.not_void = true;
    st.synctex.record_box(env, &st.mem, '[', this_box);
}

pub fn synctex_tsilv(st: &mut EngineState, env: &mut EngineEnv, this_box: NodePtr<BoxNode>) {
    if !synctex_active(st) {
        return;
    }
    enter_box(st, this_box);
    if let Some(index) = &mut st.synctex.preview_index {
        index.close(RecordKind::VBoxEnd);
    }
    st.synctex.record(env, "]\n");
}

pub fn synctex_void_vlist(st: &mut EngineState, env: &mut EngineEnv, p: NodePtr<BoxNode>) {
    if !synctex_active(st) {
        return;
    }
    enter_box(st, p);
    st.synctex.record_box(env, &st.mem, 'v', p);
}

pub fn synctex_hlist(st: &mut EngineState, env: &mut EngineEnv, this_box: NodePtr<BoxNode>) {
    if !synctex_active(st) {
        return;
    }
    enter_box(st, this_box);
    st.synctex.flags.not_void = true;
    st.synctex.record_box(env, &st.mem, '(', this_box);
}

pub fn synctex_tsilh(st: &mut EngineState, env: &mut EngineEnv, this_box: NodePtr<BoxNode>) {
    if !synctex_active(st) {
        return;
    }
    enter_box(st, this_box);
    if let Some(index) = &mut st.synctex.preview_index {
        index.close(RecordKind::HBoxEnd);
    }
    if st.synctex.record(env, ")\n") {
        st.synctex.count += 1;
    }
}

fn synctex_ignored(st: &EngineState, tag: i32, line: i32) -> bool {
    st.synctex.flags.off || !st.eqtb.synctex_enabled() || tag <= 0 || line <= 0
}

fn flush_deferred_kern(st: &mut EngineState, env: &mut EngineEnv) {
    if let Some(kern) = st.synctex.deferred_kern {
        st.synctex.record_kern(env, &st.mem, kern);
    }
}

impl SynctexCtxt {
    fn context_changed(&self, tag: i32, line: i32) -> bool {
        self.node.is_none() || tag != self.tag || line != self.line
    }
}

pub fn synctex_void_hlist(st: &mut EngineState, env: &mut EngineEnv, p: NodePtr<BoxNode>) {
    let (tag, line) = (st.mem[p].sync.tag, st.mem[p].sync.line);
    if synctex_ignored(st, tag, line) {
        return;
    }
    flush_deferred_kern(st, env);
    enter_node(st, p, tag, line);
    st.synctex.record_box(env, &st.mem, 'h', p);
}

pub fn synctex_math(st: &mut EngineState, env: &mut EngineEnv, p: NodePtr<MathNode>) {
    if !synctex_active(st) {
        return;
    }
    let (tag, line) = (st.mem[p].sync.tag, st.mem[p].sync.line);
    // A kern deferred in an unchanged context is dropped.
    if st.synctex.context_changed(tag, line) {
        flush_deferred_kern(st, env);
    }
    enter_node(st, p, tag, line);
    st.synctex.record_node(env, '$', tag, line, &[]);
}

/// The one polymorphic entry: `hlist_out` may have retyped a glue node to a
/// kern in place before this call, so the node is classified here.
/// Record rule or glue `p` as shipped, with `rule` the resolved size of a rule.
pub fn synctex_horizontal_rule_or_glue(
    st: &mut EngineState,
    env: &mut EngineEnv,
    p: impl MemAddr,
    rule: Option<RuleDims>,
) {
    let p = p.addr();
    let node = st.mem.classify(p);
    let (tag, line) = match node {
        Ok(AnyNode::Rule(r)) => (st.mem[r].sync.tag, st.mem[r].sync.line),
        Ok(AnyNode::Glue(g)) => (st.mem[g].sync.tag, st.mem[g].sync.line),
        Ok(AnyNode::Kern(k)) => (st.mem[k].sync.tag, st.mem[k].sync.line),
        _ => {
            issue_error(
                env,
                &format!("unknown node type {} in SyncTeX", st.mem.type_code(p)),
            );
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
            env,
            'r',
            tag,
            line,
            &[rule.wd.raw(), rule.ht.raw(), rule.dp.raw()],
        ),
        (Ok(AnyNode::Glue(_)), _) => synctex.record_node(env, 'g', tag, line, &[]),
        (Ok(AnyNode::Kern(k)), _) => synctex.record_kern(env, &st.mem, k),
        _ => {}
    }
}

pub fn synctex_kern(
    st: &mut EngineState,
    env: &mut EngineEnv,
    p: NodePtr<KernNode>,
    this_box: NodePtr<BoxNode>,
) {
    let (tag, line) = (st.mem[p].sync.tag, st.mem[p].sync.line);
    if synctex_ignored(st, tag, line) {
        return;
    }
    let changed = st.synctex.context_changed(tag, line);
    if changed {
        flush_deferred_kern(st, env);
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
        synctex.record_kern(env, &st.mem, p);
    }
}

pub fn synctex_current(st: &mut EngineState, env: &mut EngineEnv) {
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
    let len = synctex_write(env, synctex.file, &s);
    synctex.lastv = curv;
    if len > 0 {
        synctex.total_length += len;
    } else {
        synctex.abort(env);
    }
}

#[derive(Default, Clone)]
pub(crate) struct SynctexFlags {
    pub content_ready: bool,
    pub off: bool,
    pub not_void: bool,
    pub warn: bool,
    pub output_p: bool,
}

#[derive(Clone)]
pub(crate) struct SynctexCtxt {
    pub file: usize,
    pub root_name: Option<Vec<u8>>,
    pub count: i32,
    pub node: MemPtr,
    /// A kern whose record waits for the context to change. When set it equals
    /// `node`, so flush sites record it before reassigning `node`.
    pub deferred_kern: Option<NodePtr<KernNode>>,
    pub tag: i32,
    pub line: i32,
    pub curh: i32,
    pub curv: i32,
    pub magnification: i32,
    pub unit: i32,
    pub total_length: i32,
    pub lastv: i32,
    pub form_depth: i32,
    pub synctex_tag_counter: u32,
    pub flags: SynctexFlags,
    pub preview_index: Option<PreviewSyncIndex>,
}

impl Default for SynctexCtxt {
    fn default() -> Self {
        Self {
            file: INVALID_HANDLE,
            root_name: None,
            count: 0,
            node: None,
            deferred_kern: None,
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
            flags: SynctexFlags::default(),
            preview_index: None,
        }
    }
}
