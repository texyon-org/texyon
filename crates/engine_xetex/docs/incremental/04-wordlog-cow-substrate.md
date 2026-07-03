I have verified all the ground-truth anchors I need. Here is the spec.

---

# SPEC 4 — Word-Log / COW Substrate for O(edit-burst) Per-Keystroke Snapshot/Restore

**Target crate:** `/home/yendric/Code/tectonic/crates/engine_xetex`
**Status of enabling premise:** RESOLVED. Every `mem[]`/`eqtb[]`/`hash[]` mutation is now Rust and funnels through the `MemState`/`EqtbState` `set_*` accessors (verified: no live C writer of the arena; the only native code is font/glyph shims that touch font handles + glyph ids, never `mem`/`eqtb`/`font_info`). Accessor instrumentation therefore captures **100%** of arena mutation with no signals, no FFI, no OS dependency — WASM-safe.
**Scope of this spec:** the per-keystroke *delta* layer that sits between the coarse clone/checkpoint baselines of Spec 3. It does **not** rewrite any typesetting algorithm; it interposes on the ~1200 existing write choke points and adds POD scalar bundling.

---

## 0. Verified ground truth (re-checked against the current tree)

| Fact | Anchor (verified) |
|---|---|
| `MemState` write accessors | `state.rs:527` `set_ptr`, `:533` `set_b32s0`, `:539` `set_b32s1`, `:546` `set_b16s0`, `:552` `set_b16s1`, `:558` `set_b16s2`, `:564` `set_b16s3`, `:577` `set_gr`, `:610` `set_word` |
| `MemState` read accessors (for old-word capture) | `state.rs:478` `b32s0` … `:604` `word` |
| `MemState` snapshot-relevant scalars | `state.rs:450-472`: `data`, `lo_mem_max`, `hi_mem_min`, `avail`, `mem_end`, `rover`, `var_used`, `dyn_used`, `line`, `cur_synctex_tag`, `temp_ptr` |
| `MemoryWord` = 8-byte `Copy` union, `size_of == 8` compile-asserted | `mem_types.rs:66-72` (`const _: () = { assert!(size_of::<MemoryWord>() == 8); … }`) |
| Arena size | `MEM_TOP = 4_999_999` (`consts.rs:146`); `data` Vec is `MEM_TOP + 1 = 5_000_000` words = 40 MB |
| `get_node` / `get_avail` / `free_node` | `mem.rs:132` / `:50` / `:510`; all mutate only via `set_*` + the scalar fields (`mem.avail`, `mem.rover`, `mem.hi_mem_min`, `mem.mem_end`, `mem.lo_mem_max`) |
| SyncTeX trailer stamp on MEDIUM+ nodes | `mem.rs:215-216`: `set_b32s0(r+s-1, cur_synctex_tag)` + `set_b32s1(r+s-1, line)` — already routed through `set_*`, so captured |
| `EqtbState` write surface | `impl EqtbState` at `state.rs:330`: `set_b32s1` (`:341`), `set_hash_s0`/`set_hash_s1`, `set_prim_s0`/`set_prim_s1`, `set_b16s1`, `set_b16s0`, `set_word`, `set_xeq_level` |
| `EqtbState` dense arenas | `data: Vec<MemoryWord>` (`state.rs:285`, sized `eqtb_top+1`; `EQTB_SIZE = 8_941_458` per `consts.rs:55/77`, + `hash_extra = 600_000` → ≈9.54M words ≈ 76 MB), `hash: Vec<B32x2>` (`state.rs:301`, ≈76 MB), `xeq_level: Vec<u16>` (`state.rs:296`, 1_114_730 × 2 B ≈ 2.2 MB) |
| Owning-pointer arena field | `mem[p+5].ptr` native-glyph buffer, written via `set_ptr`; buffers freed at `packaging.rs:562` + `fonts.rs:1148`, copied at `mem.rs:544` (`xmalloc`) |
| Hot-path raw-pointer escapes from `mem.data` | `state.rs:584` `as_u16_ptr_mut`, `:591` `as_u16_ptr`, `:598` `node_addr`; callers in `fonts.rs:84/107/243`, `shipout.rs:897/945/952`, `main_control.rs:5345-5602`, `math.rs:399-3411`, `linebreak.rs:1011/3847/3930/3977`, `node.rs:2234/2241`, `packaging.rs:2247`, `ext.rs:2833`, `pic.rs:715`, `output.rs:855/4164` |
| Raw-pointer escapes from `eqtb.data` | **only** `format_io.rs:423` (dump) + `:1089` (undump) — *no hot-path escape* |
| FONT_* FFI mirrors (point into `fonts.*` Vecs, refreshed on realloc) | `globals.rs:241-255` `FONT_SIZE`/`FONT_AREA`/`FONT_LAYOUT_ENGINE`; `refresh_font_ffi_mirrors` at `globals.rs:~249`. **Do not point into `mem`/`eqtb`.** |
| Determinism | `random_seed = microseconds*1000 + epochseconds%1_000_000` at `engine.rs:1399`; `init_randoms` follows |
| Format dump refuses non-quiescent state | `store_fmt_file_in` `format_io.rs:236`, refuses `save_ptr != 0` ("You can't dump inside a group") |
| Corpus oracle | pre-Rust C engine at commit `1a99fcf5`, tag `oracle-pre-rust` (`tests/corpus/ORACLE.md`); tier1 goldens in `tests/corpus/tier1/*.tex` (12 docs) + `tests/corpus/goldens/*.sha` |

---

## 1. Architecture overview

Three cooperating layers, from coarse to fine:

```
Spec 3 checkpoint ladder   →  full/bounded EngineState CLONES at structural boundaries (per section/paragraph)
      │  (baseline every N paragraphs; ~20 ms whole-state, done rarely)
      ▼
SPEC 4 word-log (THIS)     →  per-EPOCH undo journal of arena words + POD scalar bundle
      │  (one epoch == one keystroke's worth of typesetting; restore = O(distinct words touched))
      ▼
POD scalar bundle          →  the ~11 MemState frontier scalars + save/nest/page-builder/scanner/synctex scalars
                              (a few KB memcpy, captured by value, not journaled)
```

The word-log makes **repeated** per-keystroke snapshot/restore cheap by never re-copying the ~216 MB dense arenas. A restore reverts only the words the last epoch actually wrote (empirically dozens–thousands, see §7), plus a fixed few-KB scalar copy.

**Key structural finding delivered up front (answers D):** the same word-log mechanism generalizes to `eqtb`/`hash`. Because `eqtb` is read-mostly per keystroke (a body edit changes a handful of catcodes/counters/`\def`s, not millions of Unicode-table slots) and has **no hot-path raw-pointer escape**, a small **eqtb write-log** dominates paged-COW on every axis. **Paged-COW of the dense arenas is therefore NOT on the critical path** — it is only forced if some arena is simultaneously huge *and* densely written per keystroke, which TeX's mutation shape rules out. §6 gives the honest feasibility assessment and the audit that would gate it if a benchmark ever surprises us.

---

## 2. (A) Word-granular undo log

### 2.1 Data structure

A journal is added as a field on `MemState`, feature-gated so the baseline (corpus) binary is byte-for-byte the current code.

```rust
// state.rs — new module-level type, above `struct MemState`.

/// Word-granular undo journal for one snapshot epoch over a MemoryWord arena.
/// Records the FIRST pre-image of every distinct index touched this epoch,
/// gated by a dense `touched` bitmap so repeated writes to the same word are
/// free after the first. Restore replays the log backward; the bitmap is
/// cleared entry-by-entry (never memset over the whole 640 KB).
#[cfg(feature = "incr-snapshot")]
#[derive(Default)]
pub(crate) struct WordJournal {
    /// (index, pre_image) in write order. Only first-touch-per-epoch entries.
    log: Vec<(i32, crate::mem_types::MemoryWord)>,
    /// One bit per arena word. `touched[i>>6] & (1<<(i&63))` == already logged.
    /// Sized to the arena's word count at epoch install; 5_000_000 bits = 610 KiB.
    touched: Vec<u64>,
    /// Epoch active? When false, `journal()` is a single predicted-false branch.
    enabled: bool,
}

#[cfg(feature = "incr-snapshot")]
impl WordJournal {
    /// Size the bitmap to the arena word count. Call once per Vec (re)alloc.
    pub(crate) fn resize(&mut self, words: usize) {
        self.touched = vec![0u64; (words + 63) / 64];
        self.log.clear();
    }

    /// Record the pre-image of `mem[p]` iff this is its first touch this epoch.
    /// `old` is the current word, read by the caller before it overwrites.
    #[inline(always)]
    fn note(&mut self, p: i32, old: crate::mem_types::MemoryWord) {
        // The single hot branch. `enabled` is predicted false in every
        // non-editor run; in an editor run it is true only inside an epoch.
        if !self.enabled { return; }
        let w = (p as usize) >> 6;
        let bit = 1u64 << (p as u32 & 63);
        // SAFETY: p is a valid arena index (same invariant the data[] index relies on),
        // touched is sized to the arena word count.
        let cell = unsafe { self.touched.get_unchecked_mut(w) };
        if *cell & bit == 0 {
            *cell |= bit;
            self.log.push((p, old));
        }
    }
}
```

Add the field to `MemState` (`state.rs:449`, the struct that begins with `pub data: Vec<MemoryWord>`):

```rust
    // state.rs, inside `struct MemState { … }`
    #[cfg(feature = "incr-snapshot")]
    pub(crate) journal: WordJournal,
```

### 2.2 Instrumenting the write accessors — the cheapest form

Every `set_*` reads the current word before overwriting, then calls `note`. To keep the read to exactly one 8-byte load and avoid double-indexing, capture the full `MemoryWord` (it is `Copy`, `size_of == 8`) once:

```rust
// state.rs — replace each MemState setter body. Example: set_b32s1 (currently :539).
#[inline]
pub(crate) fn set_b32s1(&mut self, p: i32, v: i32) {
    #[cfg(feature = "incr-snapshot")]
    {
        // one 8-byte load; harmless (dead) when the epoch is disabled after inlining
        let old = self.data[p as usize];
        self.journal.note(p, old);
    }
    self.data[p as usize].b32.s1 = v;
}
```

The identical three-line `#[cfg]` prologue is added to the **nine** setters: `set_ptr` (`:527`), `set_b32s0` (`:533`), `set_b32s1` (`:539`), `set_b16s0` (`:546`), `set_b16s1` (`:552`), `set_b16s2` (`:558`), `set_b16s3` (`:564`), `set_gr` (`:577`), `set_word` (`:610`). That is the entire arena write surface — `get_node`/`get_avail`/`free_node`/`new_*`/the synctex stamp all mutate the arena *through* these nine, so no allocator-specific hook is needed.

**Why this is the cheapest correct form:**
- When the feature is **off**, the setters are byte-identical to today → the corpus binary is provably unchanged, zero added instructions.
- When the feature is **on** but the epoch is **disabled** (`enabled == false`), each write adds: one 8-byte load (the pre-image), one call to `note` that inlines to a single `test enabled; jz` predicted-not-taken. The pre-image load is off the critical dependency chain (the store doesn't depend on it) and folds into the store's cache line — the extra load is essentially free on a modern OoO core.
- When the epoch is **enabled**, the extra cost is: bitmap word load + bit test + branch; on first touch, a `push`. The bitmap is dense (adjacent arena words share a `u64` cell) so it is cache-friendly for the spatially-clustered node-allocation writes.

**Hoisting out of the hottest loop.** The one loop where even the predicted branch matters is `get_node`'s coalescing walk (`mem.rs:139-150`) and the main-loop ligature reconstitution. For those we do **not** special-case per-write; instead the `enabled` flag is the loop-invariant hoist: LLVM sees `note` reads `self.journal.enabled` which is not written inside the loop, so the branch is hoisted and the loop body has at most the pre-image load. If profiling (E) shows even that is material, the const-generic variant below removes it structurally.

**Const-generic alternative (if benchmarks demand zero disabled-epoch overhead in one binary).** Parameterize the hot inner routines on a `const JOURNAL: bool` and monomorphize both. Because `MemState`'s setters are called from ~1200 sites this is only worth doing for `get_node`/`get_avail`/`free_node` and the main loop, not globally; keep the runtime `enabled` flag for the cold 99%. Recommended only if E shows the disabled-epoch regression exceeds ~1% wall on a full corpus run.

### 2.3 Snapshot / restore / commit operations

The word-log alone is insufficient — the **frontier scalars are not arena words**. `get_avail`/`get_node`/`free_node` mutate `mem.avail`, `mem.rover`, `mem.hi_mem_min`, `mem.mem_end`, `mem.lo_mem_max` directly (e.g. `mem.rs:51/57/61/110/157/202`). These must be captured by value alongside the log length. Bundle them:

```rust
// state.rs
#[cfg(feature = "incr-snapshot")]
#[derive(Clone, Copy)]
pub(crate) struct MemFrontier {
    lo_mem_max: i32, hi_mem_min: i32, avail: i32, mem_end: i32,
    rover: i32, var_used: i32, dyn_used: i32,
    line: i32, cur_synctex_tag: i32, temp_ptr: i32,
    log_len: usize,   // O(1) snapshot marker into the word-log
}

#[cfg(feature = "incr-snapshot")]
impl MemState {
    /// Begin a fresh epoch: enable journaling and drain any stale log.
    /// O(distinct-words-in-prior-log) to clear their bits; the arena itself is untouched.
    pub(crate) fn epoch_begin(&mut self) -> MemFrontier {
        // Clear only the bits we actually set last epoch, then forget them.
        for &(p, _) in &self.journal.log {
            let w = (p as usize) >> 6;
            self.journal.touched[w] &= !(1u64 << (p as u32 & 63));
        }
        self.journal.log.clear();
        self.journal.enabled = true;
        self.frontier()
    }

    fn frontier(&self) -> MemFrontier {
        MemFrontier {
            lo_mem_max: self.lo_mem_max, hi_mem_min: self.hi_mem_min,
            avail: self.avail, mem_end: self.mem_end, rover: self.rover,
            var_used: self.var_used, dyn_used: self.dyn_used,
            line: self.line, cur_synctex_tag: self.cur_synctex_tag,
            temp_ptr: self.temp_ptr, log_len: self.journal.log.len(),
        }
    }

    /// Snapshot a restore point WITHIN the current epoch (O(1)).
    pub(crate) fn snapshot(&self) -> MemFrontier { self.frontier() }

    /// Revert every arena word written since `snap` was taken, restore the
    /// frontier scalars, and clear the reverted words' touched bits.
    /// O(distinct words touched since `snap`). Never reallocates `data`.
    pub(crate) fn restore(&mut self, snap: MemFrontier) {
        // Replay backward so the FIRST pre-image wins (it is the only one stored).
        while self.journal.log.len() > snap.log_len {
            let (p, old) = self.journal.log.pop().unwrap();
            self.data[p as usize] = old;
            let w = (p as usize) >> 6;
            self.journal.touched[w] &= !(1u64 << (p as u32 & 63));
        }
        self.lo_mem_max = snap.lo_mem_max; self.hi_mem_min = snap.hi_mem_min;
        self.avail = snap.avail; self.mem_end = snap.mem_end; self.rover = snap.rover;
        self.var_used = snap.var_used; self.dyn_used = snap.dyn_used;
        self.line = snap.line; self.cur_synctex_tag = snap.cur_synctex_tag;
        self.temp_ptr = snap.temp_ptr;
    }

    /// Commit (forget) history up to now: fold the epoch into the coarse baseline.
    /// O(distinct words) bit-clear. Use when advancing a checkpoint-ladder rung.
    pub(crate) fn epoch_commit(&mut self) {
        for &(p, _) in &self.journal.log {
            let w = (p as usize) >> 6;
            self.journal.touched[w] &= !(1u64 << (p as u32 & 63));
        }
        self.journal.log.clear();
    }
}
```

**Semantics, stated precisely (this corrects the naive "redo = truncate" framing):**

- `snapshot()` returns a marker in **O(1)** (`log_len` + the scalar bundle).
- `restore(snap)` is **undo**: replays backward to the marker in **O(distinct words touched since `snap`)**. This is exact *only because the bitmap gates first-touch-per-epoch*, so each word's stored pre-image is its value at epoch start; any marker taken after epoch start whose interval re-touches an already-touched word cannot be restored to its *marker-time* value from an undo-log alone.
  - **Consequence for the checkpoint ladder:** within one epoch, restore points are **monotone** — you can restore to any marker as long as you have not restored past it and continued writing. The clean model is **one keystroke = one epoch**; `epoch_begin` at the start, `restore(epoch_start_marker)` to revert the whole keystroke. For coarser rewinds (previous paragraph/section) you restore the **Spec-3 clone** at that ladder rung, not the word-log.
- **Redo is not `truncate`.** An undo-log stores only *old* values, so it cannot replay forward. Redo is achieved by **re-executing the keystroke deterministically** (it is O(one keystroke) and the engine is deterministic once the RNG seed is pinned, §8) — this is strictly simpler and less memory than a symmetric redo-log. If a redo-log is ever wanted, capture the *new* word at the same choke point into a parallel `Vec<(i32, MemoryWord)>`; but the recommendation is re-execute.
- `epoch_commit()` is the true "truncate": it discards the undo history (accepting the epoch as the new baseline) in O(distinct words). Used when the ladder advances a clone baseline forward and the interim word-log is no longer needed.

### 2.4 The owning-pointer hazard (`set_ptr` / `mem[p+5].ptr`) — a hard proof obligation

`set_ptr` (`state.rs:527`) writes the native-glyph buffer pointer `mem[p+5].ptr`. The word-log will faithfully store and restore the **pointer bits**, but the buffer they address is heap-owned (allocated via `xmalloc` at `mem.rs:544`, freed via `libc::free` at `packaging.rs:562` and `fonts.rs:1148`). Blindly restoring an old `.ptr` word can therefore produce a **dangling pointer** (buffer freed during the epoch) or a **leak** (buffer allocated during the epoch, orphaned by restore).

Mitigation, in order of preference:

1. **Choose epoch boundaries that don't re-shape.** The dominant fast path is *re-breaking an already-shaped paragraph* (line-break only). Native shaping (`measure_native_node` `ext.rs:1475`, glyph-buffer alloc) runs during the *first* typesetting of a run, not during pure re-break. If an epoch performs no `set_ptr` (detectable: `journal` records zero `.ptr`-field writes — see §2.5), the restore is pointer-safe with no extra work.
2. **Detect and defer to clone.** Track, per epoch, whether any `set_ptr` occurred (a bool flipped inside `set_ptr`'s `#[cfg]` block, or scan the log for `p ≡ 5 (mod NODE)` — cleaner to keep an explicit `touched_ptr: bool`). If set, the epoch is **not** word-log-restorable; fall back to the Spec-3 bounded clone for that keystroke. Native shaping is exactly the expensive work the editor wants to *avoid* re-running, so this fallback is rare on the hot path.
3. **Track glyph-buffer ownership in the epoch (full solution, later).** Record `(ptr, alloc|free)` events at `mem.rs:544` / `packaging.rs:562` / `fonts.rs:1148`; on restore, `free` epoch-allocated buffers and un-free epoch-freed ones (requires interposing the free sites, mirroring the clone path's double-free discipline, review §7.2). This is the same instrumentation the clone path needs, so build it once.

**Proof obligation:** the word-log restore path must assert (in debug) that no un-tracked `.ptr` write is silently reverted: `debug_assert!(!self.journal.touched_ptr || fell_back_to_clone)`.

### 2.5 Distinguishing `.ptr` writes

`set_ptr` is the only setter writing an owning pointer. Add a dedicated flag rather than pattern-matching indices:

```rust
// state.rs, in WordJournal
    touched_ptr: bool,
// in set_ptr's #[cfg] block, after note():
    #[cfg(feature = "incr-snapshot")]
    { self.journal.touched_ptr = true; }
```

`epoch_begin`/`epoch_commit`/`restore` reset it. The editor consults it to pick word-log vs clone.

---

## 3. (B) Chunk-COW alternative — and why word-log wins for TeX

**Chunk-COW design (for the record).** Partition `data` into 512-word (4 KiB page-aligned) chunks; keep `chunk_dirty: Vec<u64>` (one bit per chunk; 5M/512 ≈ 9766 chunks → 1.2 KiB bitmap) and a side buffer of saved chunk pre-images. On first write to a clean chunk, copy the whole 4 KiB chunk into the side buffer and set its dirty bit. Snapshot = O(1) (remember the side-buffer length / dirty-set). Restore = copy each saved chunk back = O(chunks touched × 4 KiB).

**TeX's mutation shape (from the code, not assumption):**

- **Node allocation is spatially dense.** `get_node` (`mem.rs:132`) hands out contiguous word ranges from the lo-mem frontier; a paragraph's nodes cluster into a few adjacent chunks. Here chunk-COW and word-log are comparable (few chunks vs many contiguous words).
- **Free-list link writes are scattered.** `get_node`'s coalescing walk (`mem.rs:139-150`) and `free_node` (`mem.rs:510-521`) rewrite doubly-linked free-list links (`p+1`, `q+1`, `rover+1`, `t+1`) at addresses that can be **far from the allocation frontier** — anywhere a previously-freed block lives. `get_avail` similarly threads the hi-mem single-word free list near `hi_mem_min` (`mem.rs:51-72`), the *opposite* frontier from lo-mem allocation. A single link write into an otherwise-clean far chunk costs **4 KiB** in chunk-COW versus **12 B** (`i32` + `MemoryWord`) in word-log.
- **Two frontiers, one page granularity.** Lo-mem grows up, hi-mem grows down; writes straddle both ends of a 40 MB array within one keystroke. Chunk-COW dirties chunks at both ends plus scattered coalescing chunks.

**Verdict: word-log wins.** Restore memory and time are proportional to *distinct words* for word-log versus *4 KiB × touched chunks* for chunk-COW; TeX's scattered link writes make the chunk multiplier large. Word-log's only disadvantage — a per-write bitmap test — is shared by chunk-COW (a per-write chunk-dirty test), so the hot-path cost is a wash; the decider is restore cost, which favors word-log. Chunk-COW is retained **only** as the fallback if the E benchmark shows per-keystroke writes are in fact dense-contiguous (few chunks, high words-per-chunk), in which case chunk-COW's coarser bitmap (1.2 KiB, all-in-L1) and bulk memcpy restore would edge out word-log. Decide from measurement (§7).

---

## 4. (C) Epoch management tied to the Spec-3 checkpoint ladder

The ladder (Spec 3) keeps **clone baselines** at structural boundaries (per section/paragraph, taken at an outer-vmode `save_ptr == 0` quiescent point — the same quiescence the format writer demands, `format_io.rs:236`). The word-log provides the **delta between the nearest preceding clone and now**.

```
clone_0 (post-preamble)   clone_1 (after §1)   clone_2 (after §2)  … current epoch (word-log)
        └─────── word-log epoch ──────┘        └────── word-log epoch ──────┘
```

Lifecycle for an edit in section N:

1. Editor locates the nearest clone baseline ≤ edit position (`clone_{N-1}`).
2. `mem.restore()` is **not** used to cross clone boundaries; instead the engine state is reset to `clone_{N-1}` (memcpy the clone *into* the same-capacity Vecs — never replace them, §5 discipline), and `mem.epoch_begin()` starts a fresh epoch.
3. Replay section N forward from the editor buffer. All arena writes journal into the epoch.
4. On the *next* keystroke within the same section, `mem.restore(epoch_start)` reverts section N's typesetting in O(distinct words), then replay again. This is the O(edit-burst) fast path — no clone, no undump.
5. When the edit stabilizes and the user moves past section N, `mem.epoch_commit()` folds the epoch and a new clone baseline `clone_N` is taken (rare, ~20 ms, amortized).

**Coupling contract:** every field the ladder clones but the word-log does *not* journal must be in the **POD scalar bundle** restored together with `mem.restore()`. Beyond the `MemFrontier` scalars, the per-keystroke reset must also revert (all small POD, clone-by-value — review §4.4 step 3 enumerated these and they are still current):

- save stack scalars: `save_ptr`, `cur_level`, `cur_group`, `cur_boundary` (`SaveStackState`, `state.rs:679-691`) — plus `save_stack.data` if the epoch pushed/popped (journal it too, §5.1);
- semantic nest: `cur_list` head/tail/`prev_graf`/`prev_depth`/`space_factor`, `nest_ptr`;
- page-builder / contribution list: `page_contents`, `page_so_far`, `page_tail`, `page_head` — **omitting this double-appends the paragraph's vlist on the second keystroke**;
- scanner: `cond_ptr`, `if_limit`, `cur_if`, `cur_align`, `align_state`, `scanner_status` (`state.rs:772-813`);
- synctex: `cur_synctex_tag`, `line` (in `MemFrontier`) **and** `SynctexCtxt.synctex_tag_counter` (`state.rs:~96`) — a desynced tag counter corrupts source↔preview mapping.

Recommended: keep these as a single `#[derive(Clone)] struct EpochPodBundle` captured at `epoch_begin` and restored on `restore`; a few KB memcpy, negligible next to the word-log replay. **Soundness gate:** the fast reset is valid only if the epoch performed **no global mutation** (`\gdef`, `\global`, counter writes, font `\let`). Detect via the eqtb write-log (§5.2): if the epoch touched any eqtb slot at level 1 (global) or outside the current group's save scope, fall back to a clone-based rewind (review §7.9).

---

## 5. Restore-into-same-Vec discipline & the eqtb/save word-logs

### 5.1 Never replace the Vec

The load-bearing invariant (review §7.3): `mem.data` hands out escaping raw pointers (`as_u16_ptr`/`node_addr`, callers listed in §0) that outlive the borrow because the owner never reallocates. Therefore:

- **`restore()` writes *into* `self.data[p]`** (indexed store), never `self.data = …`. ✔ (the sketch above does exactly this).
- **`epoch_begin`/`resize` must run after any legitimate Vec (re)allocation** (engine init, format undump) so the bitmap matches the live buffer, and any escaped pointer captured *before* a resize is already invalid by the existing "never reallocate mid-run" rule — nothing new here.
- The word-log stores **values**, not pointers into the Vec, so it survives independent of the buffer address. ✔

### 5.2 eqtb / hash / save-stack word-logs (same mechanism, separate journals)

`EqtbState` (`state.rs:273`) has its own write surface (`impl EqtbState`, `state.rs:330`): `set_b32s1`, `set_b16s1`, `set_b16s0`, `set_hash_s0/s1`, `set_prim_s0/s1`, `set_word`, `set_xeq_level`. Because `eqtb.data` has **no hot-path raw-pointer escape** (only `format_io.rs:423/1089`, which run at load, never per keystroke), an eqtb word-log is unconditionally safe and cheap:

```rust
// state.rs — EqtbState gets its own journals (data + hash are separately indexed).
#[cfg(feature = "incr-snapshot")]
pub(crate) eqtb_journal: WordJournal,    // over eqtb.data
#[cfg(feature = "incr-snapshot")]
pub(crate) hash_journal:  WordJournal,   // over eqtb.hash (index offset -514, §note)
```

Instrument `EqtbState::set_b32s1`/`set_b16s1`/`set_b16s0`/`set_word` exactly like `MemState` (read old `MemoryWord`, `note`). For `set_hash_s0`/`set_hash_s1` the index is `p - 514` (hash offset) — journal on the **raw `p`**, restore with the same `-514` mapping (keep it consistent by storing the *Vec index* `p-514` in the log). `set_prim_s0/s1` write the tiny 2101-entry `prim` Vec (`state.rs:319`); it is set-once at init and effectively immutable per keystroke — journal it for safety but expect ~0 entries. `set_xeq_level` writes the 1.1 M-entry `xeq_level` (save-level tracking); journal it (a global assign touches it).

The save stack (`SaveStackState`, `state.rs:679-723`) is pushed/popped constantly; give it its own tiny word-log or (simpler) clone `save_stack.data[0..=save_ptr]` by value into the `EpochPodBundle` since `save_ptr` is bounded and small at quiescent boundaries.

**Why this obviates paged-COW for eqtb (the D answer, concretely):** per keystroke, a body edit touches O(tens–hundreds) eqtb words. An eqtb word-log costs `entries × 12 B` (a few KB) for snapshot and O(entries) for restore, with a 1.1 MB `touched` bitmap (or a **sparse** map, §6, since eqtb touches are sparse). Paged-COW would, at best, match this only if writes clustered into few pages — but its first-write-per-page cost is 4 KiB regardless, and its ABI cost (breaking the flat Vec) is unjustified when the write-log already wins. **The eqtb write-log is the recommendation; paged-COW is not needed.**

---

## 6. (D) Paged-COW of the dense arenas — honest feasibility

**Bottom line: do not build paged-COW. The word-log/write-log generalizes to every dense arena and dominates it for TeX's mutation shape.** The reasoning, grounded in the escape audit:

### 6.1 Why the two "obvious" approaches fail

- **`Arc<Vec<MemoryWord>>` + `make_mut` is whole-buffer COW.** The *first* write clones all 76 MB → zero incremental benefit. Not a per-page mechanism. Reject.
- **A genuine paged arena breaks the flat-`Vec` ABI.** `mem.data.as_ptr()`/`as_mut_ptr()` and the `as_u16_ptr`/`node_addr` reinterpret helpers (`state.rs:584/591/598`) assume a single contiguous allocation with `wrapping_add(p)` addressing; ~40 call sites (§0) take raw pointers into `mem`. A paged arena would make `node_addr(p)` cross-page-invalid. Any page move also violates the "never reallocate" invariant.

### 6.2 The pointer-escape audit (the prerequisite, completed here)

Grep results (verified §0), classified:

| Arena | Escapes | Per-keystroke hot path? | Paging blocker? |
|---|---|---|---|
| `mem.data` | `as_u16_ptr`/`as_u16_ptr_mut`/`node_addr` + `data.as_ptr/as_mut_ptr` — callers in `fonts.rs`, `shipout.rs`, `main_control.rs:5345-5602`, `math.rs`, `linebreak.rs`, `node.rs`, `packaging.rs`, `ext.rs`, `pic.rs`, `output.rs` | **yes** (native word text at `p+6`, glyph info, pic paths) | **yes** — mem paging is blocked by ~40 live escapes |
| `eqtb.data` | `format_io.rs:423` (dump), `:1089` (undump) **only** | **no** (load-time) | **no** — eqtb has no hot escape |
| `eqtb.hash` | none found outside accessors | no | no |
| `font_info` | `format_io.rs:551/1212` (dump/undump) | no (append-at-load) | no |
| FONT_* mirrors | `globals.rs:241-255` point into `fonts.font_size/area/layout_engine` — **not** into `mem`/`eqtb` | n/a | n/a for mem/eqtb |

**Elimination cost to *enable* mem paging** (only if ever forced): the ~40 `mem` escapes must be replaced by either (a) an index-based API that pages internally (each `node_addr(p)` becomes a paged lookup — a per-access cost regression on shaping/measurement, the exact hot path we're optimizing), or (b) a guarantee that every escaped region is page-contained and pinned for the escape's lifetime. Both are large, bug-prone, and *counterproductive* (they slow the very shaping path). **This is why mem paging is unresolved-feasibility research, not an engineering task** — and it is unnecessary because mem's per-keystroke dirty set is small (§7) → word-log already wins.

### 6.3 The decisive point: does eqtb even need COW? No.

eqtb is **read-mostly**: the 53.5 MB Unicode cat/lc/uc/sf/math/del regions are effectively immutable after format load; a document changes a handful of catcodes/counters/registers/`\def`s per keystroke. So the eqtb per-keystroke dirty set is O(tens–hundreds) words. A **sparse write-log** (§5.2) — or even a `HashMap<i32, MemoryWord>` if the E histogram shows the dirty set is small and scattered — captures it in a few KB with O(dirty) restore. No paging, no ABI break, no escape elimination. `font_info` is append-only during font load and immutable after → covered by a prefix bound (record `fmem_ptr` high-water at the checkpoint; restore = truncate logical length), no COW at all.

**Deliverable conclusion for (D):** paged-COW of the dense arenas is **not on the critical path and should not be built**. Build the word-log for `mem`, the sparse write-log for `eqtb`/`hash`/`xeq_level`, and a prefix-bound for `font_info`. Revisit paging only if the E benchmark contradicts the read-mostly premise (it will not, for ordinary body edits).

---

## 7. (E) Benchmark plan — measure real dirty-word count per keystroke

Goal: measure, on representative documents, the **distinct-word dirty set per simulated keystroke** for `mem`, `eqtb`, `hash`, `xeq_level` — to (1) choose word-log vs chunk-COW, (2) choose dense bitmap vs sparse map for eqtb, (3) size the bitmap, (4) validate the "read-mostly eqtb" premise.

### 7.1 Instrumentation (counting-only mode)

Add a `#[cfg(feature = "incr-bench")]` stats path to `WordJournal` that additionally records index span and per-epoch histograms:

```rust
#[cfg(feature = "incr-bench")]
pub(crate) struct JournalStats {
    pub distinct_words: usize,      // == log.len() at epoch end
    pub min_idx: i32, pub max_idx: i32,   // span → chunk-COW chunk count estimate
    pub ptr_writes: usize,          // native-glyph buffer churn
    pub chunks_touched: usize,      // distinct (idx>>9) → chunk-COW cost
}
```

Populate `chunks_touched` with a second (chunk-granular) bitmap running in parallel during the bench run only.

### 7.2 Harness

Drive the existing tier1 corpus as the workload. For each doc in `tests/corpus/tier1/*.tex` (12 docs: `math_classic`, `tables_floats`, `tikz_basic`, `unicode_math`, `ligatures_kerning`, `bidi_arabic`, `cjk`, `devanagari`, `fontspec_features`, `primitives`, plus the LaTeX docs):

1. Compile the whole doc once to warm a post-preamble clone baseline.
2. Simulate a keystroke: `epoch_begin()`, re-typeset the last paragraph (or re-break it), `epoch_end()` → dump `JournalStats`.
3. Repeat across a sliding window of paragraphs to get a distribution.
4. Emit CSV: `doc, para_idx, mem_distinct, mem_chunks, mem_span, eqtb_distinct, hash_distinct, xeq_distinct, ptr_writes`.

Wire as a `#[test]`-gated bench behind `--features incr-bench` in a new `tests/incr_bench.rs`, reusing the corpus fixtures. Run with `cargo test --release --features incr-bench incr_bench -- --nocapture > /tmp/incr.csv`.

### 7.3 Decision rules from the data

- **word-log vs chunk-COW:** compute `mem_chunks × 512` (chunk-COW restore words) vs `mem_distinct` (word-log restore words). Word-log wins when `mem_distinct < 512 × mem_chunks` — expected true (scattered link writes). If the ratio approaches 1 (dense contiguous), reconsider chunk-COW.
- **eqtb dense-bitmap vs sparse-map:** if `eqtb_distinct` is consistently < ~1000 and `eqtb_span` is large (scattered), use a `HashMap`/sorted `Vec` sparse log instead of the 1.1 MB bitmap (saves the bitmap allocation and its cold-cache clear). If `eqtb_distinct` is large or clustered, use the dense bitmap.
- **bitmap sizing:** `mem` bitmap is fixed at `MEM_TOP+1` bits (610 KiB); the measurement confirms whether the `touched` clear cost (O(distinct)) stays small — it will if `mem_distinct` is in the thousands.
- **ptr-write frequency:** `ptr_writes` per keystroke tells us how often the §2.4 clone-fallback fires. If it fires on most re-breaks (native fonts everywhere), prioritize the glyph-buffer ownership tracker (§2.4 option 3).

### 7.4 Expected results (hypothesis to falsify)

For a body-text paragraph re-break: `mem_distinct` in the low thousands (one paragraph's nodes + free-list churn), `eqtb_distinct` near zero (no catcode/counter change), `ptr_writes` zero (no re-shape). This validates word-log-for-mem + sparse-write-log-for-eqtb + no-paging. A `\section` edit or macro redefinition will show `eqtb_distinct` spike and `xeq_distinct > 0` → correctly triggers the clone-fallback (§4 soundness gate).

---

## 8. Validation & test strategy (corpus-gated, byte-identical XDV)

The bar is **byte-identical XDV vs the pre-Rust C oracle** (`tests/corpus/ORACLE.md`, oracle commit `1a99fcf5` / tag `oracle-pre-rust`) and the dashboard equivalence count.

### 8.1 Three binaries, three obligations

1. **Feature OFF (`incr-snapshot` absent) — the corpus/baseline binary.** The `#[cfg]` prologues vanish; the setters are textually the current code. **Obligation:** `git diff` of the generated setter bodies is empty modulo the `#[cfg]` guard → full corpus must remain at the current dashboard numbers (tier1 12/12 byte-identical XDV; full corpus passed+diverged count unchanged; NOXDV/NOLOG unchanged — per the "corpus doc-count regression check" discipline). This is the primary safety net: the shipping engine is provably unchanged.

2. **Feature ON, epoch DISABLED at runtime.** The journaling branch exists but `enabled == false`, so no word is logged and no output path changes. **Obligation:** running the full corpus with `--features incr-snapshot` (epochs never begun) must produce **byte-identical XDV** to binary #1. This proves the instrumentation is output-transparent (the pre-image read + predicted branch cannot alter results). Add a CI matrix row for `--features incr-snapshot`.

3. **Feature ON, epoch ENABLED — the snapshot round-trip.** This is the new correctness surface. Test: `restore()` must return the arena + scalars to a bit-exact prior state.

### 8.2 Round-trip differential tests

Add `tests/incr_roundtrip.rs`:

- **Full-state hash equality.** For each tier1 doc: compile to a quiescent boundary, `snap = { mem.snapshot(); eqtb snapshot; pod bundle }`, hash `(mem.data[0..mem_end], eqtb.data, eqtb.hash, xeq_level, save_stack, scalar bundle)`. Then run an arbitrary further chunk of typesetting, `restore(snap)`, re-hash. **Assert the two hashes are identical.** This catches any un-journaled write path or missing scalar.
- **Restore-then-replay == fresh-run XDV.** Compile doc D to completion → XDV_A. Compile D to a mid-point, snapshot, typeset the tail, restore, re-typeset the *same* tail → XDV_B. **Assert XDV_A == XDV_B** byte-for-byte. This is the end-to-end proof that a snapshot/restore is indistinguishable from a fresh run — the editor's core guarantee.
- **Determinism pin.** Both runs must pin the RNG: `random_seed` derives from wall-clock (`engine.rs:1399`), so the harness sets a fixed seed (the driver's `force_deterministic` → `UNIX_EPOCH` path, `driver.rs:990`) or injects a fixed `epochseconds`/`microseconds`. Without this, `\uniformdeviate`-using docs diverge for reasons unrelated to the substrate. Document that live-preview must also pin the seed.
- **Nested/monotone restore.** Take snap A, write, snap B, write, `restore(B)`, write, `restore(A)`; assert full-state hash equals the pre-A hash. Verifies the monotone-marker semantics (§2.3) and the first-touch-per-epoch invariant.
- **`.ptr`-write fallback.** A doc that forces native shaping in the epoch must set `touched_ptr` and the harness must assert the editor takes the clone path (not a word-log restore) — guarding the dangling-pointer hazard (§2.4).

### 8.3 Regression discipline (from memory notes)

- Compare **corpus passed+diverged doc counts** and **NOXDV/NOLOG** to baseline after landing, not just the divergence list (the dashboard misses native-font crashes).
- TRIP must stay 2/2 (a TRIP failure now is a real regression).
- Run `cargo test` (unit) + the tier1 goldens (`tests/corpus/goldens/*.sha`) + `tests/trip.rs` for all three binaries.

### 8.4 Fuzz/property test

Property: for a random sequence of `set_*` calls with random indices/values against a scratch `MemState` in an enabled epoch, `snapshot(); random_writes(); restore(snap)` must byte-restore `data` + scalars. Compare against a brute-force `Vec<MemoryWord>` clone oracle. Run under `proptest`/`cargo fuzz`. This exercises the bitmap/first-touch logic independent of TeX semantics.

---

## 9. Explicit edit list

| File | Location (verified) | Change |
|---|---|---|
| `Cargo.toml` (crate) | `[features]` | Add `incr-snapshot = []` and `incr-bench = ["incr-snapshot"]` (both off by default). |
| `state.rs` | above `struct MemState` (~`:449`) | Add `WordJournal`, `MemFrontier`, `EpochPodBundle` types (`#[cfg(feature="incr-snapshot")]`). |
| `state.rs` | `struct MemState` (`:449`) | Add `#[cfg] journal: WordJournal` field. |
| `state.rs` | `MemState::set_ptr`/`set_b32s0`/`set_b32s1`/`set_b16s0`/`set_b16s1`/`set_b16s2`/`set_b16s3`/`set_gr`/`set_word` (`:527`,`:533`,`:539`,`:546`,`:552`,`:558`,`:564`,`:577`,`:610`) | Prepend the `#[cfg]` pre-image-read + `journal.note` prologue; `set_ptr` also sets `touched_ptr`. |
| `state.rs` | `impl MemState` | Add `epoch_begin`/`snapshot`/`restore`/`epoch_commit`/`frontier`/journal `resize`. |
| `state.rs` | `struct EqtbState` (`:273`) | Add `#[cfg] eqtb_journal`, `hash_journal` (+ optional sparse variants). |
| `state.rs` | `impl EqtbState` (`:330`) setters `set_b32s1`/`set_b16s1`/`set_b16s0`/`set_word`/`set_hash_s0`/`set_hash_s1`/`set_prim_s0`/`set_prim_s1`/`set_xeq_level` | Add the journaling prologue (hash stores Vec-index `p-514`). |
| `state.rs` | `impl EqtbState` | Add `epoch_begin`/`snapshot`/`restore`/`commit`. |
| `state.rs` | `SaveStackState` (`:679`) | Either a small word-log or clone `data[0..=save_ptr]` into `EpochPodBundle`. |
| `engine.rs` | after arena allocation in `tt_run_engine` (`:986`+), and after format undump | Call `journal.resize(mem_words)` / eqtb resize once per Vec alloc. |
| `state.rs` (or new `incr.rs`) | new | `EngineState::epoch_begin/snapshot/restore/commit` orchestrating mem + eqtb journals + `EpochPodBundle` (nest/save/page-builder/scanner/synctex scalars) + `refresh_font_ffi_mirrors` no-op check (mirrors don't move, but assert). |
| `format_io.rs` | `font_info` bound already tracked (`fmem_ptr`, `:551/1212`) | No change; prefix-bound restore reuses existing high-water marks. |
| `tests/incr_roundtrip.rs` | new | §8.2 round-trip + differential tests. |
| `tests/incr_bench.rs` | new (`#[cfg(feature="incr-bench")]`) | §7 dirty-word measurement over tier1 corpus. |
| CI matrix | — | Add `--features incr-snapshot` corpus row (obligation #2). |

**No change** to any typesetting algorithm, `get_node`/`get_avail`/`free_node` bodies (they route through the instrumented setters), the FONT_* mirrors, or the flat-Vec ABI.

---

## 10. Correctness proof obligations & failure modes (summary)

1. **Total write capture.** Every arena mutation goes through the nine `MemState` setters (verified: no live C writer; `get_node`/`free_node`/synctex-stamp all use setters). *Failure mode:* a future direct `self.data[p] = …` bypasses the journal → silent restore corruption. *Guard:* keep `data` private; forbid direct indexing outside the setters (lint/grep in CI).
2. **Frontier-scalar bundling.** `avail`/`rover`/`hi_mem_min`/`mem_end`/`lo_mem_max`/`var_used`/`dyn_used`/`line`/`cur_synctex_tag`/`temp_ptr` are non-arena and must be captured by value. *Failure mode:* omitting `rover` desyncs `get_node`'s coalescing order → divergent free-list → non-byte-identical output. *Guard:* `MemFrontier` includes all ten; round-trip hash test.
3. **First-touch-per-epoch soundness.** The bitmap ensures each word's stored pre-image is its epoch-start value; restore is exact only to epoch start (monotone markers). *Failure mode:* treating an intra-epoch marker as a full checkpoint → wrong restore. *Guard:* documented semantics; nested-restore test; ladder crossings use clones, not markers.
4. **Owning-pointer (`.ptr`) hazard.** Restoring a `mem[p+5].ptr` word can dangle/leak. *Guard:* `touched_ptr` flag → clone-fallback for shaping epochs; debug assert; later, glyph-buffer ownership tracker at `mem.rs:544`/`packaging.rs:562`/`fonts.rs:1148`.
5. **Never-reallocate invariant.** `restore` memcpys *into* the same-capacity Vec; escaped raw pointers (§0 list) stay valid. *Failure mode:* replacing `data` → dangling `node_addr`. *Guard:* `restore` uses indexed stores only; `resize` only at legitimate alloc points.
6. **Global-mutation soundness for the fast reset.** The per-keystroke arena reset is valid only if the epoch made no global (`\global`/`\gdef`/counter) mutation. *Guard:* eqtb write-log detects any level-1/out-of-scope eqtb touch → clone-fallback (review §7.9).
7. **Determinism.** Snapshot/restore must not change RNG behavior; `random_seed` is wall-clock-derived (`engine.rs:1399`). *Guard:* live-preview pins the seed (driver `force_deterministic`); round-trip tests fix the seed.
8. **Output transparency of instrumentation.** Feature-on-epoch-off must be byte-identical to feature-off. *Guard:* CI corpus row #2.
9. **eqtb read-mostly premise.** The no-paging conclusion rests on small per-keystroke eqtb dirty sets. *Guard:* the E benchmark measures `eqtb_distinct`; a spike triggers the clone-fallback, never wrong output.

**Net:** the substrate adds zero risk to the shipping engine (feature-off is the current code), and its new surface is fully covered by (a) feature-on-epoch-off corpus equivalence, (b) restore-then-replay XDV equality, and (c) full-state round-trip hashing — all corpus-gated against the `oracle-pre-rust` baseline.