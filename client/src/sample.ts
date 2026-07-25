/**
 * A project to open with, so the editor has something to show before anything
 * has been loaded from a server.
 *
 * Deliberately more than one file: the point of the thing is that a project is
 * a set of files, and a sample with one would not exercise the file tree, the
 * cross-file completion, or a diagnostic attributed to a child.
 */
export const SAMPLE: Record<string, string> = {
  "main.tex": String.raw`\documentclass[11pt]{article}
\usepackage{amsmath}
\usepackage{hyperref}

\newcommand{\vect}[1]{\mathbf{#1}}

\title{A Document}
\author{You}

\begin{document}
\maketitle

\section{Introduction}\label{sec:intro}

Type on the left; the preview keeps up. The engine is resident, so an
edit replays from a checkpoint just before your cursor rather than
starting over.

\input{chapters/method}

\section{Closing}

Back in \ref{sec:method} we set out the method. See also
\url{https://tectonic-typesetting.github.io/}.

\bibliographystyle{plain}
\bibliography{refs}

\end{document}
`,

  "chapters/method.tex": String.raw`\section{Method}\label{sec:method}

A vector, written with the command the preamble defines: $\vect{x}$.

\begin{equation}\label{eq:gauss}
  \int_{-\infty}^{\infty} e^{-x^2}\,\mathrm{d}x = \sqrt{\pi}
\end{equation}

Equation \eqref{eq:gauss} is the one everybody knows.
`,

  "refs.bib": String.raw`@book{knuth1984,
  author    = {Donald E. Knuth},
  title     = {The {\TeX}book},
  publisher = {Addison-Wesley},
  year      = {1984},
}

@book{lamport1994,
  author    = {Leslie Lamport},
  title     = {{\LaTeX}: A Document Preparation System},
  publisher = {Addison-Wesley},
  year      = {1994},
}
`,
};
