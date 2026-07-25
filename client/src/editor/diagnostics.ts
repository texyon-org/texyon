/**
 * The engine's diagnostics, in the editor's gutter.
 *
 * The engine already reports `(severity, path, line, message)` — recovered from
 * the TeX transcript and attributed to the file that actually caused each one —
 * so this is a translation rather than an analysis. Two things it does have to
 * decide: which diagnostics belong to the file on screen, and where on a line
 * to put a marker the engine only located to line granularity.
 */

import type { Diagnostic as LintDiagnostic } from "@codemirror/lint";
import type { Text } from "@codemirror/state";
import type { Diagnostic as EngineDiagnostic } from "@tectonic/wasm";

/**
 * Turn the engine's diagnostics into the ones CodeMirror draws.
 *
 * `path` is the file currently open: a diagnostic raised in another file of the
 * project has nowhere to go in this editor, and putting it on a line here would
 * point at text that has nothing to do with it.
 *
 * `info` is left out. LaTeX narrates its font substitutions, and a gutter full
 * of those is a gutter nobody reads.
 */
export function toLintDiagnostics(
  diagnostics: readonly EngineDiagnostic[],
  path: string,
  doc: Text,
): LintDiagnostic[] {
  const found: LintDiagnostic[] = [];

  for (const diagnostic of diagnostics) {
    if (diagnostic.severity === "info") continue;
    if (!belongsTo(diagnostic.path, path)) continue;

    const line = diagnostic.line ?? 1;
    if (line < 1 || line > doc.lines) continue;

    const { from, to, text } = doc.line(line);

    found.push({
      // Marking the trimmed text rather than the whole line, so a marker on an
      // indented line does not underline the indentation.
      from: from + Math.max(0, text.length - text.trimStart().length),
      to: Math.max(from + 1, to),
      severity: diagnostic.severity,
      message: diagnostic.message,
      source: "tex",
    });
  }

  return found;
}

/**
 * Whether a diagnostic's file is the one on screen.
 *
 * The transcript names files as TeX opened them, which is usually exactly the
 * path the project uses but can be relative to somewhere else. Comparing the
 * tail is what makes `chapters/one.tex` and `./chapters/one.tex` the same file
 * without pretending that `one.tex` and `appendix/one.tex` are.
 */
function belongsTo(reported: string | null, open: string): boolean {
  if (reported === null) return false;

  const left = normalize(reported);
  const right = normalize(open);
  if (left === right) return true;

  return left.endsWith(`/${right}`) || right.endsWith(`/${left}`);
}

function normalize(path: string): string {
  return path.replace(/^\.\//, "").replace(/^\/+/, "");
}
