import type { VaultRepository } from '@shared/repositories';
import type { TFileLike } from './types';

function file(path: string): TFileLike {
  const name = path.split('/').pop() ?? path;
  const dot = name.lastIndexOf('.');
  return {
    path,
    name,
    basename: dot > 0 ? name.slice(0, dot) : name,
    extension: dot > 0 ? name.slice(dot + 1) : '',
    parent: path.includes('/') ? path.slice(0, path.lastIndexOf('/')) : undefined,
    stat: { ctime: 0, mtime: 0, size: 0 },
    vault: undefined as never,
  };
}

export class SynapseWorkspace {
  constructor(private readonly repository: VaultRepository) {}

  async getActiveFile(): Promise<TFileLike | null> {
    const state = await this.repository.getOpenTabsState();
    const path = state?.active_tab;
    return path ? file(path) : null;
  }

  async openLinkText(path: string): Promise<TFileLike | null> {
    const note = await this.repository.readNote(path);
    return note ? file(note.relative_path) : null;
  }
}
