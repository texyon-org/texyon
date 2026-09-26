"""Writes N random plain TeX probes for run.py --probes: <outdir> <N> [first seed].

Focus: word building, hyphenation and line breaking with native and TFM fonts.
Native text stays in LinLibertine at 8pt and 16pt, where float and double
layout agree."""
import os
import random
import sys

OUT = sys.argv[1]
N = int(sys.argv[2])
SEED0 = int(sys.argv[3]) if len(sys.argv) > 3 else 0
os.makedirs(OUT, exist_ok=True)

PRE = r"""\immediate\write-1{}\scrollmode\tracingonline1 \showboxdepth\maxdimen \showboxbreadth\maxdimen
\font\xa=xliga \font\xb=xligb \font\xc=xligc \font\lb="[LinLibertine_R_G.ttf]" at 16pt
\font\ls="[LinLibertine_R_G.ttf]" at 8pt \font\ll="[LinLibertine_R_G.ttf]:letterspace=25" at 16pt
\font\lnl="[LinLibertine_R_G.ttf]:-liga" at 16pt
\hbadness=10000 \vbadness=10000 \hfuzz=\maxdimen
"""

# \ll (letterspace) stays out of shaping runs: upstream crashes on
# letterspaced text with \XeTeXinterwordspaceshaping and \spaceskip.
NATIVE = (r"\lb", r"\ls", r"\lnl")
TFM = (r"\tenrm", r"\tenit", r"\xa", r"\xb", r"\xc", r"\tentt")
LATIN = "abcdefghijklmnopqrstuvwxyz"
REAL = ["representative", "international", "hyphenation", "office", "affluent", "difficult",
        "typographical", "considerations", "Extraordinary", "baffling", "shuffle", "waffle",
        "cooperate", "co-operate", "well-known", "multi-lingual", "reflexive", "TeX", "fjord",
        "flight", "ffi", "AVAWAY", "Tomorrow", "Waffles"]


class G:
    def __init__(self, seed):
        self.r = random.Random(seed)
        self.depth = 0

    def ch(self, *xs):
        return self.r.choice(xs)

    def dim(self, lo=-3, hi=10):
        return f"{self.r.randint(lo, hi)}.{self.r.randint(0, 99):02d}pt"

    def word(self, font):
        if font in (r"\xa", r"\xb", r"\xc"):
            alpha = "abcdefghijklmnopqrstuvwxyzZ-."
            return "".join(self.r.choice(alpha) for _ in range(self.r.randint(1, 10))).replace("--", "-")
        if self.r.random() < 0.6:
            return self.r.choice(REAL)
        return "".join(self.r.choice(LATIN) for _ in range(self.r.randint(1, 14)))

    def font(self):
        return self.r.choice(NATIVE + NATIVE + TFM)

    def hyphenchar(self):
        f = self.ch(r"\lb", r"\ls", r"\xa", r"\xb", r"\xc", r"\tenrm", r"\lnl")
        v = self.ch("-1", "0", "`-", "`-", "`Z", "`s", "`k", "`x", "255", "256", "`\\^^ad", '"2010', '"E000', "65581", "-2")
        return r"\hyphenchar%s=%s " % (f, v)

    def disc(self, font):
        shapes = ["", self.word(font)[:2], r"\kern1pt", r"\hbox{h}", self.word(font)[:1] + r"\kern2pt"]
        return r"\discretionary{%s}{%s}{%s}" % (self.r.choice(shapes), self.r.choice(shapes), self.r.choice(shapes))

    def inword(self, font):
        return self.r.choice([
            lambda: r"\-",
            lambda: self.disc(font),
            lambda: r"\kern%s " % self.dim(),
            lambda: r"\kern0pt ",
            lambda: r"\penalty%d " % self.r.choice([-10000, 0, 50, 10000]),
            lambda: r"\relax ",
            lambda: "{}",
            lambda: r"\noboundary ",
            lambda: r"\/",
            lambda: self.font() + " ",
            lambda: r"\accent%d %s" % (self.r.choice([18, 19, 94, 126, 127, 180]), self.ch("a", "e", "o", "A")),
            lambda: r"\char%d " % self.r.choice([45, 65, 97, 173, 233, 0x2010, 0xE000, 127]),
            lambda: "$%s$" % self.ch("x", "a+b", r"\alpha", "", r"\hbox{m}", "1", "f"),
            lambda: r"\beginR ", lambda: r"\endR ", lambda: r"\beginL ", lambda: r"\endL ",
            lambda: r"\beginR %s\endR{}" % self.word(font),
            lambda: r"\vadjust{\kern1pt}",
            lambda: r"\mark{m}",
            lambda: "-",
            lambda: r"\hbox{%s %s}" % (font, self.word(font)),
            lambda: r"\unskip ",
            lambda: r"\ ",
            lambda: r"\XeTeXglyph%d " % self.r.choice([43, 72, 79, 16]),
        ])()

    def hmaterial(self, font):
        out = [font, " "]
        for _ in range(self.r.randint(5, 35)):
            x = self.r.random()
            if x < 0.45:
                out.append(self.word(font))
            elif x < 0.7:
                out.append(" ")
            elif x < 0.72:
                f = self.font()
                out.append(f + " ")
                font = f
            elif x < 0.75:
                out.append(self.hyphenchar())
            else:
                out.append(self.inword(font))
        return "".join(out)

    def params(self):
        s = [r"\hsize=%dpt" % self.r.choice([10, 30, 50, 70, 100, 150, 250]),
             r"\parindent=%s" % self.dim(0, 15),
             r"\pretolerance=%d" % self.ch(-1, -1, 100, 10000),
             r"\tolerance=%d" % self.ch(100, 1000, 10000)]
        r = self.r.random
        if r() < 0.5:
            s.append(r"\lefthyphenmin=%d \righthyphenmin=%d" % (self.r.choice([0, 1, 2, 3, 63, 64, -1]), self.r.choice([0, 1, 2, 3, 63, 64, -1])))
        if r() < 0.3:
            s.append(r"\uchyph=%d" % self.ch(-1, 0, 1, 2))
        if r() < 0.3:
            s.append(r"\XeTeXhyphenatablelength=%d" % self.ch(0, 1, 3, 5, 63, 64, 100, -1))
        if r() < 0.3:
            s.append(r"\language=%d" % self.ch(0, 1, 255, 256, -1))
        if r() < 0.2:
            s.append(r"\setlanguage%d " % self.ch(0, 1, 255))
        if r() < 0.3:
            s.append(r"\looseness=%d" % self.r.randint(-2, 2))
        if r() < 0.3:
            s.append(r"\emergencystretch=%s" % self.dim(0, 20))
        if r() < 0.3:
            s.append(r"\lastlinefit=%d" % self.ch(0, 500, 1000))
        if r() < 0.2:
            s.append(r"\XeTeXinterwordspaceshaping=0 \spaceskip=%s plus 1pt" % self.dim(0, 5))
        if r() < 0.2:
            s.append(r"\XeTeXinterwordspaceshaping=0 \xspaceskip=%s" % self.dim(0, 6))
        # Upstream reads width(font_glue[f]) while \spaceskip keeps it null
        # and crashes, so shaping runs with the font's own spaces only.
        if r() < 0.3:
            s.append(r"\spaceskip=0pt \xspaceskip=0pt \XeTeXinterwordspaceshaping=%d" % self.r.randint(0, 2))
        elif r() < 0.3:
            s.append(r"\XeTeXinterwordspaceshaping=0")
        if r() < 0.2:
            s.append(r"\sfcode`\.=%d \sfcode`a=%d" % (self.ch(0, 999, 1000, 3000, 32767), self.ch(0, 999, 1000, 2000)))
        if r() < 0.3:
            s.append(r"\TeXXeTstate=%d" % self.ch(0, 1, 1))
        if r() < 0.2:
            s.append(r"\XeTeXprotrudechars=%d \rpcode\lb`.=%d \lpcode\lb`A=%d \rpcode\tenrm`,=%d" % (
                self.ch(0, 1, 2), self.ch(0, 500, 1000), self.ch(0, 300, -200), self.ch(0, 700)))
        if r() < 0.3:
            s.append(r"\hyphenpenalty=%d \exhyphenpenalty=%d" % (self.ch(-10000, -50, 0, 50, 10000), self.ch(-50, 0, 50, 10000)))
        if r() < 0.2:
            s.append(r"\XeTeXinterchartokenstate=1 \XeTeXcharclass`a=1 \XeTeXcharclass`e=2 \XeTeXinterchartoks 1 2 ={\kern1pt}\XeTeXinterchartoks 2 1 ={\penalty0 }\XeTeXinterchartoks 1 4095 ={\kern2pt}")
        if r() < 0.2:
            s.append(self.hyphenchar())
        return " ".join(s) + "\n"

    def doc(self):
        out = [PRE]
        if self.r.random() < 0.4:
            out.append(r"\hyphenation{%s}" % " ".join(self.ch("rep-re-sen-ta-tive", "in-ter-na-tion-al", "of-fice", "af-flu-ent", "a-b-c", "ab-cd", "baf-fling", "co-op-er-ate", "abc-def-ghi")) + "\n")
        for _ in range(self.r.randint(2, 6)):
            out.append(self.params())
            x = self.r.random()
            body = self.hmaterial(self.font())
            if x < 0.5:
                out.append(r"\setbox1\vbox{%s\par}\showbox1 \shipout\box1" % body + "\n")
            elif x < 0.75:
                out.append(body + r"\showlists\par" + "\n")
            elif x < 0.9:
                out.append(r"\setbox1\hbox{%s}\showbox1 \shipout\copy1" % body + "\n")
            else:
                out.append(r"\showhyphens{%s}" % body + "\n")
        out.append(r"\XeTeXinterchartokenstate=0 \TeXXeTstate=0 " + "\n")
        out.append(r"\fi\fi}}\endgroup" + "\n")
        out.append(r"\bye" + "\n")
        return "".join(out)


for i in range(N):
    seed = SEED0 + i
    with open(os.path.join(OUT, f"w{seed:05d}.tex"), "w") as f:
        f.write(G(seed).doc())
