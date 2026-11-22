#!/usr/bin/env python3
"""Groups the LaTeX suites' diffs by cause.

    crates/engine_xetex/tests/upstream/triage.py [diff dir]

Each diff is matched against the known causes below, in order. The rest are
grouped by what follows the first difference, so new causes stand out.
"""

import re
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
DEFAULT = ROOT / "target/upstream-diff/latex/diffs"
# The context latex.log_diff keeps before the first difference.
LOG_CONTEXT = 300

# (cause, side, pattern found right at the first difference)
LOG_CAUSES = [
    ("file found by case-insensitive lookup upstream", "texyon", r"^Nofile|^!LaTeXError:File`"),
    ("missing file: upstream's emergency stop is not logged", "tl", r"!Emergencystop\.<read\*>"),
    ("OpenType script list (UPSTREAM_BUGS.md 18)", "tl", r"Script'[^']*'notexplicitlysupported"),
    ("OpenType script list (UPSTREAM_BUGS.md 18)", "texyon", r"^script=\w+;"),
    ("file found by case-insensitive lookup upstream", "texyon", r"^LaTeXWarning:File`[^']*'notfound"),
    ("installed font traced from freed memory upstream (UPSTREAM_BUGS.md 21)", "texyon",
     r"^\S*\.(otf|ttf|otc|ttc)Requestedfont"),
    ("tl-nostat counts stand in for STAT tracing upstream", "tl", r"\\count3276\d"),
    ("font generated upstream (mktex is off from the next run)", "tl", r"^Badmetric\(TFM\)file"),
    ("main memory: upstream overflows", "tl", r"TeXcapacityexceeded,sorry\[mainmemorysize="),
    ("pool size: upstream overflows, both have 6.25M", "tl", r"^!TeXcapacityexceeded,sorry\[poolsize="),
    ("TeXyoN stopped early", "texyon", r"^$"),
]
# Matched against the first upstream line that differs.
XDV_CAUSES = [
    ("hyperxmp dates its metadata from the log file being written", r"pdf:stream @hyxmp@Metadata"),
    ("embedded file dates: TeX Live files have mtimes, bundle files none", r"pdf:fstream .*/ModDate"),
]


def first_changes(lines):
    """The first removed and added line of a diff, without their markers."""
    removed = next((l[1:] for l in lines if l.startswith("-") and not l.startswith("---")), "")
    added = next((l[1:] for l in lines if l.startswith("+") and not l.startswith("+++")), "")
    return removed, added


def classify(path):
    lines = path.read_text(errors="replace").split("\n")
    first = lines[0] if lines else ""
    if first.startswith(("upstream xetex", "<texyon timed out", "upstream bugs:")):
        return first
    if first == "--- upstream.log":
        m = re.search(r"character (\d+)", next((l for l in lines if l.startswith("@@")), ""))
        if not m:
            return "other"
        lo = min(LOG_CONTEXT, int(m.group(1)))
        tl, texyon = (side[lo:] for side in first_changes(lines))
        for cause, side, rx in LOG_CAUSES:
            if re.search(rx, tl if side == "tl" else texyon):
                return cause
        if re.match(r"^File:\S+Graphicfile", tl) and "notfound" in texyon[:200]:
            return "file found by case-insensitive lookup upstream"
        return f"log: upstream `{tl[:50]}` vs `{texyon[:50]}`"
    if first == "--- upstream.xdv":
        tl, texyon = first_changes(lines)
        for cause, rx in XDV_CAUSES:
            if re.search(rx, tl):
                return cause
        return f"xdv: `{tl.split(' ')[0]}` vs `{texyon.split(' ')[0]}`"
    return "other"


def main():
    d = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT
    groups = defaultdict(list)
    for f in sorted(d.glob("*.diff")):
        groups[classify(f)].append(f.stem)
    for cause, names in sorted(groups.items(), key=lambda g: -len(g[1])):
        print(f"{len(names):5}  {cause}")
        for name in names[:3]:
            print(f"         {name}")


if __name__ == "__main__":
    main()
