import { isDrawingFile } from '@entities/file-type';
import { isExcalidrawMarkdown } from '@shared/obsidian-api';

export function isSyntheticTab(p: string | null | undefined): boolean {
  if (!p) return false;
  return (
    p.startsWith('empty:') ||
    p.startsWith('diff:') ||
    p.startsWith('diff-staged:') ||
    p.startsWith('diff-rev:') ||
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
  if (path.startsWith('synapse://graph')) {
    const target = path.slice('synapse://graph'.length).replace(/^:/, '');
    if (target) {
      const name = target.split('/').pop() || target;
      return `Grafo: ${cleanTabTitle(name, target)}`;
    }
    return 'Vista de Grafo';
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
