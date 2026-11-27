// Botones y vista tipo explorador de un index.md OKF (índice de carpeta): solo visuales al
// renderizar, sin alterar el archivo físico estándar OKF en disco.
import { mount, unmount } from 'svelte';
import { Lock } from 'lucide-svelte';
import { FolderIcon } from '@shared/ui/icons';
import { t } from '@shared/lib/i18n';
import { FileIcon } from '@entities/file-type';
import { parseGitStatusInfo, GitStatusBadge } from '@entities/git';
import type { GitFileStatusKind } from '@shared/repositories';

export type NewFileKind = 'markdown' | 'mermaid' | 'd2' | 'plantuml' | 'excalidraw' | 'obsidian-excalidraw' | 'board';

export const NEW_FILE_KINDS: { kind: NewFileKind; label: string }[] = [
  { kind: 'markdown', label: 'Nota Markdown' },
  { kind: 'mermaid', label: 'Diagrama Mermaid' },
  { kind: 'd2', label: 'Diagrama D2' },
  { kind: 'plantuml', label: 'Diagrama PlantUML' },
  { kind: 'excalidraw', label: 'Dibujo Excalidraw' },
  { kind: 'obsidian-excalidraw', label: 'Dibujo Excalidraw (Obsidian, .excalidraw.md)' },
  // Se traduce al leerlo: el idioma puede cambiar
  { kind: 'board', get label() { return t('board.newKind'); } },
];

export const FOLDER_INDEX_EVENT = 'synapse:folder-index-action';

export type FolderIndexAction = { folder: string } & ({ action: 'new-file'; kind: NewFileKind } | { action: 'new-folder' });

export interface DecorateFolderIndexOptions {
  gitStatuses?: Record<string, GitFileStatusKind>;
  lockedFiles?: Set<string>;
}

/** ¿Es el index.md generado por Synapse? */
export function isGeneratedFolderIndex(path: string | null | undefined, content: string): boolean {
  return !!path && /(^|\/)index\.md$/i.test(path) && content.includes('<!-- okf-index: generado por Synapse');
}

const INTERNAL_HREF_EXCLUDE = /^(?:https?:\/\/|mailto:|tel:|#)/i;
const GIT_ROW_CLASSES = ['git-modified', 'git-staged', 'git-untracked', 'git-stashed', 'is-locked'];

function directLinkOfRow(li: HTMLLIElement): HTMLAnchorElement | null {
  for (const child of li.childNodes) {
    if (child.nodeType === Node.TEXT_NODE) {
      if ((child.textContent ?? '').trim().length > 0) return null;
      continue;
    }
    if (child.nodeType === Node.ELEMENT_NODE) {
      const el = child as HTMLElement;
      if (el.tagName === 'A' && el.hasAttribute('href')) {
        const href = el.getAttribute('data-internal-path') || el.getAttribute('href') || '';
        return href && !INTERNAL_HREF_EXCLUDE.test(href) ? (el as HTMLAnchorElement) : null;
      }
      return null;
    }
  }
  return null;
}

/**
 * Decora un `index.md` generado por OKF:
 * 1) Añade «+ Nuevo…» junto a «Archivos»/«Conceptos» y «+ Carpeta» junto a «Carpetas».
 * 2) Estiliza las listas de enlaces como filas del explorador (icono de carpeta/archivo, estado Git y candado).
 * Devuelve una función de limpieza para desmontar los componentes Svelte montados dinámicamente.
 */
export function decorateFolderIndex(
  container: HTMLElement,
  folder: string,
  opts?: DecorateFolderIndexOptions,
): () => void {
  const mounted: Record<string, any>[] = [];

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

  // Render tipo VaultExplorer para las listas de carpetas, conceptos y archivos
  container.classList.add('okf-folder-index');
  for (const ul of container.querySelectorAll('ul')) {
    const items = [...ul.children].filter((el): el is HTMLLIElement => el.tagName === 'LI');
    if (items.length === 0) continue;

    // Limpiar decoraciones previas antes de verificar para ser idempotentes al actualizar Git/locks
    for (const li of items) {
      li.querySelectorAll(':scope > .okf-explorer-icon, :scope > .okf-explorer-badges').forEach((el) => el.remove());
      li.classList.remove(...GIT_ROW_CLASSES);
    }

    const rowAnchors = items.map((li) => directLinkOfRow(li));
    if (rowAnchors.some((a) => !a)) continue;

    ul.classList.add('okf-explorer-list');

    items.forEach((li, index) => {
      const anchor = rowAnchors[index]!;
      const rawHref = anchor.getAttribute('data-internal-path') || anchor.getAttribute('href') || '';
      const cleanHref = rawHref.split('#')[0];
      let decoded = cleanHref;
      try {
        decoded = decodeURIComponent(cleanHref);
      } catch {
        /* mantener sin decodificar si hay escapes inválidos */
      }
      const isFolder = decoded.endsWith('/');
      const rel = decoded.replace(/^\.\//, '').replace(/\/+$/, '');
      if (!rel || rel.startsWith('..')) return;

      const fullPath = folder ? `${folder}/${rel}` : rel;
      const label = anchor.textContent?.trim() || rel;
      const fileName = rel.split('/').pop() || label;

      li.classList.add('okf-explorer-row');
      li.classList.toggle('okf-explorer-folder', isFolder);

      // Envolver descripción / metadatos posteriores al enlace en .okf-explorer-meta (una sola vez)
      if (!li.querySelector(':scope > .okf-explorer-meta')) {
        const trailing: ChildNode[] = [];
        let node = anchor.nextSibling;
        while (node) {
          const next = node.nextSibling;
          trailing.push(node);
          node = next;
        }
        if (trailing.length > 0) {
          const meta = document.createElement('span');
          meta.className = 'okf-explorer-meta';
          for (const t of trailing) meta.appendChild(t);
          if (meta.firstChild?.nodeType === Node.TEXT_NODE) {
            meta.firstChild.textContent = (meta.firstChild.textContent ?? '').replace(/^\s*[-—–]\s*/, '');
          }
          if ((meta.textContent ?? '').trim().length > 0) {
            anchor.after(meta);
          }
        }
      }

      // Icono de carpeta o tipo de archivo al inicio de la fila
      const iconWrap = document.createElement('span');
      iconWrap.className = 'okf-explorer-icon';
      if (isFolder) {
        mounted.push(mount(FolderIcon, { target: iconWrap, props: { isOpen: false, name: fileName, size: 15 } }));
      } else {
        mounted.push(mount(FileIcon, { target: iconWrap, props: { path: fullPath, name: fileName, size: 15 } }));
      }
      li.prepend(iconWrap);

      // Estado Git y candado al final de la fila
      const rawGit = !isFolder
        ? (opts?.gitStatuses?.[fullPath] ?? opts?.gitStatuses?.[fullPath.replace(/^\.\//, '')])
        : undefined;
      const gitStatus = rawGit ? parseGitStatusInfo(rawGit) : null;
      const isLocked = !isFolder && !!opts?.lockedFiles?.has(fullPath);

      if (gitStatus?.worktree === 'M') li.classList.add('git-modified');
      else if (gitStatus?.index) li.classList.add('git-staged');
      else if (gitStatus?.worktree === '?') li.classList.add('git-untracked');
      else if (gitStatus?.is_stashed) li.classList.add('git-stashed');
      if (isLocked) li.classList.add('is-locked');

      if (isLocked || gitStatus) {
        const badges = document.createElement('span');
        badges.className = 'okf-explorer-badges';
        if (isLocked) {
          const lockWrap = document.createElement('span');
          lockWrap.className = 'okf-explorer-lock';
          lockWrap.title = 'Archivo bloqueado (auto-guardado desactivado)';
          mounted.push(mount(Lock, { target: lockWrap, props: { size: 12 } }));
          badges.appendChild(lockWrap);
        }
        if (gitStatus) {
          mounted.push(mount(GitStatusBadge, { target: badges, props: { status: gitStatus } }));
        }
        li.appendChild(badges);
      }

      if (!li.dataset.okfRowBound) {
        li.dataset.okfRowBound = '1';
        li.addEventListener('click', (e) => {
          const target = e.target as HTMLElement | null;
          if (!target || target.closest('a, button, select, input')) return;
          anchor.click();
        });
      }
    });
  }

  return () => {
    while (mounted.length) {
      const comp = mounted.pop();
      if (comp) {
        try {
          void unmount(comp);
        } catch {
          /* ya desmontado */
        }
      }
    }
  };
}
