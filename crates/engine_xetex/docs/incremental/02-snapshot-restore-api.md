I have verified all the anchors I need. Here is the spec.

---

# SPEC 2 — `EngineState::snapshot()` / `restore()` + FFI‑Ownership Refactor

**Target crate:** `/home/yendric/Code/tectonic/crates/engine_xetex`
**Purpose:** provide cheap, correct capture/restore of the full engine arena so a live‑preview host can rewind to a prefix checkpoint and re‑typeset only the tail — *without* touching any typesetting algorithm. This spec delivers (A) the `Snapshot` type and `snapshot`/`restore` bodies, (B) the two FFI‑ownership refactors that are hard prerequisites (Arc‑shared font handles; glyph blobs moved out of the `mem` union), (C) file‑handle journaling, (D) mirror/locator refresh. Everything below is anchored to lines I opened and re‑verified in the current tree.

---

## 0. Executive summary and the reason (B) must land first

`EngineState` (`state.rs:8`) derives only `Default` — there is deliberately **no `Clone`** anywhere. The blocker to `#[derive(Clone)]` is not the POD arenas; it is three classes of field that own or alias C resources:

1. **Font FFI handles** — `FontState.font_layout_engine` / `font_mapping` (`state.rs:903,907`), each `Vec<*mut c_void>`, freed once per slot at `engine.rs:1453‑1464` (`release_font_engine` / `TECkit_DisposeConverter`). A naïve clone of these `Vec`s duplicates the raw handles → **double free** at teardown.
2. **Glyph blobs inside the `mem` union** — `mem[p+5].ptr` (`MemoryWord.ptr`, `mem_types.rs:53`) holds an `xcalloc`'d buffer for every native‑word node. A byte‑clone of `mem.data` duplicates that pointer → **double free** at `packaging.rs:562` / `fonts.rs:1148`. This is the reason `mem.data` is *not* plain‑`Copy` today, and it is the hard prerequisite for any `mem` COW.
3. **Live OS file handles** — `input_file` / `read_file` (`state.rs:196,262`), `dvi_file` / `log_file` / `rust_stdout` (`state.rs:1358,1333,1332`). These cannot be cloned; they are journaled (offset capture) or routed to null for preview.

Everything else is either a POD arena (deep‑copy or bounded copy), a plain scalar, or a re‑establishable locator. So the plan is: **(B) first** — convert (1) to Arc‑refcounted handles and (2) to a side arena of `Arc<[u8]>`, which makes `mem.data`, `eqtb.data`, `save_stack.data`, `font_info` all *genuinely* `Copy`‑element `Vec`s — **then (A)** becomes a mechanical field‑by‑field capture.

### Cost model (verified sizes)

| Arena | size expr | init value | bytes |
|---|---|---|---|
| `mem.data` | `MEM_TOP+2` words | `MEM_TOP=4_999_999` (`consts.rs:146`) | ~40 MB |
| `eqtb.data` | `eqtb_top+1` words, `eqtb_top=EQTB_SIZE+hash_extra`, `hash_extra=600_000` (`engine.rs:1009,1074`), `EQTB_SIZE=DIMEN_BASE+278` (`consts.rs:55`) | — | ~5–7 MB |
| `eqtb.hash` | `1+hash_top-514` × `B32x2` | — | ~5 MB |
| `eqtb.xeq_level` | `1_114_730` × u16 (`state.rs:321`) | fixed | 2.2 MB |
| `strings.str_pool` | `TRE_POOL_SIZE+1` u16, `=6_250_001` (`engine.rs:975`) | fixed | 12.5 MB |
| `strings.str_start` | `TRE_MAX_STRINGS+1` i32, `=565_537` | fixed | 2.3 MB |
| `fonts.font_info` | `font_mem_size+1` words, `=8_000_001` (`engine.rs:980`) | fixed | 64 MB |

A *full* deep snapshot is therefore ≈ 130 MB and dominated by three fixed‑capacity arenas that are mostly unused. **Bounded copy** (copy only up to the live high‑water mark) is mandatory for a "cheap" snapshot; it is specified in §2.3 and cuts the typical cost to the live prefix (single‑digit MB for an interactive document). The Arc/COW work in (B) is what later lets Spec 3 replace even the bounded copy with dirty‑region tracking; Spec 2 delivers the correct, allocation‑stable baseline.

---

## 1. Field‑by‑field disposition table

Legend: **POD** = plain `memcpy`/`Copy` (scalars and `Copy`‑element `Vec`s); **POD‑bounded** = POD but only `[0..hi]` is live, copy bounded; **Arc** = refcounted share (post‑(B)); **journal** = capture fd offset, re‑seek on restore; **rebuild** = recomputed/ re‑installed on restore, not stored; **locator** = per‑run pointer, re‑install on restore; **null‑at‑rest** = must be null/None at a quiescent snapshot point, asserted.

### `EngineState` top level (`state.rs:8‑24`)
All 15 sub‑states are captured; dispositions below are per field.

### `MemState` (`state.rs:449‑473`)
| field | disp | note |
|---|---|---|
| `data: Vec<MemoryWord>` | **POD‑bounded** (after (B)) | copy `[0..=lo_mem_max]` ∪ `[hi_mem_min..=mem_end]`; the gap is free. `MemoryWord` becomes pure POD once `.ptr` slot holds an index (§3.B.2). |
| `lo_mem_max,hi_mem_min,avail,mem_end,rover,var_used,dyn_used` | **POD** | the free‑list scalar bundle; **must travel atomically with `data`** (§2.2). |
| `line,cur_synctex_tag,temp_ptr` | **POD** | `get_node` synctex‑stamp inputs + scratch; travel with `data`. |

### `EqtbState` (`state.rs:273‑305`)
| field | disp |
|---|---|
| `data: Vec<MemoryWord>` | **POD‑bounded** to `eqtb_top+1` (dense; copy full or to top). |
| `prim: Vec<B32x2>` | **POD** (fixed 2101). |
| `xeq_level: Vec<u16>` | **POD** (fixed 1_114_730). |
| `hash: Vec<B32x2>` | **POD**. |
| `hash_used,hash_high,no_new_control_sequence,prim_used,cs_count,hash_extra,eqtb_top,hash_top` | **POD** scalars. |

### `SaveStackState` (`state.rs:678‑691`)
| field | disp |
|---|---|
| `data: Vec<MemoryWord>` | **POD‑bounded** to `save_ptr+1`. |
| `save_ptr,max_save_stack,save_size,cur_level,cur_boundary` | **POD**. |
| `cur_group: GroupCode` | **POD** (`Copy` enum, `state.rs:621`). |

### `ScannerState` (`state.rs:747‑883`)
All fields **POD** scalars/arrays (`cur_mark[5]`, `disc_ptr[4]`, `total_stretch[4]`, `active_width[7]`, `cur_i: B16x4`, `sa_null: MemoryWord`, `empty: B32x2`) **except**:
| field | disp |
|---|---|
| `max_reg_help_line: *const c_char` | **locator/POD‑ptr** — points at a `c"..."` rodata literal (`engine.rs:1249`); the pointer is `'static`, so copying it is safe (copy‑safe locator). |
| `native_text: Vec<u16>`, `native_text_size,native_len,save_native_len` | **POD** (per‑node scratch; small). Capture whole (they are set‑before‑read scratch; capturing is harmless and keeps restore exact). |

### `ConfigState` (`state.rs:885‑894`) — `Copy` already
All **POD**. Snapshot may skip (host re‑applies via `apply_config`) but capturing is trivial and keeps `restore` self‑contained.

### `FontState` (`state.rs:896‑992`)
| field | disp |
|---|---|
| `font_layout_engine` | **Arc** — becomes `Vec<Option<Arc<LayoutEngineHandle>>>` (§3.B.1). |
| `font_mapping` | **Arc** — becomes `Vec<Option<Arc<MappingHandle>>>` (§3.B.1). |
| `loaded_font_mapping: *mut c_void` | **null‑at‑rest** — transient scratch during `find/load_native_font`; asserted null at a between‑token snapshot; captured as raw ptr (not owning). |
| `loaded_font_flags,loaded_font_letter_space,native_font_type_flag,xtx_ligature_present,gave_char_warning_help` | **POD** scalars. |
| `font_mem_size,font_max,fmem_ptr,font_ptr,null_character` | **POD**. |
| `font_info: Vec<MemoryWord>` | **POD‑bounded** to `fmem_ptr` (font metrics are `b16/b32`, never `.ptr` — pure POD). |
| every `font_*`/`*_base`/`char_base…param_base`/`font_check/size/dsize/params/name/area/bc/ec/bchar…/glue/used/flags/letter_space` `Vec` | **POD** (all `Copy`‑element). |
| `xdv_buffer,mapped_text` | **POD** (output/scratch). |

### `InputClusterState` (`state.rs:149‑271`)
| field | disp |
|---|---|
| `param_stack,line_stack,grp_stack,if_stack,source_filename_stack,full_source_filename_stack,eof_seen,buffer` | **POD** (`Copy` elems). |
| `input_stack: Vec<InputState>` | **POD** (`InputState` is `Copy`, `engine.rs:1039`). |
| `cur_input: InputState`, `nest: Vec<ListStateRecord>`, `cur_list: ListStateRecord` | **POD** (`ListStateRecord` is `Copy`, `mem_types.rs:89`). |
| `input_file: Vec<*mut UFile>` | **journal** (§4) — capture per‑handle tell() offset + `UFile` POD body (`saved_char/skip_next_lf/encoding_mode`); do **not** copy the raw handle as an owner. |
| `read_file: [*mut UFile;16]`, `read_open:[u8;17]` | **journal** for open streams; `read_open` state is POD. |
| `name_of_input_file: *mut c_char` | **rebuild** — owned C string (`engine.rs:1484`); snapshot stores an owned `CString` copy, restore re‑`strdup`s (never aliases). |
| `abspath_of_input_file: AbsPathBuf` | **POD** (`[c_char;1024]`, `state.rs:138`). |
| all scalar cursors (`input_ptr,nest_ptr,in_open,first,last,name_length,…`) | **POD**. |

### `StringsState` (`state.rs:1490‑1505`)
| field | disp |
|---|---|
| `str_start: Vec<i32>` | **POD‑bounded** to `str_ptr+1`. |
| `str_pool: Vec<u16>` | **POD‑bounded** to `pool_ptr`. |
| all scalars | **POD**. |

### `HyphState` (`state.rs:1637‑1687`)
All fixed/POD `Vec`s and scalars. **POD** in full. (Cold; typically unchanged across a run — see §2.4 "share‑if‑unchanged" optimization.)

### `LinebreakState` (`state.rs:1523‑1559`), `MainLoopState` (`state.rs:30‑59`)
Per‑paragraph / per‑iteration scratch, all **POD** (`hlist_stack: Vec<i32>` fixed 513). Set‑before‑read; capture whole.

### `ShipoutState` (`state.rs:1305‑1371`)
| field | disp |
|---|---|
| `dvi_buf: Vec<u8>` | **POD**. |
| `write_file:[usize;16]`, `write_open:[bool;18]`, `dvi_file,log_file,rust_stdout` | **journal / route‑to‑null** (§4) — for preview these are `\write`/DVI/log sinks; the preview host routes `dvi_file`/`log_file` to null and does **not** rewind them. Captured as POD `usize` handle ids plus the `write_open` flags. |
| `current_diagnostic: *mut c_void` | **null‑at‑rest** — a live diagnostic capture buffer; null between tokens; asserted. |
| every other field (`selector,term_offset,file_offset,epochseconds,microseconds,dead_cycles,page_*,last_*,dvi_h/v/f,cur_h/v,total_pages,…`) | **POD**. |

### `ErrorState` (`state.rs:1426‑1454`)
| field | disp |
|---|---|
| `help_line:[*const c_char;6]`, `max_reg_help_line` | **copy‑safe locator** — all entries point at `c"..."` rodata (`format_io.rs:222`, `engine.rs:1249`); `'static`, copy the raw pointers verbatim. |
| `randoms[55],j_random`, `two_to_the[31],spec_log[29],dig[23],trick_buf[256]`, `random_seed`, `interaction: InteractionMode`, all counters/flags | **POD**. The captured `randoms`/`random_seed` reproduce `\uniformdeviate` exactly (determinism, §7.4). |

### `SynctexCtxt` (`state.rs:79‑96`)
| field | disp |
|---|---|
| `file: usize` | **journal / route‑to‑null** — preview disables synctex (`synctex_enabled` config); route to null, do not rewind. |
| `root_name: *mut c_char` | **rebuild** — owned C string; store owned copy, re‑`strdup` on restore (its lifecycle is inside synctex.rs; snapshot must not alias). |
| `recorder: Option<Recorder>` | **POD** (fn‑ptr, copy‑safe). |
| all scalars + `flags: SynctexFlags` | **POD**. |

### `TexmfState` (`state.rs:131‑136`)
| field | disp |
|---|---|
| `last_source_name: *mut c_char` | **rebuild** — owned C string; store owned copy, re‑`strdup` on restore. |
| `last_lineno`, `start_time_str:[u8;30]` | **POD**. |

### Module‑level statics (not in `EngineState`)
| item | disp |
|---|---|
| `ENGINE_STATE_PTR` (`state.rs:1613`) | **locator** — re‑install after restore via `set_engine_state_ptr` (§5). Not part of the snapshot. |
| `font_ffi_mirror::{FONT_SIZE,FONT_AREA,FONT_LAYOUT_ENGINE}` (`globals.rs:241‑243`) | **rebuild** — `refresh_font_ffi_mirrors` after restore (§5). |

---

## 2. (A) The `Snapshot` type and `snapshot()` / `restore()`

### 2.1 Design invariants

1. **Allocation stability.** `mem.data`, `eqtb.data`, `font_info`, `str_pool`, etc. escape raw pointers: `MemState::as_u16_ptr`/`as_u16_ptr_mut`/`node_addr` (`state.rs:584‑600`) and the `FONT_*` mirrors (`globals.rs:253‑255`, taken via `.as_ptr()`). The documented contract is *"valid past the borrow because the thread‑local owner never reallocates."* Therefore **`restore` must `copy_from_slice` INTO the existing `Vec` buffer, never replace it**, whenever capacity already matches (it always does within one run — capacities are set once at `engine.rs:995‑1110` and never grow). A capacity mismatch is only possible across format‑undump, which never happens between snapshot and restore of the same session; if it ever does, restore reallocates and **must** call the mirror‑refresh path (§5).
2. **Quiescent capture point.** Snapshots are taken at the top of `main_control_in`'s dispatch loop (between tokens), where `save_ptr` may be > 0 but the transient scratch pointers (`loaded_font_mapping`, `current_diagnostic`) are null. The host chooses checkpoint tokens; §7 validates that a snapshot/restore at that point is byte‑identical to never having snapshotted.
3. **The free‑list scalar bundle travels with `mem.data`.** `get_node`/`get_avail`/`free_node` (`mem.rs:132,50,510`) mutate `rover/avail/lo_mem_max/hi_mem_min/mem_end/var_used/dyn_used` in lockstep with the arena. Restoring `data` without these (or vice‑versa) corrupts the allocator. They are captured and restored as one unit.

### 2.2 The `Snapshot` struct

```rust
// state.rs (new section)

/// A cheap, restorable capture of the full engine arena, taken at a quiescent
/// main_control dispatch boundary. Cloneable POD arenas are bounded-copied;
/// FFI-owning resources ride Arc refcounts (fonts, glyph blobs); OS handles are
/// journaled by offset (see io/journal). Not `Send` (raw locators + Arc<[u8]>
/// non-atomic sharing are single-thread only).
pub(crate) struct Snapshot {
    // --- mem arena + its inseparable free-list scalar bundle ---
    mem_lo: Box<[MemoryWord]>,          // data[0 ..= lo_mem_max]
    mem_hi: Box<[MemoryWord]>,          // data[hi_mem_min ..= mem_end]
    mem_scalars: MemScalars,            // lo_mem_max,hi_mem_min,avail,mem_end,rover,var_used,dyn_used,line,cur_synctex_tag,temp_ptr

    // --- other POD arenas, bounded ---
    eqtb_data: Box<[MemoryWord]>,       // [0 ..= eqtb_top]
    eqtb_hash: Box<[B32x2]>,
    eqtb_xeq_level: Box<[u16]>,         // fixed; or share-if-unchanged (see 2.4)
    eqtb_prim: Box<[B32x2]>,
    eqtb_scalars: EqtbScalars,

    save_data: Box<[MemoryWord]>,       // [0 ..= save_ptr]
    save_scalars: SaveScalars,

    str_start: Box<[i32]>,              // [0 ..= str_ptr]
    str_pool: Box<[u16]>,               // [0 .. pool_ptr]
    strings_scalars: StringsScalars,

    font_info: Box<[MemoryWord]>,       // [0 ..= fmem_ptr]
    font_tables: FontTables,            // all Copy-elem font Vecs, cloned
    font_scalars: FontScalars,

    // --- FFI-owning: Arc shares (B) ---
    font_layout_engine: Vec<Option<Arc<LayoutEngineHandle>>>,
    font_mapping: Vec<Option<Arc<MappingHandle>>>,
    glyph_blobs: GlyphBlobArena,        // Arc<[u8]> per node; cheap clone

    // --- remaining substates: whole-struct POD copies ---
    scanner: ScannerSnapshot,
    input: InputSnapshot,               // POD parts + journaled handles (4)
    hyph: HyphSnapshot,                 // or Arc-shared if trie unchanged (2.4)
    linebreak: LinebreakState,          // Clone-derived POD
    mainloop: MainLoopState,
    shipout: ShipoutSnapshot,           // POD parts; sinks journaled/null
    errors: ErrorState,                 // POD; help_line ptrs copy-safe
    synctex: SynctexSnapshot,           // POD + owned root_name copy
    texmf: TexmfSnapshot,               // POD + owned last_source_name copy
    config: ConfigState,

    // --- journaling side-channel (4) ---
    io_journal: IoJournal,
}
```

Rather than mirror every scalar by hand, give each sub‑state a `#[derive(Clone)]`‑able POD "snapshot" companion, or — simpler and less error‑prone — derive `Clone` on the *pure‑POD* sub‑states directly and store the clone. Specifically:

- `MainLoopState`, `LinebreakState`, `ScannerState`(minus `native_text`/`max_reg_help_line`), `ErrorState`, `ConfigState`, `HyphState`, `ShipoutState`(minus handles) are POD once the raw‑pointer/handle fields are split out. The cleanest edit is:
  - Add `#[derive(Clone)]` to `MainLoopState`, `LinebreakState`, `ConfigState` (already `Copy`), `ErrorState` (raw pointers are copy‑safe rodata → `Clone` is fine), `HyphState`. These have **no owning pointers**, so a derived `Clone` is a correct deep copy.
  - For `ScannerState`, `ShipoutState`, `InputClusterState`, `SynctexCtxt`, `TexmfState`, `FontState`, `MemState`, `EqtbState`, `SaveStackState`, `StringsState` — do **not** derive `Clone`; use the bespoke bounded‑copy / Arc / journal logic. This is what keeps double‑free impossible: the compiler cannot accidentally clone an owning pointer because those structs remain non‑`Clone`.

### 2.3 `snapshot()` body (bounded copy of the hot arenas)

```rust
impl EngineState {
    pub(crate) fn snapshot(&self) -> Snapshot {
        // Quiescent-point assertions (debug): transient scratch must be idle.
        debug_assert!(self.fonts.loaded_font_mapping.is_null());
        debug_assert!(self.shipout.current_diagnostic.is_null());

        let m = &self.mem;
        // Bounded mem copy: lo region [0 ..= lo_mem_max], hi region
        // [hi_mem_min ..= mem_end]. The middle is the unused free gap.
        let mem_lo = m.data[..=(m.lo_mem_max as usize)].to_vec().into_boxed_slice();
        let mem_hi = m.data[(m.hi_mem_min as usize)..=(m.mem_end as usize)]
            .to_vec().into_boxed_slice();

        let e = &self.eqtb;
        let eqtb_data = e.data[..=(e.eqtb_top as usize)].to_vec().into_boxed_slice();

        let s = &self.strings;
        let str_start = s.str_start[..=(s.str_ptr as usize)].to_vec().into_boxed_slice();
        let str_pool  = s.str_pool[..(s.pool_ptr as usize)].to_vec().into_boxed_slice();

        let f = &self.fonts;
        let font_info = f.font_info[..=(f.fmem_ptr as usize)].to_vec().into_boxed_slice();

        Snapshot {
            mem_lo, mem_hi,
            mem_scalars: MemScalars::from(m),
            eqtb_data,
            eqtb_hash: e.hash.clone().into_boxed_slice(),
            eqtb_xeq_level: e.xeq_level.clone().into_boxed_slice(),
            eqtb_prim: e.prim.clone().into_boxed_slice(),
            eqtb_scalars: EqtbScalars::from(e),
            save_data: self.save_stack.data[..=(self.save_stack.save_ptr as usize)]
                .to_vec().into_boxed_slice(),
            save_scalars: SaveScalars::from(&self.save_stack),
            str_start, str_pool,
            strings_scalars: StringsScalars::from(s),
            font_info,
            font_tables: FontTables::capture(f),
            font_scalars: FontScalars::from(f),

            // (B): Arc refcount bumps — O(font_ptr), not O(bytes).
            font_layout_engine: f.font_layout_engine.clone(),
            font_mapping: f.font_mapping.clone(),
            glyph_blobs: self.glyph_blobs.clone(),   // clones Arc<[u8]> handles

            scanner: ScannerSnapshot::capture(&self.scanner),
            input: InputSnapshot::capture(&self.input),
            hyph: HyphSnapshot::capture(&self.hyph),
            linebreak: self.linebreak.clone(),
            mainloop: self.mainloop.clone(),
            shipout: ShipoutSnapshot::capture(&self.shipout),
            errors: self.errors.clone(),
            synctex: SynctexSnapshot::capture(&self.synctex),
            texmf: TexmfSnapshot::capture(&self.texmf),
            config: self.config,
            io_journal: IoJournal::capture(&self.input, &self.shipout, &self.synctex),
        }
    }
}
```

`MemScalars` and friends are trivial `#[derive(Clone,Copy)] struct` + `From<&MemState>` shims; they exist to make the free‑list bundle atomic and impossible to omit.

### 2.4 Optional share‑if‑unchanged (cheaper, still correct)

`eqtb.xeq_level`, `eqtb.prim`, and the entire `HyphState` trie are almost never mutated after format load. Wrap each in `Arc<[T]>` / `Arc<...>` inside `EngineState` and store the `Arc` in the snapshot (refcount bump). On any *write* to those arenas, use `Arc::make_mut` (COW). This is a follow‑on optimization; the baseline `.clone()` above is correct today. I flag it because it removes ~2 MB (`xeq_level`) and the multi‑MB hyph trie from every snapshot for free.

### 2.5 `restore()` body (in‑place, allocation‑stable)

```rust
impl EngineState {
    pub(crate) fn restore(&mut self, s: &Snapshot) {
        // --- mem: overwrite the two live regions IN PLACE, never realloc. ---
        // The middle gap is stale but never read (allocator bounds it by
        // lo_mem_max/hi_mem_min), so we don't need to touch it.
        let m = &mut self.mem;
        s.mem_scalars.apply(m);                       // scalars FIRST define bounds
        m.data[..s.mem_lo.len()].copy_from_slice(&s.mem_lo);
        let hi0 = s.mem_scalars.hi_mem_min as usize;
        m.data[hi0..hi0 + s.mem_hi.len()].copy_from_slice(&s.mem_hi);

        // --- eqtb ---
        s.eqtb_scalars.apply(&mut self.eqtb);
        self.eqtb.data[..s.eqtb_data.len()].copy_from_slice(&s.eqtb_data);
        self.eqtb.hash.copy_from_slice(&s.eqtb_hash);
        self.eqtb.xeq_level.copy_from_slice(&s.eqtb_xeq_level);
        self.eqtb.prim.copy_from_slice(&s.eqtb_prim);

        // --- save stack, strings, font_info (bounded, in place) ---
        s.save_scalars.apply(&mut self.save_stack);
        self.save_stack.data[..s.save_data.len()].copy_from_slice(&s.save_data);
        s.strings_scalars.apply(&mut self.strings);
        self.strings.str_start[..s.str_start.len()].copy_from_slice(&s.str_start);
        self.strings.str_pool[..s.str_pool.len()].copy_from_slice(&s.str_pool);
        s.font_scalars.apply(&mut self.fonts);
        self.fonts.font_info[..s.font_info.len()].copy_from_slice(&s.font_info);
        s.font_tables.apply(&mut self.fonts);

        // --- (B) FFI-owning: assign Arc vecs. Dropping the working state's
        // Arcs here frees any engine/mapping/blob whose refcount hits 0 and is
        // NOT held by this snapshot; anything the snapshot also holds stays live.
        self.fonts.font_layout_engine = s.font_layout_engine.clone();
        self.fonts.font_mapping        = s.font_mapping.clone();
        self.glyph_blobs               = s.glyph_blobs.clone();
        self.fonts.rebuild_layout_engine_mirror(); // flat Vec<*mut c_void> for FFI

        // --- remaining substates ---
        s.scanner.apply(&mut self.scanner);
        s.input.apply(&mut self.input);      // POD parts; handles via journal below
        s.hyph.apply(&mut self.hyph);
        self.linebreak = s.linebreak.clone();
        self.mainloop  = s.mainloop.clone();
        s.shipout.apply(&mut self.shipout);
        self.errors    = s.errors.clone();
        s.synctex.apply(&mut self.synctex);  // re-strdup root_name
        s.texmf.apply(&mut self.texmf);      // re-strdup last_source_name
        self.config    = s.config;

        // --- (C) re-seek journaled input handles to captured offsets ---
        s.io_journal.apply(&mut self.input, &mut self.shipout, &mut self.synctex);

        // --- (D) refresh escaped-pointer mirrors + locator ---
        crate::globals::refresh_font_ffi_mirrors(&self.fonts);
        crate::state::set_engine_state_ptr(self as *mut _);
    }
}
```

**Note on the `set_engine_state_ptr` line:** `restore` takes `&mut self`, but `self` is the same `Box<EngineState>` owned by `tt_run_engine` for the whole session, so its address is stable; re‑installing is only strictly needed if the host moved the box. It is cheap and defensive to always re‑install.

---

## 3. (B) FFI‑ownership refactor

### 3.B.1 Arc‑shared font layout engines and TECkit mappings

**Current shape.** `FontState.font_layout_engine: Vec<*mut c_void>` (`state.rs:903`) and `font_mapping: Vec<*mut c_void>` (`state.rs:907`); freed per slot at `engine.rs:1453‑1464`:
```rust
for font_k in 0..st.fonts.font_max {
    let engine = st.fonts.font_layout_engine[font_k as usize];
    if !engine.is_null() {
        crate::ext::release_font_engine(engine, st.fonts.font_area[font_k as usize]);
        ...
    }
    let mapping = st.fonts.font_mapping[font_k as usize];
    if !mapping.is_null() { tt_cleanup_TECkit_DisposeConverter(mapping); ... }
}
```
Read back by the OT‑math FFI mirror `tt_get_font_layout_engine_entry` (`globals.rs:276`) via `font_ffi_mirror::FONT_LAYOUT_ENGINE`.

**New shape.** Refcounted owning wrappers with `Drop` glue that calls the exact same disposers exactly once at refcount 0:

```rust
// state.rs — new
use std::sync::Arc;

/// Owns one XeTeXLayoutEngine handle; releases it once when the last Arc drops.
/// Carries `font_area` because `release_font_engine` needs it (engine.rs:1456).
pub(crate) struct LayoutEngineHandle {
    raw: *mut libc::c_void,
    font_area: i32,
}
impl Drop for LayoutEngineHandle {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            // SAFETY: sole owner at refcount 0; handle came from find_native_font.
            unsafe { crate::ext::release_font_engine(self.raw, self.font_area); }
        }
    }
}
impl LayoutEngineHandle {
    #[inline] pub(crate) fn raw(&self) -> *mut libc::c_void { self.raw }
}

/// Owns one TECkit_Converter; disposes it once when the last Arc drops.
pub(crate) struct MappingHandle { raw: *mut libc::c_void }
impl Drop for MappingHandle {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe { crate::ext::teckit_dispose_converter(self.raw); }
        }
    }
}
```

`FontState` fields change to:
```rust
pub font_layout_engine: Vec<Option<Arc<LayoutEngineHandle>>>,
pub font_mapping:       Vec<Option<Arc<MappingHandle>>>,
/// Flat raw mirror of `font_layout_engine[f].raw()` (or null), rebuilt on every
/// slot change; the only thing the borrow-free OT-math FFI callback reads.
pub font_layout_engine_raw: Vec<*mut libc::c_void>,
```

Accessor rewrites (`state.rs:1055,1291`):
```rust
#[inline]
pub(crate) fn font_layout_engine_ptr(&self, f: i32) -> *mut libc::c_void {
    self.font_layout_engine[f as usize].as_ref().map_or(std::ptr::null_mut(), |h| h.raw())
}
#[inline]
pub(crate) fn set_font_layout_engine(&mut self, f: i32, raw: *mut libc::c_void) {
    let fa = self.font_area[f as usize];
    self.font_layout_engine[f as usize] =
        (!raw.is_null()).then(|| Arc::new(LayoutEngineHandle { raw, font_area: fa }));
    self.font_layout_engine_raw[f as usize] = raw;   // keep flat mirror in step
}
#[inline]
pub(crate) fn rebuild_layout_engine_mirror(&mut self) {
    for (i, slot) in self.font_layout_engine.iter().enumerate() {
        self.font_layout_engine_raw[i] = slot.as_ref().map_or(std::ptr::null_mut(), |h| h.raw());
    }
}
```
`refresh_font_ffi_mirrors` (`globals.rs:249`) changes one line to point the mirror at the flat vec:
```rust
font_ffi_mirror::FONT_LAYOUT_ENGINE = fonts.font_layout_engine_raw.as_ptr();
```
`tt_get_font_layout_engine_entry` (`globals.rs:276`) is **unchanged** — still reads the flat `*mut c_void` mirror.

**Cleanup rewrite** (`engine.rs:1453‑1464`): the per‑slot free loop is **deleted**; teardown becomes just dropping the Vecs (which drops the Arcs, which run the disposers at refcount 0):
```rust
// tt_cleanup_in — replaces the old loop:
st.fonts.font_layout_engine = Vec::new();   // last Arc drop → release_font_engine
st.fonts.font_mapping        = Vec::new();   // last Arc drop → dispose_converter
st.fonts.font_layout_engine_raw = Vec::new();
```
(Any outstanding `Snapshot`s the host still holds also own Arcs; the handle survives until *both* the live state and every snapshot drop it — exactly the desired "a snapshot keeps its fonts alive" semantics.)

**Reload path** (`fonts.rs:1079`, the "on reload fonts" free): replace the manual free with `set_font_layout_engine(f, new_raw)` — assigning the new Arc drops the old one, freeing it iff no snapshot references it. This preserves the current "reload frees the old engine" behavior in the non‑snapshot case and makes it snapshot‑safe.

`ext::teckit_dispose_converter` is a thin safe wrapper renaming the current `tt_cleanup_TECkit_DisposeConverter` (`engine.rs:1461`) so the disposer is callable from `MappingHandle::drop`.

**Double‑free / UAF proof (fonts).** Each raw handle is owned by exactly one `Arc<Handle>`; the disposer runs only in `Handle::drop`, only at strong‑count 0. `snapshot` bumps the count (`Vec::clone` of `Option<Arc<_>>`); `restore`/reload/cleanup replace or drop the Vec, decrementing. The disposer therefore runs exactly once, when the last of {live state, all snapshots} releases the slot. No path frees a raw pointer directly anymore, so no double free is representable. The flat mirror holds a *non‑owning* copy of the raw pointer, refreshed on every mutation and after restore, so it never points at a freed handle while the OT‑math callback can run (the callback runs synchronously inside a font operation on the live state, never against a snapshot). ∎

### 3.B.2 Move glyph blobs OUT of the `mem` union

**Current shape.** `mem[p+5].ptr` (`MemoryWord.ptr`) holds an `xcalloc`'d glyph‑info buffer:
- **Writers:** `ext.rs:1586`, `ext.rs:1631` (`(*node.add(5)).ptr = glyph_info` in `measure_native_node`); `mem.rs:544‑548` (`copy_native_glyph_info` → `xmalloc` + `set_ptr(dest+5)`); `fonts.rs:70,100` (init to null); `node.rs:2226` (`set_native_glyph_info_ptr`).
- **Readers:** `ext.rs:1667,1712,1730,1802,2265` (raw `(*node.add(5)).ptr` in `measure_native_node`/`real_get_native_glyph`/`…_italic_correction`/`…_word_cp`/`store_justified_native_glyphs`); `shipout.rs:2214` (`p.native_glyph_info_ptr`); `node.rs:2222` (`native_glyph_info_ptr`); `fonts.rs:1146`.
- **Frees:** `packaging.rs:562` (`libc::free(glyph_ptr)` when flushing a native‑word node); `fonts.rs:1148` (temp space‑char node during font load).
- **All callers of the readers pass `mem.node_addr(p)`** (verified: `math.rs:1726,2388,2767,3412`; `main_control.rs:3375,3377,5496…`; `linebreak.rs:1009,3847,3931,3978`; `packaging.rs:2245`; `shipout.rs:954`). **No C/C++ caller** — every reader is a Rust `unsafe fn` and the C++ shims operate only on font handles + glyph ids (verified against ground truth). So this migration is entirely in‑Rust.

**New shape.** A side arena of refcounted immutable blobs, addressed by an index stored in the (now pure‑POD) mem word:

```rust
// state.rs — new (owned by EngineState, sibling to `mem`)
use std::sync::Arc;

/// Owns the shaped-glyph-info buffers that used to live in `mem[p+5].ptr`.
/// Each native-word node stores a 1-based slab index in mem[p+5].b32.s0
/// (0 = none). Blobs are Arc<[u8]> so snapshot is a refcount bump and a
/// post-snapshot in-place mutation (justification) copies only the touched
/// blob (Arc::make_mut COW).
#[derive(Default)]
pub(crate) struct GlyphBlobArena {
    slab: Vec<Option<Arc<[u8]>>>,
    free: Vec<u32>,
}
impl Clone for GlyphBlobArena {           // cheap: clones Arc handles, not bytes
    fn clone(&self) -> Self {
        GlyphBlobArena { slab: self.slab.clone(), free: self.free.clone() }
    }
}
impl GlyphBlobArena {
    /// Store a freshly shaped buffer; returns the 1-based handle to write into mem.
    pub(crate) fn insert(&mut self, bytes: Arc<[u8]>) -> i32 {
        let idx = if let Some(i) = self.free.pop() {
            self.slab[i as usize] = Some(bytes); i
        } else {
            self.slab.push(Some(bytes)); (self.slab.len() - 1) as u32
        };
        (idx + 1) as i32
    }
    #[inline] pub(crate) fn get(&self, h: i32) -> Option<&[u8]> {
        if h == 0 { return None; }
        self.slab[(h - 1) as usize].as_deref()
    }
    /// COW view for in-place justification writes.
    pub(crate) fn get_mut(&mut self, h: i32) -> Option<&mut [u8]> {
        if h == 0 { return None; }
        self.slab[(h - 1) as usize].as_mut().map(|a| &mut **Arc::make_mut(a))
    }
    /// Release a node's blob (returns the slot to the free list).
    pub(crate) fn remove(&mut self, h: i32) {
        if h == 0 { return; }
        let i = (h - 1) as u32;
        if self.slab[i as usize].take().is_some() { self.free.push(i); }
    }
    /// COW clone for copy_native_glyph_info: same Arc, new handle.
    pub(crate) fn dup(&mut self, h: i32) -> i32 {
        match self.get_arc(h) { Some(a) => self.insert(a), None => 0 }
    }
    fn get_arc(&self, h: i32) -> Option<Arc<[u8]>> {
        if h == 0 { return None; }
        self.slab[(h - 1) as usize].clone()
    }
}
```

`Arc::make_mut` note: `&mut **Arc::make_mut(a)` needs `Arc<[u8]>: Clone` of the slice contents — `make_mut` requires `T: Clone`; for `[u8]` unsized use `Arc<[u8]>` → prefer storing `Arc<Vec<u8>>` (sized, `make_mut`‑able) or a small newtype `Blob(Box<[u8]>)`. Concretely store `Arc<Blob>` where `struct Blob(Box<[u8]>)` derives `Clone`; then `Arc::make_mut(a).0` is the writable `[u8]`. This keeps COW type‑clean.

**mem word semantics.** `mem[p+5]` now holds `b32.s0 = blob handle (i32, 0=none)`. `MemoryWord` loses its only pointer interpretation *in practice* (the union still declares `.ptr` for FFI‑layout parity, `mem_types.rs:53`, but no live code stores a pointer there). This is what makes `mem.data` a pure‑`Copy` byte array and unblocks bounded copy / future COW.

**Reader/writer migration (exhaustive edit list):**

| site | current | new |
|---|---|---|
| `node.rs:2222` `native_glyph_info_ptr(&mem)` | returns `mem.ptr(p+5)` | rename → `native_glyph_blob(&self, arena)` returning `Option<&[u8]>` via `arena.get(mem.b32s0(p+5))`; add `native_glyph_blob_handle(&mem)->i32` = `mem.b32s0(p+5)`. |
| `node.rs:2226` `set_native_glyph_info_ptr` | `mem.set_ptr(p+5, v)` | `set_native_glyph_blob_handle(&mut mem, h:i32)` = `mem.set_b32s0(p+5, h)`. |
| `ext.rs:1586,1631` `(*node.add(5)).ptr = glyph_info` | raw xcalloc ptr | build `Arc<Blob>` from the shaped bytes, `arena.insert(...)`, store handle in `mem[p+5].b32.s0`. `measure_native_node` gains `&mut GlyphBlobArena` + node index params (drop the raw `*mut c_void node`). |
| `ext.rs:1667,1712,1730,1802` readers | `(*node.add(5)).ptr as *const u8` | `arena.get(handle)` → `&[u8]`; same pointer arithmetic over the slice. |
| `ext.rs:2265` `store_justified_native_glyphs` write | `(*node.add(5)).ptr as *mut u8` | `arena.get_mut(handle)` (COW) → `&mut [u8]`. |
| `mem.rs:535` `copy_native_glyph_info` | `xmalloc`+copy+`set_ptr(dest+5)` | `let h = arena.dup(src_handle); mem.set_b32s0(dest+5, h)` — Arc share, no byte copy (COW on later mutation). |
| `packaging.rs:559‑564` free | `libc::free(glyph_ptr)` | `arena.remove(handle); set handle 0`. |
| `fonts.rs:1146‑1150` free | `libc::free(glyph_info)` | `arena.remove(handle)`. |
| `fonts.rs:70,100` init | `set_ptr(q+5, null)` | `set_b32s0(q+5, 0)`. |
| `shipout.rs:2214` read | `native_glyph_info_ptr` | `arena.get(handle)`. |

Every reader currently receives `mem.node_addr(p)`; the migration threads `(node_index: i32, &GlyphBlobArena)` instead. Signatures change from `unsafe fn real_get_native_glyph(p_node: *mut c_void, index: u32)` to `fn real_get_native_glyph(mem: &MemState, arena: &GlyphBlobArena, p: i32, index: u32)`. The 12 call sites listed above are updated 1:1 (they already hold `p` and `&st.mem`/`&st.glyph_blobs`).

**Double‑free / UAF proof (blobs).** After migration `mem.data` contains no owning pointer, so a byte‑copy of `mem.data` (snapshot) duplicates only an integer handle — never a heap pointer. The blob bytes are owned solely by `Arc<Blob>` in `GlyphBlobArena`; `snapshot` clones the arena = Arc refcount bumps; `remove` drops one Arc (frees bytes iff no snapshot holds them); justification uses `make_mut` so a post‑snapshot in‑place edit clones the one touched blob, leaving the snapshot's copy intact. `libc::free` on a glyph blob is deleted everywhere. Hence no double free and no snapshot‑visible mutation. ∎

---

## 4. (C) File‑handle journaling

Offsets are **not** stored today. Add a journal captured at snapshot and re‑applied at restore. Preview policy: **input** handles are rewound to their captured read offset; **output** sinks (`dvi_file`, `log_file`, `write_file[]`, `synctex.file`) are *not* rewound — for live preview the host routes them to null (`ttstub_output_open` on `/dev/null` or a discard sink) and re‑opens fresh on each preview pass, because DVI/log are append‑only and cannot seek backward meaningfully.

```rust
// io/journal.rs — new
pub(crate) struct HandleOffset { pub handle: usize, pub offset: libc::size_t, pub ufile: crate::io::UFile }

#[derive(Default)]
pub(crate) struct IoJournal {
    input_file: Vec<Option<HandleOffset>>,     // by input level, in_open entries
    read_file:  Vec<Option<HandleOffset>>,     // 16 \read streams
    // output sinks: only the handle ids + open flags, no offsets (append-only)
    dvi_file: usize, log_file: usize, write_file: [usize;16], write_open: [bool;18],
    synctex_file: usize,
}

impl IoJournal {
    pub(crate) fn capture(input: &InputClusterState, ship: &ShipoutState, syn: &SynctexCtxt) -> Self {
        let cap = |h: *mut crate::io::UFile| -> Option<HandleOffset> {
            (!h.is_null()).then(|| unsafe {
                let handle = (*h).handle;
                // SEEK_CUR with 0 returns current position without moving it.
                let offset = crate::io::input_tell(handle);
                HandleOffset { handle, offset, ufile: std::ptr::read(h) } // POD 32-byte body
            })
        };
        let input_file = input.input_file.iter().map(|&h| cap(h)).collect();
        let read_file  = input.read_file.iter().map(|&h| cap(h)).collect();
        IoJournal { input_file, read_file,
            dvi_file: ship.dvi_file, log_file: ship.log_file,
            write_file: ship.write_file, write_open: ship.write_open,
            synctex_file: syn.file }
    }

    pub(crate) fn apply(&self, input: &mut InputClusterState, ship: &mut ShipoutState, syn: &mut SynctexCtxt) {
        // Re-seek every live input handle to its captured byte offset and
        // restore the UFile decode state (saved_char / skip_next_lf / encoding).
        for (slot, saved) in input.input_file.iter().zip(&self.input_file) {
            if let (false, Some(s)) = (slot.is_null(), saved) {
                unsafe {
                    crate::io::input_seek(s.handle, s.offset as libc::ssize_t, libc::SEEK_SET);
                    **slot /* *mut UFile */ = ... // write back s.ufile POD body
                }
            }
        }
        // read_file: same treatment for open \read streams.
        // Output sinks: host owns re-open; just restore ids/flags for bookkeeping.
        ship.dvi_file = self.dvi_file; ship.log_file = self.log_file;
        ship.write_file = self.write_file; ship.write_open = self.write_open;
        syn.file = self.synctex_file;
    }
}
```

`io::input_tell` is a new thin wrapper mirroring `input_seek` (`io.rs:122`) using `ttbc_input_seek(_, SEEK_CUR, 0)` to read position without side effects. **Line‑boundary seek constraint:** `UFile` buffers a line at a time (`saved_char`, `skip_next_lf`, `io.rs:46‑48`) and the engine reads whole lines via `input_line` (`io.rs:729…`). A raw byte seek must be paired with restoring the `UFile` decode state (the 32‑byte POD body, captured with `std::ptr::read`) so the next `input_line` resumes at the exact byte and decoder phase. Because the checkpoint is taken between tokens at a line boundary in practice, `saved_char == -1` and `skip_next_lf` is settled; the general capture‑and‑restore of the POD body handles the corner cases without assuming a boundary.

For a *pure in‑RAM* preview (the common case: the whole source is a Rust string), input can be a memory buffer instead of a `UFile`, and journaling degenerates to restoring a byte cursor — no `ttbc` call at all. The spec supports both; the host picks.

---

## 5. (D) Mirror refresh and locator re‑install

Two escaped‑pointer surfaces persist across calls and must be re‑established at the end of `restore` (already shown in §2.5):

1. **`refresh_font_ffi_mirrors(&self.fonts)`** (`globals.rs:249`) — repoints `FONT_SIZE/FONT_AREA/FONT_LAYOUT_ENGINE` at the current `fonts.{font_size,font_area,font_layout_engine_raw}.as_ptr()`. Required because `restore` may reassign the `font_layout_engine`/`font_layout_engine_raw` Vecs (Arc reassignment can move the raw mirror Vec if it was replaced; the in‑place `copy_from_slice` path keeps `font_size`/`font_area` stable, but the layout‑engine raw mirror is rebuilt via `rebuild_layout_engine_mirror`, so refresh is mandatory). This is the same invariant `globals.rs:231‑238` documents for reallocation sites; `restore` is a new reallocation site.
2. **`set_engine_state_ptr(self)`** (`state.rs:1623`) — re‑installs `ENGINE_STATE_PTR` for the macOS `font_feature_warning` callback locator. On Linux this is a no‑op read path but keeping the line is correct and free.

`as_u16_ptr`/`node_addr` (`state.rs:584‑600`) are transient (recomputed from `.as_ptr()` on each call), so they need no refresh — but the in‑place restore (never replacing `mem.data`) preserves their documented "owner never reallocates" contract regardless.

---

## 6. Edit list (file / fn → change)

**New code**
- `state.rs`: add `Snapshot`, `MemScalars`/`EqtbScalars`/`SaveScalars`/`StringsScalars`/`FontScalars`, `LayoutEngineHandle`, `MappingHandle`, `GlyphBlobArena` + `Blob`, `impl EngineState { snapshot, restore }`; add field `glyph_blobs: GlyphBlobArena` to `EngineState` (`state.rs:8`); change `FontState.font_layout_engine`/`font_mapping` types and add `font_layout_engine_raw` (`state.rs:903,907`); add `Clone`/capture shims for POD sub‑states.
- `io/journal.rs` (new module): `IoJournal`, `HandleOffset`; `io.rs`: add `input_tell` next to `input_seek` (`io.rs:122`).
- `ext.rs`: add `teckit_dispose_converter` safe wrapper.

**Changed**
- `engine.rs:952‑964` `tt_run_engine`: after building the box, allow the host to call `snapshot`/`restore` between runs (expose via `engine_interface`); the `PtrGuard` stays.
- `engine.rs:1453‑1464` `tt_cleanup_in`: delete the per‑slot font free loop; replace with dropping `font_layout_engine`/`font_mapping`/`font_layout_engine_raw` Vecs.
- `engine.rs:1312‑1348` init: allocate `font_layout_engine`/`font_mapping` as `Vec<None>`, allocate `font_layout_engine_raw`; call `refresh_font_ffi_mirrors` (already there at 1348).
- `globals.rs:255` `refresh_font_ffi_mirrors`: point `FONT_LAYOUT_ENGINE` at `font_layout_engine_raw`.
- `state.rs:1055,1291` accessors: `font_layout_engine_ptr` reads Arc; add `set_font_layout_engine`, `rebuild_layout_engine_mirror`; same for mapping (`state.rs:1083,1295`).
- `fonts.rs:1079` reload: use `set_font_layout_engine`.
- **Glyph‑blob migration (§3.B.2 table):** `node.rs:2222‑2228`; `ext.rs:1586,1631,1667,1712,1730,1802,2265` + `measure_native_node`/`real_get_native_*`/`store_justified_native_glyphs` signatures; `mem.rs:535‑551` `copy_native_glyph_info`; `packaging.rs:559‑567`; `fonts.rs:70,100,1146‑1152`; `shipout.rs:2214`; the 12 caller sites (`math.rs:1726,2388,2767,3412`; `main_control.rs:3375,3377,5496,5520,5600`; `linebreak.rs:1009,3847,3931,3978`; `packaging.rs:2245`; `shipout.rs:954`).

**Unchanged (deliberately):** `tt_get_font_layout_engine_entry`/`tt_get_font_*_entry` (`globals.rs:262‑278`) — still read flat raw mirrors; `MemoryWord` union declaration (`mem_types.rs:47‑54`) keeps `.ptr` for FFI layout parity; `get_node`/`get_avail`/`free_node` allocator (`mem.rs`).

---

## 7. Validation / test strategy (corpus‑gated)

The bar is **byte‑identical XDV vs. baseline**; there is a corpus dashboard (baseline 29/2/7/4, TRIP 2/2 per MEMORY). The refactor is intended to be a pure no‑op on output.

1. **Refactor‑is‑a‑no‑op gate (B first).** Land §3.B.1 and §3.B.2 *without* any snapshot API, run the full corpus + TRIP. Both refactors must produce **byte‑identical XDV** for every corpus doc and pass TRIP (2/2). Because MEMORY notes the dashboard misses native‑font crashes, additionally compare corpus *passed+diverged counts* and NOXDV/NOLOG totals to baseline (not just the divergence list), and specifically re‑run the native‑font / `unicode_math` documents that exercise `measure_native_node`, `store_justified_native_glyphs`, and `copy_native_glyph_info` — those are the glyph‑blob paths. `refresh_font_ffi_mirrors`/OT‑math correctness is only validated by that full corpus (`unicode_math`), per `globals.rs:238`.
2. **Snapshot‑identity property test.** For each corpus doc: run to completion capturing XDV `X0`. Then run again, take `snapshot()` at the first dispatch boundary, immediately `restore()` it, and continue: XDV `X1` must equal `X0` byte‑for‑byte. This proves capture+restore is the identity at a quiescent point.
3. **Prefix‑rewind equivalence.** Split each doc at a token boundary `k`. (a) Run tokens `[0..k]`, `snapshot()`, run `[k..n]` → `Xa`. (b) From the same snapshot, `restore()`, run `[k..n']` for an *edited* tail, then `restore()` again and run the *original* `[k..n]` → `Xb`. Assert `Xa == Xb`. This is the actual live‑preview use pattern and proves a restore fully erases a divergent tail (mem free‑list, save stack, glyph arena, fonts).
4. **Determinism pin.** `random_seed`/`randoms[55]` are captured in `ErrorState` (§1), so `\uniformdeviate`/`\normaldeviate` reproduce across restore. But the seed itself is `microseconds*1000 + epochseconds%1e6` (`engine.rs:1399`, gettimeofday, **not** covered by `deterministic_mode`). For deterministic corpus comparison, drive the parent crate's `force_deterministic` → `UNIX_EPOCH` (`driver.rs:990`) so `X0` is stable run‑to‑run; the snapshot tests (2,3) compare two runs of the *same* process so the live seed is already identical between capture and restore.
5. **Leak / double‑free gate.** Run the snapshot‑heavy property tests (2,3) under ASan/Valgrind on a native‑font doc (fonts + glyph blobs). Expect zero leaks and zero double frees: every `release_font_engine`/`TECkit_DisposeConverter`/blob free must fire exactly once at the last Arc drop. Add a debug refcount assertion in `tt_cleanup_in` that all font Arcs are unique (`Arc::strong_count == 1`) before teardown when no snapshots are outstanding.
6. **Mirror‑drift guard.** A debug‑only assertion after `restore` and after every `set_font_layout_engine` that `font_layout_engine_raw[f] == font_layout_engine[f].map(raw)` for all `f ≤ font_ptr`, catching a missed `rebuild_layout_engine_mirror`.

---

## 8. Correctness proof‑obligations and failure modes

1. **No dangling escaped pointer after restore.** *Obligation:* `restore` must not replace `mem.data`, `eqtb.data`, `save_stack.data`, `font_info`, `str_pool`, `str_start`, `font_size`, `font_area`, or `font_layout_engine_raw` allocations while any `FONT_*` mirror or in‑flight `as_u16_ptr`/`node_addr` pointer aliases them. *Discharge:* §2.5 uses `copy_from_slice` into the pre‑existing Vec (capacity is fixed for the session, `engine.rs:995‑1110`), and re‑runs `refresh_font_ffi_mirrors`. *Failure mode if violated:* silent UB in the OT‑math callback reading a freed mirror — caught only by the full corpus (per `globals.rs:238`), so gate #6 makes it a hard assertion.
2. **Free‑list coherence.** *Obligation:* `mem` scalars and `mem.data` regions restore atomically. *Discharge:* `MemScalars` bundles all ten scalars and is applied before the region copies in `restore`; the copied regions are exactly `[0..=lo_mem_max]` ∪ `[hi_mem_min..=mem_end]` matching the scalars. *Failure mode:* a stale `rover`/`avail` pointing into the un‑restored gap → `get_node` corruption; prevented by atomic bundling.
3. **Exactly‑once FFI disposal.** Discharged in §3.B.1 proof: raw handles are `Arc`‑owned, disposers run only in `Drop` at count 0, no direct frees remain. *Failure mode if a raw free is left behind (e.g. a missed `fonts.rs:1079`):* double free when the last Arc also drops — gate #5 (ASan) catches it.
4. **Glyph‑blob isolation.** Discharged in §3.B.2 proof: `mem` holds only an integer handle; blobs are `Arc<Blob>`; post‑snapshot justification uses `make_mut` (COW). *Failure mode if a reader keeps the old `*mut c_void` path:* a snapshot's blob is mutated in place by a later working‑state justification → divergent rewind; gate #3 (prefix‑rewind) catches it. *Obligation:* verify no `#[no_mangle]` reader of `node+5` exists — verified: `real_get_native_*` are plain `pub unsafe fn`, no C callers.
5. **`null‑at‑rest` fields.** `loaded_font_mapping`, `current_diagnostic`, `synctex` transient scratch must be null/idle at the capture point. *Discharge:* `debug_assert!` in `snapshot` (§2.3). *Failure mode:* capturing a live scratch pointer that a restore re‑installs as a stale alias — the assert makes the checkpoint contract explicit; the host must checkpoint at a dispatch boundary.
6. **Owned C strings never aliased.** `root_name`, `last_source_name`, `name_of_input_file` are `*mut c_char` owned by their modules. *Obligation:* snapshot stores an owned `CString` copy; restore `strdup`s a fresh buffer and frees the working state's current one first (no leak, no shared owner). *Failure mode:* aliasing → double free at the module's own free site (`engine.rs:1484`); prevented by copy‑on‑capture / strdup‑on‑restore.
7. **File offset validity.** *Obligation:* journaled input offsets restore the exact byte + `UFile` decode state. *Discharge:* §4 captures the 32‑byte POD `UFile` body and re‑seeks `SEEK_SET`. *Failure mode:* mid‑UTF‑16‑surrogate or mid‑line resume producing a different token stream → divergent XDV; gate #3 with a multi‑byte‑encoded input file exercises it. For the RAM‑buffer preview path this obligation is vacuous.
8. **`store_fmt_file` interaction.** `store_fmt_file_in` refuses `save_ptr != 0` (`format_io.rs:218`). Snapshot has no such restriction (it captures `save_ptr` and `save_stack.data[..=save_ptr]`), which is *why* snapshots can be taken inside a group where `\dump` cannot — an explicit capability, not a bug. *Obligation:* the bounded `save_data` copy uses `save_ptr+1`, so a captured group nesting restores intact.

---

**Bottom line:** (B) is the gating work — Arc‑refcount the two font handle Vecs and lift glyph blobs into an `Arc<Blob>` side arena, making `mem.data`/`eqtb.data`/`font_info` genuinely POD. Once those two refactors pass the corpus byte‑identically, (A) `snapshot`/`restore` is a mechanical bounded‑copy‑plus‑Arc‑bump with an in‑place restore, and (C)/(D) are the journaling and mirror‑refresh tails. Every double‑free/UAF path is closed by construction (no raw owning pointer survives in a `Copy` arena), and the whole thing is validated by the existing byte‑identical XDV corpus gate plus three snapshot property tests.