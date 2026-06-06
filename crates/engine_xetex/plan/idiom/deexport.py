#!/usr/bin/env python3
"""De-export codemod for workstream 03 / overview target #5 (session 139).

Removes vestigial `#[no_mangle] extern "C"` from *internal* Rust functions —
those whose only C references are dead prototypes in the now-empty compilation
units (`xetex/*.c`, all 0 function bodies except the setjmp shim). Verified
session 139: the only genuine C->Rust call is `tt_run_engine` from
`xetex-engine-interface.c`; every other compiled `.c` file is prototype-only.

The transform per decorated item:
  - `#[no_mangle]` line removed,
  - `extern "C" ` stripped from the `fn` signature (`pub unsafe extern "C" fn
    F` -> `pub unsafe fn F`).
Rust callers use direct path calls, which are ABI-agnostic, so this is a pure
link-surface change with zero behavior effect (the bytes out of the engine do
not move). The linker + the four-suite dashboard are the safety net.

NEVER touched:
  - `#[no_mangle]` on `static` items (exported data, some self-FFI'd),
  - any name in KEEP (genuine FFI boundary / macOS-referenced / deferred
    self-FFI).

Usage:
    python3 plan/idiom/deexport.py src/math.rs            # dry run, report
    python3 plan/idiom/deexport.py src/math.rs --apply    # rewrite in place
    python3 plan/idiom/deexport.py --all                  # dry-run every module
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src"

# Genuine FFI boundary + macOS-referenced + deferred self-FFI. Never de-export.
KEEP = {
    # engine entry / config ABI (engine-interface.c)
    "tt_run_engine", "tt_xetex_set_int_variable", "tt_xetex_set_string_variable",
    # C-ABI fixed-point helpers (kept since session 50; macos.c refs)
    "D2Fix", "Fix2D",
    # referenced by xetex-macos.c (out of scope here, keep for the macOS build)
    "font_feature_warning", "readCommonFeatures", "read_double",
    # genuine C++->Rust boundary: called from xetex-XeTeXOTMath.cpp (a compiled
    # C++ unit with real bodies, unlike the empty .c units). Verified session
    # 139 by grepping the .cpp directly — `cargo check` would NOT catch this
    # (it does not link C++), only `cargo test`'s link step would.
    "tt_get_font_area_entry", "tt_get_font_size_entry",
    "tt_get_font_layout_engine_entry",
    # self-FFI (declared in an extern "C" block in another module) — handled in
    # a dedicated commit that converts the caller to a direct path first.
    "end_token_list", "initialize_pagebuilder_variables", "tex_round",
    # Dead-on-de-export: these have zero Rust callers and no C caller; the
    # #[no_mangle] export was their only "use", so de-exporting them surfaces a
    # dead_code warning. Left exported here to keep this sweep purely about the
    # link surface; flagged for a separate dead-code-removal pass.
    "graphite_warning", "print_two", "short_display_n", "print_chars",
}

NOMANGLE_RE = re.compile(r"^\s*#\[no_mangle\]\s*$")
FN_SIG_RE = re.compile(
    r"^(?P<indent>\s*)(?P<pre>(?:pub(?:\([^)]*\))?\s+)?(?:unsafe\s+)?)"
    r"extern\s+\"C\"\s+fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)"
)
# A no_mangle item that is a static (data) — skip entirely.
STATIC_RE = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?static\b")
ITEM_ANY_RE = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:unsafe\s+)?(?:extern\s+\"C\"\s+)?(fn|static)\b")


def process(path: Path, apply: bool) -> tuple[int, list[str]]:
    lines = path.read_text().splitlines(keepends=True)
    out: list[str] = []
    i = 0
    changed = 0
    names: list[str] = []
    while i < len(lines):
        line = lines[i]
        if NOMANGLE_RE.match(line):
            # find the decorated item, skipping interleaved attrs/doc comments
            j = i + 1
            while j < len(lines) and (
                lines[j].lstrip().startswith("#[")
                or lines[j].lstrip().startswith("//")
                or lines[j].strip() == ""
            ):
                j += 1
            if j < len(lines):
                m = FN_SIG_RE.match(lines[j])
                if m and m.group("name") not in KEEP:
                    # Drop the #[no_mangle] line; emit interleaved attrs/comments
                    # unchanged; strip `extern "C" ` from the signature line.
                    # For non-snake-case names, #[no_mangle] had been suppressing
                    # the non_snake_case lint — replace it with an explicit allow
                    # so the faithful name survives without raising the warning
                    # count (a later cosmetic-rename pass can drop both).
                    name = m.group("name")
                    names.append(name)
                    changed += 1
                    if re.search(r"[A-Z]", name):
                        indent = m.group("indent")
                        out.append(f"{indent}#[allow(non_snake_case)]\n")
                    out.extend(lines[i + 1:j])
                    out.append(lines[j].replace('extern "C" ', "", 1))
                    i = j + 1
                    continue
                # else: keep #[no_mangle] (static item, keep-set fn, or a
                # no_mangle fn that is not extern "C" — nothing to strip).
        out.append(line)
        i += 1
    if apply and changed:
        path.write_text("".join(out))
    return changed, names


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("files", nargs="*")
    ap.add_argument("--all", action="store_true")
    ap.add_argument("--apply", action="store_true")
    args = ap.parse_args()
    files = (
        sorted(SRC.glob("*.rs")) if args.all
        else [ROOT / f if not Path(f).is_absolute() else Path(f) for f in args.files]
    )
    if not files:
        ap.error("give files or --all")
    total = 0
    for f in files:
        n, names = process(f, args.apply)
        total += n
        if n:
            verb = "de-exported" if args.apply else "would de-export"
            print(f"{f.relative_to(ROOT)}: {verb} {n}  ({', '.join(names)})")
    print(f"TOTAL: {total}")


if __name__ == "__main__":
    main()
