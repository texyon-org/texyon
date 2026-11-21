#!/usr/bin/env python3
"""Download arXiv LaTeX sources for the differential suite.

Samples recent submissions across categories through the arXiv API and keeps
each source as a tarball under $TEXYON_CORPUS_DIR/arxiv (default
~/.cache/texyon-corpus/arxiv). Requests are spaced as arXiv asks of
automated clients, so a large sample takes a while. Reruns skip what exists.

    crates/engine_xetex/tests/upstream/fetch_arxiv.py [--per-category N] [--categories A,B]
"""

import argparse
import gzip
import io
import os
import random
import re
import tarfile
import time
import urllib.request
import xml.etree.ElementTree as ET
from pathlib import Path

CATEGORIES = [
    "math.AG", "math.CO", "math.PR", "math.AP", "math.NT", "math.OC", "math.GT", "math.ST",
    "cs.LG", "cs.CL", "cs.CV", "cs.DS", "cs.LO", "cs.PL", "cs.CR", "cs.IT",
    "hep-th", "hep-ph", "gr-qc", "astro-ph.GA", "astro-ph.CO", "cond-mat.str-el",
    "cond-mat.mtrl-sci", "quant-ph", "physics.flu-dyn", "physics.optics", "nucl-th",
    "stat.ME", "q-bio.PE", "q-fin.MF", "econ.TH", "eess.SP", "math-ph", "nlin.CD",
]
API = "https://export.arxiv.org/api/query"
EPRINT = "https://arxiv.org/e-print/"
AGENT = "texyon-corpus-fetch/1.0"
DELAY = 3.5
ATOM = "{http://www.w3.org/2005/Atom}"

last_request = 0.0


def get(url):
    global last_request
    wait = last_request + DELAY - time.monotonic()
    if wait > 0:
        time.sleep(wait)
    last_request = time.monotonic()
    req = urllib.request.Request(url, headers={"User-Agent": AGENT})
    with urllib.request.urlopen(req, timeout=120) as r:
        return r.read()


def ids_for(cat, n, rng):
    # A random offset into the newest few thousand spreads the sample over recent years.
    start = rng.randrange(0, 4000)
    url = f"{API}?search_query=cat:{cat}&sortBy=submittedDate&sortOrder=descending&start={start}&max_results={n}"
    feed = ET.fromstring(get(url))
    out = []
    for e in feed.iter(f"{ATOM}entry"):
        ident = e.find(f"{ATOM}id").text.rsplit("/abs/", 1)[-1]
        out.append(re.sub(r"v\d+$", "", ident))
    return out


def as_tarball(data):
    """Return the e-print as tar bytes, or None when it is not LaTeX source."""
    try:
        raw = gzip.decompress(data)
    except OSError:
        return None
    try:
        with tarfile.open(fileobj=io.BytesIO(raw)) as t:
            if any(m.name.endswith(".tex") for m in t.getmembers()):
                return data
        return None
    except tarfile.TarError:
        pass
    # A single gzipped .tex file.
    if b"\\documentclass" in raw[:200000] or b"\\begin{document}" in raw:
        buf = io.BytesIO()
        with tarfile.open(fileobj=buf, mode="w:gz") as t:
            info = tarfile.TarInfo("main.tex")
            info.size = len(raw)
            t.addfile(info, io.BytesIO(raw))
        return buf.getvalue()
    return None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--per-category", type=int, default=50)
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--categories", help="comma-separated subset of CATEGORIES")
    args = ap.parse_args()
    root = Path(os.environ.get("TEXYON_CORPUS_DIR", Path.home() / ".cache/texyon-corpus")) / "arxiv"
    root.mkdir(parents=True, exist_ok=True)
    skip = root / "not-latex.txt"
    skipped = set(skip.read_text().split()) if skip.exists() else set()
    rng = random.Random(args.seed)
    cats = args.categories.split(",") if args.categories else CATEGORIES
    unknown = set(cats) - set(CATEGORIES)
    if unknown:
        raise SystemExit(f"unknown categories: {', '.join(sorted(unknown))}")
    for cat in cats:
        try:
            ids = ids_for(cat, args.per_category, rng)
        except OSError as e:
            print(f"{cat}: listing failed: {e}", flush=True)
            continue
        got = 0
        for ident in ids:
            dest = root / (ident.replace("/", "_") + ".tar.gz")
            if dest.exists() or ident in skipped:
                continue
            try:
                data = as_tarball(get(EPRINT + ident))
            except OSError as e:
                print(f"{ident}: {e}", flush=True)
                continue
            if data is None:
                with skip.open("a") as f:
                    f.write(ident + "\n")
                continue
            dest.write_bytes(data)
            got += 1
        print(f"{cat}: {got} new", flush=True)


if __name__ == "__main__":
    main()
