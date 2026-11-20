// Marcar o desmarcar tareas desde el teclado (como «Toggle checkbox status» de Obsidian), con el atajo
// configurable `toggle_task` (Ctrl+Enter por defecto). Funciona también con Vim, en cualquier modo.
import { Prec, type ChangeSpec } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { matchesShortcut, shortcutsConfig } from '@entities/settings';
import { toggleTaskLine } from './task-line';

export { toggleTaskLine };

/** Aplica `toggleTaskLine` a cada línea tocada por la selección (una vez por línea). */
export function toggleTasks(view: EditorView): boolean {
  const { state } = view;
  if (state.readOnly || !state.facet(EditorView.editable)) return false;
  const lines = new Set<number>();
  for (const r of state.selection.ranges) {
    const from = state.doc.lineAt(r.from).number;
    const to = state.doc.lineAt(r.to).number;
    for (let n = from; n <= to; n++) lines.add(n);
  }
  const changes: ChangeSpec[] = [];
  for (const n of lines) {
    const line = state.doc.line(n);
    const next = toggleTaskLine(line.text);
    if (next !== line.text) changes.push({ from: line.from, to: line.to, insert: next });
  }
  if (!changes.length) return false;
  view.dispatch({ changes, userEvent: 'input.toggle-task', scrollIntoView: true });
  return true;
}

/** Atajo `toggle_task` en el editor; antes que Vim y que el salto de línea de Ctrl+Enter. */
export function taskToggleExtension() {
  return Prec.highest(
    EditorView.domEventHandlers({
      keydown(e, view) {
        if (!matchesShortcut(e, shortcutsConfig.workspace.toggle_task)) return false;
        if (!toggleTasks(view)) return false;
        e.preventDefault();
        e.stopPropagation();
        return true;
      },
    }),
  );
}
