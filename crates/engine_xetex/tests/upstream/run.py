#!/usr/bin/env python3
"""Differential test: Tectonic's XeTeX engine against upstream TeX Live xetex.

Every probe in probes/ runs on both engines with the same plain format (built
by each engine from tests/assets/knuth-plain.tex) and the same CM TFMs. The
normalized .log files and decoded XDV pages must match.

    crates/engine_xetex/tests/upstream/run.py [-v] [probe-substring ...]

Custom test fonts live in fonts/*.pl and are compiled with pltotf. Probes
listed in known.tsv differ for a documented reason. Random documents for a
larger sweep come from fuzz/*.py:

    python3 crates/engine_xetex/tests/upstream/fuzz/layout.py /tmp/rand 1000 0
    crates/engine_xetex/tests/upstream/run.py --probes /tmp/rand

Env: XETEX (upstream binary), TECTONIC (default target/release/tectonic),
UPSTREAM_WORK (scratch dir, default target/upstream-diff).
"""

import argparse
import difflib
import hashlib
import os
import re
import shutil
import struct
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
ASSETS = ROOT / "tests" / "assets"
PROBES = HERE / "probes"
KNOWN = HERE / "known.tsv"
FONTS = HERE / "fonts"

XETEX = os.environ.get("XETEX", "/usr/local/texlive/2025/bin/x86_64-linux/xetex")
PLTOTF = os.environ.get("PLTOTF", str(Path(XETEX).with_name("pltotf")))
TECTONIC = os.environ.get("TECTONIC", str(ROOT / "target" / "release" / "tectonic"))
WORK = Path(os.environ.get("UPSTREAM_WORK", ROOT / "target" / "upstream-diff"))

ENV = dict(os.environ, TZ="UTC", SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")


def make_bundle():
    bundle = WORK / "bundle"
    if bundle.exists():
        shutil.rmtree(bundle)
    bundle.mkdir(parents=True)
    for f in ASSETS.iterdir():
        if f.suffix in (".tfm", ".otf", ".ttf", ".png", ".pdf") or f.name in ("knuth-plain.tex", "hyphen.tex"):
            shutil.copy(f, bundle / f.name)
    # Extra fonts are taken from the upstream TeX Live tree, both engines read the same copy.
    texmf = Path(XETEX).resolve().parents[2] / "texmf-dist"
    for rel in (FONTS / "texlive.txt").read_text().split():
        shutil.copy(texmf / rel, bundle / Path(rel).name)
    for pl in sorted(FONTS.glob("*.pl")):
        subprocess.run([PLTOTF, pl, bundle / (pl.stem + ".tfm")], check=True,
                       stdout=subprocess.DEVNULL)
    (bundle / "tectonic-format-plain.tex").write_text("\\input knuth-plain \\dump\n")
    # The format cache is keyed by this digest, so the binary goes into it.
    h = hashlib.sha256()
    for f in sorted(bundle.iterdir()):
        h.update(f.name.encode())
        h.update(f.read_bytes())
    h.update(Path(TECTONIC).read_bytes())
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
        # Tectonic's fixed capacities (engine.rs), so overflow messages agree.
        stack_size="5000",
        nest_size="500",
        param_size="10000",
        save_size="80000",
        buf_size="200000",
        max_in_open="15",
        expand_depth="10000",
        hash_extra="600000",
        font_mem_size="8000000",
        font_max="9000",
        pool_size="6250000",
        string_vacancies="90000",
        max_strings="565536",
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
        sys.exit("upstream xetex failed to build plain.fmt")
    return fmtdir


class UpstreamCrash(Exception):
    pass


def run_tl(name, src, bundle, fmtdir):
    d = WORK / "tl" / name
    shutil.rmtree(d, ignore_errors=True)
    d.mkdir(parents=True)
    shutil.copy(src, d / "probe.tex")
    # Pictures found in the probe directory print as ./name, like Tectonic's name.
    text = Path(src).read_bytes()
    if b"XeTeXpicfile" in text or b"XeTeXpdffile" in text:
        for f in bundle.iterdir():
            if f.suffix in (".png", ".pdf"):
                shutil.copy(f, d / f.name)
    r = subprocess.run(
        [XETEX, "-fmt=plain", "-no-pdf", "-interaction=scrollmode", "probe.tex"],
        cwd=d, env=tl_env(bundle, fmtdir), stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=60)
    if r.returncode < 0:
        raise UpstreamCrash(f"upstream xetex died with signal {-r.returncode}")
    return d


def run_tt(name, src, bundle):
    d = WORK / "tt" / name
    shutil.rmtree(d, ignore_errors=True)
    d.mkdir(parents=True)
    shutil.copy(src, d / "probe.tex")
    subprocess.run(
        [TECTONIC, "-X", "compile", "--outfmt", "xdv", "--keep-logs", "--untrusted",
         "-Z", "deterministic-mode", "-Z", "continue-on-errors", "--format", "plain",
         "--bundle", str(bundle), "probe.tex"],
        cwd=d, env=ENV, stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=60)
    return d


# Line rewrites for things Tectonic intentionally does differently.
LOG_SUBS = [
    # kpathsea reports the path it found.
    (re.compile(r"\(\./"), "("),
    (re.compile(r'(\\XeTeXp(ic|df)file) "\./'), r'\1 "'),
    # the XDV preamble comment and file-name lengths differ.
    (re.compile(r"^(Output written on \S+ \(\d+ pages?), \d+ bytes\)\.$"), r"\1)."),
]


def norm_log(path):
    if not path.exists():
        return ["<no log>"]
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    for i, l in enumerate(lines):
        if l.startswith("(./probe.tex") or l.startswith("(probe.tex"):
            lines = lines[i:]
            break
    out = []
    for l in lines:
        # Tectonic has no command line in the terminal buffer.
        if l == "<*> probe.tex":
            out += ["<*> ", "    "]
            continue
        if out[-2:] == ["<*> ", "    "] and l == " " * 13:
            continue
        for rx, rep in LOG_SUBS:
            l = rx.sub(rep, l)
        out.append(l)
    while out and out[-1] == "":
        out.pop()
    # Without \end upstream asks the terminal, Tectonic has none.
    text = "\n".join(out)
    text = text.replace("*\n! Emergency stop.\n<*> \n    \nEnd of file on the terminal!\n\n", "<no legal \\end>\n")
    text = text.replace("! Emergency stop\n*** (job aborted, no legal \\end found)\n", "<no legal \\end>\n")
    # Tectonic's empty terminal line yields one \par before it gives up.
    text = text.replace("{\\par}\n<no legal \\end>", "<no legal \\end>")
    return text.split("\n")


def xdv_pages(path):
    """Decode XDV/DVI page contents to text, skipping the preamble and postamble."""
    if not path.exists():
        return ["<no xdv>"]
    b = path.read_bytes()
    out = []
    i = 0

    def u(n):
        nonlocal i
        v = int.from_bytes(b[i:i + n], "big")
        i += n
        return v

    def s(n):
        nonlocal i
        v = int.from_bytes(b[i:i + n], "big", signed=True)
        i += n
        return v

    if u(1) != 247:
        return ["<bad xdv>"]
    u(1)
    out.append(f"pre {u(4)} {u(4)} {u(4)}")
    k = u(1)
    i += k
    while i < len(b):
        op = u(1)
        if op <= 127:
            out.append(f"setchar {op}")
        elif op <= 131:
            out.append(f"set{op - 127} {u(op - 127)}")
        elif op == 132:
            out.append(f"setrule {s(4)} {s(4)}")
        elif op <= 136:
            out.append(f"put{op - 132} {u(op - 132)}")
        elif op == 137:
            out.append(f"putrule {s(4)} {s(4)}")
        elif op == 138:
            out.append("nop")
        elif op == 139:
            i += 44
            out.append(f"bop {struct.unpack('>10i', b[i - 44:i - 4])}")
        elif op == 140:
            out.append("eop")
        elif op == 141:
            out.append("push")
        elif op == 142:
            out.append("pop")
        elif 143 <= op <= 146:
            out.append(f"right {s(op - 142)}")
        elif op in (147, 152, 161, 166):
            out.append(f"{ {147: 'w0', 152: 'x0', 161: 'y0', 166: 'z0'}[op]}")
        elif 148 <= op <= 151:
            out.append(f"w {s(op - 147)}")
        elif 153 <= op <= 156:
            out.append(f"x {s(op - 152)}")
        elif 157 <= op <= 160:
            out.append(f"down {s(op - 156)}")
        elif 162 <= op <= 165:
            out.append(f"y {s(op - 161)}")
        elif 167 <= op <= 170:
            out.append(f"z {s(op - 166)}")
        elif 171 <= op <= 234:
            out.append(f"fnt {op - 171}")
        elif 235 <= op <= 238:
            out.append(f"fnt {u(op - 234)}")
        elif 239 <= op <= 242:
            n = u(op - 238)
            out.append(f"xxx {b[i:i + n]!r}")
            i += n
        elif 243 <= op <= 246:
            f = u(op - 242)
            c, sz, ds = u(4), u(4), u(4)
            a, l = u(1), u(1)
            nm = b[i:i + a + l].decode("latin-1")
            i += a + l
            out.append(f"fntdef {f} {c} {sz} {ds} {nm}")
        elif op == 248:
            i += 4
            out.append(f"post {u(4)} {u(4)} {u(4)} {u(4)} {u(4)} {u(2)} {u(2)}")
        elif op == 249:
            break
        elif op == 252:
            f, sz, flags, n = u(4), u(4), u(2), u(1)
            # upstream records the path kpathsea found
            nm = b[i:i + n].rsplit(b"/", 1)[-1]
            i += n
            idx = u(4)
            extra = [u(4) for bit in (0x200, 0x1000, 0x2000, 0x4000) if flags & bit]
            out.append(f"nativefont {f} {sz} {flags} {nm!r} {idx} {extra}")
        elif op == 253:
            w, n = s(4), u(2)
            xy = [s(4) for _ in range(2 * n)]
            g = [u(2) for _ in range(n)]
            out.append(f"glyphs {w} {xy} {g}")
        elif op == 254:
            n = u(2)
            t = b[i:i + 2 * n]
            i += 2 * n
            w, n = s(4), u(2)
            xy = [s(4) for _ in range(2 * n)]
            g = [u(2) for _ in range(n)]
            out.append(f"textglyphs {t!r} {w} {xy} {g}")
        else:
            out.append(f"<op {op}>")
            break
    return out


def load_known():
    known = {}
    if KNOWN.exists():
        for l in KNOWN.read_text().splitlines():
            if l.strip() and not l.startswith("#"):
                name, reason = l.split("\t", 1)
                known[name] = reason
    return known


def compare(name, src, bundle, fmtdir):
    try:
        tl = run_tl(name, src, bundle, fmtdir)
        tt = run_tt(name, src, bundle)
    except UpstreamCrash as e:
        return name, [f"<crash: {e}>"]
    except (OSError, subprocess.TimeoutExpired) as e:
        return name, [f"<error: {e}>"]
    diffs = []
    a, log = norm_log(tl / "probe.log"), norm_log(tt / "probe.log")
    if a != log:
        diffs += list(difflib.unified_diff(a, log, "upstream.log", "tectonic.log", lineterm="", n=2))
    a, b = xdv_pages(tl / "probe.xdv"), xdv_pages(tt / "probe.xdv")
    # Tectonic keeps no outputs after a fatal error.
    fatal = any(l.startswith(("! TeX capacity exceeded", "! Emergency stop")) for l in log)
    if a != b and not fatal:
        diffs += list(difflib.unified_diff(a, b, "upstream.xdv", "tectonic.xdv", lineterm="", n=2))
    return name, diffs


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("-v", "--verbose", action="store_true", help="print diffs")
    ap.add_argument("-j", "--jobs", type=int, default=os.cpu_count())
    ap.add_argument("--probes", type=Path, default=PROBES, help="probe directory")
    ap.add_argument("filters", nargs="*")
    args = ap.parse_args()

    if not Path(XETEX).exists():
        sys.exit(f"upstream xetex not found at {XETEX} (set XETEX)")
    if not Path(TECTONIC).exists():
        sys.exit(f"tectonic not found at {TECTONIC} (cargo build --release -p tectonic)")

    WORK.mkdir(parents=True, exist_ok=True)
    for old in WORK.glob("*.diff"):
        old.unlink()
    bundle = make_bundle()
    fmtdir = make_tl_format(bundle)

    root = args.probes.resolve()
    probes = sorted(root.rglob("*.tex"))
    names = [str(p.relative_to(root).with_suffix("")).replace("/", "-") for p in probes]
    todo = [(n, p) for n, p in zip(names, probes)
            if not args.filters or any(f in n or f in str(p.relative_to(root)) for f in args.filters)]

    # Warm the Tectonic format cache once before running in parallel.
    if todo:
        compare(*todo[0], bundle, fmtdir)

    known = load_known()
    with ThreadPoolExecutor(args.jobs) as ex:
        results = list(ex.map(lambda t: compare(*t, bundle, fmtdir), todo))

    passed, crashed, failed, allowed = 0, 0, [], []
    for name, diffs in results:
        if not diffs:
            passed += 1
            if name in known:
                print(f"STALE  {name} is in known.tsv but now matches")
            continue
        if diffs[0].startswith("<crash"):
            print(f"CRASH  {name} ({diffs[0]})")
            crashed += 1
            continue
        (allowed if name in known else failed).append((name, diffs))
        tag = "KNOWN " if name in known else "DIFF  "
        print(f"{tag} {name}")
        if args.verbose:
            print("\n".join(diffs[:200]))
        (WORK / f"{name}.diff").write_text("\n".join(diffs) + "\n")

    print(f"\n{len(todo)} probes: {passed} match, {len(allowed)} known, {len(failed)} diverge,"
          f" {crashed} upstream crashes"
          f" (diffs in {WORK})")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
