#!/usr/bin/env python3
import argparse
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src"

UNSAFE_FN_RE = re.compile(
    r"^(?P<indent>\s*)(?P<pub>(?:pub(?:\([^)]*\))?\s+)?)(?P<unsafe>unsafe\s+)(?P<extern>(?:extern\s+\"[^\"]*\"\s+)?)(?P<fn>fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*))"
)

def find_unsafe_fns(path: Path):
    lines = path.read_text(encoding="utf-8").splitlines()
    results = []
    for idx, line in enumerate(lines):
        m = UNSAFE_FN_RE.match(line)
        if m:
            results.append((idx + 1, m.group("name"), m.group(0)))
    return results

def main():
    files = sorted(SRC.glob("*.rs"))
    total = 0
    for f in files:
        fns = find_unsafe_fns(f)
        if fns:
            print(f"{f.relative_to(ROOT)}:")
            for line_no, name, match_str in fns:
                print(f"  Line {line_no:4d}: {match_str.strip()}")
                total += 1
    print(f"Total unsafe fns found: {total}")

if __name__ == "__main__":
    main()
