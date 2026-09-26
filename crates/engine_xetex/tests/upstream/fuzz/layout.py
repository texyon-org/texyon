"""Writes N random plain TeX probes for run.py --probes: <outdir> <N> [first seed]."""
import os
import random
import sys

OUT = sys.argv[1]
N = int(sys.argv[2])
SEED0 = int(sys.argv[3]) if len(sys.argv) > 3 else 0
os.makedirs(OUT, exist_ok=True)

PRE = r"""\immediate\write-1{}\scrollmode\tracingonline1 \showboxdepth\maxdimen \showboxbreadth\maxdimen
\font\xa=xliga \font\xb=xligb \font\xc=xligc \font\lb="[LinLibertine_R_G.ttf]" at 16pt
\hyphenchar\xa=`- \hyphenchar\xb=`- \hyphenchar\lb=`-
"""


class G:
    def __init__(self, seed):
        self.r = random.Random(seed)
        self.depth = 0

    def ch(self, *xs):
        return self.r.choice(xs)

    def dim(self, lo=-5, hi=20):
        return f"{self.r.randint(lo, hi)}.{self.r.randint(0, 99):02d}pt"

    def glue(self):
        s = self.dim(0, 10)
        if self.r.random() < 0.6:
            s += f" plus {self.dim(0, 5)}" if self.r.random() < 0.7 else f" plus {self.r.randint(1, 3)}fil"
        if self.r.random() < 0.5:
            s += f" minus {self.dim(0, 3)}" if self.r.random() < 0.8 else " minus 1fil"
        return s

    def word(self, font):
        if font in (r"\xa", r"\xb", r"\xc"):
            alpha = "abcdefghijklmnopqrstuvwxyzZ-."
        else:
            alpha = "abcdefghijklmnopqrstuvwxyzffifl"
        n = self.r.randint(1, 12)
        w = "".join(self.r.choice(alpha) for _ in range(n))
        return w.replace("--", "-")

    def font(self):
        return self.ch(r"\tenrm", r"\tenit", r"\tenbf", r"\xa", r"\xb", r"\xc", r"\lb", r"\tentt", r"\sevenrm")

    def hitem(self, font):
        x = self.r.random()
        if x < 0.45:
            return self.word(font)
        if x < 0.6:
            return " "
        opts = [
            lambda: r"\-",
            lambda: r"\discretionary{%s}{%s}{%s}" % (self.word(font)[:2], self.word(font)[:2], self.word(font)[:3]),
            lambda: r"\kern%s " % self.dim(),
            lambda: r"\hskip %s\relax " % self.glue(),
            lambda: r"\penalty%d " % self.r.choice([-10000, -500, -1, 0, 50, 10000]),
            lambda: r"\vrule width %s height %s depth %s\relax " % (self.dim(0, 3), self.dim(0, 8), self.dim(0, 3)),
            lambda: r"\hbox{%s}" % self.hlist(font, 4),
            lambda: "$%s$" % self.math(3),
            lambda: r"\/",
            lambda: r"\noboundary ",
            lambda: r"\accent%d %s" % (self.r.choice([18, 19, 94, 126, 127]), self.word(font)[:1] or "a"),
            lambda: r"\char%d " % self.r.randint(32, 127),
            lambda: r"\leaders\hbox{.}\hskip %s\relax " % self.glue(),
            lambda: r"\vadjust{\kern%s}" % self.dim(),
            lambda: r"\mark{m%d}" % self.r.randint(0, 99),
            lambda: r"\insert100{\hbox{ins}}",
            lambda: r"\beginR %s\endR{}" % self.hlist(font, 3),
            lambda: r"\beginL %s\endL{}" % self.hlist(font, 3),
            lambda: r"\unskip ",
            lambda: r"\unkern ",
            lambda: r"\unpenalty ",
            lambda: self.font() + " ",
            lambda: r"\spacefactor%d " % self.r.choice([1, 999, 1000, 1001, 3000]),
            lambda: r"\break ",
            lambda: r"\hfil ",
            lambda: r"\hss ",
            lambda: r"\ ",
        ]
        return self.r.choice(opts)()

    def hlist(self, font, n):
        self.depth += 1
        if self.depth > 3:
            self.depth -= 1
            return self.word(font)
        out = font + " " + "".join(self.hitem(font) for _ in range(self.r.randint(1, n)))
        self.depth -= 1
        return out

    def matom(self):
        return self.ch("a", "b", "x", "1", "2", "+", "-", "=", "<", ",", ";", "(", ")", "[", "]",
                       r"\alpha", r"\sum", r"\int", r"\prod", r"\infty", r"\cdot", r"\to", r"\le",
                       r"\mathop{\rm lim}", r"\sin", r"\log", r"\{", r"\|", r"\langle", "!", "?", ".")

    def math(self, n):
        self.depth += 1
        if self.depth > 3:
            self.depth -= 1
            return self.matom()
        parts = []
        for _ in range(self.r.randint(1, n)):
            x = self.r.random()
            if x < 0.4:
                parts.append(self.matom())
            elif x < 0.5:
                parts.append("^{%s}" % self.math(2))
            elif x < 0.6:
                parts.append("_{%s}" % self.math(2))
            else:
                parts.append(self.r.choice([
                    lambda: r"{%s\over %s}" % (self.math(2), self.math(2)),
                    lambda: r"{%s\atop %s}" % (self.math(2), self.math(2)),
                    lambda: r"{%s\above %s %s}" % (self.math(2), self.dim(0, 2), self.math(2)),
                    lambda: r"\sqrt{%s}" % self.math(2),
                    lambda: r"\left%s %s\right%s" % (self.ch("(", "[", r"\{", ".", "|", r"\langle"), self.math(3), self.ch(")", "]", r"\}", ".", "|")),
                    lambda: r"\left( %s\middle| %s\right)" % (self.math(2), self.math(2)),
                    lambda: r"\mathchoice{a}{b}{c}{d}",
                    lambda: r"\sum\limits_{%s}^{%s}" % (self.math(1), self.math(1)),
                    lambda: r"\int\nolimits_{%s}" % self.math(1),
                    lambda: r"\hat{%s}" % self.matom(),
                    lambda: r"\widetilde{%s}" % self.math(2),
                    lambda: r"\underline{%s}" % self.math(2),
                    lambda: r"\overline{%s}" % self.math(2),
                    lambda: r"\vcenter{\hbox{v}}",
                    lambda: r"\mathpunct{,}",
                    lambda: r"\mathinner{x}",
                    lambda: r"\mkern%dmu" % self.r.randint(-5, 18),
                    lambda: r"\mskip %dmu plus %dmu" % (self.r.randint(0, 9), self.r.randint(0, 9)),
                    lambda: r"\nonscript\mskip 5mu",
                    lambda: r"\displaystyle ",
                    lambda: r"\scriptstyle ",
                    lambda: r"\scriptscriptstyle ",
                    lambda: r"\penalty%d " % self.r.choice([-100, 0, 100]),
                    lambda: r"\hbox{%s}" % self.word(r"\tenrm"),
                    lambda: r"\big( ",
                    lambda: r"\Bigg\| ",
                    lambda: r"\discretionary{x}{y}{z}",
                    lambda: r"\mathaccent\"7016 %s" % self.matom(),
                    lambda: r"\raise%s\hbox{r}" % self.dim(),
                    lambda: r"{\rm %s}" % self.word(r"\tenrm"),
                    lambda: r"\it %s" % self.word(r"\tenrm"),
                ])())
        self.depth -= 1
        return " ".join(parts)

    def par_params(self):
        s = []
        s.append(r"\hsize=%dpt" % self.r.randint(40, 300))
        s.append(r"\parindent=%s" % self.dim(0, 20))
        s.append(r"\tolerance=%d" % self.r.choice([100, 200, 1000, 5000, 10000]))
        s.append(r"\pretolerance=%d" % self.r.choice([-1, 100, 1000]))
        s.append(r"\hbadness=%d" % self.r.choice([0, 1000, 10000]))
        s.append(r"\hfuzz=%s" % self.dim(0, 3))
        if self.r.random() < 0.3:
            s.append(r"\looseness=%d" % self.r.randint(-2, 2))
        if self.r.random() < 0.3:
            s.append(r"\emergencystretch=%s" % self.dim(0, 20))
        if self.r.random() < 0.3:
            s.append(r"\lastlinefit=%d" % self.r.choice([0, 500, 1000]))
        if self.r.random() < 0.2:
            s.append(r"\hangindent=%s \hangafter=%d" % (self.dim(-30, 30), self.r.randint(-3, 3)))
        if self.r.random() < 0.2:
            n = self.r.randint(1, 4)
            s.append(r"\parshape %d " % n + " ".join("%s %dpt" % (self.dim(0, 30), self.r.randint(30, 200)) for _ in range(n)))
        if self.r.random() < 0.3:
            s.append(r"\leftskip=%s \rightskip=%s" % (self.glue(), self.glue()))
        if self.r.random() < 0.2:
            s.append(r"\parfillskip=%s" % self.glue())
        if self.r.random() < 0.3:
            s.append(r"\hyphenpenalty=%d \exhyphenpenalty=%d" % (self.r.randint(-100, 1000), self.r.randint(-100, 1000)))
        if self.r.random() < 0.2:
            s.append(r"\adjdemerits=%d \doublehyphendemerits=%d \finalhyphendemerits=%d \linepenalty=%d" % tuple(self.r.randint(0, 20000) for _ in range(4)))
        if self.r.random() < 0.2:
            s.append(r"\interlinepenalties=3 %d %d %d \clubpenalties 2 %d %d \widowpenalties 2 %d %d" % tuple(self.r.randint(-100, 1000) for _ in range(7)))
        if self.r.random() < 0.2:
            s.append(r"\lefthyphenmin=%d \righthyphenmin=%d" % (self.r.randint(0, 4), self.r.randint(0, 4)))
        if self.r.random() < 0.2:
            s.append(r"\uchyph=%d" % self.r.randint(-1, 1))
        if self.r.random() < 0.2:
            s.append(r"\XeTeXinterwordspaceshaping=%d" % self.r.randint(0, 2))
        return " ".join(s) + "\n"

    def paragraph(self):
        f = self.font()
        body = f + " " + "".join(self.hitem(f) for _ in range(self.r.randint(5, 40)))
        if self.r.random() < 0.2:
            body += "$$%s%s$$" % (self.math(5), self.ch("", r"\eqno(1)", r"\leqno(2)"))
            body += "".join(self.hitem(f) for _ in range(self.r.randint(0, 10)))
        return body

    def align(self):
        cols = self.r.randint(1, 4)
        pre = "&".join(self.ch("#", r"\hfil#", r"#\hfil", r"\hfil#\hfil", r"\tabskip=%s#" % self.glue(), r"$#$") for _ in range(cols))
        if self.r.random() < 0.3:
            pre = "&" + pre
        rows = []
        for _ in range(self.r.randint(1, 5)):
            cells = []
            for _ in range(self.r.randint(1, cols + 1)):
                c = self.word(r"\tenrm")
                c = self.ch("", r"\omit ", r"\span ") + c if self.r.random() < 0.3 else c
                cells.append(c)
            row = self.ch("&", r"\span")[0:0] + "&".join(cells)
            rows.append(row + r"\cr")
            if self.r.random() < 0.2:
                rows.append(r"\noalign{\kern%s}" % self.dim())
        spec = self.ch("", " to %dpt" % self.r.randint(20, 200), " spread %s" % self.dim())
        return r"\halign%s{%s\cr %s}" % (spec, pre, "".join(rows))

    def doc(self):
        out = [PRE]
        out.append(r"\vsize=%dpt \maxdepth=%s \topskip=%s" % (self.r.randint(30, 300), self.dim(0, 5), self.glue()) + "\n")
        out.append(r"\TeXXeTstate=%d \holdinginserts=%d \count100=%d \dimen100=%s \skip100=%s" % (
            self.r.randint(0, 1), self.r.choice([0, 0, 1]), self.r.choice([0, 500, 1000]), self.dim(0, 100), self.glue()) + "\n")
        out.append(r"\output={\showbox255 \showbox100 \message{[\topmark|\firstmark|\botmark]}\shipout\box255 \ifvoid100 \else\shipout\box100 \fi}" + "\n")
        for _ in range(self.r.randint(2, 6)):
            x = self.r.random()
            if x < 0.5:
                out.append(self.par_params())
                if self.r.random() < 0.5:
                    out.append(r"\setbox1\vbox{%s\par}\showbox1 \box1" % self.paragraph() + "\n")
                else:
                    out.append(self.paragraph() + r"\showlists\par" + "\n")
            elif x < 0.65:
                out.append(self.align() + "\n")
            elif x < 0.75:
                out.append(r"\setbox2\vbox{%s\par}\setbox3\vsplit2 to %dpt \showbox2 \showbox3 \box3" % (self.paragraph(), self.r.randint(0, 60)) + "\n")
            elif x < 0.85:
                out.append(r"\vskip %s\penalty%d \hrule height %s\kern%s" % (self.glue(), self.r.choice([-10000, 0, 100]), self.dim(0, 2), self.dim()) + "\n")
            else:
                out.append(r"\setbox1\hbox to %dpt{%s}\showbox1 \box1" % (self.r.randint(0, 200), self.hlist(self.font(), 12)) + "\n")
        out.append(r"\showlists" + "\n" + r"\bye" + "\n")
        return "".join(out)


for i in range(N):
    seed = SEED0 + i
    with open(os.path.join(OUT, f"r{seed:05d}.tex"), "w") as f:
        f.write(G(seed).doc())
