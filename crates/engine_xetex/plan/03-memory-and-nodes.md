# 03 — Memory arena & node types

TeX's performance and its data-structural elegance both flow from its
single contiguous `mem[]` arena, in which *all* dynamically allocated
objects — token lists, nodes, glue specs, list heads, save records —
are interleaved as sequences of `memory_word`s. We preserve this
model in Rust for three reasons:

1.  **Cache locality.** The line-breaker, page-builder, and
    paragraph-construction routines all walk linked lists; contiguous
    arena storage is what makes them fast.
2.  **Semantic fidelity.** The C algorithms reason in terms of integer
    `mem` indices (`p`, `q`, `cur_p`, …) that point freely between
    node types. A `Box<dyn Node>` model cannot express this without
    constant downcasts.
3.  **Round-tripping with format files.** `.fmt` files are serialized
    images of `mem[]` and `eqtb[]`. If our in-memory layout is a
    faithful mirror of the C one (modulo endianness cleanups), the
    format-file code translates almost directly.

## `MemoryWord`

The canonical C definition (from `xetex-xetexd.h:99–104`) is:

```c
typedef union {
    b32x2 b32;      // two 32-bit ints: s0, s1
    b16x4 b16;      // four 16-bit ints: s0, s1, s2, s3
    double gr;      // a 64-bit float (glue ratio)
    void *ptr;      // a native pointer (font layout engines, etc.)
} memory_word;
```

Rust equivalent (in `src/mem_types.rs`):

```rust
#[repr(C)]
#[derive(Copy, Clone)]
pub union MemoryWord {
    pub b32: B32x2,
    pub b16: B16x4,
    pub gr: f64,
    pub ptr: *mut std::ffi::c_void,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct B32x2 {
    pub s0: i32,
    pub s1: i32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct B16x4 {
    pub s0: u16,
    pub s1: u16,
    pub s2: u16,
    pub s3: u16,
}
```

`#[repr(C)]` guarantees layout compatibility with the C union, which
is required because during Phase A (see `02-state.md`) both C and
Rust read the same backing array. The big-endian layout of the
original code is only relevant for format-file I/O; in-memory, we
follow the host byte order and trust that the format dump/undump
routines do the byte-swapping they already do in C (`xetex-swap.h`).

The union is safe because:

-   TeX only ever reads from the field it most recently wrote, and
    the existing C algorithms are already carefully consistent.
-   All access is mediated by typed accessor methods (below), so
    raw union access is confined to `mem_types.rs` where it can be
    audited in one sitting.

## The `Mem` struct

```rust
pub struct Mem {
    data: Vec<MemoryWord>,  // the arena itself

    // Allocator state (from xetex-ini.c globals)
    pub lo_mem_max: MemIdx,
    pub hi_mem_min: MemIdx,
    pub mem_end: MemIdx,
    pub avail: MemIdx,       // head of single-word free list
    pub rover: MemIdx,       // rover in the variable-size free list
    pub var_used: i32,
    pub dyn_used: i32,
}

impl Mem {
    pub fn new(initial_size: usize) -> Self { ... }

    // Direct word access (only inside the arena module; external
    // code uses typed views)
    #[inline]
    pub(crate) fn word(&self, p: MemIdx) -> &MemoryWord { &self.data[p.0 as usize] }
    #[inline]
    pub(crate) fn word_mut(&mut self, p: MemIdx) -> &mut MemoryWord { &mut self.data[p.0 as usize] }
}
```

## `MemIdx`

A newtype for arena indices. Never an `i32` in idiomatic Rust code.

```rust
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct MemIdx(pub i32);

impl MemIdx {
    pub const NULL: MemIdx = MemIdx(0);
    pub fn is_null(self) -> bool { self == Self::NULL }
    pub fn offset(self, delta: i32) -> MemIdx { MemIdx(self.0 + delta) }
}
```

Keeping this as `i32` (not `u32`) matters because some C code stores
negative sentinel values in fields that can also hold real pointers.
Until each such site is audited, we stay faithful.

## Typed node accessors

For every node variety in `xetex-xetexd.h:198–322`, we introduce
inherent methods on `Mem`. Example, from the BOX macros:

```c
#define BOX_width(p)        mem[(p) + 1].b32.s1
#define BOX_depth(p)        mem[(p) + 2].b32.s1
#define BOX_height(p)       mem[(p) + 3].b32.s1
#define BOX_shift_amount(p) mem[(p) + 4].b32.s1
#define BOX_list_ptr(p)     mem[(p) + 5].b32.s1
```

becomes

```rust
impl Mem {
    #[inline]
    pub fn box_width(&self, p: MemIdx) -> Scaled {
        Scaled(unsafe { self.word(p.offset(1)).b32.s1 })
    }
    #[inline]
    pub fn set_box_width(&mut self, p: MemIdx, w: Scaled) {
        self.word_mut(p.offset(1)).b32.s1 = w.0;
    }

    // ... depth, height, shift_amount, list_ptr, list_ptr_idx ...
}
```

Accessor conventions:

-   Read methods are named after the field (`box_width`) and return
    the *semantic* type (`Scaled`, `MemIdx`, `FontId`, …), not raw
    `i32` / `u16`.
-   Write methods are `set_<name>` and take the semantic type.
-   All union field access is `unsafe` internally, wrapped by a
    `#[inline] pub fn` whose contract is "the caller owns a valid
    node at `p` of the right variety". The docs say so.
-   Accessors are `#[inline]` because they are hot and trivial.

## Node-type variants

There's a separate question of how to *discriminate* node types.
The C engine stores the type in `mem[p].b16.s1` (the `NODE_type`
macro). Rust will define:

```rust
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u16)]
pub enum NodeType {
    HList = 0,
    VList = 1,
    Rule = 2,
    Ins = 3,
    Mark = 4,
    Adjust = 5,
    Ligature = 6,
    Disc = 7,
    WhatsIt = 8,
    Math = 9,
    Glue = 10,
    Kern = 11,
    Penalty = 12,
    Unset = 13,
    Style = 14,
    Choice = 15,
    // ...
}

impl Mem {
    #[inline]
    pub fn node_type(&self, p: MemIdx) -> NodeType {
        // SAFETY: every node has a valid type byte at mem[p].b16.s1
        unsafe { std::mem::transmute(self.word(p).b16.s1) }
    }
    // (or via a match / TryFrom if we want stricter bounds)
}
```

The exact numbering mirrors `xetex-constants.h`.

## Typed "view" structs (optional sugar)

For readability in translation-heavy code, we can define short-lived
view types:

```rust
pub struct BoxRef<'a> { mem: &'a Mem, p: MemIdx }
impl<'a> BoxRef<'a> {
    pub fn width(&self) -> Scaled { self.mem.box_width(self.p) }
    pub fn height(&self) -> Scaled { self.mem.box_height(self.p) }
    // ...
}

impl Mem {
    pub fn box_ref(&self, p: MemIdx) -> BoxRef<'_> { BoxRef { mem: self, p } }
}
```

These are *optional*. If a translation reads more naturally as
`mem.box_width(p)` directly, use that. Don't create view types
speculatively.

## Why not an enum-per-node model?

A classic "clean Rust" design would be:

```rust
enum Node {
    HList(HList),
    Glue(GlueSpec),
    Penalty(i32),
    // ...
}

type Mem = Vec<Node>;
```

This is **wrong for this project**, for these reasons:

1.  TeX nodes are variable-sized (a box is 7 words, a glue node is 3,
    a char node is 1, a native-word node is NATIVE_NODE_SIZE + glyph
    count words, …). An `enum` forces every slot to be the size of
    the largest variant.
2.  Native-word nodes have *trailing variable-length data*
    (UTF-16 characters and glyph metrics) that lives *inline in
    `mem[]`*. There is no clean way to represent this in a Rust enum
    — you'd need a parallel side-allocation and lose the locality.
3.  The algorithms assume that `p + k` is the k-th word after `p`
    *regardless of the node type between them*. Enum variants break
    this.
4.  Format file round-trips depend on the flat layout.

The enum model is seductively clean but fundamentally incompatible
with the invariants TeX relies on. The newtype-index + typed-accessor
model keeps the invariants and is "clean enough".

## `Scaled` and other unit newtypes

TeX has a bunch of integer types that are semantically distinct:

```rust
/// 1/65536 of a point. TeX's basic length unit.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
pub struct Scaled(pub i32);

/// Internal font number (1..font_ptr).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct FontId(pub u16);

/// Token — packed (cmd, chr) pair as in `cur_tok`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Token(pub i32);

/// String number into the string pool.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct StrNumber(pub i32);
```

Operator overloads (`Add`, `Sub`, multiplication by integer) on
`Scaled` come for free via `derive` or short `impl` blocks. Don't
let primitive `i32` escape into translation code where a semantic
type applies.

## Migration of `mem[]` ownership (Phase B recap)

In Phase A, `mem` is a raw C pointer declared in `globals.rs`. In
Phase B, ownership transfers to Rust:

1.  All C functions that allocate or free nodes have been translated
    (Phase 2 of the migration order).
2.  On engine startup, Rust constructs `Mem` via
    `Mem::new(initial_size)`, which allocates a `Vec<MemoryWord>`.
3.  Until the C side is fully gone, the C `mem` extern (if still
    referenced) points at the Rust `Vec`'s data via a shim written
    in Rust:

    ```rust
    #[no_mangle]
    pub static mut mem: *mut MemoryWord = std::ptr::null_mut();

    fn install_mem_ptr(state: &mut EngineState) {
        unsafe { mem = state.mem.data.as_mut_ptr(); }
    }
    ```

    We call `install_mem_ptr` on engine init and again whenever the
    `Vec` is reallocated (which must not happen during normal
    execution — pre-allocate to the max). Vec reallocations
    invalidate the C pointer, so either guarantee no reallocs or
    allocate once at maximum size.

## Summary

-   Flat `Vec<MemoryWord>`, no enum-per-node.
-   `MemIdx` newtype, never bare `i32` at call sites.
-   Typed accessors on `Mem` with `Scaled` / `FontId` / `MemIdx`
    return values.
-   `MemoryWord` is a `#[repr(C)]` union for FFI compatibility in
    Phase A.
-   Cache locality, semantic fidelity, and format-file
    round-tripping all preserved.
