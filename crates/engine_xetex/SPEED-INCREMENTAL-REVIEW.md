<!-- Generated review: speed + incremental/live-preview architecture for the Tectonic XeTeX engine. -->

> **How to read the `file:line` anchors.** Every *mechanism* cited below was
> spot-verified against the current tree and is real, but the exact line numbers
> have drifted by roughly 10–50 lines (the analysis worked from slightly stale
> offsets). Treat an anchor as "find the named function/feature *near* that line,"
> not as an exact address. Verified examples: the synctex trailer stamp is at
> `mem.rs:216` (`set_b32s0(r+s-1, cur_synctex_tag)`), not :245; the
> "You can't dump inside a group" / `save_ptr != 0` guard is at
> `format_io.rs:230-231`, not :239; the `TRACING_COMMANDS` re-read is at
> `main_control.rs:6300`, not :6353; `measure_native_node` is at `ext.rs:1475`;
> the driver's `read_digest`/`write_digest` convergence check and
> `force_deterministic` fallback are in the top-level `src/driver.rs` (~:96/:990).
>
> One research strand (general TeX/LuaTeX profiling numbers) failed its schema in
> the workflow, so the "where TeX spends time" claims lean on the Lode paper +
> code cost-model rather than an independent profiling survey.

---

# Making the Tectonic XeTeX Engine Faster — Toward a Typst-like Live-Preview TeX Editor

A systems architecture review for the owner of the Rust XeTeX engine (`crates/engine_xetex`). Scope constraint honored throughout: **we do not rewrite typesetting algorithms.** Every recommendation is about *when* work runs (startup avoidance, caching, incremental recompute) and about making `EngineState` cheap to snapshot and restore. All anchors are `file:line` from the cost-model findings.

---

## 1. Executive summary + baseline cost model

### 1.1 The thesis

Today `process()` runs `tt_run_engine` (engine.rs:986) as a **cold batch job**. Every invocation heap-boxes a fresh `EngineState` (engine.rs:997), allocates ~277 MB of arenas from scratch and memsets them to zero, undumps the entire 24–27 MB LaTeX format (format_io.rs:885), retokenizes and re-typesets the whole document, re-shapes every word through HarfBuzz, re-emits the whole XDV, then `tt_cleanup_in` (engine.rs:1504) frees everything and the `Box` drops. **Nothing survives between compiles.** The multi-pass driver (driver.rs:1705) does this 2–3 times per document to converge `\ref`/`\tableofcontents`.

For a live-preview editor this is the wrong shape at three time-scales:

1. **Per session:** the format load + arena zero-fill is fixed doc-independent work (~100–300 ms) that should happen *once*, not once per keystroke.
2. **Per keystroke:** re-typesetting the whole body and re-shaping every unchanged paragraph is redundant; the edited paragraph line-breaks in <1–2 ms (Lode Table 1), so the ceiling is set entirely by everything we *avoid* — but avoiding it soundly requires a checkpoint positioned *immediately before the edit* (§4.4), not merely a persistent engine.
3. **Per convergence:** the 2–3 pass rerun loop is only needed when `.aux` actually changes (driver.rs:1357); a body-text keystroke leaves it byte-identical and needs exactly one pass.

The single structural fact that makes all of this tractable **now**: the entire mutable engine is one owned `Box<EngineState>` (engine.rs:997) whose heavy core is flat `Vec`s of `Copy` POD (`MemoryWord` is an 8-byte `Copy` union, mem_types.rs:68), and **every cross-reference in the graph is an i32 arena index, never a machine pointer** — so the whole object graph survives a memcpy to a new base address unchanged. The prior zero-global-state work (memory notes, session 249/01e) means the *Rust* mutation surface is fully contained in that one struct. (One premise still needs verification, not assumption — that no C translation unit, chiefly xetex-xetex0.c, holds any `static`/C-global surviving a call beyond its `mem[]` writes; see §5/§7.1/§7.7.) This is exactly the substrate a snapshot/restore incremental engine needs, and it is why Tectonic is a *better* target for this than LuaTeX (which needs Lua node hacks) or C-XeTeX (which needs `fork()`).

### 1.2 Baseline per-compile cost model

Three buckets. Sizes verified against `MemoryWord = 8 B` (mem_types.rs:68) and the compiled-in constants (consts.rs:37–55; hash_extra=600,000 at engine.rs:1056).

**A. Fixed startup (document-independent, paid every run today):**

| Arena | Size | Anchor | Zeroed? |
|---|---|---|---|
| eqtb.data | 76.3 MB (9,541,459 words) | format_io.rs:945 | xcalloc in C |
| eqtb.hash | 76.3 MB (9,540,945 words) | format_io.rs:935 | xcalloc in C |
| font_info | 64.0 MB (8,000,001 words) | format_io.rs:1249 | xcalloc in C |
| mem.data | 40.0 MB (5,000,000 words) | format_io.rs:976 | **xmalloc in C — extra memset** |
| str_pool | 12.5 MB | format_io.rs:1044 | **xmalloc in C — extra memset** |
| str_start | 2.26 MB | format_io.rs:1036 | **xmalloc in C — extra memset** |
| xeq_level | 2.23 MB | state.rs:326 | — |
| input/save/font/hyph arrays | 3–4 MB | engine.rs:1063,1078 | — |
| **Total** | **≈277 MB across ~40 Vecs** | | |

Note the shape: **eqtb.data + eqtb.hash + font_info ≈ 216 MB (~78% of footprint) are dense arenas** — the eqtb pair holds the Unicode cat/lc/uc/sf/math/del tables (53.5 MB, consts.rs:47–55) plus the RLE/sparse-decoded dump image; they **cannot be shrunk by a "live-prefix" bound**. Only `mem.data`, `str_pool`, `str_start`, and the used prefix of `font_info` are sparsely occupied.

Plus **format undump**: 24–27 MB read via `ttstub_input_read`, **byte-swapped word-by-word** on little-endian (undump_bytes, format_io.rs:143–157), RLE-decoded for eqtb (format_io.rs:1128–1154), sparse-decoded for hash (format_io.rs:1195–1224), free-list-walked for mem (format_io.rs:1084–1100), and scattered into the 277 MB. Plus **O(EQTB) re-init loops**: the `xeq_level` fill runs `INT_BASE..=EQTB_SIZE` = 1,114,730 iterations every run (engine.rs:503), and the eqtb "undefined" fill runs up to hash_extra entries (format_io.rs:957).

Reasoned wall-time (no bench): ~277 MB memset at ~10 GB/s ≈ 28 ms, plus first-touch mmap page-faulting, plus tens of ms of undump read+swap. **~100–300 ms cold fixed overhead before the first glyph** — 6–20× a 16 ms frame budget. Note ~55 MB of that memset (mem 40 + str_pool 12.5 + str_start 2.26) is work the **C engine never did** (those were `xmalloc`, uninitialized — format_io.rs:949, engine.rs:1075).

**B. Typesetting (per-token, document-dependent):**

- `main_control_in` loop (main_control.rs:6349): `get_x_token_in → get_next_scc → get_next_in`; ≥1 cat-code eqtb read per source char (scan.rs:1629,1649,1660,1664), plus a re-read of `TRACING_COMMANDS` **every iteration** (main_control.rs:6353), then the `match (mode,cmd)` dispatch (~400 arms; hot text arm hoisted first at main_control.rs:6361).
- `id_lookup` (hash.rs:45) re-hashes and re-compares **every** cs token on every occurrence — no memoization.
- Node allocation: `get_node` (mem.rs:163) walks/coalesces the lo-mem free list and unconditionally stamps 2 synctex trailer words per MEDIUM+ node (mem.rs:245–248). 107 `get_node` + 123 `get_avail` static call sites.
- **Innermost tax:** every field touch is a bounds-checked `self.data[p as usize]` (state.rs:487–628). Static sites: 1365 `.b32s1`, 808 `.set_b32s1`, 247 `.b16s1`, … executed billions of times dynamically. Zero `get_unchecked` in the crate.
- **Shaping:** `measure_native_node` (ext.rs:1484) runs HarfBuzz/Graphite `layoutChars` + ICU bidi per native word, ~5 xcalloc/free pairs per word (ext.rs:1531–1646). Per-glyph metrics are *already* cached at the C++ layer (`getCachedGlyphBBox`/`cacheGlyphBBox`), so the redundant cost is the **shaping + bidi**, not the bbox lookups. **No shape-run cache** keyed on (font, text, features): identical paragraphs are re-shaped (re-`layoutChars` + re-bidi) from scratch every keystroke.

**C. Shipout (per-page):**

- `ship_out_in` (shipout.rs:537) walks and emits the full box for every page; DVI pushed **one byte at a time** through `emit_dvi_out → dvi_out` (shipout.rs:525) with a per-byte bounds+limit check; per-call throwaway `Vec<u8>` in `dvi_native_font_def`/`movement` (shipout.rs:402,412,2556). Per-page BOP byte offsets already tracked via the `last_bop` back-chain (shipout.rs:690–697).

**The takeaway:** for a live editor, bucket A should be paid once per session, bucket B should be paid only for changed paragraphs, and bucket C should be reused for unchanged pages. Tiers 1–3 below attack A, then B/C, then the snapshot substrate that unifies them.

---

## 2. Tier 1 — Cheap wins now

These require no incremental architecture; they cut fixed and micro-hotpath cost, and several are prerequisites (or standalone hedges) for Tiers 2–3.

### 2.1 Resident warmed engine (do not drop the state)

**Mechanism.** The whole engine is one owned `Box<EngineState>` (engine.rs:997). The post-format-load point at **engine.rs:1310/1476** — immediately after `load_fmt_file`, before `start_input`/`main_control`, `history == SPOTLESS` — is a clean, document-independent boundary where the state is a pure function of the format bytes and the raw-pointer fields (font engines, file handles) are still null/empty. Capture ONE resident "golden" `EngineState` there and, on each recompile, `clone_from` the working state from golden instead of re-running alloc + undump.

`clone_from` **reuses the working Vec allocations** (no free/realloc, no file I/O, no undump, no byte-swap). It **converts cold undump (~100–300 ms) into a warm memcpy** of the dense arenas (~216 MB) plus the small live prefixes ≈ **~20–25 ms** (and, with the paged-COW/Arc layer of §4.3, potentially much less for *repeated* restores).

**What this does and does not buy.** It removes the alloc + undump + byte-swap from every run, but the restore is itself a ~20–25 ms memcpy — i.e. *still bucket-A-sized*. It **does not by itself put the per-keystroke path inside a 16 ms frame**; it converts cold startup into a warm memcpy. Keystroke-scale latency requires the incremental substrate of §4.3 (COW/word-log) and §4.4 (checkpoint-bracketed fast path) layered on top.

**Impact:** high (removes cold undump/alloc from every recompile). **Effort:** high (needs the snapshot infra of §4, plus refactoring `tt_run_engine`'s setjmp/`EngineAbort` boundary from a per-run to a per-recompile call boundary — the ongoing `EngineAbort`/`Result` work is the prerequisite). **Risk:** the raw-pointer fields must not be bit-copied; at the golden point they are null/empty so the snapshot is clean, but use a purpose-built arena snapshot, not `derive(Clone)`. Validate against the corpus dashboard — any restore divergence silently corrupts output.

### 2.2 Format-load elimination (undump exactly once per session)

**Mechanism.** The undump is **fully deterministic** — identical bytes, identical resulting arena image, every run (format_io.rs:885). Once the golden state is resident (§2.1), the format file is never re-read: the ~24–27 MB read + per-word byte-swap (format_io.rs:143–157) + RLE/sparse/free-list scatter (format_io.rs:1128,1195,1084) runs exactly once per editor session.

**Impact:** high. **Effort:** medium (subsumed by §2.1). **Risk:** an in-RAM golden clone avoids the on-disk format-compat problem entirely; do **not** persist a raw arena image to disk unless you version-gate it on `.fmt` bytes + `FORMAT_SERIAL` + engine version + host endianness (a raw clone is host-endian only).

### 2.3 Arena allocation reuse / drop redundant zeroing

**Mechanism.** Two independent wins if you don't yet have §2.1:

- **(a) Reuse-in-place (a standalone near-term win in its own right — see roadmap #8):** keep the six big Vecs on the `ProcessingSession` and clear-in-place across the driver's rerun loop rather than free+realloc; the undump already rewrites the meaningful regions, so scope the reset to `[0..mem_end]` / the eqtb dumped ranges. This removes **~120 MB/pass of allocator+memset traffic** with *no snapshot machinery at all*, and is worth landing independently of the resident-engine work.
- **(b) Skip C-xmalloc zeroing:** mem/str_pool/str_start were `xmalloc` (uninitialized) in C (format_io.rs:949, engine.rs:1075). Allocate `with_capacity` + a controlled uninit fill only where TeX requires a defined free region, dropping ~55 MB of memset the C engine never did.

**Impact:** (a) high; (b) **low–medium** — ~55 MB memset ≈ ~5 ms, and it is fully subsumed by §2.1 the moment the golden clone lands. **Effort:** medium. **Risk:** (a) is low (must reset exactly the dirtied words / re-zero the live prefix or stale state leaks). (b) is **high-risk** — it requires *proving* TeX never reads those words before writing them (the free-list/avail management should guarantee this but it must be audited; getting it wrong is UB / nondeterministic output), for a payoff §2.1 already captures. Treat (a) as an unconditional near-term win; treat (b) as low-priority and skip it once §2.1 lands.

### 2.4 Fold O(EQTB) re-init loops into one-time construction

**Mechanism.** `initialize_more_variables_in` (engine.rs:426) runs the 1,114,730-iteration `xeq_level` fill (engine.rs:503) plus hundreds of scalar resets on every run; the load path fills eqtb "undefined" entries up to hash_extra (format_io.rs:957). These recompute a fixed result each time. With golden restore they run once per session.

**Impact:** medium. **Effort:** low (as part of §2.1). **Risk:** low.

### 2.5 Right-size / grow-on-demand the over-allocated headroom

**Mechanism.** `font_info` is sized to `font_mem_size = 8,000,000` words = 64 MB (engine.rs:1025, format_io.rs:1249) though a LaTeX format's live `fmem_ptr` is a few thousand words. `hash_extra = 600,000` (engine.rs:1056) adds ~4.8 MB to both eqtb.data and eqtb.hash. In addition, the **~25 per-font side tables** (`font_check`/`font_size`/`font_dsize`/`font_params`/… , each sized `font_max+1 = 9001` and almost entirely zero) belong to the same right-sizing/reset story. Size `font_info` to `fmem_ptr + slack` and grow on demand (fonts.rs appends only via `fmem_ptr` in `read_font_info_tfm`:945 and `push_font_info_b32s1_in`:1063); size the side tables to the live font count; and/or make the caps configurable per document class.

**Impact:** medium (under §2.1 this becomes a memory-footprint concern — relevant when many warm sessions coexist — not a per-run time cost). **Effort:** medium. **Risk:** growth-on-demand touches the `fmem_ptr` overflow path; must preserve exact overflow-error semantics. `font_info` is *not* aliased by escaping node pointers (unlike `mem.data`), so growing it is safer than growing `mem`.

### 2.6 Micro-hotpath items

These are per-token constant-factor wins; land them when touching the accessors regardless of the incremental work.

- **Unchecked arena accessors (measure-first).** Every hot read is `self.data[p as usize]` (state.rs:487–628) — the union projection is already `unsafe`, but the Vec index still emits a length-compare + panic branch. A `get_unchecked`/`get_unchecked_mut` fast path behind a `debug_assert!(in-bounds)` (checked in debug/CI, skipped in release) would remove a compare+branch from every arena/eqtb touch **without changing any algorithm**. *But the expected payoff is modest* — removing a well-predicted branch from an already-`unsafe` projection typically yields low-single-digit %, and LLVM may already hoist the check out of the hottest loops. **Do not assume High impact; measure first** (disassemble the hot loops to confirm the bounds check is present and not hoisted before investing). *Risk (Med):* negative sentinels (`TEX_NULL = -0x0FFFFFFF`) currently `as usize`-wrap to a huge index the bounds check catches as a crash; an unchecked path turns a stray bad index into UB. Mitigate with the `debug_assert` in every accessor and audit that no accessor is ever called with a raw sentinel. **Impact low–med (unverified), effort medium — benchmark before touching.**

- **Gate the synctex trailer stamp.** `get_node` unconditionally writes 2 synctex trailer words for every MEDIUM+ node (mem.rs:245–248) — one per glue/kern/box/rule/disc/math/ligature, i.e. ~O(words) extra 2-word writes. For a live preview synctex output is off (or in-memory, §5). Cache a bool in `MemState` refreshed on `\synctex` assignment (the write funnels through `eq_word_define`/`geq_word_define`) and gate the stamp on it. *Bonus:* removing this stamp also makes two runs that differ only in source-line numbering produce **byte-identical `mem`** for identical layout — improving snapshot dedup across edits. *Risk:* verify all synctex readers are enabled-gated so the gate never corrupts a word a consumer reads unconditionally. **Impact medium, effort low.**

- **Cache `TRACING_COMMANDS` (and `TRACING_MACROS`).** Re-read on every loop iteration (main_control.rs:6353) and per expansion (expand.rs:1287,414,480,1600,1603) purely to decide whether to trace. Cache as a bool in `EngineState`, invalidated only on write to that specific int param (hook exists in `eq_word_define`). *Risk:* user-mutable mid-run (`\tracingcommands=N`); must invalidate on write to `INT_BASE+36` exactly. **Impact low, effort low** (worth it only bundled with the accessor work).

- **Bulk input read.** `input_line_in` reads one byte per `ttstub_input_getc` FFI call (io.rs:505–523,756); TFM parse does the same (fonts.rs:276). A bulk `ttstub_input_read` into a line buffer (the bridge already exposes it — format_io.rs:29) + in-Rust newline/encoding scan removes millions of cross-language calls per compile. *Risk:* must preserve exact CR/LF/EOF/`skip_next_lf` handling (io.rs:506–511) and the ICU/UTF-16 BOM peek-and-seek (io.rs:309–333); behavior-testable against the corpus. **Impact medium, effort medium.**

- **Bulk DVI emit.** The DVI hot path pushes each byte through `dvi_out` (shipout.rs:525) with a per-byte bounds+limit check, and `dvi_native_font_def`/`dvi_font_def_bytes`/`movement` allocate a throwaway `Vec<u8>` per call then re-loop byte-by-byte (shipout.rs:807–809,823–825,330–332). Emit whole slices with a single bounds check + `copy_from_slice`, splitting only at the `HALF_BUF` boundary, and reuse a scratch buffer in `ShipoutState`. *Risk:* must reproduce the `dvi_swap` disk-flush at the same byte boundaries (shipout.rs:502) so `last_bop`/`page_loc` offsets stay identical; verify byte-identical XDV against corpus. **Impact medium, effort medium.**

- **`match (mode,cmd)` dispatch.** C dispatches on one dense `abs(mode)+cur_cmd`; the port matches `(mode,cmd)` where mode is sparse `{1,104,207}` (main_control.rs:6361,6391), likely lowering cold arms to a comparison tree. Reconstructing the dense index `k = mode.unsigned_abs() as i32 + cmd as i32` restores O(1) dispatch for the cold-but-frequent assignment/math arms. *Risk:* readability regression and the compiler may already lower acceptably — **needs a disassembly/perf check first.** The hot text path is already hoisted, so typesetting is unaffected. **Impact low, effort medium — measure before touching.**

---

## 3. Tier 2 — Caching & memoization

Grounded in the comemo/salsa/Adapton analysis: a compiler function's *real* dependency set is far smaller than the global state it *could* touch, so naive "re-run if any input changed" never fires. The three memoization families differ only in how they discover the true dependency set — comemo replays recorded fine-grained getter calls (`Tracked<T>` + 128-bit SipHash constraints, constraint.rs), salsa walks red-green dependency edges with backdating and durability tiers, Adapton demand-cleans a named computation graph. The XeTeX-specific lesson from all three: **the front-end (`main_control` reads AND writes eqtb on nearly every token) has no natural pure-function boundary — memoize the back-end units that are nearly pure given a small parameter slice.**

### 3.1 Shape cache (the single highest-leverage change; no snapshot machinery needed)

**Mechanism.** `measure_native_node` (ext.rs:1484) is the one choke point where all shaping happens, and its output is a **pure function** of (font-engine identity, the UTF-16 text run, `use_glyph_metrics`, `font_letter_space_entry(f)` at ext.rs:1651). It has no date/time input — **safe to cache unconditionally.** For a live editor 99% of paragraphs are byte-identical between keystrokes yet each recompile re-shapes them all (driven from main_control.rs:5579,5603,5683; linebreak.rs:1037,3989,4036; packaging.rs:2309).

**Scope of the win.** The saved cost is the **`layoutChars` shaping pass + the ICU bidi run** per node — *not* the per-glyph metric lookups, which the C++ layer already caches (`getCachedGlyphBBox`/`cacheGlyphBBox`). Don't double-count the metrics; the redundant work being eliminated is shaping+bidi.

Add a cache keyed on `(font_number, letter_space, use_glyph_metrics, hash+bytes of the UTF-16 run)` → `(width, glyph_count, packed glyph_info blob)`. On hit, memcpy the blob to `mem[node+5].ptr` and set the exact three writes at ext.rs:1595–1597/1640–1642 (`mem[node+1].b32.s1 = width`, `mem[node+4].b16.s0 = glyph_count`), skipping the ICU bidi + shaper calls entirely. Key on the **text bytes** (not node identity) so it survives across compiles even though the node arena is rebuilt each run. For cross-run keys include the font-spec string (name/var/feat/size), since `font_number` only captures the baked-in feature set *within* a run.

This one change also absorbs the extra re-measures automatically: the `tail_is_native` merge re-measure (main_control.rs:5560–5583), `merge_justifiable_native_words` (shipout.rs:993), and `store_justified_native_glyphs` (ext.rs:2277) all become cache hits on same font+text. The interword-space temp-node measure (main_control.rs:5656–5693) discards its result after reading one width — memoize it width-only on `(font, pp_text, tail_text)`.

**Impact:** high (dominant redundant cost across recompiles). **Effort:** medium. **Risk:** exclude when `font_letter_space_entry != 0` unless letter_space is in the key; glyph blob layout must match `measure_native_node` exactly (locations `FixedPoint[]`, then u16 glyph ids). Determinism preserved.

### 3.2 Font-engine and TFM cache (cross-run)

**Mechanism.** `find_native_font_in` (ext.rs:1064) builds a fresh `XeTeXLayoutEngine` per `\font` through FreeType/fontconfig every compile (ext.rs:1109,1126,1139,1179,1187). `load_native_font_in` already dedupes *within* a run (fonts.rs:1110–1127) but nothing persists across runs. In a live loop the preamble's `\font` set is stable across keystrokes. Add a process-level cache keyed on `(name_string, var_string, feat_string, scaled_size)` → shared immutable `XeTeXLayoutEngine` (all key components already computed at ext.rs:1084–1092). Same for classic TFM: cache the parsed `FontState` tables per `(nom, aire, size)` and splice into the font arena rather than re-parsing byte-by-byte via `tfm_getc` FFI (fonts.rs:567, 276–326).

**Impact:** medium (font-engine), low (TFM — most modern XeTeX docs use native fonts). **Effort:** medium. **Risk:** the key must include var/feat/size; cross-run sharing requires engines be truly immutable (HarfBuzz shape state is per-call, so this holds). Font-file mtime is a determinism hazard in principle (installed fonts are effectively static for an editor; a correctness-conscious cache can validate mtime). Combined with §4's Arc-refcount, engines become shareable across both runs *and* snapshots.

### 3.3 Converged-aux caching (collapse the 2–3 pass rerun to 1)

**Mechanism.** `is_rerun_needed` (driver.rs:1350) returns a rerun reason **only** when a ReadThenWritten file's `read_digest != write_digest` (driver.rs:1357) — i.e. the `.aux`/`.toc`/`.out` changed. For the overwhelming majority of live-preview keystrokes (editing body text, not `\label`/`\ref`/`\section`/TOC), the regenerated `.aux` is byte-identical, so convergence needs exactly ONE pass. Today the driver still re-runs from a cold engine every compile because `bs.mem.files` is per-`ProcessingSession`.

Two levels, deployable independently:

- **Standalone (no resident loop):** the digest-compare of driver.rs:1357 already tells you, *within a single ProcessingSession's rerun loop*, whether a further pass is needed. Wiring it to skip the extra pass is a this-quarter win requiring no lifecycle refactor.
- **Full steady-state benefit (needs the resident loop, §2.1):** persist the converged in-memory `.aux`/`.toc` (`bs.mem.files` survives pass→pass, driver.rs:1723) and the events map across edits; on each edit run a single pass seeded with the cached `.aux`, and fall back to the multi-pass loop only when the freshly written `.aux` digest diverges. This drops steady-state keystroke cost from 2–3 passes to 1.

**Impact:** high. **Effort:** medium. **Risk:** low — the digest compare already gives the exact structural-edit signal. A wrong identity assumption would falsely declare convergence and freeze cross-references, so key on buffer identity/version, not just name.

### 3.4 Page/box reuse (incremental XDV) — later stage

**Mechanism.** `ship_out_in` (shipout.rs:537) emits the full box for every page; `finalize_dvi_file_in` (shipout.rs:2125) rewrites the whole stream. The per-page BOP byte offset is already tracked (`page_loc = dvi_offset + dvi_ptr`, last_bop back-chain at shipout.rs:690–697, BOP/EOP at shipout.rs:691,742). Hash each page's emitted byte range and, on recompile, skip re-writing unchanged trailing pages / patch only changed ranges. Coarser and higher-value first: cache the shaped+broken paragraph node lists so re-shipout is cheap.

**Impact:** medium. **Effort:** high. **Risk:** page content depends on `\count0..9` written into BOP (shipout.rs:692–694) plus float/mark migration, so "unchanged source" ≠ "unchanged page" — needs a dependency check. XDV consumers expect a valid POST/POST_POST/font-def trailer (shipout.rs:2146–2182); splicing must rebuild it. **Do this after shape caching**, and pin the determinism hazards in §3.5 first.

### 3.5 Determinism / cache-key analysis (the gate for any output cache)

A cache key is only sound if it folds in every hidden input. Enumerated from the findings:

- **`build_date`** → `\time/\day/\month/\year` via `get_date_and_time` (engine.rs:1330, texmfmp.rs:54) and `\creationdate` via `init_start_time` (engine.rs:1251, texmfmp.rs:605). Already pinnable via `deterministic_mode` (driver.rs:1861).
- **`random_seed` — the sharp trap.** Seeded from `gettimeofday` (engine.rs:1449 = `micros*1000 + epochsecs % 1e6`), and `deterministic_mode` does **not** touch it (it only overrides mtime + absolute-path exposure, driver.rs:1861–1868). Any doc using `\uniformdeviate`/`\normaldeviate` (expand.rs:1186; scan.rs:4578) produces different output every run — poisoning any input-keyed memoization AND potentially oscillating the digest-based convergence check (driver.rs:1357). **Fix (one expression, low risk):** derive the default seed from `build_date` (already threaded in as `libc::time_t`, engine.rs:989/1330) instead of `gettimeofday`; `\setrandomseed` (ext.rs:2805) still lets docs override. Good news: `ErrorState.randoms`/`j_random`/`random_seed` live *inside* `EngineState` (state.rs:1460–1479), so they clone correctly across snapshot/restore.
- **`gettimeofday`** → `\elapsedtime`/`get_microinterval` (texmfmp.rs:77).
- **Live-FS reads:** `\filemoddate` (texmfmp.rs:634–646), `\filesize` (texmfmp.rs:378), `\filedump` (texmfmp.rs:419), `\mdfivesum`, `\pdffilesize`.
- **Input file contents** + the primary input; **shell-escape** output (non-reproducible).
- **SyncTeX** embeds file paths + timing (`synctex_sheet` at shipout.rs:544). Disable or exclude its byte ranges from page hashing.
- **Safe constants:** DVI PRE comment `"tectonic"` (shipout.rs:679), numerator/denominator/mag (shipout.rs:674–675). Shaping (§3.1) and font metrics have no date/time input.

**Correct cache key** = `hash(format bytes, build_date, primary input digest, every read-input digest, random_seed, shell-escape-disabled)`. The driver already computes per-file SHA256 digests (bridge_core lib.rs:487) — those are exactly the ingredients; the clock/random/shell inputs must be explicitly folded in.

*Aside (avoided re-work in the convergence path):* convergence re-hashes full file contents every pass (bridge_core lib.rs:468–512). For a resident preview that persists digests, a file with unchanged buffer identity can skip re-hashing (driver.rs:1357). Low impact, low effort; key on buffer identity/version, never name, or you falsely declare convergence. **Impact low.**

### 3.6 Where comemo/salsa fit — and where they don't

- **comemo** drops into imperative code with a wrapper: put `EngineState.eqtb`/`.mem` behind a `#[track]` trait whose methods are TeX's fine-grained reads (`catcode(c)`, `count(n)`, `int_par(p)`, `cur_font()`, `meaning(cs)`). Memoize a **back-end** unit — `line_break` (an hlist + `\hsize`/`\tolerance`/`\hyphenpenalty`/`\language`/font → a vlist of line boxes) or the page builder — so an edit elsewhere that leaves those getters equal reuses the cached `mem` subtree. `mem` boxes are position-independent ("movable"), so a cached hbox/vbox is reusable wherever later placed. Assignments are comemo's `mutable` calls and must go through the tracked surface. The single-owned `EngineState` is the enabler — `Tracked` needs one value to wrap, which `static mut` made impossible.
- **The wall:** `main_control` reads *and writes* eqtb on nearly every token, so tokenization/expansion touches a huge ever-mutating getter set — comemo is viable only for back-end units fed an already-tokenized hlist, **never for memoizing expansion itself**.
- **salsa** wants a DAG of pure queries over explicit inputs — the polar opposite of `main_control`'s sequential read-write loop; realistic only for a re-architected front-end or the back-end queries. Its **durability tiers** map beautifully, though: the `.fmt` boundary is the top durability tier (preamble/class/hyphenation = high, body counters/text = low). Its **backdating** (re-typeset a paragraph, if line boxes come out byte-identical don't invalidate downstream) is exactly what an incremental TeX wants.
- **Adapton's** best transferable idea is **naming** for the "insert one line, everything shifts" case: name each paragraph/page by its SyncTeX source-line tag (`MemState.cur_synctex_tag`) or content hash rather than positional index, so a paragraph merely pushed down keeps its cached boxes via nominal match — **but** naming is sound only if the named unit's eqtb regime (fonts, catcodes, counters printed via `\the`) is captured, so it must be paired with comemo-style constraint checking (counters like page/section numbers usually *do* change).

**Bottom line for Tier 2:** ship §3.1 (shape cache) and §3.3 (converged-aux) now — they need no snapshot machinery and no dependency framework. Treat comemo-style back-end memoization as a Tier-3-adjacent research track, and pin §3.5 determinism before any output-level cache.

---

## 4. Tier 3 — Incremental compilation via EngineState snapshot/restore (the heart)

The realization from all prior art: TeX has no stable, reusable intermediate typesetting state (Mädje/Haug: "any seemingly local change can alter all subsequent typesetting decisions"). The honest, pragmatic model is therefore **not** fine-grained dependency tracking of expansion but **prefix-checkpoint + forward replay**: snapshot `EngineState` at clean boundaries, restore the nearest checkpoint before the edit, replay forward. This is fundamentally **TeXpresso's checkpoint-and-replay model** (§4b), realized with an in-memory clone instead of `fork()`.

### 4.1 (a) What blocks a cheap clone — the non-POD/pointer fields

`EngineState` and every sub-state derive only `Default`, never `Clone` (state.rs:8; no `impl Clone` anywhere). The heavy core is trivially memcpy-cloneable — `MemState.data`, `eqtb.data`, `eqtb.hash`, `xeq_level`, `str_pool`, `font_info` are `Vec` of `Copy` types and **all cross-refs are i32 indices**, so the graph survives a memcpy to a new base. The obstacles are three classes of owning pointer that a naive byte-clone would alias (→ double-free / UAF), not slowness:

1. **`mem[].ptr` native-glyph buffers.** `mem[p+5].ptr` holds an `xcalloc`'d glyph-info buffer (`copy_native_glyph_info` mem.rs:595–609, written ext.rs:1597,1642, accessor node.rs:2242), `libc::free`'d at packaging.rs:587. A `Vec<MemoryWord>` byte-clone copies the pointer → two arenas own one buffer → double free. **These sit INSIDE the same `mem` arena**, so any paged-COW that shares a page carrying such a pointer re-creates the aliased-owner double-free.
2. **FFI font handles.** `FontState.font_layout_engine` and `font_mapping` (state.rs:923,927) are `Vec<*mut c_void>` of `XeTeXLayoutEngine`/`TECkit_Converter`, freed per-slot in `tt_cleanup_in` (engine.rs:1511–1521 via `release_font_engine`/`TECkit_DisposeConverter`).
3. **Owned C strings & live OS handles.** `SynctexCtxt.root_name` (state.rs:83), `TexmfState.last_source_name` (state.rs:135), `InputClusterState.name_of_input_file` (state.rs:272); and live fds — `input_file: Vec<*mut UFile>` (state.rs:199), `read_file: [*mut UFile;16]` (state.rs:266), `write_file` (state.rs:1359), `dvi_file`/`log_file`/`rust_stdout` (state.rs:1387,1362,1361), `SynctexCtxt.file` (state.rs:82) — each with a read/write cursor. Duplicating the state gives two engines sharing one fd/cursor.

**Copy-safe** (shallow-copy fine): `*const c_char` help fields pointing at `c"..."` static rodata (`help_line` state.rs:1462, `max_reg_help_line` state.rs:897), the fn-ptr recorder (state.rs:86), and `ENGINE_STATE_PTR` (a per-run *locator*, not state — re-install on restore, state.rs:1646). The FONT_* FFI mirrors (globals.rs:308–334) point into the *original* Vecs — after clone/restore they must be refreshed (`refresh_font_ffi_mirrors`) or an OT-math callback reads freed memory.

**Do NOT `derive(Clone)`.** Build a purpose-made `fn snapshot(&self) -> Snapshot` that (1) deep-copies the POD arenas, (2) Arc-shares or rebuilds the FFI-owning ones (§4.2), (3) journals fds *including each open handle's byte offset* (§7.5), (4) asserts the raw-pointer fields are in a known state. The correctness bar: missing **any one** owning-pointer class yields a double-free/UAF that only triggers at teardown or when a shared node is freed in one clone — needs an exhaustive audit of every `mem[].ptr` writer (grep `set_ptr`) and every FFI Vec freed in `tt_cleanup_in`. And, as an explicit prerequisite: **audit xetex-xetex0.c for any residual `static`/C-global that survives a call** — the "whole-struct clone captures everything" claim (§7.1) holds only if no C-side cross-call state exists beyond `mem[]` writes.

### 4.2 The FFI-ownership refactor (the enabler for safe sharing)

Two moves make `mem` and the font tables safe to share:

- **Arc-share font engines/mappings** in a refcounted side table keyed by font number (immutable after load). A snapshot clones a cheap handle; the last dropped snapshot frees the resource — replacing the per-slot free loop in `tt_cleanup_in` (engine.rs:1511–1521) with refcount-drop. The Arc drop must call the correct C disposer exactly once. *Alternative:* keep engines as a **rebuildable cache keyed by `font_name`/`font_area`** (state.rs:965,966) and re-load lazily on restore — trades restore latency for zero snapshot cost. This composes with the §3.2 font-engine cache.
- **Move glyph-info blobs out of the union** into a separate owned arena addressed by *index*, so `mem.data` becomes plain-`Copy` and COW-cloneable. High blast radius (touches every reader: shipout.rs:2266, ext.rs measure/store paths, node.rs:2242) but it is the load-bearing enabler — and it is a **hard prerequisite for any paged-COW of `mem`** (§4.3), since a shared page carrying such a pointer re-creates the double-free.

### 4.3 (b) Checkpoint substrate options

First, a size reconciliation, because it drives the whole per-keystroke cost story:

- `st.mem.data` is a 40 MB flat word array (mem.rs:22), but the live working set is only `var_used + dyn_used` words near the two frontiers; `store_fmt_file_in` already serializes just `[0..lo_mem_max]` + `[hi_mem_min..mem_end]` (format_io.rs:236). `str_pool`/`str_start`/the used prefix of `font_info` are likewise sparsely occupied. These **can** be live-prefix-bounded to a few MB.
- **But eqtb.data (76.3 MB) + eqtb.hash (76.3 MB) + font_info full extent are dense** — the Unicode tables + RLE/sparse dump image fill them. They **cannot be shrunk by a live-prefix bound.** They are ~216 MB and read-mostly after format load.

**Consequence:** a "full" snapshot is **not** "a few MB" — it is dominated by the ~216 MB dense arenas, ≈ **~20 ms per memcpy**. Live-prefix bounding only trims the *sparse* arenas (mem/str_pool/str_start/font_info-prefix); it does **not** touch the dense 216 MB. So **naive per-keystroke full/bounded cloning eats most of a 16 ms frame on its own**, and the per-keystroke path genuinely needs the **COW/Arc layer** below (Research-tier) — *not* the watermark reset (which serves a different, narrower case, §4.4 step 3).

Options, ranked for THIS codebase:

| Option | Snapshot | Restore | Portable? | Verdict for XeTeX |
|---|---|---|---|---|
| **Repurpose format dump/undump** | O(arena): RLE encode + byteswap heap-copy | O(arena): alloc+zero + byteswap + RLE expand | yes (on-disk) | **No.** Size-optimized, not speed-optimized; 10–30× slower than a flat clone; **omits every mid-document stack** (§4.6). Keep strictly for `\dump`/preloaded-format. |
| **In-memory whole-state clone** | **~20 ms full memcpy** (~216 MB dense arenas that *cannot* be prefix-bounded, + small live prefixes of the sparse arenas) | memcpy back into the Box + mirror refresh + file re-seek — **no pointer fixup** (i32 indices) | yes | **Baseline correctness path.** Correct today with the ~10 raw-pointer fields handled (§4.1). Note: ~20 ms is still bucket-A-sized — too slow to repeat every keystroke. |
| **Live-prefix bounded memcpy** | trims only mem→`max(lo_mem_max, mem_end−hi_mem_min)`, `str_pool`→`pool_ptr`, `str_start`→`str_ptr`, `font_info`→`fmem_ptr` (bounds already tracked: format_io.rs:354,413,416–422,573,577); **eqtb.data/eqtb.hash stay dense** | same | yes | **Marginal.** Shaves the sparse arenas but the clone stays ~200+ MB because eqtb+hash dominate. Useful, not decisive. |
| **Userspace paged COW / word-log** (instrument the accessors) | O(edit burst) | O(edit burst) | **yes (WASM-safe)** | **Best per-keystroke** — the only option that makes *repeated* snapshots cheap. See below; feasibility gated on the pointer-escape audit. |
| **OS COW: `fork()`** | tens of µs (PTE dup, sub-ms) | O(1) discard child | Linux/macOS only | Only if the engine runs as a **subprocess**; entangles fds + C globals + longjmp path. This is TeXpresso's mechanism. |
| **`mmap` + `mprotect`/SIGSEGV, `userfaultfd` WP, soft-dirty PTE** | µs–ms | O(dirty pages), needs stored pre-images | Linux only | Strictly worse than instrumenting the accessors, which we already route every write through. |
| **CRIU** | I/O-bound full RSS | tens of ms process rebuild | Linux only | Too heavy for keystrokes; only "suspend/resume engine across sessions". |
| **Persistent DS (im-rs / RRB / HAMT)** | O(1) | O(1) | yes | **Disqualified for `mem`.** Turns each inner-loop indexed load into a 4-level pointer chase + refcount — cache-hostile constant-factor regression on the hottest path. Reserve for cold side tables. |
| **Content-defined chunking (FastCDC)** | ~30 ms full scan | O(chunks) | yes | **Wrong tool.** CDC resynchronizes a *shifting* byte stream; `mem` has stable absolute i32 indices, so fixed-block Merkle hashing dominates it. Only for disk dedup of accumulated `.fmt` checkpoints. |

**The recommended per-keystroke substrate — userspace instrumentation at the accessor choke point.** Because every arena write already funnels through `MemState::set_b32s1`/`set_b16s1`/`set_b32s0`/… (~1200 static sites) and `get_node`/`free_node` — there is no scattered raw `mem[i] = x` — you can slot in journaling with no signals, no FFI, no Linux dependency:

- **Word-granular undo log (fine variant, best match to TeX's mutation shape).** On each write within an epoch, append `(index: i32, old_word: 8B)`, gated by a "touched" bitmap so only the FIRST write to each index is logged (~640 KB bitmap for 5M words caps growth). Snapshot = record log length (O(1)); restore = replay backward (O(#distinct words changed)); redo = truncate. This exactly captures TeX's **scattered single-word free-list link writes** (`rover`/`avail`) which page-COW would over-copy at 4 KB granularity.
- **Chunk COW (coarse variant).** 512-word (4 KB) chunks, a per-chunk dirty bit, copy-on-first-write into a side buffer. Snapshot O(1), restore O(chunks touched). Simpler, but wastes a 4 KB copy on a single-word link write.

Both are O(edit burst) for snapshot *and* restore — exactly "localized bursts near the two frontiers." Use `store_fmt_file_in`'s live-extent bookkeeping as the **coarse baseline/checkpoint** and layer the word-log deltas between checkpoints.

**The one real blocker for full write-capture:** if any residual C code (xetex-xetex0.c) still writes `mem[]` directly through the shared raw pointer, the accessor instrumentation misses it. Full capture requires the write path to be entirely in Rust (the ongoing threading work). Until then, either restrict to a pure-Rust write regime or fall back to a page-fault (mprotect/uffd) catch for C-side writes.

**COW for the dense-static arenas — unresolved-feasibility research, not merely "high effort."** eqtb.data (76.3 MB) + eqtb.hash (76.3 MB) + font_info (64 MB) are ~78% of the footprint, and read-mostly after format load (the 53.5 MB Unicode cat/lc/uc/sf/math/del regions, consts.rs:47–55, are effectively immutable — a doc changes a handful of catcodes/registers, not millions). They can't be live-prefix-bounded (dense), so the *only* way to make *repeated* per-keystroke snapshots viable is to share their unchanged pages across snapshots instead of re-memcpying ~216 MB (~20 ms) each time.

**But the naive framing "Arc-share / paged COW" is oversold if presented as an effort estimate:**
- `Arc<Vec<MemoryWord>>` + `make_mut` is **whole-buffer** COW: the *first* write copies all 76 MB, giving zero incremental benefit. It is not a real per-page mechanism.
- Genuine *paged* COW requires a **custom paged arena** — which **breaks the flat-`Vec` ABI** that the escaping raw pointers depend on (`as_u16_ptr`/`node_addr` state.rs:598–616; FONT_* mirrors globals.rs:308–315). `make_mut`/any page move also violates the load-bearing "never reallocate" invariant and invalidates the mirrors.

So paged COW of the dense arenas is **unresolved-feasibility research, gated on first eliminating every raw-pointer escape** (an explicit audit prerequisite) and interposing all page-move-triggering paths on the `set_*` choke point with mirror-refresh discipline. Downgrade its confidence accordingly (roadmap #20): it is the mechanism that *would* make repeated snapshots cheap, but it is not a green-lit engineering task.

**str_pool/str_start — the O(1) freebie (do this first).** `StringsState` (state.rs:1521–1535) is strictly **append-only** within a run (`pool_ptr`/`str_ptr` only grow; `init_pool_ptr`/`init_str_ptr` already record the post-load high-water mark for the format dump). No interior mutation, no owning pointers. Snapshot = save the scalar tuple `(str_ptr, pool_ptr)`; restore = truncate logical length (no copy of the shared prefix). This is the lowest-effort structural-sharing win and the ideal **proof-of-concept for the snapshot harness** before tackling mem/eqtb. *Risk:* confirm nothing rewrites `str_pool` below `pool_ptr` mid-run (TeX strings are immutable once flushed — low risk).

### 4.4 (c) Concrete design: checkpoint after preamble / at structure boundaries, restore + replay forward

The key correctness fact: **the cost of a *restore* depends on where the *checkpoint* was taken, and soundness depends on the checkpoint bracketing the edited region.**

1. **Post-preamble checkpoint (the base case).** Run the preamble once — the expensive `\usepackage`/font-load/catcode cascade — and snapshot the `EngineState` at the first outer-vmode boundary, where `save_ptr == 0` and the nest is shallow. This is *precisely* the quiescent state the format dump already requires (it hard-refuses `save_ptr != 0`: "You can't dump inside a group", format_io.rs:239), so the omitted stacks are near-empty and even a bounded clone is well-defined. On each body edit: restore that snapshot and **replay the entire body from the buffer**. This is **O(tail-from-preamble)** — correct for any edit, but it re-runs all paragraphs after the preamble; it is *not* O(1) and does *not* have the accumulated body state of a mid-document edit.

2. **Ladder of per-section / per-paragraph checkpoints (the mechanism that enables cheap edits).** Keep a checkpoint at each structural boundary. An edit in section N restores section N−1's checkpoint — which **already contains the accumulated eqtb/nest/counter/`\def` regime as it exists at that point** — and replays only section N forward. Replay cost is O(tail-after-the-nearest-preceding-checkpoint). This is Strokov's texlite scheme (per-page core dump → restart from the affected page) realized with the in-memory clone instead of a disk core-dump, and with the differential/reference-dump idea mapped to the word-log deltas of §4.3. **This ladder is a *correctness* prerequisite for the per-paragraph fast path below, not a later scaling optimization** (see step 3).

3. **Fast-path arena-reset (re-breaking the *same* paragraph, per keystroke).** This is the O(1) restore — but it is **valid only for re-breaking a paragraph against a checkpoint taken *immediately before that paragraph***. It is **not** achievable from a post-preamble checkpoint alone: line-breaking paragraph N requires the eqtb/nest/counter regime **as it exists at position N** (accumulated font changes, `\parindent`/`\baselineskip` inside environments, list depth, counters, macros from paragraphs 1..N−1). Re-breaking N against post-preamble-only state produces **wrong output** for any N after a state-mutating paragraph.

   Given a checkpoint bracketing the paragraph (the ladder of step 2, or an O(tail) forward replay to just before it), a pure body paragraph only *allocates* nodes and pushes/pops a *balanced* save stack without mutating global eqtb, so the "restore" is an **O(1) reset of the arena high-water marks and the small POD sub-states** back to the pre-paragraph values — discarding every paragraph node at once, no clone, no undump. But the rewind set is **larger than just the mem frontier**; to avoid leaking state into the next keystroke you must reset **all** of:
   - `mem` frontier: `lo_mem_max`, `hi_mem_min`, `rover`, `avail` (mem.rs:74–88,214–236);
   - the save stack: `save_ptr`, `cur_level`, `cur_group`, `cur_boundary` (state.rs:695–709);
   - **the semantic nest**: `cur_list` head/tail/`prev_graf`/`prev_depth`/`space_factor`, `nest_ptr` (state.rs:200–218) — otherwise the second keystroke appends into a stale list;
   - **the page-builder / contribution list**: `page_contents`, `page_so_far`, `page_tail`, `page_head` (state.rs:1349–1367) — **otherwise the second keystroke appends a *second copy* of the paragraph's vlist to the page contribution list**;
   - **synctex position**: `cur_synctex_tag`, `line`, and (see §5) the `SynctexCtxt.synctex_tag_counter` (state.rs:96) — a desynced tag counter corrupts source↔preview mapping across keystrokes.

   The simplest robust implementation re-clones the small POD sub-states (nest, save, page-builder, scanner, synctex scalars — all a few KB) and resets the mem frontier, rather than hand-rewinding each field. **Sound only if the paragraph performed no global mutation** (`\gdef`, `\global`, counter writes, font `\let`); detect that and fall back to the bounded clone if violated (§7.9).

4. **Restore correctness — bundle the free-list state.** A `mem` restore must snapshot `data` **together with** `rover`, `avail`, `lo_mem_max`, `hi_mem_min`, `mem_end`, `var_used`, `dyn_used`, `cur_synctex_tag`, `line`, `temp_ptr` — all already inside `MemState` (state.rs:457–481) — or the free lists desync and `get_node`'s coalescing order (which depends on `rover`) diverges. Restore must **reuse the same-capacity Vec** (memcpy *into* it), never replace it, to keep escaped pointers valid.

### 4.5 (d) Why full incremental is hard, and the pragmatic boundary

TeX's front-end reads-and-writes one giant eqtb on every token via `\catcode`, `\the`, counters, and macros, so **everything transitively depends on everything** — there are almost no natural pure-function boundaries. Salsa's precondition (a DAG of pure queries) is the opposite of `main_control`; treating each eqtb slot as an input makes a single early assignment bump the revision for everything after it, defeating fine-grained reuse. comemo can only wrap the back-end. Adapton's naming needs the named unit's eqtb regime captured, which for page/section counters it usually is not.

**Therefore: do not attempt fine-grained dependency tracking of expansion.** The pragmatic boundary is **prefix-checkpoint + forward replay** (§4.4): checkpoint at structure boundaries, restore the nearest clean checkpoint before the edit, replay the tail. This is O(tail-after-edit), not O(1), but it is *honest about TeX's semantics* and it wins big for ordinary body-text edits (the dominant editing case) and near-zero for edits that perturb the catcode/expansion regime — which correctly fall back to a full recompile. Layer the back-end memoization (§3.1 shape cache, comemo-style `line_break` cache) *on top* of the replay to make the replayed tail itself cheap.

### 4.6 What the format writer OMITS (why clone, not format-reuse)

`store_fmt_file_in` captures only str_pool, mem free-list+hi-mem, eqtb+hash+prim, fonts, hyph — a **quiescent bottom-level** state. It OMITS exactly the mid-document "position" state a checkpoint needs, each marked "Not serialized" in state.rs: the input stack + `cur_input` + `input_ptr` (state.rs:180–191), `param_stack`/`param_ptr` (156,240), the file stacks (line/grp/if/source, `input_file`, `eof_seen`, `in_open`, `open_parens` — state.rs:163–171,199,212–216), the input buffer (172–178), the semantic nest + `cur_list` + `nest_ptr` (200–218), the **entire save stack** (`SaveStackState` 695–709: `save_ptr`/`cur_level`/`cur_group`/`cur_boundary`/`data`), the page-builder + contribution list (`ShipoutState` `page_contents`/`page_so_far`/`page_tail`/`best_page_break`/`output_active`/`dead_cycles`/`insert_penalties` 1349–1367), the conditional stack + align state + `scanner_status` (`ScannerState` `cond_ptr`/`if_limit`/`cur_if`/`cur_align`/`align_state` 791–831), and `xeq_level` (save-level per int-param, 300–301). Repurposing the format writer means hand-adding serialize/deserialize for all of these — a large bug-prone surface where missing any one (e.g. the save stack when snapshotting inside a group, or `xeq_level` for global-assign tracking) silently corrupts restored typesetting. A whole-`EngineState` clone captures them all — *provided* the "no C-side cross-call state" premise is verified (§7.1). **This is the single strongest argument for clone over format-reuse.**

---

## 4b. Synthesis of the Lode paper, and alignment/divergence

**What Lode ("Real-Time LuaTeX", TUG 2026 preprint) shows.** A 100-page book compiles in 10–30 s, but one paragraph line-breaks in <1 ms on a mid-range i7 (§3, Table 1: short 0.17 ms, medium 0.70 ms, long 1.99 ms, inline math 0.20 ms, display math 0.11 ms — "the dominant cost is line breaking, not content complexity"). Table 2 (500 consecutive compiles) shows **O(1) latency independent of document state** — no degradation. Table 4 (edit the same paragraph mid-document) shows LuaLaTeX **constant 0.70 ms at 10/100/300 pages** vs Typst 12.6/76/206 ms (**linear O(n)**). HCI budgets: 100 ms = "instantaneous", 16 ms = 60 Hz frame; 0.1–2 ms/paragraph sits inside the frame budget.

**Lode's core thesis:** the unit of compilation should NOT be the whole document. From Knuth-Plass (§4): "**Line breaking is local.** Given the same `\hsize` and content, TeX produces identical line breaks regardless of what appears before or after the paragraph," whereas "**Page breaking is global**… but it does not need to happen on the same keystroke." So the editing loop recompiles the ONE edited paragraph; page numbers/cross-refs/floats converge later in the background.

**Lode's four mechanisms (§5), built on vanilla TeX Live with NO engine modifications:** (1) **keep the engine alive** (startup ~1 s for a book preamble with OpenType fonts); (2) **capture output without PDF** — extract the display list directly from the post-line-break node structures (positioned glyphs in scaled points); (3) render that glyph stream to HTML5 Canvas via opentype.js; (4) **background convergence** — a periodic full compile produces a complete page-position cache onto which fast-path paragraphs are overlaid, converging page numbers/refs/floats "within seconds." The accepted tradeoff: "temporary inconsistency: pages the user is not viewing may lag until a background compile converges." Locality boundaries (§4): footnotes, counter-dependent content (`\ref`, `\thepage`), and `\parshape` set by surrounding code — these "fall to the background convergence path."

**Two distinct models — and which one this report's spine belongs to.** It is worth being explicit, because the report mixes them:

- **Lode's model** keeps a **fully-compiled resident engine** whose global state is held *fresh* by the **always-running background full compile**; fast-path paragraphs are overlaid on top of that continuously-refreshed state. Lode's global-state freshness comes from the background compile, not from any checkpoint.
- **The checkpoint-and-replay spine of §4.4** (post-preamble/section checkpoint → restore-nearest → replay tail) is **TeXpresso's model**, not Lode's. A pure checkpoint-replay must *reconstruct* the accumulated global state by **replaying the tail** (§4.4 step 2), because a checkpoint alone does not have the freshness Lode's background compile provides.

Merging the two silently inherits a soundness gap: the O(1) per-paragraph fast path (§4.4 step 3) needs either Lode's continuously-fresh resident state *or* a checkpoint bracketing the paragraph (the ladder). It is **not** deliverable from a post-preamble checkpoint alone. Keep the two models labeled.

**How Tectonic aligns.** Strongly, and it is a *better* substrate than Lode's LuaTeX prototype:

- **Persistent engine = don't drop the `Box<EngineState>`** (§2.1). Lode fights LuaTeX's batch file-in/file-out model with a persistent process; in Tectonic "keeping the engine alive" is literally holding the one owned struct — **no OS process, no `fork()`**. The setjmp/`EngineAbort` boundary must become a per-recompile call boundary (the ongoing `EngineAbort`/`Result` work is the prerequisite).
- **Per-paragraph line-break locality → O(1), *given a checkpoint bracketing the paragraph*** (§4.4 step 2+3). Restore the checkpoint taken just before paragraph N, write `\hsize` into eqtb, push the paragraph's tokens, drive `main_control` only far enough to reach `line_break` on that single hlist — the *same* `line_break` the batch engine calls, invoked in isolation with the correct accumulated regime. This reproduces Lode Table 2/4. It is **O(1) only relative to a per-paragraph checkpoint**; from a post-preamble checkpoint the honest cost is O(tail) forward replay.
- **Arena watermark reset** (§4.4 step 3) is the O(1) *restore of the same paragraph position* — and Tectonic's `mem[]` arena (`lo_mem_max`/`hi_mem_min`/`rover`/`avail`, mem.rs:74–88,214–236) is *tailor-made* for it. *This mapping is our engine-specific inference: the Lode paper does not specify the reset mechanism its LuaTeX prototype uses — do not cite Lode for it.* And note this is TeXpresso's checkpoint-restore primitive realized in-value, not a mechanism the Lode paper describes.
- **Display-list extraction bypassing XDV** (§5). Skip `ship_out` entirely; walk the post-`line_break` hlist/vlist in `mem[]` using the **typed node handles** (AnyBox/AnyNode from the phase-2 typing work) and emit `(glyph_id, x_sp, y_sp)` + rules/colors. **Tectonic's advantage:** because it's a Rust library with typed `mem[]` accessors, this is a direct typed read of the arena — not the "reverse-engineering LuaTeX's opaque node format" Lode had to do. *Burden:* output must be bit-identical to batch XDV (same glue_set rounding, same XeTeX protrusion/expansion/native-word positioning) or the preview diverges from print.
- **In-process = no IPC tax.** Lode's Table 3 charges IPC+serialization (0.12/1.03 ms) + deserialization (0.02/0.09 ms). Tectonic is an in-process Rust library: the paragraph-compile returns a Rust `Vec` of glyph structs read directly from `mem[]` — **zero engine-side serialize**, collapsing the ~0.12–1.03 ms IPC row to nearly nothing. Serialization is needed only at the actual UI boundary (WASM→JS or draw-to-canvas), one hop not two.
- **Local/global split + background convergence** (§5). Two engines: the fast path serves keystroke line-breaks from a resident checkpoint; a **separate cloned `EngineState`** runs a full compile (with `ship_out`) on a background thread to produce the page-position cache *and to keep the global regime fresh in Lode's sense* (§3.3 converged-aux). Clean value-semantics threading — enabled by the zero-global-state work — lets you `clone` the state and run the background compile without `fork()`. Cancel/restart it when a new edit lands.
- **Locality-boundary detection** (§4). In Tectonic these boundaries are reads of global eqtb: `\ref`/`\thepage` resolve from the `.aux` pass and the page counter; LaTeX counters are `\count` registers in eqtb; `\parshape` is `par_shape_ptr` in eqtb pointing into `mem[]`. Detect conservatively (scan the paragraph token stream for those primitives; check `par_shape_ptr != null`; check for a footnote insertion node) and route non-local paragraphs to the background full compile. Err toward the slow path.

**How Tectonic differs / improves on the paper.** Lode explicitly contrasts against **TeXpresso** (Bour, TUGboat 44(2)), which "forks the engine every ~500 ms to make a copy-on-write checkpoint and re-runs from the nearest checkpoint before an edit — but requires a modified XeTeX engine and `fork()`, limiting it to Linux and macOS." **Tectonic already has the primitive TeXpresso emulates with fork+COW, as a *value*, not a process image:** the format dump/undump is a full serialization of `EngineState`, and — better — the in-memory clone of §4 works on **Windows and WASM** where `fork()` is unavailable, deleting exactly the platform limitation Lode attributes to TeXpresso. Since this report's incremental spine *is* the checkpoint-and-replay model, TeXpresso is its closest ancestor; Tectonic's contribution is making that model portable and value-based. **Net:** Tectonic can implement Lode's four mechanisms *and* TeXpresso's checkpoint primitive, portably, with less reverse-engineering.

*One caveat where Tectonic differs from Lode's "no engine modifications" claim:* Lode achieves it because LuaTeX is already a resident scriptable process; Tectonic must first refactor the `tt_run_engine` setjmp/lifecycle into a per-recompile call boundary and land the FFI-ownership refactor (§4.2). That is real work — but it is the *same* work that unblocks the whole live-preview goal, and it is in-crate.

---

## 5. Live-preview editor architecture

Putting the tiers together into a resident-engine editor:

**Process/thread topology.**

- **One resident engine process** holding the warm `Box<EngineState>` (§2.1). No `fork()`. On session start: init → `load_fmt_file` → run the document preamble through `main_control` to the first outer-vmode boundary → capture the **post-preamble checkpoint**, then a **ladder of per-section/per-paragraph checkpoints** (§4.4 step 2) as the body compiles. Font engines loaded once (§3.2), shape cache warm (§3.1).
- **Fast path (foreground, per keystroke):** debounced (~16–50 ms after last edit) recompile from the **nearest checkpoint before the edit**, located via SyncTeX source-line tags. Restore = watermark-reset (O(1)) *only when the edit re-breaks a paragraph bracketed by a per-paragraph checkpoint and performs no global mutation*; otherwise restore the nearest section checkpoint and **replay the tail** (O(tail)); a whole-body edit falls back to the bounded clone (§4.3–4.4). For the edited paragraph, `line_break` runs in isolation against the restored accumulated regime; unchanged paragraphs hit the shape cache and (eventually) a comemo-style `line_break` cache.
- **Input source for the fast path is the in-memory editor buffer, not a re-opened file** (§7.5). Because the primary source is exactly what the editor changed between snapshot and restore, you drive `cur_input.loc`/`limit` from the editor's in-memory buffer; `ttstub tell()`/seek capture is reserved for the *stable included* files only.
- **Background convergence (separate thread, cloned `EngineState`):** periodic full compile with `ship_out` → page-position cache + converged `.aux`/`.toc` (§3.3), and — in Lode's sense — the mechanism that keeps the global regime fresh so fast-path overlays stay consistent. Cancellable/restartable on new edits. Eventual consistency for `\ref`/`\thepage`/floats.

**Source ↔ preview mapping (SyncTeX-driven, in-memory).** `synctex.rs` already records, for every shipped box/rule/glue/math node, its input-file `tag` (a per-`\input` counter, `SynctexCtxt.synctex_tag_counter`, state.rs:96) and source `line` (`synctex_tag_box`/`synctex_line_box`, synctex.rs:55–79; `record_node_*` from synctex.rs:276+). Today this is serialized to a gzipped `.synctex` file. **For a resident preview, add an in-memory sink alongside the file writer** so the loop answers forward (source→preview) and inverse (preview→source) position queries without round-tripping through disk. `synctex_enabled` is already plumbed (engine_interface.rs:19,29). These tags are also the **checkpoint-locator** and the **Adapton-style stable name** (§3.6). **The fast path must include `synctex_tag_counter` and `cur_synctex_tag` in its rewind set** (§4.4 step 3): a desynced tag counter corrupts source↔preview mapping across keystrokes even when the layout is otherwise correct. Rebuild/re-key the in-memory index per compile; invalidate when the input file set changes.

**Rendering.** Two options: (a) **XDV → preview** via the existing `ship_out` (bulk-emit per §2.6) for the background full compile, feeding a page renderer; (b) **display-list extraction** (§4b) for the fast path — walk the post-`line_break` list in `mem[]` via typed handles, emit positioned glyphs, draw to canvas, no XDV/PDF. The fast path draws the edited paragraph's glyphs at the cached page y-offset from the background page-position cache.

**Editing UX boundaries where checkpoints are cheapest.** The checkpoint is cheapest — and the fast-path arena-reset is *sound* — at **quiescent boundaries where `save_ptr == 0` and the nest is shallow**: outer vertical mode, between paragraphs, at the page-builder boundary (exactly the state the format dump requires, format_io.rs:239). So: snapshot **after the preamble** and **at each structural boundary** (`\section`, page break), and drive `main_control` to stop cleanly at `\par` without entering the output-routine/page-builder. Edits *inside* a group, mid-math, or that change catcodes/`\endlinechar`/active chars route to the slow path — a local reparse is unsound when the catcode/expansion regime up to the edit changes (§3.6, Typst incremental-reparse caveat). Debouncing hides the slow-path cost from the typing latency.

---

## 6. Prioritized roadmap

Ordered by impact/effort with dependencies. "This month" = low-risk, no new architecture. "Research" = deep engine surgery or unresolved feasibility.

| # | Item | §  | Impact | Effort | Risk | Depends on | Horizon |
|---|---|---|---|---|---|---|---|
| 1 | **Shape cache** on `measure_native_node` (saves shaping+bidi; metrics already C++-cached) | 3.1 | High | Med | Low | — | **Ship this month** |
| 2 | **Pin `random_seed` to `build_date`** (one expr) + fold clock/FS/shell into cache key | 3.5 | High | Low | Low | — | **Ship this month** |
| 3 | **Gate synctex trailer stamp** | 2.6 | Med | Low | Low | — | **Ship this month** |
| 4 | **str_pool/str_start high-water-mark snapshot** (harness POC) | 4.3 | Med | Low | Low | — | **Ship this month** |
| 5 | **Unchecked accessors** (debug_assert-gated) — **benchmark first** | 2.6 | Low–Med (unverified) | Med | Med | disassembly/perf check | This quarter |
| 6 | **Bulk input read** + **bulk DVI emit** | 2.6 | Med | Med | Low | — | This quarter |
| 7 | **Font-engine + TFM cross-run cache** | 3.2 | Med | Med | Low | — | This quarter |
| 8 | **Arena reuse-in-place** across rerun loop (standalone ~120 MB/pass win; drop the "fallback" framing) | 2.3 | High | Med | Med | — | This quarter |
| 9 | **Converged-aux → 2–3 passes to 1** (digest-compare prototype standalone; full steady-state benefit needs #10) | 3.3 | High | Med | Low | digest-compare (standalone); resident loop (10) for full benefit | This quarter (prototype); Next quarter (full) |
| 10 | **Refactor `tt_run_engine` lifecycle** into per-recompile call boundary (`EngineAbort`/`Result`) | 2.1 | High | High | Med | ongoing EngineAbort work | Next quarter |
| 11 | **FFI-ownership refactor:** Arc-share font engines + move glyph blobs out of the union (prereq for any `mem` COW) | 4.2 | High | High | High | — | Next quarter |
| 12 | **`EngineState::snapshot()/restore()`** (POD deep-copy of dense arenas, FFI handled, fds journaled **incl. per-handle `tell()` offset capture**, mirrors refreshed; **audit xetex-xetex0.c for C-side cross-call state**) | 4.1–4.3 | High | High | High | 10, 11 | Next quarter |
| 13 | **Post-preamble checkpoint + full-body replay** (resident warmed engine) | 2.1, 4.4 | High | High | High | 12 | 2 quarters |
| 14 | **In-memory SyncTeX index** for source↔preview (drives fast path from editor buffer, not re-opened file) | 5 | Med | Med | Low | 10 | 2 quarters |
| 15 | **Word-granular COW/undo-log** at accessors → per-keystroke O(edit-burst) restore | 4.3 | High | High | Med | 11, 12; pure-Rust `mem` write path | Research |
| 16 | **Per-section/per-paragraph checkpoint ladder** (Strokov-style) — **correctness prerequisite for the per-paragraph fast path** | 4.4 | High | High | High | 13, 15 | Research |
| 17 | **Per-paragraph line-break in isolation + arena watermark reset** (TeXpresso-style fast path; O(1) *only* vs a bracketing checkpoint) | 4b, 4.4 | High | High | High | 13, 15, **16** | Research |
| 18 | **Display-list extraction bypassing XDV** + canvas render | 4b, 5 | High | High | High | 17 | Research |
| 19 | **Background convergence** (cloned-state full compile thread + page-position cache; keeps global regime fresh) | 4b, 5 | High | High | High | 12, 13 | Research |
| 20 | **Paged COW of eqtb+hash+font_info** for repeated snapshots — **unresolved feasibility**, gated on eliminating all raw-pointer escapes (`as_u16_ptr`/`node_addr`/FONT_* mirrors) | 4.3 | High (if feasible) | Research | High | 11, 12, pointer-escape audit + elimination | Research |
| 21 | **comemo-style `line_break`/page memoization** (back-end constrained caching) | 3.6 | Med | High | High | 12 | Research |
| 22 | **Incremental XDV / page-range splicing** | 3.4 | Med | High | Med | 2, 9 | Research |

**Critical path to a shippable live preview:** 1 → 2 → 3/4 (cheap wins, harness POC) → 8/9 (arena reuse + single-pass loop) → 10 → 11 → 12 (safe snapshot, offsets captured) → 13 (post-preamble checkpoint + full-body replay) → 14 (mapping) → 16 (checkpoint ladder — the correctness base for cheap edits) → 17 (per-paragraph fast path) → 18 (render) → 19 (background convergence, global freshness). Items 15/20/21/22 make it *fast at scale*; item 16 is not merely scaling — it is the **soundness prerequisite** for 17 and must precede it. A resident engine with post-preamble checkpoint + full-body replay + shape cache already gives a large speedup over cold batch compilation; the checkpoint ladder + fast path then take it to per-keystroke latency.

---

## 7. Risks, correctness traps, open questions

### 7.1 The checkpoint must capture ALL live state

The dominant correctness trap. A snapshot that captures only the big arenas **leaks state between compiles**. The full mutation surface (§4.6) includes, beyond mem/eqtb/hash/strings/fonts/hyph: the input stack + `cur_input` + `input_ptr` (state.rs:180–191), `param_stack` (156,240), the file stacks + `input_file` + `eof_seen` + `in_open` + `open_parens` (163–171,199,212–216), the input buffer (172–178), the semantic **nest** + `cur_list` + `nest_ptr` (200–218), the entire **save stack** (695–709), the **page-builder** + contribution list (`ShipoutState` 1349–1367), the conditional stack + align state + `scanner_status` (791–831), and **`xeq_level`** (300–301). Because the zero-global-state work folded all hot *Rust* scalars into `EngineState`, a struct-level clone captures all of these — **but this rests on an unverified premise: that no C translation unit (chiefly xetex-xetex0.c) holds cross-call state beyond its `mem[]` writes.** Add an explicit action item (roadmap #12): audit xetex-xetex0.c for any residual `static`/C-global surviving a call (the memory notes still list FONT_* locators + `ENGINE_STATE_PTR`, and the setjmp/longjmp path touches C-side state). Until that audit passes, "zero global state" is an *enabler if true*, not a guarantee. Any hand-rolled subset snapshot must be audited against the §4.6 list; missing the save stack when snapshotting inside a group, or `xeq_level` for global-assign save-level tracking, silently corrupts restored typesetting.

### 7.2 Owning-pointer classes (double-free / UAF)

Enumerated in §4.1. Missing any one — `mem[].ptr` glyph buffers (mem.rs:595–609, freed packaging.rs:587), font engines/mappings (state.rs:923,927, freed engine.rs:1511–1521), owned C strings — yields a double-free/UAF that only triggers at teardown or when a shared node is freed in one clone. Requires an exhaustive audit of every `mem[].ptr` writer (grep `set_ptr`) and every FFI Vec freed in `tt_cleanup_in`. **Never bit-copy these; Arc-share or rebuild.**

### 7.3 The "never reallocate" invariant vs COW

`mem`/eqtb hand out escaping raw pointers (`as_u16_ptr`/`node_addr`, state.rs:598–616; FONT_* mirrors, globals.rs:308–315) and rely on the Vecs never reallocating. Therefore: (a) restore must memcpy **into** the same-capacity Vec, never replace it; (b) any COW page move (`make_mut` or a paged arena) must refresh the FONT_* mirrors (`refresh_font_ffi_mirrors`) and is forbidden mid-operation; (c) any `as_mut_ptr`/`node_addr` escape bypasses accessor-level dirty-tracking and must be audited. **This makes paged COW of the dense arenas (roadmap #20) unresolved-feasibility research, not merely "high effort":** `Arc + make_mut` is whole-buffer (76 MB first-write copy, zero incremental benefit), and a genuine paged arena breaks the flat-`Vec` ABI these escapes depend on. The pointer-escape audit + elimination is a hard prerequisite before #20 can even be scoped.

### 7.4 Free-list determinism

Restore correctness depends on the `rover` free-list pointer and `get_node`'s coalescing order (mem.rs:171–181). Snapshot `data` together with `rover`/`avail`/`lo_mem_max`/`hi_mem_min`/`mem_end`/`var_used`/`dyn_used` or two restored runs diverge. The middle gap can legitimately be touched by allocations *after* restore, so a bracketed clone must recompute the bracket, not assume it static.

### 7.5 File-handle / I/O state — and the editor buffer as primary source

Open fds (`input_file`/`read_file`/`write_file`/`dvi`/`log`/synctex, §4.1 class 3) can't be bit-copied. Two distinct cases:

- **The edited primary source is the editor's in-memory buffer, not a file.** Re-open+seek is *invalid* for it: the buffer is exactly what changed between snapshot and restore, so any cached offset is stale. The fast path drives `cur_input.loc`/`limit` directly from the in-memory editor buffer (you own it). This is the design, not a fallback.
- **Stable included files** (`\input`ed, unchanged) *can* use re-open-by-name + seek — but this requires capturing `ttstub tell()` **per open handle at checkpoint time** (offsets aren't stored anywhere today; the input stack records name/loc, state.rs:1085–1096, but not the byte offset). **This is new, required instrumentation folded into roadmap #12.** The `UFile` decode buffer means the seek must land on a **line boundary** — checkpointing between lines/paragraphs sidesteps this.

For the preview pipeline, route DVI/XDV/log to a null sink so those handles are absent from the snapshot. Snapshot only at an **input-quiescent boundary** (`in_open` shallow, no live `\openin`/`\openout`).

### 7.6 Determinism / replay reproducibility

Replay from a checkpoint must be reproducible. Trap sources: the `gettimeofday` `random_seed` (§3.5 — the sharp one; fix it first), `\elapsedtime`, `\filemoddate`/`\filesize`/`\filedump` live-FS reads, and shell-escape. Safe: `build_date`-derived `\time/\day/\month/\year` are set once at init (engine.rs:1330, so replay-stable); `ErrorState.randoms`/`random_seed`/`SynctexCtxt.synctex_tag_counter` live in `EngineState` so they clone correctly across restore. Side effects **outside** `EngineState` — `\write`/`\openout`/`\immediate` output, XDV shipout bytes — must be rewound or be pure functions of the replayed state (the whatsit/`ext.rs`/`io.rs` I/O paths are the leak for rewind-and-replay).

### 7.7 The C-write blocker

Userspace accessor instrumentation (§4.3) misses any residual direct `mem[]` writes through the shared raw pointer in xetex-xetex0.c. Full write-capture requires the write path entirely in Rust (the ongoing threading work). This is the write-side counterpart of the §7.1 cross-call-state audit: grep and quantify the residual C writers. Until the write path is pure Rust, restrict to a pure-Rust write regime or fall back to a page-fault (mprotect/uffd) catch — accepting the Linux-only, page-granular cost for the C-write set.

### 7.8 Tokenizer context-sensitivity (line-cache soundness)

Any line-level tokenization/`id_lookup` cache (hash.rs:45) is unsound without a **catcode-generation counter**: a `\catcode`/`\endlinechar` change or a macro redefinition that reshapes later tokenization invalidates verbatim retokenization of "unchanged" lines. Correct scope is "retokenization of physically unchanged lines whose active catcode regime is unchanged." Same gating applies to incremental reparse (§3.6).

### 7.9 Locality boundaries (fast-path soundness)

The per-paragraph fast path (§4b, 4.4 step 3) is sound only when (a) it is restored from a checkpoint **bracketing that paragraph** (so it has the correct accumulated eqtb/nest/counter regime — a post-preamble-only checkpoint is *unsound* for any paragraph after a state-mutating one, §4.4/§1.1), (b) the paragraph performs no global mutation, and (c) it reads no global counter/page state. Detection must **err toward the slow path** — a missed dependency (footnote coupling page breaking, `\ref`/`\thepage`, `\parshape` from surrounding code, `\gdef`/`\global`) produces a paragraph that looks locally correct but is globally wrong until the next background convergence. Footnotes are the hardest (they alter page breaking, not just paragraph metrics).

### 7.10 Bit-identical output burden

Both display-list extraction (§4b) and incremental XDV (§3.4) must produce output **bit-identical** to the batch XDV — same glue_set rounding, same XeTeX protrusion/expansion/native-word positioning — or the live preview diverges from the final print. Validate every snapshot/cache change against the corpus dashboard (passed+diverged count, NOXDV/NOLOG, per the memory notes) — the dashboard misses native-font crashes, so compare corpus counts to baseline, not just the divergence list. **Any restore divergence silently corrupts output.**

### 7.11 Open questions (need measurement/decision)

1. **Real dirty-page/dirty-word count per edit** on a representative document — determines whether word-log (fine) or chunk-COW (coarse) wins, and the page size (§4.3).
2. **Actual wall-time of the fixed startup and of a full clone** — the ~100–300 ms cold and the ~20 ms warm-memcpy figures are reasoned, not benched. Bench the memset, first-touch fault-in, undump, and the dense-arena memcpy separately to size the §2.1 payoff and confirm the per-keystroke frame-budget pressure that motivates COW (#20).
3. **Unchecked-accessor payoff (§2.6, roadmap #5)** — does LLVM already hoist the bounds check out of the hot loops? Disassemble and micro-benchmark before investing; the expected gain is low-single-digit %.
4. **`match (mode,cmd)` dispatch** — does the compiler already lower it to a jump table? Disassemble before touching (§2.6).
5. **Warm-session memory footprint** — with headroom allocated once and dense arenas shared/COW'd, how many concurrent documents/sessions can be kept warm (§2.5)? Drives whether font_info/side-table right-sizing matters.
6. **Where exactly does residual C code write/hold `mem[]` or cross-call state** (§7.1, §7.7)? Grep and quantify xetex-xetex0.c to decide the pure-Rust-write-path timeline vs. the page-fault fallback, and to validate the "whole-struct clone captures everything" premise.
7. **Checkpoint ladder cadence** — per-section vs per-page vs per-paragraph: memory cost (snapshots ∝ boundary count) vs replay-tail length tradeoff (§4.4 step 2, Strokov).
8. **Background-convergence cancellation semantics** — how to cheaply cancel/restart a cloned-state full compile when a new edit lands mid-flight (§4b), and how stale the fast-path overlay may be before it's misleading.

---

### Closing

The codebase is unusually well-positioned. The single owned `Box<EngineState>`, the `Copy` POD arenas, the all-i32-index object graph, the accessor choke point for every arena write, the already-tracked live-extent bookkeeping, the existing SyncTeX node→(tag,line) map, and the format dump as a proof-of-concept checkpoint together mean the hard part of a Typst-like live preview — cheap, correct whole-state snapshot/restore — is a tractable *engineering* problem with a clear path. Ship the Tier-1/Tier-2 cheap wins now (shape cache, seed pinning, synctex-stamp gate, converged-aux, string-mark POC, arena reuse), land the FFI-ownership refactor and the purpose-built `snapshot()/restore()` (with file-offset capture and the C-side cross-call-state audit), then build the post-preamble checkpoint, the **checkpoint ladder** (the soundness base), and only then the per-paragraph fast path and background convergence. The correctness gates are precise and enumerated with `file:line`: the O(1) fast path is valid *only* against a bracketing checkpoint (not the post-preamble one); a full clone is a ~20 ms ~216 MB dense-arena memcpy, so *repeated* per-keystroke snapshots need the paged-COW research track (feasibility-gated on eliminating the pointer escapes), not the watermark reset; the fast-path rewind set must include the nest, the page contribution list, and the synctex counters; the owning-pointer classes must never be bit-copied; and every change must be checked bit-identical against the corpus dashboard.