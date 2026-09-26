# engine_xetex architecture

The XeTeX engine in Rust. Procedures keep their `tex.web` names (`hpack`,
`line_break`, `fin_align`, ...), so upstream's WEB source is the map to the code.
The only non-Rust code built here is TECkit (`xetex/`).

## Module map

State and memory:

- `state.rs`: `EngineState`, the single owner of a run's state, split into
  sub-states (`mem`, `eqtb`, `scanner`, `input`, `save_stack`, `fonts`,
  `shipout`, `errors`, ...). See "API shape" below for how code reaches them.
- `mem.rs`: the `mem` arena. Fixed list heads (`CONTRIB_HEAD`, `HOLD_HEAD`,
  `ALIGN_HEAD`, `END_SPAN`, ...), the static glue specs (`ZERO_GLUE`, ...),
  both free lists (`st.mem.get_avail()`, `st.mem.get_node(s)`), node
  constructors (`new_glue`, `new_null_box`, ...), glue reference counts and
  the semantic nest.
- `node.rs`: typed handles. `NodePtr<K>` is a `Copy` index tagged with a kind
  (`HListNode`, `GlueNode`, `GlueSpecNode`, `UnsetNode`, `AlignRecord`, ...).
  Fields are read as `p.width(mem)` and written as `p.set_width(mem, w)`.
  Shared layouts are traits (`HasLink`, `HasWidth`, `IsBox`, ...), `FixedNode`
  kinds get `alloc`/`free`, and `ListRecord` kinds get `at` because their list
  proves their kind. `AnyNode`/`AnyBox` come out of `MemState::classify`.
  Noad subfields are `NodePtr<MathField>` and `NodePtr<DelimiterField>`
  (`q.nucleus()`, `q.left_delimiter()`, ...). Token lists are `MemPtr` chains
  read with `p.token(mem)` and `p.link(mem)`, their heads carrying
  `token_ref_count`.
- `units.rs`: `MemIdx`/`MemPtr` (null is `TEX_NULL`, not zero), `Scaled`,
  `FontId`, `StrNumber`, `NodeType`, `GlueSign`. `mem_types.rs` holds
  `MemoryWord`.
  - `Scaled` is every length: node dimensions, font metrics, `dimen_par`,
    the shipout and page builder positions, the line breaker's widths. It
    adds, subtracts, negates and scales by an `i32` like the raw words did.
    TeX's checked arithmetic (`x_over_n`, `xn_over_d`, `mult_and_add`,
    `add_or_sub`, `fract`) is generic over `i32` and `Scaled` (`TexWord`),
    `half` is `Scaled::half`. Sentinels have names: `RUNNING` rule
    dimensions, `IGNORE_DEPTH`, `MAX_DIMEN`, `INFINITE_DIMEN`, `DESIGN_SIZE`.
  - `FontId` indexes the per-font tables (`PerFont<T>` in `FontState`), is
    what char and native nodes store, and comes out of `cur_font`,
    `math_font` and `scan_font_ident`. `NULL_FONT` is `font_base`.
  - `StrNumber` is a string number (a single character below
    `TOO_BIG_CHAR`, else a pool string): the string pool API, `text(p)`,
    font names and areas, `job_name` and the scanned file name parts.
    `StrNumber::NONE` is TeX's 0 for "no string".
  - `cur_val` stays a raw `i32` because it holds whatever was scanned.
    `cur_dimen()` and `set_cur_dimen()` read and write it as a length.
- `consts.rs`: WEB `@d` constants and the `eqtb` layout (region bases and
  parameter indices, read through `EqtbState::int_par`, `dimen_par`, ...).
  Entries are read as `eq_type`/`eq_level`/`equiv` (or `int_at` in the
  integer regions) and through named views (`cur_font`, `box_reg`, `toks`,
  `cat_code`, `par_shape`, ...). The save stack has `saved(k)` and
  `save_type`/`save_level`/`save_index`.
- `cmd.rs`: command codes, in `xetex.web` order.
- `primitives.rs`: the primitive catalog. Its order fixes the hash layout and
  therefore the format.
- `errors.rs`: `ErrorState`. Set help with `st.errors.help(&[...])`, then call
  `error(st)?`. Every fatal path is one `Abort` (overflow, confusion, fatal
  error, interaction halt, I/O failure, ...) propagated with `?` up to the run
  loop, which prints the WEB termination text and maps it to a history code.
  `get_avail` overflow shows the runaway text, `get_node` overflow does not.

The typesetting pipeline:

- `scan.rs` (`get_next`, scanning, box registers), `expand.rs` (macros,
  conditionals), `main_control.rs` (the main loop, command verbs, alignments).
- `linebreak.rs` (paragraphs, hyphenation passes), `hyphenation.rs` (patterns
  and exceptions), `packaging.rs` (`hpack`, `vpack`, `vert_break`, `vsplit`),
  `pagebuilder.rs`, `math.rs` (`mlist_to_hlist`), `shipout.rs` (XDV output).
- `fonts.rs` (TFM and font parameters), `ext.rs` and `ot_math.rs` (OpenType
  and Graphite through `tectonic_xetex_layout`), `pic.rs` (pictures).
- `save.rs` (save stack, `eqtb` definitions, sparse registers), `hash.rs`,
  `string_pool.rs`, `input.rs`, `io.rs`, `format_io.rs` (`\dump` and load).

Crate and C boundaries:

- `bridge.rs` is the engine's I/O over the run's `CoreBridgeState` (files,
  diagnostics, shell escape, MD5, abort message), calling bridge-core's Rust
  methods with byte-slice names. Replay sources answer first. A run installs
  its state with `bridge::enter(api)`, a scope that holds the exclusive borrow
  and backs the one static pointer `with_bridge` reads. bridge-core's C global
  state (`ttbc_global_engine_enter`) is set only around calls into C that do
  their own I/O, which are the pdf_io picture probes (`bridge::with_c_io`).
- Fonts and shaping go straight to `tectonic_xetex_layout`'s Rust API:
  `Font`, `LayoutEngine` (`state::FontEngineHandle`), `FontManager`, the
  glyph-box and protrusion caches. Loading a font file takes the run's
  `CoreBridgeState` (`with_bridge`). That crate has no C API.
- `extern "C"` remains only for real C: `layout.rs` (TECkit converters, ICU
  line breaking and bidi, with their disposal, plus the HarfBuzz tag and
  `with_c_str` helpers), `io.rs` (ICU input converters), `pic.rs` and
  `engine.rs` (pdf_io), `compat.rs` (Windows time calls). File names are
  Rust bytes (`name_of_file`, `name_of_input_file`, the synctex and
  source-special names) and become C strings only at those calls.
- Raw pointers stay in those modules. Elsewhere `unsafe` is limited to
  plain-data byte views (`mem_types.rs`, `state.rs`, `format_io.rs`),
  HarfBuzz MATH queries (`ot_math.rs`), the bridge pointer and `strerror`
  (`bridge.rs`), the C time calls (`texmfmp.rs`) and the preview
  controller's pointers (`session.rs`). Every block has a `// SAFETY:` comment
  (`clippy::undocumented_unsafe_blocks` is clean). The
  engine proper prints and reports with Rust strings: `print(st, "...")`,
  `print_nl`, `print_esc` take a `&str`, bytes, a byte character or a
  `StrNumber`. A byte slice prints each byte as the character of that code.

Live preview:

- `session.rs` runs a resident engine. `checkpoint.rs` keeps a ladder of
  restorable rungs from reverse deltas (`journal.rs`) plus non-journaled state
  (`checkpoint/bundle.rs`). `checkpoint/oracle.rs` verifies restores when enabled.
- `replay/` models immutable editor sources and resumable command fences.
  `sync_index.rs` and `page_manifest.rs` record source and page geometry.

## API shape

- Logic that touches one sub-state is a method on that state, in an `impl`
  block of the module that owns the logic: `st.mem.free_node(p, s)`,
  `st.strings.length(s)`, `st.fonts.char_info(f, c)`,
  `st.input.cur_list.fin_mlist(&mut st.mem, p)`. Other sub-states it needs are
  plain parameters (split borrows), so the receiver is the state it mainly
  works on.
- Procedures that need the whole `EngineState` are free functions named like
  `tex.web`, taking `st` first: `line_break(st, d)`, `print_int(st, n)`,
  `main_control(st)`. `EngineState` itself has no methods.
- A sub-state method and an `EngineState` procedure may share a name when the
  procedure wraps it with error reporting (`finite_shrink`, `fetch`).

## Typing principle

The type system knows the node type.

- Allocators return `NodePtr<K>`, and functions take and return typed handles.
  A type known upstream is never re-derived downstream. When a function has to
  rediscover a type, fix the producer instead.
- The one legitimate classification point is a walk over a heterogeneous list:
  a single `match st.mem.classify(..)` at the dispatch, with typed handles
  flowing out of each arm. No scattered type-code comparisons, no
  `classify_x().expect()` at use sites.
- Raw words stay raw only at genuine `i32` boundaries (nest fields, `loc`,
  format dump). Don't add `Option` or sentinel juggling just to avoid an `i32`.
- A missing accessor is a small method on that kind's `impl` in `node.rs`.
- Typed accessors compile to the same loads as raw ones. Keep the hot paths
  (the main loop's character path, `get_next`/`expand`, `hpack`/`vpack`,
  `line_break`, `mlist_to_hlist`, `hlist_out`) free of extra classification
  and allocation.
- Comments explain only what the code cannot: a TeX quirk, a safety argument.

## Validation

Output must stay byte-identical (`.xdv`, `.pdf`, `.log`) unless a change is a
deliberate fix toward upstream.

- Spec: upstream `xetex.web` (`texk/web2c/xetexdir/` in texlive-source) is the
  golden reference. A TeX Live `xetex` is useful for spot checks. Fix a
  divergence toward WEB as its own `fix(...)` commit.
- Dashboard, after every change:

  ```sh
  cargo test -p tectonic --test tex-outputs
  cargo test -p tectonic --test trip
  cargo test -p tectonic --test engine_rust
  cargo test -p tectonic --test latex_goldens
  cargo test -p tectonic_engine_xetex
  ```

  Add `--test formats` for format changes, and `--test live_preview_session`
  plus `--test live_preview_replay_regressions` for checkpoint or journal work.
- Corpus A/B (`scripts/`): `corpus_fetch.sh` fills the corpus cache
  (`$TECTONIC_CORPUS_DIR`), then bless a baseline release build into scratch
  goldens and check the candidate against them:

  ```sh
  ORACLE_BIN=<baseline>/tectonic CORPUS_GOLDENS=/tmp/gold \
    crates/engine_xetex/scripts/corpus.sh bless all
  CANDIDATE_BIN=target/release/tectonic CORPUS_GOLDENS=/tmp/gold \
    crates/engine_xetex/scripts/corpus.sh check all
  ```

  The committed goldens in `tests/corpus/goldens` came from the retired C
  engine and lag behind deliberate fixes, so A/B against a baseline is the
  reliable check. `lvt_survey.sh` refreshes `tests/corpus/lvt-viable.txt`, and
  `parity.sh` logs a quick `tex-outputs` count.
- Upstream differential tests live under `tests/upstream/` and compare the
  engine against upstream XeTeX directly.
- `FORMAT_SERIAL` (`lib.rs`) is also the bundle URL version
  (`tectonic_bundles::get_fallback_bundle_url`). Never bump it for format
  content fixes.
