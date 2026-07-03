I have verified everything needed. Here is the spec.

---

# SPEC 6 — Determinism, Cache-Keys, and the Residual-C / FFI Audit
**Target:** `crates/engine_xetex` (pure-Rust XeTeX core) + parent driver `crates/../src/driver.rs`
**Goal it serves:** a Typst-style live-preview editor built on cheap `EngineState` snapshot/restore. Snapshot/restore is only *sound* if a compile is a pure function of a knowable set of inputs, and only *useful* if we can key cache entries on those inputs. This spec (a) enumerates every hidden input that must be folded into the cache key or pinned for replay, and (b) proves the residual-C/FFI surface holds no cross-call TeX state and never writes the arena — i.e. that the Spec-4 accessor instrumentation is write-complete.

All anchors below were opened and re-verified against the current tree.

---

## PART A — DETERMINISM / CACHE-KEY

### A.0 The determinism model

A compile's XDV output is a function of these input classes. Everything that is **not** in this list must be proven to have no effect on `mem[]`/`eqtb[]`/pool/XDV:

1. Format file (`.fmt`) bytes.
2. Primary input filename + the byte contents of every file the run reads (TeX source, `\input`, `.aux`, class/style, images, mapping files).
3. Font files resolved + their bytes + the *resolver's* verdict (fontconfig / CoreText), and the shaping library versions (ICU / HarfBuzz / Graphite / TECkit).
4. The `ConfigState` tuple.
5. `build_date` (`SOURCE_DATE_EPOCH`), which feeds `\time/\day/\month/\year` **and** the RNG seed.
6. Anything read from the wall clock, the RNG, the filesystem metadata (mtime/size), or a shell-escape subprocess.

Class 6 is the enemy: those are *hidden* inputs. The rest are content we can hash. The job of Part A is to (i) kill the wall-clock leak into the RNG so a compile becomes a pure function of classes 1–5, (ii) enumerate the remaining class-6 primitives and either pin or key them, and (iii) give the exact cache-key formula.

### A.1 Enumerated hidden inputs (verified)

Grep of `gettimeofday|localtime|gmtime|libc::time|get_mtime|get_file_md5|get_data_md5|rand|getpid|SystemTime|Instant` across `src/` yields **exactly** the following clock/RNG/FS reads. There are no others.

| # | Hidden input | Site (verified) | Reaches output via | Replay-stable today? |
|---|---|---|---|---|
| 1 | `gettimeofday` → `epochseconds`,`microseconds` | `texmfmp.rs:96` in `get_seconds_and_micros`, called at `engine.rs:1200` | RNG seed (#2) + `\elapsedtime` (#3) | **NO** |
| 2 | `random_seed = microseconds*1000 + epochseconds%1e6` | `engine.rs:1399` | `init_randoms` → `\uniformdeviate`,`\normaldeviate`,`\the\randomseed` (`scan.rs:4484`) | **NO** (this is the trap) |
| 3 | `\elapsedtime` = `get_microinterval` | `scan.rs:4481` → `texmfmp.rs:72` (calls `gettimeofday` at `:96`) | `\the\elapsedtime` | **NO** |
| 4 | `\resettimer` re-reads clock into `epochseconds/microseconds` | `ext.rs:2767-2772` | resets #3's baseline | **NO** |
| 5 | `\setrandomseed` | `ext.rs:2775-2782` | RNG stream | YES (source-controlled, folded via input bytes) |
| 6 | `\time/\day/\month/\year` from `build_date` | `engine.rs:1281-1285` → `texmfmp.rs:49` `localtime` | int params | YES *iff* `build_date` in key |
| 7 | `\creationdate` (PDF date) from `build_date` | `texmfmp.rs:592` `init_start_time`→`getcreationdate_in` `:599` | pool string | YES *iff* `build_date` in key |
| 8 | `\filemoddate{f}` = file mtime | `texmfmp.rs:619` `getfilemoddate_in` → `ttstub_input_get_mtime` `:631` | pool string | **NO** (FS metadata) |
| 9 | `\filesize{f}` | `texmfmp.rs:368` `getfilesize_in` → `ttstub_input_get_size` | pool string | YES (content-derived, if content in key) |
| 10 | `\filedump` | `texmfmp.rs:406` `getfiledump_in` | pool string | YES (content-derived) |
| 11 | `\mdfivesum` (file or string) | `texmfmp.rs:289` `getmd5sum_in` → `ttbc_get_data_md5`/`ttstub_get_file_md5` | pool string | YES (content-derived) |
| 12 | `\pdfshellescape` state | `scan.rs:4487` reads `config.shell_escape_enabled` | int value | YES (config in key) |
| 13 | shell-escape subprocess | driver `sysrq_shell_escape` (`../src/driver.rs:670`), engine trigger `ttstub_shell_escape` (`shipout.rs:29`) | side-effect files re-read by later passes | **NO** (non-hermetic) |
| 14 | Font resolver verdict | `find_native_font_in` `ext.rs:1057`, `load_native_font_in` `fonts.rs:1110` | glyph metrics → box dims → XDV | **NO** (system-dependent) |

Note the classic DVI-date trap is **already closed**: the XDV preamble comment is the fixed literal `c"tectonic"` (`shipout.rs:642`), not a timestamp; SyncTeX's preamble writes only `SyncTeX Version:N` (`synctex.rs:152`) and the (input-derived) root path, no clock. So the only genuine non-determinism is rows 1–4, 8, 13, 14.

### A.2 The sharp trap and its one-expression fix (row 2)

**Current** (`engine.rs:1399`):
```rust
st.errors.random_seed = st.shipout.microseconds * 1000 + (st.shipout.epochseconds % 1_000_000);
```
`microseconds`/`epochseconds` are filled from `gettimeofday` at `engine.rs:1200-1202` and are **not** gated by any deterministic flag — so `\uniformdeviate`/`\normaldeviate`/`\the\randomseed` are non-reproducible even under `SOURCE_DATE_EPOCH`/`force_deterministic` (which only pins `build_date`). This is a live-preview correctness hole: two snapshots of the "same" document diverge.

**Fix** — derive the default seed from `build_date` (already a param of `tt_run_engine`, `engine.rs:946`), matching the original bit-shape (`micros*1000 + secs%1e6`) but from the pinned epoch:

```rust
// engine.rs:1399 — deterministic default seed derived from build_date, not the wall clock.
// build_date is seconds; synthesize the sub-second term deterministically.
let bd = build_date as i64;
st.errors.random_seed =
    (((bd.wrapping_mul(1000) + (bd % 1_000_000)) & 0x7FFF_FFFF) as i32);
```

Simplest acceptable variant (if you don't care about matching the two-term shape): `st.errors.random_seed = (build_date as i32).wrapping_mul(1000) & 0x7FFF_FFFF;`. Either is fine — the only requirement is *pure function of `build_date`*. Then the `microseconds`/`epochseconds` gettimeofday capture at `engine.rs:1200-1202` becomes dead for seeding.

**Snapshot-clone confirmation:** the entire RNG state clones with `EngineState` — `randoms:[i32;55]` (`state.rs:1430`), `j_random:u8` (`state.rs:1431`), `random_seed:i32` (`state.rs:1449`) all live in `ErrorState` (a field of `EngineState`, `state.rs:10`). `epochseconds`/`microseconds` live in `ShipoutState` (`state.rs:1315-1316`, `EngineState.shipout`). So once seeded deterministically, restore reproduces the stream exactly; no extra plumbing needed.

### A.3 `\elapsedtime` / `\resettimer` (rows 1,3,4) — a deterministic-timer knob

`\elapsedtime` (`scan.rs:4481` → `get_microinterval` `texmfmp.rs:72`) and `\resettimer` (`ext.rs:2767`) both read `gettimeofday` (`texmfmp.rs:96`). These are inherently wall-clock deltas. For live-preview reproducibility, pin them behind a config flag rather than mutate the RNG-seed fix above.

**Add** `ConfigState.deterministic_timers: bool` (`state.rs`, the `ConfigState` struct currently ends at `max_print_line: i32`). Plumb it exactly like `shell_escape_enabled` (`lib.rs:129/191`, `engine_interface.rs:31/49`). Then gate the two clock reads:

```rust
// texmfmp.rs — get_seconds_and_micros gains an EngineState/config view, or:
pub unsafe fn get_seconds_and_micros_det(cfg: &ConfigState, build_date: i32,
                                         seconds: *mut i32, micros: *mut i32) {
    if cfg.deterministic_timers {
        *seconds = build_date; *micros = 0; return;
    }
    /* existing gettimeofday path */
}
```
With `deterministic_timers` set, `\the\elapsedtime` is a stable function of how much *virtual* time the doc claims (0 if no `\resettimer` cheats), and `\resettimer` becomes a no-op re-pin to `build_date`. This is the correct default for the editor; leave it `false` for CLI parity.

### A.4 Date/time from `build_date` (rows 6,7) — already replay-stable

`get_date_and_time` (`texmfmp.rs:49`, `localtime`) and `init_start_time`/`getcreationdate` (`texmfmp.rs:592/599`, `gmtime`) consume only `build_date`. These are pure given `build_date`. **Caveat:** `localtime` (row 6) depends on the process `TZ`/local timezone → `\time/\day` differ across machines with the same `build_date`. `getcreationdate` uses `gmtime` (UTC, machine-independent) but the date/time int params use `localtime`. **Recommendation:** for the editor, either fold the effective `TZ` into the cache key, or set `TZ=UTC` for the engine process. `build_date` itself must be in the key (A.9).

### A.5 FS-derived primitives (rows 8–11)

- **`\filemoddate` (row 8)** is the only content-*independent* FS read: `ttstub_input_get_mtime` (`texmfmp.rs:631`). Two options: (i) fold the mtime of every file touched by `\filemoddate` into the key, or (ii) intercept at the bridge and return a pinned mtime (`build_date`) under `deterministic_timers`. Prefer (ii) for hermeticity.
- **`\filesize`/`\filedump`/`\mdfivesum` (rows 9–11)** are pure functions of file *content*; already covered because the driver hashes every read file (A.9). No extra work beyond ensuring these opens go through the tracked bridge I/O (they do: `ttstub_input_open` at `texmfmp.rs:373/428/624`).

### A.6 Shell-escape (row 13)

Shell-escape (`ttstub_shell_escape`, `shipout.rs:29`; driver `sysrq_shell_escape`, `../src/driver.rs:670`) runs arbitrary external programs whose output later re-enters as read files. This is fundamentally non-hermetic. For the editor:
- Default `shell_escape_enabled=false` (config already exists: `state.rs:892`, driver default disabled `../src/driver.rs:1044`). `\pdfshellescape` then reads 0 (`scan.rs:4487`) — deterministic.
- If shell-escape must be allowed, the cache key must fold the **write_digests** of every file the shell-escape produced (the driver already records `write_digest`, `../src/driver.rs:648`). Treat a shell-escape run as cache-poisoning: never restore a snapshot across a shell-escape boundary.

### A.7 Input file contents + format identity (classes 1–2)

The driver already hashes these: `FileSummary.read_digest` = SHA256 of a file's contents **at first read** (`../src/driver.rs:96`, populated in `event_input_closed` at `:665-666`), and `write_digest` at last write (`:100/:648`). These digests are what the rerun-convergence loop compares (`is_rerun_needed`, `../src/driver.rs:1350`). They are the ready-made per-file hashes for the cache key. The `.fmt` format file arrives as an input file too (`load_fmt_file` reads via `ttstub_input_open`, `format_io.rs:26`), so its bytes are hashed the same way; equivalently key on `dump_name` + format digest.

### A.8 Fonts + shaping libraries (row 14) — the biggest hidden input

`find_native_font_in` (`ext.rs:1057`) → `findFontByName`/`getFullName` (fontconfig on Linux, CoreText on macOS) resolves an abstract font name to a concrete file; `load_native_font_in` (`fonts.rs:1110`) opens it and pulls glyph metrics through HarfBuzz/Graphite/ICU/TECkit (`layoutChars`, `getGlyphWidth`, …, `ext.rs:185-202`). The resolved file, its bytes, and the shaping library versions **all** feed glyph advances → box dimensions → XDV bytes. The cache key must fold:
- SHA256 of each resolved font file (opened via the tracked bridge, so its digest is already in the driver's map for file-backed fonts; system fonts opened outside the bridge need explicit hashing).
- The resolver decision (fontconfig config generation / CoreText version) — safest is to fold the resolved absolute path + file digest, which the resolver output implies.
- Library version stamps: `ICU_VERSION_SUFFIX` (compiled in, `ext.rs:209` etc.), HarfBuzz/Graphite/TECkit versions, and **the platform** (`target_os`), because macOS uses the AAT path (see B.5) and Linux does not — identical fonts can shape differently.

### A.9 The cache-key formula

Let `H` = SHA256, `‖` = length-prefixed concatenation. The **compile cache key** is:

```
K = H(
      "xetex-cachekey-v1"
    ‖ ENGINE_BUILD_ID              // git rev / crate version of engine_xetex
    ‖ PLATFORM                     // target_os + target_arch  (AAT vs HB divergence, B.5)
    ‖ LIB_VERSIONS                 // ICU_VERSION_SUFFIX ‖ harfbuzz ‖ graphite2 ‖ teckit ‖ freetype
    ‖ build_date (8 bytes LE)      // engine.rs:946; feeds \time/\day/\month/\year + RNG seed (post A.2)
    ‖ TZ_or_"UTC"                  // A.4 caveat (localtime in get_date_and_time)
    ‖ config_tuple                 // A.9.1 below
    ‖ H(format_file_bytes)         // dump_name identity (format_io.rs:26)
    ‖ Σ_over_sorted(name)  H(name ‖ read_digest[name])     // every input file, driver read_digest
    ‖ Σ_over_sorted(font)  H(resolved_path ‖ H(font_bytes))// A.8
    ‖ [ if shell_escape: Σ H(name ‖ write_digest[name]) else 0 ]  // A.6
  )
```

**A.9.1 config_tuple** = the full `ConfigState` (`state.rs`, verified fields):
`file_line_error_style_p`, `halt_on_error_p`, `in_initex_mode`, `synctex_enabled`, `semantic_pagination_enabled`, `shell_escape_enabled`, `max_print_line`, **+ new** `deterministic_timers`. Rationale: `semantic_pagination_enabled` flips the output extension and sets `XETEX_GENERATE_ACTUAL_TEXT` (`engine.rs:1409-1413`) → changes XDV; `synctex_enabled` changes SyncTeX emission; `shell_escape_enabled` changes `\pdfshellescape` (`scan.rs:4487`) and permits row-13 side effects; the error-style/halt/max_print_line flags change log/error routing (not XDV, but keep them for a total-output key). Note `interaction` (`ErrorState.interaction`, `state.rs:1436`) also matters (Batch vs errorstop changes the selector at `engine.rs:1403`) — fold it if the host can set it.

**Composition with the driver's per-file SHA256:** the driver's `read_digest`/`write_digest` (`../src/driver.rs:96/100`) are exactly the per-file hashes `K` consumes; `read_digest`s go **into** `K` (inputs), `write_digest`s are the **output** the cache stores against `K` (except the shell-escape feedback case A.6, where a produced file's `write_digest` becomes a subsequent input). The existing convergence machinery (`is_rerun_needed`, `../src/driver.rs:1350`, comparing `read_digest` vs `write_digest` for `ReadThenWritten` files) is orthogonal: it detects multi-pass fixpoints; `K` identifies a single pass. For the editor, key each *pass* on `K` including the prior pass's produced `.aux` digest.

### A.10 Edit list (Part A)

| File:line | Change |
|---|---|
| `engine.rs:1399` | Replace gettimeofday-derived seed with the `build_date`-derived expression (A.2). |
| `state.rs` `ConfigState` (after `max_print_line`) | Add `pub deterministic_timers: bool`. |
| `lib.rs:129/191`, `engine_interface.rs:31/49`, `lib.rs:241` block | Plumb `deterministic_timers` through the config setter (mirror `shell_escape_enabled`). |
| `texmfmp.rs:90` `get_seconds_and_micros` + call sites `engine.rs:1200`, `ext.rs:2770`, `texmfmp.rs:75` | Route through a deterministic variant returning `(build_date,0)` when the flag is set (A.3). |
| `texmfmp.rs:631` `getfilemoddate_in` | Under `deterministic_timers`, substitute `build_date` for the mtime (A.5). |
| Driver / host | Build `K` (A.9); set `TZ=UTC` or fold `TZ`; hash system-font files not opened via the bridge. |

Snapshot/restore itself needs **no** determinism edits: all RNG + timer state is inside `EngineState` (A.2/A.3) and travels with the snapshot.

### A.11 Validation (Part A)

Corpus bar = byte-identical XDV vs baseline. Because the seed change is guarded, staged validation:
1. **Neutral proof:** for every corpus doc that does **not** reference `\uniformdeviate|\normaldeviate|\the\randomseed|\the\elapsedtime|\filemoddate|\resettimer`, `K`-relevant behavior is unchanged and XDV must stay byte-identical. The seed is computed but never *consumed*, so `init_randoms` (`engine.rs:1401`) runs but no `randoms[]` value is read → **zero XDV delta expected**. This should be the entire dashboard minus a tiny set. Run the corpus; expect no new divergences and no drop in passed/diverged count (per the doc-count regression note).
2. **RNG docs:** author targeted fixtures using `\uniformdeviate`/`\the\randomseed`. Pre-fix these are non-reproducible (run twice, XDV differs). Post-fix, run twice → **identical**. That is the fix's success criterion; there is no stable baseline to match (the old baseline was itself noise).
3. **Timer/mtime docs:** with `deterministic_timers=true`, fixtures using `\the\elapsedtime`/`\filemoddate` produce identical XDV across runs and across `touch`-ing the input file.
4. **Snapshot equivalence:** run doc to completion → XDV_A; snapshot at midpoint, restore, finish → XDV_B; assert `XDV_A == XDV_B` on a random-using doc (exercises RNG-state clone).
5. **NOXDV/NOLOG guard:** confirm no corpus doc flips into a native-font crash (dashboard blind spot per memory).

**Failure modes:** (a) if any corpus doc reads `\the\randomseed` into typeset output, its baseline XDV changes — expected, re-baseline. (b) If the seed expression overflows `i32` differently than intended, `\uniformdeviate` distribution shifts — the `& 0x7FFF_FFFF` mask keeps it non-negative like the original `abs`-style seed. (c) `localtime` TZ dependence (A.4) can cause cross-machine `\the\day` divergence even with the fix — mitigated only by `TZ` pinning.

---

## PART B — RESIDUAL-C / CROSS-CALL-STATE AUDIT

**Claim under test:** no live C/C++ writes `mem[]`/`eqtb[]`/`font_info[]` and none holds cross-call TeX state; therefore the Spec-4 accessor set (`MemState`/`EqtbState` setters) is the complete write-surface.

### B.1 C/C++ translation-unit inventory (verified `wc -l`)

`build.rs` compiles `C_FILES` (`build.rs:213`) + `CXX_FILES` (`build.rs:264`) + `xetex-macos.c` **only** `if is_mac_os` (`build.rs:135-137`). Line counts:

- **Gutted shells (algorithm bodies deleted; declarations/comments only):** `xetex-xetex0.c` 10, `xetex-synctex.c` 10, `xetex-pagebuilder.c` 11, `xetex-errors.c` 12, `xetex-output.c` 13, `xetex-stringpool.c` 13, `xetex-scaledmath.c` 20, `xetex-linebreak.c` 23, `xetex-texmfmp.c` 45, `xetex-pic.c` 48, `xetex-shipout.c` 88, `xetex-math.c` 96, `xetex-ext.c` 97, `xetex-io.c` 101, `xetex-engine-interface.c` 85, `xetex-ini.c` 199.
- **Real native code:** `teckit-NormalizationData.c` 2635 (static Unicode tables), `teckit-Engine.cpp` 2207 (TECkit converter engine), `xetex-macos.c` 664 (**macOS-only** AAT/CoreText), `xetex-XeTeXOTMath.cpp` 526 (OpenType MATH-table reads).

### B.2 Does any C code touch `mem/eqtb/font_info`?

Grep `mem|eqtb|font_info|cur_list|save_stack|avail|rover|hi_mem_min|lo_mem_max` across all `*.c/*.cpp`:
- `xetex-ini.c:20` `memory_word *eqtb;`, `:45` `list_state_record cur_list;` are **bare symbol definitions** kept to satisfy the extern in `xetex-xetexd.h:460`. Grep for any *use* (`grep -w eqtb`/`grep -w cur_list`) returns **only** these definitions and dead comments (`:40-41`, `:46-47`, `:75`, `:117`) documenting that `mem`/`lo_mem_max`/`hi_mem_min`/`var_used`/`dyn_used`/`avail`/`mem_end`/`rover`/`font_info`/`hash`/`prim`/`xeq_level` all moved to `EngineState`. **No C statement reads or writes them.** They are dead storage the linker keeps; nothing dereferences `eqtb`/`cur_list`.
- The real native TUs (`teckit-*`, `xetex-macos.c`, `xetex-XeTeXOTMath.cpp`, `xetex-ext.c`'s lone `cgColorToRGBA32`) contain **zero** references to these symbols.

**Conclusion:** no live C path reads or writes the TeX arena. Confirmed.

### B.3 `static` locals in native code (cross-call state check)

Grep of `static ` (excluding `static const`/`static_assert`/`static inline`):
- `xetex-macos.c:352` `static char buffer[256]` inside `GetGlyphNameFromCTFont` — a **scratch return buffer**, overwritten at entry (`buffer[0]=0`), returned, immediately copied by the Rust caller (`ext.rs:2359` `getGlyphName` → copies then `freeGlyphName`). Deterministic given (font,gid); holds no state across the *logical* operation. macOS-only. (Thread-safety caveat, not a determinism/snapshot caveat — irrelevant to a single-threaded engine run.)
- `teckit-Engine.cpp:630` `static int _depth` is under `#ifdef TRACING` — **not compiled** in normal builds.
- `teckit-Engine.cpp:861/926/948/1449/1459/1471/1709`, `xetex-XeTeXOTMath.cpp:292/305/318`, `xetex-macos.c:214/246/300/329/446`, `xetex-ext.c:80` are **`static` functions** (internal linkage), not variables.
- `teckit-NormalizationData.c` is entirely `static const` Unicode tables (rodata).

**No mutable C static holds cross-call TeX state.** The only mutable static (`buffer[256]`) is per-call scratch on the macOS path.

### B.4 Every `extern "C"` callee Rust still invokes, classified

Grepping `extern "C"` + `link_name` across `src/*.rs` yields these callee groups. Classification: **[bridge]** = Tectonic `tectonic_bridge_core` I/O (stateful in the *bridge*, not the engine; the source of truth for input bytes/digests), **[libc/alloc]** = libc or xmalloc, **[stateless-fn]** = pure given args, **[font-handle]** = operates on an opaque font/engine/converter handle passed in by Rust (state lives in the handle, owned by `FontState` Vecs and freed at `engine.rs:1456/1461`), **[static-scratch]** = the macOS return buffer, **[none]** = empty/dead block.

| Module | Callees (representative) | Class |
|---|---|---|
| `texmfmp.rs:19` | `ttstub_get_file_md5`,`ttbc_get_data_md5`,`ttstub_input_open/get_size/get_mtime/read` | [bridge] + [libc] `xmalloc`,`_tt_abort` |
| `io.rs:106/158` | `_ttbc_get_core_state`,`ttbc_input_seek/close`,`ttstub_input_*`,`ttstub_output_*`,`xetex_ucnv_*`,`TECkit_CreateConverter/ConvertBuffer/ResetConverter` | [bridge] + [font-handle] (ucnv/TECkit converter handles) |
| `ext.rs:61` | ~120 fns: `createLayoutEngine`,`layoutChars`,`getGlyph*`,`mapCharToGlyph`,`createFont*`,`findFontByName`,`getFullName`,`count*/getInd*`,`ubidi_*`,`ubrk_*`,`initGraphiteBreaking`,`TECkit_*`,`xetex_ucnv_*`,`CFRelease`,`release_font_engine` | [font-handle] + [stateless-fn] (`D2Fix`,`Fix2D`,`hb_tag_from_string`) + [bridge] `ttstub_input_*` |
| `math.rs:130` | `isOpenTypeMathFont`,`usingOpenType`,`get_native_mathsy/ex_param`,`get_ot_math_*`,`get_ot_assembly_ptr`,`ot_part_*`,`free_ot_assembly` (→ `xetex-XeTeXOTMath.cpp`) | [font-handle] (all take font id / engine ptr / assembly ptr) |
| `fonts.rs:24` | `get_loaded_font_design_size`,`isOpenTypeMathFont`,`get_ot_math_constant`,`ttstub_input_getc` | [font-handle] + [bridge] |
| `scan.rs:45`,`linebreak.rs:90`,`main_control.rs:29`,`expand.rs:77` | `usingGraphite`,`usingOpenType`,`get_cp_code`,`set_cp_code` | [font-handle] (font-num keyed cp-code cache in the layout engine) |
| `pic.rs:73` | `pdf_open/close`,`pdf_doc_get_page*`,`check_for_jpeg/bmp/png`,`*_get_bbox`,`min4/max4` | [stateless-fn] + [bridge] `ttstub_input_open` |
| `shipout.rs:20`,`synctex.rs:21`,`output.rs:63`,`format_io.rs:17`,`save.rs:13`,`errors.rs:29` | `ttstub_output_*`,`ttstub_shell_escape`,`ttstub_diag_*`,`ttbc_diag_*`,`ttstub_issue_warning/error`,`_ttbc_set_error_message` | [bridge] |
| `engine.rs:42/62/85` | `xmalloc`,`ttstub_output_open_stdout`,`pdf_files_init`,`pdf_files_close`,`destroy_font_manager`,`TECkit_DisposeConverter`, statics `TEX_format_default`/`format_default_length`/`pool_free`/`strings_free`/`bad`/`output_file_extension` | [libc/bridge] + module-static locators (rodata/config), **not TeX arena** |
| `mem.rs:28`,`ext.rs:356`,`scaled_math.rs:12`,`pagebuilder.rs:60` | `xmalloc`; `D2Fix`/`Fix2D`; empty | [libc]/[stateless]/[none] |

**Every callee is one of:** bridge I/O (state in the bridge, which is the digestable input/output surface), libc/alloc, a stateless math/geometry helper, or a font/converter-handle operation whose state is owned by Rust-held handles (`FontState.font_layout_engine`/`font_mapping`, `state.rs:923/927`) that are freed on teardown (`engine.rs:1456/1461`) and reload (`fonts.rs:1079`). **None reads or writes `mem/eqtb/font_info`. None retains hidden global TeX state across calls.**

### B.5 The macOS AAT reach-back (verified)

The one exception the memory note flags: `font_feature_warning` (`output.rs:2861`, `#[no_mangle] extern "C"`) reaches the run's `EngineState` via the thread-local locator `current_engine_state()` (`output.rs:2869`; locator defined `state.rs:1613`, installed `engine.rs:963`, cleared by `PtrGuard::drop` `engine.rs:960`). Verified callers: `xetex-macos.c:554` and `:606` — **only** the macOS AAT path (`loadAATfont`). Grep confirms no other TU calls `font_feature_warning`/`font_mapping_warning`. And `xetex-macos.c` is compiled **only** under `is_mac_os` (`build.rs:135-137`, `XETEX_MAC`). On the OpenType/HarfBuzz path (all platforms) feature warnings are produced in Rust (`find_native_font_in`, `ext.rs`), not via this callback.

So the reach-back is: **(a) macOS-only, (b) via a per-run re-installed locator (not persisted state), (c) it only *reads* the state to print a diagnostic and calls Rust `MemState`/print accessors — it does not introduce a non-instrumented arena write.** Under snapshot/restore the locator is re-established each run entry, so it is snapshot-safe.

### B.6 Verdict: is the Spec-4 accessor set write-complete?

**Yes, on Linux/non-macOS with the OpenType path — unconditionally.** Every `mem[]`/`eqtb[]`/`font_info[]` mutation is Rust and funnels through the `MemState`/`EqtbState` setters enumerated in Spec 4 (`set_b32s1` `state.rs:539`, … `set_word` `state.rs:610`; eqtb `state.rs:703-721`) plus the node allocators (`get_node` `mem.rs:133`, `get_avail` `mem.rs:66`, `free_node`, `new_*`). The C shells define but never touch the arena symbols (B.2); no C static holds TeX state (B.3); every FFI callee is stateless / bridge / font-handle (B.4). The instrumentation choke point is therefore complete: to observe/snapshot the arena it is sufficient to hook those setters + allocators.

**Caveats / platform config under which this holds:**
1. **macOS AAT (`cfg(target_os="macos")` + AAT fonts):** the `font_feature_warning` callback (B.5) re-enters Rust through the locator and drives `MemState`/print accessors. It performs no *un-instrumented* write, but it is an FFI→Rust re-entry: any snapshot taken *during* a native `loadAATfont`/`layoutChars` call would be mid-callback. **Constraint:** only snapshot at engine-quiescent points (between `main_control` tokens), never inside a shaping/AAT call. The locator (`ENGINE_STATE_PTR`, `state.rs:1613`) must be re-installed on restore (it already is, `engine.rs:963`). Because macOS uses AAT/CoreText, its shaping output differs from Linux HarfBuzz → **platform must be in the cache key** (A.9).
2. **`static char buffer[256]` (`xetex-macos.c:352`):** per-call scratch, thread-unsafe. Irrelevant to single-threaded snapshot/restore; relevant only if you ever parallelize macOS shaping.
3. **The 3 `FONT_*` FFI mirror statics** (`globals.rs ~308-334`, per memory) point *into* `FontState` Vecs; they are locators, re-established each run — but if a snapshot/restore *moves* those Vecs, the mirrors must be refreshed (Spec 4/5 concern, flagged here for completeness).
4. **Bridge I/O is stateful but external:** input bytes/digests and output files live in the driver's `mem`/`events` maps (`../src/driver.rs`), which is exactly the surface the cache key hashes (A.9). Snapshot/restore of the *engine* does not snapshot the bridge; a restored run must be re-attached to a bridge whose input-file map matches the cache key.

**Bottom line:** the "no live C writer of the TeX arena" premise is confirmed; the Spec-4 accessor instrumentation is the complete write-surface. The only reach-back into Rust state is the macOS-only, locator-mediated `font_feature_warning` diagnostic, which writes nothing outside the instrumented accessors. Snapshotting is therefore sound provided snapshots are taken at engine-quiescent points and the platform (AAT vs HarfBuzz) + font + library versions are folded into the cache key.

---

## PART C — PROOF OBLIGATIONS & RESIDUAL FAILURE MODES

1. **Purity after A.2/A.3:** must prove no remaining `gettimeofday`/`localtime` value reaches XDV except through `build_date`+`TZ`. Grep guard (CI): fail if `gettimeofday`/`Instant::now`/`SystemTime::now` appears in `src/` outside the `deterministic_timers=false` branch of `get_seconds_and_micros`.
2. **Seed-consumption invariant:** `init_randoms` runs unconditionally (`engine.rs:1401`) but must have **no** XDV effect unless `randoms[]` is read (`scaled_math.rs:509/585/593`) — validated by the A.11.1 neutral corpus pass showing zero divergence.
3. **Snapshot quiescence:** obligation that no snapshot is captured while a native shaping/AAT/TECkit call is on the stack (B.6 caveat 1). Enforce by only snapshotting between `main_control` iterations.
4. **Locator re-install on restore:** `set_engine_state_ptr` (`engine.rs:963`) + `FONT_*` mirror refresh must run after every restore, else the macOS callback / FFI font ops dereference a stale pointer.
5. **Cache-key completeness:** obligation that any *new* clock/FS/random read added to the engine is added to Table A.1 and to `K`. Enforce with the same CI grep as (1) plus a `link_name`/`extern "C"` diff check.
6. **Failure modes that survive:** (a) `localtime` TZ dependence (A.4) — cross-machine `\the\day` drift unless `TZ` pinned/keyed; (b) shell-escape non-hermeticity (A.6) — never cache/restore across it; (c) font-resolver drift (A.8) — a fontconfig cache change on the host silently changes shaping unless the resolved path+bytes are keyed; (d) macOS AAT vs Linux HB divergence (B.6) — same source, different XDV, correctly separated only if platform is in `K`.