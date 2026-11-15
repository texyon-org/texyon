"""LaTeX suites for run.py: both engines read the same TeX Live tree.

TeX Live's xetex runs with its own kpathsea lookup. TeXyoN gets a directory
bundle of symlinks, one per file name, each pointing at the file kpsewhich
picks for xelatex, so both engines see identical inputs.
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
import subprocess
import tarfile
import time
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
CORPUS = Path(os.environ.get("TEXYON_CORPUS_DIR", Path.home() / ".cache/tectonic-corpus"))

# Same as run.py's plain probes: Tectonic's fixed capacities. Main memory gets TeXyoN's
# `MAIN_MEMORY_WORDS`, through extra_mem_top since token lists can only grow there.
CAPACITIES = dict(
    stack_size="5000", nest_size="500", param_size="10000", save_size="80000",
    buf_size="200000", max_in_open="15", expand_depth="10000", hash_extra="600000",
    font_mem_size="8000000", font_max="9000", pool_size="6250000",
    string_vacancies="90000", max_strings="565536", extra_mem_top=str((1 << 26) - 5_000_000),
)
SKIP_TREES = ("doc", "source", "scripts")
MISC_FONT_SUFFIXES = (".tec", ".map", ".fea", ".cmap")
FONT_SUFFIXES = (".tfm", ".vf", ".ofm", ".ovf", ".otf", ".ttf", ".ttc", ".otc", ".pfb", ".pfa",
                 ".afm", ".tec", ".fea", ".cmap")


@dataclass
class Case:
    """One document compiled by both engines from a fresh copy of its files."""
    name: str
    suite: str
    files: list  # (source path, name inside the case directory)
    main: str
    passes: int = 1
    timeout: int = 300
    tarball: Path | None = None
    key_extra: str = ""

    def key(self):
        h = hashlib.sha256(f"{self.main}|{self.passes}|{self.key_extra}".encode())
        for src, dest in sorted(self.files, key=lambda f: f[1]):
            h.update(dest.encode())
            h.update(Path(src).read_bytes())
        if self.tarball:
            h.update(self.tarball.read_bytes())
        return h.hexdigest()

    def cost(self):
        """Input size, a rough guess of run time for scheduling the big cases first."""
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
        self.texmf = Path(xetex).resolve().parents[2] / "texmf-dist"
        self.fontconfig = work / "fontconfig" / "fonts.conf"
        self.fmtdir = work / "tl-latex-fmt"
        self.bundle = work / "tl-bundle"
        self._nested = None

    def nested(self):
        """The bundle's `dir/name` links."""
        if self._nested is None:
            self._nested = {f"{d.name}/{f.name}" for d in self.bundle.iterdir() if d.is_dir()
                            for f in d.iterdir()}
        return self._nested

    def env(self):
        return dict(
            os.environ, TZ="UTC", SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1", LC_ALL="C",
            PATH=f"{self.bin}:{os.environ.get('PATH', '')}", TEXMFHOME="/nonexistent",
            TEXMFVAR=str(self.work / "texmf-var"), FONTCONFIG_FILE=str(self.fontconfig),
            TEXFORMATS=f"{self.fmtdir}:", **CAPACITIES,
            # TeXyoN never generates fonts, and a font made by one case would change later ones.
            MKTEXTFM="0", MKTEXMF="0", MKTEXPK="0",
        )

    def prepare(self):
        # Upstream overflows an 8 MB C stack in documents TeXyoN runs through (arxiv/2309.07781).
        _, hard = resource.getrlimit(resource.RLIMIT_STACK)
        resource.setrlimit(resource.RLIMIT_STACK, (hard, hard))
        self.work.mkdir(parents=True, exist_ok=True)
        self._clock()
        self._fontconfig()
        self._bundle()
        self._format()

    def _clock(self):
        """A preload that stops upstream's clock at the epoch.

        Upstream seeds `\\randomseed` from the clock even with SOURCE_DATE_EPOCH. At the epoch
        its seed is 0, as TeXyoN's is in deterministic mode with a build date of 0.
        """
        self.clock = self.work / "epoch-clock.so"
        src = self.work / "epoch-clock.c"
        src.write_text(
            "struct timeval { long tv_sec; long tv_usec; };\n"
            "int gettimeofday(struct timeval *tv, void *tz) {\n"
            "  (void)tz; if (tv) { tv->tv_sec = 0; tv->tv_usec = 0; } return 0;\n}\n")
        subprocess.run(["cc", "-shared", "-fPIC", "-O2", "-o", str(self.clock), str(src)],
                       check=True)

    def _fontconfig(self):
        self.fontconfig.parent.mkdir(parents=True, exist_ok=True)
        dirs = "".join(f"  <dir>{self.texmf}/fonts/{d}</dir>\n" for d in ("opentype", "truetype"))
        self.fontconfig.write_text(
            '<?xml version="1.0"?>\n<!DOCTYPE fontconfig SYSTEM "fonts.dtd">\n<fontconfig>\n'
            f"{dirs}  <cachedir>{self.fontconfig.parent / 'cache'}</cachedir>\n</fontconfig>\n")

    def _kpsewhich(self, names, *opts):
        found = {}
        for i in range(0, len(names), 2000):
            r = subprocess.run(
                [str(self.bin / "kpsewhich"), "-progname=xelatex", "-engine=xetex", *opts,
                 *names[i:i + 2000]],
                env=self.env(), stdout=subprocess.PIPE, text=True, check=False)
            for p in r.stdout.split("\n"):
                if p.startswith("/"):
                    found.setdefault(Path(p).name, p)
        return found

    def _bundle(self):
        names = set()
        for dirpath, dirs, files in os.walk(self.texmf):
            rel = os.path.relpath(dirpath, self.texmf)
            if rel.startswith(SKIP_TREES):
                dirs.clear()
                continue
            names.update(f for f in files if not re.search(r"\s", f) and not f.startswith("-"))
        names = sorted(names)
        # What TeX itself can open: \input and \openin search TeX's path for any name,
        # and fonts come from the font paths.
        found = self._kpsewhich(names, "-format=tex")
        fonts = [n for n in names if n.lower().endswith(FONT_SUFFIXES) and n not in found]
        for name, path in self._kpsewhich(fonts).items():
            found.setdefault(name, path)
        misc = [n for n in names if n not in found and n.endswith(MISC_FONT_SUFFIXES)]
        found.update(self._kpsewhich(misc, "-format=misc fonts"))
        for skip in ("ls-R", "texmf.cnf"):
            found.pop(skip, None)
        if self.bundle.exists():
            shutil.rmtree(self.bundle)
        self.bundle.mkdir(parents=True)
        h = hashlib.sha256()
        for name in sorted(found):
            os.symlink(found[name], self.bundle / name)
            st = os.stat(found[name])
            h.update(f"{name}\0{found[name]}\0{st.st_size}\0{st.st_mtime_ns}\n".encode())
        # kpathsea also finds `dir/name` below a `//` path entry, as TeXyoN's bundles do.
        tex_tree = str(self.texmf / "tex") + "/"
        for name, path in sorted(found.items()):
            parent = Path(path).parent.name
            if path.startswith(tex_tree) and parent not in found:
                (self.bundle / parent).mkdir(exist_ok=True)
                link = self.bundle / parent / name
                if not link.exists():
                    os.symlink(path, link)
        (self.bundle / "tectonic-format-latex.tex").write_text("\\input xelatex.ini\n")
        (self.bundle / "tectonic-format-plain.tex").write_text("\\input plain \\dump\n")
        # Formats are cached by this digest, and a new engine build must not reuse them.
        h.update(Path(self.texyon).read_bytes())
        (self.bundle / "SHA256SUM").write_text(h.hexdigest() + "\n")
        self.digest = h.hexdigest()

    def _format(self):
        t = hashlib.sha256(Path(self.xetex).read_bytes())
        for lsr in (self.texmf / "ls-R", self.texmf.parent / "texmf-var" / "ls-R"):
            if lsr.exists():
                t.update(lsr.read_bytes())
        t.update(FORMAT_RECIPE.encode())
        t.update((HERE / "tl-nostat.tex").read_bytes())
        self.tl_digest = t.hexdigest()
        self.fmtdir.mkdir(parents=True, exist_ok=True)
        fmt = self.fmtdir / "xelatex.fmt"
        if fmt.exists():
            fmt.unlink()
        shutil.copy(HERE / "tl-nostat.tex", self.fmtdir)
        subprocess.run(
            [self.xetex, "-ini", "-etex", "-jobname=xelatex", "-progname=xelatex",
             "-interaction=batchmode", FORMAT_RECIPE],
            cwd=self.fmtdir, env=self.env(), stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL, check=False)
        if not fmt.exists():
            raise SystemExit("upstream xetex failed to build xelatex.fmt")

    def run_tl(self, case, d):
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

    def run_tt(self, case, d):
        subprocess.run(
            [self.texyon, "-X", "compile", "--pass", "tex", "--reruns", str(case.passes - 1),
             "--outfmt", "xdv", "--keep-logs", "--keep-intermediates", "--untrusted",
             "-Z", "deterministic-mode", "-Z", "continue-on-errors",
             "--bundle", str(self.bundle), case.main],
            cwd=d, env=self.env(), stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            # One process runs every pass, and upstream gets the timeout per pass.
            timeout=case.timeout * case.passes)


FONT_EXT_RX = re.compile(r"^(nativefont \d+ \d+ \d+ b['\"].*?)\.(otf|ttf|otc|ttc|dfont)(['\"])", re.I)


def resolve_moves(lines):
    """Writes each w, x, y and z move as the plain right or down it stands for.

    Upstream's font records carry full paths, which shifts where the DVI buffer is
    flushed, and that decides whether TeX may still turn an earlier move into a
    register. Positions stay exact, only the encoding is dropped.
    """
    regs = {"w": 0, "x": 0, "y": 0, "z": 0}
    stack = []
    axis = {"w": "right", "x": "right", "y": "down", "z": "down"}
    out = []
    for line in lines:
        op, _, arg = line.partition(" ")
        if op in ("w0", "x0", "y0", "z0"):
            out.append(f"{axis[op[0]]} {regs[op[0]]}")
        elif op in regs:
            regs[op] = int(arg)
            out.append(f"{axis[op]} {arg}")
        elif op == "push":
            stack.append(dict(regs))
            out.append(line)
        elif op == "pop":
            regs = stack.pop() if stack else regs
            out.append(line)
        elif op == "bop":
            regs = {"w": 0, "x": 0, "y": 0, "z": 0}
            stack = []
            out.append(line)
        else:
            out.append(line)
    return out


# Bump when the way upstream is run changes, so cached results are redone.
CACHE_VERSION = "6"
# At a line start only, since a document may quote them in its own text.
FATAL_RX = re.compile(r"^(?:\(That makes 100 errors; please try again\.\)|! Emergency stop\."
                      r"|! TeX capacity exceeded|\*\*\* \(job aborted)", re.M)
FONT_DEF_RX = re.compile(r"^(nativefont|fntdef) (\d+) ")
FONT_USE_RX = re.compile(r"^fnt (\d+)$")


def renumber_fonts(lines):
    """Numbers fonts in order of definition: the internal numbers count every font loaded."""
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


# Dates TL takes from its clock or file mtimes, which TeXyoN pins to the epoch: hyperxmp's XMP
# dates and the InstanceID seeded from them, and embedded files' ModDate.
XMP_DATE_RX = re.compile(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d")
XMP_INSTANCE_RX = re.compile(r"(<xmpMM:InstanceID>uuid:)[0-9a-f-]+")
MOD_DATE_RX = re.compile(r"(/ModDate ?\(D:)[0-9]{14}")


def without_dates(line):
    if line.startswith("xxx b'pdf:stream @hyxmp@Metadata"):
        return XMP_INSTANCE_RX.sub(r"\1", XMP_DATE_RX.sub("", line))
    if line.startswith("xxx b'pdf:fstream"):
        return MOD_DATE_RX.sub(r"\1", line)
    return line


def normalize_xdv(lines):
    """Decoded pages, native fonts named without the extension upstream's lookup adds."""
    lines = [FONT_EXT_RX.sub(r"\1\3", line) for line in lines]
    # Pictures from the TeX Live tree carry the path kpathsea found, as in the log.
    lines = [without_dates(PATH_RX.sub("", line)) if line.startswith("xxx") else line
             for line in lines]
    return drop_empty_pushes(renumber_fonts(resolve_moves(lines)))






def xdv_lines(path, xdv_pages):
    return normalize_xdv(xdv_pages(path))


# TeXyoN keeps Tectonic's \pdfoutput, so arXiv's usual first line works there.
FORMAT_RECIPE = "\\input tl-nostat \\countdef\\pdfoutput=32759 \\input xelatex.ini"
# A line break TeX made for width may fall anywhere in the prefix, see JOIN.
PATH_RX = re.compile(
    "".join(re.escape(c) + "\0?" for c in "/usr/local/texlive/") + r"[\d\0]+/(?:[^\s/()<>{}\"']+/)*")


# Marks where a full-width line was joined to the next, which may have been a real line end.
JOIN = "\0"


def strip_path(m):
    """A found path reduced to its file name, ending it at a join that follows a file name."""
    segments = m.group(0).split("/")
    for k, seg in enumerate(segments[4:-1], start=4):
        if JOIN in seg and "." in seg.split(JOIN)[0]:
            return "/".join(segments[k:])
    return ""


# Only found names: TeX's `(name` on opening a file, and `pdf:image ... (name)`.
NESTED_RX = re.compile(r"(?<=\()[\w.+-]+/[\w.+-]+")


def strip_bundle_dirs(text, nested, case_dir):
    """A tree file asked for as `dir/name`, which upstream prints as a path reduced to its name."""
    def sub(m):
        rel = m.group(0)
        return rel.split("/", 1)[1] if rel in nested and not (case_dir / rel).exists() else rel
    return NESTED_RX.sub(sub, text)


def strip_special_dirs(xdv, nested, case_dir):
    """Both sides alike, since a macro writes a bitmap's name as asked and the engine a PDF's path."""
    return [strip_bundle_dirs(l, nested, case_dir) if l.startswith("xxx") else l for l in xdv]


def latex_log(path, main):
    if not path.exists():
        return "<no log>"
    return latex_text(path.read_text(encoding="utf-8", errors="replace"), main)


def latex_text(raw, main, width=79):
    """The log from the main file on, unwrapped, with found paths reduced to names."""
    if raw is None:
        return "<no log>"
    lines = raw.split("\n")
    out, buf = [], ""
    for line in lines:
        buf += line
        if len(line) != width:
            out.append(buf)
            buf = ""
        else:
            buf += JOIN
    if buf:
        out.append(buf)
    text = "\n".join(out)
    text = PATH_RX.sub(strip_path, text).replace(JOIN, "").replace("(./", "(")
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


def same_log(a, b):
    return squeeze(a) == squeeze(b)


# Upstream bug 18: TL reads the script list from GSUB twice, so fontspec leaves out scripts only
# GPOS lists and notes that the font lacks them.
SCRIPT_TAG_RX = re.compile(r"script=\w+;")
SCRIPT_NOTE_RX = re.compile(
    r"PackagefontspecInfo:(\(fontspec\))?Script'[^']*'notexplicitlysupportedwithinfont"
    r".{0,400}?differentfontshouldbechosen\.")
# Upstream bug 21: TL's \XeTeXtracingfonts prints freed memory, up to a NUL, for fonts found by name.
REQUEST_BY_NAME_RX = re.compile(r'(Requestedfont"[^\["][^"]*"at[0-9.]+pt->)')
FONT_FILE_RX = re.compile(r"[\w.+-]*?\.(otf|ttf|otc|ttc|dfont)", re.I)
FREED_MEMORY_MAX = 64


def without_script_choice(text):
    return SCRIPT_NOTE_RX.sub("", SCRIPT_TAG_RX.sub("", text))


def same_but_found_names(tl, tt):
    """Equal once TL's freed memory after each font request by name stands for TeXyoN's file name."""
    a, b = REQUEST_BY_NAME_RX.split(tl), REQUEST_BY_NAME_RX.split(tt)
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


def same_with_bugs(tl, tt, bugs):
    if 18 in bugs:
        tl, tt = without_script_choice(tl), without_script_choice(tt)
    return same_but_found_names(tl, tt) if 21 in bugs else tl == tt


def upstream_bugs_explain(tl_log, log):
    """The upstream bugs whose log effects are all that separates the logs, or None."""
    tl, tt = squeeze(tl_log), squeeze(log)
    every = {18, 21}
    if not same_with_bugs(tl, tt, every):
        return None
    return [n for n in sorted(every) if not same_with_bugs(tl, tt, every - {n})]


def log_diff(a, b, context=300):
    """Where two logs first differ once whitespace is ignored."""
    a, b = squeeze(a), squeeze(b)
    p = next((i for i in range(min(len(a), len(b))) if a[i] != b[i]), min(len(a), len(b)))
    lo = max(0, p - context)
    return ["--- upstream.log", "+++ texyon.log",
            f"@@ first difference at character {p} of {len(a)} vs {len(b)}, whitespace removed @@",
            "-" + a[lo:p + context], "+" + b[lo:p + context]]


def lvt_cases(_tl):
    cases = []
    suites = CORPUS / "suites"
    for repo in ("latex2e", "latex3"):
        base = suites / repo
        if not base.exists():
            continue
        support = list((base / "support").glob("*")) if (base / "support").exists() else []
        for tdir in sorted(base.glob("**/testfiles*")):
            if not tdir.is_dir() or tdir.name == "testfiles-disabled" or "support" in tdir.parts:
                continue
            local = list((tdir / "support").glob("*")) if (tdir / "support").exists() else []
            files = [(f, f.name) for f in support + local if f.is_file()]
            for lvt in sorted(tdir.glob("*.lvt")):
                # l3benchmark repeats until the timer advances, which deterministic mode freezes.
                if lvt.stem.startswith("m3benchmark"):
                    continue
                rel = lvt.relative_to(base).with_suffix("")
                cases.append(Case(
                    name=f"{repo}/{rel}", suite="lvt", files=files + [(lvt, lvt.stem + ".tex")],
                    main=lvt.stem + ".tex", passes=2, timeout=180))
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
    index = CORPUS / "arxiv" / "mains.json"
    mains = json.loads(index.read_text()) if index.exists() else {}
    changed = False
    for tb in sorted((CORPUS / "arxiv").glob("*.tar.gz")):
        ident = tb.name[: -len(".tar.gz")]
        if ident not in mains:
            tmp = CORPUS / "arxiv" / ".probe"
            shutil.rmtree(tmp, ignore_errors=True)
            tmp.mkdir()
            try:
                with tarfile.open(tb) as t:
                    t.extractall(tmp, filter="data")
                mains[ident] = find_main(tmp)
            except (tarfile.TarError, OSError):
                mains[ident] = None
            shutil.rmtree(tmp, ignore_errors=True)
            changed = True
        if mains[ident]:
            cases.append(Case(name=f"arxiv/{ident}", suite="arxiv", files=[], main=mains[ident],
                              passes=2, timeout=600, tarball=tb))
    if changed:
        index.write_text(json.dumps(mains, indent=0, sort_keys=True))
    return cases


def manifest_cases(_tl):
    """The authored and harvested documents of tests/corpus/manifest.tsv."""
    corpus = ROOT / "crates/engine_xetex/tests/corpus"
    cases = []
    for line in (corpus / "manifest.tsv").read_text().splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        tier, ident, rel, _src = line.split("\t")
        main = (corpus / rel) if tier == "tier1" else (CORPUS / "raw" / rel)
        if not main.exists():
            continue
        top = main.parent
        files = [(f, str(f.relative_to(top))) for f in top.rglob("*") if f.is_file()] \
            if tier != "tier1" else [(main, main.name)]
        cases.append(Case(name=f"docs/{ident}", suite="docs", files=files, main=main.name,
                          passes=2, timeout=900))
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
                              files=files, main=tex.name, passes=2, timeout=600))
    return cases


SUITES = {"lvt": lvt_cases, "arxiv": arxiv_cases, "docs": manifest_cases, "tldocs": tldocs_cases}


def available_kib():
    with open("/proc/meminfo") as f:
        return next(int(l.split()[1]) for l in f if l.startswith("MemAvailable:"))


def wait_for_memory(lock, min_kib=2_500_000, settle=0.5):
    """Start cases one at a time, each once the machine has memory to spare."""
    with open(lock, "w") as f:
        fcntl.flock(f, fcntl.LOCK_EX)
        while available_kib() < min_kib:
            time.sleep(1 + random.random())
        # Lets the previous case grow before the next one reads the free memory.
        time.sleep(settle)


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


def compare(tl, case, work, cache_dir, xdv_pages, keep_failures, pixels=False):
    """Compare both engines on one case. Returns (status, diff lines)."""
    safe = case.name.replace("/", "-")
    key = hashlib.sha256((case.key() + tl.tl_digest + CACHE_VERSION).encode()).hexdigest()
    cached = cache_dir / f"{safe}.json.gz"
    tl_res = None
    if cached.exists():
        try:
            data = json.loads(gzip.decompress(cached.read_bytes()))
            if data.get("key") == key:
                tl_res = data
        except (OSError, ValueError):
            pass
    tt_dir = work / "tt" / safe
    tl_dir = work / "tl" / safe
    stem = Path(case.main).with_suffix("")
    wait_for_memory(work / "start.lock")

    def upstream():
        shutil.rmtree(tl_dir, ignore_errors=True)
        tl_dir.mkdir(parents=True)
        case.populate(tl_dir)
        try:
            crash = tl.run_tl(case, tl_dir)
        except subprocess.TimeoutExpired:
            crash = "upstream xetex timed out"
        # Upstream writes its outputs to the working directory, TeXyoN next to the input.
        log_path = tl_dir / f"{stem.name}.log"
        res = dict(key=key, crash=crash,
                   log=log_path.read_text(encoding="utf-8", errors="replace")
                   if log_path.exists() else None,
                   xdv=xdv_pages(tl_dir / f"{stem.name}.xdv"))
        cached.write_bytes(gzip.compress(json.dumps(res).encode(), 3))
        return res

    def ours():
        shutil.rmtree(tt_dir, ignore_errors=True)
        tt_dir.mkdir(parents=True)
        case.populate(tt_dir)
        try:
            tl.run_tt(case, tt_dir)
        except subprocess.TimeoutExpired:
            return None
        log = latex_log(tt_dir / f"{stem}.log", case.main)
        xdv = xdv_lines(tt_dir / f"{stem}.xdv", xdv_pages)
        return (strip_bundle_dirs(log, tl.nested(), tt_dir),
                strip_special_dirs(xdv, tl.nested(), tt_dir))

    with ThreadPoolExecutor(2) as ex:
        tl_future = ex.submit(upstream) if tl_res is None else None
        tt_future = ex.submit(ours)
        if tl_future:
            tl_res = tl_future.result()
        tt_res = tt_future.result()
    try:
        rerun_tl = None if tl_future else upstream
        return judge(tl, case, tl_res, tt_res, tl_dir, tt_dir, work, stem, rerun_tl, pixels)
    finally:
        shutil.rmtree(tl_dir, ignore_errors=True)
        if tt_res is None or not keep_failures:
            shutil.rmtree(tt_dir, ignore_errors=True)


def judge(tl, case, tl_res, tt_res, tl_dir, tt_dir, work, stem, rerun_tl, pixels):
    """Status and diff lines, `rerun_tl` set when upstream's outputs come from the cache."""
    if tl_res["crash"]:
        return "tl-crash", [tl_res["crash"]]
    if tt_res is None:
        return "timeout", ["<texyon timed out>"]
    log, xdv = tt_res
    tl_log = latex_text(tl_res["log"], case.main)
    tl_xdv = strip_special_dirs(normalize_xdv(tl_res["xdv"]), tl.nested(), tt_dir)
    no_pages = not any(line.startswith("bop") for line in tl_xdv)
    diffs = []
    if not same_log(tl_log, log):
        diffs += log_diff(tl_log, log)
    # TeXyoN keeps no output after a fatal error, upstream keeps the pages shipped so far.
    fatal = xdv == ["<no xdv>"] and FATAL_RX.search(tl_res["log"] or "") is not None
    if tl_xdv != xdv and not fatal:
        diffs += window_diff(tl_xdv, xdv, "xdv")
    if not diffs:
        shutil.rmtree(tt_dir, ignore_errors=True)
        if fatal:
            return "match-fatal", []
        return ("match-nopages" if no_pages else "match"), []
    bugs = upstream_bugs_explain(tl_log, log) if tl_xdv == xdv or fatal else None
    if bugs is not None:
        shutil.rmtree(tt_dir, ignore_errors=True)
        return "match-upstream", [f"upstream bugs: {', '.join(map(str, bugs))}"] + diffs
    if pixels and tl_xdv != xdv and not no_pages and xdv != ["<no xdv>"]:
        if rerun_tl:
            rerun_tl()
        same, note = same_pixels(tl, case, tl_dir / f"{stem.name}.xdv", tt_dir / f"{stem}.xdv",
                                 work / "pixels" / case.name.replace("/", "-"))
        diffs.append(note)
        if same:
            return "diverge-samepixels", diffs
    return ("diverge-nopages" if no_pages else "diverge"), diffs


PIXEL_DPI = "100"


def rasterize(tl, xdv, out, timeout):
    """Pages of an XDV as grayscale PNGs, through upstream's xdvipdfmx, or None."""
    out.mkdir(parents=True, exist_ok=True)
    pdf = out / "out.pdf"
    try:
        subprocess.run([str(tl.bin / "xdvipdfmx"), "-q", "-o", str(pdf), xdv.name], cwd=xdv.parent,
                       env=tl.env(), stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                       stderr=subprocess.DEVNULL, timeout=timeout, check=False)
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


def same_pixels(tl, case, tl_xdv, tt_xdv, out):
    """Whether both XDVs render to the same pages, and a note for the diff."""
    shutil.rmtree(out, ignore_errors=True)
    a = rasterize(tl, tl_xdv, out / "upstream", case.timeout)
    b = rasterize(tl, tt_xdv, out / "texyon", case.timeout)
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
