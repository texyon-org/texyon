# Integration Analysis: Six-Spec Live-Preview XeTeX Program

The connective layer across Specs 1–6. Section 1 finds the seams, Section 2 audits the load-bearing claims, Section 3 gives one dependency-ordered roadmap. I do not rewrite the specs; I reconcile them.

The single most important cross-cutting fact: **three specs independently rewrite `measure_native_node` and the `mem[p+5]` glyph slot, and four specs independently define a type named `Snapshot`.** Those collisions drive most of what follows.

---

## 1. CROSS-SPEC CONSISTENCY CHECK

### Seam 1 — Glyph-blob ownership: three incompatible models for `mem[p+5]` (Spec 1 §1/§2.3 vs Spec 2 §3.B.2 vs Spec 5 §2.4)

The same node slot is modeled three different ways:

- **Spec 1 §2.3** keeps a raw `*mut` blob in `mem[p+5].ptr`; a cache hit does `libc::free(old)` + `xmalloc` + `copy_nonoverlapping`, then `(*node.add(5)).ptr = dst`.
- **Spec 2 §3.B.2** *removes* the pointer from the union entirely: `mem[p+5].b32.s0` holds a 1-based integer handle into a `GlyphBlobArena` of `Arc<Blob>`; `measure_native_node`'s signature drops the raw `*mut c_void node` and gains `&mut GlyphBlobArena`.
- **Spec 5 §2.4** assumes `mem[p+5].ptr` still holds a `malloc` block and deep-copies each buffer (`reown_native_glyph_buffers`) on clone.

These cannot coexist. **Resolution:** Spec 2 §3.B.2 is the canonical model and must land first (it is the deepest refactor and the one that makes `mem.data` genuinely POD, which every snapshot spec depends on). Consequences that must be propagated:
- **Spec 1's shape-cache blob becomes `Arc<Blob>`, not `Box<[u8]>`.** A cache hit is then a pure `arena.dup(cached_handle)` Arc refcount bump — *cheaper* than Spec 1's own xmalloc+memcpy design, and it inherits the COW isolation Spec 2 built. Spec 1 §2.7 PA-1/PA-3 (fresh-copy isolation, free-before-overwrite) are superseded by Spec 2's `Arc::make_mut` COW.
- **Spec 5 §2.4 `reown_native_glyph_buffers` and Spec 5 E2 are deleted** — obsoleted entirely. Under the Arc arena a snapshot is a slab `.clone()` (refcount bumps); there is no raw pointer to alias, so Spec 5's "one genuinely hard case" evaporates. Spec 5 §4.2's extractor code (`native_glyph_info_ptr(&st.mem) as *const u8`) must be rewritten to `arena.get(handle)`.
- **Spec 1's `store_justified_native_glyphs` in-place mutation** (Spec 1 §2.4, ext.rs:2265) must route through `arena.get_mut()` (COW), not a raw `*mut u8`.

### Seam 2 — Font-engine handle: same Arc refactor defined three times, incompatibly (Spec 1 §3.2 B1 vs Spec 2 §3.B.1 vs Spec 5 E3)

All three convert `font_layout_engine: Vec<*mut c_void>` to an Arc-owned handle whose `Drop` disposes the engine — but they disagree on details:

| | Handle name | Drop calls | Carries `font_area`? |
|---|---|---|---|
| Spec 1 §3.2 | `EngineHandle` | `deleteLayoutEngine` (ext.rs:1271) | **no (gap)** |
| Spec 2 §3.B.1 | `LayoutEngineHandle` | `release_font_engine` (ext.rs:1263) | yes |
| Spec 5 E3 | `LayoutEngineHandle` | `release_font_engine`/`DisposeConverter` | — |

**Resolution:** Spec 2 §3.B.1 is correct and canonical — the current teardown (engine.rs:1453–1464) uses `release_font_engine`, which *needs* `font_area`; Spec 1's `EngineHandle` omitting `font_area` is a latent bug. There is exactly **one** unified `LayoutEngineHandle { raw, font_area }`. Spec 1 Part B's *cross-run process cache* is then a legitimate **consumer layered on top** of Spec 2's handle (the process cache holds one `Arc`, the `FontState` slot holds another; teardown/reload drops the slot Arc, freeing only when no cache and no snapshot references it). Spec 1 must not define a competing handle type.

**Additional dependency:** Spec 2 introduces `font_layout_engine_raw: Vec<*mut c_void>` (the flat mirror the OT-math FFI callback reads, §5/globals.rs:255). Spec 1's `finish_native_font_from_cached` and Spec 1 PB-3 must call Spec 2's `rebuild_layout_engine_mirror` after installing a cached engine — Spec 1 mentions "FONT_* mirrors" but predates the `_raw` vec.

### Seam 3 — Four `Snapshot` types with inconsistent cost models (Spec 2 §2.2, Spec 3 §B.4, Spec 4, Spec 5 §2.6)

There are four snapshot mechanisms:
- **Spec 2:** bounded copy of live `mem_lo`/`mem_hi` ranges; full sub-state disposition table; in-place `copy_from_slice` restore.
- **Spec 3:** same sparse bounded copy **plus** high-water truncation for strings/fonts/hyph; adds lifecycle fields (page-builder, DVI position, `escape_epoch`).
- **Spec 4:** per-epoch word-log undo journal (delta, not full copy).
- **Spec 5 §2.6:** `clone_from` of the **full** `mem.data` (40 MB) — yet claims restore is "~1–8 MB, sub-millisecond."

Two genuine contradictions:
1. **Spec 5's cost model is internally inconsistent and contradicts Spec 3.** `clone_from(&snap.core.mem_data)` copies the whole 40 MB arena (Spec 3 §0.3 explicitly calls a full copy "unacceptable"). **Resolution:** adopt Spec 2 §2.3 bounded copy (`[0..=lo_mem_max] ∪ [hi_mem_min..=mem_end]`) as the physical `Snapshot`; Spec 5's `clone_from` full-arena is the naïve baseline and is replaced.
2. **Three types literally named `Snapshot`.** **Resolution:** one canonical type = Spec 2's disposition table (most complete) + Spec 3's lifecycle fields (page-builder links, DVI position, `escape_epoch`) wrapped in Spec 3's `Checkpoint{src_pos,...}`. Spec 4's word-log is not a competing `Snapshot`; it is the **fine-grained delta layer between Spec 3 clone baselines** (Spec 4 §1 says exactly this) and shares Spec 2/3's `MemFrontier` scalar bundle.

### Seam 4 — Spec 3's `truncate_fonts` is written against the pre-Spec-2 raw-pointer font type (Spec 3 §C.2 vs Spec 2 §3.B.1)

Spec 3 `truncate_fonts` does `st.fonts.font_layout_engine[f] = null` and calls `release_font_engine` directly on a raw `*mut c_void`. After Spec 2 that field is `Vec<Option<Arc<LayoutEngineHandle>>>` — Spec 3's code **will not compile**. **Resolution:** Spec 3's truncate becomes "drop the slot Arcs above `font_ptr`," which auto-frees iff no snapshot holds them — strictly safer than the manual free. Spec 2's Arc-vec capture **subsumes** Spec 3's truncate: restoring Spec 2's snapshot re-installs exactly the fonts-at-capture and drops later ones via refcount. So Spec 3 §C.2's `truncate_fonts` is deleted in favor of Spec 2's Arc-vec assignment. Note Spec 3 C.2's separate obligation — invalidating the native-font *dedup* cache above `font_ptr` (Spec 3 §C.2 failure mode, Spec 5 E-in-fonts.rs) — survives and must still be done.

### Seam 5 — Spec 4's `.ptr` hazard machinery is obsoleted by Spec 2, but a *new* arena-consistency obligation appears (Spec 4 §2.4/§2.5 vs Spec 2 §3.B.2)

Spec 4 spends §2.4–§2.5 on the "owning-pointer hazard": word-logging `set_ptr` restores a raw `mem[p+5].ptr` that may dangle/leak, mitigated with a `touched_ptr` flag and clone-fallback. **Once Spec 2 lands, `mem[p+5]` holds an integer handle, not a pointer** — restoring it is safe, and Spec 4 §2.4/§2.5's entire `touched_ptr` apparatus can be **removed**. Spec 4 was written assuming blobs stay in the union.

But a **new seam** replaces it: a word-log restore reverts the *handle integer*, while the `GlyphBlobArena` slab/free-list also mutated during the epoch. Reverting the handle without reverting the arena's slab state can dangle (handle points at a slot the arena freed and reused) or leak. **Resolution:** the `GlyphBlobArena` must be epoch-managed alongside the word-log — either snapshot the (cheap, Arc-refcount) slab+free vectors in the `MemFrontier`/`EpochPodBundle`, or forbid word-log epochs that touch the arena (the "re-break only, no re-shape" fast path Spec 4 §2.4 option 1 already prefers). This is a net simplification but must be stated as a replacement obligation, not a deletion.

### Seam 6 — Spec 4's `set_*` instrumentation vs Spec 2's `copy_from_slice` restore (write-path coordination)

Spec 2 §2.5 `restore` writes **directly into `mem.data` via `copy_from_slice`**, bypassing the instrumented `set_*`. If a Spec 4 word-log epoch is active, a Spec 2 coarse restore mutates the arena **without journaling**, desynchronizing the `touched` bitmap and log. **Resolution:** coarse restore (Spec 2/3) and word-log epoch (Spec 4) are mutually exclusive states of the same layered system — a coarse restore must `epoch_commit()`/clear the word-log first (Spec 4 §2.3 has `epoch_commit`; wire it into Spec 2's restore). Spec 4 §4 already frames this ("mem.restore() is not used to cross clone boundaries; instead reset to the clone and epoch_begin") — but Spec 2's restore body doesn't know about the journal. They must be integrated: one restore entry point that clears the journal.

### Seam 7 — Rewind set agreement on SyncTeX (Spec 3 §D.2 vs Spec 4 §4 vs Spec 5 §6.3) — CONSISTENT

The one place the specs were independently written and **agree**: all three put the SyncTeX tag counter in the fast-path rewind set. Spec 3 §D.2 step 5 restores `cur_synctex_tag`, `mem.line`, `synctex_tag_counter`, `synctex.tag/line`; Spec 4 §4 lists `SynctexCtxt.synctex_tag_counter` in the `EpochPodBundle`; Spec 5 §6.3/PO-SYNC-1 states `synctex_tag_counter` "must be in the fast-path rewind set." They also agree on field *location* (`cur_synctex_tag`/`line` in `MemState`; `synctex_tag_counter` in `SynctexCtxt`). **No action** beyond confirming the unified `Snapshot` includes all of them. Likewise the **page-builder/contrib-list double-append trap** is independently flagged by Spec 3 §D.2 step 4, Spec 4 §4, and Spec 5 §10 — a strong three-way cross-validation; keep `page_tail`/`CONTRIB_HEAD`/`PAGE_HEAD` in the rewind set.

### Seam 8 — Output strategy: incremental XDV (Spec 3 §E.5) vs display-list bypass (Spec 5 §4)

Spec 3 Part C's `restore_and_replay` ends by calling `engine_finish` (which **emits XDV**), and Spec 3 §E.5 wrestles with DVI-stream coherence (buffer in memory, flush only at `engine_finish`, re-emit from `checkpoints[0]`). Spec 5 §4 **bypasses XDV entirely on the foreground path** — `ship_out_dl_in` walks the same vlist into a `Vec<DlItem>`; XDV is produced only by background convergence. These are two different visions. **Resolution:** Spec 5's DL path is the foreground preview; Spec 3's XDV re-emit applies to the **background/final** compile only. Concretely, Spec 3's `ResidentEngine::compile`/`restore_and_replay` must be **parameterized by sink** (XDV via `engine_finish` for batch/background; DL via `ship_out_dl_in` for foreground). Spec 3 §E.5's "never flush DVI to the real sink before `engine_finish`" obligation is then vacuous on the foreground path and retained for background. They are compatible once the sink is a parameter.

### Seam 9 — Two `ResidentEngine` definitions (Spec 3 Part A vs Spec 5 §5.1)

Both define `ResidentEngine` + `CheckpointLadder` + `Checkpoint`. Spec 3's is the engine mechanics (boot/compile/restore_and_replay/rewind_paragraph, Tiers 0/1/2, `into_history_recoverable`). Spec 5's is the topology (on_edit, debounce, background thread, `EngineCmd/Event`, DL frames, synctex index). The `Checkpoint` fields differ (Spec 3: `src_pos/src_line/snap`; Spec 5: `input_pos/snap/page_base/dl_prefix_hash`). **Resolution:** Spec 3's `ResidentEngine` (src/resident.rs) is the **core**; Spec 5's becomes a higher-level `ResidentSession` wrapping it and adding threading/source-model/DL-frame/diff. Merge the `Checkpoint` field sets. Spec 3 has no background convergence; Spec 5 §5.2 adds it — see Seam 10.

### Seam 10 — Spec 5's native two-thread topology violates the single-locator invariant (Spec 5 §5.1/§5.2 vs Spec 2 §5 vs Spec 6 §B.5) — **UNRESOLVED, HIGH RISK**

Spec 5 §5.2 spawns a **background thread** with a cloned engine (`self.warm.snapshot().into_owned_engine()`) running the font FFI. But:
- **Spec 2 §2.2** declares the `Snapshot` **`!Send`** ("raw locators … single-thread only").
- **Spec 6 §B.5 / Spec 2 §5** establish that `ENGINE_STATE_PTR` and the three `FONT_*` mirror statics (globals.rs) are **process-global locators**. The `FONT_*` mirrors are `static mut` (not thread-local). Two engines shaping on two threads would **race on `FONT_*`** (data race / UB), and the macOS `font_feature_warning` callback resolves a single global locator.

So Spec 5's native two-thread convergence is **unsound as written**. **Resolution options (must pick one):** (a) make `ENGINE_STATE_PTR` and `FONT_*` genuinely thread-local *and* prove font handles are thread-confined — large, risky; or (b) adopt Spec 5 §5.4's **cooperative single-thread** model on native too (the background compile yields at `build_page` boundaries via the same `cancel`/generation check), which sidesteps the shared-locator race entirely. Recommend (b) for v1; it also gives WASM parity for free. Note also Spec 2's "`Arc<[u8]>` non-atomic sharing" remark (§2.2) is a terminology slip — `Arc` is atomic/`Send`; the real `!Send` cause is the raw font pointers and the shared locators, which is the actual blocker for (a).

### Seam 11 — Quiescence: "may snapshot inside a group" (Spec 2) vs "only `save_ptr==0`" (Spec 3/4/5)

Spec 2 §2.1 and §8.8 explicitly permit `save_ptr > 0` at snapshot (capturing `save_stack.data[..=save_ptr]`) and tout snapshotting inside a group as a **capability**. Spec 3 §B.2 `is_quiescent`, Spec 4 §4, and Spec 5 §3/PO-FMT-1 **require** `save_ptr==0`, with Spec 5 citing the `store_fmt_file` constraint (format_io.rs). **Resolution:** this is **mechanism vs policy**, not a contradiction. The Snapshot *mechanism* (Spec 2) correctly handles `save_ptr>0`. The checkpoint-*ladder policy* (Spec 3/5) places rungs only at `save_ptr==0` outer-vmode boundaries so replay resumes cleanly. Spec 5/3 slightly **over-cite** the format-dump constraint: as Spec 2 §8.8 notes, the dump restriction is about `\dump`, not about snapshot soundness. Document them as distinct: general snapshots may occur in a group; ladder rungs are restricted by policy.

### Seam 12 — Three escalation/locality mechanisms for "is this edit local?" (Spec 3 §E.2–E.3 vs Spec 4 §4 vs Spec 5 §7)

Three independently-designed detectors:
- **Spec 5 §7:** lexical *pre-scan* of the edited byte range for `\ref/\gdef/\global/\setcounter/…` → route before running.
- **Spec 3 §E.3:** *runtime* `escape_epoch` bumped in `save.rs` `geq_*`/`eq_word_define@level1`, plus str_ptr/font_ptr/hyph movement + dimension check → escalate after running.
- **Spec 4 §4:** eqtb *write-log* detects any level-1/out-of-scope eqtb touch → clone fallback.

No conflict, but redundant. **Resolution:** compose as one policy — Spec 5's lexical scan is the cheap **pre-filter** (avoid even starting a fast path); Spec 4's eqtb write-log is the **precise runtime mechanism** implementing Spec 3's detection; Spec 3's hand-enumerated `save.rs` hooks are **subsumed** by Spec 4's write-log (which cannot miss a write path the way manual hooks can — see Audit §2.3). Use Spec 4's write-log as the authoritative escape detector; keep Spec 3's str_ptr/font_ptr/hyph/dimension signals as additional escalation triggers.

### Seam 13 — Spec 1 cache keys are process-scoped; Spec 6 cache key is cross-process (Spec 1 §2.1/§3.4 vs Spec 6 §A.8/A.9)

Spec 1's `FontFingerprint`/`EngineKey` capture name/size/letterspace/flags (+ font-file `content_digest`), sufficient for a **process-lifetime in-memory** cache (same libs, same platform within one process). Spec 6 §A.8/A.9 requires **platform (AAT vs HarfBuzz)** and **library versions (ICU/HarfBuzz/Graphite/TECkit/FreeType)** in any key that identifies shaping output. **Resolution:** consistent *as long as* Spec 1's caches stay in-memory/process-scoped (they do — Spec 1 §2.2 "process-level cache … survives across `tt_run_engine`"). If ever promoted to a persistent cross-process cache, Spec 1's keys must fold Spec 6 §A.9's `PLATFORM ‖ LIB_VERSIONS`. Flag as a forward constraint, not a present bug.

### Seam 14 — Strings high-water truncate (Spec 3 §B.4) vs byte-copy (Spec 2 §1) — proof-obligation divergence

Spec 3 §B-2 captures strings/fonts/hyph as **high-water marks** and truncates on restore, asserting appends are the exact inverse. Spec 2 captures `str_pool`/`str_start` as **bounded byte copies**. For fonts, Seam 4 already resolves in Spec 2's favor. For strings, Spec 3's truncate-only is cheaper **but** relies on "no rewrite below high-water" — TeX can `flush_string` (decrement `str_ptr`), and whether pool bytes below a checkpoint's high-water can ever be reused/overwritten before restore is a real proof obligation (Spec 3 §B-2 asserts write-once; that needs the no-flush-below-checkpoint invariant verified). **Resolution:** default to Spec 2's byte-copy (unconditionally safe); adopt Spec 3's high-water truncate only after validating the invariant on the corpus. This is a genuine unproven optimization, not an established equivalence.

---

## 2. CORRECTNESS AUDIT

### 2.1 The reframing "accessor-instrumentation is write-complete except platform font shims" is INCOMPLETE — it omits in-Rust raw-pointer-escape writes (challenges Spec 4 §10.1, Spec 6 §B.6)

This is the most important finding. Spec 6 Part B rigorously proves **no C code** writes `mem/eqtb/font_info` (B.2), and concludes the `MemState`/`EqtbState` `set_*` set is "the complete write-surface" (B.6). Spec 4 §10.1 restates this as "every arena mutation goes through the nine setters."

**This is false for in-Rust code.** Spec 1 §1 and §2.3 and Spec 2 §3.B.2 all show `measure_native_node` writing node slots via **raw `MemoryWord` pointer arithmetic**, e.g. `(*node.add(1)).b32.s1 = w`, `(*node.add(3)).b32.s1 = h`, bypassing `set_b32s1`. Spec 4 §0 *itself* enumerates the hot-path raw-pointer escapes (`as_u16_ptr_mut`, `node_addr`) with writer call sites in `fonts.rs`, `shipout.rs`, `main_control.rs:5345–5602`, `math.rs`, `linebreak.rs`, `pic.rs`. **These escaped `*mut` writes are not journaled by Spec 4's `set_*` instrumentation.** Spec 6's proof is about the C boundary; it does not establish that Rust never writes the arena outside `set_*` — and Rust demonstrably does.

**Consequence:** Spec 4's word-log write-completeness (its foundational claim, §10.1) holds **only for epochs that perform no raw-escape write** — i.e. pure re-break with no native (re)shaping and no pic/native-text fill. Spec 4 §2.4 gestures at "re-break performs no `set_ptr`," but that argument is about *ownership*, not about the un-journaled width/height/depth writes in `measure_native_node`. **Verdict: PLAUSIBLE-to-CONFIRMED hole.** Required fixes: either (a) reroute the raw-escape writes in `measure_native_node`/native-text fill through instrumented setters (Spec 2's `measure_native_node` rewrite is the natural place — make it write width/height/depth via `set_b32s1`, not raw), or (b) restrict word-log epochs to shaping-free phases and detect any escape-write to force a clone fallback (analogous to Spec 4's dropped `touched_ptr`). The roadmap must not treat Spec 4's instrumentation as write-complete until this is closed.

### 2.2 Double-free freedom after the glyph-blob migration — HOLDS, conditional on Spec 1's justification path being ported (Spec 2 §3.B.2 proof, PA-1)

Spec 2's proof (mem holds only an integer handle; blobs are `Arc<Blob>`; `make_mut` COW on post-snapshot justification) is **sound**. The mechanics check out: at `store_justified_native_glyphs`, if a snapshot co-holds the Arc (strong_count 2), `arena.get_mut` → `Arc::make_mut` clones into the live slab, leaving the snapshot's copy intact; the node's handle still names the (now-unique) live slot. **Residual risk is integration, not logic:** Spec 1's justification code (§2.4) was written for a raw `*mut u8` in-place write. If it is ported without switching to `arena.get_mut`, it mutates through a stale/aliased pointer. **Verdict: CONFIRMED-safe if and only if Spec 1's in-place mutators are ported to the COW accessor.** Gate with Spec 2 §7.5 ASan on the native-font/justification subset.

### 2.3 Escape-guard completeness for Tier-0 soundness — the proof is sound but the guard is not proven complete (Spec 3 §E.4 vs §E.3, I.7)

Spec 3 §E.4's proof that Tier-0 from a non-adjacent checkpoint is **unsound** (free-list dangling from eqtb-referenced nodes in the freed range + counter incoherence) is rigorous and correct; the restriction of Tier-0 to `para_local` under a clean escape guard is the right conclusion. **But the whole edifice rests on the escape guard detecting *every* escaping mutation** (Spec 3 §I.7 admits the false-negative risk). Spec 3 §E.3 implements the guard as *hand-enumerated* `save.rs` hooks (`geq_*`, `eq_word_define@level1`) plus counter deltas — exactly the kind of manual enumeration that can miss a write path. **Verdict: the soundness is conditional and the condition is not independently proven.** Mitigation (and cross-spec synergy): replace the hand-enumerated hooks with **Spec 4's eqtb write-log** (§5.2), which detects *any* level-1/out-of-scope eqtb write by construction — turning "we think we hooked every global write" into "we journaled every eqtb write." Until then, Spec 3's negative test (§G.3 "forcing Tier-0 anyway would diverge") is the only backstop and must be a hard CI gate.

### 2.4 Bit-identical display-list extraction — PLAUSIBLE, with a maintenance hazard and an API mismatch (Spec 5 §4.3, PO-DL-1)

Spec 5 correctly identifies the fragile spots (`cur_glue` f64 accumulation **order**, `tex_round`, LR/bidi flips, `\XeTeXinterwordspaceshaping` list mutation) and the mitigation (mechanically clone `hlist_out_in`→`hlist_out_dl`, cross-check via `--dl-verify`). Three unproven residuals:
1. **The cross-check itself is nontrivial.** `--dl-verify` reconstructs XDV glyph positions "from the DVI movement stream" (§4.3) and asserts equality — but XDV rounds h/v to DVI units while `DlItem` keeps sp; the comparison must model DVI's rounding or produce false diffs/false passes. The reconstruction is new code that can be wrong in the same direction as the extractor.
2. **Duplicating a ~400-line function is a divergence farm.** Spec 5 §4.3 says "unify only once corpus-clean," but two hand-maintained copies of `hlist_out_in` will drift. **The `--dl-verify` cross-check must run continuously in CI**, not once — otherwise a later edit to the batch `hlist_out_in` silently desyncs the preview.
3. **API mismatch with Spec 2:** §4.2's extractor reads `native_glyph_info_ptr(&st.mem) as *const u8` — the pre-Spec-2 raw-pointer API (Seam 1). Must become `arena.get(handle)`. **Verdict: PLAUSIBLE; the geometry claim is credible but unverified until `--dl-verify` is green across the corpus including bidi/CJK/justified/leaders/math-as-kern cases.**

### 2.5 The `random_seed` fix — CORRECT and well-guarded, with one residual Spec 6 already concedes (Spec 6 §A.2, A.4)

The fix (derive seed from `build_date`, `& 0x7FFF_FFFF`) is sound: the original `microseconds*1000 + epochseconds%1e6` was already positive-in-`i32`; the new `build_date`-derived expression overflows `i32` (seconds ≈1.7e9 ×1000) so `wrapping_mul` + mask is needed and correct. `init_randoms` running unconditionally is XDV-neutral unless `randoms[]` is *read* (Spec 6 §C.2), validated by the neutral corpus pass. **The reframing that this makes a compile a pure function of classes 1–5 is correct for the RNG.** Residual, which Spec 6 §A.4/§C.6 honestly flags: **`localtime` TZ-dependence in `get_date_and_time` remains** — same `build_date`, different `\the\day` across machines/timezones — unfixed unless `TZ=UTC` is pinned or folded into the key. **Verdict: CONFIRMED-correct; TZ residual is real and unproven-hermetic.** Also note this fix is *required* for Spec 5's `diff_pages` cross-session stability (PO-DET-1) and Spec 1's cross-run cache validity — Spec 3 §0.4/§G.4 under-sells it by framing the wall-clock seed as "a help" (true only *within* one session).

### 2.6 Snapshot round-trip identity — the immediate-restore test is too weak; the prefix-rewind test is the real gate (Spec 2 §7.2 vs §7.3, Spec 5 PO-SNAP-1)

All snapshot specs assert "we captured every field," validated by snapshot-then-immediately-restore = identity (Spec 2 §7.2, Spec 5 §8.2). **This test cannot catch a missed field that happens to be equal at the restore instant but diverges after replay** (e.g. a scanner cursor, a `linebreak` scratch, `native_text` — Spec 2 marks several as "set-before-read scratch," an assumption that is only tested by replay). The decisive test is **Spec 2 §7.3 prefix-rewind equivalence** (run tail, restore, run a *different* tail, restore, run original tail, assert equality) and **Spec 3 §G.3 edit-equivalence vs cold batch**. **Verdict: the enumeration is thorough but "complete" is unproven; make the prefix-rewind/edit-equivalence differential the primary gate, not round-trip identity.** The `null-at-rest` `debug_assert`s (Spec 2 §2.3: `loaded_font_mapping`, `current_diagnostic`) are debug-only — in release a live scratch pointer would be captured and re-installed stale; these should be promoted to a checked precondition on the ladder's checkpoint-token selection.

### 2.7 macOS AAT reach-back — correctly scoped, but snapshot-during-shaping must be structurally forbidden (Spec 6 §B.5/B.6, Spec 2 §5)

Spec 6 §B.5 verifies the `font_feature_warning` reach-back is macOS-AAT-only, locator-mediated, and writes nothing outside instrumented accessors. Sound. The residual (Spec 6 §B.6 caveat 1): a snapshot taken *inside* a native `loadAATfont`/`layoutChars` call would be mid-callback. Spec 6 makes this a *constraint* ("only snapshot between `main_control` tokens"). **Verdict: CONFIRMED given quiescence.** But this must be *structurally enforced*, not just documented: the ladder's `is_quiescent` (Spec 3 §B.2) already guarantees between-token, so it holds — provided no checkpoint hook fires from within shaping. Also flag: this is the second reason Seam 10's two-thread topology is unsound — the AAT callback resolves a single global locator.

---

## 3. UNIFIED DEPENDENCY-ORDERED ROADMAP

**Impact/Effort/Risk:** H/M/L. **Horizon:** *Now* = enabling core; *Next* = incremental engine; *Later* = editor UI. The critical path is bolded.

| # | Work item (spec origin) | Impact | Effort | Risk | Depends on | Horizon |
|---|---|---|---|---|---|---|
| **W1** | **Finalize residual-C/FFI audit + CI grep guards for write-completeness (Spec 6 Part B; Spec 6 §C.1/C.5)** — including the §2.1 gap: enumerate in-Rust raw-escape writers | H | L | L | — | **Now** |
| W0 | `random_seed`←`build_date` fix + `deterministic_timers` config (Spec 6 §A.2/A.3/A.5) | M | L | L | — | Now |
| **W2** | **Unified `Arc<LayoutEngineHandle{raw,font_area}>` + `MappingHandle`; `font_layout_engine_raw` mirror; teardown/reload via Arc drop (Spec 2 §3.B.1; supersedes Spec 1 §3.2, Spec 5 E3)** | H | M | M | W1 | **Now** |
| **W3** | **Glyph blobs → `GlyphBlobArena<Arc<Blob>>`; `mem[p+5]`=handle; rewrite `measure_native_node` (route width/height/depth via `set_*`, §2.1) + 12 call sites (Spec 2 §3.B.2; obsoletes Spec 5 §2.4)** | H | H | M-H | W1 | **Now** |
| **W4** | **Canonical `Snapshot` (bounded copy) + in-place `restore` + `MemFrontier`/scalar bundles (Spec 2 §2; subsumes Spec 3 §B.4, Spec 5 §2.6)** | H | M | M | W2, W3 | **Now** |
| W5 | IO journaling (Spec 2 §4) + mirror/locator refresh on restore (Spec 2 §5) — **downgraded** to RAM-buffer path first | M | M | M | W4 | Now |
| **W6** | **`ResidentEngine` lifecycle split: `init_once`/`run_to_end`/`finish`/`teardown` + `into_history_recoverable`; `tt_run_engine` stays a byte-identical shim (Spec 3 Part A)** | H | M | M | W4 | **Next** |
| W7 | Checkpoint ladder + Tier-1 `restore_and_replay` + `MemBuf`/`SourceProvider` input (Spec 3 §B/C, Spec 5 E5) | H | H | M | W6 | Next |
| W9 | Shape cache rebuilt on `Arc<Blob>` share (Spec 1 Part A) — **independent of snapshot** | H | M | M | W3 | Next (parallel) |
| W10 | Word-log substrate: mem + eqtb/hash write-logs; instrument `set_*` **and close the raw-escape gap (§2.1)** (Spec 4 §2/§5) | H | H | H | W1, W3, W4 | Next (parallel) |
| W11 | Font-engine + TFM cross-run cache on W2 handle, keyed w/ content-digest + lib-version/platform (Spec 1 Part B + Spec 6 §A.8) | M | M | M | W2, W0 | Next |
| W8 | Tier-0 fast path + unified escape guard (Spec 4 eqtb-write-log as detector + Spec 5 lexical pre-scan + Spec 3 signals) | H | H | H | W7, W10 | Next |
| W12 | Display-list extractor `ship_out_dl_in` + `--dl-verify` continuous gate (Spec 5 §4) — uses W3 arena API | H | H | M-H | W3, W4 | Later |
| W13 | In-memory SyncTeX forward/inverse index (Spec 5 §6) | M | M | L | W12 | Later |
| W14 | Topology: **cooperative single-thread convergence + cancellation** (Spec 5 §5.3/§5.4; **reject native two-thread, Seam 10**) + WASM | M | H | H | W6, W12 | Later |
| W15 | Cache-key `K` formula + driver plumbing (Spec 6 §A.9) | M | M | L | W0, W11 | Later |
| W16 | Permanent CI: `--snap-roundtrip`, `--dl-verify`, feature-on-epoch-off corpus row, ASan native-font subset, TRIP (Spec 2 §7, 3 §G, 4 §8, 5 §8) | H | M | L | all | continuous |

### Critical path
**W1 → W2 → W3 → W4 → W6 → W7 → W8.** Everything else hangs off W3 (caches) or W4/W6 (snapshot/lifecycle). W0 is a cheap early prerequisite (small, unblocks W15 and Spec 5 diff-stability). **W2 and W3 are the two riskiest corpus-visible refactors** (FFI double-free surface); land each in isolation, corpus-gated as a byte-identical no-op (Spec 2 §7.1, Spec 5 E3 "dedicated corpus run"), before W4. W3 must land before W9/W12 because both consume the arena API.

### Caveats downgraded or removed by the verified pure-Rust finding (Spec 6 §B)

- **REMOVED — "must defeat live C mutation of the arena mid-run."** Spec 6 §B.2/B.6 proves no C statement reads or writes `mem/eqtb/font_info` (the C symbols are dead linker storage). Snapshot of `MemState`/`EqtbState` is therefore *complete with respect to C*. This is the linchpin that makes W4 a hand-written value capture rather than an impossible cross-language coherence problem.
- **DOWNGRADED — file-handle journaling risk (Spec 2 §4, Spec 5 §2.5).** Because the foreground fast path is fully in-RAM (editor owns the source), UFile byte-seek/decode-state restoration is needed only for file-backed inputs; the RAM-buffer preview reduces journaling to a byte cursor. W5 can ship the RAM path first and defer the UFile POD-body seek.
- **DOWNGRADED — glyph-blob double-free hazard (Spec 5 §2.4).** Superseded by W3's Arc arena; the deep-copy-on-clone loop is deleted, and double-free becomes unrepresentable by construction (Spec 2 §3.B.2 proof), pending the §2.2 port of Spec 1's in-place mutators.
- **DOWNGRADED — Spec 4's `.ptr` owning-pointer hazard (§2.4/§2.5).** After W3, `mem[p+5]` is an integer handle; the `touched_ptr`/clone-fallback machinery is removed (replaced by the smaller GlyphBlobArena-epoch obligation, Seam 5).

### Caveats explicitly NOT removed (retained/added by this analysis)

- **RETAINED & SHARPENED — accessor write-completeness is NOT total (Audit §2.1).** The reframing holds for the *C* boundary, not for in-Rust raw-pointer-escape writers (`measure_native_node` et al.). W10 must close this or restrict epochs; do not mark Spec 4 write-complete until then.
- **RETAINED — macOS AAT reach-back + single global locator (Audit §2.7, Seam 10).** Forces (a) snapshots only at quiescent points, (b) **platform in the cache key** (Spec 6 §A.9), and (c) **rejection of Spec 5's native two-thread topology** in favor of cooperative single-thread.
- **RETAINED — `localtime` TZ non-hermeticity (Audit §2.5)** and **shell-escape non-hermeticity (Spec 6 §A.6)** — never cache/restore across a shell-escape boundary; pin or key `TZ`.