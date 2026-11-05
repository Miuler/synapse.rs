import { openSearchPanel } from '@codemirror/search';
import { EditorView } from '@codemirror/view';

/** Abre el panel de búsqueda con el foco en el campo de reemplazo (no existe si es de solo lectura). */
export function openReplacePanel(editor: HTMLElement): void {
  const view = EditorView.findFromDOM(editor);
  if (!view) return;
  openSearchPanel(view);
  const field = view.dom.querySelector<HTMLInputElement>('.cm-search input[name="replace"]');
  field?.focus();
  field?.select();
}
