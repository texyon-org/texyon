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
import { yCollab, yUndoManagerKeymap } from "y-codemirror.next";
import type { Awareness } from "y-protocols/awareness";
import * as Y from "yjs";

import type { CompletionSources } from "./completion";
import { latexCompletions } from "./completion";
import { toLintDiagnostics } from "./diagnostics";
import { latexHighlighting } from "./highlight";
import { latex } from "./latex";

export interface EditorProps {
  /** The file being edited, which is also what diagnostics are filtered to. */
  path: string;
  /** Its text as the workspace has it. */
  text: string;
  /** Everything completion reads: the project, the engine and the corpus. */
  sources: CompletionSources;
  /** The engine's latest diagnostics, for the whole project. */
  diagnostics: readonly Diagnostic[];
  /** Where to put the cursor, when a backward search asked for one. */
  jumpToLine: { line: number } | null;
  onChange: (text: string) => void;
  /** The cursor moved to this line, for a forward search. */
  onCursorLine: (line: number) => void;
  /** For someone the project is shared with read-only. */
  readOnly?: boolean;
  /** Edit a shared text instead: changes go to it, and other people's cursors show. */
  shared?: { text: Y.Text; awareness: Awareness | null } | null;
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
  sources,
  diagnostics,
  jumpToLine,
  onChange,
  onCursorLine,
  readOnly = false,
  shared = null,
}: EditorProps) {
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);

  // Read through refs so the extensions below can be built once and still see
  // current values; rebuilding them would discard the undo history.
  const latest = useRef({ sources, onChange, onCursorLine });
  latest.current = { sources, onChange, onCursorLine };

  useEffect(() => {
    const parent = host.current;
    if (parent === null) return;

    const sharedText = shared?.text ?? null;
    const extensions: Extension[] = [
      // Undo in a shared text reverts only your own edits, never someone else's.
      sharedText === null
        ? history()
        : yCollab(sharedText, shared?.awareness ?? null, { undoManager: new Y.UndoManager(sharedText) }),
      EditorState.readOnly.of(readOnly),
      EditorView.editable.of(!readOnly),
      lineNumbers(),
      highlightActiveLineGutter(),
      highlightActiveLine(),
      indentOnInput(),
      bracketMatching(),
      closeBrackets(),
      lintGutter(),
      latex,
      // The mode assigns tags; this is what draws them.
      latexHighlighting,
      autocompletion({
        override: [latexCompletions(() => latest.current.sources)],
        // TeX is punctuation-heavy; completing on every character would fire
        // constantly inside maths.
        activateOnTyping: true,
        closeOnBlur: true,
      }),
      keymap.of([
        ...closeBracketsKeymap,
        ...defaultKeymap,
        ...searchKeymap,
        ...(sharedText === null ? historyKeymap : yUndoManagerKeymap),
        indentWithTab,
      ]),
      EditorView.lineWrapping,
      EditorView.updateListener.of((update) => {
        // A shared text reaches the workspace through the document instead.
        if (update.docChanged && sharedText === null) {
          latest.current.onChange(update.state.doc.toString());
        }
        if (update.selectionSet) {
          const head = update.state.selection.main.head;
          latest.current.onCursorLine(update.state.doc.lineAt(head).number);
        }
      }),
    ];

    const created = new EditorView({
      state: EditorState.create({ doc: sharedText?.toString() ?? text, extensions }),
      parent,
    });
    view.current = created;

    return () => {
      created.destroy();
      view.current = null;
      // Otherwise the others keep seeing a cursor in a file this editor left.
      shared?.awareness?.setLocalStateField("cursor", null);
    };
    // Built once per file. `text` is read at creation and thereafter through
    // transactions, so it is deliberately not a dependency.
    // A shared text keeps its view through a rename, so whoever is typing keeps focus.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [shared?.text ?? path, readOnly]);

  // The workspace changed this file from somewhere other than this editor —
  // another pane, a file being loaded. Replacing a document that already
  // matches would move the cursor to the start on every keystroke.
  useEffect(() => {
    const current = view.current;
    if (current === null || shared !== null) return;
    if (current.state.doc.toString() === text) return;

    current.dispatch({
      changes: { from: 0, to: current.state.doc.length, insert: text },
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
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
    if (jumpToLine.line < 1 || jumpToLine.line > current.state.doc.lines) return;

    const line = current.state.doc.line(jumpToLine.line);
    current.dispatch({
      selection: { anchor: line.from },
      effects: EditorView.scrollIntoView(line.from, { y: "center" }),
    });
    current.focus();
  }, [jumpToLine]);

  return <div className="editor" ref={host} />;
}
