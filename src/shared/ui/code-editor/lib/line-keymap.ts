import { Prec } from '@codemirror/state';
import { indentUnit } from '@codemirror/language';
import { EditorView, keymap } from '@codemirror/view';
import { copyLineDown, indentLess, indentMore } from '@codemirror/commands';
import { getCM, Vim } from '@replit/codemirror-vim';

const INDENT = '    ';

/** Solo lectura: Tab sigue moviendo el foco. */
function isReadOnly(view: EditorView): boolean {
  return view.state.readOnly || !view.state.facet(EditorView.editable);
}

/** En Vim, Tab solo edita en modo inserción; en los demás modos se consume sin hacer nada. */
function isVimOutsideInsert(view: EditorView): boolean {
  const vimState = getCM(view)?.state.vim;
  return !!vimState && !vimState.insertMode;
}

/** Tab inserta la sangría en el cursor; con texto seleccionado sangra las líneas. */
function tabForward(view: EditorView): boolean {
  if (isReadOnly(view)) return false;
  if (isVimOutsideInsert(view)) return true;
  if (view.state.selection.ranges.some((r) => !r.empty)) return indentMore(view);
  view.dispatch(view.state.replaceSelection(INDENT), { scrollIntoView: true, userEvent: 'input' });
  return true;
}

function tabBackward(view: EditorView): boolean {
  if (isReadOnly(view)) return false;
  if (isVimOutsideInsert(view)) return true;
  indentLess(view);
  return true;
}

/** Desplaza la vista `lines` líneas sin mover el cursor (Ctrl+E / Ctrl+Y de Vim). */
function scrollLines(lines: number) {
  return (view: EditorView): boolean => {
    view.scrollDOM.scrollTop += lines * view.defaultLineHeight;
    return true;
  };
}

let vimRegistered = false;

/** En Vim, Ctrl+D es media página; se reasigna a duplicar la línea (Ctrl+E/Ctrl+Y ya los trae Vim). */
function registerVimDuplicateLine(): void {
  if (vimRegistered) return;
  vimRegistered = true;
  Vim.defineAction('synapseDuplicateLine', (cm) => {
    copyLineDown(cm.cm6 as EditorView);
  });
  Vim.mapCommand('<C-d>', 'action', 'synapseDuplicateLine', {}, { context: 'normal' });
  Vim.mapCommand('<C-d>', 'action', 'synapseDuplicateLine', {}, { context: 'insert' });
}

/**
 * Ctrl+E baja la pantalla una línea, Ctrl+Y la sube y Ctrl+D duplica la línea; Tab y Shift+Tab sangran
 * sin que el editor pierda el foco. Prioridad alta para ganarle a Vim (media página con Ctrl+D) y a seleccionar la siguiente coincidencia.
 */
export function lineKeymap() {
  registerVimDuplicateLine();
  return [
    indentUnit.of(INDENT),
    // CodeMirror deja que Esc seguido de Tab saque el foco; en Vim, Esc se pulsa constantemente.
    EditorView.domEventObservers({
      keydown(e, view) {
        if (e.key === 'Escape') queueMicrotask(() => view.setTabFocusMode(false));
      },
    }),
    Prec.highest(
      keymap.of([
        { key: 'Tab', run: tabForward },
        { key: 'Shift-Tab', run: tabBackward },
        { key: 'Ctrl-e', run: scrollLines(1), preventDefault: true },
        { key: 'Ctrl-y', run: scrollLines(-1), preventDefault: true },
        { key: 'Ctrl-d', run: copyLineDown, preventDefault: true },
      ]),
    ),
  ];
}
