import { isDrawingFile } from '@entities/file-type';
import { isExcalidrawMarkdown } from '@shared/obsidian-api';

/** Copia de una pestaña en el mismo panel: `copy:<n>:<pestaña original>`. */
export function isTabCopy(p: string | null | undefined): boolean {
  return !!p && /^copy:\d+:/.test(p);
}

/** La pestaña original de una copia (o la misma, si no es copia). */
export function baseTabPath(p: string): string {
  let path = p;
  while (isTabCopy(path)) path = path.slice(path.indexOf(':', 'copy:'.length) + 1);
  return path;
}

/** Id para una copia nueva de `path` que no choque con las pestañas abiertas. */
export function newTabCopyId(path: string, open: string[]): string {
  const base = baseTabPath(path);
  let n = 2;
  while (open.includes(`copy:${n}:${base}`)) n++;
  return `copy:${n}:${base}`;
}

export function isSyntheticTab(p: string | null | undefined): boolean {
  if (!p) return false;
  return (
    isTabCopy(p) ||
    p.startsWith('empty:') ||
    p.startsWith('diff:') ||
    p.startsWith('diff-staged:') ||
    p.startsWith('diff-rev:') ||
    p.startsWith('diff-commit:') ||
    p.startsWith('diff-files:') ||
    p.startsWith('rev:') ||
    p.startsWith('synapse://')
  );
}

export function isDrawingTab(path: string | null | undefined, content?: string): boolean {
  if (!path || isSyntheticTab(path)) return false;
  if (isDrawingFile(path)) return true;
  if (content && isExcalidrawMarkdown(content)) return true;
  return false;
}

export function cleanTabTitle(rawTitle: string, path: string | null | undefined): string {
  if (!path) return rawTitle;
  const lowerPath = path.toLowerCase();
  if (lowerPath.endsWith('.excalidraw.md')) {
    return rawTitle.replace(/\.excalidraw\.md$/i, '').replace(/\.excalidraw$/i, '');
  }
  return rawTitle;
}

export function getTabDisplayTitle(path: string | null | undefined, fallbackTitle: string): string {
  if (!path) return fallbackTitle;
  if (path.startsWith('empty:')) return 'Nueva pestaña';
  if (path === 'synapse://update-notes') return 'Novedades de la nueva versión';
  if (path.startsWith('synapse://news')) return 'Novedades';
  if (path.startsWith('synapse://graph')) {
    const target = path.slice('synapse://graph'.length).replace(/^:/, '');
    if (target) {
      const name = target.split('/').pop() || target;
      return `Grafo: ${cleanTabTitle(name, target)}`;
    }
    return 'Vista de Grafo';
  }
  if (path.startsWith('diff-files:')) {
    const [a, b] = path.slice('diff-files:'.length).split('\n');
    return `Diff: ${a?.split('/').pop()} ↔ ${b?.split('/').pop()}`;
  }
  if (path.startsWith('diff-staged:')) {
    const f = path.slice('diff-staged:'.length).split('/').pop() || path;
    return `Diff (staged): ${f}`;
  }
  if (path.startsWith('diff-rev:')) {
    const parts = path.slice('diff-rev:'.length).split(':');
    const rev = parts[0] || '';
    const f = parts.slice(1).join(':').split('/').pop() || parts.slice(1).join(':');
    return `Diff (${rev.slice(0, 7)}): ${f}`;
  }
  if (path.startsWith('diff-commit:')) {
    const parts = path.slice('diff-commit:'.length).split(':');
    const rev = parts[0] || '';
    const f = parts.slice(1).join(':').split('/').pop() || parts.slice(1).join(':');
    const [from, to] = rev.split('..');
    if (to) return `Diff (${from.slice(0, 7)}…${to.slice(0, 7)}): ${f}`;
    return `Cambios (${rev.slice(0, 7)}): ${f}`;
  }
  if (path.startsWith('diff:')) {
    const f = path.slice('diff:'.length).split('/').pop() || path;
    return `Diff: ${f}`;
  }
  if (path.startsWith('rev:')) {
    const parts = path.slice('rev:'.length).split(':');
    const rev = parts[0] || '';
    const f = parts.slice(1).join(':').split('/').pop() || parts.slice(1).join(':');
    return `[${rev.slice(0, 7)}] ${f}`;
  }
  return cleanTabTitle(fallbackTitle, path);
}

export function parseTabDragData(e: DragEvent): { path: string; windowLabel: string; title?: string } | null {
  try {
    const rawCustom = e.dataTransfer?.getData('application/x-synapse-tab');
    if (rawCustom) return JSON.parse(rawCustom);
    const rawText = e.dataTransfer?.getData('text/plain');
    if (rawText && rawText.startsWith('synapse-tab:')) {
      return JSON.parse(rawText.slice('synapse-tab:'.length));
    }
  } catch {}
  return null;
}
