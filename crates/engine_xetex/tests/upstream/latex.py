"""LaTeX suites for run.py: both engines read the same TeX Live tree.

TeX Live's xetex finds files with kpathsea. TeXyoN gets a directory bundle of
symlinks, one per file name, each to the file kpsewhich picks for xelatex, so
both engines read identical inputs.
"""

import fcntl
import gzip
import hashlib
import json
import os
import random
import re
import resource
import shutil
import signal
import subprocess
import tarfile
import threading
import time
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass, field
from pathlib import Path

import xdv

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
CORPUS = Path(os.environ.get("TEXYON_CORPUS_DIR", Path.home() / ".cache/texyon-corpus"))

# TeXyoN's fixed capacities, so overflow messages agree.
CAPACITIES = dict(
    stack_size="5000", nest_size="500", param_size="10000", save_size="80000",
    buf_size="200000", max_in_open="15", expand_depth="10000", hash_extra="600000",
    font_mem_size="8000000", font_max="9000", pool_size="6250000",
    string_vacancies="90000", max_strings="565536",
)
# TeXyoN's main memory, through extra_mem_top since token lists only grow there.
LATEX_CAPACITIES = dict(CAPACITIES, extra_mem_top=str((1 << 26) - 5_000_000))
# TeXyoN defines \pdfoutput, so upstream gets one for arXiv's usual first line.
FORMAT_RECIPE = "\\input tl-nostat \\countdef\\pdfoutput=32759 \\input xelatex.ini"
# Bump when the way upstream is run changes, so cached results are redone.
CACHE_VERSION = "6"

TL_TIMEOUT = "upstream xetex timed out"
NO_LOG, NO_XDV = "<no log>", ["<no xdv>"]
SKIP_TREES = ("doc", "source", "scripts")
FONT_SUFFIXES = (".tfm", ".vf", ".ofm", ".ovf", ".otf", ".ttf", ".ttc", ".otc", ".pfb", ".pfa",
                 ".afm", ".tec", ".fea", ".cmap")
MISC_FONT_SUFFIXES = (".tec", ".map", ".fea", ".cmap")
LOG_WIDTH = 79
PIXEL_DPI = "100"
REQUEUE = "requeue"

# At a line start only, since a document may quote them in its own text.
FATAL_RX = re.compile(r"^(?:\(That makes 100 errors; please try again\.\)|! Emergency stop\."
                      r"|! TeX capacity exceeded|\*\*\* \(job aborted)", re.M)
FONT_EXT_RX = re.compile(r"^(nativefont \d+ \d+ \d+ b['\"].*?)\.(otf|ttf|otc|ttc|dfont)(['\"])", re.I)
FONT_DEF_RX = re.compile(r"^(nativefont|fntdef) (\d+) ")
FONT_USE_RX = re.compile(r"^fnt (\d+)$")
# Dates TeX Live takes from its clock or file times, which TeXyoN pins to the epoch.
XMP_DATE_RX = re.compile(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d")
XMP_INSTANCE_RX = re.compile(r"(<xmpMM:InstanceID>uuid:)[0-9a-f-]+")
MOD_DATE_RX = re.compile(r"(/ModDate ?\(D:)[0-9]{14}")
# TeX's `(name` on opening a file, and `pdf:image ... (name)`.
NESTED_RX = re.compile(r"(?<=\()[\w.+-]+(?:/[\w.+-]+)+")
# Marks where a full-width log line was joined to the next, which may have been a real line end.
JOIN = "\0"

# Upstream bug 18: TeX Live reads the script list from GSUB twice, so fontspec leaves out
# scripts only GPOS lists and notes that the font lacks them.
SCRIPT_TAG_RX = re.compile(r"script=\w+;")
SCRIPT_NOTE_RX = re.compile(
    r"PackagefontspecInfo:(\(fontspec\))?Script'[^']*'notexplicitlysupportedwithinfont"
    r".{0,400}?differentfontshouldbechosen\.")
# Upstream bug 21: \XeTeXtracingfonts prints freed memory, up to a NUL, for fonts found by name.
REQUEST_BY_NAME_RX = re.compile(r'(Requestedfont"[^\["][^"]*"at[0-9.]+pt->)')
FONT_FILE_RX = re.compile(r"[\w.+-]*?\.(otf|ttf|otc|ttc|dfont)", re.I)
FREED_MEMORY_MAX = 64


@dataclass
class Case:
    """One document compiled by both engines from a fresh copy of its files."""
    name: str
    suite: str
    main: str
    files: list[tuple[Path, str]] = field(default_factory=list)  # source, name in the case dir
    passes: int = 1
    timeout: int = 300
    tarball: Path | None = None

    def key(self):
        h = hashlib.sha256(f"{self.main}|{self.passes}|".encode())
        for src, dest in sorted(self.files, key=lambda f: f[1]):
            h.update(dest.encode())
            h.update(Path(src).read_bytes())
        if self.tarball:
            h.update(self.tarball.read_bytes())
        return h.hexdigest()

    def cost(self):
        """Input size, a rough guess of run time for starting the big cases first."""
        size = self.tarball.stat().st_size * 4 if self.tarball else 0
        return size + sum(Path(src).stat().st_size for src, _ in self.files)

    def populate(self, d):
        if self.tarball:
            with tarfile.open(self.tarball) as t:
                t.extractall(d, filter="data")
        for src, dest in self.files:
            target = d / dest
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy(src, target)
        # \filemoddate reads real file times upstream, TeXyoN's deterministic mode the epoch.
        for f in d.rglob("*"):
            os.utime(f, (0, 0), follow_symlinks=False)


class TeXLive:
    """The TeX Live installation both engines read, with the bundle and format for it."""

    def __init__(self, xetex, texyon, work):
        self.bin = Path(xetex).parent
        self.xetex = xetex
        self.texyon = texyon
        self.work = work
        self.root = Path(xetex).resolve().parents[2]
        self.texmf = self.root / "texmf-dist"
        self.fontconfig = work / "fontconfig" / "fonts.conf"
        self.fmtdir = work / "tl-latex-fmt"
        self.bundle = work / "tl-bundle"
        self.clock = work / "epoch-clock.so"
        # A line break TeX made for width may fall anywhere in a path, see JOIN.
        self.path_rx = re.compile(
            "".join(re.escape(c) + "\0?" for c in f"{self.root}/") + r"(?:[^\s/()<>{}\"']+/)*")
        self.root_depth = len(self.root.parts)
        self._nested = None

    def nested(self):
        """The bundle's `dir/.../name` links."""
        if self._nested is None:
            self._nested = {str(f.relative_to(self.bundle)) for d in self.bundle.iterdir()
                            if d.is_dir() for f in d.rglob("*") if not f.is_dir()}
        return self._nested

    def env(self):
        return dict(
            os.environ, TZ="UTC", SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1", LC_ALL="C",
            PATH=f"{self.bin}:{os.environ.get('PATH', '')}", TEXMFHOME="/nonexistent",
            TEXMFVAR=str(self.work / "texmf-var"), FONTCONFIG_FILE=str(self.fontconfig),
            TEXFORMATS=f"{self.fmtdir}:", **LATEX_CAPACITIES,
            # TeXyoN never generates fonts, and a font made by one case would change later ones.
            MKTEXTFM="0", MKTEXMF="0", MKTEXPK="0",
        )

    def prepare(self):
        # Upstream overflows an 8 MB C stack in documents TeXyoN runs through (arxiv/2309.07781).
        _, hard = resource.getrlimit(resource.RLIMIT_STACK)
        resource.setrlimit(resource.RLIMIT_STACK, (hard, hard))
        self.work.mkdir(parents=True, exist_ok=True)
        self._build_clock()
        self._write_fontconfig()
        self._build_bundle()
        self._build_format()

    def _build_clock(self):
        """A preload that stops upstream's clock at the epoch.

        Upstream seeds `\\randomseed` from the clock even with SOURCE_DATE_EPOCH. At the epoch
        its seed is 0, as TeXyoN's is in deterministic mode.
        """
        src = self.work / "epoch-clock.c"
        src.write_text(
            "struct timeval { long tv_sec; long tv_usec; };\n"
            "int gettimeofday(struct timeval *tv, void *tz) {\n"
            "  (void)tz; if (tv) { tv->tv_sec = 0; tv->tv_usec = 0; } return 0;\n}\n")
        subprocess.run(["cc", "-shared", "-fPIC", "-O2", "-o", str(self.clock), str(src)], check=True)

    def _write_fontconfig(self):
        self.fontconfig.parent.mkdir(parents=True, exist_ok=True)
        dirs = "".join(f"  <dir>{self.texmf}/fonts/{d}</dir>\n" for d in ("opentype", "truetype"))
        self.fontconfig.write_text(
            '<?xml version="1.0"?>\n<!DOCTYPE fontconfig SYSTEM "fonts.dtd">\n<fontconfig>\n'
            f"{dirs}  <cachedir>{self.fontconfig.parent / 'cache'}</cachedir>\n</fontconfig>\n")

    def _kpsewhich(self, names, *opts):
        found = {}
        # Run elsewhere, a name like README.md resolves to the checkout's own copy.
        empty = self.work / "kpsewhich"
        empty.mkdir(parents=True, exist_ok=True)
        for i in range(0, len(names), 2000):
            r = subprocess.run(
                [str(self.bin / "kpsewhich"), "-progname=xelatex", "-engine=xetex", *opts,
                 *names[i:i + 2000]],
                cwd=empty, env=self.env(), stdout=subprocess.PIPE, text=True, check=False)
            for p in r.stdout.split("\n"):
                if p.startswith("/"):
                    found.setdefault(Path(p).name, p)
        return found

    def _build_bundle(self):
        names = set()
        for dirpath, dirs, files in os.walk(self.texmf):
            if os.path.relpath(dirpath, self.texmf).startswith(SKIP_TREES):
                dirs.clear()
                continue
            names.update(f for f in files if not re.search(r"\s", f) and not f.startswith("-"))
        names = sorted(names)
        # \input and \openin search TeX's path for any name, fonts come from the font paths.
        found = self._kpsewhich(names, "-format=tex")
        fonts = [n for n in names if n.lower().endswith(FONT_SUFFIXES) and n not in found]
        for name, path in self._kpsewhich(fonts).items():
            found.setdefault(name, path)
        misc = [n for n in names if n not in found and n.endswith(MISC_FONT_SUFFIXES)]
        found.update(self._kpsewhich(misc, "-format=misc fonts"))
        for skip in ("ls-R", "texmf.cnf"):
            found.pop(skip, None)

        shutil.rmtree(self.bundle, ignore_errors=True)
        self.bundle.mkdir(parents=True)
        h = hashlib.sha256()
        for name in sorted(found):
            os.symlink(found[name], self.bundle / name)
            st = os.stat(found[name])
            h.update(f"{name}\0{found[name]}\0{st.st_size}\0{st.st_mtime_ns}\n".encode())
        # kpathsea also finds `dir/.../name` below a `//` path entry, as TeXyoN's bundles do.
        tex_tree = self.texmf / "tex"
        for name, path in sorted(found.items()):
            if not path.startswith(f"{tex_tree}/"):
                continue
            dirs = Path(path).relative_to(tex_tree).parent.parts
            for i in range(len(dirs)):
                if dirs[i] in found:
                    continue
                link = self.bundle.joinpath(*dirs[i:], name)
                link.parent.mkdir(parents=True, exist_ok=True)
                if not link.exists():
                    os.symlink(path, link)
        # The engine looks formats up under the bundle names it kept from Tectonic.
        (self.bundle / "tectonic-format-latex.tex").write_text("\\input xelatex.ini\n")
        (self.bundle / "tectonic-format-plain.tex").write_text("\\input plain \\dump\n")
        # TeXyoN caches formats by this digest, so a new build must change it.
        h.update(Path(self.texyon).read_bytes())
        (self.bundle / "SHA256SUM").write_text(h.hexdigest() + "\n")

    def _build_format(self):
        t = hashlib.sha256(Path(self.xetex).read_bytes())
        for lsr in (self.texmf / "ls-R", self.root / "texmf-var" / "ls-R"):
            if lsr.exists():
                t.update(lsr.read_bytes())
        t.update(FORMAT_RECIPE.encode())
        t.update((HERE / "tl-nostat.tex").read_bytes())
        self.tl_digest = t.hexdigest()
        self.fmtdir.mkdir(parents=True, exist_ok=True)
        fmt = self.fmtdir / "xelatex.fmt"
        fmt.unlink(missing_ok=True)
        shutil.copy(HERE / "tl-nostat.tex", self.fmtdir)
        subprocess.run(
            [self.xetex, "-ini", "-etex", "-jobname=xelatex", "-progname=xelatex",
             "-interaction=batchmode", FORMAT_RECIPE],
            cwd=self.fmtdir, env=self.env(), stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL, check=False)
        if not fmt.exists():
            raise SystemExit("TeX Live's xetex failed to build xelatex.fmt")

    def run_tl(self, case, d):
        """None, or why upstream gave no result."""
        for _ in range(case.passes):
            r = subprocess.run(
                [self.xetex, "-fmt=xelatex", "-progname=xelatex", "-no-pdf", "-no-shell-escape",
                 "-interaction=nonstopmode", case.main],
                cwd=d, env=dict(self.env(), LD_PRELOAD=str(self.clock)), stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=case.timeout)
            if r.returncode < 0:
                return f"upstream xetex died with signal {-r.returncode}"
            # TeXyoN's reruns end at a fatal error, so upstream's must too.
            log = d / Path(case.main).with_suffix(".log").name
            if log.exists() and FATAL_RX.search(log.read_text(errors="replace")):
                break
        return None

    def warm_texyon(self):
        """Builds TeXyoN's format once, before the parallel runs would each build it."""
        d = self.work / "warm"
        shutil.rmtree(d, ignore_errors=True)
        d.mkdir(parents=True)
        (d / "warm.tex").write_text("\\documentclass{article}\\begin{document}x\\end{document}\n")
        subprocess.run(
            [self.texyon, "compile", "--outfmt", "xdv", "--untrusted",
             "-Z", "deterministic-mode", "--bundle", str(self.bundle), "warm.tex"],
            cwd=d, env=self.env(), stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL, check=False)
        shutil.rmtree(d, ignore_errors=True)

    def run_texyon(self, case, d):
        subprocess.run(
            [self.texyon, "compile", "--pass", "tex", "--reruns", str(case.passes - 1),
             "--outfmt", "xdv", "--keep-logs", "--keep-intermediates", "--untrusted",
             "-Z", "deterministic-mode", "-Z", "continue-on-errors",
             "--bundle", str(self.bundle), case.main],
            cwd=d, env=self.env(), stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            # One process runs every pass, and upstream gets the timeout per pass.
            timeout=case.timeout * case.passes)

    def strip_path(self, m):
        """A found path reduced to its file name, cut at a join that follows a file name."""
        segments = m.group(0).split("/")
        for k, seg in enumerate(segments[self.root_depth - 1:-1], start=self.root_depth - 1):
            if JOIN in seg and "." in seg.split(JOIN)[0]:
                return "/".join(segments[k:])
        return ""


def resolve_moves(lines):
    """Writes each w, x, y and z move as the plain right or down it stands for.

    Upstream's font records carry full paths, which shifts where the DVI buffer is
    flushed, and that decides whether TeX may still turn an earlier move into a
    register. Positions stay exact, only the encoding is dropped.
    """
    axis = {"w": "right", "x": "right", "y": "down", "z": "down"}
    regs = dict.fromkeys(axis, 0)
    stack = []
    out = []
    for line in lines:
        op, _, arg = line.partition(" ")
        if op in ("w0", "x0", "y0", "z0"):
            out.append(f"{axis[op[0]]} {regs[op[0]]}")
            continue
        if op in regs:
            regs[op] = int(arg)
            out.append(f"{axis[op]} {arg}")
            continue
        if op == "push":
            stack.append(dict(regs))
        elif op == "pop":
            regs = stack.pop() if stack else regs
        elif op == "bop":
            regs = dict.fromkeys(axis, 0)
            stack = []
        out.append(line)
    return out


def renumber_fonts(lines):
    """Numbers fonts in order of definition, since the internal numbers count every font loaded."""
    ids = {}
    out = []
    for line in lines:
        m = FONT_DEF_RX.match(line)
        if m:
            n = ids.setdefault(m.group(2), str(len(ids)))
            out.append(f"{m.group(1)} {n} {line[m.end():]}")
            continue
        m = FONT_USE_RX.match(line)
        out.append(f"fnt {ids.get(m.group(1), '?' + m.group(1))}" if m else line)
    return out


def drop_empty_pushes(lines):
    """Removes push/pop pairs with nothing between them.

    TeX drops such a pair itself, but only while the push is still in its buffer,
    so whether one survives depends on where the buffer was flushed.
    """
    out = []
    for line in lines:
        if line == "pop" and out and out[-1] == "push":
            out.pop()
        else:
            out.append(line)
    return out


def without_dates(line):
    if line.startswith("xxx b'pdf:stream @hyxmp@Metadata"):
        return XMP_INSTANCE_RX.sub(r"\1", XMP_DATE_RX.sub("", line))
    if line.startswith("xxx b'pdf:fstream"):
        return MOD_DATE_RX.sub(r"\1", line)
    return line


def normalize_xdv(tl, lines):
    """Decoded pages, native fonts named without the extension upstream's lookup adds."""
    lines = [FONT_EXT_RX.sub(r"\1\3", line) for line in lines]
    # Pictures from the TeX Live tree carry the path kpathsea found, as in the log.
    lines = [without_dates(tl.path_rx.sub("", line)) if line.startswith("xxx") else line
             for line in lines]
    return drop_empty_pushes(renumber_fonts(resolve_moves(lines)))


def strip_bundle_dirs(text, nested, case_dir):
    """A tree file asked for as `dir/.../name`, which upstream prints as a path reduced to its name."""
    def sub(m):
        rel = m.group(0)
        return rel.rsplit("/", 1)[1] if rel in nested and not (case_dir / rel).exists() else rel
    return NESTED_RX.sub(sub, text)


def strip_special_dirs(xdv_lines, nested, case_dir):
    """Both sides alike, since a macro writes a bitmap's name as asked and the engine a PDF's path."""
    return [strip_bundle_dirs(l, nested, case_dir) if l.startswith("xxx") else l for l in xdv_lines]


def latex_text(tl, raw, main):
    """The log from the main file on, unwrapped, with found paths reduced to names."""
    if raw is None:
        return NO_LOG
    out, buf = [], ""
    for line in raw.split("\n"):
        buf += line
        if len(line) == LOG_WIDTH:
            buf += JOIN
        else:
            out.append(buf)
            buf = ""
    if buf:
        out.append(buf)
    text = tl.path_rx.sub(tl.strip_path, "\n".join(out)).replace(JOIN, "").replace("(./", "(")
    text = re.sub(r"(Output written on \S+ \(\d+ pages?), \d+ bytes\)\.", r"\1).", text)
    # Upstream's terminal holds the command line, TeXyoN's is empty.
    text = text.replace("<*> " + main, "<*> ")
    start = text.find("(" + main)
    if start >= 0:
        text = text[start:]
    return text.rstrip()


def squeeze(text):
    # Found paths are longer upstream, so TeX breaks lines elsewhere, and a line that
    # happens to fill the width cannot be told from a wrapped one.
    return re.sub(r"\s+", "", text)


def without_script_choice(text):
    return SCRIPT_NOTE_RX.sub("", SCRIPT_TAG_RX.sub("", text))


def same_but_found_names(tl_log, log):
    """Equal once upstream's freed memory after each font request by name stands for TeXyoN's file name."""
    a, b = REQUEST_BY_NAME_RX.split(tl_log), REQUEST_BY_NAME_RX.split(log)
    if len(a) != len(b) or a[0] != b[0]:
        return False
    for i in range(1, len(a), 2):
        if a[i] != b[i]:
            return False
        name = FONT_FILE_RX.match(b[i + 1])
        ends = [0, name.end()] if name else [0]
        if not any(a[i + 1].endswith(b[i + 1][e:])
                   and len(a[i + 1]) - (len(b[i + 1]) - e) <= FREED_MEMORY_MAX for e in ends):
            return False
    return True


def same_with_bugs(tl_log, log, bugs):
    if 18 in bugs:
        tl_log, log = without_script_choice(tl_log), without_script_choice(log)
    return same_but_found_names(tl_log, log) if 21 in bugs else tl_log == log


def upstream_bugs_explain(tl_log, log):
    """The upstream bugs whose log effects are all that separates the logs, or None."""
    a, b = squeeze(tl_log), squeeze(log)
    every = {18, 21}
    if not same_with_bugs(a, b, every):
        return None
    return [n for n in sorted(every) if not same_with_bugs(a, b, every - {n})]


def log_diff(a, b, context=300):
    """Where two logs first differ once whitespace is ignored."""
    a, b = squeeze(a), squeeze(b)
    p = next((i for i in range(min(len(a), len(b))) if a[i] != b[i]), min(len(a), len(b)))
    lo = max(0, p - context)
    return ["--- upstream.log", "+++ texyon.log",
            f"@@ first difference at character {p} of {len(a)} vs {len(b)}, whitespace removed @@",
            "-" + a[lo:p + context], "+" + b[lo:p + context]]


def window_diff(a, b, label, context=3, limit=60):
    """The differing stretch of two line lists, without a quadratic diff."""
    p = 0
    while p < len(a) and p < len(b) and a[p] == b[p]:
        p += 1
    s = 0
    while s < len(a) - p and s < len(b) - p and a[-1 - s] == b[-1 - s]:
        s += 1
    lo = max(0, p - context)
    out = [f"--- upstream.{label}", f"+++ texyon.{label}",
           f"@@ first difference at line {p + 1}, {len(a) - p - s} vs {len(b) - p - s} lines differ @@"]
    out += [" " + l for l in a[lo:p]]
    out += ["-" + l for l in a[p:len(a) - s][:limit]]
    out += ["+" + l for l in b[p:len(b) - s][:limit]]
    out += [" " + l for l in a[len(a) - s:len(a) - s + context]]
    return out


def lvt_cases(_tl):
    cases = []
    for repo in ("latex2e", "latex3"):
        base = CORPUS / "suites" / repo
        if not base.exists():
            continue
        support = [f for f in (base / "support").glob("*") if f.is_file()]
        for tdir in sorted(base.glob("**/testfiles*")):
            if not tdir.is_dir() or tdir.name == "testfiles-disabled" or "support" in tdir.parts:
                continue
            local = [f for f in (tdir / "support").glob("*") if f.is_file()]
            files = [(f, f.name) for f in support + local]
            for lvt in sorted(tdir.glob("*.lvt")):
                # l3benchmark repeats until the timer advances, which deterministic mode freezes.
                if lvt.stem.startswith("m3benchmark"):
                    continue
                rel = lvt.relative_to(base).with_suffix("")
                cases.append(Case(name=f"{repo}/{rel}", suite="lvt", main=lvt.stem + ".tex",
                                  files=files + [(lvt, lvt.stem + ".tex")], passes=2, timeout=180))
    return cases


def find_main(d):
    """The top-level file of an arXiv submission."""
    readme = next(iter(d.glob("00README*")), None)
    if readme:
        for line in readme.read_text(errors="replace").splitlines():
            parts = line.split()
            if len(parts) >= 2 and parts[1] == "toplevelfile" and (d / parts[0]).exists():
                return parts[0]
    best = None
    for f in d.rglob("*.tex"):
        try:
            text = f.read_text(errors="replace")
        except OSError:
            continue
        body = re.sub(r"(?m)(?<!\\)%.*$", "", text)
        if not re.search(r"\\document(class|style)", body) or "\\begin{document}" not in body:
            continue
        rank = (f.parent == d, f.stem in ("main", "ms", "paper", "article"), len(text))
        if best is None or rank > best[0]:
            best = (rank, f)
    return str(best[1].relative_to(d)) if best else None


def arxiv_cases(_tl):
    cases = []
    arxiv = CORPUS / "arxiv"
    index = arxiv / "mains.json"
    mains = json.loads(index.read_text()) if index.exists() else {}
    changed = False
    for tarball in sorted(arxiv.glob("*.tar.gz")):
        ident = tarball.name.removesuffix(".tar.gz")
        if ident not in mains:
            tmp = arxiv / ".probe"
            shutil.rmtree(tmp, ignore_errors=True)
            tmp.mkdir()
            try:
                with tarfile.open(tarball) as t:
                    t.extractall(tmp, filter="data")
                mains[ident] = find_main(tmp)
            except (tarfile.TarError, OSError):
                mains[ident] = None
            shutil.rmtree(tmp, ignore_errors=True)
            changed = True
        if mains[ident]:
            cases.append(Case(name=f"arxiv/{ident}", suite="arxiv", main=mains[ident],
                              passes=2, timeout=600, tarball=tarball))
    if changed:
        index.write_text(json.dumps(mains, indent=0, sort_keys=True))
    return cases


def manifest_cases(_tl):
    """The authored documents of tests/corpus/manifest.tsv."""
    corpus = ROOT / "crates/engine_xetex/tests/corpus"
    cases = []
    for line in (corpus / "manifest.tsv").read_text().splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        _tier, ident, rel, _src = line.split("\t")
        main = corpus / rel
        cases.append(Case(name=f"docs/{ident}", suite="docs", main=main.name,
                          files=[(main, main.name)], passes=2, timeout=900))
    return cases


def tldocs_cases(tl, max_dir_bytes=40_000_000):
    """Package documentation sources shipped in TeX Live's doc tree."""
    cases = []
    doc = tl.texmf / "doc"
    sizes = {}
    for area in ("latex", "xelatex", "generic"):
        for tex in sorted((doc / area).rglob("*.tex")):
            try:
                text = tex.read_text(errors="replace")
            except OSError:
                continue
            if not re.search(r"(?m)^\s*\\documentclass", text) or "\\begin{document}" not in text:
                continue
            top = tex.parent
            if top not in sizes:
                sizes[top] = sum(f.stat().st_size for f in top.rglob("*") if f.is_file())
            if sizes[top] > max_dir_bytes:
                continue
            files = [(f, str(f.relative_to(top))) for f in top.rglob("*")
                     if f.is_file() and f.suffix not in (".log", ".aux")
                     and not (f.suffix == ".pdf" and f.stem == tex.stem)]
            cases.append(Case(name=f"tldocs/{tex.relative_to(doc).with_suffix('')}", suite="tldocs",
                              main=tex.name, files=files, passes=2, timeout=600))
    return cases


SUITES = {"lvt": lvt_cases, "arxiv": arxiv_cases, "docs": manifest_cases, "tldocs": tldocs_cases}


def available_kib():
    """Free memory, or the room left under this process's cgroup limit if that is smaller."""
    with open("/proc/meminfo") as f:
        free = next(int(l.split()[1]) for l in f if l.startswith("MemAvailable:"))
    try:
        group = Path("/proc/self/cgroup").read_text().split("::", 1)[1].strip().lstrip("/")
        group = Path("/sys/fs/cgroup") / group
        limit = (group / "memory.max").read_text().strip()
        if limit != "max":
            stat = dict(l.split() for l in (group / "memory.stat").read_text().splitlines())
            # Page cache the kernel can drop counts as free, as MemAvailable does.
            used = int((group / "memory.current").read_text()) - int(stat["inactive_file"])
            free = min(free, (int(limit) - used) // 1024)
    except (OSError, IndexError, KeyError, ValueError):
        pass
    return free


def wait_for_memory(lock, min_kib=2_500_000, settle=0.5):
    """Starts cases one at a time, each once the machine has memory to spare."""
    with open(lock, "w") as f:
        fcntl.flock(f, fcntl.LOCK_EX)
        while available_kib() < min_kib:
            time.sleep(1 + random.random())
        # Lets the previous case grow before the next one reads the free memory.
        time.sleep(settle)


class Governor:
    """Keeps parallel cases from exhausting memory, by stopping the newest to rerun it alone."""

    def __init__(self, work, min_kib=1_500_000):
        self.work = work.resolve()
        self.min_kib = min_kib
        self.stop = threading.Event()
        self.thread = threading.Thread(target=self._watch, daemon=True)

    def __enter__(self):
        (self.work / REQUEUE).mkdir(parents=True, exist_ok=True)
        self.thread.start()
        return self

    def __exit__(self, *exc):
        self.stop.set()
        self.thread.join()

    def _watch(self):
        while not self.stop.wait(0.25):
            if available_kib() < self.min_kib and self._evict():
                # Killed engines take a moment to hand their memory back.
                self.stop.wait(3)

    def _evict(self):
        """Kills the engines of the most recently started case, if another case is running."""
        cases = {}
        for p in Path("/proc").iterdir():
            if not p.name.isdigit():
                continue
            try:
                rel = Path(os.readlink(p / "cwd")).relative_to(self.work)
                start = int((p / "stat").read_text().rsplit(")", 1)[1].split()[19])
            except (OSError, ValueError, IndexError):
                continue
            if len(rel.parts) < 2 or rel.parts[0] not in ("texyon", "tl"):
                continue
            first, pids = cases.get(rel.parts[1], (start, []))
            cases[rel.parts[1]] = (min(first, start), pids + [int(p.name)])
        if len(cases) < 2:
            return False
        newest = max(cases, key=lambda c: cases[c][0])
        (self.work / REQUEUE / newest).touch()
        for pid in cases[newest][1]:
            try:
                os.kill(pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        return True


@dataclass
class Run:
    """One case's directories and results, upstream's possibly from the cache."""
    case: Case
    tl_dir: Path
    texyon_dir: Path
    tl_res: dict | None = None
    texyon_res: tuple | None = None
    rerun_tl: object = None

    @property
    def stem(self):
        return Path(self.case.main).with_suffix("")


def compare(tl, case, cache_dir, keep_failures, pixels=False):
    """Compares both engines on one case. Returns the status and diff lines."""
    safe = case.name.replace("/", "-")
    run = Run(case, tl.work / "tl" / safe, tl.work / "texyon" / safe)
    key = hashlib.sha256((case.key() + tl.tl_digest + CACHE_VERSION).encode()).hexdigest()
    cached = cache_dir / f"{safe}.json.gz"
    if cached.exists():
        try:
            data = json.loads(gzip.decompress(cached.read_bytes()))
            # A timeout only stands for runs given no more time than it had.
            timed_out = data.get("crash") == TL_TIMEOUT and data.get("timeout", 0) < case.timeout
            if data.get("key") == key and not timed_out:
                run.tl_res = data
        except (OSError, ValueError):
            pass
    requeued = tl.work / REQUEUE / safe
    requeued.unlink(missing_ok=True)
    wait_for_memory(tl.work / "start.lock")

    def upstream():
        shutil.rmtree(run.tl_dir, ignore_errors=True)
        run.tl_dir.mkdir(parents=True)
        case.populate(run.tl_dir)
        try:
            crash = tl.run_tl(case, run.tl_dir)
        except subprocess.TimeoutExpired:
            crash = TL_TIMEOUT
        if requeued.exists():
            return None
        # Upstream writes its outputs to the working directory, TeXyoN next to the input.
        log_path = run.tl_dir / f"{run.stem.name}.log"
        res = dict(key=key, crash=crash, timeout=case.timeout,
                   log=log_path.read_text(encoding="utf-8", errors="replace")
                   if log_path.exists() else None,
                   xdv=xdv.pages(run.tl_dir / f"{run.stem.name}.xdv"))
        cached.write_bytes(gzip.compress(json.dumps(res).encode(), 3))
        return res

    def ours():
        d = run.texyon_dir
        shutil.rmtree(d, ignore_errors=True)
        d.mkdir(parents=True)
        case.populate(d)
        try:
            tl.run_texyon(case, d)
        except subprocess.TimeoutExpired:
            return None
        log_path = d / f"{run.stem}.log"
        log = latex_text(tl, log_path.read_text(encoding="utf-8", errors="replace"), case.main) \
            if log_path.exists() else NO_LOG
        pages = normalize_xdv(tl, xdv.pages(d / f"{run.stem}.xdv"))
        return strip_bundle_dirs(log, tl.nested(), d), strip_special_dirs(pages, tl.nested(), d)

    with ThreadPoolExecutor(2) as ex:
        tl_future = ex.submit(upstream) if run.tl_res is None else None
        texyon_future = ex.submit(ours)
        if tl_future:
            run.tl_res = tl_future.result()
        run.texyon_res = texyon_future.result()
    if requeued.exists():
        shutil.rmtree(run.tl_dir, ignore_errors=True)
        shutil.rmtree(run.texyon_dir, ignore_errors=True)
        return "requeue", []
    run.rerun_tl = None if tl_future else upstream
    try:
        return judge(tl, run, pixels)
    finally:
        shutil.rmtree(run.tl_dir, ignore_errors=True)
        if run.texyon_res is None or not keep_failures:
            shutil.rmtree(run.texyon_dir, ignore_errors=True)


def judge(tl, run, pixels):
    """The status and diff lines of a finished run."""
    if run.tl_res["crash"]:
        return "tl-crash", [run.tl_res["crash"]]
    if run.texyon_res is None:
        return "timeout", ["<texyon timed out>"]
    log, pages = run.texyon_res
    tl_log = latex_text(tl, run.tl_res["log"], run.case.main)
    tl_pages = strip_special_dirs(normalize_xdv(tl, run.tl_res["xdv"]), tl.nested(), run.texyon_dir)
    no_pages = not any(line.startswith("bop") for line in tl_pages)
    diffs = []
    if squeeze(tl_log) != squeeze(log):
        diffs += log_diff(tl_log, log)
    # TeXyoN keeps no output after a fatal error, upstream keeps the pages shipped so far.
    fatal = pages == NO_XDV and FATAL_RX.search(run.tl_res["log"] or "") is not None
    if tl_pages != pages and not fatal:
        diffs += window_diff(tl_pages, pages, "xdv")
    if not diffs:
        shutil.rmtree(run.texyon_dir, ignore_errors=True)
        if fatal:
            return "match-fatal", []
        return ("match-nopages" if no_pages else "match"), []
    bugs = upstream_bugs_explain(tl_log, log) if tl_pages == pages or fatal else None
    if bugs is not None:
        shutil.rmtree(run.texyon_dir, ignore_errors=True)
        return "match-upstream", [f"upstream bugs: {', '.join(map(str, bugs))}"] + diffs
    if pixels and tl_pages != pages and not no_pages and pages != NO_XDV:
        if run.rerun_tl:
            run.rerun_tl()
        same, note = same_pixels(tl, run.case, run.tl_dir / f"{run.stem.name}.xdv",
                                 run.texyon_dir / f"{run.stem}.xdv",
                                 tl.work / "pixels" / run.case.name.replace("/", "-"))
        diffs.append(note)
        if same:
            return "diverge-samepixels", diffs
    return ("diverge-nopages" if no_pages else "diverge"), diffs


def rasterize(tl, xdv_path, out, timeout):
    """Pages of an XDV as grayscale PNGs, through upstream's xdvipdfmx, or None."""
    out.mkdir(parents=True, exist_ok=True)
    pdf = out / "out.pdf"
    try:
        subprocess.run([str(tl.bin / "xdvipdfmx"), "-q", "-o", str(pdf), xdv_path.name],
                       cwd=xdv_path.parent, env=tl.env(), stdin=subprocess.DEVNULL,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=timeout,
                       check=False)
        if not pdf.exists():
            return None
        subprocess.run(["pdftoppm", "-r", PIXEL_DPI, "-gray", "-png", str(pdf), str(out / "p")],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=timeout,
                       check=False)
    except subprocess.TimeoutExpired:
        return None
    finally:
        pdf.unlink(missing_ok=True)
    return sorted(out.glob("p-*.png"))


def same_pixels(tl, case, tl_xdv, texyon_xdv, out):
    """Whether both XDVs render to the same pages, and a note for the diff."""
    shutil.rmtree(out, ignore_errors=True)
    a = rasterize(tl, tl_xdv, out / "upstream", case.timeout)
    b = rasterize(tl, texyon_xdv, out / "texyon", case.timeout)
    if not a or not b:
        return False, f"pixels: {'upstream' if not a else 'TeXyoN'} XDV did not render"
    if len(a) != len(b):
        return False, f"pixels: {len(a)} pages upstream, {len(b)} TeXyoN (in {out})"
    differ = [n for n, (x, y) in enumerate(zip(a, b), 1) if x.read_bytes() != y.read_bytes()]
    if not differ:
        shutil.rmtree(out, ignore_errors=True)
        return True, f"pixels: all {len(a)} pages equal at {PIXEL_DPI} dpi"
    for n, (x, y) in enumerate(zip(a, b), 1):
        if n not in differ:
            x.unlink()
            y.unlink()
    shown = ", ".join(map(str, differ[:20])) + (" ..." if len(differ) > 20 else "")
    return False, f"pixels: {len(differ)} of {len(a)} pages differ ({shown}), PNGs in {out}"
