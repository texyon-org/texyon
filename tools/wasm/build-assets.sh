#!/usr/bin/env bash
# Build the support-file pack the browser build needs.
#
# A cold LaTeX run reads a ~24 MB format dump and a few hundred class, package,
# font and metric files. In a browser every one of those would otherwise be a
# network round trip taken *during* typesetting, with TeX blocked on it. So we
# compile a set of seed documents natively, keep everything they touched, and
# ship it as one pack that the Worker loads once at startup.
#
# Usage: tools/wasm/build-assets.sh [output-dir] [project-dir...]
#        output-dir defaults to target/wasm-assets
#        each project-dir is an extra seed, compiled from its main.tex
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
OUT="${1:-$REPO/target/wasm-assets}"
shift || true
PROJECTS=()
for dir in "$@"; do
    PROJECTS+=("$(cd "$dir" && pwd)")
done

TEXYON="$REPO/target/release/texyon"
if [ ! -x "$TEXYON" ]; then
    TEXYON="$REPO/target/debug/texyon"
fi
if [ ! -x "$TEXYON" ]; then
    echo "no tectonic binary; run: cargo build --release" >&2
    exit 1
fi

# The same bundle the browser reads at runtime. Seeding from the default
# bundle instead bakes an older LaTeX kernel into the format, and any package
# fetched on demand then fails with mismatched L3 support files.
BUNDLE_URL="${BUNDLE_URL:-https://s3.de-1.yendric.be/uploads/01a0117b-2294-7203-bb52-edb7070f93fb/texlive2026.ttb}"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
export TEXYON_CACHE_DIR="$WORK/cache"

# Seed documents. Between them these pull in the format dump plus the classes,
# packages, fonts and metrics that a general-purpose editor can reach
# without the user hitting the network. Add to this list to widen the pack.
mkdir -p "$WORK/seeds"

cat > "$WORK/seeds/core.tex" <<'TEX'
\documentclass[11pt]{article}
\usepackage{amsmath,amssymb,amsthm}
\usepackage[T1]{fontenc}
\usepackage{lmodern}
\title{Seed}\author{Seed}
\begin{document}
\maketitle
\tableofcontents
\section{Text}\label{sec:text}
Roman \textit{italic} \textbf{bold} \texttt{mono} \textsc{small caps}.
Reference to \S\ref{sec:text} and a footnote.\footnote{Like this.}
\begin{abstract}An abstract.\end{abstract}
\section{Mathematics}
\begin{equation}\int_{-\infty}^{\infty}e^{-x^2}\,dx=\sqrt{\pi}\end{equation}
\begin{align} a &= b + c \\ &= d \end{align}
\[ \sum_{k=1}^{n} k = \frac{n(n+1)}{2}, \quad \mathbb{R}, \mathcal{L}, \mathfrak{g} \]
% Nested and scripted math pulls the extension and symbol fonts in at script
% and scriptscript sizes, which is where cmex7/cmex5 and friends come from.
\[ x^{\int_{a}^{b} f} \quad
   a^{\sum_{i}^{j} b^{\prod_{k}^{l} c}} \quad
   \sqrt{\sqrt{\sqrt{x}}} \quad
   \frac{\frac{\frac{a}{b}}{c}}{d} \quad
   \left[\left(\left\{x\right\}\right)\right]^{\left(\frac{1}{2}\right)} \]
\[ \oint \bigcup \bigcap \bigoplus \iiint \left\| x \right\|
   \underbrace{a+b}_{c} \overbrace{d}^{e} \]
\[ \mathbf{A}^{\mathsf{T}}, \mathrm{d}x, \mathsf{S}, \mathtt{t},
   \alpha^{\beta^{\gamma}}, \hat{x}, \tilde{y}, \vec{z}, \dot{w} \]
\begin{theorem}A theorem.\end{theorem}
\section{Structure}
\begin{itemize}\item one \item two\end{itemize}
\begin{enumerate}\item one \item two\end{enumerate}
\begin{tabular}{lcr}a & b & c\\ 1 & 2 & 3\end{tabular}
\begin{verbatim}verbatim\end{verbatim}
\begin{quote}A quotation.\end{quote}
\end{document}
TEX

cat > "$WORK/seeds/wide.tex" <<'TEX'
\documentclass[12pt,twocolumn]{article}
\usepackage[margin=1in]{geometry}
\usepackage{graphicx}
\usepackage{xcolor}
\usepackage{hyperref}
\usepackage{booktabs}
\usepackage{listings}
\usepackage{url}
\usepackage{enumitem}
\usepackage{caption}
\usepackage{multicol}
\begin{document}
\section{Wide}
\textcolor{blue}{Colour}, \href{https://example.com}{a link}, \url{https://example.com}.
\begin{table}[h]\centering\caption{A table}
\begin{tabular}{lr}\toprule x & y\\ \midrule 1 & 2\\ \bottomrule\end{tabular}
\end{table}
\begin{lstlisting}
int main(void) { return 0; }
\end{lstlisting}
\begin{itemize}[noitemsep]\item tight\end{itemize}
\end{document}
TEX

# core.tex loads lmodern, which substitutes Latin Modern for every Computer
# Modern face — so on its own it would never pull cmex/cmsy/cmmi in. This seed
# is deliberately stock: no fontenc, no lmodern, which is what a document gets
# by default and therefore what the editor must be able to typeset offline.
cat > "$WORK/seeds/stock.tex" <<'TEX'
\documentclass{article}
\usepackage{amsmath,amssymb}
\begin{document}
\section{Stock Computer Modern}
Roman \textit{italic} \textbf{bold} \texttt{mono} \textsf{sans} \textsc{caps}.
{\tiny tiny} {\small small} {\large large} {\Large Large} {\huge huge}
\[ \int_{-\infty}^{\infty} e^{-x^{2}}\,dx = \sqrt{\pi} \]
\[ x^{\int_{a}^{b} f} \quad a^{\sum_{i}^{j} b^{\prod_{k}^{l} c}} \quad
   \sqrt{\sqrt{\sqrt{x}}} \quad \frac{\frac{\frac{a}{b}}{c}}{d} \]
\[ \left[\left(\left\{\frac{1}{2}\right\}\right)\right]^{\left(\frac{1}{2}\right)}
   \quad \oint \bigcup \bigoplus \left\| x \right\|
   \quad \underbrace{a+b}_{c} \overbrace{d}^{e} \]
\[ \mathbb{R}, \mathcal{L}, \mathfrak{g}, \mathbf{A}^{\mathsf{T}},
   \alpha^{\beta^{\gamma}}, \hat{x}, \vec{z} \]
\end{document}
TEX

cat > "$WORK/seeds/report.tex" <<'TEX'
\documentclass{report}
\begin{document}\chapter{One}Text.\end{document}
TEX

cat > "$WORK/seeds/book.tex" <<'TEX'
\documentclass{book}
\begin{document}\chapter{One}Text.\end{document}
TEX

# The Computer Modern families are size-specific: an 11pt document loads cmex8
# where a 10pt one loads cmex7. Sweep the class sizes so the pack holds all of
# them rather than whichever the other seeds happened to reach.
for size in 10pt 11pt 12pt; do
    cat > "$WORK/seeds/sizes-$size.tex" <<TEX
\\documentclass[$size]{article}
\\usepackage{amsmath,amssymb}
\\begin{document}
\\title{S}\\author{S}\\maketitle
\\section{S}\\subsection{S}\\subsubsection{S}
{\\tiny t}{\\scriptsize s}{\\footnotesize f}{\\small s}{\\normalsize n}%
{\\large l}{\\Large L}{\\LARGE L}{\\huge h}{\\Huge H}
\\[ \\int_a^b \\sum_i^j \\prod_k^l \\sqrt{\\sqrt{x}} \\frac{\\frac{a}{b}}{c}
   \\left[\\left(\\frac{1}{2}\\right)\\right]^{\\left\\{x\\right\\}}
   \\oint \\bigcup \\bigoplus \\left\\| y \\right\\| \\]
\\[ x^{\\int_a^b f} \\quad a^{\\sum_i^j b^{\\prod_k^l c}} \\quad
   \\mathbb{R}\\mathcal{L}\\mathfrak{g}\\mathbf{A}\\mathsf{S}\\mathtt{t} \\]
\\end{document}
TEX
done

echo "== compiling seed documents"
for seed in "$WORK"/seeds/*.tex; do
    echo "   $(basename "$seed")"
    # Seeds are allowed to report TeX errors; what matters is which support
    # files they touched on the way.
    (cd "$WORK/seeds" && "$TEXYON" -X compile "$seed" --bundle "$BUNDLE_URL" --outfmt pdf --print >/dev/null 2>&1) || true
done
for project in "${PROJECTS[@]}"; do
    echo "   $project"
    (cd "$project" && "$TEXYON" -X compile main.tex --bundle "$BUNDLE_URL" --outfmt pdf --print >/dev/null 2>&1) || true
done

FORMAT="$(find "$TEXYON_CACHE_DIR/formats" -name '*.fmt' | head -1)"
if [ -z "$FORMAT" ]; then
    echo "no format dump was produced" >&2
    exit 1
fi

DATA_DIR="$(find "$TEXYON_CACHE_DIR/bundles/data" -mindepth 1 -maxdepth 1 -type d | head -1)"
if [ -z "$DATA_DIR" ]; then
    echo "no bundle files were cached" >&2
    exit 1
fi

echo "== packing into $OUT"
rm -rf "$OUT/files"
mkdir -p "$OUT/files"

# The format dump is the single biggest asset, and it is the one file that must
# be present before the engine can start at all, so it gets its own entry.
cp "$FORMAT" "$OUT/files/latex.fmt"

# The ttb cache mirrors the TeX tree, but the engine asks for bare names, so
# the pack flattens to basenames.
count=0
while IFS= read -r -d '' file; do
    name="$(basename "$file")"
    if [ -e "$OUT/files/$name" ]; then
        if ! cmp -s "$file" "$OUT/files/$name"; then
            echo "   warning: duplicate name $name with different content, keeping the first" >&2
        fi
        continue
    fi
    cp "$file" "$OUT/files/$name"
    count=$((count + 1))
done < <(find "$DATA_DIR" -type f -print0)

# Seed compiles swallow errors, so a network flake would otherwise ship a
# silently incomplete pack that only fails at the smoke test or later.
for must in article.cls amsmath.sty cmr5.pfb cmr10.tfm; do
    if [ ! -f "$OUT/files/$must" ]; then
        echo "pack is missing $must, a seed compile probably failed" >&2
        exit 1
    fi
done

# One manifest so the Worker knows the whole set up front and can fetch it in
# parallel rather than discovering names one blocking call at a time.
python3 - "$OUT" <<'PY'
import json, os, sys

out = sys.argv[1]
files_dir = os.path.join(out, "files")

entries = {
    name: os.path.getsize(os.path.join(files_dir, name))
    for name in sorted(os.listdir(files_dir))
}

manifest = {
    "version": 1,
    "format": "latex.fmt",
    "totalBytes": sum(entries.values()),
    "files": entries,
}

with open(os.path.join(out, "manifest.json"), "w") as f:
    json.dump(manifest, f, indent=2)
    f.write("\n")

print(f"   {len(entries)} files, {manifest['totalBytes'] / 1e6:.1f} MB uncompressed")
PY

echo "== done: $count support files plus the format dump"
