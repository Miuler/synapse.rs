import { EditorSelection, EditorState, Prec } from '@codemirror/state';
import { keymap, type EditorView } from '@codemirror/view';

/** Límite de coincidencias seleccionadas, para no congelar el editor en archivos enormes. */
const MAX_OCCURRENCES = 2_000;

/**
 * Selecciona la palabra bajo el cursor y todas sus coincidencias exactas de palabra completa
 * (`server` no incluye `servers` ni `myserver`). La selección principal queda en la del cursor.
 */
export function occurrenceSelection(state: EditorState): EditorSelection | null {
  const head = state.selection.main.head;
  const word = state.wordAt(head);
  if (!word) return null;
  const text = state.sliceDoc(word.from, word.to);
  const isWordChar = (pos: number) => {
    if (pos < 0 || pos >= state.doc.length) return false;
    return state.charCategorizer(pos)(state.sliceDoc(pos, pos + 1)) === 0;
  };

  const ranges = [];
  let main = 0;
  const doc = state.doc.toString();
  for (let at = doc.indexOf(text); at !== -1 && ranges.length < MAX_OCCURRENCES; at = doc.indexOf(text, at + text.length)) {
    const end = at + text.length;
    if (isWordChar(at - 1) || isWordChar(end)) continue;
    if (at === word.from) main = ranges.length;
    ranges.push(EditorSelection.range(at, end));
  }
  return ranges.length ? EditorSelection.create(ranges, main) : null;
}

/** Shift+F6: como renombrar en JetBrains, pero textual: lo que se escribe cambia todas las coincidencias. */
export function selectOccurrences(view: EditorView): boolean {
  const selection = occurrenceSelection(view.state);
  if (!selection) return false;
  view.dispatch({ selection, scrollIntoView: true, userEvent: 'select' });
  return true;
}

/** Atajo con prioridad alta para que el modo Vim no se lo quede. Necesita selección múltiple activa. */
export function renameOccurrencesKeymap() {
  return [EditorState.allowMultipleSelections.of(true), Prec.highest(keymap.of([{ key: 'Shift-F6', run: selectOccurrences }]))];
}
