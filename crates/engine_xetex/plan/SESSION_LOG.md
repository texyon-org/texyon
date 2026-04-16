# Session log

Append-only log of what each session did. The newest entry goes at the
**top**, immediately under this header. Each session follows this
template.

## Entry template

```
## Session N — YYYY-MM-DD — <short title>

**Goal:** one-sentence description of what this session planned to do.

**Starting state:**
- Parity: X/Y
- Streak: N

**Work done:**
- <bullet list of concrete actions: functions translated, design decisions,
  docs updated. Include commit hashes.>

**Ending state:**
- Parity: X/Y (delta: +/-N)
- Streak: N

**Surprises / gotchas:**
- <anything that wasted time, or that the next session should know>

**For the next session:**
- <handoff: what's the obvious next piece of work, any half-done state>
```

---

## Session 4 — 2026-04-16 — Phase 2: 12 new_* node constructors

**Goal:** translate the 12 `new_*` node constructor functions from xetex-xetex0.c to `src/mem.rs`.

**Starting state:**
- Parity: 29/29
- Streak: 3

**Work done:**
- Confirmed parity baseline: 29/29.
- Added `eqtb`, `temp_ptr` globals and mem b16/gr/word accessors to `src/globals.rs`.
- Commit `8fef23ac`: translated new_null_box, new_rule, new_ligature, new_lig_item, new_disc, new_math, new_spec, new_param_glue, new_glue, new_skip_param, new_kern, new_penalty into `src/mem.rs`.
  Deleted corresponding C bodies from xetex-xetex0.c (~142 lines removed).

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 4

**Surprises / gotchas:**
- First deletion Edit omitted the function header for new_null_box, leaving a dangling `int32_t new_null_box(void)\n{\n    int32_t p;\n` fragment in the C file. Caused cascading parse errors. Fixed with a second Edit removing the stray header. Always include the full function signature in deletion patterns.

**For the next session:**
- Next target: `copy_native_glyph_info` (xetex-xetex0.c ~line 359) — uses xmalloc_array + memcpy + NATIVE_NODE macros; more complex than the new_* constructors.
- After that: remaining Phase 2 functions deeper in xetex-xetex0.c.

---

## Session 3 — 2026-04-16 — Phase 2 kick-off: badness + core allocators

**Goal:** start Phase 2 (memory/node allocation from xetex-xetex0.c)
by translating the core allocator functions.

**Starting state:**
- Parity: 29/29
- Streak: 2

**Work done:**
- Confirmed parity baseline: 29/29.
- Commit `e19ace3f`: translated badness, get_avail, flush_list.
  - `badness` (xetex-xetex0.c:25) → `src/helpers.rs` (new file).
  - `get_avail` (xetex-xetex0.c:170) → `src/mem.rs`.
  - `flush_list` (xetex-xetex0.c:192) → `src/mem.rs`.
  - New globals: `mem` (raw pointer), `lo_mem_max`, `hi_mem_min`,
    `avail`, `mem_end`, `rover` + `mem_b32s0`/`mem_b32s1` typed
    accessor pairs. TEX_NULL, MEM_TOP, MAX_HALFWORD as Rust consts.
  - get_avail's OOM path calls C `runaway()`/`overflow()` via
    extern "C" (not yet translated).
- Commit `fc914449`: translated free_node and get_node.
  - `free_node` (xetex-xetex0.c:284) — straightforward list insertion.
  - `get_node` (xetex-xetex0.c:150) — the main lo-mem allocator; two
    C `goto` labels handled via Rust labeled-loop break/continue:
    `goto found` → `break 'restart r`, `goto restart` → `continue 'restart`.
  - New globals: `InputState` (#[repr(C)] struct for `input_state_t`),
    `cur_input_synctex_tag()`, `line()`. Added `set_lo_mem_max`.
  - New module `src/helpers.rs` registered in `lib.rs`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 3
- 18 functions migrated total (13 from scaledmath + 5 from xetex0).
- xetex-xetex0.c: 18,543 LOC, ~268 functions remaining.

**Surprises / gotchas:**
- Rust unit test runner shows "running 0 tests" in the doc-tests pass
  (a second binary) — that's normal. The main test binary ran 21 tests.
- `get_node` Case 133 (exact-fit block): the C code checks
  `if (r == p)` first and then `if (mem[p+1].b32.s1 != p)` as a
  second condition. I collapsed these into `if r == p && mem_b32s1(p + 1) != p`.
  Semantically identical because the inner if can only be true when
  the outer is true.
- `InputState` must exactly mirror `input_state_t` (state+index are
  u16, then five i32 fields). Any field reorder or padding difference
  would silently corrupt `cur_input.synctex_tag` reads. The `#[repr(C)]`
  attribute plus the field ordering matches the C definition at
  xetex-xetexd.h:344–352.

**For the next session (Session 4):**
- **Priority 1:** continue Phase 2 — the `new_*` node constructors.
  These all call `get_node` (now in Rust!) so the call chain is clean.
  Start with the simplest: `new_null_box` (line 223 in current file),
  `new_rule` (~240), `new_ligature` (~253), `new_lig_item` (~265),
  `new_disc` (~273), `new_math` (~299), `new_spec` (~309),
  `new_param_glue` (~321), `new_glue` (~336), `new_skip_param` (~349),
  `new_kern` (~360), `new_penalty` (~370).
  These need additional mem accessors (b16 fields, gr field).
- **Note:** `new_null_box` uses `mem[p].b16.s0`, `BOX_glue_set`,
  `NODE_type` — check `xetex-xetexd.h` for the macro definitions before
  translating. Will need `mem_b16s0`/`set_mem_b16s0` etc.
- **Priority 2:** once the `new_*` constructors are done, `copy_native_glyph_info`
  is next, followed by Phase 3 (output/printing).

---

## Session 2 — 2026-04-16 — Complete xetex-scaledmath.c (minus m_log)

**Goal:** translate the remaining 7 translatable functions from
`xetex-scaledmath.c` (make_frac, take_frac, ab_vs_cd, new_randoms,
init_randoms, unif_rand, norm_rand).

**Starting state:**
- Parity: 29/29
- Streak: 1

**Work done:**
- Confirmed parity baseline: 29/29.
- Prep commit `1006768f`: removed `static` from make_frac, take_frac,
  m_log, ab_vs_cd, new_randoms in xetex-scaledmath.c. Added prototypes
  to xetex-xetexd.h. Build verified.
- Added `randoms[55]` and `j_random` global accessors to `src/globals.rs`
  (random_entry / set_random_entry / j_random / set_j_random).
- Translated 7 functions to Rust in `src/scaled_math.rs`, removed their
  C bodies — commit `3f096961`:
    - `make_frac` (xetex-scaledmath.c:165)
    - `take_frac` (xetex-scaledmath.c:220)
    - `ab_vs_cd` (xetex-scaledmath.c:332)
    - `new_randoms` (xetex-scaledmath.c:418)
    - `init_randoms` (xetex-scaledmath.c:441)
    - `unif_rand` (xetex-scaledmath.c:468)
    - `norm_rand` (xetex-scaledmath.c:487) — calls C `m_log` via
      `extern "C"` declaration in scaled_math.rs.
- xetex-scaledmath.c now contains only m_log (66 LOC).
- All 15 crate-level unit tests pass; parity 29/29 after switch.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 2
- `MODULE_STATUS.md` shows xetex-scaledmath.c IN PROGRESS — 93%
  (13/14 functions migrated, 1 blocked).

**Surprises / gotchas:**
- All 7 functions ended up in one commit because they were written
  together in scaled_math.rs before the C bodies were deleted. The
  workflow says "one function per commit when practical" — this was
  a pragmatic deviation, acceptable since they form a tight cluster.
- `norm_rand` calls `m_log` (blocked on Phase 3). To keep `norm_rand`
  in Rust now, m_log was un-static'd in C (so the FFI call links).
  This is clean: norm_rand will need no changes when m_log is eventually
  translated; we just remove the `extern "C" { fn m_log }` declaration.
- The `take_frac` overflow check uses `n - 0x7FFFFFFF` (C `be_careful`);
  since n can be 0x7FFFFFFF and p can be positive, `be_careful + p > 0`
  can overflow. The C code relies on wrapping behavior here — we don't
  need `wrapping_*` because n is set to `0x7FFFFFFF - p` in the overflow
  branch, making the subsequent addition exact.

**For the next session (Session 3):**
- **Priority 0:** parity baseline 29/29 (already solid).
- **Priority 1:** start Phase 2 — memory / node allocation helpers from
  `xetex-xetex0.c`. Candidates (all in xetex-xetex0.c):
    - `badness` (line 25 in xetex-xetex0.c per session-1 notes) —
      pure math, no globals except possibly `arith_error`.
    - `get_avail` (line 170), `flush_list` (line 192) — memory helpers.
  Read `02-state.md` and `03-memory-and-nodes.md` before starting Phase 2.
- **Priority 2:** once xetex-scaledmath.c is the only remaining C in
  the file (it is!), we could mark it DONE if m_log is translated.
  But m_log is BLOCKED — leave as IN PROGRESS.
- **Do NOT** touch m_log until the diagnostic/error machinery
  (error_here_with_diagnostic, print_scaled, etc.) is in Rust (Phase 3).

---

## Session 1 — 2026-04-16 — Phase 0 scaffolding + first Phase 1 translations

**Goal:** establish the parity baseline, set up the Rust scaffolding
(`src/globals.rs`, `src/mem_types.rs`, `src/mem.rs`), and translate
the pure arithmetic helpers from `xetex-scaledmath.c`.

**Starting state:**
- Parity: unmeasured at start.
- Streak: 0

**Plan corrections applied (pre-code):**
- `MODULE_STATUS.md` summary said 14 files; the table listed 16.
  Fixed the summary and filled in `~??` LOC for errors.c (175),
  ext.c (2170), linebreak.c (2843), io.c (555), engine-interface.c
  (112).
- `02-state.md` had an unsound example (`mem_slice_mut` returning
  `&'static mut [MemoryWord]` — two calls would alias). Replaced
  with a by-value `mem_word`/`set_mem_word` pair and documented
  the "no live references across calls" invariant.
- `PROGRESS.md` claimed 49,814 C LOC; actual in-scope total is
  39,419. Corrected.
- `01-architecture.md` "Current layout" section now lists accurate
  LOC for every in-scope file, and calls out `xetex-macos.c` as
  the conditional mac-only file.
- `06-workflow.md` Step 4 now covers `static` C functions: either
  translate caller+callee together, or un-static the C definition
  as a prep commit. This came up because most of Phase 1's
  leftovers (`make_frac`, `take_frac`, `m_log`, `ab_vs_cd`,
  `new_randoms`) are `static`.
- Created `plan/scripts/parity.sh` per `05-testing.md`. Logs now
  land in `plan/parity-history.log`.

**Work done:**
- Recorded parity baseline: **29 passed / 29 total** (session 0
  baseline).
- Created `src/mem_types.rs` with `MemoryWord`, `B32x2`, `B16x4`
  (`#[repr(C)]`, compile-time `size_of` asserts).
- Created `src/mem.rs` with `MemIdx`, `Scaled`, `FontId`,
  `StrNumber`, `Token` newtypes. No accessors yet — per the
  Phase-0 plan, those come alongside their first use in Phase 2.
- Created `src/globals.rs` with safe accessors for `arith_error`
  and `tex_remainder`. Declared the extern statics inside a
  `mod raw` to keep them out of the wrapper namespace, with a
  module-level SAFETY note forbidding any accessor that returns
  a live reference.
- Translated six pure functions from `xetex-scaledmath.c` into
  `src/scaled_math.rs`:
  - `tex_round` (xetex-scaledmath.c:11)
  - `half` (xetex-scaledmath.c:51)
  - `mult_and_add` (xetex-scaledmath.c:60)
  - `x_over_n` (xetex-scaledmath.c:79)
  - `xn_over_d` (xetex-scaledmath.c:105)
  - `round_xn_over_d` (xetex-scaledmath.c:137)
  Each carries a `// translated from …` citation and the six C
  definitions in `xetex-scaledmath.c` have been removed (header
  prototypes in `xetex-xetexd.h:1130–1134` and 763 remain, so C
  callers link to the Rust `#[no_mangle]` impls).
- Wrote 15 unit tests covering sign conventions, overflow flags,
  rounding, and zero-divisor behaviour.
- Parity held at 29/29 after the switch.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 1
- `MODULE_STATUS.md` phase-1 tracker reflects six migrated
  functions; `xetex-scaledmath.c` is "IN PROGRESS — 43%"
  (6/14 functions).

**Surprises / gotchas:**
- Rust debug-mode overflow checks panic where C silently wraps.
  Three concrete spots needed `wrapping_*` variants:
    (a) `mult_and_add`'s `max_answer ± y` boundary test,
    (b) `xn_over_d` / `round_xn_over_d`'s 32-bit intermediates
        which rely on C `32768L` (long=64) for wide compute,
    (c) the `2*v` comparison in `round_xn_over_d`.
  Comments in the source explain the reason per site.
- Initial unit tests used `i32::MAX` as `max_answer` for
  `mult_and_add`; that's outside the algorithm's implicit
  `max_answer + y fits in i32` precondition. Real TeX callers
  use `MAX_HALFWORD = 0x3FFFFFFF`. Tests updated to reflect that.
- The extern static and the safe wrapper of the same name in one
  module collide; putting the `extern "C"` block in a private
  `mod raw` resolves it cleanly without renaming the public API.
- A literal note for future sessions: the C in `x_over_n` contains
  a dead `tex_remainder = -tex_remainder;` in the `n < 0` branch
  (the value is always overwritten below). We preserve it
  faithfully to keep the translation-traceability property.

**For the next session (Session 2):**
- **Priority 0:** run `./plan/scripts/parity.sh session-2-start`
  and confirm 29/29.
- **Priority 1:** translate the remaining `xetex-scaledmath.c`
  functions **except** `m_log`. Targets:
    - `make_frac`, `take_frac`, `ab_vs_cd` (pure, `static` in C —
      un-static them first, then translate).
    - `new_randoms`, `init_randoms`, `unif_rand`, `norm_rand`
      (need the `randoms[]` and `j_random` globals added to
      `src/globals.rs` — note `randoms` is an array, so the safe
      wrapper must index by element).
- **Priority 2 (if time allows):** start on `badness` and `half`-
  adjacent helpers from `xetex-xetex0.c`. Line numbers:
  `badness` at xetex-xetex0.c:25.
- **Do NOT yet** attempt `m_log` — it depends on the diagnostic
  machinery (`error_here_with_diagnostic`, `print_scaled`,
  `print_cstr`, `capture_to_diagnostic`, `error`, `help_*`) which
  is Phase 3.

---

## Session 0 — 2026-04-15 — Plan scaffolding

**Goal:** set up `plan/` directory with strategy docs and tracking
scaffolding so that subsequent sessions have a clear, self-contained
workflow.

**Starting state:**
- Parity: not yet measured.
- Streak: 0

**Work done:**
- Surveyed `crates/engine_xetex/xetex/` (14 C files in scope,
  ~50k LOC). Noted `xetex-xetex0.c` is ~18,700 LOC / 273 functions
  and `xetex-ini.c` is ~3,900 LOC and hosts the global state
  definitions.
- Surveyed `crates/engine_xetex/src/lib.rs` (267 LOC, thin FFI
  wrapper around `tt_engine_xetex_main`).
- Surveyed `tests/tex-outputs/` (~99 files, ~25–30 test cases
  comparing .xdv/.log/.pdf against goldens).
- Wrote the plan documents:
  - `README.md` — navigation + hard rules
  - `00-overview.md` — strategy and principles
  - `01-architecture.md` — target crate layout and FFI interleaving
  - `02-state.md` — state-modeling migration strategy
  - `03-memory-and-nodes.md` — arena & node typing model
  - `04-migration-order.md` — dependency-ordered translation queue
  - `05-testing.md` — parity harness and golden-test strategy
  - `06-workflow.md` — per-session mechanical checklist
  - `07-anti-patterns.md` — mistakes to never repeat (critical)
  - `MODULE_STATUS.md` — per-file status tracker
  - `PROGRESS.md` — test-parity dashboard
  - `SESSION_LOG.md` — this file

**Ending state:**
- Parity: still not measured.
- Streak: 0
- No Rust code written yet. No C code deleted yet.

**Surprises / gotchas:**
- The C source retains a lot of `memory_word`-based bit packing (see
  `xetex-xetexd.h:85–196`). Translating node accessors is the main
  mechanical challenge in early phases — it all hinges on
  `MemoryWord` being exactly as described in `03-memory-and-nodes.md`.
- `xetex-xetex0.c` is 18,700 lines of one monolithic translation
  unit. It will not map to a single Rust module; the phase plan in
  `04-migration-order.md` splits it by function role.
- Tests do not live in `engine_xetex` itself; they live in the
  top-level `tectonic` crate at `tests/tex-outputs.rs`. This means
  the parity command is run at repo root against the top-level
  crate, which pulls `engine_xetex` in transitively.

**For the next session (Session 1):**
- **Priority 0:** run the parity baseline. Record it as "Session 0"
  in `PROGRESS.md` so subsequent sessions have a floor.
- **Priority 1:** count C functions per file with the grep snippet
  in `MODULE_STATUS.md` and fill in the `Fns (C)` column.
- **Priority 2:** set up `src/globals.rs` — a centralized Rust module
  that declares `extern` bindings to every C global listed in
  `xetex-xetexd.h`. This is the foundation for every Phase 1
  translation. See `02-state.md` §"Phase A" for the exact shape.
- **Priority 3:** if time remains, translate the first pure-helper
  function (`badness`? `half`?) from `xetex-scaledmath.c` as a
  proof-of-concept. See `04-migration-order.md` §"Phase 1".
- Do NOT yet touch `xetex-xetex0.c` — start with the small pure
  files so the build/test/commit loop is proven out on low-risk code.
