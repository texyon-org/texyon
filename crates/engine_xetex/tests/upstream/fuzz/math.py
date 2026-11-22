"""Random plain TeX probes of math lists with TFM fonts, for run.py --probes."""

import gen

PRE = r"""\immediate\write-1{}\scrollmode\tracingonline1 \showboxdepth\maxdimen \showboxbreadth\maxdimen
\font\xa=xliga \font\lb="[LinLibertine_R_G.ttf]" at 16pt
\output={\showbox255 \shipout\box255}
"""

DELIMS_L = ["(", "[", r"\{", r"\lbrace", r"\langle", r"\lfloor", r"\lceil", "|", r"\|", "/", r"\backslash",
            ".", r"\uparrow", r"\Downarrow", r"\updownarrow", r"\lgroup", r"\lmoustache", r"\arrowvert"]
DELIMS_R = [")", "]", r"\}", r"\rbrace", r"\rangle", r"\rfloor", r"\rceil", "|", r"\|", "/", ".",
            r"\downarrow", r"\Uparrow", r"\rgroup", r"\rmoustache", r"\Arrowvert", r"\bracevert"]


class G(gen.Generator):
    def atom(self):
        return self.ch("a", "b", "f", "x", "y", "V", "W", "P", "1", "2", "+", "-", "=", "<", ">", ",", ";", ":",
                       "(", ")", "[", "]", "!", "?", ".", "/", "|", "'", r"\alpha", r"\beta", r"\Gamma",
                       r"\sum", r"\int", r"\oint", r"\prod", r"\bigcup", r"\infty", r"\cdot", r"\to",
                       r"\le", r"\ne", r"\pm", r"\times", r"\ldots", r"\cdots", r"\partial", r"\nabla",
                       r"\sin", r"\lim", r"\max", r"\log", r"\{", r"\|", r"\langle", r"\rangle",
                       r'\mathchar"%X' % self.r.choice([0x141, 0x7141, 0x8000, 0x3D, 0x1350, 0x7FFF, 0x2203]),
                       r'\delimiter"%X' % self.r.choice([0x4266308, 0x5267309, 0x26A30C, 0x0, 0x7FFFFFF]),
                       r'\fam%d x' % self.r.randint(0, 3), r'\fam1 \mathchar"7141',
                       r"\xa ab", r"\rm A", r"\it f", r"\bf x", "~")

    def sub(self, n=2):
        self.depth += 1
        if self.depth > 4:
            self.depth -= 1
            return self.atom()
        parts = [self.item() for _ in range(self.r.randint(1, n))]
        self.depth -= 1
        return " ".join(parts)

    def item(self):
        x = self.r.random()
        if x < 0.3:
            return self.atom()
        if x < 0.45:
            return self.atom() + self.ch("^{%s}" % self.sub(), "_{%s}" % self.sub(),
                                         "^{%s}_{%s}" % (self.sub(1), self.sub(1)), "'", "''^2", "_x^y")
        return self.r.choice([
            lambda: r"{%s\over %s}" % (self.sub(), self.sub()),
            lambda: r"{%s\atop %s}" % (self.sub(), self.sub()),
            lambda: r"{%s\above %s %s}" % (self.sub(), self.dim(-1, 3), self.sub()),
            lambda: r"{%s\overwithdelims %s%s %s}" % (self.sub(), self.ch(*DELIMS_L), self.ch(*DELIMS_R), self.sub()),
            lambda: r"{%s\atopwithdelims %s%s %s}" % (self.sub(), self.ch(*DELIMS_L), self.ch(*DELIMS_R), self.sub()),
            lambda: r"{%s\abovewithdelims %s%s %s %s}" % (self.sub(), self.ch(*DELIMS_L), self.ch(*DELIMS_R), self.dim(0, 2), self.sub()),
            lambda: r"\left%s %s\right%s" % (self.ch(*DELIMS_L), self.sub(3), self.ch(*DELIMS_R)),
            lambda: r"\left%s %s\middle%s %s\right%s" % (self.ch(*DELIMS_L), self.sub(), self.ch(*DELIMS_L), self.sub(), self.ch(*DELIMS_R)),
            lambda: r"\sqrt{%s}" % self.sub(3),
            lambda: r"\root %s\of{%s}" % (self.sub(1), self.sub()),
            lambda: r'\radical"%X {%s}' % (self.r.choice([0x270370, 0x0, 0x161, 0x7FFFFFF]), self.sub()),
            lambda: r"\mathchoice{%s}{%s}{%s}{%s}" % (self.sub(1), self.sub(1), self.sub(1), self.sub(1)),
            lambda: r"%s\limits_{%s}^{%s}" % (self.ch(r"\sum", r"\int", r"\mathop{\rm lim}", r"\mathop{f}", r"\prod"), self.sub(1), self.sub(1)),
            lambda: r"%s\nolimits_{%s}^{%s}" % (self.ch(r"\sum", r"\int", r"\mathop{f}", r"\mathop{\it f}"), self.sub(1), self.sub(1)),
            lambda: r"%s\displaylimits_{%s}" % (self.ch(r"\sum", r"\int"), self.sub(1)),
            lambda: r"\mathop{%s}%s" % (self.sub(1), self.ch("", r"\limits^x", "_y")),
            lambda: r'\mathaccent"%X %s' % (self.r.choice([0x7016, 0x7012, 0x705E, 0x717E, 0x7365, 0x17F]), self.ch("a", "f", "x", "W", "{ab}", r'{\bf x}', "{%s}" % self.sub(1))),
            lambda: r"\widehat{%s}" % self.sub(),
            lambda: r"\vec %s" % self.ch("a", "x", "f"),
            lambda: r"\underline{%s}" % self.sub(),
            lambda: r"\overline{%s}" % self.sub(),
            lambda: r"\vcenter{\hbox{%s}}" % self.ch("v", "$x$", r"\vrule height 8pt depth 3pt"),
            lambda: r"\vcenter to %s{\vss\hbox{c}}" % self.dim(0, 20),
            lambda: r"\mkern%dmu" % self.r.randint(-9, 18),
            lambda: r"\mskip %dmu plus %dmu minus %dmu" % (self.r.randint(-5, 9), self.r.randint(0, 9), self.r.randint(0, 5)),
            lambda: r"\nonscript\mskip %dmu" % self.r.randint(0, 9),
            lambda: r"\nonscript\kern 3pt",
            lambda: self.ch(r"\displaystyle", r"\textstyle", r"\scriptstyle", r"\scriptscriptstyle") + " ",
            lambda: r"\mathbin{%s}" % self.sub(1),
            lambda: r"\mathrel{%s}" % self.sub(1),
            lambda: r"\mathpunct{%s}" % self.sub(1),
            lambda: r"\mathinner{%s}" % self.sub(1),
            lambda: r"\mathopen{%s}\mathclose{%s}" % (self.sub(1), self.sub(1)),
            lambda: r"\mathord{%s}" % self.sub(1),
            lambda: r"\penalty%d " % self.ch(-10000, -100, 0, 100, 10000),
            lambda: r"\discretionary{%s}{}{}" % self.ch("", "-", "x"),
            lambda: r"\hbox{%s}" % self.ch("h", "$y^2$", r"\tenrm ab"),
            lambda: r"\raise%s\hbox{r}" % self.dim(),
            lambda: r"\big%s \Big%s \bigg%s \Bigg%s" % (self.ch(*DELIMS_L), self.ch(*DELIMS_R), self.ch(*DELIMS_L), self.ch(*DELIMS_R)),
            lambda: r"\phantom{%s}" % self.sub(1),
            lambda: r"\smash{%s}" % self.sub(1),
            lambda: r"\matrix{a&b\cr c&%s\cr}" % self.sub(1),
            lambda: r"\pmatrix{1&%s\cr}" % self.sub(1),
            lambda: r"\buildrel %s\over %s" % (self.atom(), self.ch("=", r"\to")),
            lambda: r"\kern%s" % self.dim(),
            lambda: r"\hskip %s plus 1fil" % self.dim(0, 5),
            lambda: r"\/",
            lambda: r"{\fam%d %s}" % (self.r.randint(0, 3), self.sub(1)),
            lambda: r"\mathsurround=%s" % self.dim(0, 5),
            lambda: r"\hbox{\beginR $%s$ r\endR}" % self.sub(1),
            lambda: r"\hat{\hat{%s}}" % self.atom(),
            lambda: r"\cases{a&b\cr c&d\cr}",
        ])()

    def formula(self, n):
        return " ".join(self.item() for _ in range(self.r.randint(1, n)))

    def params(self):
        s = [r"\hsize=%dpt \parindent=%s \tolerance=%d \hbadness=%d" % (
            self.r.randint(60, 300), self.dim(0, 20), self.ch(200, 1000, 10000), self.ch(0, 10000))]
        opts = [
            lambda: r"\mathsurround=%s" % self.dim(0, 5),
            lambda: r"\relpenalty=%d \binoppenalty=%d" % (self.r.randint(-100, 10000), self.r.randint(-100, 10000)),
            lambda: r"\scriptspace=%s" % self.dim(-1, 3),
            lambda: r"\delimiterfactor=%d \delimitershortfall=%s" % (self.r.randint(0, 1200), self.dim(-5, 10)),
            lambda: r"\nulldelimiterspace=%s" % self.dim(-2, 5),
            lambda: r"\thinmuskip=%dmu \medmuskip=%dmu plus 2mu \thickmuskip=%dmu minus 1mu" % tuple(self.r.randint(0, 9) for _ in range(3)),
            lambda: r"\parshape 2 %s %dpt %s %dpt" % (self.dim(0, 30), self.r.randint(50, 250), self.dim(0, 30), self.r.randint(50, 250)),
            lambda: r"\hangindent=%s \hangafter=%d" % (self.dim(-30, 30), self.r.randint(-2, 2)),
            lambda: r"\abovedisplayskip=%s \belowdisplayskip=%s \abovedisplayshortskip=%s \belowdisplayshortskip=%s" % (self.dim(0, 12), self.dim(0, 12), self.dim(0, 5), self.dim(0, 5)),
            lambda: r"\predisplaypenalty=%d \postdisplaypenalty=%d \displaywidowpenalty=%d" % (self.r.randint(-100, 10000), self.r.randint(-100, 1000), self.r.randint(-100, 1000)),
            lambda: r"\predisplaydirection=%d" % self.r.randint(-1, 1),
            lambda: r"\TeXXeTstate=%d" % self.r.randint(0, 1),
            lambda: r"\leftskip=%s \rightskip=%s" % (self.dim(0, 20), self.dim(0, 20)),
            lambda: r'\mathcode`\x="8000 {\catcode`\x=13 \gdef x{\mathchar"7178 ^*}}',
            lambda: r'\mathcode`\;="%X' % self.r.choice([0x603B, 0x8000, 0x7FFF, 0x0]),
            lambda: r"\skewchar\teni=%d" % self.ch(-1, 127, 48, 0),
            lambda: r"\textfont1=%s" % self.ch(r"\tenrm", r"\teni", r"\xa", r"\lb"),
            lambda: r"\scriptfont1=%s" % self.ch(r"\fivei", r"\teni", r"\xa"),
            lambda: r"\textfont3=\tenex \scriptfont3=\tenex",
            lambda: r"\fam=%d" % self.r.randint(-1, 3),
            lambda: r"\displayindent=%s" % self.dim(0, 20),
        ]
        for _ in range(self.r.randint(0, 5)):
            s.append(self.r.choice(opts)())
        return " ".join(s) + "\n"

    def para(self):
        body = self.ch("", "Text ", r"\noindent ", "A B C D E F ")
        for _ in range(self.r.randint(1, 4)):
            x = self.r.random()
            if x < 0.5:
                body += "$%s$ %s" % (self.formula(8), self.ch("", "word ", "and ", "x "))
                if self.r.random() < 0.15:
                    body = r"\beginR " + body + r"\endR "
            elif x < 0.8:
                body += "$$%s%s$$ " % (self.formula(6), self.ch("", r"\eqno(%d)" % self.r.randint(1, 9),
                                                                   r"\leqno(*)", r"\eqno{%s}" % self.formula(2)))
            elif x < 0.9:
                body += r"$$\halign{#\hfil&$#$\cr a&%s\cr\noalign{\kern2pt}b&%s\cr}$$ " % (self.formula(2), self.formula(2))
            else:
                body += r"$$\displaylines{%s\cr %s\cr}$$ " % (self.formula(3), self.formula(3))
        return body

    def doc(self):
        out = [PRE, r"\TeXXeTstate=%d" % self.r.randint(0, 1) + "\n"]
        for _ in range(self.r.randint(1, 5)):
            out.append(self.params())
            x = self.r.random()
            if x < 0.4:
                out.append(r"\setbox1\vbox{%s\par}\showbox1 \box1" % self.para() + "\n")
            elif x < 0.7:
                out.append(self.para() + r"\showlists\par" + "\n")
            else:
                out.append(r"\setbox1\hbox{$%s%s$}\showbox1 \box1" % (self.ch("", r"\displaystyle ", r"\scriptstyle "), self.formula(10)) + "\n")
        out.append(r"$$}}}$$\fi\fi\fi\endgroup\endgroup" + "\n")
        out.append(r"\TeXXeTstate=0 \bye" + "\n")
        return "".join(out)


if __name__ == "__main__":
    gen.main("m", G, __doc__)
