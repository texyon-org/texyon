Verification complete. All anchors confirmed against the current tree. Here is the spec.

---

# Live-Preview XeTeX Editor — Implementation Spec

**Target:** `/home/yendric/Code/tectonic/crates/engine_xetex` (parent driver `../../src/driver.rs`)
**Goal:** A Typst-like live editor whose preview updates in <50 ms on the foreground path, built on cheap `EngineState` snapshot/restore + an in-memory display list, **without rewriting any core typesetting algorithm.** We change *when* work runs and *what we snapshot*; every node-layout routine stays byte-for-byte as-is.
**Correctness bar:** byte-identical XDV vs. the corpus baseline (dashboard: passed+diverged count, NOXDV/NOLOG), enforced continuously. The live path is an *addition*; the batch XDV path must remain provably untouched.

All `file:line` anchors below were opened and verified in the working tree at spec-authoring time.

---

## 0. Why this is now tractable (verified ground truth)

The port has crossed the threshold that used to block snapshotting:

1. **The TeX core is pure Rust.** The xetex C files are gutted shells (`xetex-xetex0.c`=10 L, `xetex-ini.c`=199 L is just the `tt_engine_xetex_main` wrapper + a musl linker hack). The only live native code is *platform font shims* (`xetex-macos.c`, `teckit-*`, `xetex-XeTeXOTMath.cpp`) that operate on font handles + glyph ids and **never touch `mem[]`/`eqtb`/`font_info`.** Therefore **every arena mutation is Rust** and funnels through `MemState`/`EqtbState` accessors. There is no live C writer of the TeX arena to defeat a snapshot.

2. **The arena is a plain `Vec<MemoryWord>`.** `MemState.data: Vec<MemoryWord>` (`state.rs:450`); `MemoryWord` is an 8-byte `Copy` union (`mem_types.rs:68`). A snapshot of the arena is a `Vec::clone` of POD — no pointer chasing, no serialization.

3. **`EngineState` is a single owned `Box`** (`engine.rs:952`), 15 named sub-states (`state.rs:8–24`): `config, errors, shipout, scanner, save_stack, eqtb, mem, input, strings, hyph, fonts, linebreak, texmf, synctex, mainloop`. It derives only `Default` — **no `Clone` exists anywhere today.** Adding a correct `Clone`/snapshot is the foundational task (Spec 2, §2 below).

The residual snapshot hazards are a *bounded, enumerable* set of FFI-owning fields (§2.3). Everything else is POD or a Rust-owned `Vec`.

---

## 1. Architecture at a glance

```
        ┌────────────────────────── ResidentEngine (one warm Box<EngineState>) ──────────────────────────┐
        │                                                                                                 │
 edit ──┤  1. debounce (~16–50ms)                                                                         │
 event  │  2. pick nearest checkpoint ≤ edit (checkpoint ladder, §3)                                      │
        │  3. Snapshot::restore_into(&mut warm)         ← cheap arena memcpy + FFI refresh (§2)            │
        │  4. replay tokens from checkpoint to EOF on the FOREGROUND state                                │
        │  5. run pagebuilder → ship_out_DL (display-list sink, NOT XDV)  (§4)                             │
        │  6. diff display list vs. last, push positioned glyphs + synctex index to UI (§5,§6)             │
        │                                                                                                 │
        │  Background convergence thread (§5A): full compile on a CLONED EngineState → .aux + page cache;  │
        │  cancellable when a new edit lands. Feeds converged cross-refs back as new checkpoints.          │
        └─────────────────────────────────────────────────────────────────────────────────────────────┘
```

Two engines, one code path. The **batch/XDV path is literally unchanged**: `ship_out_in` (`shipout.rs:505`) still emits XDV. The live path calls a *sibling* `ship_out_dl_in` that walks the same vlist with the same geometry but writes to a `Vec<PositionedItem>` instead of the DVI buffer (§4). We prove the two agree by keeping XDV emission wired and byte-comparing on the corpus.

---

## 2. FOUNDATION — `EngineState` snapshot/restore (Spec 2)

This is the load-bearing capability. Everything else assumes it.

### 2.1 What a snapshot must capture

The engine's *entire* observable state lives in the `Box<EngineState>`. There are **no hidden globals that carry algorithmic state** across the snapshot boundary — memory note *zero-global-state DONE (01e)* confirms the hot-path scalar fold is complete; the only residual statics are `ENGINE_STATE_PTR` (`state.rs`, a per-run *locator*, re-installed on restore, not state) and 3 `FONT_*` FFI mirror locators in `globals.rs` (~308–334) that *point into* the font `Vec`s and must be refreshed after any move (§2.3).

So: **snapshot = deep-copy the `EngineState`, with special handling for a bounded set of FFI-owning fields.**

### 2.2 The three field classes

Walking the 15 sub-states, every field is exactly one of:

**(A) POD / Rust-owned `Vec` of POD — trivially `Clone`.**
`mem.data: Vec<MemoryWord>` (`state.rs:450`) and scalars `lo_mem_max, hi_mem_min, avail, mem_end, rover, var_used, dyn_used, line, cur_synctex_tag, temp_ptr` (`state.rs:451–472`); `eqtb.data: Vec<MemoryWord>` (`state.rs:285`), `prim`, save-level arrays; `strings.str_pool/str_start`; `save_stack.data`; `input.buffer/nest/input_stack/param_stack/…`; all the `fonts.font_*` metric `Vec`s (`font_check, font_size, font_info, char_base, width_base, …`, `state.rs:927–970+`). These are the bulk of the bytes and clone with a `memcpy`.

**(B) Copy-safe locators / static rodata — copy the value, meaningless to own.**
`errors.help_line`/`help_line[]` (`&'static CStr` ptrs), `synctex.recorder` (fn-ptr, `state.rs:84`), the `FONT_*` mirrors, `ENGINE_STATE_PTR`. These copy bit-for-bit but must be *re-derived*, not trusted, after a move (§2.3).

**(C) FFI-owning raw pointers — the snapshot blockers.** Enumerated in §2.3.

### 2.3 The FFI-owning field inventory (the entire hazard set)

| Field | Location | Owns | Freed at | Snapshot rule |
|---|---|---|---|---|
| `FontState.font_layout_engine: Vec<*mut c_void>` | `state.rs:903` | `XeTeXLayoutEngine` | `engine.rs:1456` (`release_font_engine`) | **share** (refcount) |
| `FontState.font_mapping: Vec<*mut c_void>` | `state.rs:907` | `TECkit_Converter` | `engine.rs:1461` (`DisposeConverter`) | **share** (refcount) |
| `FontState.loaded_font_mapping` | `state.rs:897` | transient converter | reload `fonts.rs:1079` | transient; null between fonts |
| `mem[p+5].ptr` native-glyph buffers | `node.rs:2222` `native_glyph_info_ptr` (offset 5) | shaped-glyph `malloc` block | `packaging.rs:562`, `fonts.rs:1148` | **deep-copy on clone** (§2.4) |
| `SynctexCtxt.root_name` | `state.rs:81` | C string | synctex path | own `CString` or dup |
| `TexmfState.last_source_name` | `state.rs:132` | C string | (leaks at teardown) | dup |
| `InputClusterState.name_of_input_file` | `state.rs:268` | C string | `engine.rs:1484` | dup |
| `input.input_file: Vec<*mut UFile>` | `state.rs` | open file handles | `engine.rs:1471` | **not restorable — see §2.5** |
| `input.read_file: [*mut UFile;16]` | `state.rs:262` | `\read` streams | close | §2.5 |
| `shipout.dvi_file/log_file/rust_stdout` | `state.rs` | output handles | close | live path uses its own sink |
| `SynctexCtxt.file` | `state.rs:80` | gz handle | synctex terminate | live path uses in-mem sink (§6) |

**Key insight:** classes (B) and (C-files) are *I/O boundary* state, and the live fast-path **does not do file I/O** — it replays from an in-memory token buffer into an in-memory display list. So the snapshot the fast path needs is the *typesetting core* (`mem, eqtb, strings, save_stack, scanner, input-stacks, fonts-metrics, hyph, linebreak, mainloop, synctex-counters`), not the file handles. This shrinks the hazard set dramatically.

### 2.4 The one genuinely hard case: native-glyph buffers

`mem[p+5].ptr` (`node.rs:2222`) points at a heap block laid out (per `make_xdv_glyph_array_data_in`, `shipout.rs:2204–2216`) as `glyph_count` × `FixedPoint{x:i32,y:i32}` (`shipout.rs:2199`) locations, followed by `glyph_count` × `u16` glyph ids. Size is fully determined by `native_glyph_count` (`node.rs:2195`, `mem[p+4].b16.s0`). A snapshot that clones `mem.data` copies the *pointer*, aliasing one `malloc` block across two engines — a double-free at `packaging.rs:562`/`fonts.rs:1148`.

**Rule:** the arena cloner must walk live native-word/glyph whatsit nodes and deep-copy each buffer. Because size is derivable, this is mechanical:

```rust
// state.rs (new): deep-clone the native-glyph side buffers after a bulk mem clone.
impl MemState {
    /// Re-owns every `mem[p+5].ptr` native-glyph block reachable in the arena,
    /// so the clone shares no malloc allocation with the source.
    unsafe fn reown_native_glyph_buffers(&mut self, fonts: &FontState) {
        for p in self.iter_whatsit_native_nodes() {          // walk hi/lo mem for Whatsit+native subtype
            let w = NodePtr::<WhatsitNode>::from_idx_unchecked(MemIdx::new(p));
            let src = w.native_glyph_info_ptr(self);
            if src.is_null() { continue; }
            let gc = w.native_glyph_count(self) as usize;
            let bytes = gc * NATIVE_GLYPH_INFO_SIZE;         // 8 (FixedPoint) + 2 (u16) per glyph
            let dst = libc::malloc(bytes) as *mut u8;
            std::ptr::copy_nonoverlapping(src as *const u8, dst, bytes);
            w.set_native_glyph_info_ptr(self, dst as *mut c_void);
        }
    }
}
```

`iter_whatsit_native_nodes` need not walk the whole page graph — the fast path only ever snapshots *before* shaping produces these (checkpoints sit at `\begin{document}`-class boundaries, §3), so in practice the checkpoint arena has **zero** live native-glyph buffers and this loop is empty. We still implement it for correctness of mid-document checkpoints.

**Cheaper alternative (recommended):** never snapshot mid-shape. Place checkpoints only at paragraph/page boundaries where `contrib_head` is empty and no native buffers are live; assert `iter_whatsit_native_nodes().next().is_none()` at snapshot time. Falls back to deep-copy only if the assert trips.

### 2.5 Files: replay, don't restore

Restoring `input.input_file` handles to a mid-file `loc` is neither possible nor needed. The fast path holds the **full source text in memory** (the editor owns it). A checkpoint records only the *logical* input position (`input_stack` records: `state, index, start, loc, limit, name, synctex_tag` — `globals::InputState`, seen at `engine.rs:1039–1048`), and replay re-derives file content from the editor buffer via a *memory-backed* `UFile` shim. Concretely: introduce a `SourceProvider` trait; the live `start_input` variant reads lines from `SourceProvider::line(name, n)` instead of `u_open`. This is the single new seam in `io.rs`.

### 2.6 The snapshot API

```rust
// state.rs (new module `snapshot`)
pub(crate) struct Snapshot {
    core: EngineStateCore,          // deep clone of the typesetting-core sub-states (class A+B)
    fonts_shared: FontHandles,      // Arc-wrapped layout-engine / mapping handles (class C, shared)
    synctex_counters: SynctexSeed,  // tag_counter + tag/line cursors (§6)
}

impl EngineState {
    /// O(bytes-of-arena) memcpy of the core; FFI handles shared, not duped.
    pub(crate) fn snapshot(&self) -> Snapshot { /* … */ }

    /// Overwrite `self`'s core in place from `snap`; re-install locators
    /// (ENGINE_STATE_PTR, FONT_* mirrors), rebind Arc font handles.
    pub(crate) fn restore_into(&mut self, snap: &Snapshot) {
        // 1. clone-from the arena Vecs into self's existing Vecs (reuse capacity)
        self.mem.data.clone_from(&snap.core.mem_data);
        self.eqtb.data.clone_from(&snap.core.eqtb_data);
        // … strings, save_stack, input stacks, fonts metric Vecs …
        // 2. copy POD scalars
        // 3. rebind FFI: fonts share (Arc clone), refresh FONT_* mirrors into the Vecs
        crate::globals::refresh_font_mirrors(&self.fonts);
        // 4. ENGINE_STATE_PTR already points at self (same Box); no-op
    }
}
```

`clone_from` reuses the destination `Vec`'s allocation (no per-restore malloc), so a restore is ~1–8 MB of `memcpy` — sub-millisecond. **The `Box<EngineState>` never moves**, so `ENGINE_STATE_PTR` (`state.rs:1623` `set_engine_state_ptr`, installed `engine.rs:963`) stays valid; only the `FONT_*` mirrors (which point *into* the font `Vec`s, and those `Vec`s may reallocate) need refresh — hence `refresh_font_mirrors`.

**Font handle sharing:** wrap `font_layout_engine`/`font_mapping` entries in `Arc<LayoutEngineHandle>` where `Drop` calls `release_font_engine`. Snapshot clones the `Arc` (refcount++), restore installs it. `tt_cleanup_in` (`engine.rs:1446`) drops the last `Arc`. This makes fonts loaded once shared across all checkpoints — correct, because a font is immutable after load, and `load_native_font_in` already dedups within a run (`fonts.rs:1110`).

### 2.7 Proof obligation for snapshot fidelity

**PO-SNAP-1 (round-trip identity):** For any batch corpus doc, `st.snapshot()` immediately followed by `restore_into` before `ship_out` must yield **byte-identical XDV**. Test harness: gate on the dashboard with a build flag `--snap-roundtrip` that snapshots at every `build_page` entry and restores. If XDV diverges, the snapshot is lossy — a field was missed.

**PO-SNAP-2 (no aliasing):** ASan/valgrind run of the corpus with snapshot+drop-source proves no double-free of native-glyph buffers or font handles.

---

## 3. WHEN work runs — the checkpoint ladder

We do not re-run TeX from scratch per keystroke. We keep a *ladder* of snapshots keyed by input position, and replay only the suffix after the edit.

```rust
struct Checkpoint {
    input_pos: SourcePos,     // (tag, line, byte) — the resumable frontier
    snap: Snapshot,           // §2.6
    page_base: usize,         // first page index produced at/after this point
    dl_prefix_hash: u64,      // display-list hash up to here (for early-out)
}
struct CheckpointLadder { rungs: Vec<Checkpoint>, spacing: usize /* ~every N pages or M source lines */ }
```

On an edit at source position `e`:
1. `rung = ladder.floor(e)` — nearest checkpoint with `input_pos ≤ e`.
2. `warm.restore_into(&rung.snap)`.
3. Seek the in-memory `SourceProvider` to `rung.input_pos`; replay `main_control` to EOF.
4. As pages emit, run `ship_out_dl_in` (§4); as a page's display list matches the previous frame's (`dl_prefix_hash`), **stop early** — nothing below changed.

**Why replay-to-EOF is acceptable:** most edits are near the end of what the user is looking at; the rung sits just above the viewport. Replaying a few pages of `main_control` is the cost, and it's bounded by "distance from last checkpoint," not document size.

**Ladder maintenance:** after a successful foreground compile, if the edited region is "clean" (no locality violation, §7), promote the just-computed state at the next spacing boundary into a fresh rung. Evict rungs above the edit (they're stale). This is the Adapton-style incremental frontier; the synctex tag (§6) is the *stable name* that lets us match a rung's node identities across edits.

**Checkpoint placement constraint (from §2.4):** rungs land only at `build_page` boundaries where `page_contents == EMPTY` (`pagebuilder.rs:704`) and `contrib_head` is drained, guaranteeing no live native-glyph buffer and a trivially clean arena.

---

## 4. DISPLAY-LIST EXTRACTION — bypassing XDV (Spec 5B)

Instead of `ship_out → XDV → xdvipdfmx`, walk the same finished vlist and emit positioned primitives. This is a **clone of `hlist_out_in`/`vlist_out_in`** with the `emit_dvi_*` calls replaced by `push` into a `Vec` — *same traversal, same geometry math, no algorithm change.*

### 4.1 The exact geometry we must reproduce

From `hlist_out_in` (`shipout.rs:1216`) and `vlist_out_in` (`shipout.rs:1615`), the geometry a glyph's final `(x,y)` depends on:

- **Cursor state** `shipout.cur_h/cur_v` (page sp), advanced per node.
- **Char nodes** (`shipout.rs:1315–1339`): advance `cur_h += font_character_width(f,c)` (`shipout.rs:2193`); the char's origin is `(cur_h, cur_v)` at emit time; TFM font-mapping remap (`apply_tfm_font_mapping`, `shipout.rs:1320`) applies before position.
- **Glue** (`output_hlist_glue_in`, `shipout.rs:1074`): the *set* width is `rule_wd = spec.width - cur_g` then `+= cur_g` where `cur_g = tex_round(glue_set * cur_glue clamped ±1e9)` (`shipout.rs:1092–1100`) — **this rounding must be bit-identical**; it uses `st.mem.gr(this_box+6)` (the box's `glue_set` f64) and `scaled_math::tex_round`. Order of accumulation into `cur_glue` matters (it's a running f64 sum).
- **Boxes** (`shipout.rs:1354–1380`): recurse with `cur_v = base_line + shift_amount`; `edge = cur_h + width`.
- **Native word/glyph whatsits** (`shipout.rs:1389–1456`): glyph positions come from the `mem[p+5]` buffer — `FixedPoint{x,y}` per glyph (`shipout.rs:2237`), *relative to the node origin* `(cur_h, cur_v)`; node advances `cur_h += width`. This is where the real text lives.
- **Rules** (`finish_hlist_rule_in`, `finish_vlist_rule_in`): `rule_ht/dp/wd`, `NULL_FLAG` → running dimension.
- **`\XeTeXinterwordspaceshaping`>1** triggers `merge_justifiable_native_words` (`shipout.rs:1233`) which *mutates* the list — the extractor must run it too, identically.
- **LR / bidi** (`reverse`, `new_edge`, `LR_ptr` machinery, `shipout.rs:1257–1600`): direction flips adjust `cur_h`. Must be replayed.
- **Leaders**, **margin kerns**, **math nodes** (converted to kerns, `shipout.rs:1556`).

### 4.2 The extractor

```rust
// shipout_dl.rs (new)
#[repr(C)]
pub struct FixedPoint { pub x: i32, pub y: i32 }   // mirrors shipout.rs:2199

pub enum DlItem {
    Glyph   { font: i32, glyph: u16, x: i32, y: i32 },       // native single glyph
    NativeWord { font: i32, glyphs: Vec<(u16, i32, i32)>,    // (gid, x_sp, y_sp) absolute
                 text: Vec<u16> },                           // for copy/search
    Char    { font: i32, code: u16, x: i32, y: i32 },        // TFM char
    Rule    { x: i32, y: i32, w: i32, h: i32, d: i32 },
    Color   { rgba: u32 },                                   // from \special color pushes
    Link    { rect: (i32,i32,i32,i32), target: LinkTarget }, // pdf annots
}
pub struct DisplayList {
    pub page_no: i32,
    pub width_sp: i32, pub height_sp: i32,
    pub items: Vec<DlItem>,
    pub node_index: Vec<(MemIdx, usize)>,   // node → item range, for synctex (§6)
}

/// Sibling of ship_out_in (shipout.rs:505). Same page-setup prologue
/// (cur_h/cur_v/offsets, shipout.rs:595–697) but DL sink instead of DVI.
pub(crate) unsafe fn ship_out_dl_in(st: &mut EngineState, p: i32) -> Result<DisplayList, ShipoutAbort>;

/// Clone of hlist_out_in (shipout.rs:1216). IDENTICAL cursor arithmetic;
/// every `emit_dvi_*` becomes `dl.push(...)`; every geometry line is copied verbatim.
unsafe fn hlist_out_dl(st: &mut EngineState, dl: &mut DisplayList) -> Result<(), ShipoutAbort>;
unsafe fn vlist_out_dl(st: &mut EngineState, dl: &mut DisplayList) -> Result<(), ShipoutAbort>;
```

Native-word glyph extraction (the hot case), lifted directly from `make_xdv_glyph_array_data_in` (`shipout.rs:2204–2263`) but into structs instead of the xdv byte buffer:

```rust
// inside hlist_out_dl, WhatsitSubtype::NativeWord/At/Glyph arm (mirrors shipout.rs:1389)
let gc = whatsit.native_glyph_count(&st.mem) as usize;
let info = whatsit.native_glyph_info_ptr(&st.mem) as *const u8;
let locs = info as *const FixedPoint;
let gids = info.add(gc * 8) as *const u16;               // same layout as shipout.rs:2216
let base_x = st.shipout.cur_h;                            // node origin == emit-time cursor
let base_y = st.shipout.cur_v;
let mut glyphs = Vec::with_capacity(gc);
for i in 0..gc {
    glyphs.push((*gids.add(i),
                 base_x + (*locs.add(i)).x,               // FixedPoint is already in sp, node-relative
                 base_y + (*locs.add(i)).y));
}
dl.items.push(DlItem::NativeWord { font: whatsit.native_font(&st.mem).0, glyphs,
                                   text: read_native_text(whatsit, &st.mem) });
st.shipout.cur_h += whatsit.width(&st.mem).0;             // EXACT same advance as shipout.rs:1418/1453
```

### 4.3 The risk, stated plainly

The extractor **must match XDV geometry exactly** — every `tex_round`, every `cur_glue` accumulation order, every LR flip, or the preview drifts from the final PDF. The mitigation is structural: **derive `hlist_out_dl`/`vlist_out_dl` mechanically from `hlist_out_in`/`vlist_out_in`** (copy the function, delete DVI-encoding-only lines, keep all cursor math), and **validate by cross-check** (§8): a build mode that runs *both* sinks on the same page and asserts every `DlItem::{Glyph,Char}.{x,y}` equals the position XDV would place (reconstructed from the DVI movement stream). Divergence is a hard test failure.

**Do NOT refactor `hlist_out_in` into a shared generic** in v1 — that risks the batch XDV path (corpus regression). Duplicate now; unify only once the DL path is corpus-clean and you can prove the generic emits identical XDV.

---

## 5. PROCESS / THREAD TOPOLOGY (Spec 5A)

### 5.1 Native (desktop) topology

One resident engine, **no fork**. The warm `Box<EngineState>` lives on the foreground thread. Background convergence runs on a *cloned* engine.

```rust
pub struct ResidentEngine {
    warm: Box<EngineState>,
    ladder: CheckpointLadder,
    source: Arc<RwLock<SourceModel>>,      // editor text, line-indexed
    last_frame: Vec<DisplayList>,
    bg: Option<BackgroundCompile>,         // in-flight full compile
}

pub enum EngineCmd {
    Edit { range: SourceRange, text: String },
    Scroll { viewport_pages: Range<usize> },
    ForwardSync { pos: SourcePos },        // source → preview
    InverseSync { page: i32, x: i32, y: i32 },  // preview → source
    Converge,                              // request full background compile
}
pub enum EngineEvent {
    Frame { pages: Vec<DisplayList>, dirty: Range<usize> },
    SyncResult(SyncHit),
    Converged { aux_digest: DigestData, page_cache: PageCache },
    Diagnostics(Vec<TexDiag>),
}

impl ResidentEngine {
    /// Foreground fast path. Debounced by the caller (~16–50 ms).
    pub fn on_edit(&mut self, ed: Edit) -> EngineEvent {
        self.cancel_background();                              // §5.3
        let rung = self.ladder.floor(ed.pos());
        self.warm.restore_into(&rung.snap);                   // §2.6, sub-ms
        self.source.write().apply(ed);
        let pages = self.replay_and_extract(rung.input_pos);  // §3 steps 3–4
        let dirty = diff_pages(&self.last_frame, &pages);
        self.last_frame = pages.clone();
        self.maybe_promote_checkpoint();                      // §3
        EngineEvent::Frame { pages, dirty }
    }
}
```

### 5.2 Background convergence

`\ref`, page numbers, `\tableofcontents`, marks — anything needing a *converged* `.aux` — is resolved by a full recompile on a **cloned** engine (enabled entirely by §2). It writes `.aux`, and the parent driver's fixed-point loop (`driver.rs`: `read_digest` `:96`, `write_digest` `:100`, digest compare at `:1357`) decides convergence exactly as batch mode does. On `Converged`, the resident engine ingests the page cache and *rebuilds the ladder* so the fast path now has correct cross-refs.

```rust
struct BackgroundCompile { handle: JoinHandle<...>, cancel: Arc<AtomicBool> }
impl ResidentEngine {
    fn spawn_background(&mut self) {
        let mut clone = self.warm.snapshot().into_owned_engine();  // full deep clone incl. file seam
        let cancel = Arc::new(AtomicBool::new(false));
        let src = self.source.clone();
        self.bg = Some(BackgroundCompile { cancel: cancel.clone(), handle: thread::spawn(move || {
            run_full_compile(&mut clone, src, cancel)              // checks `cancel` at each build_page
        })});
    }
}
```

### 5.3 Cancellation

`main_control` is a tight loop; add one cooperative check. The cleanest seam is `build_page_in` (`pagebuilder.rs:670`) and the token-getter: poll `cancel` (an `Arc<AtomicBool>`) at page boundaries and at `main_control`'s top. On cancel, unwind via the existing `EngineAbort` machinery (memory: `main_control_in` already returns `Result<(), EngineAbort>`, caught before `final_cleanup`) — add a `EngineAbort::Cancelled` leaf that skips output and returns cleanly. Because the clone is thrown away, no cleanup subtlety: just drop it (Arc font handles refcount down; native buffers freed by `Drop`).

### 5.4 WASM (single-threaded) story

No thread. Convergence runs **cooperatively**: `run_full_compile` is a state machine that yields (returns `Poll::Pending`) at each `build_page` boundary, driven from `requestAnimationFrame`. The `cancel` flag becomes "a newer edit generation exists" checked at the same yield points. The snapshot/restore path is *identical* — the win of pure-Rust + `Box<EngineState>` is that the whole thing compiles to WASM with no pthread dependency. The font shims are the only `cfg`-gated native code and are already `#[cfg(macos)]`/optional; WASM uses the HarfBuzz/OT path.

---

## 6. SYNCTEX IN-MEMORY INDEX (Spec 5C)

### 6.1 What exists

`synctex.rs` already records, per node, a `(tag, line, curh/unit, curv/unit, w, h, d)` tuple while `hlist_out`/`vlist_out` walk the page — e.g. `synctex_record_node_hlist` (`synctex.rs:335`) emits `"(tag,line:h,v:w,h,d\n"`. `tag` and `line` come from the node's synctex trailer, stamped at allocation by `get_node` (`mem.rs:214–216`: `set_b32s0(r+s-1, cur_synctex_tag)`, `set_b32s1(r+s-1, line)` for `MEDIUM_NODE_SIZE`+ nodes). `synctex_tag_counter` (`state.rs:94`) is bumped per input file in `synctex_start_input` (`synctex.rs:620–637`). Today all of this serializes to a gz `.synctex` file (`SynctexCtxt.file`, `state.rs:80`).

### 6.2 Add an in-memory sink

The DL walk (§4) already visits every node with `cur_h/cur_v` set — it *is* the synctex pass. So the in-memory index is a **byproduct of `ship_out_dl_in`**: at each node record point (the same points as `synctex_hlist`/`synctex_math`/etc., called from `hlist_out_in` at `shipout.rs:1299, 1341, 1493, 1497`), push into a spatial + logical index instead of formatting a string.

```rust
pub struct SynctexIndex {
    // forward: source (tag,line) → set of preview rects
    forward: BTreeMap<(i32 /*tag*/, i32 /*line*/), Vec<PreviewRect>>,
    // inverse: R-tree / grid over (page, x, y) → (tag, line)
    inverse: SpatialGrid<SourcePos>,
}
pub struct PreviewRect { page: i32, x: i32, y: i32, w: i32, h: i32, d: i32 }

// Called from ship_out_dl_in at every node-record site (mirrors synctex.rs:335 etc.)
fn dl_record_hlist(idx: &mut SynctexIndex, mem: &MemState, page: i32, p: i32, cur_h: i32, cur_v: i32) {
    let (tag, line) = (synctex_tag_box(mem, p), synctex_line_box(mem, p));   // synctex.rs:50,54
    let r = PreviewRect { page, x: cur_h, y: cur_v,
                          w: synctex_width(mem,p), h: synctex_height(mem,p), d: synctex_depth(mem,p) };
    idx.forward.entry((tag, line)).or_default().push(r);
    idx.inverse.insert(r.into_bbox(), SourcePos { tag, line });
}
```

Forward query (`ForwardSync`): `forward.range((tag,line)..)` → highlight rects in the preview. Inverse query (`InverseSync`): `inverse.nearest(page,x,y)` → jump the editor to `(tag,line)`.

### 6.3 Keying the ladder / stable names

The `(tag, line)` pair is the **Adapton stable name**: it survives edits that don't renumber input (edits *below* keep their tags; edits *above* shift `line` but the tag is per-file). A checkpoint rung stores the `synctex_tag_counter` value (`SynctexCtxt.synctex_tag_counter`, part of the snapshot core) so replay re-derives identical tags — **this is why the tag counter must be in the fast-path rewind set** (it is, being a plain `u32` field of `SynctexCtxt`, class A). If it weren't restored, replayed nodes would get fresh tags and every inverse-sync would break after one edit.

### 6.4 Live path never opens the gz file

The fast path sets `SynctexCtxt.file = INVALID_HANDLE` (the `Default`, `state.rs:102`) and routes records to `SynctexIndex`. The `synctex_enabled` config (`config.synctex_enabled`, `state.rs:890`) and the recorder fn-ptr (`state.rs:84`) are untouched for the batch path. So `.synctex` file emission and the in-memory index are mutually exclusive per compile mode — no regression risk to the on-disk format the corpus checks.

---

## 7. LOCALITY-BOUNDARY ROUTING (Spec 5D)

Not every edit is fast-path-safe. The fast path assumes the edit's effect is *local* — replaying from the rung reproduces the true state. That's false when the edit touches global, order-dependent state.

**Route to the SLOW (background) path when the edited region reads or writes any of:**

- **Counters / cross-refs:** `\ref`, `\pageref`, `\cite`, `\label`, `\setcounter`, `\thepage`, ToC/LoF — resolved only after `.aux` convergence (`driver.rs` digest loop). Detect by scanning the edited token stream for these control sequences.
- **`\parshape`, `\hangindent`, `\looseness`** applied *above* the rung — they change earlier line breaks; a rung below them already baked the wrong shape. Safe only if the edit is within the rung's paragraph.
- **Footnotes / floats / marks / inserts:** they migrate to other pages via the pagebuilder (`fire_up_in`, `pagebuilder.rs:240`); a local replay can't see the float land elsewhere.
- **Any write to `eqtb` global scope** (`\gdef`, `\global`, `\def` at outer group) whose reader is below the replay window. Conservative detector: if the edit's token expansion writes eqtb at `save_stack` level 0, escalate. (Reads of *global* eqtb inside the replay window are fine — the rung snapshot captured eqtb.)
- **`\input`/`\include`** structural changes (file set changes → `read_files` in `driver.rs:286` changes).

**Policy: err to the slow path.** The fast path is an *optimistic* preview; the background compile is the source of truth and always runs (debounced longer, e.g. 300 ms). When they disagree (background produces a different display list), the background result wins and the ladder is rebuilt. So a mis-classified "local" edit produces a briefly-wrong preview that self-corrects within one convergence cycle — never a wrong PDF.

**Detector implementation:** a cheap lexical pre-scan of the edited byte range for the control-sequence set above, plus a "distance to enclosing paragraph/group boundary" check against the rung. Both are string/position ops on the editor model — no engine run needed to decide routing.

---

## 8. VALIDATION STRATEGY (Spec 5E) — the bit-identical burden

The corpus dashboard (byte-identical XDV vs. baseline; passed+diverged count, NOXDV/NOLOG) is the non-negotiable gate. Every change is measured against it.

**Layered gates:**

1. **Batch path untouched (regression firewall).** The XDV `ship_out_in` and all of `hlist_out_in`/`vlist_out_in` are *not edited* by this project (§4 duplicates rather than refactors). CI: full corpus, expect **zero** new divergences, and per memory *Corpus doc-count regression check*, compare **passed+diverged count** (not just the divergence list) plus NOXDV/NOLOG to catch native-font crashes the dashboard's divergence list misses.

2. **Snapshot round-trip (PO-SNAP-1, §2.7).** Build flag `--snap-roundtrip`: snapshot+restore at every `build_page` entry; run corpus; assert byte-identical XDV. Any divergence = a lossy snapshot field. This is the single most important new test.

3. **DL ↔ XDV geometry cross-check (§4.3).** Build flag `--dl-verify`: run both sinks per page; reconstruct XDV glyph positions from the DVI movement stream and assert every `DlItem` position matches. Run over the corpus. This proves the extractor reproduces XDV geometry.

4. **Determinism pin.** The live path must be reproducible or the DL diff (`diff_pages`) thrashes. `random_seed` is set from wall clock — `st.errors.random_seed = microseconds*1000 + epochseconds%1_000_000` (`engine.rs:1399`), fed from `get_seconds_and_micros` (`engine.rs:1200`, `libc::gettimeofday` via `texmfmp.rs`). This is **not** covered by any deterministic flag. For the live engine, pin the seed (and `build_date`, the `time_t` arg to `tt_run_engine`, `engine.rs:946`; the driver already forces `UNIX_EPOCH` under `force_deterministic`, `driver.rs:1000/1004`) to a fixed value so snapshot/replay is bit-stable. Proof obligation **PO-DET-1:** two fast-path compiles of identical source produce identical display lists.

5. **ASan/valgrind (PO-SNAP-2).** No double-free of native-glyph buffers (`packaging.rs:562`, `fonts.rs:1148`) or font handles (`engine.rs:1456/1461`) under snapshot+drop.

6. **Synctex fidelity.** In-memory index vs. the on-disk `.synctex` for the same doc: assert the `(tag,line)→(h,v)` tuples match the gz records byte-for-byte (both derive from the same `synctex_*` readers, so this is a tight equivalence).

---

## 9. EXPLICIT EDIT LIST

| # | File | Fn / item | Change |
|---|---|---|---|
| E1 | `state.rs` | new `mod snapshot`: `Snapshot`, `EngineState::snapshot`, `restore_into` | §2.6. `clone_from` core Vecs; copy scalars; rebind Arc fonts; refresh mirrors. |
| E2 | `state.rs` | `MemState::reown_native_glyph_buffers`, `iter_whatsit_native_nodes` | §2.4. Deep-copy `mem[p+5]` buffers on clone. |
| E3 | `state.rs` | `FontState.font_layout_engine/font_mapping` → `Vec<Arc<LayoutEngineHandle>>` (or a shared table) | §2.6. `Drop` calls `release_font_engine`/`DisposeConverter`; removes the explicit frees at `engine.rs:1454–1463` (replaced by Arc drop). **Corpus-gated — verify XDV unchanged.** |
| E4 | `globals.rs` | `refresh_font_mirrors` | §2.6. Re-point `FONT_*` mirrors (~308–334) into the (possibly moved) font Vecs after restore. |
| E5 | `io.rs` | `SourceProvider` trait + memory-backed `start_input` variant | §2.5. Fast path reads source from editor buffer, not `u_open`. Batch path unchanged. |
| E6 | `shipout_dl.rs` (new) | `ship_out_dl_in`, `hlist_out_dl`, `vlist_out_dl`, `DisplayList`, `DlItem` | §4. Mechanical clone of `ship_out_in`/`hlist_out_in`/`vlist_out_in` (`shipout.rs:505/1216/1615`) with DL sink. |
| E7 | `synctex.rs` | `SynctexIndex`, `dl_record_*` | §6. In-memory forward/inverse index built during `ship_out_dl_in`. |
| E8 | `errors.rs` | `EngineAbort::Cancelled` leaf | §5.3. Cooperative cancel unwind. |
| E9 | `pagebuilder.rs` / `main_control.rs` | cancel poll at `build_page_in` (`pagebuilder.rs:670`) + main-loop top | §5.3. `Arc<AtomicBool>` check; only active in live mode. |
| E10 | new crate/module `resident` | `ResidentEngine`, `CheckpointLadder`, `EngineCmd/Event`, `BackgroundCompile` | §3, §5. The orchestration layer; consumes E1–E7. |
| E11 | `engine.rs` | live-mode seed pin | §8.4. Fixed `random_seed`/`build_date` when a `live_deterministic` config flag is set. |
| E12 | build/CI | `--snap-roundtrip`, `--dl-verify` flags + dashboard hooks | §8. |

**Ordering:** E1–E4 (snapshot) first, gated by PO-SNAP-1 on the corpus, before anything else. E6/E7 (extractor) next, gated by `--dl-verify`. E5/E8–E11 (orchestration) last. E3 (Arc fonts) is the riskiest corpus-visible change — do it in isolation with a dedicated corpus run.

---

## 10. CORRECTNESS PROOF-OBLIGATIONS & FAILURE MODES

| ID | Obligation | Failure mode if violated |
|---|---|---|
| PO-SNAP-1 | snapshot∘restore preserves XDV byte-identically | Silent preview drift; a missed field (e.g. a `linebreak` scratch, a `scanner` cursor) makes replay diverge. Caught by §8.2. |
| PO-SNAP-2 | no arena/font double-free under snapshot+drop | Crash / heap corruption. The native-glyph buffer (`mem[p+5]`) aliasing (§2.4) and font-handle sharing (E3) are the two hazards. |
| PO-DET-1 | fast-path compile is deterministic | DL diff thrashes; seed from `gettimeofday` (`engine.rs:1399`) leaks nondeterminism. Fixed by E11. |
| PO-DL-1 | `DlItem` positions == XDV positions | Preview ≠ PDF. The `cur_glue` f64 accumulation order and `tex_round` (`shipout.rs:1092`) are the fragile spots. Caught by §8.3. |
| PO-SYNC-1 | `synctex_tag_counter` restored per rung | Inverse-sync breaks after first edit (§6.3). It's in the snapshot core; assert it. |
| PO-ROUTE-1 | non-local edits go slow-path | Wrong preview until convergence — *tolerable* (self-corrects), never a wrong PDF, because background compile is source-of-truth. |
| PO-FMT-1 | can't snapshot inside a group | `store_fmt_file_in` refuses `save_ptr != 0` (`format_io.rs:230`). Same constraint for checkpoints: only snapshot at `save_ptr == 0` / `build_page` boundaries (§3). |

**Additional failure modes:**
- **Mid-shape checkpoint** with live native buffers → §2.4 deep-copy required; §3 avoids by placing rungs at empty-`contrib_head` boundaries.
- **`\XeTeXinterwordspaceshaping` list mutation** (`shipout.rs:1233`) mutates the arena during output — the DL path must run it, and a snapshot taken *after* output is dirty. Checkpoints precede output; the DL walk operates on a *throwaway* view or the batch path re-derives. Take checkpoints before `ship_out`, never after.
- **Font handle lifetime under Arc:** a font loaded in a background clone that converges and replaces the ladder must not free a handle the foreground still references — Arc refcounting handles this, but `tt_cleanup_in` (`engine.rs:1446`) must drop Arcs, not `libc::free` raw (E3).

---

## 11. Sequenced delivery

1. **E1–E4 + PO-SNAP-1/2** — snapshot/restore, corpus-clean, ASan-clean. *This alone is the enabling capability; everything else is UI.*
2. **E6–E7 + PO-DL-1, PO-SYNC-1** — display-list extractor + in-memory synctex, `--dl-verify`-clean on corpus.
3. **E5, E8–E11 + E10 orchestration** — resident engine, ladder, background convergence, cancellation, WASM cooperative variant.
4. **E12** — permanent CI gates.

The critical path is step 1: a lossless, aliasing-free `EngineState` snapshot at `build_page` boundaries, proven by byte-identical corpus XDV. Once that holds, the live preview is a walk of the same nodes the shipout already visits, with the DVI encoder swapped for a `Vec` — no typesetting algorithm is rewritten anywhere.