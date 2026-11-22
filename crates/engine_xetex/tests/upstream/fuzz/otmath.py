"""Random plain TeX probes of OpenType math, for run.py --probes."""

import gen

FONTS = ["latinmodern-math.otf", "texgyrepagella-math.otf"]
LETTERS = {"a": 0x1D44E, "b": 0x1D44F, "x": 0x1D465, "y": 0x1D466, "f": 0x1D453, "g": 0x1D454,
           "A": 0x1D434, "P": 0x1D443, "V": 0x1D449, "W": 0x1D44A}
DELIMS = {"(": 0x28, ")": 0x29, "[": 0x5B, "]": 0x5D, "|": 0x7C, "/": 0x2F, "<": 0x27E8, ">": 0x27E9,
          "{": 0x7B, "}": 0x7D}
OPS = {r"\sum": 0x2211, r"\int": 0x222B, r"\prod": 0x220F, r"\bigcup": 0x22C3, r"\oint": 0x222E,
       r"\iint": 0x222C}
ACCENTS = [0x302, 0x303, 0x304, 0x306, 0x307, 0x30C, 0x20D7, 0x23DE, 0x23DF, 0x23DC, 0x23B4, 0x2190, 0x2192]
BOTTOM = [0x332, 0x330, 0x23DF, 0x23DD, 0x20EE]


def preamble(r):
    f = r.choice(FONTS)
    g = r.choice(FONTS)
    s1, s2, s3 = r.choice([(16, 8, 4), (16, 8, 4), (8, 4, 4), (16, 16, 8)])
    ssty = r.random() < 0.5
    out = [r"\immediate\write-1{}\scrollmode\tracingonline1 \showboxdepth\maxdimen \showboxbreadth\maxdimen"]
    out.append(r'\font\mA="[%s]" at %dpt \font\mB="[%s]%s" at %dpt \font\mC="[%s]%s" at %dpt'
               % (f, s1, f, ":+ssty=0" if ssty else "", s2, f, ":+ssty=1" if ssty else "", s3))
    out.append(r'\font\nA="[%s]" at %dpt' % (g, s1))
    out.append(r"\textfont2=\mA \scriptfont2=\mB \scriptscriptfont2=\mC")
    if r.random() < 0.8:
        out.append(r"\textfont3=\mA \scriptfont3=\mB \scriptscriptfont3=\mC")
    out.append(r"\textfont1=\mA \scriptfont1=\mB \scriptscriptfont1=\mC")
    if r.random() < 0.5:
        out.append(r"\textfont0=\mA \scriptfont0=\mB \scriptscriptfont0=\mC")
    out.append(r"\textfont4=\nA \scriptfont4=\nA \scriptscriptfont4=\nA")
    fam_letters = r.choice(["1", "1", "2", "4"])
    for k, v in LETTERS.items():
        out.append(r'\Umathcode`%s="7 "%s "%X' % (k, fam_letters, v))
    out.append(r'\Umathcode`+="2 "2 "2B \Umathcode`=="3 "2 "3D \Umathcode`,="6 "2 "2C \Umathcode`-="2 "2 "2212')
    for k, v in DELIMS.items():
        cls = 4 if k in "([<{" else 5
        if k in "|/":
            cls = 0
        out.append(r'\Umathcode`\%s="%d "2 "%X \Udelcode`\%s="%s "%X' % (k, cls, v, k, r.choice(["2", "3"]), v))
    for name, v in OPS.items():
        out.append(r'\Umathchardef%s="1 "%s "%X' % (name, r.choice(["2", "3"]), v))
    out.append(r'\def\sqrt{\Uradical "%s "221A }' % r.choice(["2", "3"]))
    out.append(r"\def\S#1{\setbox0\hbox{#1}\showbox0 \shipout\copy0 }")
    out.append(r"\mathsurround=%dpt \delimiterfactor=%d \delimitershortfall=%dpt \scriptspace=%dpt"
               % (r.choice([0, 0, 1, 3]), r.choice([901, 1000, 500]), r.choice([5, 0, 2]), r.choice([0, 1])))
    return "\n".join(out) + "\n"


class G(gen.Generator):
    def atom(self):
        r = self.r
        return r.choice(list(LETTERS) + ["+", "=", ",", "-", "1", "2", r"\sum", r"\int", r"\prod",
                                          r"\oint", "(", ")", "[", "]", "|"])

    def tall(self):
        return r"\vrule height %dpt depth %dpt width 1pt" % (self.r.choice([5, 12, 30, 80, 200, 600]),
                                                                self.r.choice([0, 5, 20, 100]))

    def delim(self):
        return self.r.choice([d for d in DELIMS if d not in "{}"] + [".", r"\{", r"\}", r'\Udelimiter 4 2 "2016 ', r'\Udelimiter 4 3 "230A '])

    def m(self, n=4):
        r = self.r
        self.depth += 1
        if self.depth > 4:
            self.depth -= 1
            return self.atom()
        parts = []
        for _ in range(r.randint(1, n)):
            x = r.random()
            if x < 0.3:
                parts.append(self.atom())
            elif x < 0.45:
                parts.append("%s%s{%s}" % (self.atom(), r.choice(["^", "_"]), self.m(2)))
            elif x < 0.5:
                parts.append("%s^{%s}_{%s}" % (r.choice(list(OPS)) + r.choice(["", r"\limits", r"\nolimits", r"\displaylimits"]),
                                                self.m(2), self.m(2)))
            else:
                parts.append(r.choice([
                    lambda: r"{%s\over %s}" % (self.m(2), self.m(2)),
                    lambda: r"{%s\atop %s}" % (self.m(2), self.m(2)),
                    lambda: r"{%s\above %dpt %s}" % (self.m(2), r.choice([0, 1, 3]), self.m(2)),
                    lambda: r"{%s\overwithdelims%s%s %s}" % (self.m(2), self.delim(), self.delim(), self.m(2)),
                    lambda: r"{%s\abovewithdelims%s%s 2pt %s}" % (self.m(2), self.delim(), self.delim(), self.m(2)),
                    lambda: r"\sqrt{%s}" % self.m(2),
                    lambda: r"\sqrt{%s}" % self.tall(),
                    lambda: r"\left%s %s\right%s" % (self.delim(), self.m(3), self.delim()),
                    lambda: r"\left%s %s\middle%s %s\right%s" % (self.delim(), self.tall(), self.delim(), self.m(2), self.delim()),
                    lambda: r"\left%s %s\right%s" % (self.delim(), self.tall(), self.delim()),
                    lambda: r'\Umathaccent%s "0 "2 "%X {%s}' % (r.choice(["", " fixed"]), r.choice(ACCENTS), self.m(r.choice([1, 1, 3]))),
                    lambda: r'\Umathaccent bottom%s "0 "2 "%X {%s}' % (r.choice(["", " fixed"]), r.choice(BOTTOM), self.m(2)),
                    lambda: r'\Umathaccent "0 "2 "%X {\hbox to %dpt{}}' % (r.choice(ACCENTS), r.choice([5, 20, 60, 200])),
                    lambda: r"\mathchoice{%s}{%s}{%s}{%s}" % (self.atom(), self.atom(), self.atom(), self.atom()),
                    lambda: r.choice([r"\displaystyle ", r"\textstyle ", r"\scriptstyle ", r"\scriptscriptstyle "]),
                    lambda: r"\overline{%s}" % self.m(2),
                    lambda: r"\underline{%s}" % self.m(2),
                    lambda: r"\vcenter{\hbox{v}}",
                    lambda: r'\mathop{%s}%s^{%s}_{%s}' % (self.atom(), r.choice([r"\limits", r"\nolimits", ""]), self.atom(), self.atom()),
                    lambda: r'\Umathchar "1 "2 "%X ' % r.choice(list(OPS.values())),
                    lambda: r'\mathchar"%X ' % r.choice([0x0141, 0x7161, 0x3230, 0x1350]),
                    lambda: r"\fam%d %s" % (r.choice([0, 1, 2, 4]), self.atom()),
                    lambda: r"\mkern%dmu\mskip 3mu plus 2mu" % r.randint(-5, 10),
                    lambda: r"{%s}^{%s}" % (self.tall(), self.atom()),
                    lambda: r'\radical"%X {%s}' % (r.choice([0x270370, 0x161370]), self.m(1)),
                    lambda: r'\delimiter"%X ' % r.choice([0x4162300, 0x5163301]),
                ])())
        self.depth -= 1
        return " ".join(parts)

    def doc(self):
        r = self.r
        out = [preamble(r)]
        for _ in range(r.randint(3, 8)):
            k = r.random()
            if k < 0.5:
                out.append(r"\S{$%s$}" % self.m(5))
            elif k < 0.8:
                out.append(r"\S{$\displaystyle %s$}" % self.m(5))
            else:
                out.append(r"\setbox1\vbox{\hsize 300pt \noindent x $$%s%s$$}\showbox1 \shipout\box1 "
                           % (self.m(4), r.choice(["", r"\eqno(1)", r"\leqno(2)"])))
        out.append(r"\showlists")
        out.append("$$}}}$$\\fi\\fi\\fi\\endgroup\\endgroup")
        out.append(r"\bye")
        return "\n".join(out) + "\n"


if __name__ == "__main__":
    gen.main("o", G, __doc__)
