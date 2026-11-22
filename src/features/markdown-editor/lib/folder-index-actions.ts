// Botones de la vista de un index.md OKF (índice de carpeta): crear archivos y subcarpetas en esa
// carpeta. La vista avisa con un evento y el espacio de trabajo lo atiende.

export type NewFileKind = 'markdown' | 'mermaid' | 'd2' | 'plantuml' | 'excalidraw' | 'obsidian-excalidraw';

export const NEW_FILE_KINDS: { kind: NewFileKind; label: string }[] = [
  { kind: 'markdown', label: 'Nota Markdown' },
  { kind: 'mermaid', label: 'Diagrama Mermaid' },
  { kind: 'd2', label: 'Diagrama D2' },
  { kind: 'plantuml', label: 'Diagrama PlantUML' },
  { kind: 'excalidraw', label: 'Dibujo Excalidraw' },
  { kind: 'obsidian-excalidraw', label: 'Dibujo Excalidraw (Obsidian, .excalidraw.md)' },
];

export const FOLDER_INDEX_EVENT = 'synapse:folder-index-action';

export type FolderIndexAction = { folder: string } & ({ action: 'new-file'; kind: NewFileKind } | { action: 'new-folder' });

/** ¿Es el index.md generado por Synapse? */
export function isGeneratedFolderIndex(path: string | null | undefined, content: string): boolean {
  return !!path && /(^|\/)index\.md$/i.test(path) && content.includes('<!-- okf-index: generado por Synapse');
}

/**
 * Añade a la vista del índice los botones: «+ Nuevo…» (desplegable de tipos) junto a «Archivos» o
 * «Conceptos» y «+ Carpeta» junto a «Carpetas»; los que no tengan su sección van arriba.
 */
export function decorateFolderIndex(container: HTMLElement, folder: string): void {
  container.querySelectorAll('.folder-index-actions').forEach((e) => e.remove());
  const emit = (detail: FolderIndexAction) => window.dispatchEvent(new CustomEvent(FOLDER_INDEX_EVENT, { detail }));

  const newSelect = () => {
    const sel = document.createElement('select');
    sel.className = 'folder-index-new';
    sel.title = 'Crear un archivo en esta carpeta';
    const first = document.createElement('option');
    first.value = '';
    first.textContent = '+ Nuevo…';
    sel.appendChild(first);
    for (const { kind, label } of NEW_FILE_KINDS) {
      const o = document.createElement('option');
      o.value = kind;
      o.textContent = label;
      sel.appendChild(o);
    }
    sel.addEventListener('change', () => {
      const kind = sel.value as NewFileKind;
      sel.value = '';
      if (kind) emit({ folder, action: 'new-file', kind });
    });
    return sel;
  };
  const folderButton = () => {
    const b = document.createElement('button');
    b.type = 'button';
    b.className = 'folder-index-new-folder';
    b.textContent = '+ Carpeta';
    b.title = 'Crear una subcarpeta en esta carpeta';
    b.addEventListener('click', () => emit({ folder, action: 'new-folder' }));
    return b;
  };
  const wrap = (...children: HTMLElement[]) => {
    const span = document.createElement('span');
    span.className = 'folder-index-actions';
    span.append(...children);
    return span;
  };

  const headings = [...container.querySelectorAll('h1')];
  const find = (name: string) => headings.find((h) => h.textContent?.trim().toLowerCase() === name.toLowerCase());
  const carpetas = find('Carpetas');
  const archivos = find('Archivos') ?? find('Conceptos');
  if (carpetas) carpetas.appendChild(wrap(folderButton()));
  if (archivos) archivos.appendChild(wrap(newSelect()));
  const missing: HTMLElement[] = [];
  if (!archivos) missing.push(newSelect());
  if (!carpetas) missing.push(folderButton());
  if (missing.length) {
    const bar = wrap(...missing);
    bar.classList.add('folder-index-top');
    container.prepend(bar);
  }
}
