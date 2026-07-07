# Live-preview XeTeX: the measured plan

**Status:** design plan, grounded in measurements taken 2026-07-06 on this machine
(i9-12900K, 24 threads, 15 GiB RAM, WSL2 6.6, ext4). Supersedes the deleted
`docs/incremental/` spec set (00–06) and updates `../SPEED-INCREMENTAL-REVIEW.md`;
where this document and those disagree, this one is right — every load-bearing
number here was **measured**, not reasoned, and every structural claim was
re-verified against the tree at `748306bc`(+worktree) on the same day.

Goal (unchanged): a Typst-class live-preview TeX editor on this engine.
Scope constraint (unchanged): we do not rewrite typesetting algorithms — we change
**when** work runs, what is cached, and how `EngineState` is snapshot/restored.

---

## 0. Executive summary

1. **The batch pipeline wastes ~65–97% of its wall time on non-typesetting work
   (~97% for 1-page docs, ~65–77% for 50–150-page docs).**
   Measured: one TeX pass = **~224 ms fixed + 2.7–10.6 ms/page**, and ~212 ms of
   the fixed cost is process spawn + arena alloc/zero + format undump (a run that
   dies on a missing input file still takes 212 ms and 304 MiB RSS). The CLI always
   runs **≥2 full passes** (even `empty.tex`: pass 1 creates the `.aux`, which
   triggers a rerun), toc/xref docs run 3 (2.07× wall). On a live network every
   invocation also blocks **~2.2–2.6 s** on a bundle-CDN touch. Typesetting itself —
   the only part that is "TeX" — is single-digit ms per page.

2. **A resident engine + single-pass convergence is therefore worth ~3–33× before any
   incrementality**: 1-page doc 325 ms → <10 ms; 30-page toc doc 599 ms → ~90 ms;
   239-page arXiv doc 2,041 ms → ~650 ms (full re-typeset, warm state, one pass).

3. **The diff/tree question for `mem[]` is settled by data.** Per steady-state page
   the engine performs ~100k–250k `mem` writes but touches only **~17k–26k distinct
   words** (139–242 distinct 4 KiB blocks); a single-paragraph edit touches **~4k
   words (32 KB of data; ~48 KB as first-touch journal entries)**. eqtb (76.3 MB)
   is decisively read-mostly after format load: **70–171 distinct slots per page**,
   ≤0.05% of the table per whole document.
   Full-copy snapshots (116 MB for mem+eqtb ≈ 6–7 ms; the 216 MiB dense set
   12–14 ms) and persistent tree structures
   (measured **17–18× read penalty** on `im::Vector` — reads are the engine's
   innermost loop) are both dead. The winner is a **generation-elided undo journal
   in the `set_*` accessors** (+2–8 ns/write; restore ≈ 6.5 ns/word ⇒ a paragraph
   restores in ~30 µs, a page in ~0.15 ms), consolidated into per-checkpoint delta
   rungs, with occasional compact key-frames. This is exactly the design published
   as *Snapshottable Stores* (ICFP 2024) and used by SAT solvers/Prolog trails for
   decades; the previous plan's "paged-COW of the dense arenas" research track is
   **cancelled — unnecessary**, not just risky.

4. **The foreground architecture comes from Lode (TUG 2026) and texlite (1998), the
   substrate from TeXpresso — which is literally built on Tectonic's XeTeX.**
   TeXpresso's engine patch surface is four files (ini, engine-interface, a synctex
   extension, one header) plus one token counter in `get_next`; everything else
   lives at the ttstub I/O layer, and checkpoint/rollback is `fork()`-based. We own
   all of that surface in Rust
   and can replace `fork()` with in-value snapshots, which removes its POSIX-only,
   macOS-font, and single-thread limitations. Its policy layer (500-token snapshot
   cadence, post-edit fence clusters, 32-rung exponential ladder, append-only XDV
   with truncate-on-rollback, 10 ms stuck-worker guard) transfers unchanged.

5. **Per-keystroke budget closes with room to spare.** Restore-to-fence (µs–0.5 ms)
   + re-typeset the edited paragraph (~1–2 ms; Lode's paper measures 0.17–1.99 ms
   median per paragraph in *interpreted* LuaTeX, Table 1, and our compiled
   `line_break` should not be slower) +
   display-list extraction (no XDV, no IPC — in-process typed arena walk) lands the
   visible update in **single-digit ms**, independent of document length. Global
   layout (page numbers, refs, floats) converges in a background pass, the same
   contract Word/LibreOffice have shipped for decades.

Bottom line: nothing on the critical path is research any more. The riskiest items
are two corpus-gated refactors (glyph blobs out of the `mem` union; Arc'd font
engines) and careful bookkeeping in the restore path. Everything else is
engineering with measured budgets.

---

## 1. Measured baseline

### 1.1 Wall-clock cost model (offline, `-X compile --outfmt xdv`, median of 10)

| doc | pages | passes | wall | note |
|---|---:|---:|---:|---|
| empty | 1 | 2 | **325 ms** | 1 pass: 222 ms |
| gen10 (plain lorem) | 10 | 2 | 371 ms | |
| gen50 | 49 | 2 | 580 ms | 1 pass: 362 ms |
| gen150 | 147 | 2 | 1,096 ms | 1 pass: 617 ms |
| gentoc (toc+refs) | 30 | 3 | 599 ms | 1 pass: 290 ms |
| lshort (fontspec-heavy) | 141 | 3 | ~4,900 ms | ~10.6 ms/page-pass |
| arXiv 2012.12757 | 239 | 3 | 2,041 ms | 1 pass: 810 ms |

Derived: **single pass ≈ 224 ms + 2.69 ms/page** (plain text; real docs
2.4–10.6 ms/page); each extra pass ≈ +100 ms fixed + ~2.5 ms/page; xdvipdfmx
+88 ms fixed (+0.3 ms/page, up to +285 ms font-heavy); synctex +9–15%;
peak RSS 310–370 MiB. A fail-fast run (missing input) takes **212 ms / 304 MiB** —
that is the pure setup tax: spawn + ~272 MB arena alloc/zero + 24–27 MB format
undump. **Network:** with default args every invocation opens real HTTPS
connections to the bundle CDN (~2.2–2.6 s blocked wall here); `-C/--only-cached`
did *not* suppress the connection; `unshare -rn` falls back cleanly with
byte-identical output. A preview loop must run hermetically offline.

### 1.2 Write/dirty profile (instrumented engine, all `MemState`/`EqtbState` setters)

Steady-state, per shipped page (medians; gen50 = the instrumented 59-page plain
doc of 300 identical paragraphs, real = 239-page arXiv doc):

| metric | gen50 | real |
|---|---:|---:|
| `mem` write events | 100,994 | 250,364 |
| **distinct `mem` words** | **16,994** | **26,124** |
| distinct 4 KiB blocks | 139 | 242 |
| eqtb write events | 366 | 3,630 |
| **distinct eqtb slots** | **70** | **171** |
| str_pool growth | 0 | 0–13 chars |
| `save_ptr` at ship_out | 34 | ≤187 |

Key facts:
- Write redundancy 6.0–9.6× ⇒ any journal must be **first-touch gated** (bitmap or
  generation stamp), or it carries 6–10× dead weight.
- **Single-paragraph edit burst: ~46k writes / ~4k distinct words / +8 fresh 4 KiB
  blocks / ~0 new eqtb slots** ⇒ ~48 KB of first-touch pre-images.
- Live `mem` regions ≈ **3.0 MB** of the 40 MB arena (lo ≈ 22–29k words,
  hi ≈ 300–350k words incl. the format's 293k-word token region, which is never
  touched); free-list recycling keeps steady state flat (300 paragraphs grew
  lo-mem by only 21k words).
- Whole-document cumulative dirty set vs the post-format baseline: **285 KB / 735 KB**
  (words) — the entire body of a 239-page document perturbs less than 1 MB of arena.
- Preamble = 13–30 pages' worth of write traffic and ~1,000–4,600 distinct eqtb
  slots (vs 70–171/page in the body) ⇒ a warmed post-preamble engine cuts per-edit
  write volume ~30×.
- **eqtb is read-mostly, decisively**: ≤0.05% of its 9.54 M slots are written by an
  entire document body. Never copy 76.3 MB; journal the slots.
- Raw-escape writes (bypassing the accessors): **12 call sites, ≤1.1% of write
  events (0.33% on the real doc)**, all node-local native-text/glyph payloads
  adjacent to accessor-written headers. The old spec set's "~40–43 escape writers"
  fear is now a bounded, auditable list needing explicit range-mark hooks.

(Instrumentation diff preserved at the session scratchpad `reports/instrument-writes.diff`;
counters were added to every `MemState`/`EqtbState` setter, the raw-escape channels,
and dumps at `load_fmt_file`/`ship_out`/end.)

### 1.3 Substrate speeds on this machine

| operation | cost |
|---|---|
| memcpy 216 MiB warm→warm | 12–14 ms (1 thread) / **6.5 ms (8 threads)** |
| memcpy 40 MiB (`mem` only) | 2.0–2.6 ms |
| `Vec::clone_from` into resident buffer | = memcpy (zero abstraction cost) |
| fresh allocate + clone 216 MiB | **74–79 ms** (WSL2 first-touch faults ≈ 1 µs/page) |
| memset 277 MiB | 15–17 ms (clearing costs *more* than restoring from golden) |
| copy 1k / 10k random 4 KiB blocks | 0.2 ms / 4 ms |
| journal push (idx+old word+bitmap) | **+2–8 ns per write** over the 5–8 ns raw write |
| journal replay | 100k words = **0.45 ms**; 1M = 6.4 ms (crossover vs full copy ≈ 1M) |
| xxh3 per 4 KiB block | ≈ cost of copying the block ⇒ hash-diffing never beats write-tracking |
| fork() at 300 MiB RSS | 1–2.9 ms + **1.9–2.2 µs/page deferred COW tax** (+0.5 µs/page parent wp-faults) |
| `im::Vector` (RRB) random read | **107–113 ns vs 6.2 ns flat Vec = 17–18×** — disqualified |
| soft-dirty clear+scan of 300 MiB | 0.47 + 0.81 ms (Linux-only; use as CI coverage oracle) |

---

## 2. Architecture: two models, one system

**Model A — edit-local now, global later** (Lode "Real-Time LuaTeX", TUG 2026;
texlite 1998; every word processor): line breaking is paragraph-local (same
`\hsize` + content ⇒ identical breaks regardless of context; Knuth–Plass runs in
0.1–2 ms *per paragraph* — Lode Table 1 — O(1) in document size), page breaking is global but need
not block the keystroke. Foreground = recompile only the edited paragraph and
draw its display list; background = full compile that reconverges page numbers,
refs, floats, and *replaces the overlay only if different* (texlite's contract).
This is why LuaLaTeX holds a constant 0.70 ms per mid-document paragraph edit at
10/100/300 pages while Typst — which reconverges global layout every edit — takes
12.6/76/206 ms (Lode Table 4).

**Model B — checkpoint + forward replay** (TeXpresso, on Tectonic's own engine):
snapshot the interpreter at input-read boundaries; on an edit, roll back to the
nearest snapshot *before* the edit and re-run forward; plant a dense cluster of
"fence" snapshots just before the edit point so the *next* keystroke in the same
region rolls back ~50 tokens, not 500. TeX's catcode-mutable grammar makes
parse-tree incrementality (Typst's red-green trees) unsound — you incrementalize
**execution**, not artifacts. That is the deep reason the TeXpresso/texlite lineage
exists, and it is our spine too.

**Synthesis.** Model B is the substrate (it supplies the *correct accumulated
regime bracketing any paragraph* — the thing Model A's fast path needs to be sound,
per the E.4 unsoundness proof carried over from the old spec set: re-breaking
paragraph N against post-preamble-only state is wrong for any N after a
state-mutating paragraph). Model A is the product behavior on top. We implement
B as **values** (`Box<EngineState>` clones + journaled deltas) instead of
`fork()`, which is what makes it portable (Windows/WASM) and lets the background
compile and the fast path coexist without processes.

What we take verbatim from TeXpresso (all verified in its source):
token-counter logical clock (one `+= 1` in `get_next`); snapshot-at-read-boundary
placement, cadence ≥500 tokens; per-file consumed-byte watermarks (`SEEN`) as the
dependency trace; ≤16 post-edit fences at `(offset−64) & !63` then 50/100/200…
tokens backward; ladder budget with exponential decimation (dense near cursor);
rollback = discard snapshots past the watermark + truncate append-only outputs
(XDV byte-indexed by BOP offsets, synctex, log) to per-snapshot marks; live editor
buffers pushed via generation counter (no file reopen); 10 ms unresponsive-worker
kill. What we drop: `fork()` itself, and its single-forward-pass staleness for
`.aux` (our background convergence handles that).

---

## 3. The plan

Phases are dependency-ordered; each has a **gate** (all gates are corpus-based:
byte-identical XDV vs baseline, comparing passed+diverged *counts*, since the
dashboard misses native-font crashes; plus `cargo test -p tectonic_engine_xetex` —
unit tests are not in the default gate). Baseline dashboard: 29/2/7/4, TRIP 2/2.

### Phase 0 — Determinism pins + hermetic runs (days; do first)

1. **`random_seed` ← `build_date`** instead of `gettimeofday` (engine.rs:1435; use
   `wrapping_mul` + `& 0x7FFF_FFFF`). It is the one hidden input `deterministic_mode`
   misses; it poisons replay, page-diffing, and every cache key. `\setrandomseed`
   still overrides. One expression.
2. **Pin `TZ`** (or fold into keys): `\time/\day/\month/\year` go through `localtime`.
3. **Hermetic offline profile** for the preview loop: no CDN touch (measured
   2.2–2.6 s/invocation; `-C` alone did not prevent the connection — worth a driver
   fix upstream), fonts and bundle resolved from cache only.
4. CI guards: grep-fail on new clock/RNG reads outside gated branches; the
   14-row hidden-input table from the old Doc 06 (clock, RNG, `\elapsedtime`,
   `\filemoddate`, shell-escape, font-resolver verdict) is carried forward as the
   checklist; DVI-date trap is already closed (`c"tectonic"` literal preamble).

*Gain: correctness precondition for everything below (and removes seconds of
network wall from every dev-loop compile).*

### Phase 1 — Resident engine: pay bucket-A once per session (1–2 weeks of work)

**Mechanism.** The whole engine is one `Box<EngineState>`; the **golden point is
engine.rs:1453** (immediately before `start_input_in`): state there is a pure
function of (format bytes, config, build_date); all FFI/OS-handle fields are still
null (the format cannot contain native fonts — dump guard at format_io.rs:683).

1. **Lifecycle split** of `tt_run_engine` (engine.rs:955): `init_once` (arena alloc
   + format undump, ~450-line prologue) / `run_to_end` / `finish`
   (final_cleanup + close files) / `teardown` (today's unconditional `tt_cleanup_in`
   at engine.rs:1478 frees everything — a resident engine needs end-of-document ≠
   end-of-process). `tt_run_engine` remains a byte-identical shim. The Result-based
   `main_control` (already landed) makes this a call-boundary refactor, not a
   setjmp fight; the only residual longjmp sources are the dpx pic/pdf probe path
   and macOS AAT — both out-of-band for the preview loop.
2. **Golden snapshot + `clone_from` restore.** Capture one golden `EngineState` at
   the golden point; per recompile, `clone_from` into the resident working state.
   Measured: 12–14 ms single-thread (6.5 ms with 8 threads) vs 212 ms cold setup —
   and Phase 3 replaces even that with µs-scale journal rewinds for the common case.
   Restore must copy **into** existing buffers (never reallocate — escaped pointers
   and FONT_* mirrors depend on stable bases; `refresh_font_ffi_mirrors` hook
   already exists), and must bundle the free-list scalars
   (`lo_mem_max/hi_mem_min/rover/avail/mem_end/var_used/dyn_used` + synctex tag/line)
   atomically with `mem.data`.
3. **Single-pass convergence.** The driver already keeps in-memory `.aux`/`.toc`
   across passes (`MemoryIo` on the session bridge; rerun decided by
   `read_digest != write_digest` in `is_rerun_needed`, src/driver.rs:1350). Reuse
   the converged aux across *edits* (not just passes): steady-state body edits leave
   the aux byte-identical ⇒ exactly one pass. Also: skip the guaranteed second pass
   for the aux `nonexistent → \relax`-only transition (saves the ~100 ms extra pass
   on every trivial doc: `empty.tex` 325 → 222 ms even before the resident engine).
4. **Known resident-engine hazards to fix while here** (found in the audit): ext.rs
   `BRK_ITER`/`BRK_LOCALE_STR_NUM` — an ICU break-iterator cache **keyed by
   str_number**, stale across restores; io.rs statics (`NORMALIZERS`, buffers);
   `SAVED_MAPPING_NAME`. Fold into `EngineState` or key correctly.

*Measured gain: 1-page 325→<10 ms; 30-page toc 599→~90 ms; 147-page 1,096→~400 ms;
239-page 2,041→~650 ms. This is the single biggest user-visible step and it needs
no snapshot machinery beyond one golden clone.*

### Phase 2 — FFI ownership: make `EngineState` a true value (2–4 weeks, the two risky refactors)

The audit re-confirmed exactly three classes of non-value state, and both fixes
land as independent, corpus-gated, byte-identical no-ops **before** anything
depends on them:

1. **Glyph blobs out of the `mem` union** (the #1 blocker, unchanged). Today
   `mem[p+5].ptr` holds an xcalloc'd blob (10 B/glyph: `FixedPoint[n]` then
   `u16 gid[n]`), written by `measure_native_node` (ext.rs:1518/1606) through raw
   node pointers, freed in `flush_node_list` (packaging.rs:533–539) — any byte-clone
   of the arena double-frees. Replace with a `GlyphBlobArena`
   (`slab: Vec<Option<Arc<Blob>>>` + free list), 1-based integer handle stored in
   `mem[p+5].b32.s0`; `copy_native_glyph_info` becomes a refcount bump;
   `store_justified_native_glyphs`' in-place mutation goes through `Arc::make_mut`
   (COW). ~12 call sites (enumerated in the old Doc 02, re-verified). This also
   converts most raw-escape *writes* into accessor writes, shrinking the journal's
   blind spot at the same time. Gate: ASan on the native-font/justification corpus
   subset + byte-identical XDV.
2. **Arc'd font engines/mappings.** `font_layout_engine`/`font_mapping:
   Vec<*mut c_void>` → `Arc<LayoutEngineHandle{raw, font_area}>` with `Drop`
   disposers (must carry `font_area` — the release path needs it); keep a flat
   `*mut` mirror Vec for the OT-math C++ callback (already the only C reader,
   via `tt_get_font_layout_engine_entry`). Deletes the per-slot free loop in
   `tt_cleanup_in`. Enables snapshot sharing *and* the Phase 6 cross-run font cache.
3. **Owned C strings + handles**: `root_name`, `last_source_name`,
   `name_of_input_file` re-`strdup` on clone; OS handles (dvi/log/write/synctex,
   `input_file`/`read_file`) are route-to-null or journaled — the preview fast path
   does no file I/O (input comes from the editor buffer), so its snapshots simply
   assert these are quiescent.

After Phase 2 there are **zero owning raw pointers inside the arenas**, and
`impl Clone`-equivalent `snapshot()/restore()` becomes mechanical. (Deliberately
still not `derive(Clone)`: only sub-states with no owning pointers get `Clone`
derived, so the compiler rejects accidental aliasing — the old Doc 02 discipline,
kept.)

### Phase 3 — The snapshot substrate: journal + rungs + key-frames (the diff/tree answer)

This is the direct answer to "efficiently clone the entire EngineState (diff or
tree-based algorithms for mem[])". The design space, closed by measurement:

| substrate | verdict | why (measured) |
|---|---|---|
| full copy per keystroke | ✗ | 116 MB mem+eqtb, 12–14 ms, GBs for a ladder |
| live-prefix bounded copy | ◐ key-frames only | live mem ≈ 3.0 MB + eqtb-as-delta ≈ 57 KB ⇒ **~3.1 MB ≈ 0.5–1 ms** — great per-rung base, borderline per keystroke |
| persistent structures (RRB/HAMT/im) | ✗ | 17–18× read penalty on the innermost loop; O(1) clone taxes every read forever |
| page/chunk COW (4 KiB) | ◐ acceptable for `mem`, wrong for eqtb | 2.8–3.2× the bytes of a deduped word log; O(1) metadata; eqtb's 171 scattered slots would cost 171 blocks |
| **word journal w/ first-touch elision** | ✅ per-keystroke spine | +2–8 ns/write; 48 KB/paragraph, 0.2–0.3 MB/page; restore 6.5 ns/word (paragraph ≈ 30 µs, page ≈ 0.15 ms) |
| **eqtb slot journal** | ✅ | 0.8–2.1 KB/page (12 B/slot); never copy 76.3 MB |
| Merkle/hash diffing | ✗ | hashing a block costs the same as copying it; only useful for cross-run cache keys |
| fork() COW | ✗ | works on WSL2 (1–3 ms) but 1.9–2.2 µs/page deferred fault tax ≈ 5× eager copying; no Windows/WASM; TeXpresso's own TODO says "implement snapshotting without fork" |
| mprotect/uffd/soft-dirty | CI oracle only | 250–600× a journal push per stray word; not on WASM. Soft-dirty scan (0.8 ms/300 MB) verifies journal coverage in CI |

**Design** (= *Snapshottable Stores*, ICFP 2024 — journaled version history over
plain mutable arrays, record elision via generation stamps; reads stay flat loads;
semi-persistent/linear variant suffices for a prefix-chain ladder; production
precedent: SAT/SMT trails, Prolog WAM, Conchon–Filliâtre persistent union-find):

1. **Always-on write journal** in the 10 `MemState` setters + `EqtbState` setters
   (+ `SaveStackState`, string/font watermarks): first-touch gate (per-word
   generation stamp or the 610 KiB bitmap), push `(u32 idx, MemoryWord old)`.
   Measured overhead bound: 100k–276k writes/page × 2–8 ns ≈ **0.2–2.2 ms/page** of
   typesetting — and GC card-marking literature (0.1–3.1% mutator overhead in
   production JVMs) says the branch-predictable version will sit at the low end.
   The **12 raw-escape sites** get explicit `mark_range(p, n)` hooks (they write
   node-local payloads at known offsets; ≤1.1% of events). CI oracle: Linux
   soft-dirty page scan diffed against the journal's touched set.
2. **Checkpoint = cut the journal** (O(1)) at quiescent boundaries
   (`save_ptr==0`-ish; measured at ship_out: `save_ptr` 34–187, trivially small;
   post-`build_page` points expected similar, unmeasured); consolidate the since-last-cut journal
   into a **per-rung reverse-delta block** (dedup by address ⇒ Δ = distinct words:
   0.2–0.3 MB/page word-granular). Restore to rung k = apply reverse blocks
   k..current (0.15 ms/page-of-distance). Scalars/small sub-states (nest, save
   stack, page builder, scanner, synctex counters, `MemFrontier`) ride each rung as
   a plain POD bundle (KB-scale) — cheaper than instrumenting, and it structurally
   captures the double-append trap (`page_tail`/CONTRIB_HEAD) and the synctex tag
   counter that the old specs proved must travel together.
3. **Key-frames** every ~16–32 rungs: the **compact live-region copy** —
   `[0..=lo_mem_max] ∪ [hi_mem_min..=mem_end]` (~3.0 MB) + eqtb-delta vs the
   post-format golden (≤57 KB whole-doc) + watermarks ≈ **3.1 MB, 0.5–1 ms** —
   bounds worst-case restore-chain length. One full golden (272 MB, in place)
   exists per session, from Phase 1.
4. **Ladder policy** (TeXpresso constants as defaults): rungs at read/paragraph
   boundaries ≥500 tokens apart; post-edit fence cluster (≤16, 50/100/200…-token
   spacing) so repeat edits in one region roll back ~50 tokens; exponential
   decimation under a memory budget. Budget math: a 239-page document at
   one rung/page ≈ 50–70 MB raw; zstd on consecutive deltas (RetroArch statestream
   precedent: 3.5–67×) brings a full ladder to ~5–20 MB — WASM-feasible (wasm32
   linear memory caps at 4 GB: one golden + deltas fit; a full-copy ladder never
   could).
5. **Restore correctness bundle** (carried from the old specs, still right):
   free-list scalars atomically with `mem.data`; strings restored by bounded
   byte-copy first (the truncate-only optimization stays unproven until the
   `flush_string`-below-watermark invariant is corpus-validated); fonts restored by
   Arc-vec assignment + dedup-cache truncation above `font_ptr`; hyph trie built
   eagerly at checkpoint creation (it is lazily built on first second-pass —
   determinism hazard otherwise); `xeq_level` journaled with eqtb; GlyphBlobArena
   slab+free-list snapshot rides the rung bundle (else a journal rewind reverts
   handle words while the slab has moved on).
6. **Forward replay** re-runs the engine from the restored rung with input served
   from the **editor's in-memory buffer** (a `SourceProvider`/MemBuf input path —
   the edited file must never be re-opened+seeked; stable `\input`ed files may be).
   Byte watermarks per input file (TeXpresso `SEEN`) pick the rung; the token
   counter is the logical clock.

*Gain: per-edit engine-state turnaround goes from 12–14 ms (golden clone) to
**µs–0.5 ms** for fence-local edits; checkpoint capture cost ~0 (journal cut);
memory bounded. Combined with 2.4–10.6 ms/page replay, an edit's visible page is
re-typeset well inside a 60 Hz frame.*

### Phase 4 — Paragraph fast path + display-list extraction (Model A foreground)

All four pieces were design-studied against the current tree and are S/M-grade:

1. **One-paragraph driver** (~150–500 LOC, small end preferred): thread a
   `stop_at_par` condition into `main_control_in` (~30 LOC) rather than cloning the
   dispatch loop; run the *real* horizontal-mode machinery (option "build the hlist
   yourself and call `line_break` directly" founders on ligatures/shaping/
   interchartoks/space factors — half of `main_control`). Suppression trick:
   `push_nest` an **internal-vmode** level (what `\vbox{` does) so the
   `(HMODE, PAR_END)` arm calls `end_graf_in` but not `build_page_in`
   (main_control.rs:~4400 guards on outer vmode; belt-and-braces
   `output_active = true` makes `build_page_in` a no-op). Harvest the vlist between
   the pushed nest's head/tail: hboxes with final `glue_set/shift/width` (hpack ran
   inside `post_line_break`), baselineskip glue materialized, penalties in place.
   Seed from the bracketing rung: full eqtb + `prev_depth` (first baselineskip
   depends on it) + `prev_graf` + zeroed protrusion cache
   (`last_leftmost_char/last_rightmost_char`).
2. **Display-list extractor** (~500–700 LOC, read-only walker on the typed node
   layer — `classify`/`AnyNode`/`AnyBox` cover it; char nodes tested via
   `hi_mem_min` first). Emits `DisplayItem{Glyph{font,gid,x,y}, TfmChar, Rule, …}`
   + per-item `(synctex_tag, line)`. Must replicate exactly: the **cumulative**
   glue arithmetic (`cur_glue: f64` accumulator, `cur_g = tex_round(clamp(glue_set
   * cur_glue))` — rounding the running total, not per-glue, or positions drift
   ±1 sp per glue node), shift_amount/baseline stepping, native-word blobs (read
   directly: node-relative `FixedPoint` positions + gids — **no shaping call at
   extraction time**), rule running dimensions. Bail to slow path on: leaders,
   TeXXeT/RTL (`reverse()` destructively mutates the list at output time),
   `upwards` mode, pic/pdf whatsits. A paragraph-relative walk **without the page
   builder is exact**: the fast-path vlist is unpacked, so vertical glue is at
   natural size; page-level stretch only enters at vpack, which the overlay model
   defers to the background pass.
3. **Locality detection** (err to slow path; ~100–300 LOC): (a) post-typeset node
   scan for `Ins | Mark | Adjust` and `Write/Special/Open/Close` whatsits — the
   page-cycle couplers, exact and cheap; (b) a mutation trap during the fast-path
   run: any `geq_define`/`geq_word_define`/level-one `eq_define` sets a `nonlocal`
   bit (the journal gives this for free — any eqtb write at global level is visible
   in it); (c) `\ref`/`\thepage` staleness is *inherently* background-path — the
   background compile recomputes truth and the editor re-marks. Do not build a
   token pre-scan detector (macro indirection defeats it).
4. **In-memory SyncTeX index** (~150–300 LOC): every ≥MEDIUM node already carries
   `(tag,line)` stamped at `get_node`; the nine `synctex_record_node_*` fns have
   page-absolute geometry in hand — add an optional Vec sink on `SynctexCtxt`
   (skip the gz serialization for preview; `synctex_write` at synctex.rs:109 is the
   single choke point). Editor edit at (file,line) → tag table → bracketing hbox
   records → page + y-anchor + owning rung. This is also the checkpoint locator and
   the stable cross-edit paragraph name.

*Gain (measured/inferred): paragraph re-typeset ≈ 1–2 ms (texlode: 0.17–1.99 ms
median in LuaTeX on a slower laptop; >80% of its round-trip is line breaking, and
our IPC row is zero because extraction is an in-process arena walk). Total
keystroke: restore (µs) + paragraph (1–2 ms) + extract (sub-ms) + draw ⇒
**single-digit ms, independent of document size** — Typst-class interactivity with
LaTeX compatibility.*

### Phase 5 — Background convergence + orchestration

- **v1 topology: time-sliced single engine thread** (run the background full
  compile in token-bounded quanta between fast-path requests). This dodges the two
  real walls found in the audit: the process-global `FONT_*` mirror statics *and*
  `bridge_core`'s `ENGINE_LOCK` global mutex + single global core-state pointer
  (crates/bridge_core/src/lib.rs:202,306–310) — two concurrent in-process engines
  are a cross-crate redesign, not an engine patch. Fork-based background compile is
  the M-grade sidestep on Linux/macOS; true two-thread is R-grade and not needed
  for v1. (This also matches the old Doc 00's Seam-10 resolution and gives WASM
  parity for free.)
- **Cancellation**: one relaxed-atomic check after `get_x_token_in` in the main
  loop (+ optionally at `build_page_in` entry), surfaced as an `EngineAbort`
  variant (~30 LOC). Granularity is one command token; the long primitives between
  checks are single-digit-ms. 10 ms unresponsive ⇒ kill and fall back a rung
  (TeXpresso's guard).
- **Output contract** (texlite/Word): the visible page updates immediately from the
  fast path; the background pass produces the page-position cache
  (page → \count0..9 + per-paragraph (tag, line-span, y)) and the converged aux;
  overlays are replaced only if the background result differs. Background XDV is
  append-only with the BOP byte-offset chain (already tracked via `last_bop`) —
  rollback = truncate, unchanged prefix pages keep their bytes and their renders.
- **Editor protocol**: adopt TeXpresso's shape (async change/goto/render messages,
  byte-range edits against a driver-side VFS, generation counter invalidating
  engine-side read buffers) — its Emacs/Neovim/VS Code frontends prove the shape.

### Phase 6 — Caches and longer-horizon items (parallel/after)

- **Shape cache** on `measure_native_node` (the one shaping choke point; pure
  function of engine+text+letter-space+metrics flags). With the Phase-2 blob arena
  a hit is an `Arc` bump. Keyed by font fingerprint + UTF-16 run; default-off
  build proves byte-identity. Saves the HarfBuzz/ICU-bidi run per unchanged word
  across background recompiles (the fast path already avoids re-shaping unchanged
  paragraphs entirely — this cache mainly accelerates the *background* pass and
  first-compile of repeated content).
- **Font-engine + TFM cross-run cache** keyed with content digests (+ lib versions
  if ever persisted). In-run slot dedup exists; engine *construction* is repaid per
  run today.
- **Micro-hotpath** (bench-first, from the v1 review, unchanged priority): bulk
  input read (byte-per-FFI-call today), bulk DVI emit, synctex-stamp gating,
  unchecked accessors only if disassembly shows un-hoisted bounds checks.
- **WASM target**: the whole design is fork-free and signal-free by construction;
  linear-memory golden + delta ladder fits the 4 GB cap; ship display lists out of
  the worker as transferables (structured-clone of MB-scale buffers costs hundreds
  of ms; transfer is O(1)). SwiftLaTeX shipped WASM-XeTeX "checkpointing" to real
  users — precedent, plus a warning label (their own docs admit occasional bugs ⇒
  our corpus/differential gates are the moat).
- **Incremental XDV page-splicing** for the final PDF path: keep v1 = re-emit from
  rung 0 in background; per-page patching stays out of scope until the display-list
  path is proven.

---

## 4. Validation strategy (the part that makes this safe to build)

1. **Corpus gate** on every phase: byte-identical XDV (passed+diverged counts vs
   baseline 29/2/7/4, TRIP 2/2), each risky refactor (Phase 2.1, 2.2) landed alone
   as a no-op.
2. **Prefix-rewind equivalence** (the real invariant, stronger than
   snapshot-then-restore identity): run tail → restore → run *edited* tail →
   restore → run original tail ⇒ byte-identical to never having restored. This is
   the live-preview access pattern as a differential test.
3. **`--dl-verify`**: continuous cross-check display-list positions vs the XDV the
   batch path emits for the same state (modeling DVI-unit rounding explicitly).
   The extractor is a mechanical sibling of `hlist_out`/`vlist_out` — treat it as a
   divergence farm and test it continuously.
4. **Journal-coverage oracle** (Linux CI): soft-dirty page scan vs journal touched
   set after each corpus doc — catches any future raw-escape write the hooks miss.
5. **ASan native-font subset** for the blob-arena and Arc-font refactors
   (double-free/UAF class).
6. **Forced-Tier-0 divergence test**: deliberately run the paragraph fast path in a
   non-local situation and assert the detector routed it to the slow path (negative
   test for the escape guard).
7. **Warm-cache byte-identity**: corpus twice in one process; run 2 output ==
   run 1 (validates resident state, caches, and restore hygiene together).

---

## 5. Expected end-state performance (all from measured components)

| scenario | today (CLI) | Phase 1 | Phase 3+4 |
|---|---:|---:|---:|
| keystroke → visible paragraph update | 325–4,900 ms | <10 ms–1.5 s (one warm pass) | **2–8 ms** (restore µs–0.5 ms + par 1–2 ms + extract <1 ms + draw) |
| keystroke → correct visible page (re-typeset page) | same | same | **~5–15 ms** (fence restore + page replay 2.4–10.6 ms) |
| global convergence (refs/toc/page numbers) | 2–3 full cold passes | 1 warm pass (<10 ms–1.5 s by doc size) | background, cancellable, from nearest rung (≈ tail only) |
| session RAM | ~350 MiB/run, freed | ~350 MiB resident | + ladder ~5–70 MB (zstd/raw) + golden |
| portability | all | all | all incl. Windows/WASM (no fork, no signals) |

The 100 ms "instantaneous" HCI budget is met at Phase 1 for small docs; the 16 ms
frame budget is met at Phase 3+4 for all docs, with document-size-independent
foreground latency — the property Typst does not have (it reconverges globally per
edit: 206 ms at 300 pages) and texlode demonstrated for LuaTeX.

## 6. Risk register (what's genuinely hard)

| risk | grade | mitigation |
|---|---|---|
| Glyph-blob arena refactor (Phase 2.1) | M–H | isolated no-op landing, ASan, 12 enumerated sites; also *shrinks* the journal blind spot |
| Arc font engines w/ exactly-once disposal (2.2) | M | `Drop` handles + mirror kept; corpus + ASan |
| Restore-bundle completeness (Phase 3.5) | M | prefix-rewind differential is the detector; POD-bundle-per-rung captures whole sub-states, not hand-picked fields |
| Extractor bit-fidelity (glue rounding, native words) | M | `--dl-verify` continuous; bail list (RTL/leaders/upwards) keeps v1 honest |
| Fast-path soundness (locality) | M | journal-based global-write trap + node scan; err-to-slow-path; background pass is the source of truth — mis-routes self-correct, never a wrong PDF |
| bridge_core global lock (two engines in-process) | R (deferred) | v1 time-slices one thread; fork sidestep on POSIX; redesign only if v1 UX demands true parallelism |
| Journal write overhead regresses batch compiles | L | measured bound 0.2–2.2 ms/page worst case; feature-gated build rows (off = textually current code) |
| WSL2/machine variance of the numbers | L | all substrate numbers re-derivable from `bytes/17 GB·s⁻¹ (1T) or /34 (8T) + 1 µs/cold page`; bench harnesses preserved |

Resolved-by-measurement (were open questions in v1): real dirty counts per
edit/page (§1.2); cold vs warm startup split (§1.1); word-log vs chunk-COW vs
paged-COW (§3, Phase 3); eqtb read-mostly premise (confirmed, decisively);
fork viability (works, loses anyway); persistent-structure viability (dead);
eqtb size dispute in the old docs (76.3 MB is correct); raw-escape scale
(12 sites, ≤1.1% — not ~43 unknowns).

## 7. Mapping to the superseded spec set

The deleted `docs/incremental/` W-items survive with these dispositions:
W0/W1 → Phase 0 (seed fixed; audit done — this document *is* the W1 closure);
W2/W3 → Phase 2 (unchanged, still the critical refactors); W4/W5 → Phase 3
(bounded copy demoted to key-frames; journal promoted from "parallel track" to
spine — the measured 6–9.6× write redundancy and 70–171 eqtb slots/page make the
choice); W6/W7 → Phases 1/3; W8 → Phase 4.3 (escape guard = journal, as Doc 00
Seam 12 wanted); W9/W11 → Phase 6; W10 → closed (12 sites + range-marks + CI
oracle); W12/W13 → Phase 4.2/4.4; W14 → Phase 5 (cooperative v1 confirmed, and
the bridge_core lock found as the second reason); W15 → Phase 0.4/6; W16 → §4.
The E.4 Tier-0 unsoundness proof, the exactly-once-disposal proof shape, the
EpochPodBundle contents, and the hidden-input table are carried forward intact.
Doc 02's "eqtb ≈ 5–7 MB" was wrong (76.3 MB); Doc 04's paged-COW rejection was
right for the wrong scale of reason — it is not merely awkward, it is *unneeded*.

## 8. Prior-art ledger (condensed; full notes in session reports)

- **TeXpresso** (let-def/texpresso; vendored Tectonic XeTeX C): fork-based
  checkpoints at read boundaries, 500-token cadence, ≤16 fences, 32-rung
  decimated ladder, SEEN byte-watermarks, append-only XDV + truncate, 10 ms guard.
  Its own comment on macOS fonts: the real fix is "XeTeX snapshotting without
  fork" — i.e., this plan.
- **texlite** (Strokov, MAPS 26, 1998): per-page core dumps at ship_out with
  two-level delta chains; paragraph "emergency repair" (unwrap lines → splice →
  re-`line_break`); background recompile replaces the page "only if it differs —
  usually it does not".
- **texlode** (Lode, TUG 2026 preprint + benchmark repo): resident LuaTeX,
  ~1 ms per-paragraph recompile constant in doc size (paper Table 1: 0.17/0.70/
  1.99 ms median short/medium/long; Table 4: LuaLaTeX constant 0.70 ms vs Typst
  12.6/76/206 ms at 10/100/300 pages); \vbox isolation benchmarking methodology
  worth copying.
- **Typst/Haug thesis + comemo**: constraint-validated layout caching + the
  corpus test "retain one cache level, assert all-hit and identical output"
  (copy this test design); red-green incremental parsing (do **not** copy —
  catcodes); evaluation/export remain O(n) per edit (their own Future Work).
- **Snapshottable Stores** (Allain et al., ICFP 2024, Distinguished Paper): our
  journal design, formalized — flat current version, reverse-delta history,
  generation-stamp record elision; "reads are native"; full-copy alternative
  measured 50× slower, persistent-map alternative 3× slower in their workloads.
- **SwiftLaTeX**: WASM XeTeX with engine-state checkpointing shipped to users.
- **RetroArch/GGPO/Slippi**: full memcpy viable to ~8 MB/frame; everyone needing
  history depth moved to consecutive deltas + periodic key-frames + zstd (3.5–67×).
- **Word/LibreOffice**: background repagination — the UX contract of Phase 5.

---

*Measurement provenance: wall-clock (hyperfine, offline via `unshare -rn`, 10
runs/entry), write-profile (instrumented setters, full diff preserved), substrate
micro-benchmarks (Rust `--release` + C, two full runs) — all under the session
scratchpad; instrumentation worktree left the user tree untouched. Anchors:
engine.rs:955/1453/1478, format_io.rs:683, driver.rs:1350, main_control.rs:~4400,
linebreak.rs:1286/2497, packaging.rs:533/878, ext.rs:1518/2248, synctex.rs:109,
mem.rs:244, state.rs (setters :527–:610), bridge_core lib.rs:202 — verified
2026-07-06.*
