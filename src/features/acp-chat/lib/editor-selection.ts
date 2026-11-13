import { EditorView } from '@codemirror/view';

export interface CapturedSelection {
  text: string;
  /** Líneas 1-based; 0 si la selección viene de la vista de lectura y no hay líneas. */
  startLine: number;
  endLine: number;
}

/** Selección actual del editor activo (CodeMirror) o, si no hay, la del texto renderizado. */
export function captureSelection(): CapturedSelection | null {
  const focused = document.activeElement?.closest('.cm-editor');
  const visible = document.querySelector('.tab-pane:not(.hidden) .cm-editor');
  const dom = (focused ?? visible) as HTMLElement | null;
  const view = dom ? EditorView.findFromDOM(dom) : null;
  if (view) {
    const sel = view.state.selection.main;
    if (!sel.empty) {
      const doc = view.state.doc;
      return {
        text: doc.sliceString(sel.from, sel.to),
        startLine: doc.lineAt(sel.from).number,
        endLine: doc.lineAt(sel.to).number,
      };
    }
  }
  const text = window.getSelection()?.toString() ?? '';
  return text.trim() ? { text, startLine: 0, endLine: 0 } : null;
}
