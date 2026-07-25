import { autocompletion, closeBrackets, closeBracketsKeymap } from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { bracketMatching, indentOnInput } from "@codemirror/language";
import { lintGutter, setDiagnostics } from "@codemirror/lint";
import { searchKeymap } from "@codemirror/search";
import { EditorState, type Extension } from "@codemirror/state";
import {
  EditorView,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
} from "@codemirror/view";
import type { Diagnostic } from "@tectonic/wasm";
import { useEffect, useRef } from "react";

import type { SymbolIndex } from "../project/symbols";
import { latexCompletions } from "./completion";
import { toLintDiagnostics } from "./diagnostics";
import { latexHighlighting } from "./highlight";
import { latex } from "./latex";

export interface EditorProps {
  /** The file being edited, which is also what diagnostics are filtered to. */
  path: string;
  /** Its text as the workspace has it. */
  text: string;
  /** Everything the project declares, for completion. */
  symbols: SymbolIndex;
  /** The engine's latest diagnostics, for the whole project. */
  diagnostics: readonly Diagnostic[];
  /** Where to put the cursor, when a backward search asked for one. */
  jumpToLine: number | null;
  onChange: (text: string) => void;
  /** The cursor moved to this line, for a forward search. */
  onCursorLine: (line: number) => void;
}

/**
 * The text half of the editor.
 *
 * CodeMirror owns a mutable view that React must not re-create, so the view is
 * built once and everything after that is a transaction against it. The two
 * places this matters are commented below: replacing the document when the file
 * changes, and *not* replacing it when the change came from here.
 */
export function Editor({
  path,
  text,
  symbols,
  diagnostics,
  jumpToLine,
  onChange,
  onCursorLine,
}: EditorProps) {
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);

  // Read through refs so the extensions below can be built once and still see
  // current values; rebuilding them would discard the undo history.
  const latest = useRef({ symbols, onChange, onCursorLine });
  latest.current = { symbols, onChange, onCursorLine };

  useEffect(() => {
    const parent = host.current;
    if (parent === null) return;

    const extensions: Extension[] = [
      lineNumbers(),
      highlightActiveLineGutter(),
      highlightActiveLine(),
      history(),
      indentOnInput(),
      bracketMatching(),
      closeBrackets(),
      lintGutter(),
      latex,
      // The mode assigns tags; this is what draws them.
      latexHighlighting,
      autocompletion({
        override: [latexCompletions(() => latest.current.symbols)],
        // TeX is punctuation-heavy; completing on every character would fire
        // constantly inside maths.
        activateOnTyping: true,
        closeOnBlur: true,
      }),
      keymap.of([
        ...closeBracketsKeymap,
        ...defaultKeymap,
        ...searchKeymap,
        ...historyKeymap,
        indentWithTab,
      ]),
      EditorView.lineWrapping,
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          latest.current.onChange(update.state.doc.toString());
        }
        if (update.selectionSet) {
          const head = update.state.selection.main.head;
          latest.current.onCursorLine(update.state.doc.lineAt(head).number);
        }
      }),
    ];

    const created = new EditorView({
      state: EditorState.create({ doc: text, extensions }),
      parent,
    });
    view.current = created;

    return () => {
      created.destroy();
      view.current = null;
    };
    // Built once per file. `text` is read at creation and thereafter through
    // transactions, so it is deliberately not a dependency.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [path]);

  // The workspace changed this file from somewhere other than this editor —
  // another pane, a file being loaded. Replacing a document that already
  // matches would move the cursor to the start on every keystroke.
  useEffect(() => {
    const current = view.current;
    if (current === null) return;
    if (current.state.doc.toString() === text) return;

    current.dispatch({
      changes: { from: 0, to: current.state.doc.length, insert: text },
    });
  }, [text]);

  useEffect(() => {
    const current = view.current;
    if (current === null) return;

    current.dispatch(setDiagnostics(current.state, toLintDiagnostics(diagnostics, path, current.state.doc)));
  }, [diagnostics, path]);

  // A backward search landed here: put the cursor on the line and bring it into
  // view, centred rather than scrolled to the edge.
  useEffect(() => {
    const current = view.current;
    if (current === null || jumpToLine === null) return;
    if (jumpToLine < 1 || jumpToLine > current.state.doc.lines) return;

    const line = current.state.doc.line(jumpToLine);
    current.dispatch({
      selection: { anchor: line.from },
      effects: EditorView.scrollIntoView(line.from, { y: "center" }),
    });
    current.focus();
  }, [jumpToLine]);

  return <div className="editor" ref={host} />;
}
