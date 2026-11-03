import { Prec } from '@codemirror/state';
import { keymap, type EditorView } from '@codemirror/view';
import { copyLineDown } from '@codemirror/commands';
import { Vim } from '@replit/codemirror-vim';

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
 * Ctrl+E baja la pantalla una línea, Ctrl+Y la sube y Ctrl+D duplica la línea. Prioridad alta para
 * ganarle a Vim (media página con Ctrl+D) y a seleccionar la siguiente coincidencia.
 */
export function lineKeymap() {
  registerVimDuplicateLine();
  return Prec.highest(
    keymap.of([
      { key: 'Ctrl-e', run: scrollLines(1), preventDefault: true },
      { key: 'Ctrl-y', run: scrollLines(-1), preventDefault: true },
      { key: 'Ctrl-d', run: copyLineDown, preventDefault: true },
    ]),
  );
}
