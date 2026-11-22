#!/usr/bin/env python3
"""Differential test of TeXyoN against TeX Live's XeTeX.

Both engines run the same documents and their normalized logs and XDV pages
must match. A difference with a known cause is listed in known.tsv, a case
left out entirely in slow.tsv.

    crates/engine_xetex/tests/upstream/run.py [--suite SUITES] [-j N] [-v] [filter ...]

Suites:
    probes  plain TeX documents in probes/ (the default)
    lvt     the LaTeX kernel's regression tests
    arxiv   arXiv sources, fetched first with fetch_arxiv.py
    docs    the documents of tests/corpus/manifest.tsv
    tldocs  the package documentation in TeX Live's doc tree
    all     every suite

Needs Linux, TeX Live 2026, a C compiler and, for --pixels, pdftoppm. The
environment variables XETEX (TeX Live's xetex), TEXYON (default
target/release/texyon), UPSTREAM_WORK (default target/upstream-diff) and
TEXYON_CORPUS_DIR (default ~/.cache/texyon-corpus) override the defaults.
The fuzz/ generators write random probes for --probes.
"""

import argparse
import difflib
import hashlib
import os
import re
import shutil
import subprocess
import sys
from concurrent.futures import ProcessPoolExecutor, ThreadPoolExecutor, as_completed
from pathlib import Path

import latex
import xdv

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
ASSETS = ROOT / "tests" / "assets"
PROBES = HERE / "probes"
KNOWN = HERE / "known.tsv"
SLOW = HERE / "slow.tsv"
FONTS = HERE / "fonts"

XETEX = os.environ.get("XETEX", "/usr/local/texlive/2026/bin/x86_64-linux/xetex")
PLTOTF = os.environ.get("PLTOTF", str(Path(XETEX).with_name("pltotf")))
TEXYON = os.environ.get("TEXYON", str(ROOT / "target" / "release" / "texyon"))
WORK = Path(os.environ.get("UPSTREAM_WORK", ROOT / "target" / "upstream-diff"))

ENV = dict(os.environ, TZ="UTC", SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")
PROBE_TIMEOUT = 60
# Each LaTeX case can take gigabytes, so the default leaves room.
DEFAULT_JOBS = max(1, (os.cpu_count() or 2) // 2)

LOG_SUBS = [
    # kpathsea's paths print as ./name, TeXyoN's as name.
    (re.compile(r"\(\./"), "("),
    (re.compile(r'(\\XeTeXp(ic|df)file) "\./'), r'\1 "'),
    # The XDV preamble comment and font names differ in length.
    (re.compile(r"^(Output written on \S+ \(\d+ pages?), \d+ bytes\)\.$"), r"\1)."),
]
NO_END = "<no legal \\end>\n"


def make_bundle():
    bundle = WORK / "bundle"
    shutil.rmtree(bundle, ignore_errors=True)
    bundle.mkdir(parents=True)
    for f in ASSETS.iterdir():
        if f.suffix in (".tfm", ".otf", ".ttf", ".png", ".pdf") or f.name in ("knuth-plain.tex", "hyphen.tex"):
            shutil.copy(f, bundle / f.name)
    texmf = Path(XETEX).resolve().parents[2] / "texmf-dist"
    for rel in (FONTS / "texlive.txt").read_text().split():
        shutil.copy(texmf / rel, bundle / Path(rel).name)
    for pl in sorted(FONTS.glob("*.pl")):
        subprocess.run([PLTOTF, pl, bundle / (pl.stem + ".tfm")], check=True, stdout=subprocess.DEVNULL)
    # The engine looks formats up under the bundle names it kept from Tectonic.
    (bundle / "tectonic-format-plain.tex").write_text("\\input knuth-plain \\dump\n")
    # TeXyoN caches formats by this digest, so a new build must change it.
    h = hashlib.sha256()
    for f in sorted(bundle.iterdir()):
        h.update(f.name.encode())
        h.update(f.read_bytes())
    h.update(Path(TEXYON).read_bytes())
    (bundle / "SHA256SUM").write_text(h.hexdigest() + "\n")
    return bundle


def tl_env(bundle, fmtdir):
    return dict(
        ENV,
        TEXINPUTS=f".:{bundle}:",
        TFMFONTS=f"{bundle}:",
        OPENTYPEFONTS=f"{bundle}:",
        TTFONTS=f"{bundle}:",
        TEXPICTS=".:",
        TEXFORMATS=f"{fmtdir}:",
        openout_any="a",
        **latex.CAPACITIES,
    )


def make_tl_format(bundle):
    fmtdir = WORK / "tl-fmt"
    fmtdir.mkdir(parents=True, exist_ok=True)
    shutil.copy(HERE / "tl-nostat.tex", fmtdir)
    subprocess.run(
        [XETEX, "-ini", "-etex", "-interaction=batchmode", "-jobname=plain",
         "\\input tl-nostat \\input knuth-plain \\dump"],
        cwd=fmtdir, env=tl_env(bundle, fmtdir), stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL, check=False)
    if not (fmtdir / "plain.fmt").exists():
        sys.exit("TeX Live's xetex failed to build plain.fmt")
    return fmtdir


class UpstreamCrash(Exception):
    pass


def fresh_dir(side, name, src):
    d = WORK / side / name
    shutil.rmtree(d, ignore_errors=True)
    d.mkdir(parents=True)
    shutil.copy(src, d / "probe.tex")
    return d


def run_tl(name, src, bundle, fmtdir):
    d = fresh_dir("tl", name, src)
    # A picture found in the working directory prints as ./name, which the logs normalize.
    text = Path(src).read_bytes()
    if b"XeTeXpicfile" in text or b"XeTeXpdffile" in text:
        for f in bundle.iterdir():
            if f.suffix in (".png", ".pdf"):
                shutil.copy(f, d / f.name)
    r = subprocess.run(
        [XETEX, "-fmt=plain", "-no-pdf", "-interaction=scrollmode", "probe.tex"],
        cwd=d, env=tl_env(bundle, fmtdir), stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=PROBE_TIMEOUT)
    if r.returncode < 0:
        raise UpstreamCrash(f"TeX Live's xetex died with signal {-r.returncode}")
    if r.returncode == 3 and b"Internal error" in r.stdout:
        raise UpstreamCrash("TeX Live's xetex hit an internal error")
    return d


def run_texyon(name, src, bundle):
    d = fresh_dir("texyon", name, src)
    subprocess.run(
        [TEXYON, "compile", "--outfmt", "xdv", "--keep-logs", "--untrusted",
         "-Z", "deterministic-mode", "-Z", "continue-on-errors", "--format", "plain",
         "--bundle", str(bundle), "probe.tex"],
        cwd=d, env=ENV, stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=PROBE_TIMEOUT)
    return d


def norm_log(path):
    if not path.exists():
        return ["<no log>"]
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    start = next((i for i, l in enumerate(lines) if l.startswith(("(./probe.tex", "(probe.tex"))), 0)
    out = []
    for line in lines[start:]:
        # TeXyoN's terminal holds no command line.
        if line == "<*> probe.tex":
            out += ["<*> ", "    "]
            continue
        if out[-2:] == ["<*> ", "    "] and line == " " * 13:
            continue
        for rx, rep in LOG_SUBS:
            line = rx.sub(rep, line)
        out.append(line)
    while out and out[-1] == "":
        out.pop()
    text = "\n".join(out)
    # Without \end upstream reads the terminal, and TeXyoN has none.
    text = text.replace("*\n! Emergency stop.\n<*> \n    \nEnd of file on the terminal!\n\n", NO_END)
    text = text.replace("! Emergency stop\n*** (job aborted, no legal \\end found)\n", NO_END)
    # TeXyoN's empty terminal line gives one \par before it stops.
    text = text.replace("{\\par}\n" + NO_END, NO_END)
    return text.split("\n")


def load_list(path):
    """Case names and reasons from a known.tsv-style file."""
    entries = {}
    for line in path.read_text().splitlines():
        if line.strip() and not line.startswith("#"):
            name, reason = line.split("\t", 1)
            entries[name] = reason
    return entries


def compare_probe(name, src, bundle, fmtdir):
    try:
        tl = run_tl(name, src, bundle, fmtdir)
        ours = run_texyon(name, src, bundle)
    except UpstreamCrash as e:
        return name, [f"<crash: {e}>"]
    except (OSError, subprocess.TimeoutExpired) as e:
        return name, [f"<error: {e}>"]
    diffs = []
    a, b = norm_log(tl / "probe.log"), norm_log(ours / "probe.log")
    if a != b:
        diffs += difflib.unified_diff(a, b, "upstream.log", "texyon.log", lineterm="", n=2)
    # TeXyoN keeps no output after a fatal error.
    fatal = any(l.startswith(("! TeX capacity exceeded", "! Emergency stop")) for l in b)
    a, b = xdv.pages(tl / "probe.xdv"), xdv.pages(ours / "probe.xdv")
    if a != b and not fatal:
        diffs += difflib.unified_diff(a, b, "upstream.xdv", "texyon.xdv", lineterm="", n=2)
    return name, diffs


def run_probes(args):
    for old in WORK.glob("*.diff"):
        old.unlink()
    bundle = make_bundle()
    fmtdir = make_tl_format(bundle)
    root = args.probes.resolve()
    todo = []
    for path in sorted(root.rglob("*.tex")):
        rel = path.relative_to(root)
        name = str(rel.with_suffix("")).replace("/", "-")
        if not args.filters or any(f in name or f in str(rel) for f in args.filters):
            todo.append((name, path))
    # The first probe builds TeXyoN's format, which the parallel ones then share.
    if todo:
        compare_probe(*todo[0], bundle, fmtdir)
    with ThreadPoolExecutor(args.jobs) as ex:
        results = list(ex.map(lambda t: compare_probe(*t, bundle, fmtdir), todo))

    known = load_list(KNOWN)
    passed = crashed = failed = allowed = 0
    for name, diffs in results:
        if not diffs:
            passed += 1
            if name in known:
                print(f"STALE  {name} is in known.tsv but now matches")
            continue
        if diffs[0].startswith("<crash"):
            crashed += 1
            print(f"CRASH  {name} ({diffs[0]})")
            continue
        if name in known:
            allowed += 1
            print(f"KNOWN  {name}")
        else:
            failed += 1
            print(f"DIFF   {name}")
        if args.verbose:
            print("\n".join(diffs[:200]))
        (WORK / f"{name}.diff").write_text("\n".join(diffs) + "\n")
    print(f"\n{len(todo)} probes: {passed} match, {allowed} known, {failed} diverge, "
          f"{crashed} upstream crashes (diffs in {WORK})")
    return failed > 0


def compare_case(tl, case, cache, keep, pixels):
    status, diffs = latex.compare(tl, case, cache, keep, pixels)
    if diffs:
        out = tl.work / "diffs" / (case.name.replace("/", "-") + ".diff")
        out.write_text("\n".join(diffs) + "\n")
    return case, status, diffs


def run_latex(args, suites):
    tl = latex.TeXLive(XETEX, TEXYON, WORK / "latex")
    tl.prepare()
    cases = [c for s in suites for c in latex.SUITES[s](tl)]
    if args.filters:
        cases = [c for c in cases if any(f in c.name for f in args.filters)]
    slow = load_list(SLOW)
    skipped = sum(c.name in slow for c in cases)
    cases = [c for c in cases if c.name not in slow]
    if skipped:
        print(f"skipping {skipped} cases listed in {SLOW.name}", flush=True)
    for c in cases:
        c.timeout *= args.timeout_scale
    cache = tl.work / "tl-cache"
    cache.mkdir(parents=True, exist_ok=True)
    diffdir = tl.work / "diffs"
    diffdir.mkdir(parents=True, exist_ok=True)
    for old in diffdir.glob("*.diff"):
        old.unlink()

    cases.sort(key=lambda c: c.cost(), reverse=True)
    tl.warm_texyon()
    results = []
    tally = {}

    def finished(n, case, status, diffs):
        results.append((case, status, diffs))
        tally[status] = tally.get(status, 0) + 1
        if status not in ("match", "match-nopages"):
            print(f"[{n}/{len(cases)}] {status.upper():16} {case.name}", flush=True)
        if n % 25 == 0 or n == len(cases):
            so_far = ", ".join(f"{v} {k}" for k, v in sorted(tally.items()))
            print(f"[{n}/{len(cases)}] so far: {so_far}", flush=True)

    requeued = []
    with latex.Governor(tl.work), ProcessPoolExecutor(args.jobs) as ex:
        futures = [ex.submit(compare_case, tl, c, cache, args.keep, args.pixels) for c in cases]
        n = 0
        for future in as_completed(futures):
            case, status, diffs = future.result()
            if status == "requeue":
                print(f"[{n}/{len(cases)}] {'REQUEUE':16} {case.name} (low memory, reruns alone)",
                      flush=True)
                requeued.append(case)
            else:
                n += 1
                finished(n, case, status, diffs)
        # One at a time, so the governor has nothing to stop.
        for case in requeued:
            n += 1
            finished(n, *compare_case(tl, case, cache, args.keep, args.pixels))

    known = load_list(KNOWN)
    counts = {}
    for case, status, diffs in results:
        if status.startswith("diverge") and case.name in known:
            status = "known"
        counts[(case.suite, status)] = counts.get((case.suite, status), 0) + 1
        if args.verbose and diffs:
            print(f"== {case.name}\n" + "\n".join(diffs[:200]))
    print()
    for suite in suites:
        row = {status: v for (s, status), v in counts.items() if s == suite}
        summary = ", ".join(f"{v} {k}" for k, v in sorted(row.items()))
        print(f"{suite}: {sum(row.values())} cases, {summary}")
    print(f"(diffs in {diffdir})")
    return any(s.startswith("diverge") or s == "timeout" for (_, s) in counts)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    ap.add_argument("-v", "--verbose", action="store_true", help="print diffs")
    ap.add_argument("-j", "--jobs", type=int, default=DEFAULT_JOBS)
    ap.add_argument("--probes", type=Path, default=PROBES, help="probe directory")
    ap.add_argument("--suite", default="probes",
                    help="comma-separated: probes, " + ", ".join(latex.SUITES) + ", or all")
    ap.add_argument("--keep", action="store_true", help="keep TeXyoN's outputs of diverging cases")
    ap.add_argument("--pixels", action="store_true", help="render diverging XDVs and compare their pages")
    ap.add_argument("--timeout-scale", type=int, default=1,
                    help="multiply each case's timeout, for documents that are just slow")
    ap.add_argument("filters", nargs="*", help="run only cases whose name contains one")
    args = ap.parse_args()

    if not Path(XETEX).exists():
        sys.exit(f"TeX Live's xetex not found at {XETEX} (set XETEX)")
    if not Path(TEXYON).exists():
        sys.exit(f"texyon not found at {TEXYON} (cargo build --release, or set TEXYON)")
    suites = ["probes", *latex.SUITES] if args.suite == "all" else args.suite.split(",")
    unknown = [s for s in suites if s != "probes" and s not in latex.SUITES]
    if unknown:
        sys.exit(f"unknown suite: {', '.join(unknown)}")

    WORK.mkdir(parents=True, exist_ok=True)
    latex_suites = [s for s in suites if s != "probes"]
    failed = run_latex(args, latex_suites) if latex_suites else False
    if "probes" in suites:
        failed = run_probes(args) or failed
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
