#!/usr/bin/env python3
# node_field_recon.py — read-only recon for the `01c` typed-node cookbook.
#
# Counts node-field access sites in the engine_xetex Rust crate, grouped by
# the *word subfield* (`b32.s0/s1`, `b16.s0..s3`, `gr`, `ptr`) and the
# *offset* `N` in `mem_<subfield>(expr + N)`. The (subfield, offset) pair is
# what the C field macros key on (see `xetex/xetex-xetexd.h`), so this is the
# mechanical signal that survives Stage-1's collapse of the per-family macros
# (`BOX_width`, `GLUE_SPEC_stretch`, …) into the generic `mem_b32s1(p + N)`
# accessor.
#
# IMPORTANT (read `01c` §A): the offset does NOT uniquely identify a node
# family — many families share `(p+1).b32.s1`, etc. This script reports the
# mechanical histogram; the family attribution + ranking lives in `01c` §A,
# built by joining this histogram to the header macro table. Treat the
# per-family numbers in the doc as a header-informed split of these signatures,
# not a per-site classification.
#
# Usage:
#   python3 plan/idiom/node_field_recon.py            # full histogram
#   python3 plan/idiom/node_field_recon.py --by-file  # add per-file totals
#
# It touches nothing; it only reads src/*.rs.

import re
import sys
from collections import defaultdict
from pathlib import Path

SRC = Path(__file__).resolve().parents[2] / "src"

SUBFIELDS = ("b32s0", "b32s1", "b16s0", "b16s1", "b16s2", "b16s3", "gr", "ptr", "node_addr")

# (set_)?mem_<sub>(  <arg-without-nested-parens>  )
CALL = re.compile(
    r"\b(set_)?mem_(" + "|".join(SUBFIELDS) + r")\(\s*([^()]*?)\s*\)"
)
# trailing "+ N" in the argument, e.g. "p + 1", "cur_p + 4"
TRAIL_OFFSET = re.compile(r"\+\s*(\d+)\s*$")


def subfield_pretty(sub: str) -> str:
    if sub in ("gr", "ptr", "node_addr"):
        return sub
    # b32s1 -> b32.s1
    return sub[:3] + "." + sub[3:]


def main() -> int:
    by_file = "--by-file" in sys.argv

    # key: (subfield, offset_str) -> [reads, writes]
    hist = defaultdict(lambda: [0, 0])
    file_tot = defaultdict(int)
    total_reads = total_writes = 0

    for path in sorted(SRC.glob("*.rs")):
        text = path.read_text(encoding="utf-8", errors="replace")
        for m in CALL.finditer(text):
            is_write = bool(m.group(1))
            sub = m.group(2)
            arg = m.group(3)
            om = TRAIL_OFFSET.search(arg)
            offset = om.group(1) if om else "0"
            hist[(sub, offset)][1 if is_write else 0] += 1
            file_tot[path.name] += 1
            if is_write:
                total_writes += 1
            else:
                total_reads += 1

    # ---- histogram, sorted by total traffic ----
    rows = sorted(hist.items(), key=lambda kv: -(kv[1][0] + kv[1][1]))
    print(f"{'subfield':<10} {'+off':>5} {'reads':>7} {'writes':>7} {'total':>7}")
    print("-" * 42)
    for (sub, off), (r, w) in rows:
        print(f"{subfield_pretty(sub):<10} {('+'+off):>5} {r:>7} {w:>7} {r+w:>7}")
    print("-" * 42)
    print(f"{'TOTAL':<10} {'':>5} {total_reads:>7} {total_writes:>7} "
          f"{total_reads+total_writes:>7}")
    print(f"\ndistinct (subfield,offset) signatures: {len(hist)}")
    print(f"sites with explicit +N (N>0): "
          f"{sum(r+w for (s,o),(r,w) in hist.items() if o != '0')}")
    print(f"sites at offset 0 (node header word: link/info/type/subtype/char): "
          f"{sum(r+w for (s,o),(r,w) in hist.items() if o == '0')}")

    if by_file:
        print("\nper-file node-field site totals:")
        for name, n in sorted(file_tot.items(), key=lambda kv: -kv[1]):
            print(f"  {n:>5}  {name}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
