# 07 — Anti-patterns (read before writing code)

The previous Rustification attempt collapsed under the weight of specific
bad patterns introduced by LLMs trying to chase test failures. This file
catalogs those patterns. **If you are about to write code that resembles
anything in this list, stop and re-read the C source instead.**

## A.1 — Output-chasing with ad-hoc branches

> "The DVI file was off by a byte at offset 0x14a8 and the model fixed it
> by adding `if self.current_macro_name == "\\leaders" { … }`."

**Never do this.** XeTeX's behavior is fully determined by the state of
`mem`, `eqtb`, the input stack, and the current mode — not by the name of
the macro currently being expanded, not by the byte offset in the output,
not by the sequence number of the function call.

If tests regress, do not look at the diff in the output file first. Look
at the C source of the function you most recently translated. Re-read it
line-by-line next to your Rust translation. The bug is there.

**Forbidden predicates in behavioral branches:**

-   `== "\\macro_name"` on any currently-expanding token / control
    sequence name
-   comparisons against DVI byte offset, page number, or paragraph
    serial number (*unless* the C code itself branches on them for the
    same reason)
-   thread-local "breadcrumbs" recording which function called the
    current function ("if we got here via `scan_dimen`, do X")
-   hard-coded magic numbers that map to specific test inputs
    (the test *input filenames* are especially forbidden)

## A.2 — "Just one more field on the god-struct"

The C engine accesses state through ~150 `extern` globals. The temptation
when translating a function is to dump all referenced globals into a single
`EngineState` struct as fields, with no further thought. This is fine as a
*migration step* but must not become the end state.

-   The end-state `EngineState` should have sub-structs:
    `state.mem`, `state.eqtb`, `state.save_stack`, `state.input`,
    `state.strings`, `state.fonts`, `state.shipout`, etc. See
    `02-state.md`.
-   A function that only touches the memory arena takes `&mut Mem`, not
    `&mut EngineState`. This is how the borrow checker stops surprise
    aliasing bugs in the linked-list manipulation code.

## A.3 — `unsafe` as a transliteration shortcut

Pointer arithmetic in C becomes `unsafe` slice arithmetic in Rust. This
The
right tool for `mem[p + 1].b32.s1` is a typed `NodePtr<K>` handle plus a field
method defined directly on it (taking the `Mem` context as a parameter):

```rust
let p: NodePtr<HListNode> = mem.expect_hlist(raw_p)?;

impl<K: HasDimensions> NodePtr<K> {
    pub fn width(self, mem: &Mem) -> Scaled {
        mem[self.idx().offset(1)].scaled_s1()
    }

    pub fn set_width(self, mem: &mut Mem, w: Scaled) {
        mem[self.idx().offset(1)].set_scaled_s1(w);
    }
}
```

Raw `MemIdx`-taking field helpers are acceptable as a migration bridge
inside `Mem`, but they should not become the final call-site API because
they do not prove that the index references a node with that field.

`unsafe` is *only* allowed in:

-   FFI boundary code (declared in a single `c_api` module)
-   The bit-fiddling inside `MemoryWord` itself, where the implementation
    is 10 lines and can be audited in one sitting
-   Places where a profiler has shown a safe alternative is a real bottleneck
    — and then with a long `// SAFETY:` comment

## A.4 — Cloning to appease the borrow checker

If you write `.clone()` to make an error go away, you have almost
certainly structured the function wrongly. Typical fixes:

-   Split the function so two disjoint sub-borrows don't overlap.
-   Take indices (`MemIdx`) and re-index at each access, rather than
    holding references across calls.
-   Introduce a helper method on the owning struct instead of pulling
    fields out into local bindings.

Clone-to-silence only ever makes the next borrow-check error harder to
diagnose. Take the time to restructure.

Exception: copying a small `Copy` type (a `Scaled`, a `MemIdx`, an
enum tag) is not "cloning" and is fine.

## A.5 — Eager `Box<dyn Trait>` polymorphism for node types

Nodes (hbox, vbox, glue, penalty, char, …) *look* like variants that want
a trait. Don't. They are stored interleaved in a flat arena; the
existing C code hot paths depend on random-access integer indexing. A
`Box<dyn Node>` per node would obliterate cache locality and introduce
vtable dispatch in the paragraph-breaker's inner loop.

The right model is in `03-memory-and-nodes.md`: a flat
`Vec<MemoryWord>`, newtyped indices, and thin typed-view structs
(`HBoxRef<'_>`, `GlueRef<'_>`) that borrow `&Mem` and expose fields.
Enums for node type discrimination are fine; boxing per node is not.

## A.6 — Speculative generics and type parameters

Don't parameterize over "any memory backend" or "any scaled integer".
The engine has exactly one memory backend and exactly one scaled type.
Making them generic adds noise and compile time without buying anything.

## A.7 — "Modernizing" algorithms

The C code's line-breaker, page-builder, and math layout are Knuth's
and Jin-Wen Tsai's algorithms, refined over decades. **Do not rewrite
them** to use a different approach on the grounds that it's more
idiomatic. Translate faithfully. If an algorithm is legitimately
hideous to translate, add a `TODO` and leave it as a close transliteration
for now — post-rewrite cleanup is a separate, much safer phase.

## A.8 — Implementing things not in the C source

If the C code has a `#if 0` block, a dead branch, or unreachable code,
the Rust version should have nothing there. Do not "fix" dead code
while translating. Do not add `// TODO: implement X` for features that
the C version doesn't implement either.

## A.9 — Per-function panics "just in case"

Do not add `unwrap()` / `expect()` / `panic!()` at the top of each
translated function as a "this shouldn't happen" guard. The C code
does not have those guards, so Rust adding them is inventing a new
failure mode. If a Rust type forces you to handle an `Option`, look
at the C — the C code probably *does* handle the equivalent case
(often silently on a NULL pointer) and your Rust code should match.

## A.10 — Chasing the format file

TeX's format files (`plain.fmt`) are binary dumps of engine state. The
C engine reads and writes them through `load_fmt_file` /
`store_fmt_file`. These functions must be translated as a *pair* and
verified with a round-trip test (dump, read back, compare), not with
external format files. Do not attempt to make the Rust engine read a
format file produced by the C engine from a different codebase era —
use `FORMAT_SERIAL` to force regeneration.

## A.11 — "I'll document it later"

Every translated function gets a source-citation comment in the same
commit that introduces it. Every session ends with updates to
`MODULE_STATUS.md`, `SESSION_LOG.md`, and `PROGRESS.md`. Missing these
is the reason previous sessions couldn't resume cleanly. Future you
will thank present you for the 90 seconds.

## A.12 — Large PRs ("I'll just finish this module first")

Aim for one function, one commit. At most, a small set of closely
related functions (e.g., all the `new_*` node constructors). A session
that translates 30 functions in one untested commit is a session
whose failures cannot be bisected.
