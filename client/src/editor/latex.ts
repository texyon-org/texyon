/**
 * LaTeX for CodeMirror.
 *
 * The `stex` stream mode rather than a Lezer grammar. A stream mode tokenizes
 * line by line without building a tree, which for TeX is the honest trade:
 * a tree would have to know what macros expand to before it could say what most
 * of a document *is*, and an editor cannot wait for macro expansion to colour a
 * word. What syntax highlighting is actually for — telling commands, maths and
 * comments apart — a stream mode does exactly.
 */

import { StreamLanguage } from "@codemirror/language";
import { stex } from "@codemirror/legacy-modes/mode/stex";

export const latex = StreamLanguage.define(stex);
