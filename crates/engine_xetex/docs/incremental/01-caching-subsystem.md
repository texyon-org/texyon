I now have verified ground truth for every anchor. Writing the spec.

---

# SPEC 1 — Native-Shaping & Font-Engine Caching Subsystem

**Target:** `/home/yendric/Code/tectonic/crates/engine_xetex`
**Scope:** Two process-lifetime caches that eliminate the dominant redundant work in an interactive edit/re-typeset loop **without touching any typesetting algorithm**. Part A caches HarfBuzz/ICU shaping results per (font, text-run). Part B caches the `XeTeXLayoutEngine` + TFM parse per (name,var,feat,size). Both are validated by byte-identical XDV against the corpus baseline.

All file:line anchors below were opened and confirmed against the current tree.

---

## 0. Why this is the highest-leverage cheap win

`measure_native_node` (`src/ext.rs:1473`) is the shaping choke point. It is called on **every** native-word node the engine builds, and it runs the full ICU-BiDi + HarfBuzz layout pipeline (`ubidi_setPara`, `layoutChars`, `getGlyphs`, `getGlyphAdvances`, `getGlyphPositions`) each time. Verified call sites:

| Site | File:line | Context |
|---|---|---|
| new native char | `src/fonts.rs:241` | `new_native_character_in` |
| tail re-measure (merge into prev word) | `src/main_control.rs:5496` | `tail_is_native` branch |
| fresh word | `src/main_control.rs:5520` | main loop word append |
| interword-space probe (temp node) | `src/main_control.rs:5600` | `\XeTeXinterwordspaceshaping` |
| line-break-locale split words | `src/linebreak.rs:3931`, `:3978` | `\XeTeXlinebreaklocale` |
| `set_native_metrics` wrapper | `src/linebreak.rs:1009` | called from line breaker |
| hpack merge re-measure | `src/packaging.rs:2245` | `merge_justifiable_native_words` |
| justification re-measure | `src/ext.rs:2259` | `store_justified_native_glyphs` (shipout, `src/shipout.rs:954`) |

In a live-preview loop the user edits one paragraph; 99% of the document's words are re-shaped to the identical result. Caching the shaping output turns each unchanged word from "full HarfBuzz layout" into "one `xmalloc` + `memcpy` + three field stores."

**Determinism (proof it is pure).** `measure_native_node` reads only: the layout engine handle (immutable after font load), the node's inline UTF-16 payload, `font_letter_space_entry(f)` (`src/state.rs:1060`), and for the metrics branch `getCachedGlyphBBox`/`getGlyphBounds` (a deterministic C glyph-bbox cache) or `height_base_entry`/`depth_base_entry` (`src/state.rs:1145/1150`). It touches **no** date/time source (contrast `src/engine.rs:1399` random-seed, `:1281` date) and **no** RNG. HarfBuzz shaping of a fixed (engine, text, direction) is deterministic. Therefore the function is a pure function of its inputs → cacheable with zero XDV risk, as long as the key captures every input.

---

## 1. The data being cached — exact byte layout

`measure_native_node` writes exactly these node slots (all via the `MemoryWord` raw pointer at `node = p_node as *mut MemoryWord`):

- **width** `(*node.add(1)).b32.s1` — `src/ext.rs:1584` (mixed-BiDi), `:1629` (single-run); post-letterspace fixup `:1652`.
- **glyph_count** `(*node.add(4)).b16.s0` — `src/ext.rs:1585`, `:1630`.
- **glyph_info blob ptr** `(*node.add(5)).ptr` — `src/ext.rs:1586`, `:1631`.
- **height** `(*node.add(3)).b32.s1` — `src/ext.rs:1663` (font-metric branch), `:1701` (glyph-metric branch).
- **depth** `(*node.add(2)).b32.s1` — `src/ext.rs:1664`, `:1702`.

The **blob** (`node+5.ptr`) is `xcalloc`'d at `src/ext.rs:1521`/`:1609` with size `glyph_count * NATIVE_GLYPH_INFO_SIZE` bytes, where `NATIVE_GLYPH_INFO_SIZE = 10` (`src/consts.rs:152`). Its layout, confirmed against the writer (`src/ext.rs:1525-1526`, `:1613-1614`) and reader (`real_get_native_glyph`, `src/ext.rs:1709-1719`):

```
byte 0                         glyph_count*8                 glyph_count*10
| FixedPoint[glyph_count]     | u16 glyph_ids[glyph_count]  |
| (x:i32, y:i32) each = 8 B   | 2 B each                    |
```

`FixedPoint` is `#[repr(C)] { x: i32, y: i32 }` (`src/ext.rs:273`). The glyph-id array starts at byte offset `glyph_count * 8` (see `real_get_native_glyph` `src/ext.rs:1713`, `real_get_native_italic_correction` `src/ext.rs:1731`, `store_justified_native_glyphs` `src/ext.rs:2267`). Total = `glyph_count*8 + glyph_count*2 = glyph_count*10`. This blob is **self-contained and position-independent**: it holds no pointers, only fixed-point coordinates and u16 glyph ids. That is what makes it trivially `memcpy`-cacheable.

The blob is `libc::free`'d at exactly two sites — `src/packaging.rs:562` (node teardown, via `w.native_glyph_info_ptr(mem)` then `w.set_native_glyph_info_ptr(mem, null)`) and `src/fonts.rs:1148` (the probe space node during font load). The canonical clone helper already in-tree is `copy_native_glyph_info` (`src/mem.rs:535`): `xmalloc(byte_count + 1)` then `copy_nonoverlapping`. **The cache-injection path mirrors this exactly**, so ownership semantics (node owns its blob, freed once at `packaging.rs:562`) are preserved.

---

## 2. Part A — Shape Cache

### 2.1 The key: a cross-run-stable font fingerprint + the text run + metric flag

`font_number` (`f`) is **not** stable across runs (fonts are re-loaded in load order each run). The cache key must capture the true shaping inputs:

1. **Font identity** — everything that determines the `XeTeXLayoutEngine`'s shaping: the full spec string used at load (`name/var:feat`), `actual_size`, `loaded_font_letter_space`, and the OT feature flags folded into the engine (embolden/slant/extend/rgb/vertical). All of these are exactly the bytes that were assembled into `st.input.name_of_file` in `find_native_font_in` (`src/ext.rs:1149-1193`) plus `font_size[f]` (`src/state.rs:934`) plus `font_letter_space[f]` (`src/state.rs:925`). We fingerprint them **once at font load** into a `u64` and store it in a new parallel table.
2. **The UTF-16 run** — `native_length` u16 units starting at `node+6` (`native_text_ptr`, `src/node.rs:2233`).
3. **`use_glyph_metrics`** — because the height/depth branch (`src/ext.rs:1662`) differs by this flag; folding it into the key lets us store height/depth directly.

```rust
// src/shape_cache.rs  (new file)

/// Stable per-font shaping fingerprint, computed once at native-font load.
/// Equal fingerprint  <=>  identical XeTeXLayoutEngine shaping behavior.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FontFingerprint(pub u64);

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ShapeKey {
    pub font: FontFingerprint,
    pub use_glyph_metrics: bool,
    pub text: Box<[u16]>,          // exact UTF-16 run, native_length units
}

pub struct ShapeEntry {
    pub width: i32,               // node+1.b32.s1  (post letter-space)
    pub height: i32,              // node+3.b32.s1
    pub depth: i32,               // node+2.b32.s1
    pub glyph_count: u16,         // node+4.b16.s0
    pub blob: Box<[u8]>,          // glyph_count*NATIVE_GLYPH_INFO_SIZE bytes,
                                  // canonical (cache-owned); node gets a fresh copy
    pub cost: u32,                // approx work saved, for eviction accounting
}
```

Fingerprint construction (called from `load_native_font_in`, see edit §2.5):

```rust
impl FontFingerprint {
    /// `name_bytes` = the exact `name_of_file` bytes used to build the engine
    /// (name/var:feat, src/ext.rs:1149-1193); `size`,`letter_space` per §1 tables.
    pub fn compute(name_bytes: &[u8], size: i32, letter_space: i32, flags: i8) -> Self {
        use std::hash::{Hash, Hasher};
        let mut h = ahash::AHasher::default();   // or std SipHasher; must be fixed-seed
        name_bytes.hash(&mut h);
        size.hash(&mut h);
        letter_space.hash(&mut h);
        flags.hash(&mut h);
        FontFingerprint(h.finish())
    }
}
```

> Note on hashing: use a **fixed-seed** hasher (ahash with a constant key, or `std::hash::SipHasher::new_with_keys(0,0)` behind a wrapper). The fingerprint only needs intra-process stability, but a fixed seed also makes cache behavior reproducible in tests. Collisions are handled because the `ShapeKey` also stores the full `text`; a fingerprint collision at most causes a wrong-font hit — see the collision proof-obligation in §2.7. To make that impossible, prefer storing the fingerprint **plus** `size` in the key struct rather than only the hash (belt-and-suspenders; cheap).

### 2.2 The cache container

```rust
pub struct ShapeCache {
    map: hashbrown::HashMap<ShapeKey, ShapeEntry>,
    bytes: usize,           // running sum of blob + text bytes
    byte_limit: usize,      // default 128 MiB, configurable
    // simple generational eviction ring (see §2.6)
    order: std::collections::VecDeque<ShapeKey>,
}

impl ShapeCache {
    pub fn get(&mut self, k: &ShapeKey) -> Option<&ShapeEntry> { self.map.get(k) }
    pub fn insert(&mut self, k: ShapeKey, e: ShapeEntry) { /* §2.6 */ }
}
```

**Where it lives.** This is a *process-level* cache (survives across `tt_run_engine` invocations), so it does **not** go into `EngineState` (which is per-run and the snapshot unit). Put it behind a process-global guarded handle, mirroring the existing `ENGINE_STATE_PTR` locator idiom (`src/state.rs:1623`):

```rust
// src/shape_cache.rs
use std::sync::Mutex;
static SHAPE_CACHE: Mutex<Option<ShapeCache>> = Mutex::new(None);

pub fn with_shape_cache<R>(f: impl FnOnce(&mut ShapeCache) -> R) -> R {
    let mut g = SHAPE_CACHE.lock().unwrap();
    f(g.get_or_insert_with(|| ShapeCache::new(default_byte_limit())))
}
```

Single-threaded engine → the `Mutex` is uncontended (one `lock`/call); if even that overhead is unwanted, use `thread_local!` + `RefCell` since `tt_run_engine` is driven on one thread. (The engine already uses raw statics for `ENGINE_STATE_PTR`; a `thread_local` RefCell is strictly safer and adequate.)

### 2.3 The instrumented entry point

Do **not** scatter cache logic across the eight call sites. Wrap `measure_native_node` so every caller benefits with zero churn. Rename the existing body to `measure_native_node_uncached` and add a thin cached front:

```rust
// src/ext.rs — replaces the public fn at 1473
pub unsafe fn measure_native_node(
    fonts: &crate::state::FontState,
    p_node: *mut c_void,
    use_glyph_metrics: c_int,
) -> Result<(), NativeFontAbort> {
    let node = p_node as *mut crate::mem_types::MemoryWord;
    let f = (*node.add(4)).b16.s2 as i32;
    let len = (*node.add(4)).b16.s1 as usize;         // native_length
    let txt = std::slice::from_raw_parts(node.add(6) as *const u16, len);

    // Fingerprint table lookup is O(1); FONT_BASE fonts (non-native) never
    // reach here because the caller only measures WHATSIT native-word nodes.
    let fp = fonts.font_fingerprint(f);                // new accessor, §2.5
    let key = crate::shape_cache::ShapeKey {
        font: fp,
        use_glyph_metrics: use_glyph_metrics != 0,
        text: txt.into(),                              // Box<[u16]> copy
    };

    if let Some(hit) = crate::shape_cache::with_shape_cache(|c| c.get(&key).map(|e| {
        (e.width, e.height, e.depth, e.glyph_count, e.blob.clone())
    })) {
        let (w, h, d, gc, blob) = hit;
        // free any pre-existing blob (mirrors free semantics; measure itself
        // overwrites without freeing — see proof-obligation PA-3)
        let old = (*node.add(5)).ptr;
        if !old.is_null() { libc::free(old); }
        // fresh owned copy into node+5.ptr, exactly like copy_native_glyph_info
        let byte_count = gc as usize * NATIVE_GLYPH_INFO_SIZE;
        let dst = xmalloc(byte_count + 1);
        std::ptr::copy_nonoverlapping(blob.as_ptr(), dst as *mut u8, byte_count);
        (*node.add(5)).ptr = dst;
        (*node.add(4)).b16.s0 = gc;
        (*node.add(1)).b32.s1 = w;
        (*node.add(3)).b32.s1 = h;
        (*node.add(2)).b32.s1 = d;
        return Ok(());
    }

    // MISS: run the real shaper (unchanged body), then snapshot the result.
    measure_native_node_uncached(fonts, p_node, use_glyph_metrics)?;

    let gc = (*node.add(4)).b16.s0;
    let blob_ptr = (*node.add(5)).ptr as *const u8;
    let byte_count = gc as usize * NATIVE_GLYPH_INFO_SIZE;
    let blob: Box<[u8]> = if !blob_ptr.is_null() && byte_count > 0 {
        std::slice::from_raw_parts(blob_ptr, byte_count).to_vec().into_boxed_slice()
    } else {
        Box::new([])
    };
    let entry = crate::shape_cache::ShapeEntry {
        width: (*node.add(1)).b32.s1,
        height: (*node.add(3)).b32.s1,
        depth: (*node.add(2)).b32.s1,
        glyph_count: gc,
        cost: (len as u32).max(1),
        blob,
    };
    crate::shape_cache::with_shape_cache(|c| c.insert(key, entry));
    Ok(())
}
```

`measure_native_node_uncached` = the current body verbatim (lines `1478-1704`), renamed. No logic change.

### 2.4 Interaction with the mutating callers — verified safe

- **`tail_is_native` merge** (`src/main_control.rs:5476-5510`): builds a *new, longer* node (old text + new text), measures it, frees the old node. The key text = concatenated payload → distinct key → correct independent entry. The old node's blob is freed by `free_node` path normally. ✅
- **Interword-space probe** (`src/main_control.rs:5570-5610`): allocates `tmp`, fills `pp_text + ' ' + tail_text`, measures, reads `tmp+1` width, frees `tmp`. Cache hit returns identical width; `t_val` computation is byte-identical. The probe is exactly the kind of repeated work the cache kills. ✅
- **`store_justified_native_glyphs`** (`src/ext.rs:2251`): calls `measure_native_node` with `use_glyph_metrics=0` (line `2259`), *then mutates the blob's `locations[i].x` in place* (lines `2281`, `2289`) and restores width (`2296`). Because the cache injects a **fresh `xmalloc`'d copy** (not the cache's `Box<[u8]>`), the in-place justification writes to the node's private buffer and never touches the cached canonical blob. ✅ This is the single most important ownership invariant — see PA-1.
- **`merge_justifiable_native_words`** (`src/packaging.rs:2245`): flushes a scanned run into one node then measures the merged text. Distinct concatenated key → correct. ✅
- **letter-space fixup** (`src/ext.rs:1640-1655`): already folded into the cached `width` and into the blob's `locations` because it runs *before* we snapshot on a miss, and `font_letter_space` is part of the fingerprint. ✅

### 2.5 Fingerprint table — FontState edit

Add a parallel table to `FontState` and populate it at the two font-load completion points.

```rust
// src/state.rs — FontState struct, near font_letter_space (line 925)
/// Per-font shaping fingerprint (cross-run-stable font identity). NOT
/// format-serialized (recomputed at load). Newtype over u64.
pub font_fingerprint: Vec<crate::shape_cache::FontFingerprint>,
```

Accessor (next to `font_letter_space_entry`, `src/state.rs:1060`):

```rust
pub(crate) fn font_fingerprint(&self, f: i32) -> crate::shape_cache::FontFingerprint {
    self.font_fingerprint[f as usize]
}
pub(crate) fn set_font_fingerprint(&mut self, f: i32, v: crate::shape_cache::FontFingerprint) {
    self.font_fingerprint[f as usize] = v;
}
```

**Population site.** In `load_native_font_in` (`src/fonts.rs:1031`), immediately after the font slot is fully initialized (after `set_font_layout_engine_ptr` at `src/fonts.rs:985` runs inside `init_native_font_tables`, and after `font_flags`/`font_size` are set at `src/fonts.rs:1159`,`:1172`). The exact bytes to fingerprint are `st.input.name_of_file[0..name_length]` (the string assembled in `find_native_font_in`, which already includes `/var:feat`), plus `actual_size` and `loaded_font_letter_space`:

```rust
// src/fonts.rs — after line 1172 (font_flags set), before Ok(font_ptr)
let name_bytes: Vec<u8> =
    st.input.name_of_file[..st.input.name_length as usize]
        .iter().map(|&c| c as u8).collect();
let fp = crate::shape_cache::FontFingerprint::compute(
    &name_bytes,
    actual_size,
    st.fonts.loaded_font_letter_space,
    st.fonts.loaded_font_flags as i8,
);
st.fonts.set_font_fingerprint(font_ptr, fp);
```

The de-dup early-return branch (`src/fonts.rs:1069-1086`) returns an already-loaded `f` whose fingerprint is already set — no action needed. The `font_fingerprint` Vec must be sized alongside every other `font_max+1` table (grep the init that resizes `font_letter_space`/`font_flags` and add `font_fingerprint.resize(font_max+1, Default::default())` at the same spot; also in the format-load table allocation path so a `.fmt`-loaded run gets fresh fingerprints — since fingerprint is recomputed from name/size which *are* serialized, you can also repopulate it in a single post-`load_fmt_file` sweep over `FONT_BASE+1..=font_ptr`).

### 2.6 Eviction & memory bound

Interactive sessions are long-lived; bound the cache. Policy: **byte-budgeted CLOCK/second-chance** (simpler than true LRU, no per-hit bookkeeping cost in the hot path):

```rust
impl ShapeCache {
    pub fn insert(&mut self, k: ShapeKey, e: ShapeEntry) {
        let sz = e.blob.len() + k.text.len() * 2 + std::mem::size_of::<ShapeEntry>();
        while self.bytes + sz > self.byte_limit {
            let Some(victim) = self.order.pop_front() else { break };
            if let Some(old) = self.map.remove(&victim) {
                self.bytes -= old.blob.len() + victim.text.len()*2
                              + std::mem::size_of::<ShapeEntry>();
            }
        }
        self.bytes += sz;
        self.order.push_back(k.clone());
        self.map.insert(k, e);
    }
}
```

Default `byte_limit` = 128 MiB (a full book of native-word blobs is well under this; a Latin word of ~6 glyphs is ~60 B blob + text). Expose via an env var (`TECTONIC_SHAPE_CACHE_BYTES`) read in `default_byte_limit()`; `0` disables (returns miss-only). This keeps the cache invisible to the corpus runner unless deliberately enabled, so baseline behavior is untouched by default (see §4).

### 2.7 Part A proof-obligations / failure modes

- **PA-1 (no shared-blob mutation):** cache-hit injects a fresh `xmalloc` copy; `store_justified_native_glyphs` and any future in-place mutator write only the node's private buffer. Enforced by the `xmalloc + copy_nonoverlapping` in §2.3. *Test:* a doc that triggers `\XeTeXinterwordspaceshaping`-driven justification twice on identical text must produce identical XDV cached vs uncached.
- **PA-2 (key completeness):** every input to `measure_native_node_uncached` is in the key. Inputs enumerated in §0. `height_base_entry`/`depth_base_entry` are font-intrinsic → folded into the fingerprint via font identity (they are set at load from `ot_get_font_metrics`, `src/fonts.rs:970-971`, deterministic per engine). `getCachedGlyphBBox` is a deterministic C cache keyed on `(f as u16, glyph_id)` and only affects the `use_glyph_metrics=1` height/depth which we store per-flag. ✅
- **PA-3 (free semantics):** the real `measure_native_node` overwrites `node+5.ptr` **without freeing** the prior blob (verified: `src/ext.rs:1586`,`:1631` are plain assignments). Our hit path *frees* the old ptr first. This is output-neutral (blob content unused after overwrite) and strictly *reduces* leaks; it does not introduce double-free because the node's single owning slot is updated atomically. If matching the original leak exactly is desired for paranoia, drop the free — but freeing is correct and preferable for a long-running server. *Obligation:* run under ASan/valgrind on the corpus to confirm no double-free and net-lower RSS.
- **PA-4 (fingerprint collision):** a 64-bit collision would let word A of font X hit an entry shaped for font Y. Probability negligible, but to make it *impossible*, store `size:i32` alongside the fingerprint in `ShapeKey` (already proposed) and, for absolute safety in a validation build, key on the full `name_bytes` instead of the hash (feature-gated `shape_cache_strict`). *Obligation:* the corpus run in strict mode must be byte-identical to the hashed mode.
- **PA-5 (native_length overflow):** `native_length` is `u16`; a run is ≤ 65535 units. `text: Box<[u16]>` of that size is fine. Empty runs (`len==0`) produce `glyph_count==0`, `blob==[]`; hit path stores width/height/depth only. ✅

---

## 3. Part B — Font-Engine + TFM Cross-Run Cache

### 3.1 The redundant work

On every run, `load_native_font_in` (`src/fonts.rs:1031`) → `find_native_font_in` (`src/ext.rs:1055`) → `load_ot_font` (`src/ext.rs:800`) rebuilds the platform font (`createFontFromFile`/`createFont`, `src/ext.rs:1088`,`:1100`,`:1153`,`:1170`), re-parses the feature string, and calls `createLayoutEngine` (`src/ext.rs:1026`). That HarfBuzz face + shaper construction is the second-largest fixed cost of a re-typeset. TFM fonts similarly re-parse the metric file on each `\font`.

**Note the existing in-run dedup:** `find_native_font_in`'s caller `load_native_font_in` already de-dups *within a single run* (`src/fonts.rs:1069-1086`: loop over loaded fonts comparing name+size, returning the existing `f` and calling `release_font_engine` on the just-built duplicate). Part B extends that to *across* runs.

### 3.2 Cache design — shared immutable layout engine

```rust
// src/font_engine_cache.rs (new)

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct EngineKey {
    pub spec: Box<[u8]>,   // the full name_of_file bytes (name/var:feat)
    pub size: i32,         // scaled_size resolved (actual_size)
    pub content_digest: u64, // font-file content hash — determinism guard, §3.4
}

pub struct CachedEngine {
    /// XeTeXLayoutEngine — opaque, immutable after creation. Shared read-only
    /// across runs. Refcounted so a run can `deleteLayoutEngine` its clone
    /// only when the process cache evicts.
    engine: *mut libc::c_void,
    /// design size / metrics captured at build so callers need not re-derive.
    design_size: i32,
    letter_space: i32,
    flags: i8,
}
```

**Sharing model & the snapshot interplay (forward ref to Spec 2).** `FontState::font_layout_engine: Vec<*mut c_void>` (`src/state.rs:903`) is freed per-font in `tt_cleanup`/`release_font_engine` (`src/ext.rs:1263`, `deleteLayoutEngine` `src/ext.rs:1271`) and on reload (`src/fonts.rs:1079`). For a *shared* engine we must **not** let per-run teardown delete a cache-owned handle. Two coherent options:

- **Option B1 (refcount, recommended):** wrap each cached engine in an `Arc<EngineHandle>` where `EngineHandle` holds the raw `*mut c_void` and its `Drop` calls `deleteLayoutEngine`. `FontState` stores, in parallel to `font_layout_engine`, an `Option<Arc<EngineHandle>>` per slot that keeps the cache entry alive for the run's duration. Per-run teardown drops the `Arc` (no delete unless it was the last ref); the process cache holds the other `Arc`. This is exactly the ownership model Spec 2 needs for cheap snapshots: the snapshot clones `Arc`s, not engines.
- **Option B2 (cache owns, runs borrow):** the cache owns all engines forever (until eviction); `FontState` slots hold borrowed raw pointers and per-run teardown is taught to **skip** `deleteLayoutEngine` for cache-backed fonts (a per-slot `bool cache_backed` flag). Simpler but the `release_font_engine` call at `src/fonts.rs:1079` and the cleanup at `tt_cleanup` must both consult the flag.

Recommend **B1** because it composes directly with snapshot/restore (Arc clone is the snapshot primitive) and localizes the `deleteLayoutEngine` decision in one `Drop`.

### 3.3 Insertion / lookup edit sites

**Lookup** goes at the top of `find_native_font_in` (`src/ext.rs:1055`), after the name/var/feat split (`splitFontName`, `src/ext.rs:1074`) and after `name_of_file`/`scaled_size` are resolved — but the cleanest seam is *inside* `load_native_font_in` (`src/fonts.rs:1043`), wrapping the `find_native_font_in` call, because that is where `actual_size` (`src/fonts.rs:1048`) and the assembled `name_of_file` both exist and where the in-run dedup already lives:

```rust
// src/fonts.rs — replace the find_native_font_in call at ~1043
let key = crate::font_engine_cache::EngineKey::from_current(st, s)?; // reads name_of_file + digest
if let Some(cached) = crate::font_engine_cache::lookup(&key) {
    // install a shared engine into this run's font slot without rebuilding
    return finish_native_font_from_cached(st, u, nom, aire, s, cached);
}
let font_engine = find_native_font_in(st, name_of_file, s)?;   // MISS: build it
// ... existing body ...
// after the slot is built, publish into the process cache:
crate::font_engine_cache::insert(key, CachedEngine::capture(st, font_ptr));
```

`finish_native_font_from_cached` replicates the *state-registration* tail of `load_native_font_in` (`src/fonts.rs:1069-1173`: name string interning, dedup check, `font_ptr` bump, `push_font_info_*`, `set_font_layout_engine_ptr`, `set_font_fingerprint`) but with the engine handle taken from the `Arc` instead of `find_native_font_in`. Critically, it must **not** re-run `find_native_font_in`'s side effects that mutate `st.fonts.loaded_font_*`; capture those (`loaded_font_letter_space`, `loaded_font_flags`, design size) in `CachedEngine` and restore them into the slot. The `num_font_dimens`/OT-math-constants push (`src/fonts.rs:1163-1168`) must still run because it writes into this run's `font_info` arena — cache the `isOpenTypeMathFont` result and the 56 `get_ot_math_constant` values in `CachedEngine` to avoid re-querying, or just re-query (cheap once the engine exists).

### 3.4 Determinism hazard: font-file mtime / content change

A cross-run cache keyed only on `(name,var,feat,size)` goes **stale** if the underlying font file changes between edits (user swaps a `.ttf`, or a bundle update). This is the one determinism hazard. Mitigations, in order of rigor:

1. **Content digest in the key** (`EngineKey.content_digest`): Tectonic resolves fonts through the bundle / `ttstub` I/O. Compute a cheap digest (size + first/last 4 KiB hash, or the bundle's content id if exposed) at font-open and fold it into the key. A changed file → different digest → miss → rebuild. This is the correct fix and is what `EngineKey` above encodes.
2. **Session epoch:** if digesting is impractical, tag the cache with a monotonically-increasing "font epoch" that the editor bumps whenever it detects a font-set change (fs-watch on font dirs), invalidating the whole engine cache.
3. **TFM parse cache** obeys the same rule: key the parsed metric tables on `(area,name,size,content_digest)`. TFM parsing lives in `fonts.rs` (`read_tfm_word` `src/fonts.rs:253`, the `read_font_info`/`load_tfm_font_mapping_in` path); cache the produced `font_info` slice + the per-font index tables (`char_base`/`width_base`/… `src/state.rs:967-975`) as an immutable `Arc<TfmData>` and memcpy them into the run's arena on hit.

For the corpus (deterministic bundle, no live edits) the digest is constant → cache is always valid → byte-identical. For live editing the digest guarantees correctness across font swaps.

### 3.5 Part B proof-obligations / failure modes

- **PB-1 (no aliasing writes to a shared engine):** `XeTeXLayoutEngine` is read-only during shaping (HarfBuzz `hb_shape` on an immutable face). Confirm no code path mutates the engine post-build (grep `setFontLayoutDir` `src/ext.rs:1023` — called *before* `createLayoutEngine`, on the `XeTeXFont`, not the shared engine; safe). Vertical/embolden/etc. are baked at creation. ✅
- **PB-2 (teardown must not double-delete):** with B1, exactly one `Arc` drop reaching refcount 0 calls `deleteLayoutEngine`. The per-reload `release_font_engine` at `src/fonts.rs:1079` (called on a *duplicate* just-built engine) must operate on a **non-cached** engine — ensure the dedup path builds-then-releases only when it *missed* the cache; if it hit, no fresh engine exists to release. Audit `src/fonts.rs:1079` and `tt_cleanup` (engine.rs:1456) accordingly.
- **PB-3 (FONT_* FFI mirrors):** `globals.rs` FONT_* mirrors point *into* the `font_layout_engine` Vec (per ground-truth §4); after installing a cached engine pointer into a slot, refresh those mirrors exactly as the normal load path does. No new hazard beyond the existing one, but the mirror-refresh must run in `finish_native_font_from_cached`.
- **PB-4 (format dump):** engines are never serialized (`font_layout_engine` is not format-dumped). A `.fmt`-loaded run must rebuild or cache-lookup engines on first use — unchanged, since the cache lookup keys on name/size which *are* in the format.

---

## 4. Validation / test strategy (corpus-gated, byte-identical XDV)

The engine is corpus-gated; the bar is **byte-identical XDV vs baseline**, plus no regression in the corpus dashboard's passed/diverged/NOXDV/NOLOG counts (per project memory: the dashboard misses native-font crashes, so compare *counts*, not just the divergence list).

1. **Default-off gate.** Ship both caches **disabled by default** (`byte_limit==0` / engine-cache env unset) so the untouched corpus run is provably identical to baseline (the wrapper degrades to `measure_native_node_uncached` + the existing in-run font dedup). Land this first; confirm 0 divergences.
2. **Cache-on differential.** Add a test mode env (`TECTONIC_SHAPE_CACHE_BYTES=134217728`, engine-cache on) and re-run the **entire corpus twice in one process** (run doc, keep caches warm, run same doc again) comparing XDV of run-2 to the baseline XDV of run-1. Any single-byte diff = a key-completeness bug (PA-2/PA-4) or ownership bug (PA-1). This is the decisive test: the second run must hit the cache for ~100% of words yet emit identical XDV.
3. **Native-font stress subset.** Curate the corpus docs that exercise: CJK/Arabic BiDi mixed runs (mixed-BiDi branch `src/ext.rs:1502`), `\XeTeXinterwordspaceshaping`, `\XeTeXlinebreaklocale`, justified paragraphs (`store_justified`), OT-math fonts (`num_font_dimens==65`, `src/fonts.rs:1163`), and letterspaced fonts (`font_letter_space != 0`). Run each cached vs uncached; assert XDV byte-identity and NOLOG identity.
4. **Memory safety.** Run the native-font subset under valgrind/ASan with caches on: assert no leaks beyond the intentional process-cache retention, no double-free (PA-3, PB-2), no use-after-free of injected blobs across `store_justified`.
5. **Strict-fingerprint cross-check.** Build with `shape_cache_strict` (key on full name bytes, no hash) and confirm the corpus XDV matches the hashed build → proves no fingerprint collision affected output (PA-4).
6. **Determinism guard.** A regression test that mutates a font file's content between two runs must produce a cache miss (different `content_digest`) and correct new XDV (PB-1/§3.4).
7. **TRIP.** TRIP passes on baseline (per memory, dashboard 29/2/7/4, TRIP 2/2). TRIP uses TFM fonts only, so it exercises the Part B TFM cache path but not shaping; a TRIP failure with caches on = a Part B TFM regression. Keep TRIP green as a hard gate.

---

## 5. Explicit edit list

**New files**
- `src/shape_cache.rs` — `FontFingerprint`, `ShapeKey`, `ShapeEntry`, `ShapeCache`, `with_shape_cache`, `default_byte_limit` (§2.1–2.6).
- `src/font_engine_cache.rs` — `EngineKey`, `CachedEngine`, `EngineHandle`(Arc/Drop→`deleteLayoutEngine`), `lookup`/`insert`/`from_current` (§3.2–3.3).

**`src/ext.rs`**
- `:1473` rename current body → `measure_native_node_uncached`; add cached wrapper `measure_native_node` (§2.3).
- (Part B) no structural change; `find_native_font_in` (`:1055`) stays the miss path.

**`src/state.rs`**
- `FontState` (near `:925`): add `font_fingerprint: Vec<FontFingerprint>` (+ `Default`, + the `font_max+1` resize wherever `font_letter_space`/`font_flags` are sized; also format-load sweep).
- After `:1060`: `font_fingerprint`/`set_font_fingerprint` accessors (§2.5). Optionally an `Arc<EngineHandle>` parallel table for B1.

**`src/fonts.rs`**
- `:1043` wrap `find_native_font_in` with engine-cache lookup; add `finish_native_font_from_cached` (§3.3).
- After `:1172` populate `set_font_fingerprint` (§2.5).
- `:1079` audit `release_font_engine` for cache-backed engines (PB-2).
- TFM path (`read_font_info`/`:253`) add `Arc<TfmData>` cache (§3.4).

**`src/engine.rs`**
- `:1456`/`tt_cleanup` audit so per-run engine teardown drops the `Arc` (B1) or skips cache-backed handles (B2) instead of unconditionally `deleteLayoutEngine`.

**`src/lib.rs` / module root** — `mod shape_cache; mod font_engine_cache;`.

**No changes** to any typesetting algorithm, to `measure_native_node_uncached`'s body, to node layout, or to XDV emission. The cache is a pure memoization layer over a proven-pure function plus a shared-immutable font-engine store, both gated off by default and validated by byte-identical corpus XDV.