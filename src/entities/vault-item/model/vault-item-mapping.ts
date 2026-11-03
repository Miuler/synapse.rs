import type { VaultFile, VaultNote } from '@shared/repositories';
import type { VaultItem } from './types';

/** Elemento de la bóveda a partir de la lista que envía el backend. */
export function toVaultItem(n: VaultNote, index: number): VaultItem {
  let relPath = `${n.title}.md`;
  if (typeof n.relative_path === 'string') {
    relPath = n.relative_path;
  } else if (n.relative_path && typeof n.relative_path === 'object' && (n.relative_path as unknown as string[])[0]) {
    relPath = (n.relative_path as unknown as string[])[0];
  }
  return {
    id: String(index + 1),
    title: n.title,
    relative_path: relPath,
    abs_path: n.abs_path,
    view_mode: n.view_mode,
    stat: typeof n.mtime === 'number' ? { ctime: n.ctime ?? null, mtime: n.mtime, size: n.size ?? 0 } : undefined,
  };
}

/** Archivo en el formato que usa la fachada de Obsidian (`TFile`). */
export function toVaultFile(item: VaultItem): VaultFile {
  const path = item.relative_path;
  const name = path.split('/').pop() ?? path;
  const dot = name.lastIndexOf('.');
  const slash = path.lastIndexOf('/');
  return {
    path,
    name,
    basename: dot > 0 ? name.slice(0, dot) : name,
    extension: dot > 0 ? name.slice(dot + 1) : '',
    parent: slash >= 0 ? path.slice(0, slash) : undefined,
    stat: item.stat ?? { ctime: null, mtime: 0, size: 0 },
    view_mode: item.view_mode,
  };
}
