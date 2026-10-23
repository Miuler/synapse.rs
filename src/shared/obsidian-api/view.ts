import { Component } from './component';
import { createDiv } from './dom';
import type { TFile } from './types';
import type { WorkspaceLeaf } from './leaf';
import type { SynapseApp } from './app';

/**
 * Clase base canónica View de Obsidian.
 * Representa una vista visual dentro de una hoja de trabajo (WorkspaceLeaf).
 */
export abstract class View extends Component {
  readonly app: SynapseApp;
  readonly leaf: WorkspaceLeaf;
  readonly containerEl: HTMLElement;
  icon = 'document';
  navigation = true;

  constructor(leaf: WorkspaceLeaf) {
    super();
    this.leaf = leaf;
    this.app = leaf.app;
    this.containerEl = createDiv({ cls: 'view-container' });
  }

  /** Tipo identificador único de la vista (ej: 'markdown', 'excalidraw', 'graph') */
  abstract getViewType(): string;

  /** Texto para mostrar en la pestaña de la hoja de trabajo */
  getDisplayText(): string {
    return 'View';
  }

  /** Icono de la pestaña */
  getIcon(): string {
    return this.icon;
  }

  /** Invocado cuando la vista se abre y monta en el DOM */
  async onOpen(): Promise<void> {}

  /** Invocado cuando la vista se cierra */
  async onClose(): Promise<void> {}

  /** Obtiene el estado serializable de la vista */
  getState(): any {
    return {};
  }

  /** Restaura el estado de la vista */
  async setState(_state: any, _result?: any): Promise<void> {}
}

/**
 * ItemView extiende View añadiendo un contenedor de contenido principal (contentEl)
 * y una barra de acciones de cabecera (actionsEl).
 */
export abstract class ItemView extends View {
  readonly contentEl: HTMLElement;
  readonly actionsEl: HTMLElement;

  constructor(leaf: WorkspaceLeaf) {
    super(leaf);
    this.actionsEl = createDiv({ cls: 'view-actions', parent: this.containerEl });
    this.contentEl = createDiv({ cls: 'view-content', parent: this.containerEl });
  }

  /**
   * Añade un botón de acción en la barra superior de la vista.
   */
  addAction(icon: string, title: string, callback: (evt: MouseEvent) => any): HTMLElement {
    const actionEl = createDiv({ cls: 'view-action clickable-icon', title, parent: this.actionsEl });
    actionEl.setAttribute('aria-label', title);
    this.registerDomEvent(actionEl, 'click', callback as any);
    return actionEl;
  }
}

/**
 * TextFileView es la clase base canónica para cualquier vista respaldada por un archivo
 * de texto en la bóveda (Markdown, Canvas, Excalidraw, etc.).
 * El plugin `obsidian-excalidraw-plugin` hereda directamente de esta clase.
 */
export abstract class TextFileView extends ItemView {
  file: TFile | null = null;
  data = '';

  constructor(leaf: WorkspaceLeaf) {
    super(leaf);
  }

  /** Retorna los datos serializados actuales de la vista listos para persistir */
  abstract getViewData(): string;

  /** Carga los datos de texto en el motor de renderizado de la vista */
  abstract setViewData(data: string, clear: boolean): void;

  /** Limpia el estado interno de la vista al descargar */
  abstract clear(): void;

  /**
   * Guarda los datos actuales en el archivo de la bóveda mediante app.vault.modify.
   */
  async save(clear?: boolean): Promise<void> {
    if (!this.file) return;
    const currentData = this.getViewData();
    this.data = currentData;
    await this.app.vault.modify(this.file, currentData);
    if (clear) {
      this.clear();
    }
  }

  /**
   * Carga el archivo en la vista, leyendo el contenido desde la bóveda e invocando setViewData.
   */
  async onLoadFile(file: TFile): Promise<void> {
    this.file = file;
    const content = await this.app.vault.read(file);
    this.data = content;
    this.setViewData(content, true);
  }

  /**
   * Descarga el archivo actual, guardando los cambios pendientes si los hubiera.
   */
  async onUnloadFile(file: TFile): Promise<void> {
    if (this.file && this.file.path === file.path && this.getViewData() !== this.data) {
      await this.save();
    }
    this.clear();
    this.file = null;
    this.data = '';
  }

  /** Comprueba si esta vista puede manejar una extensión dada */
  canAcceptExtension(_extension: string): boolean {
    return true;
  }

  /** Solicita el guardado asíncrono seguro */
  requestSave(): void {
    this.save().catch((err) => {
      console.error('[TextFileView] Error al guardar archivo:', err);
    });
  }

  override getDisplayText(): string {
    return this.file ? this.file.basename : 'Archivo';
  }
}

/**
 * Vista de Markdown por defecto utilizada cuando no hay una vista especializada registrada.
 */
export class DefaultMarkdownView extends TextFileView {
  getViewType(): string {
    return 'markdown';
  }

  getViewData(): string {
    return this.data;
  }

  setViewData(data: string, _clear: boolean): void {
    this.data = data;
    this.contentEl.textContent = data;
  }

  clear(): void {
    this.data = '';
    this.contentEl.textContent = '';
  }
}
