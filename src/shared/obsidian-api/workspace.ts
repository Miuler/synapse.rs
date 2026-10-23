import type { VaultRepository } from '@shared/repositories';
import { TFile, type SynapseVaultLike, type SynapseWorkspaceLike } from './types';

export class SynapseWorkspace implements SynapseWorkspaceLike {
  constructor(
    private readonly repository: VaultRepository,
    private readonly vault: SynapseVaultLike
  ) {}

  /**
   * Obtiene el archivo actualmente activo en las pestañas del espacio de trabajo.
   */
  async getActiveFile(): Promise<TFile | null> {
    const state = await this.repository.getOpenTabsState();
    const activePath = state?.active_tab;
    if (!activePath || activePath.startsWith('empty://')) return null;

    const file = await this.vault.getAbstractFileByPath(activePath);
    if (file instanceof TFile) return file;

    return new TFile(this.vault, activePath);
  }

  /**
   * Resuelve y abre un archivo a partir de un texto de enlace (wikilink o ruta relativa).
   */
  async openLinkText(linktext: string, sourcePath: string): Promise<TFile | null> {
    const resolvedPath = await this.repository.resolveVaultLink(linktext, sourcePath);
    const target = resolvedPath || linktext.replace(/^\[\[(.*)\]\]$/, '$1').trim();
    const file = await this.vault.getAbstractFileByPath(target);
    return file instanceof TFile ? file : null;
  }
}
