# CLAUDE.md

## Project goal

TeXyoN is a LaTeX engine built for live preview: XeTeX ported to Rust from Tectonic, kept resident between edits so a keystroke re-typesets only what it has to and redraws only the pages that changed. Two frontends drive it: the web editor (texyon-org/editor, through the wasm build) and the VS Code extension in `packages/vscode` (through the native binary).

`README.md` is the front door and should describe the project this way: what it does, current status, how to build and run it.

## Layering

- The engines (`engine_xetex`, `xetex_layout`, `engine_xdvipdfmx`, `pdf_io`, the `bridge_*` crates) typeset and nothing else. Upstream `xetex.web` is the source of truth for engine behaviour. The C oracle and passing goldens are only proxies, so when they disagree with WEB, fix toward WEB. `crates/engine_xetex/ARCHITECTURE.md` maps the engine.
- `live_preview` owns the preview: source staging, the resident engine driver, the pipeline, publication, diagnostics, pixels and SVG, PDF export and source navigation. `PreviewSession` is the one interface to it. Its core is portable, and only the `native` feature may assume an operating system.
- Hosts are thin: `crates/wasm` for the browser Worker, the `preview` command for native. A host supplies I/O, tuning and transport, and contains no preview logic. A feature one frontend needs goes into `live_preview`, so every frontend gets it.
- `preview_protocol` holds the message types, defined once for every host.
- `preview_render` holds the renderer. The MuPDF backend is AGPL and stays behind an opt-in feature. Licensing and credits live in `THIRD_PARTY_NOTICES.md`, keep it current when adding vendored code.
- The VS Code extension uses the native binary. Wasm is only a fallback for platforms without one, such as VS Code in the browser.

## Code style

- Type safety is key. When something can be built so the compiler catches future mistakes, prefer that over a shape that relies on remembering manual changes: newtypes for indices, ids and units (`FontId`, `StrNumber`, `Scaled`, `RequestSeq`) instead of bare integers, enums for codes and message kinds instead of numeric or string literals.
- No type hacks: no `unwrap`/`expect` outside tests, no lossy `as` casts where `try_from` fits, no `#[allow(...)]` to silence warnings, no stringly-typed state. Fix the typing at the root.
- `unsafe` only at FFI and allocation boundaries, each block with a `// SAFETY:` line (clippy denies undocumented ones).
- Errors are typed in libraries; `anyhow` only at the host and binary edge.
- Engine code: a function whose first parameter is one sub-state (`MemState`, `FontState`, `EqtbState`, ...) is a method on it. TeX procedures taking the whole `EngineState` stay free functions named as in tex.web, with no `_in` suffix.
- Comments only for a constraint or rationale the code cannot show, usually one line. No narration, no em dashes, no semicolons in comments. `missing_docs` is denied, so public items get a short doc line, nothing more.
- No debug knobs or leftovers in shipped code: no env-var switches, no ad hoc dumps or timing prints, no dead experiment paths.
- Tests must be able to fail: assert exact outputs, and check a new test catches the bug it is for.
- User-facing text (CLI output, errors, extension UI) is short and clear. Spell the name TeXyoN in prose and UI, and `texyon` for identifiers. No em dashes.

## Checks

- `cargo fmt --all` (stable rustfmt), `cargo clippy -p <crate> --all-targets --no-deps -- -D warnings`, `cargo test -p <crate>` for what changed.
- Engine behaviour changes: the goldens corpus (`crates/engine_xetex/scripts/corpus.sh`) and the upstream harness (`crates/engine_xetex/tests/upstream/run.py`). Both take minutes and load the machine, so ask before running them.
- Wasm changes: the real test is the editor. Build the engine assets from the editor repo and type in `/sample`.
- `packages/vscode`: `npm test`.
- Agent worktrees use their own target directory, never the main checkout's.

## Commits

- Short conventional commits, e.g. `fix(preview): keep the last frame on abort`. Summary line only, no body unless the why is not obvious.
- No AI references: no session links, no `Co-Authored-By`, no "generated with" lines.
