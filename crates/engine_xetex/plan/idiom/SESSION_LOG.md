# Idiom session log

Recent Stage 2 idiomatization handoff log. Newest entry is at the top.
Read this file first in new sessions; only open archives when investigating older context.

Archives:
- `session-log/sessions-100-149.md`
- `session-log/sessions-050-099.md`
- `session-log/sessions-001-049.md`

## Idiom session 182 — 2026-06-13 — Execution: thread the print core + the output.rs show-node walkers

**Type:** execution (workstream 01 / fused `01b`+`01c`). Large low-docs batch
per user request. Took the documented keystone — the **print subsystem** — then
cashed it out across the `output.rs` show-node walkers, the single largest
mechanical vein (top of `phase_c_recon.py`: `show_*`/`print_*`, all blocked
because the print primitives were `with_state`-bearing so a walker holding a
`mem` borrow nest-panicked on any print call).

**Code (2 commits):**
1. `f0475cdd` — threaded the **print core** over `&mut EngineState`: `_in`
   cores for `print_char`/`print_raw_char`/`print_ln`/`print`, the
   cstr/nl/esc wrappers, `print_int`/`hex`/`scaled`/`roman_int`/`ucs_code`,
   `print_the_digs`, `warn_char`, `print_current_string`, touching
   `shipout`/`errors`/`strings`/`eqtb`/`config` fields directly; the global
   `unsafe fn`s became thin `with_state_mut` wrappers (behaviour-preserving by
   construction). Removed dead `doing_special`/`set_trick_buf`.
2. `9d898677` — threaded the **walkers**: `short_display`, `show_node_list`
   (+ `show_box`/`show_info`/`print_subsidiary_data`) and ~22 leaf print
   helpers (`print_native_word`, `print_cs`, `sprint_cs`, `print_file_name`,
   `print_spec`, `print_glue`, `print_rule_dimen`, `print_fam_and_char`,
   `print_delimiter`, `print_font_and_char`, `print_mark`, `show_token_list`,
   `token_show`, `print_size`, `print_write_whatsit`, `print_sa_num`,
   `print_mode`, `print_in_mode`, `print_group`, `print_skip_param`,
   `print_style`, `print_totals`). Walkers read mem/eqtb/strings/save_stack
   directly and call the print `_in` cores. Per established precedent the `_in`
   cores are **safe `fn`** with narrow `unsafe {}` only around the genuine
   FFI / raw-ptr / `CStr` ops (the de-unsafe drops `unsafe fn` rather than
   raising it). Added `MemState::gr`. `show_node_list` (480 lines) converted
   via a scoped ordered Python transform. Removed 11 now-dead global wrappers +
   `breadth_max`/`depth_threshold`/`set_*`/`font_in_short_display` accessors.
   `temp_ptr` stays a `mod raw` call (raw-safe under a borrow, §4).

**Tests:** `cargo check`+`fmt --check`; `node::` **48/48**; dashboard
**29/2/7/4** (after each commit); rebuilt `target/release/tectonic`;
`plan/scripts/corpus.sh check all`: **637 passed / 2 diverged / 0
missing-golden**, only the known FP divergences `1905.00073v1` /
`1905.00148v1`. Metrics: warnings **41**; `unsafe fn` **564 → 551**.

**Next:** thread `show_activities` (top recon target, score 125; needs the
`nest_*` cluster), then `show_save_groups`/`show_context`/`print_cmd_chr`/
`print_meaning`/`print_param`/`print_length_param` — all were blocked behind
the print subsystem and are now unblocked (their callees are the `_in` cores
landed this session).

---

## Idiom session 181 — 2026-06-13 — Execution: threaded hyphenation trie-op/init/pattern insertion clusters

**Type:** execution (workstream 01 / fused threading, low-docs larger
mechanical batch). Used two explorer subagents for sidecar scouting: one
confirmed the next output/shipout work should start with a primitive print-core
threading slice; one confirmed `init_trie`/`new_patterns` were the next clean
hyphenation clusters.

**Code:** threaded `new_trie_op` into `new_trie_op_in(&mut HyphState, ...)`,
folded `init_trie` into `init_trie_in(&mut HyphState)`, and threaded the
completed-pattern word insertion plus saved-hyphen-code insertion subclusters.
Scanner/output calls and duplicate-pattern diagnostics stay outside hyph
borrows; overflow diagnostics still go through `TrieAbort`/`trie_abort` after
the borrow drops. Deleted the now-dead trie-op/trie-pack bridge accessors from
`globals.rs`; live engine/format/linebreak bridges stayed.

**Validation:** `cargo fmt --check -p tectonic_engine_xetex`; `cargo check -p
tectonic_engine_xetex`; `cargo test -p tectonic_engine_xetex node::` 48/48;
dashboard **29/2/7/4**; `cargo build --release -p tectonic`; full corpus
`plan/scripts/corpus.sh check all` = **637 passed / 2 diverged / 0
missing-golden**, only known `1905.00073v1` and `1905.00148v1`. Warnings
remain **41**; `unsafe fn` **566 -> 564**.

**Next:** either continue `hyphenation.rs` with short-borrow
`new_hyph_exceptions` subclusters, or take the output print-core slice before
`short_display`/`show_node_list` (`print_char`/`print_ln`/string/numeric
primitives behind wrappers).

---

## Idiom session 180 — 2026-06-13 — Execution: typed hpack native-word cleanup + threaded hyphenation trie packing

**Type:** execution (workstream 01 / fused threading+typing), larger
low-docs mechanical batch. Used two explorer subagents for scouting only:
one confirmed output/shipout walkers need a threaded print core first; the
other identified `hyphenation.rs` trie packing as the cleanest non-node slice.

**Code:** added `WhatsitNode::native_text_ptr{,_mut}` and used typed
`WhatsitNode`/`AdjustNode`/`InsNode`/`MarkNode` accessors in the already-
threaded `hpack_in` native-word merge / adjust extraction path. Threaded
`trie_node`, `compress_trie`, `trie_fix`, `first_fit`, and `trie_pack` over
`&mut HyphState`; `first_fit`/`trie_pack` return a local overflow sentinel so
`overflow("pattern memory")` still runs after the state borrow drops. Removed
dead `trie_taken`/`trie_min` bridge accessors from `globals.rs`.

**Validation:** `cargo fmt`; `cargo check -p tectonic_engine_xetex`;
`cargo test -p tectonic_engine_xetex node::` (**48/48**); dashboard
**29/2/7/4**; `cargo build --release -p tectonic`; `git diff --check`;
`plan/scripts/corpus.sh check all` = **637 passed / 2 diverged / 0
missing-golden**, only known `1905.00073v1` and `1905.00148v1`. Warnings
remain **41**; `unsafe fn` **568 → 566**.

**Next:** either keep going in `hyphenation.rs` with `new_trie_op` using the
same overflow-sentinel pattern, or take the print-core slice before
`short_display`/`show_node_list`. Avoid `shipout` walkers until the print/DVI
bridges are threaded.

---

## Idiom session 179 — 2026-06-13 — Execution: type the last untyped list-node families (Mark/Adjust/Ins/Unset + eTeX sparse-array marks)

**Type:** execution (workstream 01 / `01c` Stage 3 — the typing backlog, not
threading). Larger low-docs session per user request; 2 code commits.

**Recon finding (why this shape):** the cheap typed-*consumer* vein in
already-threaded math/packaging fns is exhausted, and the remaining math.rs
`mem_*` mass is bucket-C/D (`init_math`/`app_display`/`after_math` are
scanner/output-coupled; `clean_box`/`make_fraction_global` interleave
`mlist_to_hlist`/`hpack`/OT-math globals). The genuinely-mechanical,
compiler-checked work left is **building typed surfaces for the families
`AnyNode` still carried as raw `MemIdx`** (`01c` §1 rank 9/11) and adopting
them in the already-threaded packaging walkers.

**Code (2 commits):**
1. `12ea8d28` — built `MarkNode` (mark_ptr +1.b32.s1, mark_class +1.b32.s0),
   `AdjustNode` (adjust_ptr +1.b32.s1), `InsNode` (ins_ptr +4.b32.s0,
   split_top_ptr +4.b32.s1), and `UnsetNode` (box-layout width/depth/height +
   `HasListPtr`; deliberately **not** `HasDimensions` — its +4 slot is
   `glue_shrink`, not `shift_amount`, a naming landmine avoided). Promoted
   `AnyNode::{Mark,Adjust,Ins,Unset}` from raw `MemIdx` to `NodePtr<K>`, so
   `classify()` now confines each family's `from_idx_unchecked` to its one
   audited arm. Adopted the typed accessors in `flush_node_list`,
   `copy_node_list`, `hpack_in`, and `vpackage_in`. **Whatsit is now the only
   raw `AnyNode` variant.** 4 `node::` roundtrip tests.
2. (this commit) — built `SaNode` (eTeX sparse-array record: indexed
   `child(mem, i)` at `mem[p+i/2+1].b32.s{0|1}` + `ref_count` at `b16.s0`) and
   fully typed the already-threaded `do_marks` with it, replacing all ~17 raw
   `mem.b32s{0,1}(q+N)` / `b16s0(q)` sparse-array accesses (the token-list
   refcount reads on the *pointed-to* lists stay raw — those aren't a typed
   family). The audited handle is built once at the `do_marks` entry by
   context (no `NODE_type` to classify). 1 `node::` test.

**Disasm gate:** skipped per the session-150 precedent — every new accessor is
an already-ICF-proven shape (`MemPtr`/`Scaled` at +1/+2/+3/+4.b32.s0/s1,
`b16.s0`), so no new shape was introduced.

**Tests:** `cargo check` + `fmt --check`; `node::` **47/47**; full dashboard
**29/2/7/4** (after each slice); rebuilt `target/release/tectonic`; corpus
`plan/scripts/corpus.sh check all`: **637 passed / 2 diverged / 0
missing-golden**, only the known FP divergences `1905.00073v1` and
`1905.00148v1` (no regression). Metrics: warnings **41**; `unsafe fn` **568**;
`from_idx_unchecked` **125 → 131** (+6: the 4 promoted-family classify arms + 1
`SaNode` test + 1 `do_marks` audited handle — all single-site boundaries; node
74→79, packaging 0→1, math 36, linebreak 12, mem 3); **packaging method-form
`+N` accesses 89 → 52 (−37)**.

**Next:** the only untyped `AnyNode` family left is **Whatsit/native** (`01c`
§1 rank 9) — variable-size, trailing inline UTF-16/glyph data, a raw native
pointer, FFI-measured: the documented "hardest, last" family and a dedicated
slice, not a mechanical batch. Otherwise the remaining levers are all
bucket-C/D threading: the print subsystem (`print_*` → state params, which
unblocks the `shipout`/`output` show-node walkers), the display-math trio
`init_math`/`app_display`/`after_math`, or the `pagebuilder`/`hyphenate`/
`line_break` tails.

---

## Idiom session 178 — 2026-06-13 — Execution: thread/type `mlist_to_hlist` (both passes)

**Type:** execution (workstream 01 / fused `01b`+`01c`). Took the documented
next bridge host — the central math driver `mlist_to_hlist` — and threaded/
typed its tractable islands across both passes. Larger-than-usual session per
user request (4 code commits), batched validation, single doc update. First
committed the inherited sessions 174–177 WIP (`1dbd2808`) as a clean baseline.

**Recon finding (why this shape):** there is no large block of easy bucket-A/B
work left. The remaining raw-accessor mass is in big bucket-C/D hosts
(`main_control` 744 raw `mem_*` calls, `math` 343, `shipout` 309). The
`output.rs` show-node walkers the recon ranks as "B" are actually bucket-C:
`print_char`/`print_esc`/… are `with_state`-bearing, so a walker holding a mem
borrow across a print nest-panics. So "bigger session" = grind *many* islands
through one host, not find easy whole-function threads.

**Code (4 commits, all in `src/math.rs`):**
1. `dfa8a34e` pass-1 reswitch CHOICE_NODE + GLUE_NODE → typed `with_state`
   islands. CHOICE via `classify_choice` + `ChoiceNode` branch accessors +
   threaded `flush_node_list` (new `mlist_choice_node` helper); GLUE via
   `GlueNode` subtype/glue_ptr + threaded `delete_glue_ref`/`flush_node_list`.
2. `9e4b3546` pass-1 action dispatch: new `mlist_track_box` (hpacked box
   height/depth → max_h/max_d + typed `free_node`, one borrow) and
   `mlist_advance` (noad step + style reset; r_type + link read in one borrow);
   applied to ProcessNucleus/CheckDimensions/DoneWithNoad, deduplicating the
   repeated tail.
3. `ba798025` pass-2 islands: BIN_NOAD fixup + list-reset prelude; the
   move-and-detach arm; new-hlist splice + tail walk; inter-element glue
   spacing (threaded `new_glue`); penalty insertion (threaded `new_penalty`);
   final link-read + threaded `free_node`. Removed the dead `new_glue_global`
   import.
4. `c848a53b` ProcessNucleus nucleus store: fold `set q+1 = p` + the
   script-presence check (q+3/q+2 != EMPTY) into one borrow guarding the
   bucket-D `make_scripts_global`.

Bucket-D calls (`make_*_global`, `hpack`, native FFI, `mlist_to_hlist`
recursion) stay outside the borrows by design.

**Tests:** `cargo check`+`fmt --check`; `node::` 42/42; full dashboard
**29/2/7/4** (run after the pass-2 batch); rebuilt `target/release/tectonic`;
corpus `plan/scripts/corpus.sh check all`: **637 passed / 2 diverged / 0 missing-golden**, only the known FP
divergences `1905.00073v1` and `1905.00148v1` (no regression). Metrics:
warnings **41**; `unsafe fn` **568**; `from_idx_unchecked` total **125**
(node 74, math 36, linebreak 12, mem 3, packaging 0; +1 in math from the
GLUE-arm classified-boundary handle); **`math.rs` raw `mem_*` bridge calls
343 → 276 (−67)** — the headline reduction this session.

**Next:** `mlist_to_hlist`'s remaining raw islands are bucket-D-heavy (the
ProcessNucleus MATH_CHAR/SUB_MLIST head: `fetch`, `new_native_character`,
`measure_native_glyph` FFI, `hpack`, recursion). Continue with another bridge
host. `make_op_global`/`make_math_accent_global` are already largely threaded;
the biggest unblocked levers now are threading the **print subsystem**
(`print_*` → state params), which would unblock the whole `shipout`/`output`
show-node-walker family, or the `pagebuilder`/`hyphenate`/`line_break` tails.

---

## Idiom session 177 — 2026-06-13 — Execution: thread/type `make_math_accent_global` islands

**Type:** execution (workstream 01 / fused `01b`+`01c`). Followed session
176 by taking another real bridge host, not pointer-only cleanup.

**Code:** split `make_math_accent_global` into typed helper islands for the
initial accent noad/font snapshot, scripted-base reclean, native glyph setup
around `measure_native_glyph`, box dimension/shift updates, and top/bottom
accent stack assembly. Kept `clean_box`, `vpackage`, OpenType variant/assembly
callbacks, and native glyph measurement outside live `with_state` borrows.
Top-accent kern insertion now uses the typed `KernNode` allocator; repeated
noad conversions in this slice go through one local `assume_noad` boundary.

**Tests:** `cargo check -p tectonic_engine_xetex`; `cargo fmt --check -p
tectonic_engine_xetex`; `cargo test -p tectonic_engine_xetex node::` **42/42**;
dashboard **29/2/7/4**; rebuilt `target/release/tectonic`; final
`plan/scripts/corpus.sh check all`: **637 passed, 2 diverged, 0
missing-golden**, only known `1905.00073v1` and `1905.00148v1`;
`git diff --check`. Metrics: warnings **41**; `unsafe fn` **568**;
`from_idx_unchecked` total **124** (node.rs 74, math 35, linebreak 12, mem 3;
packaging 0).

**Next:** continue with another bridge host: `mlist_to_hlist` itself,
`make_op_global`, shipout/output walkers, or the `hyphenate`/`line_break`
tail.

---

## Idiom session 176 — 2026-06-13 — Execution: thread `vsplit` as a coherent packaging slice

**Type:** execution (workstream 01 / fused `01b`+`01c`). Continued the
session-175 packaging WIP by taking the deferred `vsplit` bridge island,
not a pointer-only cleanup.

**Code:**
1. Split `vsplit` into an explicit-state `start_vsplit`/`vsplit_resume`
   core over `MemState`/`EqtbState`/`ScannerState`/`ShipoutState`, while
   keeping the legacy `pub unsafe fn vsplit` wrapper. The core now threads
   through `find_sa_element`, `delete_sa_ref`, `flush_node_list`,
   `do_marks`, `prune_page_top`, `vpackage_in`, `free_node`, and
   `delete_token_ref` instead of chaining the old `_global` bridge calls.
2. Moved the migrated state boundaries into the slice: `scanner.disc_ptr`,
   `scanner.sa_root`, `scanner.cur_mark`, and box-register `eqtb` writes are
   direct. The source box is classified as a typed `NodePtr<VListNode>`.
   `cur_val`/`cur_ptr` stay as raw hot globals, and Mark-node fields stay raw
   because that family is intentionally not typed yet.
3. Reused the resumable `vert_break` core rather than calling the legacy
   wrapper under a borrow. Infinite-shrinkage still pauses through the
   `vsplit` wrapper, prints the original diagnostic after dropping the state
   borrow, repairs the glue spec through the threaded typed helper, and then
   resumes the same `VertBreakCursor`.
4. Plumbed `vpackage_in` outcomes through `VsplitOutcome` so vpack
   diagnostics are printed only after the state borrow drops. Both potential
   packages are preserved and printed in TeX order.

**Tests:** `cargo check -p tectonic_engine_xetex`; `cargo fmt --check -p
tectonic_engine_xetex`; `cargo test -p tectonic_engine_xetex node::` **42/42**;
dashboard **29/2/7/4** (`tex-outputs`, `trip`, `engine_rust`,
`latex_goldens`); rebuilt `target/release/tectonic`; ran
`crates/engine_xetex/plan/scripts/corpus.sh check all`: **637 passed, 2
diverged, 0 missing-golden**, only the known `1905.00073v1` and
`1905.00148v1` (script exits nonzero because it reports those known diffs);
`git diff --check`. Metrics: warnings **41**; `unsafe fn` **568**;
`from_idx_unchecked` total **123** unchanged (node.rs 74, math 34,
linebreak 12, mem 3; packaging 0).

**Next:** packaging's `vert_break`/`vsplit` bridge island is now threaded.
Continue with another real bridge host (`mlist_to_hlist`, shipout/output
walkers, `make_op_global`/`make_math_accent_global`, or the
`hyphenate`/`line_break` tail). Do not spend the next slice on pointer-only
cleanup.

---

## Idiom session 175 — 2026-06-13 — Execution: thread and type `vert_break`

**Type:** execution (workstream 01 / fused `01b`+`01c`). Targeted the
remaining packaging global-accessor island instead of another pointer cleanup.

**Code:**
1. Split `vert_break` into a resumable explicit-state core over
   `MemState`/`ScannerState` and kept the legacy `pub unsafe fn vert_break`
   as the bridge wrapper. The core no longer calls `mem_b16s1`,
   `mem_b32s1`, `active_width`, or `best_height_plus_depth` globals; it
   reads/writes `scanner.active_width` and `scanner.best_height_plus_depth`
   directly.
2. Adopted typed/classified access in the new core: list-node dispatch goes
   through `MemState::classify`; HList/VList/Rule dimensions use typed
   accessors; Glue/Kern/Penalty arms use `GlueNode`, `GlueSpecNode`,
   `KernNode`, and `PenaltyNode` methods. Whatsit PIC/PDF and untyped
   Ins/Mark behavior remains raw as planned.
3. Preserved the mid-loop infinite-shrinkage error ordering by making the
   core pause with a `VertBreakStep::InfiniteShrinkage`. The wrapper drops
   the state borrow, emits the original diagnostic/error, then reborrows to
   copy the glue spec with threaded `new_spec`, set the copied
   `shrink_order` through the typed glue-spec setter, delete the old ref via
   threaded `delete_glue_ref`, and resume the same TeX loop state.

**Not done:** `vsplit` was inspected but left as the next slice. It still
crosses several bridge boundaries (`find_sa_element_global`, mark state,
`prune_page_top_global`, `vpackage`, box-register writes); threading it
coherently should pull those boundaries together rather than just retyping
raw fields in place.

**Tests:** `cargo check -p tectonic_engine_xetex`; `cargo fmt --check -p
tectonic_engine_xetex`; `cargo test -p tectonic_engine_xetex node::` **42/42**;
dashboard **29/2/7/4** (`tex-outputs`, `trip`, `engine_rust`,
`latex_goldens`); rebuilt `target/release/tectonic` and ran
`plan/scripts/corpus.sh check all`: **637 passed, 2 diverged, 0
missing-golden**, only the known `1905.00073v1` and `1905.00148v1`; `git
diff --check`. Metrics: warnings **41**; `unsafe fn` **568**;
`from_idx_unchecked` total **123** unchanged (node.rs 74, math 34,
linebreak 12, mem 3; packaging 0).

**Next:** thread `vsplit` as a coherent slice, probably including the mark
collection loop, sparse-array calls, `prune_page_top`, and `vpackage` outcome
plumbing so the function can stay under explicit state without nested
bridges. Keep Unset/Ins/Mark/Adjust/Whatsit fields raw unless their typed
surfaces are deliberately built.

---

## Idiom session 174 — 2026-06-13 — Execution: prettify typed packaging arm bodies

**Type:** execution (workstream 01 / `01c` §3 follow-through). Continued
the session-173 packaging work: arms already received typed `AnyNode`
handles, but several bodies still reached back through raw `p + N` /
`r + N` offsets for built families.

**Code:**
1. Added a small `copy_box_shell` helper for `just_copy`/`copy_node_list`:
   it copies the full box shell, classifies the copied header as an
   `AnyBox`, and sets `list_ptr` through the typed setter. HList/VList
   copy paths now read child lists through `list_ptr()`; `Unset` stays raw
   because it is not typed yet.
2. Retyped glue/ligature/disc copy bodies: source fields now come from
   `GlueNode::{glue_ptr,leader_ptr}`, `LigatureNode::lig_ptr`, and
   `DiscNode::{pre_break,post_break}`; copied destination nodes copy their
   header first, classify, then write destination pointer fields via typed
   setters. This removed the old `mem_b32 p + 1` style bodies while keeping
   allocation sizes and copy order equivalent.
3. Retyped more already-classified consumers: `flush_node_list` uses typed
   choice branch pointers; `prune_page_top` uses typed HList/VList/Rule
   heights via a factored split-top-skip helper; `hpack`/`vpackage` measure
   leader boxes/rules through typed dimensions when classification succeeds
   (fallback remains the old raw read for untyped/unexpected leader records);
   `hpack`'s ligature temporary char-node now uses ligature `font`,
   `character`, and `link` accessors.
4. Removed the dummy `r = 0` initialization in `copy_node_list`; the
   compiler now verifies every copy path assigns `r`.

**Left raw deliberately:** `Unset`, `Ins`, `Mark`, `Adjust`, and `Whatsit`
fields in these walkers still use raw offsets because those families are not
typed yet. `vert_break`/`vsplit` still read through global `mem_*` accessors;
threading them is a separate fused slice, not an arm-body cleanup.

**Tests:** `cargo check -p tectonic_engine_xetex`; `cargo fmt --check -p
tectonic_engine_xetex`; filtered `node`/`units` tests **46/46**; dashboard
**29/2/7/4** (`tex-outputs`, `trip`, `engine_rust`, `latex_goldens`);
`git diff --check`. Metrics: warnings **42 → 41**; `unsafe fn` **568**;
`from_idx_unchecked` total **123** unchanged (node.rs 74, math 34,
linebreak 12, mem 3; packaging 0).

**Next:** the remaining packaging raw offsets are either untyped-family
fields or unthreaded/global islands. Continue with a real fused threading
slice (`vert_break`/`vsplit`, `mlist_to_hlist`, shipout/output walkers), or
build typed surfaces for the low-rank families still represented as raw
`MemIdx` (`Unset`, `Ins`, `Mark`, `Adjust`, selected Whatsit/native records).

---

## Idiom session 173 — 2026-06-13 — Execution: adopt `classify` across the packaging list-walkers (large, low-docs)

**Type:** execution (workstream 01 / `01c` §3 — the session-172 "Next"
item: clean `classify` adoptions). 2 commits.

**Code:**
1. `flush_node_list` / `copy_node_list` / `just_copy` now dispatch on
   `MemState::classify(p) -> AnyNode` instead of a raw `match
   mem.b16s1(p)` over `*_FNL`/`*_CNL` constants. This **removes all 3 of
   packaging's `from_idx_unchecked`** (the `flush_node_list` GLUE arm now
   binds the typed `NodePtr<GlueNode>` straight from `AnyNode::Glue`, and
   its three noad arms — ORD..ACCENT / LEFT|RIGHT / FRACTION — collapse
   into one `AnyNode::Noad` arm sub-dispatching on `noad_type`).
   Per-walker exhaustiveness preserved: `flush_node_list`'s `AnyNode`
   match is total (no catch-all); `copy_node_list` keeps `_ =>
   confusion("copying")` for the valid-but-uncopied variants
   (noad/style/choice) and `just_copy` keeps `_ => None` (silent skip),
   both reached via `classify().map_err(...)?` / `classify().ok()`. Dead
   `*_FNL`/`*_CNL` node-type consts + now-unused imports removed.
2. `prune_page_top` adopts the same surface (`AnyNode::HList|VList|Rule`,
   `Whatsit|Mark|Ins`, `Glue|Kern|Penalty`, `_ => confusion("pruning")`);
   dead `*_PPT` node-type consts + `HLIST_NODE`/`RULE_NODE` imports
   removed.

**Behaviour-preserving:** every typed arm lowers to the same field
access the raw match performed; `classify`'s `Err` (invalid type code)
maps to the exact catch-all each walker already had.

**Not done (recorded):** `ignorable_protrusion_node_in` (linebreak) and
the math noad helpers are left raw — linebreak's `*_node()` /
`from_idx_unchecked` are the **intentional `*_in`/confinement layer**
(`01c` §6, session-172 note), not classify targets; the math sites are
justified post-dispatch boundaries in helpers that receive an
already-typed node from a caller (`mlist_to_hlist`) that is itself still
unthreaded. The threaded-function `classify`-adoption vein is now
**exhausted** — every remaining `match mem.b16s1` dispatch loop
(`shipout`/`output`/`math.rs` `mlist_to_hlist`) reads through the
`mem_b16s1` **global** accessor, i.e. is unthreaded, so adopting
`classify` there requires threading the host first (bucket C/D).

**Tests:** `cargo check`; `cargo fmt --check`; `node::` **42/42**;
dashboard **29/2/7/4**; release corpus exit 0, only the known FP
divergences `1905.00073v1` / `1905.00148v1` (**no regression**).
Metrics: warnings **43 → 42**; `unsafe fn` **568**; `from_idx_unchecked`
total **126 → 123** (packaging **3 → 0**; node.rs 74, math 34,
linebreak 12, mem 3).

**Next:** the mechanical `classify`/consumer vein is exhausted; the
remaining levers are all **threading** large bridge/global functions so
their dispatch loops can then adopt `classify` in the same fused slice
(`01b` §3a): `mlist_to_hlist` (math), the `shipout`/`output`
node-walkers, `vert_break`/`vsplit`, `make_op_global`/
`make_math_accent_global`, the `hyphenate`/`line_break` tail. No more
pointer-only or raw-dispatch-only passes.

---

## Idiom session 172 — 2026-06-13 — Execution: unified `AnyNode` + `classify`, adopted in hpack/vpackage

**Type:** execution (workstream 01 / `01c` §3 — the single classification
boundary). Prompted by user design question: the big `match NodeType` loops
each did a per-arm `from_idx_unchecked`; can the typed handle be found
automatically?

**Answer + code (2 commits):**
1. Built the unified surface the plan specified but never landed (`01c` §3):
   `AnyNode` (typed `NodePtr<K>` per built family; raw `MemIdx` for the
   not-yet-typed native/whatsit/unset/ins/mark/adjust) + `MemState::classify(p)
   -> Result<AnyNode, InvalidNodeType>`. It confines every built family's
   `from_idx_unchecked` to one audited site; a consumer matches `AnyNode` and
   writes no `unsafe`, with the static type inferred per arm. Noads collapse to
   one `Noad` variant (refine via `noad_type`). Context-disambiguated codes are
   classified by node-list meaning (2→Rule not Delta, 14→Style not Edge).
   **Disasm gate (`01c` §4): a comprehensive typed classify-dispatch compiles
   to a jump table → direct field load, no enum materialized — tighter codegen
   than the equivalent raw multi-arm `NodeType` match (123 vs 217 bytes).** Unit
   test `classify_dispatches_to_typed_handles`.
2. Adopted it in the `hpack`/`vpackage` main loops: `match
   mem.classify(node_idx)` replaces `match NodeType::from_raw(...)` + 11 per-arm
   `from_idx_unchecked`. The Ins|Mark|Adjust arm binds the matched value (`arm
   @ (..)`) to recover the Adjust discriminant. `packaging.rs`
   `from_idx_unchecked` 14→3 (the rest are `flush_node_list`, which dispatches
   on raw `*_FNL` constants — a separate, larger adoption).

**Tests:** `cargo check`; `node::` **42/42**; dashboard **29/2/7/4**; release
corpus exit 0, only known FP divergences `1905.00073v1`/`1905.00148v1` (**no
regression**). Metrics: warnings **43**; `unsafe fn` **568**; `from_idx_unchecked`
total **126** (node.rs **74** = the audited confinement layer + classify +
per-family classifiers + allocs + tests; packaging **3**, math **34**,
linebreak **12** = confined `*_in` helpers, shipout **0**).

**Next clean `classify` adoptions:** `flush_node_list`/`copy_node_list`/
`just_copy` (restructure the raw-`*_FNL`-constant match to `AnyNode`, with the
noad arm sub-dispatching on `noad_type` and the box arm grouping
HList|VList|Unset), then `mlist_to_hlist` (math, 34 sites). linebreak's
`from_idx_unchecked` are the intentional `*_in` confinement helpers, **not**
classify targets.

---

## Idiom session 171 — 2026-06-13 — Execution: land hpack/vpackage thread, drop non-fix unsafe helpers, type more consumers

**Type:** execution (workstream 01 / `01b` §3a). Large low-docs session.

**Code (3 commits):**
1. Validated + cleaned + committed the inherited uncommitted WIP that
   threads `hpack`→`hpack_in` / `vpackage`→`vpackage_in` over explicit
   state, splitting overfull/underfull diagnostics into borrow-dropping
   wrappers (`print_{h,v}pack_diag`/`_common_ending`, `PackDiag`) and
   adopting `classify_box` + `VListNode::alloc`. Removed dead
   `total_stretch`/`total_shrink` setters and the node-type constants the
   threaded packers stopped referencing.
2. **Design fix (user-flagged):** deleted the never-wired
   `AnyBox::from_node_type` / `RuleNode::from_node_type` helpers the WIP
   added. Relocating `from_idx_unchecked` into a per-call helper does NOT
   eliminate the unsafe tag→type assertion; `classify_box`/`classify_*`
   are the single audited boundary that turns a runtime `NODE_type` into a
   typed handle, after which each `match` arm carries the static type with
   zero further unsafe.
3. Typed more box/glue/noad/choice consumers in already-threaded fns:
   packaging `flush_node_list` GLUE arm (close the glue family), math
   `apply_math_limit_switch`, `stack_glue_into_box` (glue spec),
   `build_ext_delimiter_box` (vlist dims), `first_script_glyph_step`
   (choice mlists), `rebox` (classify-once box), shipout
   `finish_{h,v}list_rule` (classify output box).

**Tests:** `cargo check`; `node::` **41/41**; dashboard **29/2/7/4**.
Rebuilt `target/release/tectonic`; `plan/scripts/corpus.sh check all`
exit 0 with only the known FP divergences `1905.00073v1` /
`1905.00148v1` (**no regression**). Metrics: warnings **43**,
`with_state` **1156**, `unsafe fn` **568**, `#[no_mangle]` **11**, bare
`[pqr]: i32` **347** (up from new threaded packer helper params; retires
as callers thread).

**Finding for next session:** the cheap consumer-typing vein in
already-threaded packaging/math/shipout fns is now genuinely thin (this
session + prior ones largely exhausted it). The remaining big levers are
**threading large bridge-based functions**: `vert_break`/`vsplit`
(packaging; the mid-loop "infinite glue shrinkage" print+continue path
blocks a single borrow — bucket D), `make_op_global`/`make_math_accent_global`
(math), `hyphenate`/`line_break` tail (linebreak, 65/47 bridge sites),
and the still-unthreaded shipout/pagebuilder/output/main_control/scan
bodies. Each is a careful bucket-C/D slice, not a mechanical batch.

---

## Idiom session 170 - 2026-06-12 - Execution: typed shipout records + threaded `post_line_break`

**Type:** execution (workstream 01 / `01b` Phase-C threading + typed-node
closure). Larger mechanical slice; docs intentionally minimal.

**Code:** added typed `EdgeNode` and untagged shipout `MovementNode` records in
`node.rs`, then adopted them directly in `shipout.rs` `new_edge`,
`prune_movements`, and `movement`. Split `post_line_break` around the remaining
global `hpack` boundary: the prefix and suffix now thread explicit state and use
typed `DiscNode`, `GlueNode`, `GlueSpecNode`, `KernNode`, `MathNode`,
`PassiveNode`, and `AnyBox` accessors. Removed dead one-line wrappers/accessors
for margin-kern insertion, LR-stack update, `set_just_box`, linebreak
width/indent getters, and `best_line`. The split initially exposed stale
`just_box` state in display math; fixed by restoring `scanner.just_box` before
the suffix calls paths that still read the global bridge.

**Tests:** `cargo check -p tectonic_engine_xetex`; `cargo fmt --check -p
tectonic_engine_xetex`; `cargo test -p tectonic_engine_xetex node::` **41/41**;
dashboard **29/2/7/4** (`tex-outputs`, `trip`, `engine_rust`,
`latex_goldens`); `git diff --check`. Rebuilt `target/release/tectonic`;
corpus returned **637 passed, 2 diverged, 0 missing-golden**, only known
`1905.00073v1` and `1905.00148v1`. Metrics: warnings **43**, `with_state`
**1155**, `unsafe fn` **568**, direct `#[no_mangle]` grep **11**, current bare
`[pqr]: i32` grep **341**.

**Next:** `hpack` is the main remaining boundary in this area; next large slice
should thread and type `hpack`/packaging or another closed dependency. Avoid
pointer-only helper passes.

---

## Idiom session 169 — 2026-06-12 — Execution: inline delta fields + post-line-break entry

**Type:** execution (workstream 01 / `01b` Phase-C threading + typed-node
closure). Low-docs mechanical slice.

**Code:** removed the index-based `delta_node_width_in` /
`set_delta_node_width_in` wrappers and inlined direct `DeltaNode`
`dwidth`/`dstretch*`/`dshrink` getters and setters throughout the already
threaded `try_break` and `deactivate_node` paths. Threaded the first
`post_line_break` island, `reverse_best_breaks`, over `MemState` +
`LinebreakState` and typed it through `ActiveNode`/`PassiveNode`; deleted the
now-dead `globals::best_bet` one-line bridge.

**Tests:** `cargo check -p tectonic_engine_xetex`; `cargo test -p
tectonic_engine_xetex node::` **41/41**; dashboard **29/2/7/4**
(`tex-outputs`, `trip`, `engine_rust`, `latex_goldens`). Rebuilt
`target/release/tectonic`; corpus returned **637 passed, 2 diverged, 0
missing-golden**, only known `1905.00073v1` and `1905.00148v1`. Metrics:
warnings **43**, `with_state` **1162**, `unsafe fn` **571**, direct
`#[no_mangle]` grep **11**, current bare `[pqr]: i32` grep **343**.

**Next:** continue `post_line_break` only in closed slices that can thread
state and type nodes together; `hpack` remains a boundary until it is threaded.

---

## Idiom session 168 — 2026-06-12 — Execution: inline typed linebreak consumers

**Type:** execution (workstream 01 / `01b` Phase-C threading + typed-node
closure). Larger mechanical slice; docs intentionally minimal.

**Code:** continued the existing uncommitted linebreak WIP. Per user direction,
did not grow more one-line field-helper wrappers; kept context-boundary
constructors and inlined typed calls inside threaded bodies. Typed more
already-threaded linebreak/protrusion islands: `total_pw` active/passive/disc
fields, active/delta/passive links in setup/cleanup/best-break/`try_break`
deactivation, margin-kern insertion links via `MarginKernNode`, and
`update_lr_stack` math subtype reads via `MathNode`. Native/whatsit and legacy
`line_break`/`post_line_break` raw islands remain for later closed thread+type
slices.

**Tests:** `cargo check -p tectonic_engine_xetex`; `cargo test -p
tectonic_engine_xetex node::` **41/41**; dashboard **29/2/7/4**
(`tex-outputs`, `trip`, `engine_rust`, `latex_goldens`). Rebuilt
`target/release/tectonic`; corpus returned **637 passed, 2 diverged, 0
missing-golden**, only known `1905.00073v1` and `1905.00148v1`. Metrics:
warnings **43**, `with_state` **1162**, `unsafe fn` **571**, direct
`#[no_mangle]` grep **11**, current bare `[pqr]: i32` grep **277**.

**Next:** thread `post_line_break` only as a closed slice with its
allocation/global dependencies, or continue already-threaded linebreak helper
raw islands. Do not add one-line field-helper wrappers that will be removed
later.

---

## Idiom session 167 — 2026-06-12 — Execution: typed `linebreak` main-loop consumers

**Type:** execution (workstream 01 / `01b` Phase-C threading + typed-node
closure). Larger mechanical slice; docs intentionally minimal.

**Code:** continued from the existing uncommitted linebreak WIP. Added typed
context-boundary helpers for already-classified linebreak/main-list families
(`GlueNode`/`GlueSpecNode`, `KernNode`, `LigatureNode`, `DiscNode`,
`MathNode`, `PenaltyNode`, `RuleNode`, `AnyBox`) and used them immediately in
threaded `try_break` and protrusion helpers. `try_break` now computes
discretionary widths, trailing discard widths, glue-spec stretch/shrink, math
width, kern width/subtype, ligature font/char, and passive/active/delta
allocation setup through typed handles instead of raw `mem[p+n]` fields. Added
typed active/delta list-link accessors in `node.rs`. Kept native/whatsit raw;
that family is not typed yet.

**Tests:** `cargo check -p tectonic_engine_xetex`; `cargo test -p
tectonic_engine_xetex node::` **41/41**; dashboard **29/2/7/4**
(`tex-outputs`, `trip`, `engine_rust`, `latex_goldens`). Rebuilt
`target/release/tectonic`; `CANDIDATE_BIN=... plan/scripts/corpus.sh check all`
returned **637 passed, 2 diverged, 0 missing-golden**, only known
`1905.00073v1` and `1905.00148v1`. Final post-format local rename only changed
helper parameter names; final `cargo check` still green. Metrics: warnings
**43**, `with_state` **1162**, `unsafe fn` **571**, direct `#[no_mangle]`
grep **11**, bare `[pqr]: i32` params **349**.

**Next:** finish the remaining `line_break` main-loop raw islands or move into
`post_line_break`; native/whatsit typing remains a separate family-sized slice.

---

## Idiom session 166 — 2026-06-11 — Execution: thread more `line_break` setup/teardown

**Type:** execution (workstream 01 / `01b` Phase-C threading + typed-node
closure). Larger mechanical slice; docs intentionally minimal.

**Code:** continued upward from `try_break` into `line_break` without adding
pointer-only intermediate helpers. Added threaded, typed helpers for pass setup
(`start_linebreak_pass`), active/passive cleanup, best-break selection, and the
last-line-fit glue adjustment. These use direct `MemState`/`ScannerState`/
`LinebreakState`/`EqtbState`/`ErrorState` fields plus `NodePtr<ActiveNode>`,
`GlueNode`, and `GlueSpecNode`; allocation failures still defer through
`NodeRecycleAbort` so aborts run after the borrow drops. Also typed the
already-threaded `finite_shrink` glue-spec shrink-order write.

**Cleanup:** removed the dead active-node/delta/passive linebreak bridge
wrappers and the now-dead `globals.rs` linebreak accessors exposed by this
slice (`set_first_p`, best setters, passive accessors, active-node-size getter,
`do_last_line_fit`, etc.). Warnings remained at **43**.

**Tests:** `cargo check -p tectonic_engine_xetex`; `cargo test -p
tectonic_engine_xetex node::` **41/41**; dashboard **29/2/7/4**
(`tex-outputs`, `trip`, `engine_rust`, `latex_goldens`). Rebuilt
`target/release/tectonic` after the final tree and ran
`CANDIDATE_BIN=... plan/scripts/corpus.sh check all`: **637 passed, 2
diverged, 0 missing-golden**, only known `1905.00073v1` and `1905.00148v1`.
Metrics: `with_state` **1162**, `unsafe fn` **571**, direct `#[no_mangle]`
grep **11**, bare `[pqr]: i32` params **344**.

**Next:** continue through the remaining `line_break` main-loop islands and
then `post_line_break`, or switch to threading `hpack`/`vpackage`.

---

## Idiom session 165 — 2026-06-11 — Execution: thread `try_break` directly

**Type:** execution (workstream 01 / `01b` Phase-C threading + typed consumer
closure). Large mechanical session; docs intentionally minimal.

**Code:** threaded `linebreak.rs::try_break` to explicit
`&mut MemState`, `&mut ScannerState`, `&mut LinebreakState`,
`&EqtbState`, `&mut FontState`, `&ConfigState`, and `&mut ErrorState`.
Inside the threaded core, active/delta/passive linebreak records now use the
existing typed handles directly (`active(...)`, `delta_node_width_in`,
`passive_node(...)`) and the linebreak/scanner arrays are accessed as state
fields. Cold `get_node` exhaustion, `confusion`, and the `total_pw` pdf-error
path are deferred through a narrow `TryBreakAbort` so aborts run after the
thread-local borrow drops. Legacy `line_break` call sites now enter through
`try_break_global`.

**Cleanup:** deleted the now-dead linebreak bridge wrappers
(`delta_node_width`, passive setters, active break/fitness getters,
`total_pw_global`) and removed dead `globals.rs` bridge accessors for
`try_break`-private fields (`cur_active_width`, `break_width`, best-place/best
line arrays, minimal/minimum/easy/fill/best-pl reads and dead setters).

**Tests:** `cargo check -p tectonic_engine_xetex`; `cargo test -p
tectonic_engine_xetex node::` **41/41**; dashboard **29/2/7/4**
(`tex-outputs`, `trip`, `engine_rust`, `latex_goldens`). Rebuilt
`target/release/tectonic`; `plan/scripts/corpus.sh check all` produced the
expected **637 passed, 2 diverged, 0 missing-golden**, with only known
divergences `1905.00073v1` and `1905.00148v1`. Metrics: warnings 43,
`with_state` 1177, `unsafe fn` 571, direct `#[no_mangle]` grep 11.

**Next:** continue upward into `line_break` setup/teardown so the remaining
active-node setter wrappers can disappear at a real threaded boundary, or move
to `hpack`/`vpackage`.

---

## Idiom session 164 — 2026-06-11 — Execution: typed math/linebreak consumers (larger mechanical batch)

**Type:** execution (workstream 01 / `01b` §3a + `01c` Stage 3). Large
mechanical session per user request; docs intentionally minimal. 2 commits.

**Commits:**
1. `28fcf31d` `engine_xetex: type more node consumers`
   - Committed the inherited `linebreak.rs` active-list typed accessor wrapper
     (`ActiveNode` around the old active helper layer).
   - Typed several `math.rs` consumer islands without entering the native
     whatsit family: `make_op_global`'s LIMITS/math-char mem clusters,
     `compute_ot_math_accent_pos_step`, and box-side native-glyph helpers.
2. `1d7346af` `engine_xetex: type linebreak delta consumers`
   - Added narrow `DeltaNode`/`PassiveNode` bridge helpers and used them in
     `try_break` / `deactivate_node` for delta widths and passive break fields.

**Tests:** `cargo check -p tectonic_engine_xetex`; `node::` **41/41**;
dashboard **29/2/7/4** (`tex-outputs`, `trip`, `engine_rust`,
`latex_goldens`). Rebuilt `target/release/tectonic` and ran
`CANDIDATE_BIN=... plan/scripts/corpus.sh check all`: **637 passed, 2
diverged, 0 missing-golden**. The divergences were only the known
`1905.00073v1` and `1905.00148v1`.

**Notes:** two explorers were useful for target selection: both pointed at
`make_op_global`/linebreak internals and away from `make_math_accent_global` as
a full-threading target. User correctly called out that helper-per-field
wrappers like `active_node_total_demerits_in` are transitional ceremony now that
typed pointers exist; future linebreak work should collapse toward direct typed
handles inside threaded cores instead of growing this layer. Metrics after the
session: warnings 43, `with_state` 1202, `unsafe fn` 571, direct
`#[no_mangle]` grep 11.

**Next:** thread `try_break`/linebreak internals properly and delete the
transitional helper wrappers as raw pointers enter typed cores; or thread
`hpack`/`vpackage`. Avoid adding more legacy bridge wrappers except at real
raw-to-typed boundaries.

---


## Idiom session 163 — 2026-06-10 — Execution: subscr/supscr rename + bulk noad & glue-spec consumer typing (large, low-docs)

**Type:** execution (workstream 01 / `01b` §3a + `01c` Stage 3). Large mechanical
session per user request (less docs, more code). 5 commits.

**Commits:**
1. `e8c364db` **subscr/supscr rename slice** (the session-162 landmine fix).
   Offset-preserving swap of the inverted `NodePtr<Noad>` accessor names: now
   `supscr_*` = +2, `subscr_*` = +3 (matching TeX and this build). Method
   *definitions* (node.rs) and *every call site* (math.rs, mem.rs) were swapped
   together via a `subscr_`↔`supscr_` identifier swap (perl on math.rs/mem.rs,
   hand-rewrite of node.rs methods + addr getters), so each site still hits the
   same word — behaviour-preserving, compiler-checked. Removed the landmine
   comment. This unblocks typing noad consumers by semantic name.
2. `818b95a2` typed noad consumers in **math.rs**: `make_over`,
   `make_under_finish`, `make_vcenter`, `make_fraction_finish` (nucleus
   field/type writes via `NodePtr<Noad>`); `append_script_box`,
   `finish_subsup_stack_global`, `finish_superscript_branch_global`,
   `make_scripts_global`, `make_left_right_global` (nucleus/supscr/subscr reads
   + new_hlist/noad_type). Each function's noad family fully closed.
3. `20c6283b` typed noad consumers in **packaging.rs** `flush_node_list`:
   standard-noad arm (Ord..Accent) + fraction-noad arm (numerator=`supscr_field`
   +2, denominator=`subscr_field` +3). Box arm left raw (also matches
   `UNSET_NODE`, which `classify_box` rejects).
4. `1ff467c7` typed the glue-spec **`ref_count` trap** (the canonical
   "ref_count is a count, NOT a link") at 4 **packaging.rs** sites: `just_copy`
   (GLUE), `copy_node_list` (INS + GLUE), `flush_node_list` (GLUE), via
   `NodePtr<GlueSpecNode>::from_glue_ptr` + `ref_count`/`set_ref_count`.

**Method:** match **offset → node.rs accessor** mechanically; close one family
per function (per-family closure rule). Behaviour-preserving — every typed
accessor lowers to the identical raw field access. **Deliberately left raw**
(recorded, not regressions): the bridge-mixed math hosts `make_op_global`,
`make_math_accent_global`, and the giant `mlist_to_hlist` (they access noad/box
fields via BOTH `mem_b32s1` bridges and `st.mem`, so typing one half breaks the
closure rule — thread them whole first); the box arms of
`flush_node_list`/`copy_node_list` (UNSET_NODE shares the layout but
`classify_box` rejects it); linebreak.rs `*_in` accessor helpers (already a
clean named confinement layer — re-routing through `NodePtr` is redundant).

**Tests:** `node::` 38/38; dashboard **29/2/7/4** (every commit). Corpus
(`plan/scripts/corpus.sh check all`): **637 passed, 2 diverged, 0
missing-golden** — the 2 are the pre-existing known arXiv divergences
(`1905.00073v1`, `1905.00148v1`, tier2 FP/native-text), matches baseline, NOT
regressions. Metrics: `with_state` 1197, `unsafe fn` 571 (unchanged — pure
consumer typing, no new threading), warnings 43.

**On subagents (user asked):** the remaining work is concentrated in single files
(math.rs, packaging.rs) and is parity-sensitive, so parallel subagents would
just conflict on the same file and risk silent offset bugs; serial edits with the
dashboard as the gate were the safer high-throughput path. Cross-file
parallelism isn't available yet because the other high-traffic files aren't
`mem`-threaded (packaging has 0 `mem:&MemState` fns outside the few node-walkers;
linebreak/shipout are mostly bridge/global). A subagent ran the background corpus
check while editing continued.

**Next:** the cheap box/noad/glue *consumer* vein in already-threaded functions
is now largely exhausted. Highest-value next steps are **threading** work, not
consumer typing: (a) thread `make_op_global` / `make_math_accent_global`
end-to-end (convert their `mem_b32s1` bridges to `st.mem`/typed) so their
box+noad families can be closed; (b) build a typed family for the rule-11
page/linebreak internals (active/passive/movement/mark/ins) or the native/whatsit
family (rank 9) and adopt consumers in shipout/copy_node_list; (c) thread
`hpack`/`vpackage` (the canonical box-dimension consumers, currently on bridges).

---

## Idiom session 162 — 2026-06-10 — Execution: bulk box-dimension consumer typing in math.rs (large mechanical batch)

**Type:** execution (workstream 01 / `01b` §3a + `01c` Stage 3). Intentionally
large, low-docs session per user request — typed-consumer adoption only (no new
node families). Two commits.

**Code (10 functions, ~21 raw box accessors retyped; math.rs 145→124
method-style accessors):**
- Commit 1 (`2ff59e2f`): `add_script_space_and_restore_font`,
  `initial_script_shifts`, `finish_subsup_stack`, `finish_subscript_only`,
  `prepare_superscript`, `finish_superscript_branch`, `make_under_finish`,
  `make_vcenter` — raw `mem.b32s1(box+N)` → typed `AnyBox`
  width/depth/height/shift_amount/list_ptr/link via `classify_box`. Dropped the
  now-unused `AnyBox` import (warnings 44→43).
- Commit 2 (`adbd33f9`): `char_box` (the built box `b`), `make_fraction` (vbox
  `v` + numerator `x` / denominator `z`).

**Method:** match **offset → node.rs accessor** mechanically (node.rs is the
test-validated offset authority). Each function's **box family is fully closed**
(per-family closure rule, `01b` §3a); other families in the same body
(noad `q+N`, native-glyph `p`, kern/rule links) left raw — separate slices.
Behaviour-preserving: every typed accessor lowers to the identical raw field
access.

**Skipped (recorded, not regressions):** `make_op_global` is only *partially*
threaded (mixes `set_mem_b32s1` bridge calls and `st.mem.` closures on the same
box `x`) — typing only the closure half would violate the closure rule. Thread
the whole function first. `make_math_accent` box reads (`x+1`/`x+3`) likewise
sit in a large body with other raw box accesses; defer to a whole-function
slice.

**⚠ FINDING — node.rs subscr/supscr names are inverted vs TeX (offset-correct,
NAME-wrong):** `NodePtr<Noad>::subscr_*` reads **offset +2**, `supscr_*` reads
**offset +3**. But TeX (and this build, per `prepare_subscript_global(q + 3)`)
has **supscr = +2, subscr = +3**. The methods are used consistently by offset
(dashboard green), so **no behaviour bug ships** — but the names are a landmine:
a future consumer that calls `subscr_type` expecting the subscript gets the
*superscript* word. This is why this session left all noad `q+2`/`q+3` reads
**raw** rather than adopt the mislabelled accessors. **Recommended next:** a
dedicated, atomic rename slice that swaps the `subscr_*`↔`supscr_*` method names
*and* every call site name together (net offset unchanged at every site →
behaviour-preserving; compiler-checked; gate on the dashboard). Until then,
match noad fields by offset, not by these names.

**Tests:** `node::` 38/38; dashboard **29/2/7/4** (both commits). Corpus
(`plan/scripts/corpus.sh check all`): **637 passed, 2 diverged, 0
missing-golden** — the 2 are the pre-existing known arXiv divergences
(`1905.00073v1`, `1905.00148v1`, tier2 FP/native-text), NOT regressions; matches
baseline. Metrics: `with_state`
1198, `unsafe fn` 571 (unchanged — pure consumer typing, no new threading).

**Next:** (a) the subscr/supscr rename slice (above) to unblock safe noad
typing; then (b) more box-dimension consumers once their host functions are
fully threaded (`make_op_global`, `make_math_accent`); or (c) thread
`make_op_global` end-to-end so its box `x` can be typed.

---

## Idiom session 161 — 2026-06-08 — Execution: MarginKernNode implementation and `attach_hkern_to_new_hlist` refactoring

**Type:** execution (workstream 01 / `01b` §3a + `01c` Stage 3). MarginKernNode & Kern attachment.

**Code:** Added `MARGIN_KERN_NODE` (type 40) and `MARGIN_KERN_NODE_SIZE` (3) constants to `src/consts.rs`. Defined `MarginKernNode` marker, its `alloc`/`width`/`set_width` implementations, `HasNodeHeader` implementation, and `MemState::classify_margin_kern` in `src/node.rs`. Added address getters (`nucleus_addr`, `subscr_addr`, `supscr_addr`) to `NodePtr<Noad>`. Refactored `new_margin_kern` in `src/mem.rs` to use `MarginKernNode` and typed accessors. Refactored `attach_hkern_to_new_hlist` and `attach_hkern_to_new_hlist_global` in `src/math.rs` to adopt typed node pointers.

**Tests:** `node::` **38/38** unit tests pass. Dashboard green: `tex-outputs` **29/29**, `trip` **2/2**, `engine_rust` **7/7**, `latex_goldens` **4/4**. Warnings went from 43 to **44**.

**Next:** implement safe `maybe_attach_script_kern` core.

---

## Idiom session 160 — 2026-06-08 — Execution: typed math list and radical consumers adoption

**Type:** execution (workstream 01 / `01b` §3a + `01c` Stage 3). Typed math list and radical consumers.

**Code:** Added `noad_type` getter to `NodePtr<Noad>`. Added delegation methods `set_width`, `set_depth`, `set_height`, `set_shift_amount`, `list_ptr`, `set_list_ptr`, `link`, and `set_link` to `AnyBox`. Refactored `fin_mlist` in `src/math.rs` to use typed `NodePtr<Noad>` instead of raw macro reads and writes. Refactored `build_choices_finish` in `src/math.rs` to use typed `NodePtr<ChoiceNode>`. Refactored `overbar`, `make_radical_finish`, and `make_radical_prepare_delimiter` in `src/math.rs` to use typed `NodePtr<KernNode>`, `NodePtr<RuleNode>`, `NodePtr<Noad>`, and `AnyBox` accessors.

**Tests:** `node::` **34/34** unit tests pass. Dashboard green: `tex-outputs` **29/29**, `trip` **2/2**, `engine_rust` **7/7**, `latex_goldens` **4/4**. Warnings went from 42 to **43**.

**Next:** continue implementing typed consumers in `math.rs` or `packaging.rs` (e.g. dimension-reading leaves) or remaining raw constructor families.

---

## Idiom session 159 — 2026-06-08 — Execution: typed glue spec constructors and consumers adoption

**Type:** execution (workstream 01 / `01b` §3a + `01c` Stage 3). Typed glue spec.

**Code:** refactored `new_spec`, `new_skip_param`, and `delete_glue_ref` in `src/mem.rs` to use typed getters and setters from `NodePtr<GlueSpecNode>` instead of raw macro reads and writes. Also refactored the legacy bridges `math_glue_global` and `math_kern_global` in `src/math.rs` to adopt the typed interfaces for `GlueSpecNode` and `KernNode` respectively, ensuring that the `scaled_mu_value` side-effect calls (which mutably borrow state) occur outside of the state-borrowing closures to avoid `RefCell already borrowed` panics. Cleaned up unused imports `SMALL_NODE_SIZE` from `src/mem.rs` and `get_node` from `src/math.rs`.

**Tests:** `node::` **34/34** unit tests pass. Dashboard green: `tex-outputs` **29/29**, `trip` **2/2**, `engine_rust` **7/7**, `latex_goldens` **4/4**. Warnings dropped from 44 to **42**.

**Next:** continue implementing typed consumers in `math.rs` or `packaging.rs` (e.g. dimension-reading leaves) or remaining raw constructor families.

---


## Idiom session 158 — 2026-06-08 — Execution: typed math noad consumers adoption

**Type:** execution (workstream 01 / `01b` §3a + `01c` Stage 3). Typed math noad consumers.

**Code:** added generic `link` and `set_link` getters/setters on `NodePtr<K>` where `K: HasNodeHeader` to traverse node lists in a typed manner. Added character and family getters and setters on `NodePtr<Noad>` for the math fields (nucleus, subscript, superscript) and the accent character field. Refactored the core ligature/kern math processor `make_ord_body` and its wrapper `make_ord_global` in `src/math.rs` to adopt the typed `NodePtr<Noad>` interfaces. Also refactored `finish_accent_math_field` to use typed getters and setters. Extended the `noad` unit test in `src/node.rs` to round-trip and verify all the new getters and setters.

**Tests:** `node::` **34/34** unit tests pass. Dashboard green: `tex-outputs` **29/29**, `trip` **2/2**, `engine_rust` **7/7**, `latex_goldens` **4/4**. Warnings stayed at **44**.

**Next:** continue implementing typed consumers in `math.rs` or `packaging.rs` (e.g. dimension-reading leaves) now that noad field getters/setters are available.

---


## Idiom session 157 — 2026-06-08 — Execution: typed noad constructor adoption

**Type:** execution (workstream 01 / `01b` §3a + `01c` Stage 3). Constructor typing slice.

**Code:** added unified `Noad` typed surface with multiple type-stamping allocators (for standard noads of size 4, radical/accent variants of size 5, and fraction variants of size 6). Added getters and setters for math fields (nucleus, subscr, supscr), fraction thickness, and accent character. Also added helpers to get the sub-element delimiter addresses. Adopted the surface in `new_noad`. Removed unused `NOAD_SIZE` and `ORD_NOAD` constants from `src/mem.rs`.

**Tests:** `node::` **34/34** unit tests pass. Dashboard green: `tex-outputs` **29/29**, `trip` **2/2**, `engine_rust` **7/7**, `latex_goldens` **4/4**. Warnings stayed at **42**.

**Next:** either type `new_lig_item` as a non-header lig-stack record, or begin implementing typed consumers of these typed surfaces in `math.rs`/`packaging.rs`.

---


## Idiom session 156 — 2026-06-08 — Execution: typed style/choice constructor adoption

**Type:** execution (workstream 01 / `01b` §3a + `01c` Stage 3). Constructor
typing slice.

**Code:** added `StyleNode` and `ChoiceNode` typed surfaces with type-stamping
allocators. `ChoiceNode` exposes the four nullable math-list alternatives:
display/text/script/scriptscript. Adopted the surfaces in `new_style` and
`new_choice`. Deliberately skipped `new_lig_item` for now because it is a
lig-stack record with no `NODE_type` header, not a normal list node.

**Tests:** `node::` **32/32**. Dashboard green: `tex-outputs` **29/29**,
`trip` **2/2**, `engine_rust` **7/7**, `latex_goldens` **4/4**. Warnings stayed
**42**.

**Next:** either type `new_lig_item` as a non-header lig-stack record, or start
the noad constructor surface (`new_noad`). Typed consumers in math/packaging
also remain good targets.

---


## Idiom session 155 — 2026-06-08 — Execution: typed glue/math constructor adoption

**Type:** execution (workstream 01 / `01b` §3a + `01c` Stage 3). Longer
typed-pointer constructor session, intentionally light docs.

**Code:** added `NodePtr::<GlueNode>::alloc`, preserving this XeTeX build's
`MEDIUM_NODE_SIZE` glue allocation because SyncTeX uses the extra word. Adopted
it in `new_glue` and `new_param_glue`; both now fill `subtype`/`leader_ptr`/
`glue_ptr` through typed setters and bump the target `GlueSpecNode` refcount
through typed accessors. Added the narrow `MathNode` type for `MATH_NODE`
width/subtype and adopted it in `new_math`. Noads remain untouched.

**Tests:** `node::` **28/28**. Dashboard green: `tex-outputs` **29/29**,
`trip` **2/2**, `engine_rust` **7/7**, `latex_goldens` **4/4**. Warnings stayed
**42**.

**Next:** remaining raw constructor families: `new_lig_item` if it deserves a
narrow typed surface, then noad/style/choice. Also good: typed consumers in
packaging/math/shipout now that the common list-node constructors are typed.

---


## Idiom session 154 — 2026-06-08 — Execution: typed rule/ligature/disc allocation + `append_to_vlist` box typing

**Type:** execution (workstream 01 / `01b` §3a + `01c` Stage 3). One larger
typed-pointer session.

**Code:** added `RuleDimen` (`Running` vs `Set(Scaled)`) and `RuleNode`, then
typed `new_rule` and `fraction_rule`. Added `LigatureNode` and typed
`new_ligature`. Added `DiscNode` and typed `new_disc`. Added `AnyBox` dimension
helpers and retyped `append_to_vlist` so the legacy wrapper classifies `i32` to
`AnyBox` once at the boundary.

**Tests:** `node::` **25/25**. Dashboard green: `tex-outputs` **29/29**,
`trip` **2/2**, `engine_rust` **7/7**, `latex_goldens` **4/4**. Warnings stayed
**42**. Metrics unchanged: `with_state` 1197, `unsafe fn` 571, bare `p/q/r:
i32` params 379, direct `#[no_mangle]` grep 11.

**Next:** continue typed consumers now that rule/disc/ligature exist. Good
targets: more already-threaded packaging/math leaves that read box/rule dims,
or constructor families still raw (`new_math`, `new_glue`, lig-item/noad).

---


## Idiom session 153 — 2026-06-08 — Execution: box-family typed alloc adoption for `new_null_box`

**Type:** execution (workstream 01 / `01b` §3a + `01c` §8), the session-152
"Next" item: adopt the typed `alloc` pattern in **`new_null_box`**. **1 code
slice.** Dashboard **29/2/7/4**; warnings **42 → 42**; `node::` **18 → 19**.

### What changed

- Added a shared list-node header capability in `src/node.rs`:
  **`HasNodeHeader`** + `NodePtr<K>::subtype`/`set_subtype` for node families
  that really carry a `NODE_type`/subtype header. Implemented it for
  `HListNode`, `VListNode`, `GlueNode`, `KernNode`, and `PenaltyNode`.
  **Deliberately not implemented for `GlueSpecNode`** because its word 0 is
  `ref_count`/stretch-order/shrink-order, not a list-node header. This
  resolves session 152's "box needs `set_subtype`" handoff without exposing a
  false subtype on glue specs.
- Deduped the per-family subtype methods from `GlueNode`, `KernNode`, and
  `PenaltyNode`; existing call sites still use the same API, now through the
  shared header impl.
- Added **`NodePtr::<HListNode>::alloc(mem) -> Option<Self>`**: wraps
  `get_node(BOX_NODE_SIZE)`, writes `NODE_type = HLIST_NODE`, and confines the
  family construction to one audited `from_idx_unchecked` site. This is the
  box-family type-stamping allocator, still using TeX's free-list arena.
- Rewrote **`new_null_box`** (`src/mem.rs`) to allocate via
  `NodePtr::<HListNode>::alloc` and fill the box through typed setters:
  subtype, dimensions, nullable list pointer, glue sign/order/set. The legacy
  public surface still returns `i32`; `ALLOC_FAILED` propagation and the
  `_global` overflow wrapper are unchanged.

### Tests + validation

- Added `alloc_hlist_type_stamps_and_carves_a_usable_handle`, which builds a
  minimal free-list block, allocates an hlist, asserts it classifies back as
  `AnyBox::HList`, and verifies writable typed fields.
- Ran:
  - `cargo test -p tectonic_engine_xetex node:: --lib` → **19/19**
  - `cargo test -p tectonic --test tex-outputs` → **29/29**
  - `cargo test -p tectonic --test trip` → **2/2**
  - `cargo test -p tectonic --test engine_rust` → **7/7**
  - `cargo test -p tectonic --test latex_goldens` → **4/4**

### Metrics / handoff

Behaviour-preserving: each typed setter lowers to the same raw field write.
Metrics by the existing grep commands: `with_state` **1197**, `unsafe fn`
**571**, bare-`i32` node-ptr params **379**, `#[no_mangle]` direct grep **11**
(unchanged by this slice). New audited `unsafe` construction site: +1
(`HListNode::alloc`), matching the existing per-family `alloc` pattern.

**Next:** rank-5 **rule** remains the next family-build slice, but it needs a
running-dimension wrapper for `Scaled`-or-`NULL_FLAG` before `new_rule` can be
typed. The biggest live Phase-C typing win is still box-dimension-reading
leaves in `packaging.rs`/`shipout.rs`; `append_to_vlist` is already threaded
and wants `AnyBox` dimension helpers because its `b` can be hlist or vlist.

---


## Idiom session 152 — 2026-06-07 — Execution: extend the typed alloc pattern to kern + penalty — the FIRST type-stamping `alloc`s

**Type:** execution (workstream 01 / `01b` §3a + `01c` §8), the session-151
"Next" item: *extend the typed `alloc` pattern to kern/penalty/box when a
real allocating consumer threads*. **2 commits** (one family each, A.12).
Dashboard **29/2/7/4**; warnings **42 → 42**; `node::` **16 → 18**.

### What this session adds (and why it is new vs. 151)

Session 151 built the **first** typed allocator, `NodePtr::<GlueSpecNode>::
alloc`, but a glue_spec carries **no `NODE_type`**, so its `alloc` stamps
nothing. This session lands the **first type-stamping `alloc`s** — the
documented-but-never-exercised half of the `01c` §8 pattern ("a type-stamping
family's `alloc` additionally `set_b16s1(NODE_type)` before returning"):

- **`NodePtr::<KernNode>::alloc(mem) -> Option<Self>`** (`src/node.rs`):
  `get_node(MEDIUM_NODE_SIZE)`, **writes `NODE_type = KERN_NODE`**, then
  constructs the handle through the family's single audited
  `from_idx_unchecked`. `None` = the §5 arena-exhaustion sentinel.
- **`NodePtr::<PenaltyNode>::alloc`** (same shape, stamps `PENALTY_NODE`) +
  a small **`PenaltyNode::set_subtype`** (the shared-header b16.s0 half the C
  `new_penalty` zeroes; penalty has no meaningful subtype, doc'd as such).

### The two real consumers, now fully typed + `unsafe`-free

Both constructors were **already** threaded to `&mut MemState`, so this is
pure ptr/typing adoption (no threading work — the same shape as 151's
`math_glue`):

- **`new_kern`** (`src/mem.rs`): `let Some(p) =
  NodePtr::<KernNode>::alloc(mem) else { return ALLOC_FAILED };
  p.set_subtype(mem, NORMAL); p.set_width(mem, Scaled(w)); p.idx().0`.
  Fully typed, zero raw `mem.set_*` indexing, **`unsafe`-free** (it already
  was, but the hand-rolled `get_node` + raw `set_b16s1(KERN_NODE)` /
  `set_b16s0` / `set_b32s1(p+1)` are gone — confined into `alloc`).
- **`new_penalty`** (`src/mem.rs`): the analogous rewrite via
  `PenaltyNode::alloc` + `set_subtype(0)` + `set_penalty(m)`.

`KERN_NODE`/`PENALTY_NODE` imports **net-moved** from `mem.rs` into
`node.rs`'s `alloc`s (the type code now lives at the one stamping site);
`MEDIUM_NODE_SIZE` stays in `mem.rs` (other constructors still call
`get_node` directly).

### Tests + validation

Two new `node::` unit tests (16 → 18), each building a minimal single-free-
block arena and asserting (a) `alloc` carves a usable, writable handle and
(b) it **classifies back** (`classify_kern`/`classify_penalty` → `Some`),
which directly verifies the type-stamp the `glue_spec` test could not. Both
commits validated on the full dashboard (**29/2/7/4**) + the lib tests.

### Metrics

Behaviour-preserving (each typed setter/alloc lowers to the identical
`get_node` + `set_b16s*`/`set_b32s1`): `with_state` **1197**, `unsafe fn`
**571**, `#[no_mangle]` unchanged, bare-`i32` node-ptr params **379** (the
constructors still *return* `i32`, so no param metric moves this slice — the
win is the typed surface + the confined `from_idx_unchecked`). Net new
`unsafe` blocks: +2 (the two `alloc` audited sites), but they are the
single confined construction site per family — the pattern, not a leak.

### Lesson / handoff

The type-stamping `alloc` is now proven across two families. Per the `01c` §8
A.6 discipline, **do NOT** generalise to one `alloc<K>()` yet — but kern +
penalty + glue_spec means **three** families now share the shape, so the
generalisation question is getting closer (the blocker is that a single
`alloc<K>` needs a per-`K` "size + type-code" trait, which is the A.6
"≥2 real consumers" bar — revisit when box/rule adopt it). **Next:**
- **`new_null_box`** (box family) is the natural next `alloc` adoption, but
  it needs a box-family `set_subtype` (the box methods are trait-gated;
  `subtype` is the rank-0 *shared header* field — consider adding a single
  universal `subtype`/`set_subtype` on `NodePtr<K>` for **all** `K` and
  deduping the per-family copies on GlueNode/KernNode/PenaltyNode, but that
  is its own slice because a blanket `impl<K: NodeKind>` collides with the
  existing inherent per-family methods → method-resolution error unless they
  are removed together).
- **rank-5 rule family** (`new_rule` is its waiting consumer) — still the
  first sentinel-bearing family; needs the running-dimension wrapper
  (`Scaled`-or-`NULL_FLAG`) before `width`/`depth`/`height` can be typed.
- box-dimension-reading **leaves** in `packaging.rs`/`shipout.rs`
  (`append_to_vlist` is fully threaded; `b` is hlist-or-vlist → needs
  `AnyBox` dim helpers) remain the biggest §8 *param*-metric win.

---

## Idiom session 151 — 2026-06-07 — Execution: typesafe node allocation (`NodePtr::<GlueSpecNode>::alloc`) — confine `from_idx_unchecked`, remove the `math_glue` unsafe

**Type:** execution (workstream 01 / `01b` §3a + `01c` §8), prompted by a
design question: *"in `math_glue` you do an unsafe type assertion —
shouldn't there be a typesafe way to allocate a node of the correct type?
Isn't that a common usecase? What does the plan say?"* **1 commit.**
Dashboard **29/2/7/4**; warnings **42 → 42**; `node::` **16/16**.

### What the plan says (and the answer)

The plan **supports** a typed allocator and is explicit that the free-list
arena stays (`01c` §8 NON-GOAL: no per-node heap alloc; `03` perf rules).
The `unsafe { from_idx_unchecked }` at the `math_glue` call site was a
*symptom* — `math_glue` was hand-rolling the `get_node` + handle
construction that belongs in one audited place. `from_idx_unchecked` is
"unsafe" only as a **parity**-discipline marker (its own docs: not
memory-unsafe), so the fix is to **centralise** it in the allocator, not to
sprinkle it at call sites.

### The change

- **`NodePtr::<GlueSpecNode>::alloc(mem: &mut MemState) -> Option<Self>`**
  (`src/node.rs`): wraps the existing `get_node(GLUE_SPEC_SIZE)`, maps the
  `ALLOC_FAILED` sentinel to `None` (the §5 arena-exhaustion signal, handled
  by the caller after the borrow drops), and constructs the handle via the
  family's **single audited** `from_idx_unchecked`. Doc records the design
  rules: a node *being initialised* has no `NODE_type` to classify (so
  `from_idx_unchecked`, not `classify_*`, is correct here); a type-stamping
  family's `alloc` additionally `set_b16s1(NODE_type)` before returning
  (glue_spec carries no type → stamps nothing); it is the typed lift of
  TeX's `new_*` constructors, **not** a new allocator (free-list arena
  unchanged); do **not** generalise to one `alloc<K>()` until ≥2 families
  share it with real consumers (A.6). `01c` §8 + `01b` §3a updated.
- **`math_glue`** (`src/math.rs`) now: `let spec =
  NodePtr::<GlueSpecNode>::alloc(mem).ok_or(NodeRecycleAbort::Overflow)?;`
  then the five typed setters — **`unsafe`-free**. Removed the now-unused
  `GLUE_SPEC_SIZE` import. (The one remaining `from_idx_unchecked` in
  `math.rs` is the *justified* kern-classification boundary in
  `math_kern_global`, gated by its `KERN_NODE` match-arm caller.)
- **Unit test** (16th in `node::`): builds a minimal single-free-block arena
  (rover → self, `MAX_HALFWORD` free marker) and asserts `alloc` carves a
  usable, in-bounds, writable glue_spec handle — isolated coverage of the
  allocator wiring (`GLUE_SPEC_SIZE` + null check + construction), since
  `mem.rs` has no unit tests and otherwise relies on the parity suites.

### Why `Option`, not `Result<_, NodeRecycleAbort>`

`alloc` returns the most primitive, decoupled form — `Option<Self>`, the
typed lift of `get_node`'s `ALLOC_FAILED` — so `node.rs` does not depend on
`scan.rs`'s abort enum, and each caller chooses its abort policy
(`.ok_or(NodeRecycleAbort::Overflow)?` here).

### The companion question (subtype-as-enum) — deferred by design

Same turn asked whether `subtype` should be an enum instead of `u16` +
consts. Recorded the answer (not implemented): the plan mandates a checked
enum for the node *type* field and a second classification layer only for
subtypes that change *field layout* (whatsits). Plain tag subtypes stay
`u16` because **several are used arithmetically** — verified: glue node
`subtype = skip_param + 1` (`mem.rs:540,620`), recovered by subtraction — so
a blanket subtype enum is unfaithful (A.7) and lossy. A per-family tag enum
(`subtype_kind()`) on top of the raw accessor is fine **only** where the
subtype is a verified closed tag with no arithmetic use (kern qualifies;
glue does not). Opt-in polish, not a foundation requirement.

### Metrics & validation

- Behavior-preserving: `alloc` lowers to the identical `get_node` +
  index the old code emitted; `math_glue`'s writes are unchanged. Full
  dashboard green (commit touches the live `mlist_to_hlist` glue path).
- The raw `unsafe {}` block net-**moved** from `math.rs` into the one
  `node.rs` allocator site (confined, now reusable). `unsafe fn` /
  `#[no_mangle]` / `with_state` counts unchanged. `node::` 16/16.

**Handoff / Next:** unchanged from session 150 (more fused adoptions —
box-dimension readers in `packaging.rs`/`shipout.rs` are the biggest `01b`
§8 metric win; or build rank-5 **rule** with its running-dimension wrapper).
Plus: **extend the typed `alloc` pattern** to kern/penalty/box (each
stamping its `NODE_type`) the next time a real *allocating* consumer of
those families threads — do not add them speculatively (A.6).

## Idiom session 150 — 2026-06-07 — Execution: the FIRST fused `01b` §3a threading+typing adoptions (`math_kern`, `math_glue`) + glue_spec family fix

**Type:** execution (workstream 01 / `01b` §3a — the first adoptions of the
`01c` typed layer on live call sites). **3 commits.** Dashboard **29/2/7/4**
green on every commit; warnings **42 → 42**; `node::` **15/15**.

This is the milestone the foundation (sessions 145–149) was built for: the
typed `NodePtr<K>` layer now has **live consumers**, proving the
type-as-you-thread loop end-to-end — not just unit tests + the disasm gate.

### Commit 1 — `math_kern` → `NodePtr<KernNode>` (`f2e56c32`)

`math_kern` (`math.rs`) was already threaded to `&mut MemState` and is
reached only from `mlist_to_hlist`'s first-pass `KERN_NODE` match arm. The
fused touch:
- signature `math_kern(mem, p: i32, scaled_width: Option<i32>)` →
  `(mem, p: NodePtr<KernNode>, scaled_width: Option<Scaled>)`;
- body's two raw writes `mem.set_b32s1(p+1, w)` / `mem.set_b16s0(p, EXPLICIT)`
  → `p.set_width(mem, width)` / `p.set_subtype(mem, EXPLICIT)`;
- **classification boundary** = the `math_kern_global` bridge wrapper: its
  sole caller is the `KERN_NODE` arm (dispatches on `mem_b16s1(q) ==
  KERN_NODE`), so the node is already classified — `unsafe {
  NodePtr::<KernNode>::from_idx_unchecked(MemIdx::new(p)) }` with a SAFETY
  note citing that arm. The `Option<i32>` → `Option<Scaled>` conversion
  (`Scaled(scaled_mu_value(...))`) also lives in the wrapper.

`math.rs` is the first module to `use crate::node`/`crate::units`.

### Commit 2 — complete the glue_spec family (`2e7571a7`)

**Recon during commit 1/3 found the glue_spec family was missing its `width`
field** (+1.b32.s1). The session-147 field map (and thus the session-148
build) omitted it because `width` shares the generic `BOX_width` +1.b32.s1
offset — there is **no dedicated `GLUE_SPEC_width` C macro** — yet
`new_spec` (`mem.rs`) and `math_glue` (`math.rs`) read/write `mem[p+1]` as
the spec width. Added `NodePtr::<GlueSpecNode>::width`/`set_width` + extended
the round-trip test to cover +1. No new disasm run: the +1.b32.s1 `Scaled`
shape was already ICF-proven by box/kern `width`. Corrected the `01c` §1
table row + the family doc comment. (Foundation fix; `node.rs` is
`#[allow(dead_code)]` so it is behavior-neutral, but the full dashboard was
still run.)

### Commit 3 — `math_glue` → `NodePtr<GlueSpecNode>` (`64b80f1e`)

`math_glue` (`math.rs`, already `&mut MemState`-threaded) allocates a fresh
glue_spec via `get_node(GLUE_SPEC_SIZE)` and fills five fields. The fused
touch wraps the fresh index as `NodePtr<GlueSpecNode>` and routes all five
writes through typed setters (`set_width`/`set_stretch_order`/`set_stretch`/
`set_shrink_order`/`set_shrink`, each value `Scaled(...)`-wrapped where
scaled), returning `Ok(spec.idx().0)`. **Boundary lesson:** a *freshly
allocated* node has no `NODE_type` to classify yet, so this is the
"construct a handle to a node I am initializing" case — `from_idx_unchecked`
is the correct tool, **not** `classify_*`/`from_glue_ptr`. The closure rule
holds: every glue_spec access in the body goes through the one handle (no
mixed typed/raw indexing).

### Metrics & validation

- **Both adoptions are behavior-preserving**: each typed setter lowers to
  the identical `set_b32s1`/`set_b16s*` the raw code emitted (the Scaled
  wrap is value-identity). Verified by the full dashboard on each live-path
  commit (commits 1 + 3 touch `mlist_to_hlist`).
- **`01b` §8 metric:** bare-`i32` `[pqr]: i32` params (`grep -rhoE '\b[pqr]:
  i32\b'`) **380 → 379** — `math_kern`'s `p` retyped; the trend is now
  moving down, as intended. (`math_glue`'s spec is a local, not a `[pqr]`
  param, so it does not register on this proxy metric — but it removes 5 raw
  field pokes.)
- Warnings **42 → 42** (no unused imports — both `node`/`units` imports are
  consumed). `with_state`/`unsafe fn`/`#[no_mangle]` unchanged. `node::`
  15/15, `units::` 6/6.

**Handoff / Next:** the loop is proven; keep driving it. Two complementary
tracks:
1. **More fused adoptions.** The cheapest are functions **already** threaded
   to `&mut MemState` whose node-ptr param/local is a *single* known family.
   Grep `math.rs`/`packaging.rs`/`shipout.rs`/`linebreak.rs` for
   `fn …(mem: &(mut )?MemState, …, [pqr]: i32, …)` bodies that read/write
   one family's fields via raw `mem.bXXsY(p+N)`, and retype + swap in one
   commit. Box-dimension readers (the highest-traffic family) are the
   biggest metric win. Candidates seen this session: `math.rs`
   `stack_into_box`/`make_*_finish` (box `b`/`hbox` params),
   `append_script_box`. Always confirm the param is a *single* kind (a body
   that switches on `node_type` and handles several families can't take one
   `NodePtr<K>` — leave it raw or classify per-arm).
2. **Build rank-5 rule** (`01c` §1) — the first **sentinel-bearing** family:
   `width`/`depth`/`height` (+1/+2/+3.b32.s1) are `Scaled`-**or**-`NULL_FLAG`
   running dimensions (`NULL_FLAG = -0x40000000`, `xetex-constants.h:16`).
   Introduce a running-dimension wrapper (e.g. `enum RuleDimen { Running,
   Set(Scaled) }`) before typing the fields; mirror the exact `NULL_FLAG`
   compare; unit-test both running and set cases. Verify offsets against the
   `RULE_NODE_*` macros + `new_rule`. After rule: disc → ligature → math →
   native.
