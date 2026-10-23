import type { VaultRepository } from '@shared/repositories';
import { Events } from './events';
import {
  TFile,
  type SynapseVaultLike,
  type SynapseWorkspaceLike,
  type WorkspaceLeaf,
} from './types';

export class SynapseWorkspace extends Events implements SynapseWorkspaceLike {
  constructor(
    private readonly repository: VaultRepository,
    private readonly vault: SynapseVaultLike
  ) {
    super();
  }

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
    const result = file instanceof TFile ? file : null;

    if (result) {
      this.trigger('file-open', result);
    }

    return result;
  }

  /**
   * Dispara el evento canónico 'file-open' a los plugins y observadores registrados.
   */
  triggerFileOpen(file: TFile | null): void {
    this.trigger('file-open', file);
  }

  /**
   * Dispara el evento canónico 'active-leaf-change' a los plugins y observadores registrados.
   */
  triggerActiveLeafChange(leaf: WorkspaceLeaf | null): void {
    this.trigger('active-leaf-change', leaf);
  }

  /**
   * Dispara el evento canónico 'layout-change' a los plugins y observadores registrados.
   */
  triggerLayoutChange(): void {
    this.trigger('layout-change');
  }
}
