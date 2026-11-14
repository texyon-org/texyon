# Contributing

Bug reports and pull requests are welcome. For anything larger than a fix,
open an issue first so we can agree on the approach before you write it.

## Reporting a bug

Include the smallest `.tex` file that shows the problem, what you expected,
and what TeXyoN did. If the output differs from XeTeX, say which TeX Live
version you compared against.

## Building and testing

See the README for build instructions. Before sending a pull request, run:

```sh
cargo fmt --all
cargo clippy -p <crate> --all-targets --no-deps -- -D warnings
cargo test -p <crate>
```

for the crates you changed. For the VS Code extension, run `npm test` in
`packages/vscode`.

Changes to engine behaviour also need the upstream harness
(`crates/engine_xetex/tests/upstream/run.py`). It takes a while. If you can't
run it, say so in the pull request and it will be run before merging.

## Engine behaviour

TeX Live's `xetex.web`, with its change files, decides what the engine should
do. When TeXyoN disagrees with it, the fix goes toward TeX Live, even if a
golden file has to change. If you find a bug in XeTeX itself, add it to
`crates/engine_xetex/UPSTREAM_BUGS.md`.

## Code style

- Prefer types the compiler can check: newtypes for ids and units, enums for
  codes.
- No `unwrap` or `expect` outside tests, and no `#[allow(...)]` to silence a
  warning.
- `unsafe` only at FFI and allocation boundaries, each block with a
  `// SAFETY:` comment.
- Comments explain a constraint or a reason the code can't show. Keep them
  short.

## Commits

Short conventional commits, subject line only, for example
`fix(preview): keep the last frame on abort`.

## License

By contributing you agree that your contribution is licensed under the MIT
License, as described in `LICENSE`.
