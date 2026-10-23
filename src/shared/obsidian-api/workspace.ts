import type { VaultRepository } from '@shared/repositories';
import { Events } from './events';
import {
  TFile,
  type SynapseVaultLike,
  type SynapseWorkspaceLike,
} from './types';
import { WorkspaceLeaf } from './leaf';
import { DefaultMarkdownView, View } from './view';
import type { SynapseApp } from './app';

export class SynapseWorkspace extends Events implements SynapseWorkspaceLike {
  private appInstance?: SynapseApp;
  private readonly viewCreators: Map<string, (leaf: WorkspaceLeaf) => View> = new Map();
  private readonly extensionViewTypes: Map<string, string> = new Map();
  private readonly leaves: Set<WorkspaceLeaf> = new Set();
  activeLeaf: WorkspaceLeaf | null = null;

  constructor(
    private readonly repository: VaultRepository,
    private readonly vault: SynapseVaultLike
  ) {
    super();

    // Registrar vista por defecto de Markdown
    this.registerView('markdown', (leaf) => new DefaultMarkdownView(leaf));
  }

  /**
   * Conecta la instancia de SynapseApp al espacio de trabajo.
   */
  setApp(app: SynapseApp): void {
    this.appInstance = app;
  }

  get app(): SynapseApp {
    if (!this.appInstance) {
      throw new Error('[SynapseWorkspace] appInstance aún no ha sido conectada');
    }
    return this.appInstance;
  }

  /**
   * Registra una vista personalizada para un tipo específico (ej. VIEW_TYPE_EXCALIDRAW).
   * Los plugins como Excalidraw invocan este método durante `plugin.onload()`.
   */
  registerView(type: string, viewCreator: (leaf: WorkspaceLeaf) => View): void {
    this.viewCreators.set(type, viewCreator);
  }

  /**
   * Registra qué tipo de vista debe manejar una lista de extensiones de archivo.
   * (ej: `registerExtensions(['excalidraw', 'excalidraw.md'], 'excalidraw')`).
   */
  registerExtensions(extensions: string[], viewType: string): void {
    for (const ext of extensions) {
      const clean = ext.toLowerCase().replace(/^\./, '');
      this.extensionViewTypes.set(clean, viewType);
    }
  }

  /**
   * Retorna la función constructora registrada para un tipo de vista dado.
   */
  getViewCreator(type: string): ((leaf: WorkspaceLeaf) => View) | undefined {
    return this.viewCreators.get(type);
  }

  /**
   * Resuelve el viewType correspondiente a una extensión de archivo.
   * Si no hay vista registrada, devuelve 'markdown' por defecto.
   */
  getViewTypeForExtension(extension: string): string {
    const clean = extension.toLowerCase().replace(/^\./, '');
    return this.extensionViewTypes.get(clean) || 'markdown';
  }

  /**
   * Obtiene o crea una hoja de trabajo (WorkspaceLeaf).
   */
  getLeaf(newLeaf?: boolean | 'split' | 'tab' | 'window'): WorkspaceLeaf {
    if (!newLeaf && this.activeLeaf) {
      return this.activeLeaf;
    }

    const leaf = new WorkspaceLeaf(this.app);
    this.leaves.add(leaf);
    this.setActiveLeaf(leaf);
    return leaf;
  }

  /**
   * Crea una nueva hoja dividiendo un panel existente.
   */
  createLeafBySplit(_leaf: WorkspaceLeaf, _direction?: 'horizontal' | 'vertical', _before?: boolean): WorkspaceLeaf {
    return this.getLeaf(true);
  }

  /**
   * Establece la hoja de trabajo activa y emite 'active-leaf-change'.
   */
  setActiveLeaf(leaf: WorkspaceLeaf, _params?: { focus?: boolean }): void {
    this.activeLeaf = leaf;
    this.trigger('active-leaf-change', leaf);
    if (leaf.view && (leaf.view as any).file instanceof TFile) {
      this.trigger('file-open', (leaf.view as any).file);
    }
  }

  /**
   * Retorna todas las hojas de trabajo que tengan una vista del tipo especificado.
   */
  getLeavesOfType(type: string): WorkspaceLeaf[] {
    return Array.from(this.leaves).filter((leaf) => leaf.view?.getViewType() === type);
  }

  /**
   * Retorna la vista activa si coincide con la clase solicitada, o null en caso contrario.
   */
  getActiveViewOfType<T extends View>(type: new (...args: any[]) => T): T | null {
    if (this.activeLeaf?.view instanceof type) {
      return this.activeLeaf.view as T;
    }
    return null;
  }

  /**
   * Remueve una hoja de trabajo del registro activo del workspace.
   */
  removeLeaf(leaf: WorkspaceLeaf): void {
    this.leaves.delete(leaf);
    if (this.activeLeaf === leaf) {
      this.activeLeaf = this.leaves.values().next().value || null;
      this.trigger('active-leaf-change', this.activeLeaf);
    }
  }

  /**
   * Obtiene el archivo actualmente activo en las pestañas del espacio de trabajo.
   */
  async getActiveFile(): Promise<TFile | null> {
    if (this.activeLeaf?.view && (this.activeLeaf.view as any).file instanceof TFile) {
      return (this.activeLeaf.view as any).file;
    }

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
  async openLinkText(linktext: string, sourcePath: string, newLeaf?: boolean): Promise<TFile | null> {
    const resolvedPath = await this.repository.resolveVaultLink(linktext, sourcePath);
    const target = resolvedPath || linktext.replace(/^\[\[(.*)\]\]$/, '$1').trim();
    const file = await this.vault.getAbstractFileByPath(target);
    const result = file instanceof TFile ? file : null;

    if (result) {
      const leaf = this.getLeaf(newLeaf);
      await leaf.openFile(result);
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

  /**
   * Retorna una representación del diseño del workspace.
   */
  getLayout(): any {
    return {
      type: 'root',
      children: Array.from(this.leaves).map((l) => l.getViewState()),
    };
  }
}
