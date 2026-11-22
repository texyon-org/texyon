"""What the random probe generators share: the seeded choices and the command line."""

import argparse
import random
from pathlib import Path


class Generator:
    """Random choices for one document, the same for the same seed."""

    def __init__(self, seed):
        self.r = random.Random(seed)
        self.depth = 0

    def ch(self, *options):
        return self.r.choice(options)

    def dim(self, lo=-5, hi=20):
        return f"{self.r.randint(lo, hi)}.{self.r.randint(0, 99):02d}pt"


def main(prefix, generator, about):
    """Writes documents for run.py --probes, one per seed, named by prefix and seed."""
    ap = argparse.ArgumentParser(description=about)
    ap.add_argument("outdir", type=Path)
    ap.add_argument("count", type=int)
    ap.add_argument("first_seed", type=int, nargs="?", default=0)
    args = ap.parse_args()
    args.outdir.mkdir(parents=True, exist_ok=True)
    for seed in range(args.first_seed, args.first_seed + args.count):
        (args.outdir / f"{prefix}{seed:05d}.tex").write_text(generator(seed).doc())
