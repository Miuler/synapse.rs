import { Vim } from '@replit/codemirror-vim';
import { foldAll, foldEffect, unfoldAll, unfoldCode } from '@codemirror/language';
import type { EditorView } from '@codemirror/view';
import { enclosingFoldRange } from './fold-range';

/** Pliega el bloque que contiene al cursor (no solo el que abre en su línea). */
function foldEnclosing(view: EditorView): boolean {
  const range = enclosingFoldRange(view.state, view.state.selection.main.head);
  if (!range) return false;
  view.dispatch({ effects: foldEffect.of(range) });
  return true;
}

/** Despliega si la línea del cursor tiene un pliegue; si no, pliega el bloque que lo contiene. */
function toggleEnclosing(view: EditorView): boolean {
  return unfoldCode(view) || foldEnclosing(view);
}

/** Atajos de plegado de Vim que el plugin no trae, sobre los comandos de CodeMirror. */
const FOLD_COMMANDS: [keys: string, action: string, command: (view: EditorView) => boolean][] = [
  ['zc', 'synapseFoldCode', foldEnclosing],
  ['zo', 'synapseUnfoldCode', unfoldCode],
  ['za', 'synapseToggleFold', toggleEnclosing],
  ['zM', 'synapseFoldAll', foldAll],
  ['zR', 'synapseUnfoldAll', unfoldAll],
];

let registered = false;

/** Registra zc, zo, za, zM y zR en el modo normal de Vim (una vez, el mapa de Vim es global). */
export function registerVimFolding(): void {
  if (registered) return;
  registered = true;
  for (const [keys, action, command] of FOLD_COMMANDS) {
    Vim.defineAction(action, (cm) => {
      command(cm.cm6 as EditorView);
    });
    Vim.mapCommand(keys, 'action', action, {}, { context: 'normal' });
  }
}
