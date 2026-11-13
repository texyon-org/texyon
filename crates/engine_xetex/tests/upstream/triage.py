#!/usr/bin/env python3
"""Group the LaTeX suites' diffs by cause.

    crates/engine_xetex/tests/upstream/triage.py [diff dir]

Each diff is matched against the known causes below, in order; the rest are
grouped by what follows the first difference, so new causes stand out.
"""

import re
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
DEFAULT = ROOT / "target/upstream-diff/latex/diffs"

# (cause, which side, pattern found right at the first difference)
CAUSES = [
    ("file found by case-insensitive lookup upstream", "tt", r"^Nofile|^!LaTeXError:File`"),
    ("missing file: upstream's emergency stop is not logged", "tl", r"!Emergencystop\.<read\*>"),
    ("OpenType script list (upstream reads GSUB twice)", "tl", r"Script'[^']*'notexplicitlysupported"),
    ("OpenType script list (upstream reads GSUB twice)", "tt", r"^script=\w+;"),
    ("file found by case-insensitive lookup upstream", "tt", r"^LaTeXWarning:File`[^']*'notfound"),
    ("installed font traced from freed memory upstream (UPSTREAM_BUGS.md 21)", "tt",
     r"^\S*\.(otf|ttf|otc|ttc)Requestedfont"),
    ("harness: tl-nostat counts stand in for STAT tracing upstream", "tl", r"\\count3276\d"),
    ("generated font upstream (mktex, off from the next run)", "tl", r"^Badmetric\(TFM\)file"),
    ("main memory: TeXyoN caps at 64M words, upstream at 5M", "tl",
     r"TeXcapacityexceeded,sorry\[mainmemorysize=5000000\]"),
    ("pool size: TeXyoN grows to 40M, upstream stops at 6.25M", "tl",
     r"^!TeXcapacityexceeded,sorry\[poolsize="),
    ("TeXyoN stopped early", "tt", r"^$"),
]


def first_difference(lines):
    """For a log diff: the text after the first difference on each side."""
    head = next((l for l in lines if l.startswith("@@")), "")
    m = re.search(r"character (\d+)", head)
    tl = next((l[1:] for l in lines if l.startswith("-") and not l.startswith("---")), "")
    tt = next((l[1:] for l in lines if l.startswith("+") and not l.startswith("+++")), "")
    if not m:
        return None
    # The diff keeps 300 characters of context before the difference.
    lo = min(300, int(m.group(1)))
    return tl[lo:], tt[lo:]


# XDV causes, matched against the first upstream line that differs.
XDV_CAUSES = [
    ("hyperxmp dates its metadata from the log file being written", r"pdf:stream @hyxmp@Metadata"),
    ("embedded file dates: TeX Live files have mtimes, bundle files none", r"pdf:fstream .*/ModDate"),
]


def classify(path):
    lines = path.read_text(errors="replace").split("\n")
    if lines and lines[0].startswith(("upstream xetex", "<texyon timed out")):
        return lines[0]
    if lines and lines[0] == "--- upstream.log":
        tl, tt = first_difference(lines) or ("", "")
        for cause, side, rx in CAUSES:
            if re.search(rx, tl if side == "tl" else tt):
                return cause
        if re.match(r"^File:\S+Graphicfile", tl) and "notfound" in tt[:200]:
            return "file found by case-insensitive lookup upstream"
        return f"log: upstream `{tl[:50]}` vs `{tt[:50]}`"
    if lines and lines[0] == "--- upstream.xdv":
        minus = next((l for l in lines if l.startswith("-") and not l.startswith("---")), "-")
        plus = next((l for l in lines if l.startswith("+") and not l.startswith("+++")), "+")
        for cause, rx in XDV_CAUSES:
            if re.search(rx, minus):
                return cause
        return f"xdv: `{minus[1:].split(' ')[0]}` vs `{plus[1:].split(' ')[0]}`"
    return "other"


def main():
    d = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT
    groups = defaultdict(list)
    for f in sorted(d.glob("*.diff")):
        groups[classify(f)].append(f.stem)
    for cause, names in sorted(groups.items(), key=lambda g: -len(g[1])):
        print(f"{len(names):5}  {cause}")
        for n in names[:3]:
            print(f"         {n}")


if __name__ == "__main__":
    main()
