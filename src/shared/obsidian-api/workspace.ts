import type { VaultRepository } from '@shared/repositories';
import type { SynapseVaultLike, SynapseWorkspaceLike, TFileLike } from './types';

export class SynapseWorkspace implements SynapseWorkspaceLike {
  constructor(
    private readonly repository: VaultRepository,
    private readonly vault: SynapseVaultLike
  ) {}

  /**
   * Obtiene el archivo actualmente activo en las pestañas del espacio de trabajo.
   */
  async getActiveFile(): Promise<TFileLike | null> {
    const state = await this.repository.getOpenTabsState();
    const activePath = state?.active_tab;
    if (!activePath || activePath.startsWith('empty://')) return null;

    const file = await this.vault.getAbstractFileByPath(activePath);
    if (file) return file;

    const name = activePath.split('/').pop() || activePath;
    const dot = name.lastIndexOf('.');
    return {
      path: activePath,
      name,
      basename: dot > 0 ? name.slice(0, dot) : name,
      extension: dot > 0 ? name.slice(dot + 1) : '',
      parent: activePath.includes('/') ? activePath.slice(0, activePath.lastIndexOf('/')) : undefined,
      stat: { ctime: null, mtime: Date.now(), size: 0 },
      vault: this.vault,
    };
  }

  /**
   * Resuelve y abre un archivo a partir de un texto de enlace (wikilink o ruta relativa).
   */
  async openLinkText(linktext: string, sourcePath: string): Promise<TFileLike | null> {
    const resolvedPath = await this.repository.resolveVaultLink(linktext, sourcePath);
    const target = resolvedPath || linktext.replace(/^\[\[(.*)\]\]$/, '$1').trim();
    return this.vault.getAbstractFileByPath(target);
  }
}
