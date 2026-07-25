import { useEffect, useRef } from "react";
import { EditorView, keymap, lineNumbers, highlightActiveLine } from "@codemirror/view";
import { EditorState } from "@codemirror/state";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { StreamLanguage } from "@codemirror/language";
import { stex } from "@codemirror/legacy-modes/mode/stex";

export interface EditorProps {
  /** Initial text. Later changes to this prop are ignored — the editor owns it. */
  initialValue: string;
  /** Called on every document change, synchronously with the keystroke. */
  onChange: (value: string) => void;
}

/**
 * The LaTeX editor.
 *
 * CodeMirror is set up once and left alone: the component never pushes state
 * into it on re-render, so typing never contends with React. `onChange` fires
 * from the update listener, which is as early as the new text exists.
 */
export function Editor({ initialValue, onChange }: EditorProps) {
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  // Held in a ref so a changing callback identity never rebuilds the editor.
  const notify = useRef(onChange);
  notify.current = onChange;

  useEffect(() => {
    if (!host.current) return;

    const state = EditorState.create({
      doc: initialValue,
      extensions: [
        lineNumbers(),
        highlightActiveLine(),
        history(),
        keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
        StreamLanguage.define(stex),
        EditorView.lineWrapping,
        EditorView.updateListener.of((update) => {
          if (update.docChanged) notify.current(update.state.doc.toString());
        }),
        EditorView.theme({
          "&": { height: "100%", fontSize: "13px" },
          ".cm-scroller": {
            fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
            lineHeight: "1.6",
          },
          "&.cm-focused": { outline: "none" },
        }),
      ],
    });

    view.current = new EditorView({ state, parent: host.current });

    return () => {
      view.current?.destroy();
      view.current = null;
    };
    // Deliberately mount-only: this editor is uncontrolled.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return <div className="editor" ref={host} />;
}
