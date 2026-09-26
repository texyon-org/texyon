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
- `token.rs`: `Token`, a transparent `i32` in TeX's layout (`cmd * max_char_val
  + chr` for a character, `cs_token_flag + p` for a control sequence).
  Built with `Token::new(cmd, chr)`, `Token::from_cs(p)` and helpers like
  `Token::other(b'=')`, read with `is_cs()`, `cs()`, `cmd()`, `chr()`. It is
  ordered like the raw value, so brace and parameter tests stay range checks
  (`is_left_brace()`, `is_brace()`, `is_match()`). `cur_tok`, `par_token`,
  `after_token` and token list nodes hold it. `Token::ZERO` is TeX's 0.
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
  - Overflow: TeX checks it only where WEB does (`\multiply`, `\divide`,
    the scanners' "Number too big", e-TeX expressions, the helpers above).
    Everything else wraps at 32 bits as in the C original: `Scaled`
    arithmetic wraps, and the `i32` sums of user values (`\advance`,
    penalties and demerits in `line_break`, `insert_penalties`, `prev_graf`
    and line numbers, negating a scanned value) use `wrapping_*`. So debug
    builds, which check overflow, behave like release builds. A plain `+`
    on an `i32` asserts that it cannot overflow.
  - `FontId` indexes the per-font tables (`PerFont<T>` in `FontState`), is
    what char and native nodes store, and comes out of `cur_font`,
    `math_font` and `scan_font_ident`. `NULL_FONT` is `font_base`.
  - `StrNumber` is a string number (a single character below
    `TOO_BIG_CHAR`, else a pool string): the string pool API, `text(p)`,
    font names and areas, `job_name` and the scanned file name parts.
    `StrNumber::NONE` is TeX's 0 for "no string".
  - `cur_val` stays a raw `i32` because it holds whatever was scanned.
    `cur_dimen()` and `set_cur_dimen()` read and write it as a length.
- `consts.rs`: WEB `@d` constants and the `eqtb` layout (region bases).
- `params.rs`: `IntPar`, `DimenPar`, `GluePar` and `TokPar`, the `eqtb`
  parameters read through `EqtbState::int_par`, `dimen_par`, `glue_par` and
  `tok_par` (`\parshape` is `PAR_SHAPE_LOC`). Each knows
  its `eqtb` location (`loc()`) and primitive name, a `GluePar` also the
  subtype of glue made from it.
  Entries are read as `eq_type`/`eq_level`/`equiv` (or `int_at` in the
  integer regions) and through named views (`cur_font`, `box_reg`, `toks`,
  `cat_code`, `par_shape`, ...). The save stack has `saved(k)` and
  `save_type`/`save_level`/`save_index`.
- `cmd.rs`: command codes, in `xetex.web` order. They stay `u8` because
  TeX compares ranges of them (`cur_cmd >= CALL`).
- `codes.rs`: enums for the `chr_code` modifiers of the commands that
  dispatch on them (`LastItemCode`, `ConvertCode`, `IfCode`, `FiOrElseCode`,
  `ExtensionCode`, `BoxCode`, `SkipCode`, `ShorthandDefCode`,
  `BoxDimenCode`, `ShowCode`, `InputCode`, `PageIntCode`, `TheCode`,
  `MarkCode`, `MathCharNumCode`, `DelimNumCode`, `MathAccentCode`,
  `RadicalCode`) and other small code sets (`ValueLevel` for
  `cur_val_level` and sparse-array types, `ScannerStatus`). `cur_mark` is
  indexed by `MarkCode` and `sa_root` by `ValueLevel`. `code_enum!`
  gives each `from_raw` (a jump table) and `raw`, and `name()` where the
  variants are primitives, which `primitives.rs` and `print_cmd_chr` share.
  Dispatch decodes `cur_chr` once and matches on variants. A `chr_code` that
  can also be a pointer (`\count` registers, `\toks`) stays raw. The
  `selector` is `Selector` in `state.rs`, whose `File(n)` is a `\write`
  stream and whose `with_log`/`without_terminal` steps are TeX's arithmetic.
- `primitives.rs`: the primitive catalog. Its order fixes the hash layout and
  therefore the format.
- `errors.rs`: `ErrorState`. Set help with `st.errors.help(&[...])`, then call
  `error(st)?`. Every fatal path is one `Abort` (overflow, confusion, fatal
  error, interaction halt, I/O failure, ...) propagated with `?` up to the run
  loop, which prints the WEB termination text and maps it to a history code.
  `get_avail` overflow shows the runaway text, `get_node` overflow does not.

The typesetting pipeline. The big modules are split by `tex.web` part into
submodules. The parent keeps what its children share and re-exports the
crate-facing procedures, so callers write `crate::scan::scan_int`. Children
see each other through `pub(super)`.

- `scan.rs` (keywords, math codes, rule specs, box registers) with
  `scan/next.rs` (`get_next`, `get_x_token`, `pass_text`), `scan/numbers.rs`
  (`scan_int`, `scan_dimen`, `scan_glue`, `scan_expr` and the bounded
  `scan_*_int`s), `scan/internal.rs` (`scan_something_internal`, font
  queries) and `scan/toks.rs` (`the_toks`, `scan_toks`, `read_toks`,
  `\scantokens`).
- `expand.rs` (macros, conditionals).
- `main_control.rs` (the main loop and its `dispatch_command`) with
  `main_control/main_loop.rs` (the character loop: ligatures, kerns, native
  words, interchar tokens), `lists.rs` (paragraphs, glue, kerns, inserts,
  marks, discretionaries, accents, `handle_right_brace`), `boxes.rs`
  (`begin_box`, `box_end`, `package`, `unpackage`, `delete_last`),
  `align.rs` (`\halign`/`\valign`) and `assign.rs` (`prefixed_command`,
  registers, `new_font`, the `alter_*` family).
- `linebreak.rs` (`line_break`, `try_break`, the active list) with
  `linebreak/post.rs` (`post_line_break`), `protrusion.rs` (margin kerns),
  `prehyph.rs` (finding words to hyphenate), `hyphenate.rs` (`hyphenate`,
  `reconstitute`, native-word hyphens) and `locale.rs` (ICU line breaks).
  `hyphenation.rs` holds patterns and exceptions.
- `packaging.rs` (`hpack`, `vpackage`, `append_to_vlist`) with
  `packaging/vert_break.rs` (`vert_break`, `vsplit`, `prune_page_top`,
  `do_marks`) and `packaging/copy.rs` (`copy_node_list`,
  `flush_node_list`, `just_copy`, `just_reverse`).
- `pagebuilder.rs`.
- `math.rs` (math font parameters) with `math/mlist.rs` (`mlist_to_hlist`
  and the `make_*` noad handlers), `math/scripts.rs` (`make_scripts`),
  `math/delimiters.rs` (`char_box`, `var_delimiter`, `rebox`),
  `math/build.rs` (the math-mode commands, `fin_mlist`) and
  `math/display.rs` (`init_math`, `after_math`, displays).
- `shipout.rs` (`ship_out`, `hlist_out`, `vlist_out`, movements) with
  `shipout/whatsits.rs` (`out_what`: `\special`, `\write`, `\openout`
  streams, pictures).
- `output.rs` (`print_*`, the diagnostic sink) with `output/cmd_chr.rs`
  (`print_cmd_chr`, `print_meaning`), `output/show_box.rs` (`show_box`,
  `show_node_list`), `output/show.rs` (`show_whatever`, `show_activities`,
  `show_context`, `show_token_list`, `issue_message`) and
  `output/warnings.rs` (e-TeX nesting warnings, font and `char_warning`s).
- `dvi.rs`: `DviWriter` (`st.shipout.dvi`), the XDV byte stream. It owns
  TeX's half-swapped `dvi_buf`, the file handle, `last_bop`, the push depth
  (`cur_s`, -2 after an overflow) and the page manifest that records byte
  spans for live preview. Shipout calls commands (`bop`, `eop`, `set_char`,
  `set_rule`, `fnt`, `fnt_def`, `set_glyphs`, `special`, `movement`, ...),
  never opcodes. `enter_level`/`leave_level` are `hlist_out`'s push and pop
  (an empty box's `push` is taken back). Preamble and postamble are split in
  two around `prepare_mag`, which can still report an error, as in WEB.
  `movement` optimization stays in `shipout.rs` (it needs `mem`) and patches
  earlier movements with `set_reg_at`. The buffer and position are private:
  a checkpoint keeps `into_checkpoint()` of the writer and returns with
  `restore`, which reuses the live buffer and keeps the page manifest.
- Font metrics (`fonts.rs`): `st.fonts.char_info(f, c)` is a `CharInfo`
  view (`exists()`, `tag()` as `CharTag::{None, Lig, List, Ext}`,
  `remainder()`), measured with `char_width`/`char_height`/`char_depth`/
  `char_italic`. `char_exists(f, c)` adds the `font_bc..=font_ec` check.
  Lig/kern programs are `LigKernStep`s: `lig_kern_start(f, i)` returns the
  first instruction and its index (following a redirect), `lig_kern_step(k)`
  reads one, and a step knows `matches(c)`, `is_last()`, `is_kern()`,
  `lig_op()`, `lig_char()` and `next_index(k)`. `char_kern(f, step)` and
  `ext_recipe(f, i)` (an `ExtRecipe` of `top`/`mid`/`bot`/`rep`) complete the
  set, and `param(f, n)` reads a font parameter. The TFM reader stores raw
  words, only these views interpret them.
- `fonts.rs` (TFM and font parameters), `ext.rs` (loading OpenType and
  Graphite fonts through `tectonic_xetex_layout`, feature parsing, TECkit
  mappings, font queries) with `ext/native.rs` (shaping and measuring native
  words and glyphs) and `ext/extension.rs` (`do_extension`, source
  specials), `ot_math.rs` (OpenType MATH), `pic.rs` (pictures).
- `save.rs` (save stack, `eqtb` definitions, sparse registers), `hash.rs`,
  `string_pool.rs`, `io.rs`.
- `input.rs`: the input stack. A level's `state` is a scanner state
  (`MID_LINE`, `SKIP_BLANKS`, `NEW_LINE`, which `get_next` adds to a category
  code) or `TOKEN_LIST`. Its `index` is the file number of a file level and
  the `TokenType` of a token-list level (`token_type()`, only valid when
  `state == TOKEN_LIST`).
- `format_io.rs` (`\dump` and load): each section of the format file is one
  function generic over `FmtStream`, so the same code dumps (`Dump`) and
  loads (`Undump`) it. Load-only range checks and allocation, and dump-only
  statistics, are `if S::LOADING` branches. `FmtItem` gives each stored type
  its big-endian encoding, an 8-byte word as one 64-bit value.

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
  `CoreBridgeState` (`with_bridge`). That crate has no C API. `ot_math.rs`
  reads the MATH table through `LayoutEngine::ot_math()`, bridge-harfbuzz's
  safe `ot::Math`.
- `extern "C"` remains only for real C: `layout.rs` (TECkit converters, ICU
  line breaking and bidi, with their disposal, plus the HarfBuzz tag and
  `with_c_str` helpers), `io.rs` (ICU input converters), `pic.rs` and
  `engine.rs` (pdf_io), `compat.rs` (Windows time calls). File names are
  Rust bytes (`name_of_file`, `name_of_input_file`, the synctex and
  source-special names) and become C strings only at those calls.
- Raw pointers stay in those modules. Elsewhere `unsafe` is limited to
  plain-data byte views (`mem_types.rs`, `state.rs`, `format_io.rs`),
  the bridge pointer and `strerror` (`bridge.rs`), the C time calls (`texmfmp.rs`) and the preview
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

  `cargo test` builds without optimization and with overflow checks, so the
  dashboard also exercises the wraparound rules above.

  Add `--test formats` for format changes, and `--test live_preview_session`
  plus `--test live_preview_replay_regressions` for checkpoint or journal work.
- Clippy: `cargo clippy -p tectonic_engine_xetex --all-targets -- -D warnings`
  must stay clean. CI runs it on the latest stable, whose clippy can be newer
  than a local one.
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
  engine against upstream XeTeX directly. Running them with a debug
  `tectonic` binary also checks for overflow panics (`probes/limits/wrap_*`
  push the wraparound paths). A debug build overflows its stack on
  `cov-control-expand_depth`, which release handles.
- `tests/algorithms/` runs small documents through INITEX in-process and
  checks what they `\show` (packaging, line breaking, math, the page
  builder, the scanners, register arithmetic). Their expectations come
  from TeX Live's `xetex -ini` on the same input. Fonts come from
  `tests/assets`. INITEX has no `\tracingparagraphs` or `\tracingpages`
  output here (no `stat` code), so tests show boxes and values instead.
- `FORMAT_SERIAL` (`lib.rs`) is also the bundle URL version
  (`tectonic_bundles::get_fallback_bundle_url`). Never bump it for format
  content fixes.
