# Incremental / live-preview XeTeX — implementation deep-dives

Implementation-grade specs turning the high-level review
(`../../SPEED-INCREMENTAL-REVIEW.md`) into actionable engineering. Each spec
re-verified its own `file:line` anchors against the tree; where a claim was
load-bearing it was independently re-checked (see the ledger below).

## The documents

| # | Doc | What it specifies |
|---|-----|-------------------|
| 00 | [Integration & roadmap](00-integration-roadmap.md) | **Read first.** Cross-spec consistency (14 seams), correctness audit of the riskiest claims, one dependency-ordered roadmap (W0–W16) with the critical path. |
| 01 | [Caching subsystem](01-caching-subsystem.md) | Shape cache on `measure_native_node` + font-engine/TFM cross-run cache. Key design, glyph-blob layout, exact edit sites. Highest-leverage cheap win; no snapshot machinery. |
| 02 | [snapshot()/restore() API](02-snapshot-restore-api.md) | Field-by-field disposition of every `EngineState` field; Arc-shared font engines; **glyph-blobs-out-of-the-union** migration; file-offset journaling; mirror/locator refresh. The enabler everything else depends on. |
| 03 | [Checkpoint-replay engine](03-checkpoint-replay-engine.md) | `ResidentEngine` lifecycle split; checkpoint ladder; forward-replay driver; the exact rewind set; Tier-0/1/2 fast paths; locality detection + fallback ladder. |
| 04 | [Word-log / COW substrate](04-wordlog-cow-substrate.md) | Per-keystroke O(edit-burst) snapshot via `set_*` instrumentation + touched-bitmap; chunk-COW alternative; dense-arena paged-COW feasibility; benchmark plan. |
| 05 | [Live-preview architecture](05-live-preview-architecture.md) | Resident-engine + convergence topology; **display-list extraction bypassing XDV**; in-memory SyncTeX index; WASM story; bit-identical validation. |
| 06 | [Determinism & C audit](06-determinism-and-c-audit.md) | Exhaustive hidden-input enumeration + fix sites; the definitive residual-C/FFI audit. |

## Verified-facts ledger (checked against the current tree)

- **The TeX core is pure Rust.** `build.rs` still compiles the xetex C files but
  they are gutted shells (`xetex-xetex0.c` = 10 lines; `get_node`/`main_control`
  no longer defined in C). The only real native code is platform font shims:
  `xetex-macos.c` (664L, `#[cfg(macos)]` AAT), `teckit-*`, `xetex-XeTeXOTMath.cpp`
  (526L). **None read or write `mem`/`eqtb`/`font_info`.** → no live C writer of
  the arena. (Spec 6 Part B.)
- **CORRECTION to the first review's reframing:** "all `mem[]` writes funnel
  through the `MemState` `set_*` accessors" is **true for C, false for Rust**.
  `measure_native_node` writes node slots via raw `*mut MemoryWord` arithmetic
  (`(*node.add(1)).b32.s1 = D2Fix(width)`, `ext.rs:1584/1629`), and there are
  ~43 further raw-escape writers via `as_u16_ptr_mut`/`node_addr` (`state.rs:584/598`).
  The word-log (Doc 04) is therefore write-complete **only** for epochs that
  perform no raw-escape write, unless those writers are rerouted through
  instrumented setters. This is roadmap item **W10 / Audit §2.1** — do not treat
  Doc 04 as write-complete until it is closed.
- **Glyph blob**: `NATIVE_GLYPH_INFO_SIZE = 10` (`consts.rs:152`); layout =
  `FixedPoint[glyph_count]` (8B each) then `u16 glyph_ids[glyph_count]`;
  position-independent (no pointers); `copy_native_glyph_info` at `mem.rs:535`;
  freed at `packaging.rs:562` and `fonts.rs:1148`.
- **`random_seed`** is gettimeofday-derived at `engine.rs:1399` and NOT covered
  by `deterministic_mode` — the sharp determinism trap; fix = derive from
  `build_date` (`engine.rs:989`). (Spec 6 §A.2.)
- **FFI free sites**: `release_font_engine` (`engine.rs:1456`),
  `TECkit_DisposeConverter` (`engine.rs:1461`), `name_of_input_file`
  (`engine.rs:1484`).

## Critical path (from the roadmap)

**W1** (finalize C/FFI audit + CI write-completeness guards) → **W2** (unified
`Arc<LayoutEngineHandle>`) → **W3** (glyph-blobs → `GlyphBlobArena`, makes
`mem.data` genuinely POD) → **W4** (canonical bounded-copy `Snapshot` + in-place
`restore`) → **W6** (`ResidentEngine` lifecycle split) → **W7** (checkpoint
ladder + replay) → **W8** (Tier-0 fast path + unified escape guard). W9 (shape
cache) and W10 (word-log) parallelize off W3. W0 (`random_seed` fix) is a cheap
early prerequisite.

W2 and W3 are the two riskiest corpus-visible refactors (FFI double-free
surface) — land each in isolation as a byte-identical no-op, corpus-gated,
before W4.

## Status

These are **design specs**, not landed code. Every code sketch must be
implemented, corpus-validated (byte-identical XDV; the dashboard misses
native-font crashes so compare passed+diverged counts to baseline), and the
open proof-obligations in Doc 00 §2 discharged before relying on them.
