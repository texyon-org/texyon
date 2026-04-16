# 02 — State modeling

TeX has a lot of global state. The C engine declares roughly 150
`extern` variables in `xetex-xetexd.h` (lines ~382–650), all defined
in `xetex-ini.c`. This document describes how we migrate that state to
a clean Rust representation **without breaking the build at any point**.

## The state categories

Looking at `xetex-xetexd.h:382–650`, the globals cluster into functional
groups. We will mirror those clusters as Rust sub-structs.

| Cluster         | Representative globals                                         | Rust home              |
|-----------------|----------------------------------------------------------------|------------------------|
| Memory arena    | `mem`, `lo_mem_max`, `hi_mem_min`, `rover`, `avail`, `mem_end` | `state::mem::Mem`      |
| Equivalents     | `eqtb`, `eqtb_top`, `hash`, `hash_used`, `hash_high`, `prim`   | `state::eqtb::Eqtb`    |
| Save stack      | `save_stack`, `save_ptr`, `cur_level`, `cur_group`             | `state::save_stack::SaveStack` |
| Input stack     | `input_stack`, `input_ptr`, `cur_input`, `line`, `line_stack`  | `state::input::InputStack` |
| String pool     | `str_pool`, `str_start`, `pool_ptr`, `str_ptr`                 | `state::string_pool::StringPool` |
| Fonts           | `font_info`, `font_check`, `font_size`, `font_*`               | `state::fonts::Fonts`  |
| Hyphenation     | `trie_trl`, `trie_tro`, `trie_trc`, `hyf_*`, `hyph_*`          | `state::hyph::Hyph`    |
| Page/shipout    | `total_pages`, `max_v`, `max_h`, `cur_h`, `cur_v`, ...         | `state::shipout::ShipoutState` |
| Scanner         | `cur_cmd`, `cur_chr`, `cur_cs`, `cur_tok`, `cur_val`, ...      | `state::scanner::ScannerState` |
| Error/history   | `history`, `error_count`, `interaction`, `deletions_allowed`   | `state::errors::ErrorState` |
| Config          | `halt_on_error_p`, `in_initex_mode`, `shell_escape_enabled`    | `state::config::Config` |

The top-level `EngineState` is then:

```rust
pub struct EngineState {
    pub mem: Mem,
    pub eqtb: Eqtb,
    pub save_stack: SaveStack,
    pub input: InputStack,
    pub strings: StringPool,
    pub fonts: Fonts,
    pub hyph: Hyph,
    pub shipout: ShipoutState,
    pub scanner: ScannerState,
    pub errors: ErrorState,
    pub config: Config,
    // … plus a few odds-and-ends that don't fit a cluster
}
```

## Three migration phases

We cannot migrate all state in one step — we'd have to translate every
function simultaneously. Instead:

### Phase A — compatibility globals (module `src/globals.rs`)

Until any function is translated, the C globals are the only source of
truth. When we start translating Rust functions that read/write those
globals, they do so via a single Rust module:

```rust
// src/globals.rs
use crate::mem_types::MemoryWord;

extern "C" {
    pub(crate) static mut mem: *mut MemoryWord;
    pub(crate) static mut lo_mem_max: i32;
    pub(crate) static mut hi_mem_min: i32;
    pub(crate) static mut rover: i32;
    pub(crate) static mut avail: i32;
    pub(crate) static mut mem_end: i32;
    // ... one line per global
}

// Small typed accessors so the rest of the code never touches `extern`.
//
// SAFETY invariant for this whole module: the tectonic engine holds a
// process-wide lock (bridge_core) while an engine is running, so there
// are no concurrent readers/writers of these C globals. Every accessor
// therefore does a single read-and-return or write-and-return; we
// never hand out a live `&mut` / `&` that could overlap another call.
//
// DO NOT introduce a helper that returns a reference or slice of
// `mem[..]` (e.g. `&'static mut [MemoryWord]`). Two calls would alias
// — undefined behaviour. Always fetch-by-index and commit-by-index.

#[inline]
pub fn mem_word(p: i32) -> MemoryWord {
    // SAFETY: see module docstring.
    unsafe { *mem.offset(p as isize) }
}

#[inline]
pub fn set_mem_word(p: i32, w: MemoryWord) {
    // SAFETY: see module docstring.
    unsafe { *mem.offset(p as isize) = w; }
}

#[inline]
pub fn rover() -> i32 { unsafe { rover } }
#[inline]
pub fn set_rover(v: i32) { unsafe { rover = v; } }
// ... etc
```

Rules for Phase A:

1.  All `extern static mut` declarations live **only** in
    `src/globals.rs`. Nowhere else in the crate imports `extern "C"`
    globals directly.
2.  Accessing the global happens via a thin safe wrapper
    (`rover()` / `set_rover()`). The wrappers are private to `globals.rs`
    and reborrow raw `extern` statics inside an `unsafe` block with a
    `// SAFETY: ...` comment about the single-threaded engine guarantee.
3.  Functions translated in Phase A take no `EngineState` parameter;
    they read and write globals via the wrappers. **They look and feel
    ugly — that's fine.** This phase is a beachhead, not the end state.

### Phase B — state struct behind a thread-local

Once a non-trivial chunk of Rust exists (say, 20–30% of functions
translated), we introduce `EngineState` as the real owner of the
fields, but stash it in a thread-local so that Phase-A-style function
signatures still work:

```rust
// src/state/mod.rs
thread_local! {
    pub static ENGINE: RefCell<Option<EngineState>> = const { RefCell::new(None) };
}

pub fn with_state<R>(f: impl FnOnce(&mut EngineState) -> R) -> R {
    ENGINE.with(|c| f(c.borrow_mut().as_mut().expect("engine not initialized")))
}
```

The globals wrappers in `src/globals.rs` now look like:

```rust
pub fn rover() -> i32 { crate::state::with_state(|s| s.mem.rover) }
pub fn set_rover(v: i32) { crate::state::with_state(|s| s.mem.rover = v); }
```

Note: the C side still holds the old C globals. We keep them in sync
by removing the C definition and replacing it with a Rust
`#[no_mangle]` getter/setter that reflects into the thread-local. In
practice, though, it is easier to do this *only once a given cluster
is entirely Rust-managed*, at which point the C side no longer touches
that cluster and we can delete the C global outright. Don't do it
incrementally on a field-by-field basis within a cluster — atomic
switchover of one cluster at a time is the right granularity.

**Order of clusters to migrate in Phase B** (least-coupled first):

1.  `StringPool` (small, mostly used by `output.rs`)
2.  `ErrorState` (small, no loops)
3.  `Config` (read-only during execution)
4.  `Fonts` (touches `xetex_layout`, but well-scoped)
5.  `ScannerState` (used everywhere — a big step)
6.  `SaveStack` (tied closely to eqtb)
7.  `Eqtb`
8.  `InputStack`
9.  `Mem` (the biggest; leave it for last because everything depends on it)
10. `Hyph`, `ShipoutState`, misc

Each cluster migration is its own session (or few sessions). The
thread-local bridging means the rest of the code keeps compiling and
running throughout.

### Phase C — explicit `&mut EngineState`

Once the thread-local owns everything and the C globals are gone, we
do a refactoring pass that replaces `with_state(|s| ...)` call sites
with explicit `&mut EngineState` parameters. This is where the code
becomes *idiomatic*. It's also largely mechanical: function signatures
grow a parameter, callers pass `state` through.

The refactoring can be done piecemeal, one subsystem at a time. The
thread-local stays as a safety net during this phase but is only hit
in the shrinking set of still-not-refactored functions.

At the end of Phase C the thread-local is deleted. The only remaining
globals are things like:

-   The `FORMAT_SERIAL` constant (legitimately constant)
-   The bridge_core error channel (owned by the bridge, not us)

## How to reason about aliasing during Phase A

`extern static mut` in safe Rust is a loaded gun. The reason we get
away with it is that:

1.  XeTeX is single-threaded within a process (enforced by tectonic's
    global engine lock in `bridge_core`).
2.  There are no live `&` / `&mut` references to the globals held
    across function calls — every Phase A accessor reads-and-returns
    or writes-and-returns.
3.  We never take a raw slice of `mem[]` and pass it into a callback.
    All iteration is done by index: fetch-one, mutate-one, repeat.

These invariants are what `SAFETY:` comments on the accessors must
assert. If a translation would be much cleaner by holding a
`&mut [MemoryWord]` across multiple calls, restructure it to use
indices instead — that is *also* how the C does it.

## A note on `memory_word` endianness

`xetex-xetexd.h:85–196` contains the canonical description of how
`memory_word` bit-packs `b32`, `b16`, `gr` (double), and `ptr`. The
Rust `MemoryWord` type must have *bit-identical* layout to the C union
so that Phase A works — the C code is still reading and writing the
same backing memory as the Rust code. See `03-memory-and-nodes.md`
for the type definition.

Crucially: during Phase A, the `mem` array is still allocated by
`xcalloc_array` on the C side. The Rust code reads into it via the
raw pointer from `src/globals.rs`. Only in Phase B, when `Mem` takes
ownership, does the Rust `Vec<MemoryWord>` become the backing store,
and at that moment we must free the C-side allocation and point the
(now deprecated) C extern at the Rust buffer — or better, delete the
C `mem` declaration entirely, since every C user of `mem` has been
translated away by then.
