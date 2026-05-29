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

## `MemIdx` and `MemPtr`

`MemIdx` is a newtype for valid arena indices. It never represents
`TEX_NULL` or another sentinel. Nullable TeX halfwords use `MemPtr`
until they are checked and converted to `Option<MemIdx>`.

```rust
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
#[repr(transparent)]
pub struct MemIdx(pub i32);

impl MemIdx {
    pub fn offset(self, delta: i32) -> MemIdx { MemIdx(self.0 + delta) }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[repr(transparent)]
pub struct MemPtr(pub i32);

impl MemPtr {
    pub const NULL: MemPtr = MemPtr(0);
    pub fn is_null(self) -> bool { self == Self::NULL }
    pub fn as_idx(self) -> Option<MemIdx> {
        if self.is_null() { None } else { Some(MemIdx(self.0)) }
    }
}
```

Keeping both wrappers as `i32` (not `u32`) matters because some C code
stores negative sentinel values in fields that can also hold real
pointers. Until each such site is audited, stay representation-faithful
and do not silently treat every halfword as a valid arena index.

## Typed node handles and field access

The final idiomatic API is a typed-handle layer over the flat arena, not
bare field accessors that accept any `MemIdx`. The low-level storage
remains `Vec<MemoryWord>`; the type system tracks what kind of node an
index is known to reference after a checked construction/classification
step.

```rust
pub trait NodeKind {}

#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct NodePtr<K: NodeKind> {
    idx: MemIdx,
    _kind: PhantomData<fn() -> K>,
}

pub enum HListNode {}
pub enum VListNode {}
pub enum RuleNode {}
pub enum GlueNode {}
pub enum PenaltyNode {}

impl NodeKind for HListNode {}
impl NodeKind for VListNode {}
impl NodeKind for RuleNode {}
impl NodeKind for GlueNode {}
impl NodeKind for PenaltyNode {}
```

`NodePtr<K>` is the same runtime payload as `MemIdx`; `PhantomData` is
compile-time only. It is the core handle type for translated algorithms
because it is `Copy`, cheap to pass, and does not hold a borrow of
`Mem`.

Field methods on `Mem` should take the most specific typed handle they
can. Example, from the BOX macros:

```c
#define BOX_width(p)        mem[(p) + 1].b32.s1
#define BOX_depth(p)        mem[(p) + 2].b32.s1
#define BOX_height(p)       mem[(p) + 3].b32.s1
#define BOX_shift_amount(p) mem[(p) + 4].b32.s1
#define BOX_list_ptr(p)     mem[(p) + 5].b32.s1
```

becomes

```rust
pub trait HasDimensions: NodeKind {}
pub trait HasListPtr: NodeKind {}
pub trait HasShiftAmount: NodeKind {}

impl HasDimensions for HListNode {}
impl HasDimensions for VListNode {}
impl HasDimensions for RuleNode {}

impl HasListPtr for HListNode {}
impl HasListPtr for VListNode {}

impl HasShiftAmount for HListNode {}
impl HasShiftAmount for VListNode {}

impl Mem {
    #[inline]
    pub fn width<K: HasDimensions>(&self, p: NodePtr<K>) -> Scaled {
        Scaled(unsafe { self.word(p.idx.offset(1)).b32.s1 })
    }

    #[inline]
    pub fn set_width<K: HasDimensions>(&mut self, p: NodePtr<K>, w: Scaled) {
        self.word_mut(p.idx.offset(1)).b32.s1 = w.0;
    }

    // ... depth, height, shift_amount, list_ptr, list_ptr_idx ...
}
```

This makes irrelevant field access fail to compile: a `NodePtr<GlueNode>`
cannot be passed to `width()` unless `GlueNode` explicitly implements the
layout capability. Use capability traits for shared layouts (`width`,
`height`, `list_ptr`, `shift_amount`) and concrete marker types for
fields that belong to one node family (`penalty`, `glue_spec`,
`native_word_len`, etc.).

Field-access conventions:

-   Read methods are named after the field (`width`, `penalty`) and return
    the *semantic* type (`Scaled`, `MemIdx`, `FontId`, …), not raw
    `i32` / `u16`.
-   Write methods are `set_<name>` and take the semantic type.
-   All union field access is `unsafe` internally, wrapped by a
    `#[inline] pub fn` whose contract is "the caller owns a valid
    typed node handle". The docs say so.
-   Accessors are `#[inline]` because they are hot and trivial.
-   Transitional `MemIdx`-taking accessors are allowed during migration,
    but should be treated as bridge/private primitives, not the final API.

## Node-type variants

List traversal starts from untyped TeX pointers, so classification is a
runtime boundary. The C engine stores the type in `mem[p].b16.s1` (the
`NODE_type` macro); Rust should convert that raw value with a checked
path and then return typed handles:

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

impl TryFrom<u16> for NodeType {
    type Error = InvalidNodeType;

    fn try_from(raw: u16) -> Result<Self, Self::Error> {
        match raw {
            0 => Ok(NodeType::HList),
            1 => Ok(NodeType::VList),
            // ...
            _ => Err(InvalidNodeType(raw)),
        }
    }
}

pub enum AnyNode {
    Char(NodePtr<CharNode>),
    HList(NodePtr<HListNode>),
    VList(NodePtr<VListNode>),
    Rule(NodePtr<RuleNode>),
    Glue(NodePtr<GlueNode>),
    Penalty(NodePtr<PenaltyNode>),
    Whatsit(NodePtr<AnyWhatsitNode>),
    // ...
}

impl Mem {
    #[inline]
    pub fn node_type(&self, p: MemIdx) -> Result<NodeType, InvalidNodeType> {
        NodeType::try_from(unsafe { self.word(p).b16.s1 })
    }

    pub fn classify(&self, p: MemPtr) -> Result<Option<AnyNode>, InvalidNodeType> {
        // Check TEX_NULL, char-node range, node type, and subtype where needed.
    }
}
```

The exact numbering mirrors `xetex-constants.h`. Do not use unchecked
`transmute` for `NodeType`; invalid discriminants are Rust UB. Subtyped
families such as whatsits should get a second checked classification
layer (`AnyWhatsitNode` -> `NativeWordNode`, `LanguageNode`, etc.) so
subtype-specific fields are also guarded by typed handles.

## Typed view structs

For readability in translation-heavy code, define short-lived OOP-like
view types over typed handles. They are ergonomic sugar, not storage:

```rust
pub struct HListRef<'m> {
    mem: &'m Mem,
    p: NodePtr<HListNode>,
}

impl HListRef<'_> {
    pub fn width(&self) -> Scaled { self.mem.width(self.p) }
    pub fn list_ptr(&self) -> MemPtr { self.mem.list_ptr(self.p) }
    // ...
}

impl Mem {
    pub fn hlist(&self, p: MemPtr) -> Option<HListRef<'_>> {
        // checked classification, then return the view
    }

    pub fn with_hlist_mut<R>(
        &mut self,
        p: MemPtr,
        f: impl FnOnce(HListMut<'_>) -> R,
    ) -> Option<R> {
        // checked classification, short mutable borrow, then call f
    }
}
```

Immutable views are cheap (`&Mem` + typed index) and make local code read
well. Mutable views should stay short-lived, preferably closure-scoped as
above. TeX algorithms often keep many arena indices live while walking,
allocating, freeing, and mutating nodes; the core API should therefore
remain typed `NodePtr<K>` handles plus `&mut Mem` methods, with mutable
views used only where they improve a small local edit.

## Performance rules for the typed layer

Typed handles and views must preserve C-like performance:

-   No heap allocation per node. Storage remains one flat
    `Vec<MemoryWord>`.
-   No trait objects or dynamic dispatch in hot node traversal. Marker
    traits such as `HasDimensions` are compile-time generics only.
-   Classify once at control-flow boundaries and carry typed handles
    afterward; do not repeatedly reload and rematch node tags for the
    same pointer.
-   Keep `NodePtr<K>`, `MemIdx`, `MemPtr`, `Scaled`, `FontId`, etc.
    `Copy` and `#[repr(transparent)]` where representation matters.
-   Start with safe slice indexing inside `Mem`. If profiling or
    release-mode disassembly shows bounds checks in a hot path, add a
    tiny audited unchecked helper inside `Mem`; do not expose unchecked
    indexing broadly.
-   Spot-check generated code for representative hot accessors
    (`width`, `node_type`, `glue_spec`, linebreaker traversal) after the
    pattern is introduced. In optimized builds, typed handles should
    compile to the same load/store sequence as raw index access.

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
with the invariants TeX relies on. The typed-handle model gives us most
of the useful Rust type safety while preserving the flat arena.

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
-   Typed `NodePtr<K>` handles after checked classification; field
    methods accept typed handles, not arbitrary `MemIdx` values.
-   Capability traits model shared layouts (`HasDimensions`,
    `HasListPtr`, etc.) without dynamic dispatch.
-   Short-lived OOP-like views are allowed for readability; long-lived
    mutable node objects are not the storage model.
-   `MemoryWord` is a `#[repr(C)]` union for FFI compatibility in
    Phase A.
-   Cache locality, semantic fidelity, and format-file
    round-tripping all preserved.
