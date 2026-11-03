import { Vim } from '@replit/codemirror-vim';
import { foldEffect, unfoldAll, unfoldCode, unfoldEffect } from '@codemirror/language';
import type { EditorView } from '@codemirror/view';
import { allFoldRanges, enclosingFoldRange, foldRangesWithin, foldsOnLine, foldsWithin } from './fold-range';

/** zc: pliega el bloque que contiene al cursor (no solo el que abre en su línea). */
function foldEnclosing(view: EditorView): boolean {
  const range = enclosingFoldRange(view.state, view.state.selection.main.head);
  if (!range) return false;
  view.dispatch({ effects: foldEffect.of(range) });
  return true;
}

/** zC: pliega el bloque que contiene al cursor y todos los que tiene dentro. */
function foldEnclosingRecursive(view: EditorView): boolean {
  const range = enclosingFoldRange(view.state, view.state.selection.main.head);
  if (!range) return false;
  view.dispatch({ effects: foldRangesWithin(view.state, range).map((r) => foldEffect.of(r)) });
  return true;
}

/** zO: despliega el pliegue de la línea del cursor y todos los que tiene dentro. */
function unfoldRecursive(view: EditorView): boolean {
  const folds = foldsOnLine(view.state, view.state.selection.main.head);
  if (!folds.length) return false;
  const effects = folds.flatMap((fold) => foldsWithin(view.state, fold)).map((r) => unfoldEffect.of(r));
  view.dispatch({ effects });
  return true;
}

/** za: despliega si la línea del cursor tiene un pliegue; si no, pliega el bloque que lo contiene. */
function toggleEnclosing(view: EditorView): boolean {
  return unfoldCode(view) || foldEnclosing(view);
}

/**
 * zM: pliega todos los bloques de todos los niveles. foldAll de CodeMirror solo pliega los
 * exteriores, y al abrir uno con zo los internos aparecían abiertos (en Vim siguen cerrados).
 */
function foldAllLevels(view: EditorView): boolean {
  const ranges = allFoldRanges(view.state);
  if (!ranges.length) return false;
  view.dispatch({ effects: ranges.map((r) => foldEffect.of(r)) });
  return true;
}

/** Atajos de plegado de Vim que el plugin no trae, con el comportamiento de Vim. */
const FOLD_COMMANDS: [keys: string, action: string, command: (view: EditorView) => boolean][] = [
  ['zc', 'synapseFoldCode', foldEnclosing],
  ['zC', 'synapseFoldCodeRecursive', foldEnclosingRecursive],
  ['zo', 'synapseUnfoldCode', unfoldCode],
  ['zO', 'synapseUnfoldCodeRecursive', unfoldRecursive],
  ['za', 'synapseToggleFold', toggleEnclosing],
  ['zM', 'synapseFoldAll', foldAllLevels],
  ['zR', 'synapseUnfoldAll', unfoldAll],
];

let registered = false;

/** Registra los atajos de plegado en el modo normal de Vim (una vez, el mapa de Vim es global). */
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
