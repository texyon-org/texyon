I have verified all the anchors I need. Writing the spec now as my final response.

---

# SPEC 3 — Checkpoint-and-Replay Incremental Engine + Lifecycle Refactor

**Target:** `/home/yendric/Code/tectonic/crates/engine_xetex` (parent driver at `../../src/driver.rs`)
**Goal:** a Typst-style live-preview loop built on cheap `EngineState` snapshot/restore. **No core typesetting algorithm is rewritten** — we change *when* work runs and *how state is captured/restored*.
**Bar:** byte-identical XDV vs. the batch baseline on the corpus dashboard, plus new incremental-equivalence tests.

All `file:line` anchors below were opened and confirmed against the current tree.

---

## 0. Verified ground-truth map (re-checked)

| Fact | Anchor |
|---|---|
| `EngineState` struct, derives **only `Default`** (no `Clone`) | `state.rs:7-24` |
| `tt_run_engine` entry, owns `Box<EngineState>` | `engine.rs:943`, box at `engine.rs:952` |
| `apply_config` copies host config into the box | `engine.rs:953` |
| `PtrGuard` + `set_engine_state_ptr` (per-run locator) | `engine.rs:957-964`, install at `963` |
| Per-run arena allocation (buffer/nest/save/input/hyph) | `engine.rs:999-1060` |
| initex mem/eqtb/strings/font_info alloc | `engine.rs:1062-1111` |
| font tables + `refresh_font_ffi_mirrors` | `engine.rs:1308-1349` |
| `random_seed = microseconds*1000 + epochseconds%1e6` (**gettimeofday-derived, NOT build_date**) | `engine.rs:1399`; `init_randoms` at `1401` |
| `get_seconds_and_micros` → `libc::gettimeofday` | `texmfmp.rs:90-99` |
| `main_control_in` call site (Result boundary) | `engine.rs:1426` |
| `final_cleanup_in` / `close_files_and_terminate_in` / `tt_cleanup_in` | `engine.rs:1432/1439/1442` |
| `tt_cleanup_in`: frees font engines, input files, nulls all arenas | `engine.rs:1446-1560` (font free `1453-1464`) |
| `main_control_in` fn | `main_control.rs:6138` |
| Main loop `loop {` / `get_x_token_in` / `'reswitch` / tracing / mode read | `main_control.rs:6263 / 6264 / 6266 / 6267 / 6271` |
| **"the only way out"** `(VMODE, STOP) → its_all_over_in` | `main_control.rs:6323-6326` |
| `its_all_over_in` quiescence test `page_tail==PAGE_HEAD && head==tail && dead_cycles==0` | `main_control.rs:1358`, test at `1362-1366` |
| `build_page_in` reads `CONTRIB_HEAD` | `pagebuilder.rs:670-676` |
| `store_fmt_file_in` refuses `save_ptr != 0` ("can't dump inside a group") | `format_io.rs:215`, check `218` |
| `MemState` fields (`data,lo_mem_max,hi_mem_min,avail,mem_end,rover,var_used,dyn_used,line,cur_synctex_tag,temp_ptr`) | `state.rs:450-472` |
| Arena write accessors (`set_b32s1` etc.) | `state.rs:527-611` (`set_b32s1` = `539`) |
| `get_node` + synctex trailer stamp (`s>=MEDIUM_NODE_SIZE`) | `mem.rs:132`, stamp `214-217` |
| `get_avail` | `mem.rs:50` |
| `SaveStackState` (`save_ptr,max_save_stack,save_size,cur_level,cur_group,cur_boundary,data`) | `state.rs:679-691` |
| `InputClusterState` (`cur_list 206, nest 204, nest_ptr 215, input_ptr 219, cur_input 188, force_eof 241`) | `state.rs:149-271` |
| `ListStateRecord` (`mode,head,tail,e_tex_aux,prev_graf,mode_line,aux`), size 32 | `mem_types.rs:90-101` |
| `InputState`/`cur_input` (`state,index,start,loc,limit,name,synctex_tag`) | `globals.rs:40-48` |
| `ShipoutState` (`page_tail 1318, page_contents 1320, page_so_far 1321, last_glue 1322, dead_cycles 1317, output_active 1327, best_page_break 1336, least_page_cost 1338, page_max_depth 1339` + DVI state `1340-1371`) | `state.rs:1305-1371` |
| `SynctexCtxt` (`file,root_name,tag,line,synctex_tag_counter`) | `state.rs:79-96` |
| `TexmfState` (`last_source_name,last_lineno`) | `state.rs:131-136` |
| `FontState` FFI (`loaded_font_mapping 897, font_layout_engine 903, font_mapping 907, fmem_ptr 916, font_ptr 919`) | `state.rs:896-919` |
| `refresh_font_ffi_mirrors` + `FONT_*` statics | `globals.rs:249-255`, statics `241-243` |
| Driver `read_digest/write_digest`, deterministic `mtime_override`, `build_date` | `driver.rs:96/100`, `1861-1869`, `1877` |
| Constants: `CONTRIB_HEAD = MEM_TOP-1`, `PAGE_HEAD = MEM_TOP-2`, `MEM_TOP = 4_999_999`, `LEVEL_ONE = 1`, `MEDIUM_NODE_SIZE = 3` | `consts.rs:97/146/141/17`; `PAGE_HEAD` at `main_control.rs:44` |
| save/global machinery (`new_save_level 68, eq_save 103, eq_word_define 135, geq_word_define 163 → LEVEL_ONE, unsave_in 716`) | `save.rs` |

**Design-critical consequences of the ground truth**

1. **The arena is Rust-owned end to end.** Every `mem[]` mutation funnels through `MemState` accessors (`set_b32s1` alone has 808 call sites); no live C algorithm writes `mem[]/eqtb/font_info`. So a snapshot/restore that copies `MemState`/`EqtbState` is *complete* — nothing behind our back mutates the arena mid-run. This is what makes the whole plan viable.
2. **`EngineState` deliberately has no `Clone`** (`state.rs:7`). We must **not** add `derive(Clone)`: it holds raw FFI pointers (`font_layout_engine`, `input_file`, `dvi_file`, `root_name`, `last_source_name`, `name_of_input_file`) whose bitwise copy would double-free at teardown (`engine.rs:1453-1464,1483`). The snapshot is a **hand-written partial capture** of the *value* sub-state only.
3. **The arena is huge but sparse.** `mem.data` is `MEM_TOP+2 ≈ 5·10⁶` words × 8 bytes = **40 MB**; `eqtb.data` is `EQTB_SIZE + hash_extra(600_000)` words. A naive full copy per checkpoint is unacceptable. But the arena is bipartite: live lo-mem is `[0..=lo_mem_max]`, live hi-mem is `[hi_mem_min..=mem_end]`, and the middle is unallocated (`get_node`/`get_avail` grow the two ends toward each other, `mem.rs:60-64,183-206`). A checkpoint only needs the two live ranges.
4. **`random_seed` is non-deterministic by construction** (`engine.rs:1399`, gettimeofday) and is *not* covered by the driver's deterministic mode (which only overrides mtime + absolute paths, `driver.rs:1861-1869`). For incremental replay this is actually a **help**: the seed + `ErrorState.randoms[55]` + `j_random` live inside the snapshot, so a restore reproduces the *original run's* RNG stream. The rule is simply: **never re-run `init_randoms` on restore.**

---

## Part A — Lifecycle refactor: `ResidentEngine`

### A.1 Problem

`tt_run_engine` (`engine.rs:943`) is a one-shot: it allocates a fresh `Box<EngineState>` (`952`), installs the locator (`963`), loads the format (`engine.rs:1254`), runs `main_control_in` once (`1426`), then unconditionally tears everything down via `final_cleanup_in`/`close_files_and_terminate_in`/`tt_cleanup_in` (`1432-1442`) and drops the box. The whole `TRE_*`-sizing + arena-alloc + format-load prologue (`engine.rs:966-1424`, ~450 lines) runs on *every* compile. Format load alone re-reads and re-parses megabytes.

The `EngineAbort`/`Result` boundary is **already** exactly what we need: `main_control_in` returns `Result<(), EngineAbort>` and the caller catches it (`engine.rs:1426`, `.map_err(into_engine_abort)`), turning it into a history code via `into_history`. Today that's *process-final*; we make it *per-compile*.

### A.2 Split into three phases

```
init-once (expensive, ~once per editor session)
  └─ alloc TRE_* arenas + load_fmt  ->  a warm ResidentEngine{ st: Box<EngineState> }
per-recompile (cheap)
  ├─ checkpoint(): capture Snapshot at a quiescent boundary
  ├─ compile():    drive main_control to end-of-body
  └─ restore_and_replay(edit): restore nearest Snapshot, replay tail
teardown (once, on editor close)
  └─ close_files_and_terminate + tt_cleanup
```

### A.3 What moves out of `tt_run_engine`

Refactor `tt_run_engine` into four `pub(crate)` functions on `EngineState` (keep the `extern "C" tt_run_engine` as a thin shim that calls them in sequence, preserving the batch path byte-for-byte). New file: `src/resident.rs`.

- **`engine_init_once(st, dump_name, build_date, input_file_name)`** — everything in `engine.rs:966-1425` *except* the terminal steps, i.e. `TRE_*` sizing, arena alloc, `initialize_more_variables_in`, format load (`1254`), date/time seeding (`1271-1286`), font-table bootstrap, `init_randoms` (`1401`), `start_input_in` (`1417`). Ends with the engine parked at `history = HISTORY_SPOTLESS` and `cur_input` positioned at the top of the primary input — **exactly the state at `engine.rs:1425`, one line before `main_control_in`.**
- **`engine_run_to_end(st) -> Result<(), EngineAbort>`** — the current `engine.rs:1426` call, but *without* the trailing `final_cleanup_in`. We stop `main_control` at end-of-body and keep the engine warm (see A.5).
- **`engine_finish(st)`** — `final_cleanup_in` + `close_files_and_terminate_in` (`engine.rs:1432,1439`): emit the XDV, close files. Idempotent-guarded so we can call it per-compile (each compile emits a fresh XDV) *without* freeing arenas.
- **`engine_teardown(st)`** — `tt_cleanup_in` (`engine.rs:1442`), the FFI free + arena release, run once at session end.

```rust
// src/resident.rs
pub struct ResidentEngine {
    st: Box<crate::state::EngineState>,
    ladder: CheckpointLadder,
    // Re-established every time we hand a &mut *st across an FFI/main_control boundary.
    _locator_installed: bool,
}

impl ResidentEngine {
    /// Runs the expensive prologue ONCE. Mirrors engine.rs:952-1425.
    pub unsafe fn boot(
        dump_name: *const libc::c_char,
        input_file_name: *const libc::c_char,
        build_date: libc::time_t,
    ) -> Result<ResidentEngine, i32> {
        let mut st = Box::new(crate::state::EngineState::default());
        crate::engine_interface::apply_config(&mut st.config);   // engine.rs:953
        crate::state::set_engine_state_ptr(&mut *st as *mut _);  // engine.rs:963
        engine_init_once(&mut st, dump_name, build_date, input_file_name)
            .map_err(|a| a.into_history(&mut st))?;
        Ok(ResidentEngine { st, ladder: CheckpointLadder::new(), _locator_installed: true })
    }

    /// Cheap per-recompile boundary. EngineAbort is now RECOVERABLE:
    /// a fatal in one compile does not tear down the resident arenas.
    pub unsafe fn compile(&mut self) -> i32 {
        // (re)install the locator: the &mut *st address is stable (Box), but a
        // between-compile host call may have cleared it. See globals invariant.
        crate::state::set_engine_state_ptr(&mut *self.st as *mut _);
        let r = engine_run_to_end(&mut self.st)
            .and_then(|()| engine_finish(&mut self.st));
        match r {
            Ok(()) => self.st.errors.history,
            Err(a) => a.into_history_recoverable(&mut self.st), // see A.4
        }
    }
}

impl Drop for ResidentEngine {
    fn drop(&mut self) {
        unsafe {
            crate::engine::engine_teardown(&mut self.st);   // tt_cleanup_in
            crate::state::set_engine_state_ptr(std::ptr::null_mut());
        }
    }
}
```

### A.4 Making `EngineAbort` recoverable per-compile

`into_history` (errors.rs) currently runs the *terminal* cleanup (`close_files_and_terminate`, `tt_cleanup` via `post_error_message`) and returns `HISTORY_FATAL_ERROR`. For the resident model we add a sibling `into_history_recoverable` that:

- runs the **diagnostic/print** half (error message, log flush) — unchanged;
- runs `close_files_and_terminate_in` (flush the partial XDV/log) — but **not** `tt_cleanup_in` (do not free the arenas);
- leaves `st` in a *defined* state so the next `restore_and_replay` can overwrite it wholesale.

**Proof obligation A-1 (abort leaves restorable state):** after a recoverable abort, the arena/save/nest scalars may be inconsistent, but the *next* action is always a `Snapshot::restore` which overwrites every value field. So the only requirement is that the **FFI-owning fields** (font handles, open `input_file`s, `dvi_file`) are not left double-owned. `close_files_and_terminate_in` closes the DVI/log; open input files are closed by the restore path (see C.3). Fonts loaded before the abort are retained (they're in the snapshot's font high-water; see B.4). **Failure mode:** if an abort fires mid-`load_native_font` after `font_layout_engine[f]` is set but before `font_ptr` is bumped, the half-loaded slot leaks until teardown — acceptable (bounded, freed at `engine_teardown`), but assert `font_ptr` monotonicity in debug.

### A.5 Parking `main_control` at end-of-body without tearing down

Today the loop exits only via `(VMODE, STOP) → its_all_over_in → Ok(true) → return Ok(())` (`main_control.rs:6323-6326`, `1366`). `its_all_over` fires `\end`/`\dump`. For the resident model the primary input's `\end` (or `\bye`) still drives us here; the difference is we **do not** call `tt_cleanup`. `engine_finish` emits the XDV and returns; `st` stays warm. The `cur_input` is at EOF but that's fine — the next compile restores a snapshot that repositions it.

---

## Part B — The checkpoint ladder

### B.1 Data structure

```rust
// src/resident.rs
pub struct Checkpoint {
    /// Byte offset into the editor's source buffer this checkpoint precedes.
    /// The invariant: replaying from here reproduces the doc from src_pos to EOF.
    src_pos: usize,
    /// Source line (mem.line at capture) — used for "nearest preceding" lookup.
    src_line: i32,
    snap: Snapshot,
}

pub struct CheckpointLadder {
    /// Ordered by src_pos ascending. checkpoints[0] is always the post-preamble one.
    checkpoints: Vec<Checkpoint>,
    /// The rolling "immediately-before-current-paragraph" checkpoint used ONLY by
    /// the O(1) fast path (Part D). Overwritten at every outer-vmode par boundary.
    para_local: Option<Checkpoint>,
}
```

### B.2 When a boundary is snapshot-safe

A snapshot is only sound at a **quiescent boundary**, defined identically to what the format dump requires (`format_io.rs:218`): `save_ptr == 0` (not inside any group). We strengthen it to the page/nest-quiescent condition already used by `its_all_over` (`main_control.rs:1362-1366`):

```rust
fn is_quiescent(st: &EngineState) -> bool {
    st.save_stack.save_ptr == 0                       // not inside a group (format_io.rs:218)
        && st.save_stack.cur_level == LEVEL_ONE       // consts.rs:141
        && st.input.nest_ptr == 0                     // outer vertical mode only
        && st.input.cur_list.mode == 1                // VMODE (main-vertical)
        && !st.shipout.output_active                  // not inside \output
}
```

Note we do **not** require `page_tail == PAGE_HEAD`: the page builder legitimately holds material between paragraphs. The snapshot captures the page/contrib state too (B.3), so a mid-page boundary is fine as long as we're in outer vmode with an empty save stack.

### B.3 Where in the main loop to snapshot

The natural hook is the top of the main-control loop (`main_control.rs:6263-6271`), *after* `get_x_token_in` and the mode read, when the just-scanned command is a **structural** one at outer vmode. Two policies:

- **Post-preamble checkpoint (mandatory, `checkpoints[0]`):** taken the first time `is_quiescent` holds *and* the primary body has begun. Detect "preamble done" heuristically: the first `\shipout`-free outer-vmode boundary after the first paragraph, or explicitly the moment the host marks `\begin{document}` (LaTeX) via a sentinel primitive. Simplest robust trigger: **the first time `is_quiescent()` becomes true after `mem.line` has advanced past the host-supplied `preamble_end_line`.**
- **Sectional checkpoints:** the host editor tells the engine "checkpoint here" by injecting a no-op primitive (e.g. `\TectonicCheckpoint`) that the front-end macros expand at every `\section`/`\chapter`/`\clearpage`. When `main_control` dispatches that primitive at a quiescent boundary, call `ladder.push(capture(st, cur_src_pos))`. This avoids guessing sectioning semantics inside the engine.

```rust
// new dispatch arm inside main_control_in's big switch (main_control.rs, near 6305)
(VMODE, TECTONIC_CHECKPOINT) => {
    if is_quiescent(st) {
        st.ladder_request = Some(st.mem.line); // deferred; ladder lives in ResidentEngine
    }
}
```
`main_control_in` sets a request flag on `EngineState`; `ResidentEngine::compile` drains it after the run returns to `&mut self` scope (avoids threading `&mut CheckpointLadder` through the whole main loop). Because captures happen at quiescent boundaries only, the flag can be honored lazily at the next re-entry to `ResidentEngine` scope **provided the state hasn't advanced** — so instead capture *synchronously* by giving the checkpoint hook a small callback. Concretely: pass a `checkpoint_sink: &mut dyn FnMut(&EngineState, i32)` into `main_control_in` (one new param) that calls `Snapshot::capture` immediately. This keeps the capture at the exact quiescent instant.

### B.4 `Snapshot`: the value capture (sparse arena)

```rust
// src/snapshot.rs
pub struct Snapshot {
    // --- MEM (sparse: only live ranges) ---
    mem_lo: Box<[MemoryWord]>,     // data[0 ..= lo_mem_max]
    mem_hi: Box<[MemoryWord]>,     // data[hi_mem_min ..= mem_end]
    lo_mem_max: i32, hi_mem_min: i32, avail: i32, mem_end: i32, rover: i32,
    var_used: i32, dyn_used: i32,
    mem_line: i32, cur_synctex_tag: i32, temp_ptr: i32,     // state.rs:451-472

    // --- EQTB (sparse: only [0 ..= eqtb high-water]) + hash ---
    eqtb: Box<[MemoryWord]>,       // eqtb.data[0 ..= eqtb_live_top]
    hash: Box<[B32x2]>,            // eqtb.hash (append-only; capture used prefix)
    eqtb_scalars: EqtbScalars,     // hash_used, hash_high, cs_count, prim_used, ...

    // --- SAVE STACK ---
    save_ptr: i32, cur_level: u16, cur_group: GroupCode, cur_boundary: i32,
    max_save_stack: i32,
    save_data: Box<[MemoryWord]>,  // save_stack.data[0 ..= save_ptr]  (0 at quiescent)

    // --- SEMANTIC NEST ---
    cur_list: ListStateRecord, nest_ptr: i32, max_nest_stack: i32,
    nest: Box<[ListStateRecord]>,  // nest[0 ..= nest_ptr]

    // --- INPUT (position only; buffers are re-fed from the editor, see C.3) ---
    cur_input: InputState,         // state,index,start,loc,limit,name,synctex_tag
    input_ptr: i32, in_open: i32, open_parens: i32,
    input_stack: Box<[InputState]>,// [0 ..= input_ptr]
    force_eof: bool, first: i32, last: i32,

    // --- STRINGS (append-only within a run) ---
    str_ptr: i32, pool_ptr: i32,   // high-water only; see B.5

    // --- FONTS (append-only within a run) ---
    font_ptr: i32, fmem_ptr: i32,  // high-water only; see B.4-fonts

    // --- SHIPOUT / PAGE BUILDER / DVI ---
    page_tail: i32, page_contents: u8, page_so_far: [i32;8], last_glue: i32,
    last_penalty: i32, last_kern: i32, last_node_type: i32, insert_penalties: i32,
    dead_cycles: i32, best_page_break: i32, best_size: i32, least_page_cost: i32,
    page_max_depth: i32,
    // DVI stream position (so a re-emit continues coherently OR is reset; see D/validation)
    total_pages: i32, dvi_ptr: i32, dvi_offset: i32, dvi_gone: i32,
    cur_s: i32, max_v: i32, max_h: i32, max_push: i32, last_bop: i32,

    // --- SYNCTEX ---
    synctex_tag: i32, synctex_line: i32, synctex_tag_counter: u32,

    // --- RNG (do NOT re-seed on restore) ---
    randoms: [i32;55], j_random: u8, random_seed: i32,

    // --- HYPH exceptions (\hyphenation adds entries; append-only) ---
    hyph_count: i32, hyph_next: i32,   // high-water; see E
}
```

**Sparse-arena rationale:** `mem_lo` copies `data[0..=lo_mem_max]`, `mem_hi` copies `data[hi_mem_min..=mem_end]`. For a typical page-sized quiescent boundary this is a few thousand words, not 5·10⁶. Restore writes both ranges back and resets the five frontier scalars; the untouched middle band is *definitionally free* in both the snapshot and the live arena, so it need not be copied. **Proof obligation B-1:** the free-list doubly-linked structure (`rover` chain) lives entirely within `[0..=lo_mem_max]`; restoring that range + `rover` restores the free list exactly. Verified against `get_node`'s free-list walk (`mem.rs:133-176`), which only touches indices `< lo_mem_max`.

**Append-only sub-states (strings/fonts/hyph):** the string pool (`str_ptr`/`pool_ptr`), font tables (`font_ptr`/`fmem_ptr`), and hyphenation exceptions only *grow* during a run. So the snapshot stores only the **high-water marks**; restore **truncates** back to them (see C.2). This avoids copying multi-megabyte `str_pool`/`font_info` on every checkpoint. **Proof obligation B-2:** nothing rewrites an existing string/font slot below the high-water (strings are immutable once made — `make_string` only appends, `string_pool.rs`; font metrics are write-once at load). Truncation is therefore the exact inverse of the appends done during replay.

### B.5 Cost

Checkpoint cost = O(live lo-mem + live hi-mem + live eqtb prefix + nest depth + input depth). For a mid-document boundary that is typically tens of KB, **not** the 40 MB full arena. Sectional checkpoints every few pages keep the ladder bounded (a 300-page doc ≈ 60 checkpoints × ~64 KB ≈ 4 MB).

---

## Part C — Restore + forward replay (the general path)

### C.1 Entry

```rust
impl ResidentEngine {
    /// Edit landed at byte offset `edit_pos` in the source buffer `src`.
    /// Returns the new history code.
    pub unsafe fn restore_and_replay(&mut self, src: &[u8], edit_pos: usize) -> i32 {
        let cp = self.ladder.nearest_before(edit_pos);   // binary search on src_pos
        cp.snap.restore(&mut self.st);                   // C.2
        self.reopen_input_from_buffer(src, cp.src_pos);  // C.3
        crate::state::set_engine_state_ptr(&mut *self.st as *mut _);
        let r = engine_run_to_end(&mut self.st)          // replays cp.src_pos..EOF
            .and_then(|()| engine_finish(&mut self.st));
        // Rebuild the ladder tail: everything at src_pos > cp.src_pos is now stale.
        self.ladder.truncate_after(cp.src_pos);
        match r { Ok(()) => self.st.errors.history,
                  Err(a) => a.into_history_recoverable(&mut self.st) }
    }
}
```

Cost = **O(tail length after the checkpoint)**, independent of preamble/format-load, and independent of the document *before* the checkpoint.

### C.2 `Snapshot::restore`

```rust
impl Snapshot {
    pub fn restore(&self, st: &mut EngineState) {
        // 1. MEM: overwrite the two live ranges, reset frontier scalars.
        st.mem.data[0..=self.lo_mem_max as usize]
            .copy_from_slice(&self.mem_lo);
        st.mem.data[self.hi_mem_min as usize..=self.mem_end as usize]
            .copy_from_slice(&self.mem_hi);
        st.mem.lo_mem_max = self.lo_mem_max;
        st.mem.hi_mem_min = self.hi_mem_min;
        st.mem.avail = self.avail;
        st.mem.mem_end = self.mem_end;
        st.mem.rover = self.rover;
        st.mem.var_used = self.var_used;
        st.mem.dyn_used = self.dyn_used;
        st.mem.line = self.mem_line;
        st.mem.cur_synctex_tag = self.cur_synctex_tag;
        st.mem.temp_ptr = self.temp_ptr;

        // 2. EQTB: overwrite live prefix + hash prefix; restore scalars.
        st.eqtb.data[0..self.eqtb.len()].copy_from_slice(&self.eqtb);
        st.eqtb.hash[0..self.hash.len()].copy_from_slice(&self.hash);
        self.eqtb_scalars.apply(&mut st.eqtb);

        // 3. SAVE STACK
        st.save_stack.save_ptr = self.save_ptr;         // == 0 at a real checkpoint
        st.save_stack.cur_level = self.cur_level;
        st.save_stack.cur_group = self.cur_group;
        st.save_stack.cur_boundary = self.cur_boundary;

        // 4. SEMANTIC NEST
        st.input.cur_list = self.cur_list;
        st.input.nest_ptr = self.nest_ptr;
        st.input.nest[0..=self.nest_ptr as usize].copy_from_slice(&self.nest);

        // 5. STRINGS / FONTS / HYPH: truncate append-only tables to high-water.
        Self::truncate_strings(st, self.str_ptr, self.pool_ptr);
        Self::truncate_fonts(st, self.font_ptr, self.fmem_ptr);   // frees engines > font_ptr!
        st.hyph.hyph_count = self.hyph_count;
        st.hyph.hyph_next  = self.hyph_next;

        // 6. SHIPOUT / PAGE BUILDER / DVI position
        st.shipout.page_tail = self.page_tail;
        st.shipout.page_contents = self.page_contents;
        st.shipout.page_so_far = self.page_so_far;
        st.shipout.last_glue = self.last_glue;
        st.shipout.dead_cycles = self.dead_cycles;
        st.shipout.best_page_break = self.best_page_break;
        st.shipout.least_page_cost = self.least_page_cost;
        st.shipout.page_max_depth = self.page_max_depth;
        /* ... last_penalty/last_kern/last_node_type/insert_penalties ... */
        // DVI: see validation §D — either continue the stream or rewind + re-emit.

        // 7. SYNCTEX + RNG (RNG is restored, NEVER re-seeded)
        st.synctex.tag = self.synctex_tag;
        st.synctex.line = self.synctex_line;
        st.synctex.synctex_tag_counter = self.synctex_tag_counter;
        st.errors.randoms = self.randoms;
        st.errors.j_random = self.j_random;
        st.errors.random_seed = self.random_seed;

        crate::globals::refresh_font_ffi_mirrors(&st.fonts); // globals.rs:249
    }
}
```

**`truncate_fonts` is the one FFI-aware step.** Fonts loaded *after* the checkpoint own `XeTeXLayoutEngine`/`TECkit_Converter` handles (`state.rs:903,907`). Truncating `font_ptr` back must release them exactly as `tt_cleanup_in` does (`engine.rs:1453-1464`):

```rust
fn truncate_fonts(st: &mut EngineState, font_ptr: i32, fmem_ptr: i32) {
    for f in (font_ptr + 1)..=st.fonts.font_ptr {
        let eng = st.fonts.font_layout_engine[f as usize];
        if !eng.is_null() {
            crate::ext::release_font_engine(eng, st.fonts.font_area[f as usize]); // engine.rs:1456
            st.fonts.font_layout_engine[f as usize] = std::ptr::null_mut();
        }
        let map = st.fonts.font_mapping[f as usize];
        if !map.is_null() {
            crate::engine::tt_cleanup_TECkit_DisposeConverter(map);              // engine.rs:1461
            st.fonts.font_mapping[f as usize] = std::ptr::null_mut();
        }
    }
    st.fonts.font_ptr = font_ptr;
    st.fonts.fmem_ptr = fmem_ptr;
}
```

**Proof obligation C-1 (no dangling font refs in restored mem):** the restored `mem`/`eqtb` are from *before* those fonts were loaded, so no restored node references a font slot `> font_ptr`. Verified by construction: font references in nodes (char nodes' font field, `\setfont` in eqtb) can only name fonts that existed when the node was created, and the node ranges we restore predate the load. **Failure mode:** if a font loaded before the checkpoint is *also* re-requested during replay, `load_native_font_in` dedups within a run (`fonts.rs:1110`) — but after restore the dedup cache must be consistent with the truncated table. `truncate_fonts` must also invalidate any dedup index entry `> font_ptr` (add: reset the native-font-dedup map to entries `<= font_ptr`).

### C.3 Feeding replay from the in-memory editor buffer

The replay must **not** re-open the file. It re-drives `main_control` reading from the editor's buffer starting at `cp.src_pos`. The mechanism reuses the existing input machinery:

- The editor keeps the whole source as a `Vec<u8>` (or rope). On restore we point `cur_input` at it. TeX reads a *line at a time* into `st.input.buffer` (the `input_line` path). We install a **memory-backed `UFile`**: a `UFile` variant whose `read` pulls the next line from the editor buffer at a byte cursor.

```rust
// io.rs: add a memory-backed input source
pub(crate) enum UFileSrc {
    Real(/* existing file handle */),
    MemBuf { data: Rc<[u8]>, cursor: usize },  // cursor starts at cp.src_pos
}
```

- `reopen_input_from_buffer(src, src_pos)` synthesizes the input stack the checkpoint expects: it sets `cur_input.name`, `.state`, `.index`, `.limit`, `.loc`, `.start` from the snapshot's `cur_input`, and installs a `MemBuf` whose `cursor = src_pos` so the *next* `input_line` fetches the line after the checkpoint. Because the checkpoint is at a **line boundary in outer vmode**, `cur_input.loc == cur_input.limit` (line consumed) — the snapshot's `loc/limit` are for the just-finished line; the next read starts fresh at `src_pos`.

**Proof obligation C-2 (replay stream identity):** the token stream produced from `MemBuf{cursor=src_pos}` must be byte-identical to what the batch run read from the file at the same offset. This holds iff (a) the editor buffer equals the on-disk file except for the localized edit, and (b) line-splitting (`\endlinechar` handling, `engine.rs:1264-1269`) is identical. Guarantee (b) by using the *same* `input_line` code path (`io.rs`), only swapping the byte source. **Failure mode:** trailing-whitespace stripping and `\r\n` normalization must match `u_open_in`/`input_line` exactly — cover with a golden test that feeds a file both via `Real` and `MemBuf` and asserts identical `buffer` contents line-by-line.

### C.4 Why the general path is always sound

Restore reproduces the **entire** machine value-state as of `cp.src_pos` (mem, eqtb, save, nest, page, strings/fonts/hyph high-water, RNG, synctex). Replaying the identical token tail is then *the same computation the batch engine performed*, so the XDV tail is byte-identical. The only non-value inputs are (i) RNG — restored, not re-seeded; (ii) date/time eqtb params (`\time`,`\day`,…) — set once at boot (`engine.rs:1281`) and captured in the eqtb prefix; (iii) file mtimes — same across compiles. **This is the safe fallback that every fast path degrades to.**

---

## Part D — The O(1) same-paragraph fast path

### D.1 Scenario (and its strict precondition)

The user edits a single paragraph P that the engine has **just built** and has **not yet advanced past** (the live cursor is at the outer-vmode boundary immediately after P, before any following paragraph has been processed). We want to rebuild only P.

This requires a checkpoint **immediately before P** — the `ladder.para_local` rolling checkpoint (B.1), captured at the outer-vmode boundary that *starts* P. The fast path is:

1. restore the arena/save/nest/page frontier to `para_local`;
2. re-feed P's (edited) tokens;
3. re-run `main_control` until the next outer-vmode boundary;
4. if P's contribution is state-neutral (Part E), splice and **stop** — O(|P|), not O(tail).

Crucially, **eqtb is *not* in the fast-path rewind set** because at the moment we take `para_local`, and at the moment we rewind to it, nothing after P has executed — so eqtb is *already* exactly "as of the start of P." That is the entire reason the fast path can skip the (expensive) eqtb/strings/font restore.

### D.2 The exact rewind set (each field verified)

```rust
/// O(1) rewind to the start of the current paragraph. `pl` is the para_local
/// checkpoint captured at the paragraph's opening outer-vmode boundary.
/// PRECONDITION (asserted): engine has not advanced past this paragraph, i.e.
/// eqtb/strings/fonts/hyph are byte-identical to pl (debug-checked via hashes).
fn rewind_paragraph(st: &mut EngineState, pl: &ParaLocal) {
    // 1. MEM FRONTIER — discard every node P allocated. (state.rs:451-455; mem.rs:60-64,152)
    st.mem.lo_mem_max = pl.lo_mem_max;
    st.mem.hi_mem_min = pl.hi_mem_min;
    st.mem.rover      = pl.rover;
    st.mem.avail      = pl.avail;
    st.mem.mem_end    = pl.mem_end;
    st.mem.var_used   = pl.var_used;
    st.mem.dyn_used   = pl.dyn_used;
    // free-list head words in [0..lo_mem_max] must be restored too:
    st.mem.data[0..=pl.lo_mem_max as usize].copy_from_slice(&pl.mem_lo);
    st.mem.data[pl.hi_mem_min as usize..=pl.mem_end as usize].copy_from_slice(&pl.mem_hi);

    // 2. SAVE STACK — P opened/closed only balanced groups; back to level 1. (state.rs:680-686)
    st.save_stack.save_ptr     = pl.save_ptr;      // == 0
    st.save_stack.cur_level    = pl.cur_level;     // == LEVEL_ONE
    st.save_stack.cur_group    = pl.cur_group;
    st.save_stack.cur_boundary = pl.cur_boundary;

    // 3. SEMANTIC NEST — cur_list back to the empty outer-vmode list. (state.rs:206,215; mem_types.rs:90)
    st.input.cur_list = pl.cur_list;    // head/tail/prev_graf/prev_depth(aux.s1)/space_factor(aux.s0)/mode/mode_line
    st.input.nest_ptr = pl.nest_ptr;    // == 0

    // 4. PAGE BUILDER / CONTRIB LIST — *** MISSING THIS DOUBLE-APPENDS P ***. (state.rs:1318-1339; consts.rs:97)
    st.shipout.page_tail       = pl.page_tail;
    st.shipout.page_contents   = pl.page_contents;
    st.shipout.page_so_far     = pl.page_so_far;
    st.shipout.last_glue       = pl.last_glue;
    st.shipout.last_penalty    = pl.last_penalty;
    st.shipout.last_kern       = pl.last_kern;
    st.shipout.last_node_type  = pl.last_node_type;
    st.shipout.insert_penalties= pl.insert_penalties;
    st.shipout.best_page_break = pl.best_page_break;
    st.shipout.least_page_cost = pl.least_page_cost;
    st.shipout.page_max_depth  = pl.page_max_depth;
    // the contribution list head link lives in the arena at CONTRIB_HEAD (=MEM_TOP-1);
    // it is inside [hi_mem_min..mem_end]? No — CONTRIB_HEAD/PAGE_HEAD are fixed
    // high addresses (MEM_TOP-1/-2). Restore them explicitly:
    st.mem.set_b32s1(CONTRIB_HEAD, pl.contrib_head_link);   // consts.rs:97
    st.mem.set_b32s1(PAGE_HEAD,    pl.page_head_link);      // main_control.rs:44

    // 5. SYNCTEX counters — so re-emitted node tags match. (state.rs:85,86,94; mem.rs:215)
    st.mem.cur_synctex_tag        = pl.cur_synctex_tag;
    st.mem.line                   = pl.mem_line;
    st.synctex.synctex_tag_counter= pl.synctex_tag_counter;
    st.synctex.tag                = pl.synctex_tag;
    st.synctex.line               = pl.synctex_line;
}
```

**Field-by-field justification:**

- **mem frontier** (`lo_mem_max/hi_mem_min/rover/avail/mem_end`, `state.rs:451-455`): `get_node`/`get_avail` grow these; resetting them (plus the free-list head words in `[0..=lo_mem_max]`) makes every node P allocated free again. Without restoring the `[0..=lo_mem_max]` words, the `rover` free-list would be corrupt (P mutated free-block headers via `get_node`, `mem.rs:156-206`).
- **save stack** (`save_ptr/cur_level/cur_group/cur_boundary`, `state.rs:680-686`): P may open `{...}` groups; at the para boundary they're balanced (`save_ptr==0`), but mid-rebuild we start from level 1.
- **nest** (`cur_list` fields `head/tail/prev_graf/prev_depth/space_factor/mode/mode_line`, `mem_types.rs:90-98`; `nest_ptr`, `state.rs:215`): `prev_depth` is `aux.b32.s1`, `space_factor`/`prev_graf` per `ListStateRecord`. Resetting `cur_list` discards P's hlist under construction and restores inter-paragraph `prev_depth` — **essential**, since the *next* paragraph's baselineskip depends on P's `prev_depth`.
- **page builder / contrib list** (`page_tail/page_contents/page_so_far/page_head_link/contrib_head_link`, `state.rs:1318-1321` + `CONTRIB_HEAD` at `consts.rs:97`, `PAGE_HEAD` at `main_control.rs:44`): **the prompt's flagged trap.** `build_page_in` (`pagebuilder.rs:670`) moves the paragraph from `CONTRIB_HEAD` onto the page/`PAGE_HEAD` list. If we rewind the arena+nest but leave `page_tail`/the page-list links pointing at P's boxes, the rebuilt P is appended a *second* time → duplicate paragraph in output. Restoring `page_tail`, `page_so_far`, and both head links removes P from the page.
- **synctex** (`cur_synctex_tag`, `mem.line`, `synctex_tag_counter`, `state.rs:94`; stamp at `mem.rs:215`): `get_node` stamps `cur_synctex_tag`+`line` into every MEDIUM+ node's trailer (`mem.rs:214-217`). Re-allocated nodes must re-stamp identical tags, so the counter must rewind.

`ParaLocal` also snapshots the tiny `mem_lo`/`mem_hi` slices `[0..=lo_mem_max]` and `[hi_mem_min..=mem_end]` — for a single paragraph these are a few hundred words, so this is genuinely near-O(1).

### D.3 Splice-and-stop condition

After re-running P to the next boundary, the fast path terminates *without replaying the tail* only if **P's edit is provably state-neutral and dimension-neutral** — see Part E. Otherwise it escalates.

---

## Part E — Locality detection and the fallback ladder

### E.1 The three tiers

```
Tier 0  O(|P|)   rewind_paragraph + rebuild P + splice        [Part D]
Tier 1  O(tail)  restore nearest checkpoint + replay to EOF   [Part C]
Tier 2  O(doc)   full recompile (boot path unchanged)         [batch]
```

**Policy: err toward the slow path.** Any detected mutation, any uncertainty, any unhandled command class → escalate. A false escalation costs performance; a false fast-path corrupts output.

### E.2 What makes a paragraph edit non-local

A Tier-0 rebuild is sound **only if** re-typesetting the edited P, starting from `para_local`, produces:
1. **no escaping state mutation** — nothing that changes eqtb/strings/fonts/hyph in a way visible *after* P; and
2. **no downstream-visible dimension change** — P's contributed height/depth and `prev_depth` are identical, so page breaking of everything after P is unaffected.

Concretely, escalate to Tier 1 if replaying P executes any of:

| Trigger | Detection hook | Why it escapes |
|---|---|---|
| `\global` / `\gdef` / `\xdef` | `geq_define`/`geq_word_define` called with `LEVEL_ONE` (`save.rs:163`) | writes eqtb at level 1 → visible after P |
| level-1 counter/dimen/register write | `eq_word_define`/`eq_define` where `cur_level==LEVEL_ONE` (`save.rs:135-152`) | e.g. `\section` bumps `\c@section`; `\ref`/`\thepage` read it later |
| any string made | `str_ptr` advanced past `para_local.str_ptr` (`string_pool.rs`) | `\label`/`\write` aux; changes downstream string ids |
| new font loaded | `font_ptr` advanced (`fonts.rs:1110`) | append-only table diverges from tail's expectations |
| `\hyphenation`/`\patterns` | `hyph_count`/`hyph_next` advanced (`state.rs:1648,1650`) | changes later hyphenation |
| `\parshape` set | `par_shape_ptr` (eqtb `LOCAL_BASE` slot) non-null / changed | shape affects this and possibly following pars |
| `\insert` / footnote / `\mark` | contrib list gets an INS_NODE/MARK_NODE (`build_page_in` mark path, `pagebuilder.rs`) | inserts migrate to page/other pages |
| `\vadjust`, `\write`, `\special` whatsit | whatsit appended to P's list | side effects at ship-out |
| P's height/depth or `prev_depth` changed | compare rebuilt `cur_list.aux` + box dims to the pre-edit values | page breaks of the tail shift |

### E.3 Instrumentation for detection

Add a lightweight **mutation epoch** to the fast path. Before `rewind_paragraph`, snapshot the escaping counters:
```rust
struct EscapeGuard { str_ptr: i32, font_ptr: i32, hyph_count: i32, hyph_next: i32,
                     geq_writes: u64 /* bumped in geq_define/geq_word_define */ }
```
`save.rs`'s `geq_define`/`geq_word_define`/`eq_word_define@level1` increment `st.escape_epoch` (one new `u64` on `EngineState`). After rebuilding P, if any of `{str_ptr, font_ptr, hyph_count, hyph_next, escape_epoch}` moved, **or** the recomputed contributed height/depth ≠ the pre-edit values (compare the box the paragraph produced), escalate to Tier 1. Also escalate if P's rebuild hit the `\parshape`/`\insert`/`\mark`/whatsit code paths (set a `non_local` flag in those specific dispatch arms).

This is *conservative*: `escape_epoch` bumps even for `\global` writes that happen to be idempotent. That's fine — we err slow.

### E.4 Proof: the Tier-0 fast path is **unsound** from a post-preamble-only checkpoint for a paragraph after any state-mutating paragraph

**Claim.** Let `Q` be the post-preamble checkpoint (`ladder.checkpoints[0]`). Let paragraphs `P₁,…,P_{k-1},P_k` follow `Q`. Suppose some `P_j` (`j<k`) performed a state mutation `M` (e.g. `\stepcounter`, `\gdef`, a `\newfont`, or a `\label`). Then applying the **Part D rewind set** (mem frontier + save + nest + page + synctex, but **not** eqtb/strings/fonts) to jump from the live cursor "back to before `P_k`" using `Q` is unsound.

**Proof.**

*(i) Free-list dangling.* The Part D rewind resets `lo_mem_max/hi_mem_min/rover/avail/mem_end` and the `[0..=lo_mem_max]` free-list words to their values in `Q`. But between `Q` and `P_k`, `P₁..P_{k-1}` allocated nodes; some of those nodes are still **referenced from eqtb** — e.g. a box saved by `\setbox` (a `\global\setbox` or a box register written by `P_j`), a `\parshape` pointer, a token-list macro body created by `\gdef` (`M`). Those eqtb slots are **not** rewound by the Part D set, so they still point at arena indices in `(Q.lo_mem_max .. live.lo_mem_max]`. After the rewind those indices are in the *free* region and will be handed out by the next `get_node` (`mem.rs:152`), **aliasing live eqtb-referenced storage**. The subsequent typesetting reads/writes corrupt memory. ∎(i)

*(ii) Counter/semantic incoherence.* Even ignoring memory, `P_k`'s correct typesetting depends on the eqtb state *as of `P_k`* — including any counter `M` advanced (`\c@section`), font `M` selected, or macro `M` redefined. The Part D set leaves eqtb at its **live** value (post-`P_{k-1}`, or wherever the cursor is), which is neither "`Q`'s eqtb" nor "before-`P_k`'s eqtb" in general — and if the live cursor is *after* `P_k`, it may reflect mutations from `P_{k+1}…`. So the reconstructed leading state for `P_k` is wrong, and the output diverges. ∎(ii)

*(iii) Necessity of an immediately-preceding checkpoint.* The Part D set is coherent **iff** eqtb/strings/fonts/hyph are already exactly "as of the start of `P_k`" at rewind time. That holds only when (a) the checkpoint is taken *immediately before* `P_k` (so no intervening paragraph ran), **and** (b) the engine has not advanced past `P_k` (so nothing after `P_k` mutated eqtb either). This is precisely the `para_local` rolling checkpoint (D.1), not `Q`. Therefore a single post-preamble checkpoint cannot support Tier-0 for any `P_k` preceded by a state-mutating paragraph; such edits require Tier 1 (which *does* restore eqtb from a checkpoint at or before `P_j`'s mutation, then replays forward, re-executing `M` deterministically). ∎

**Corollary (the safe rule).** Tier 0 is admissible only for the *currently-open* paragraph, using `para_local`, and only when E.2's escape guard stays clean. Every other edit takes Tier 1 (nearest checkpoint ≤ edit) or Tier 2. Because Tier 1 restores the *full* value-state and re-executes all intervening mutations by replay, it is unconditionally sound (C.4).

### E.5 DVI-stream coherence (cross-cutting failure mode)

The DVI is a forward byte stream (`ShipoutState.dvi_buf/dvi_ptr/dvi_offset/dvi_gone`, `state.rs:1360-1364`). Two viable designs:

- **Re-emit whole document (recommended for v1):** every `restore_and_replay`/`compile` restores the DVI position to the checkpoint's pre-body value **and** truncates the driver's output. But pages *before* the checkpoint were already emitted. Simplest correct choice: **checkpoints[0] is post-preamble but pre-first-page**; Tier-1 replay from checkpoints[0] re-emits the entire body, so the XDV is regenerated wholesale — byte-identical to batch. Sectional checkpoints then serve only to *speed up* by not re-parsing/re-typesetting, but the DVI is still emitted from the earliest touched page. For that we must restore the DVI writer to the state at the **page boundary** preceding the checkpoint. Snapshot `dvi_ptr/dvi_offset/dvi_gone/total_pages/last_bop/cur_s/max_v/max_h/max_push` and, on restore, rewind the driver's output sink to that byte length.
- **Incremental page patching (v2):** emit each page to its own buffer and let the front-end swap only changed pages. Out of scope for this spec; noted as the eventual Typst-parity target.

**Proof obligation E-1:** for byte-identical output, `dvi_gone` (bytes already flushed to the sink) must be ≤ the restore point, i.e. **do not flush DVI bytes past the earliest possible checkpoint until the compile is confirmed final.** Practically: buffer DVI in memory (`dvi_buf`) and only flush to the real `dvi_file` at `engine_finish`, never mid-run. Verify `finalize_dvi_file`/`dvi_swap` (shipout.rs) does not force an early sink write before the last page.

---

## Part F — Edit list (files → functions → change)

| File | Function / item | Change |
|---|---|---|
| `src/resident.rs` **(new)** | `ResidentEngine`, `CheckpointLadder`, `Checkpoint`, `ParaLocal`, `boot/compile/restore_and_replay/rewind_paragraph` | new public incremental API (Parts A–E) |
| `src/snapshot.rs` **(new)** | `Snapshot`, `EscapeGuard`, `capture()`, `restore()`, `truncate_fonts/strings` | sparse value capture/restore (Part B/C) |
| `src/engine.rs` | `tt_run_engine` (`943`) | split body into `engine_init_once` / `engine_run_to_end` / `engine_finish` / `engine_teardown`; keep `tt_run_engine` as a shim calling them in order (batch path byte-identical). `engine_run_to_end` drops the trailing `final_cleanup`; `engine_finish` = `final_cleanup_in`+`close_files_and_terminate_in` (`1432/1439`); `engine_teardown` = `tt_cleanup_in` (`1442`). Do **not** re-run `init_randoms` (`1401`) on restore. |
| `src/engine.rs` | `tt_cleanup_TECkit_DisposeConverter`, `release_font_engine` reuse (`1456/1461`) | expose for `snapshot::truncate_fonts` |
| `src/errors.rs` | `EngineAbort::into_history` | add sibling `into_history_recoverable` (A.4): print + `close_files_and_terminate`, **skip** `tt_cleanup` |
| `src/main_control.rs` | `main_control_in` (`6138`), main loop (`6263`) | add `checkpoint_sink: &mut dyn FnMut(&EngineState, i32)` param; add `(VMODE, TECTONIC_CHECKPOINT)` dispatch arm near `6305` that calls the sink at `is_quiescent`; set `non_local` flag in `\parshape`/`\insert`/`\mark`/whatsit arms |
| `src/state.rs` | `EngineState` (`8`) | add `escape_epoch: u64`, `non_local: bool` scratch (reset per compile). **Do not** add `Clone`. |
| `src/save.rs` | `geq_define`/`geq_word_define` (`324/163`), `eq_word_define`@level1 (`135`) | bump `st.escape_epoch` (E.3) |
| `src/io.rs` | `UFile`/`input_line` path | add `UFileSrc::MemBuf { data, cursor }`; `input_line` reads a line from the buffer identically to the file path (C.3) |
| `src/fonts.rs` | native-font dedup cache (`1110`) | expose truncation to `<= font_ptr` for `restore` (C.2 failure mode) |
| `src/format_io.rs` | — | none; `is_quiescent` reuses the same `save_ptr==0` gate as `store_fmt_file_in` (`218`) |
| `../../src/driver.rs` | output-sink plumbing (`648/665`) | route DVI to an in-memory buffer flushed only at `engine_finish`; add a `ResidentSession` wrapper exposing `edit(edit_pos)` (E.5) |

---

## Part G — Validation & test strategy (corpus-gated)

**Bar: byte-identical XDV vs. baseline.** Three layers:

1. **Batch-path invariance (regression gate).** After the A-refactor, `tt_run_engine` must still call `init_once → run_to_end → finish → teardown` in the exact order and produce identical output. Run the full corpus dashboard (the 29/2/7/4 baseline; TRIP is 2/2 and must stay green — a TRIP failure is a real regression per the memory note). **Also compare corpus passed+diverged *count* + NOXDV/NOLOG**, not just the divergence list (native-font crashes don't show as divergences).

2. **Snapshot round-trip identity (unit).** For every corpus doc: run to the first quiescent boundary, `capture` → `restore` into a *fresh* engine, then continue. Assert the final XDV equals the uninstrumented batch XDV **byte for byte**. This proves `Snapshot` captures everything that matters. Add a debug `EngineState::structural_hash()` (hash the live mem ranges + eqtb prefix + save + nest + shipout page fields) and assert `hash(before_capture) == hash(after_restore)`.

3. **Incremental-equivalence harness (the real test).** New `tests/incremental.rs`:
   - Take corpus doc D. Compile fully via `ResidentEngine` → `xdv_full`.
   - For each checkpoint `cp` in the ladder: `restore_and_replay(D_source, cp.src_pos)` with **no edit** → assert output == `xdv_full` (idempotent replay).
   - **Edit-equivalence:** apply a real edit at line L (insert a word, change a number), compute the incremental XDV via `restore_and_replay`, and compare against a **cold full batch compile of the edited source**. Must be byte-identical. Sweep edit positions across the doc (early/mid/late paragraphs, inside/after state-mutating paragraphs).
   - **Tier-0 vs Tier-1 agreement:** whenever the escape guard says "local," assert Tier-0 output == Tier-1 output == cold batch. Whenever it says "non-local," assert we *actually escalated* (a coverage counter), and that forcing Tier-0 anyway would diverge (negative test — documents the unsoundness proof E.4 empirically).

4. **Determinism guard.** Because `random_seed` is gettimeofday-derived (`engine.rs:1399`) and lives *inside* the snapshot, incremental replays must reproduce the original stream. Test: a doc using `\uniformdeviate`, compiled incrementally, must match the *same session's* full compile (not necessarily a different-session batch, which legitimately differs). For cross-session determinism, drive `\setrandomseed` (`ext.rs:2780`) or run under the driver's deterministic mode and additionally pin the seed.

5. **Leak/UAF gate.** Run the incremental harness under ASan/valgrind: assert `truncate_fonts` frees exactly the fonts loaded after the checkpoint (no double-free of pre-checkpoint fonts, no leak of post-checkpoint ones), and that `into_history_recoverable` never frees an arena that a subsequent restore writes into.

---

## Part H — Correctness proof-obligations (consolidated)

- **B-1** free-list integrity: restoring `[0..=lo_mem_max]` + `rover` reconstructs the exact free list (verified vs `mem.rs:133-176`).
- **B-2** append-only truncation is the exact inverse of replay's appends for strings/fonts/hyph (write-once semantics).
- **C-1** no restored node references a font slot `> font_ptr` (fonts are named only by nodes predating their load).
- **C-2** `MemBuf` token stream == file token stream at the same offset (same `input_line` path; golden test).
- **E-1** DVI bytes are never flushed to the real sink before `engine_finish`, so any restore can re-emit from the earliest checkpoint without a committed-prefix mismatch.
- **E-4** Tier-0 from a non-adjacent checkpoint is unsound (free-list dangling + counter incoherence) — hence Tier-0 uses only `para_local` under a clean escape guard; everything else is Tier-1/2.

## Part I — Failure modes to guard explicitly

1. **Double paragraph** — forgetting the page-builder/contrib rewind (D.2 step 4). Caught by incremental-equivalence test on any multi-paragraph doc.
2. **Font double-free / UAF** — `truncate_fonts` mis-ranged, or a pre-checkpoint font re-requested during replay with a stale dedup entry. Guard: dedup-cache truncation + ASan.
3. **`prev_depth`/`space_factor` drift** — nest `aux` not restored → wrong baselineskip on the paragraph *after* the edit. Caught by comparing tail XDV.
4. **Synctex tag divergence** — counter not rewound (`mem.rs:215` stamps every MEDIUM+ node). Caught by SyncTeX-on corpus byte-compare.
5. **RNG re-seed** — accidentally calling `init_randoms` on restore → different `\uniformdeviate`. Guard: `restore` restores `randoms/j_random/random_seed`; assert `engine_run_to_end` never re-seeds.
6. **Locator staleness** — `ENGINE_STATE_PTR` must be re-installed at every `compile`/`restore_and_replay` entry (the `Box` address is stable, but a between-compile host call may null it; `globals.rs` FONT_* mirrors must be refreshed after any font-table move — `refresh_font_ffi_mirrors`, `globals.rs:249`, already called at the end of `restore`).
7. **Escape-guard false-negative** — a mutation path that doesn't bump `escape_epoch`. Mitigate by making `escape_epoch` bump at the *narrowest* choke (all of `geq_*`/level-1 `eq_*` in `save.rs`), and by the negative Tier-0-vs-batch divergence test that would flag any missed class.

---

### One-paragraph summary for the owner

The C core being fully gutted (only font/glyph shims remain, none touching `mem/eqtb/font_info`) means **the entire typesetter is a pure Rust value machine funnelled through `MemState`/`EqtbState`** — so snapshot/restore is a hand-written *value* capture (never `derive(Clone)`, because of the FFI pointer fields). Make `tt_run_engine` (`engine.rs:943`) a warm `ResidentEngine`; snapshot at `save_ptr==0` outer-vmode boundaries (the same gate the format dump uses, `format_io.rs:218`) copying only the two live arena ranges + eqtb prefix + high-water marks; restore is O(tail) and always sound. The O(1) same-paragraph path is admissible **only** against a rolling pre-current-paragraph checkpoint with a clean escape guard, and — proven in E.4 — is unsound from the post-preamble checkpoint the moment any earlier paragraph mutated state (free-list dangling + counter incoherence). The two traps to not miss: **rewind the page-builder/contrib list** (`page_tail`/`CONTRIB_HEAD`/`PAGE_HEAD`) or you double-append, and **never flush DVI to the real sink before `engine_finish`** or you can't re-emit. Everything is gated by byte-identical XDV on the corpus dashboard.