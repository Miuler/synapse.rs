import { Vim } from '@replit/codemirror-vim';
import { foldAll, foldCode, toggleFold, unfoldAll, unfoldCode } from '@codemirror/language';
import type { EditorView } from '@codemirror/view';

/** Atajos de plegado de Vim que el plugin no trae, sobre los comandos de CodeMirror. */
const FOLD_COMMANDS: [keys: string, action: string, command: (view: EditorView) => boolean][] = [
  ['zc', 'synapseFoldCode', foldCode],
  ['zo', 'synapseUnfoldCode', unfoldCode],
  ['za', 'synapseToggleFold', toggleFold],
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
