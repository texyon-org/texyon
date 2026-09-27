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
\usepackage{tikz}
\usepackage{hyperref}

\title{A Document}
\author{You}

\begin{document}
\maketitle

\section{Introduction}\label{sec:intro}

Type on the left; the preview keeps up. The engine is resident, so an
edit replays from a checkpoint just before your cursor rather than
starting over.

\input{chapters/method}

\end{document}
`,

  "chapters/method.tex": String.raw`\section{Method}\label{sec:method}

\begin{equation}\label{eq:gauss}
  \int_{-\infty}^{\infty} e^{-x^2}\,\mathrm{d}x = \sqrt{\pi}
\end{equation}

Equation \eqref{eq:gauss} is the one everybody knows.

\begin{figure}[h]
  \centering
  \begin{tikzpicture}[scale=1.2]
    \draw[fill=blue!10] (0,0) -- (3,0) -- (3,2) -- cycle;
    \draw (2.7,0) -- (2.7,0.3) -- (3,0.3);
    \node[below] at (1.5,0) {$a$};
    \node[right] at (3,1) {$b$};
    \node[above left] at (1.5,1) {$c$};
    \draw[->, thick] (-0.5,0) -- (4,0);
    \draw[->, thick] (0,-0.5) -- (0,2.5);
  \end{tikzpicture}
  \caption{A right triangle, so $a^2 + b^2 = c^2$.}\label{fig:triangle}
\end{figure}

Figure \ref{fig:triangle} is drawn with TikZ.
`,
};
