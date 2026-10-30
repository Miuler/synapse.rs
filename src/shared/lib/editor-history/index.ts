import { Annotation, Prec, StateField, type Extension } from '@codemirror/state';
import { EditorView, keymap } from '@codemirror/view';
import { undo, redo } from '@codemirror/commands';
import { Vim } from '@replit/codemirror-vim';

/** Última línea modificada y su texto antes de la primera edición de la racha actual */
interface LineUndoState {
  line: number;
  original: string;
}

/** Transacciones propias de `U`: llevan el nuevo estado, porque `U` se puede alternar (revertir con otra `U`) */
const lineUndoAnnotation = Annotation.define<LineUndoState>();

/**
 * Equivalente al `U` estándar de vim: recuerda la última línea editada y el texto que tenía
 * antes de que empezara a modificarse. Los cambios que afectan a saltos de línea, el undo/redo
 * normal y las ediciones en otra línea reinician el seguimiento.
 */
const lineUndoField = StateField.define<LineUndoState | null>({
  create: () => null,
  update(value, tr) {
    if (!tr.docChanged) return value;

    const own = tr.annotation(lineUndoAnnotation);
    if (own) return own;
    if (tr.isUserEvent('undo') || tr.isUserEvent('redo')) return null;

    let touched: LineUndoState | null = null;
    let multiLine = false;
    tr.changes.iterChanges((fromA, toA, fromB, _toB, inserted) => {
      if (inserted.lines > 1 || tr.startState.doc.sliceString(fromA, toA).includes('\n')) {
        multiLine = true;
        return;
      }
      const line = tr.state.doc.lineAt(fromB).number;
      if (touched && touched.line !== line) multiLine = true;
      touched = { line, original: tr.startState.doc.lineAt(fromA).text };
    });
    if (multiLine || !touched) return null;

    const edited = touched as LineUndoState;
    // Misma línea que la racha anterior: se conserva el texto original de la primera edición
    return value && value.line === edited.line ? value : edited;
  },
});

function lineUndo(view: EditorView): boolean {
  const state = view.state.field(lineUndoField, false);
  if (!state || state.line > view.state.doc.lines) return false;

  const line = view.state.doc.line(state.line);
  view.dispatch({
    changes: { from: line.from, to: line.to, insert: state.original },
    selection: { anchor: line.from },
    annotations: lineUndoAnnotation.of({ line: state.line, original: line.text }),
    scrollIntoView: true,
  });
  return true;
}

let vimUndoConfigured = false;

/**
 * Atajos de vim para el historial: `u` y `Ctrl+r` ya vienen de serie; aquí se añade `U`
 * (deshacer los cambios de la última línea). Los mapeos de vim son globales, por lo que se
 * registran una sola vez.
 */
function configureVimLineUndo(): void {
  if (vimUndoConfigured) return;
  vimUndoConfigured = true;
  Vim.defineAction('synapseLineUndo', (cm: any) => {
    lineUndo(cm.cm6 as EditorView);
  });
  Vim.mapCommand('U', 'action', 'synapseLineUndo', {}, { context: 'normal' });
}

/**
 * Deshacer y rehacer reutilizable para cualquier editor CodeMirror.
 *
 * - `Ctrl+Z` / `Cmd+Z`: deshacer.
 * - `Ctrl+Shift+Z` / `Cmd+Shift+Z` (y `Ctrl+Y`): rehacer.
 * - Modo vim: `u` deshace, `Ctrl+r` rehace y `U` revierte los cambios de la última línea editada
 *   (otra `U` los restablece).
 *
 * Requiere que el editor ya tenga `history()` (incluido en `basicSetup`). Se usa precedencia máxima
 * para que ninguna otra extensión (plugins, vim) intercepte los atajos antes.
 */
export function undoRedoExtension(): Extension {
  configureVimLineUndo();
  return [
    lineUndoField,
    Prec.highest(
      keymap.of([
        { key: 'Mod-z', run: undo, preventDefault: true },
        { key: 'Mod-Shift-z', run: redo, preventDefault: true },
        { key: 'Mod-y', run: redo, preventDefault: true },
      ])
    ),
  ];
}
