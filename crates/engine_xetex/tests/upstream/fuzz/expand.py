"""Writes N random plain TeX probes for run.py --probes: <outdir> <N> [first seed]."""
import os
import random
import sys

OUT = sys.argv[1]
N = int(sys.argv[2])
SEED0 = int(sys.argv[3]) if len(sys.argv) > 3 else 0
os.makedirs(OUT, exist_ok=True)

PRE = r"""\immediate\write-1{}\scrollmode\tracingonline1 \showboxdepth\maxdimen \showboxbreadth\maxdimen
\font\xa=xliga \font\lb="[LinLibertine_R_G.ttf]" at 16pt
\def\mac#1#2{[#1|#2]}\long\def\lmac#1{<#1>}\protected\def\pmac{P}\def\delim#1.#2\end{(#1/#2)}
\countdef\ca=10 \dimendef\da=10 \skipdef\sa=10 \toksdef\ta=10
"""


class G:
    def __init__(self, seed):
        self.r = random.Random(seed)
        self.depth = 0

    def ch(self, *xs):
        return self.r.choice(xs)

    def num(self):
        return self.ch("0", "1", "-7", "255", "256", "32767", "32768", "'17", '"FF', "`a", "`\\^^@",
                       "2147483647", "-2147483647", r"\ca", r"\count300", r"\numexpr 3*4-1\relax",
                       r"\numexpr(7+2)/-2\relax", r"\numexpr 2147483647+1\relax", r"\numexpr 7/0\relax",
                       r"\the\catcode`a", r"\currentgrouplevel", r"\lastnodetype", r"\inputlineno",
                       r"\badness", r"\lastpenalty", r"\currentiftype", r"\currentifbranch", r"\romannumeral 0 5",
                       r"\number\romannumeral 0 12", r"\gluestretchorder\sa", r"\fontcharwd\tenrm`a", r"\da")

    def dimv(self):
        return self.ch("1pt", "-2.5pt", "1in", "3em", "2ex", "1truept", "16383.99999pt", "16384pt", r"\da",
                       r"\dimexpr 1pt*3/4\relax", r"\dimexpr (1pt+2pt)*-2\relax", r"\sa", r"\gluestretch\sa",
                       r"\fontdimen6\tenrm", r"\wd0", r"\ht0", r"\dp300", r"\hsize", r"\prevdepth", r"\pagegoal",
                       r"1.5\da", r"-\da", r"\glueshrink\thickmuskip", r"1fil", r"1 true in", r"\numexpr 3\relax pt")

    def gluev(self):
        return self.ch("1pt plus 2pt minus 3pt", "0pt plus 1fil", "3pt plus -1fill minus 2filll", r"\sa",
                       r"\glueexpr \sa*2+1pt plus 1fil\relax", r"\glueexpr\sa/0\relax", r"-\sa", r"\mutoglue\thinmuskip",
                       r"\thickmuskip", "1pt plus 1fillll", r"\skip300", "2pt plus 1pt")

    def toks(self, n=4):
        self.depth += 1
        if self.depth > 3:
            self.depth -= 1
            return "x"
        parts = []
        for _ in range(self.r.randint(1, n)):
            parts.append(self.r.choice([
                lambda: self.ch("a", "b", "xyz", " ", "12", "#", "##", "&", "^", "_", "~", "$", "%\n", "\\ "),
                lambda: r"\mac{%s}{%s}" % (self.toks(2), self.toks(2)),
                lambda: r"\lmac{%s}" % self.toks(2),
                lambda: r"\pmac ",
                lambda: r"\delim %s.%s\end " % (self.toks(2), self.toks(2)),
                lambda: r"\the\ca ",
                lambda: r"\the\ta ",
                lambda: r"\number%s " % self.num(),
                lambda: r"\romannumeral%s " % self.num(),
                lambda: r"\string%s" % self.ch(r"\relax", "a", r"\ ", "{", r"\mac"),
                lambda: r"\meaning%s" % self.ch(r"\relax", "a", r"\mac", r"\pmac", r"\undefined", r"\tenrm", r"\ca", r"\count", "~", r"\lmac"),
                lambda: r"\noexpand\mac ",
                lambda: r"\expandafter\noexpand\csname %s\endcsname " % self.ch("mac", "undefinedz", "relax", "a b"),
                lambda: r"\csname %s\endcsname" % self.ch("relax", "pmac", "ca", "tenrm"),
                lambda: r"\ifnum%s<%s T\else F\fi " % (self.num(), self.num()),
                lambda: r"\ifdim%s>%s T\fi " % (self.dimv(), self.dimv()),
                lambda: r"\ifx%s%s S\else D\fi " % (self.ch(r"\relax", "a", r"\mac", r"\undefined"), self.ch(r"\relax", "a", r"\mac", r"\undefined")),
                lambda: r"\ifcase%s zero\or one\or two\else other\fi " % self.num(),
                lambda: r"\ifdefined%s Y\fi " % self.ch(r"\mac", r"\zzz", r"\relax"),
                lambda: r"\ifcsname %s\endcsname Y\else N\fi " % self.ch("mac", "zzz", "relax"),
                lambda: r"\iftrue %s\else %s\fi " % (self.toks(2), self.toks(2)),
                lambda: r"\uppercase{%s}" % self.toks(2),
                lambda: r"\lowercase{%s}" % self.toks(2),
                lambda: r"\detokenize{%s}" % self.toks(2),
                lambda: r"\unexpanded{%s}" % self.toks(2),
                lambda: r"\expandafter\mac\expandafter{\the\ca}{%s}" % self.toks(1),
                lambda: r"\fontname\tenrm ",
                lambda: r"\jobname ",
                lambda: r"\eTeXrevision ",
                lambda: r"\the\numexpr%s\relax " % self.num(),
                lambda: r"\the\dimexpr%s\relax " % self.dimv(),
                lambda: r"\the\glueexpr%s\relax " % self.gluev(),
                lambda: r"\the\sa ",
                lambda: r"\the\da ",
                lambda: r"\expanded{%s}" % self.toks(2),
                lambda: r"\Uchar%s " % self.ch("65", "0", "-1", '"10FFFF', '"110000', "`\\ "),
                lambda: r"\Ucharcat 65 %s " % self.ch("11", "12", "1", "2", "3", "4", "6", "7", "8", "10", "13", "0", "14"),
            ])())
        self.depth -= 1
        return "".join(parts)

    def stmt(self):
        return self.r.choice([
            lambda: r"\message{%s}" % self.toks(),
            lambda: r"\edef\x{%s}\show\x" % self.toks(),
            lambda: r"\xdef\y{%s}" % self.toks(),
            lambda: r"\immediate\write-1{%s}" % self.toks(),
            lambda: r"\showtokens{%s}" % self.toks(),
            lambda: r"\showtokens\expandafter{\jobname%s}" % self.toks(2),
            lambda: r"\ca=%s\relax" % self.num(),
            lambda: r"\global\advance\ca by %s\relax" % self.num(),
            lambda: r"\multiply\ca by %s\relax" % self.num(),
            lambda: r"\divide\ca by %s\relax" % self.num(),
            lambda: r"\da=%s\relax" % self.dimv(),
            lambda: r"\advance\da by %s\relax" % self.dimv(),
            lambda: r"\sa=%s\relax" % self.gluev(),
            lambda: r"\ta={%s}" % self.toks(),
            lambda: r"\count%s=%s\relax" % (self.ch("300", "32767", "-1", "32768", "5000"), self.num()),
            lambda: r"\dimen%s=%s\relax" % (self.ch("300", "32767", "7"), self.dimv()),
            lambda: r"\skip%s=%s\relax" % (self.ch("300", "32767"), self.gluev()),
            lambda: r"\toks%s={%s}" % (self.ch("300", "32767"), self.toks(2)),
            lambda: r"\setbox%s\hbox{%s}" % (self.ch("0", "300", "32767"), self.toks(3)),
            lambda: r"\showbox%s" % self.ch("0", "300", "32767", "32768"),
            lambda: r"\showthe%s" % self.ch(r"\ca", r"\da", r"\sa", r"\ta", r"\count300", r"\skip300", r"\toks300", r"\everypar", r"\parshape", r"\interlinepenalties0", r"\lastlinefit", r"\catcode`a", r"\mathcode`a", r"\delcode`(", r"\sfcode`.", r"\uccode`a", r"\thinmuskip", r"\muskip300", r"\textfont1", r"\font", r"\hyphenchar\tenrm", r"\fontdimen1\tenrm", r"\XeTeXcharclass`a", r"\eTeXversion", r"\lastnodetype", r"\currentgrouptype"),
            lambda: r"\begingroup %s\showgroups\showifs\endgroup" % self.stmt(),
            lambda: r"{\aftergroup\message\aftergroup{\aftergroup A\aftergroup} %s}" % self.stmt(),
            lambda: r"\afterassignment\relax %s" % self.stmt(),
            lambda: r"\scantokens{%s}" % self.toks(2),
            lambda: r"\def\z#1#2{%s}\z{%s}{%s}" % (self.toks(2), self.toks(1), self.toks(1)),
            lambda: r"\let\z=%s \show\z" % self.ch(r"\mac", "a", r"\relax", r"\undefined", "{", r"\bgroup", r"\ca", r"\sa"),
            lambda: r"\futurelet\z\show\z %s" % self.toks(1),
            lambda: r"\chardef\z=%s \show\z" % self.num(),
            lambda: r"\mathchardef\z=%s \show\z" % self.num(),
            lambda: r"\catcode`\!=%s \message{a!b}" % self.ch("0", "1", "2", "3", "4", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16"),
            lambda: r"\uccode`a=%s \lccode`A=%s" % (self.num(), self.num()),
            lambda: r"\expandafter\expandafter\expandafter\meaning\expandafter\csname %s\endcsname" % self.ch("mac", "relax", "zz"),
            lambda: r"\ifnum %s>0 \message{pos}\else\ifnum %s<0 \message{neg}\fi\fi" % (self.num(), self.num()),
            lambda: r"\tracingcommands=%d \tracingmacros=%d \tracingifs=%d \tracingnesting=%d \tracingscantokens=%d" % tuple(self.r.randint(0, 3) for _ in range(5)),
            lambda: r"\errorcontextlines=%d" % self.r.choice([-1, 0, 1, 5, 100]),
            lambda: r"\errhelp{%s}\errmessage{%s}" % (self.toks(1), self.toks(1)),
            lambda: r"\escapechar=%s" % self.ch("-1", "`\\\\", "`!", "256", '"10FFFF'),
            lambda: r"\newlinechar=%s" % self.ch("-1", "`a", "10"),
            lambda: r"\endlinechar=%s" % self.ch("-1", "13", "`A"),
            lambda: self.toks(3) + r"\par",
            lambda: r"\hbox{%s}" % self.toks(3),
            lambda: r"\halign{#\cr %s\cr}" % self.toks(2),
            lambda: "$%s$" % self.toks(2),
        ])()

    def doc(self):
        out = [PRE]
        for _ in range(self.r.randint(5, 25)):
            out.append(self.stmt() + "\n")
        out.append(r"\fi\fi\fi\fi}}}\endgroup\endgroup" + "\n")
        out.append(r"\escapechar=`\\ \endlinechar=13 \newlinechar=-1 \catcode`\!=12 \bye" + "\n")
        return "".join(out)


for i in range(N):
    seed = SEED0 + i
    with open(os.path.join(OUT, f"e{seed:05d}.tex"), "w") as f:
        f.write(G(seed).doc())
