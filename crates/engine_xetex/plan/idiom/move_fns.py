#!/usr/bin/env python3
"""Mechanical function relocator for workstream 02 (module decomposition).

Moves whole top-level `fn` items (with their leading `// translated from …`
comment and `#[…]` attribute block) verbatim from one module file to another.
This is the *pure relocation* half of the §7 recipe in `02-modules.md`: it
changes no bytes inside any function, so it is parity-trivial by construction.
The *judgment* half — fixing `pub`→`pub(crate)` visibility, adding `use`
statements, redistributing imports — is deliberately NOT done here; that is the
`cargo check` loop the executing session drives by hand.

Assumptions (verified true for the engine's rustfmt'd modules):
  * target functions are top-level items starting at column 0
    (`(pub )?(unsafe )?(extern "C" )?fn NAME`);
  * their body closes with a `}` at column 0 (no earlier column-0 `}`);
  * the leading comment/attribute lines sit directly above the `fn`, separated
    from the previous item by a blank line.

Usage:
    # dry run — print what would move, touch nothing (default):
    python3 plan/idiom/move_fns.py --src src/scan.rs --dst src/errors.rs \
        --fns you_cant,report_illegal_case,privileged,cs_error,box_error,mu_error,runaway

    # apply the move:
    python3 plan/idiom/move_fns.py --src src/scan.rs --dst src/errors.rs \
        --fns you_cant,report_illegal_case,... --apply

Always run the dry run first and eyeball the ranges, then `--apply`, then
`cargo check` + the full dashboard before committing.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

FN_START = re.compile(r'^(pub(\([^)]+\))?\s+)?(unsafe\s+)?(extern\s+"C"\s+)?fn\s+(\w+)\b')
PREAMBLE = re.compile(r'^\s*(#!?\[|//)')  # attribute or comment line


def find_span(lines: list[str], name: str) -> tuple[int, int]:
    """Return [start, end) line indices (0-based) for `fn name`, including its
    leading comment/attribute block. Raises on ambiguity / not found."""
    hits = [
        i for i, ln in enumerate(lines)
        if (m := FN_START.match(ln)) and m.group(5) == name
    ]
    if not hits:
        raise SystemExit(f"error: fn `{name}` not found in source")
    if len(hits) > 1:
        raise SystemExit(
            f"error: fn `{name}` found {len(hits)} times "
            f"(lines {[h + 1 for h in hits]}); resolve by hand"
        )
    fn_line = hits[0]

    # Walk up over the contiguous comment/attribute preamble.
    start = fn_line
    while start - 1 >= 0 and PREAMBLE.match(lines[start - 1]) and lines[start - 1].strip():
        start -= 1

    # Walk down to the column-0 closing brace.
    end = None
    for i in range(fn_line + 1, len(lines)):
        if lines[i].rstrip() == "}":
            end = i + 1
            break
    if end is None:
        raise SystemExit(f"error: no column-0 closing brace found for `{name}`")

    # Consume a single trailing blank line so we don't leave double blanks.
    if end < len(lines) and lines[end].strip() == "":
        end += 1
    return start, end


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--src", required=True, type=Path)
    ap.add_argument("--dst", required=True, type=Path)
    ap.add_argument("--fns", required=True, help="comma-separated fn names")
    ap.add_argument("--apply", action="store_true", help="write the files (default: dry run)")
    args = ap.parse_args()

    names = [n.strip() for n in args.fns.split(",") if n.strip()]
    src_lines = args.src.read_text().splitlines(keepends=True)

    spans = sorted((find_span(src_lines, n) + (n,)) for n in names)
    # Reject overlaps (a fn captured inside another's preamble, etc.).
    for (s0, e0, n0), (s1, _, n1) in zip(spans, spans[1:]):
        if e0 > s1:
            raise SystemExit(f"error: spans for `{n0}` and `{n1}` overlap")

    moved_total = 0
    chunks: list[str] = []
    for s, e, n in spans:
        moved_total += e - s
        print(f"  {n:<28} src lines {s + 1}..{e}  ({e - s} lines)")
        chunks.append("".join(src_lines[s:e]))
    print(f"-> {len(spans)} fns, {moved_total} lines, {args.src} -> {args.dst}")

    if not args.apply:
        print("(dry run — pass --apply to write)")
        return 0

    # Remove from src (back-to-front so indices stay valid).
    keep = list(src_lines)
    for s, e, _ in sorted(spans, reverse=True):
        del keep[s:e]
    args.src.write_text("".join(keep))

    # Append to dst with a provenance marker.
    dst_text = args.dst.read_text()
    if dst_text and not dst_text.endswith("\n"):
        dst_text += "\n"
    banner = f"\n// ---- moved verbatim from {args.src.name} (workstream 02) ----\n\n"
    args.dst.write_text(dst_text + banner + "\n".join(c.rstrip("\n") for c in chunks) + "\n")
    print(f"applied: {moved_total} lines moved. Now fix visibility/imports + `cargo check`.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
