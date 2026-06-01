#!/usr/bin/env python3
"""Rank likely Phase-C threading candidates.

This is a read-only recon helper for plan/idiom/01b-phase-c-recipe.md. It uses
simple Rust-source heuristics, not a Rust parser, so treat its bucket/rank as a
starting point for manual review.
"""

from __future__ import annotations

import argparse
import json
import re
import signal
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Iterable


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src"

FN_RE = re.compile(
    r"(?m)^(?P<prefix>\s*(?:#\[[^\n]+\]\s*)*"
    r"(?:(?:pub(?:\([^)]*\))?\s+)?(?:unsafe\s+)?(?:extern\s+\"C\"\s+)?)?)"
    r"fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*\("
)

ACCESSOR_RE = re.compile(
    r"\b(?:"
    r"mem_(?:b16|b32|word)|set_mem_|eqtb_|set_eqtb_|"
    r"cur_(?:style|size|mu|f|c|i|mlist)|set_cur_(?:style|size|mu|f|mlist)|"
    r"font_(?:info|area|layout|bc|ec|param|char|width|height|depth|italic)|set_font_|"
    r"save_stack_|set_save_stack_|glue_|set_glue_|"
    r"page_|set_page_|dvi_|set_dvi_"
    r")"
)

STATE_METHOD_RE = re.compile(r"\bst\.(?:mem|eqtb|fonts|scanner|shipout|linebreak|save_stack)\b")
ALLOC_RE = re.compile(r"\b(?:get_node|new_[A-Za-z0-9_]+|fraction_rule|overbar)\b")
WITH_STATE_RE = re.compile(r"\bwith_state(?:_mut)?\s*\(")
GLOBAL_CALL_RE = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*_global)\s*\(")
CALL_RE = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(")

HAZARD_PATTERNS = {
    "scanner": re.compile(r"\b(?:scan_|get_x_token|get_token|back_input|begin_token_list|end_token_list)\b"),
    "diagnostic": re.compile(r"\b(?:error|overflow|confusion|fatal_error|pdf_error|char_warning|print_|diagnostic|help_line)\b"),
    "output": re.compile(r"\b(?:dvi_out|write_to_dvi|ship_out|hlist_out|vlist_out|ttstub_output|synctex)\b"),
    "packing": re.compile(r"\b(?:hpack|vpackage|clean_box|rebox|var_delimiter|mlist_to_hlist)\b"),
    "native_ot": re.compile(r"\b(?:get_ot_|isOpenType|usingOpenType|measure_native|native_glyph|build_opentype)\b"),
}

AVOID_FUNCTIONS = {
    "scan_toks",
    "main_control",
    "mlist_to_hlist",
    "hpack",
    "vpackage",
    "build_page",
    "line_break",
    "try_break",
    "hyphenate",
    "ship_out",
    "hlist_out",
    "vlist_out",
}


@dataclass
class FunctionInfo:
    name: str
    path: str
    line: int
    signature: str
    prefix: str
    body: str
    is_pub: bool
    is_unsafe: bool
    is_extern: bool
    is_no_mangle: bool
    is_global: bool
    with_state_calls: int
    global_calls: list[str]
    accessors: int
    state_methods: int
    alloc_calls: int
    hazards: list[str]
    caller_count: int = 0
    caller_files: list[str] = field(default_factory=list)
    callers_with_state_context: int = 0
    definition_only: bool = False
    bucket: str = "?"
    score: int = 0
    notes: list[str] = field(default_factory=list)


def iter_rs_files(paths: Iterable[str] | None) -> list[Path]:
    if paths:
        return [Path(p) if Path(p).is_absolute() else ROOT / p for p in paths]
    return sorted(SRC.glob("*.rs"))


def line_for_offset(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def find_matching_brace(text: str, open_idx: int) -> int:
    depth = 0
    i = open_idx
    in_line_comment = False
    in_block_comment = 0
    in_string = False
    in_char = False
    escaped = False

    while i < len(text):
        c = text[i]
        n = text[i + 1] if i + 1 < len(text) else ""

        if in_line_comment:
            if c == "\n":
                in_line_comment = False
            i += 1
            continue
        if in_block_comment:
            if c == "/" and n == "*":
                in_block_comment += 1
                i += 2
                continue
            if c == "*" and n == "/":
                in_block_comment -= 1
                i += 2
                continue
            i += 1
            continue
        if in_string:
            if escaped:
                escaped = False
            elif c == "\\":
                escaped = True
            elif c == '"':
                in_string = False
            i += 1
            continue
        if in_char:
            if escaped:
                escaped = False
            elif c == "\\":
                escaped = True
            elif c == "'":
                in_char = False
            i += 1
            continue

        if c == "/" and n == "/":
            in_line_comment = True
            i += 2
            continue
        if c == "/" and n == "*":
            in_block_comment = 1
            i += 2
            continue
        if c == '"':
            in_string = True
            i += 1
            continue
        if c == "'":
            in_char = True
            i += 1
            continue
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return i
        i += 1

    raise ValueError("unmatched brace")


def extract_functions(path: Path) -> list[FunctionInfo]:
    text = path.read_text()
    out: list[FunctionInfo] = []
    for match in FN_RE.finditer(text):
        open_idx = text.find("{", match.end())
        semi_idx = text.find(";", match.end())
        if open_idx < 0 or (semi_idx >= 0 and semi_idx < open_idx):
            continue
        try:
            close_idx = find_matching_brace(text, open_idx)
        except ValueError:
            continue

        body = text[open_idx + 1 : close_idx]
        sig = text[match.start() : open_idx].strip()
        prefix = match.group("prefix")
        name = match.group("name")
        hazards = [key for key, regex in HAZARD_PATTERNS.items() if regex.search(body)]
        global_calls = sorted(set(c for c in GLOBAL_CALL_RE.findall(body) if c != name))
        out.append(
            FunctionInfo(
                name=name,
                path=str(path.relative_to(ROOT)),
                line=line_for_offset(text, match.start()),
                signature=" ".join(sig.split()),
                prefix=prefix,
                body=body,
                is_pub="pub" in prefix,
                is_unsafe="unsafe" in prefix,
                is_extern='extern "C"' in prefix,
                is_no_mangle="#[no_mangle]" in prefix,
                is_global=name.endswith("_global"),
                with_state_calls=len(WITH_STATE_RE.findall(body)),
                global_calls=global_calls,
                accessors=len(ACCESSOR_RE.findall(body)),
                state_methods=len(STATE_METHOD_RE.findall(body)),
                alloc_calls=len(ALLOC_RE.findall(body)),
                hazards=hazards,
            )
        )
    return out


def count_callers(functions: list[FunctionInfo], files: list[Path]) -> None:
    texts = [(path, path.read_text()) for path in files]
    by_name = {f.name: f for f in functions}
    caller_files: dict[str, set[str]] = {f.name: set() for f in functions}

    for path, text in texts:
        rel = str(path.relative_to(ROOT))
        for m in CALL_RE.finditer(text):
            name = m.group(1)
            info = by_name.get(name)
            if info is None:
                continue
            window = text[max(0, m.start() - 80) : m.start()]
            if re.search(rf"\bfn\s+{re.escape(name)}\s*$", window):
                continue
            info.caller_count += 1
            caller_files[name].add(rel)
            context = text[max(0, m.start() - 180) : min(len(text), m.end() + 80)]
            if WITH_STATE_RE.search(context):
                info.callers_with_state_context += 1

    for info in functions:
        info.caller_files = sorted(caller_files[info.name])
        info.definition_only = info.caller_count == 0 and info.name in by_name


def classify(info: FunctionInfo) -> None:
    notes: list[str] = []
    score = 100

    if info.name in AVOID_FUNCTIONS:
        notes.append("avoid high-count orchestrator")
        score -= 80
    if info.is_no_mangle:
        notes.append("exported: verify no C caller before de-export")
        score -= 15
    if info.is_extern:
        notes.append("extern ABI boundary")
        score -= 20
    if info.callers_with_state_context:
        notes.append(f"{info.callers_with_state_context} caller(s) near with_state")
        score -= 30
    if info.with_state_calls:
        notes.append(f"{info.with_state_calls} internal with_state call(s)")
        score -= 20
    if info.hazards:
        notes.append("hazards: " + ",".join(info.hazards))
        score -= 12 * len(info.hazards)
    if info.alloc_calls:
        notes.append(f"{info.alloc_calls} allocator-like call(s)")
        score -= 5
    if info.definition_only and info.is_global:
        notes.append("possible dead _global wrapper")
        score += 30
    if info.is_global and info.caller_count > 0:
        notes.append("legacy wrapper/orchestrator")
        score -= 10
    if info.accessors:
        notes.append(f"{info.accessors} bridge accessor hit(s)")
        score += min(info.accessors, 25)
    if info.state_methods:
        notes.append(f"{info.state_methods} direct state hit(s)")
        score += min(info.state_methods, 10)

    if info.definition_only and info.is_global:
        bucket = "dead-wrapper"
    elif info.name in AVOID_FUNCTIONS:
        bucket = "D"
    elif "scanner" in info.hazards or len(info.hazards) >= 3 or info.callers_with_state_context:
        bucket = "D"
    elif info.hazards or info.with_state_calls:
        bucket = "C"
    elif info.accessors or info.global_calls or info.alloc_calls:
        bucket = "B"
    else:
        bucket = "A"

    info.bucket = bucket
    info.score = score
    info.notes = notes


def table(rows: list[FunctionInfo], limit: int) -> str:
    rows = rows[:limit]
    headers = ["score", "bucket", "function", "loc", "callers", "with", "glob", "acc", "hazards"]
    lines = ["  ".join(headers)]
    lines.append("  ".join("-" * len(h) for h in headers))
    for r in rows:
        loc = f"{r.path}:{r.line}"
        hazards = ",".join(r.hazards) if r.hazards else "-"
        lines.append(
            f"{r.score:>5}  {r.bucket:<11}  {r.name:<34}  {loc:<28}  "
            f"{r.caller_count:>7}  {r.with_state_calls:>4}  "
            f"{len(r.global_calls):>4}  {r.accessors:>3}  {hazards}"
        )
    return "\n".join(lines)


def main() -> int:
    signal.signal(signal.SIGPIPE, signal.SIG_DFL)

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="*", help="optional source files relative to crate root")
    parser.add_argument("--json", action="store_true", help="emit JSON instead of a table")
    parser.add_argument("--limit", type=int, default=40, help="maximum table rows")
    parser.add_argument(
        "--bucket",
        action="append",
        choices=["A", "B", "C", "D", "dead-wrapper"],
        help="filter by inferred bucket; repeatable",
    )
    parser.add_argument("--module", help="filter path substring, e.g. src/math.rs")
    parser.add_argument("--include-avoid", action="store_true", help="include known large orchestrators")
    parser.add_argument("--include-scan", action="store_true", help="include src/scan.rs in broad rankings")
    parser.add_argument("--include-unused", action="store_true", help="include uncalled non-wrapper helpers")
    parser.add_argument("--dead-wrappers", action="store_true", help="show only possible dead _global wrappers")
    args = parser.parse_args()

    files = iter_rs_files(args.paths)
    functions = [fn for path in files for fn in extract_functions(path)]
    count_callers(functions, files)
    for info in functions:
        classify(info)

    rows = functions
    if args.module:
        rows = [r for r in rows if args.module in r.path]
    elif not args.include_scan:
        rows = [r for r in rows if r.path != "src/scan.rs"]
    if args.dead_wrappers:
        rows = [r for r in rows if r.bucket == "dead-wrapper"]
    elif not args.include_unused:
        rows = [r for r in rows if r.caller_count > 0 or r.is_global]
    if args.bucket:
        wanted = set(args.bucket)
        rows = [r for r in rows if r.bucket in wanted]
    if not args.include_avoid:
        rows = [r for r in rows if r.name not in AVOID_FUNCTIONS]

    rows.sort(key=lambda r: (r.score, -r.caller_count, r.name), reverse=True)

    if args.json:
        payload = []
        for r in rows:
            d = asdict(r)
            d.pop("body", None)
            d.pop("prefix", None)
            payload.append(d)
        print(json.dumps(payload, indent=2, sort_keys=True))
    else:
        print(table(rows, args.limit))
        print()
        print("Legend: A/B are codemod candidates; C needs borrow-dropping review; D is manual.")
        print("Heuristics are conservative. Always inspect the body before editing.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
