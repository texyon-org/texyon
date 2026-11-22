"""Random plain TeX probes of the page builder and output routines, for run.py --probes."""

import gen

PRE = r"""\immediate\write-1{}\scrollmode\tracingonline1 \showboxdepth\maxdimen \showboxbreadth\maxdimen
\font\xa=xliga \hbadness10000 \vbadness10000 \hfuzz\maxdimen \vfuzz\maxdimen
\def\rep{\message{[g=\the\pagegoal\space t=\the\pagetotal\space s=\the\pageshrink\space
 d=\the\pagedepth\space ip=\the\insertpenalties\space op=\the\outputpenalty\space dc=\the\deadcycles]}}
"""

OUTPUTS = [
    r"\shipout\box255",
    r"\showbox255 \shipout\box255",
    r"\rep\showbox255 \message{[\topmark|\firstmark|\botmark]}\shipout\box255",
    r"\message{[\topmarks1|\firstmarks1|\botmarks1|\topmarks7|\firstmarks7|\botmarks7]}\shipout\box255",
    r"\setbox1\vsplit255 to 20pt \message{[\splitfirstmark|\splitbotmark|\splitfirstmarks1|\splitbotmarks1]}\showbox1 \showbox255 \shipout\box1 \shipout\box255",
    r"\ifnum\outputpenalty>-20000 \unvbox255 \penalty\outputpenalty\else\shipout\box255\fi",
    r"\showbox255 \showbox100 \showbox101 \shipout\vbox{\box255 \ifvoid100\else\box100\fi\ifvoid101\else\unvbox101\fi}",
    r"\global\holdinginserts=0 \unvbox255 \penalty\outputpenalty",
    r"\shipout\copy255 \shipout\box255 \message{\the\deadcycles}",
    r"\showlists \shipout\box255 \showbox300",
    r"\setbox0\vbox{\unvcopy255}\showbox0 \shipout\box255 \setbox0\vbox{\pagediscards}\showbox0",
    r"\shipout\vbox{\halign{#\hfil&\hfil#\cr a&b\cr\noalign{\kern2pt}\box255\cr}}",
    r"\shipout\box255 \global\vsize=\the\numexpr\vsize/2+20pt\relax",
]


class G(gen.Generator):
    def glue(self):
        s = self.dim(0, 12)
        if self.r.random() < 0.6:
            s += f" plus {self.dim(0, 6)}" if self.r.random() < 0.8 else f" plus {self.r.randint(1, 2)}fil"
        if self.r.random() < 0.5:
            s += f" minus {self.dim(0, 4)}" if self.r.random() < 0.9 else " minus 1fil"
        return s

    def word(self):
        return "".join(self.r.choice("abcdefghijklmnopqrstuvwxyz") for _ in range(self.r.randint(1, 8)))

    def text(self, n=6):
        return " ".join(self.word() for _ in range(self.r.randint(1, n)))

    def mark(self):
        c = self.ch("", "", "1", "7", "300")
        if c:
            return r"\marks%s{m%s-%d}" % (c, c, self.r.randint(0, 99))
        return r"\mark{m%d}" % self.r.randint(0, 99)

    def insert(self):
        cls = self.ch("100", "101", "102")
        body = self.ch(
            r"\hbox{i%s}" % self.word(),
            r"\hrule height %s depth %s" % (self.dim(0, 6), self.dim(0, 3)),
            r"\hsize 50pt \noindent %s\par" % self.text(10),
            r"\vskip %s\hbox{x}\penalty%d \hbox{y}" % (self.glue(), self.r.randint(-100, 100)),
            r"\hbox{a}\hbox{b}\hbox{c}\hbox{d}",
        )
        pre = ""
        if self.r.random() < 0.3:
            pre = r"\floatingpenalty=%d " % self.r.choice([0, 100, 20000])
        return r"%s\insert%s{%s}" % (pre, cls, body)

    def valign(self):
        return r"\valign{\hbox{#}\vfil&\hbox{\it #}\cr %s&%s\cr\noalign{\hskip 2pt}%s\cr}" % (
            self.word(), self.word(), self.word())

    def align(self, depth=0):
        cols = self.r.randint(1, 4)
        pre = "&".join(self.ch("#", r"\hfil#", r"#\hfil", r"\hfil#\hfil", r"\tabskip=%s#" % self.glue(),
                               r"$#$", r"\vrule#", r"(#)") for _ in range(cols))
        if self.r.random() < 0.2:
            pre = "&" + pre
        if self.r.random() < 0.2:
            pre = r"\tabskip=%s " % self.glue() + pre
        rows = []
        for _ in range(self.r.randint(1, 5)):
            cells = []
            for k in range(self.r.randint(1, cols + 1)):
                x = self.r.random()
                if x < 0.15:
                    c = r"\omit " + self.word()
                elif x < 0.2 and depth < 1:
                    c = r"\vbox{%s}" % self.align(depth + 1)
                elif x < 0.25:
                    c = r"\hfill " + self.word()
                else:
                    c = self.word()
                cells.append(c)
            row = cells[0]
            for c in cells[1:]:
                row += self.ch("&", "&", r"\span ") + c
            rows.append(row + self.ch(r"\cr ", r"\cr ", r"\crcr "))
            if self.r.random() < 0.25:
                rows.append(self.ch(r"\noalign{\kern%s}" % self.dim(), r"\noalign{\penalty%d}" % self.r.randint(-200, 200),
                                    r"\noalign{\hrule}", r"\noalign{%s}" % self.mark()))
        spec = self.ch("", "", " to %dpt" % self.r.randint(20, 200), " spread %s" % self.dim())
        ev = ""
        if self.r.random() < 0.2:
            ev = r"\everycr{\noalign{\kern1pt}}"
        return r"%s\halign%s{%s\cr %s}" % (ev, spec, pre, "".join(rows))

    def vitem(self):
        x = self.r.random()
        opts = [
            lambda: r"\hbox{%s}" % self.text(),
            lambda: r"\noindent %s\par" % self.text(12),
            lambda: r"%s%s\par" % (self.text(8), self.ch(r"\vadjust{\penalty-10000}", r"\vadjust{\kern3pt}", self.mark(), self.insert(), "")),
            lambda: r"\penalty%d " % self.ch(-10000, -10001, -500, -1, 0, 100, 10000, -20000),
            lambda: r"\vskip %s\relax " % self.glue(),
            lambda: r"\kern%s " % self.dim(),
            lambda: self.mark(),
            lambda: self.insert(),
            lambda: r"\hrule height %s\relax " % self.dim(0, 4),
            lambda: r"\vbox to %s{\hbox{v}\vss}" % self.dim(0, 40),
            lambda: r"\vtop{\hbox{a}\hbox{b}}",
            lambda: self.align(),
            lambda: self.valign(),
            lambda: r"\setbox2\vbox{%s}\setbox3\vsplit2 to %s \showbox3 \showbox2 \box3 \box2" % (
                "".join(self.ch(r"\hbox{s}", r"\penalty%d " % self.r.randint(-200, 200), r"\vskip %s " % self.glue(),
                                self.mark(), r"\kern2pt ", r"\marks1{s1}") for _ in range(self.r.randint(2, 10))),
                self.dim(0, 30)),
            lambda: r"\splitmaxdepth=%s \splittopskip=%s " % (self.dim(0, 5), self.glue()),
            lambda: r"\vsize=%s \maxdepth=%s " % (self.dim(20, 120), self.dim(0, 6)),
            lambda: r"\topskip=%s " % self.glue(),
            lambda: r"\rep ",
            lambda: r"\showlists ",
            lambda: r"\holdinginserts=%d " % self.r.randint(0, 1),
            lambda: r"\savingvdiscards=%d " % self.r.randint(0, 1),
            lambda: r"\setbox4\vbox{\splitdiscards}\showbox4 \setbox4\vbox{\pagediscards}\showbox4 ",
            lambda: r"\dimen100=%s \skip100=%s \count100=%d " % (self.dim(0, 80), self.glue(), self.r.choice([0, 500, 1000, 2000])),
            lambda: r"\dimen101=%s \skip101=%s \count101=%d " % (self.dim(0, 80), self.glue(), self.r.choice([0, 500, 1000])),
            lambda: r"\noindent$$%s\eqno(1)$$\par" % self.ch(r"\halign{#\cr a\cr b\cr}", r"x^2", r"\vcenter{\hbox{c}}"),
            lambda: r"\moveright %s\vbox{\hbox{m}}" % self.dim(),
            lambda: r"\prevdepth=%s " % self.ch("-1000pt", self.dim()),
            lambda: r"\baselineskip=%s \lineskiplimit=%s " % (self.glue(), self.dim(-2, 4)),
        ]
        return self.r.choice(opts)()

    def doc(self):
        out = [PRE]
        out.append(r"\vsize=%dpt \maxdepth=%s \topskip=%s \hsize=%dpt \parindent=0pt" % (
            self.r.randint(20, 150), self.dim(0, 5), self.glue(), self.r.randint(40, 200)) + "\n")
        out.append(r"\dimen100=%s \skip100=%s \count100=%d \dimen101=%s \count101=%d \skip101=%s" % (
            self.dim(0, 80), self.glue(), self.r.choice([0, 500, 1000]), self.dim(0, 60), self.r.choice([0, 1000, 500]), self.glue()) + "\n")
        out.append(r"\dimen102=\maxdimen \count102=1000 \skip102=0pt \maxdeadcycles=%d \holdinginserts=%d \savingvdiscards=%d" % (
            self.r.choice([3, 25]), self.r.choice([0, 0, 1]), self.r.randint(0, 1)) + "\n")
        out.append(r"\output={%s\relax}" % self.r.choice(OUTPUTS) + "\n")
        for _ in range(self.r.randint(8, 30)):
            out.append(self.vitem() + "\n")
        out.append(r"\showlists" + "\n")
        out.append(r"\output={\shipout\box255 \ifvoid100\else\shipout\box100\fi\ifvoid101\else\shipout\box101\fi\ifvoid102\else\shipout\box102\fi\relax}\holdinginserts=0 \maxdeadcycles=100" + "\n")
        out.append(r"\bye" + "\n")
        return "".join(out)


if __name__ == "__main__":
    gen.main("p", G, __doc__)
