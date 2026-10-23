import { Events } from './events';
import { createDiv, empty } from './dom';
import type { TFile } from './types';
import type { View } from './view';
import type { SynapseApp } from './app';

export interface ViewState {
  type: string;
  state?: any;
  active?: boolean;
}

/**
 * Representa una pestaña o panel de trabajo individual (WorkspaceLeaf) en Obsidian.
 * Contiene y gestiona el ciclo de vida de una View montada.
 */
export class WorkspaceLeaf extends Events {
  readonly app: SynapseApp;
  view: View | null = null;
  readonly containerEl: HTMLElement;
  readonly tabHeaderEl: HTMLElement;
  readonly tabHeaderInnerTitleEl: HTMLElement;
  parent: any = null;
  width = 0;
  pinned = false;

  constructor(app: SynapseApp) {
    super();
    this.app = app;
    this.containerEl = createDiv({ cls: 'workspace-leaf' });
    this.tabHeaderEl = createDiv({ cls: 'workspace-tab-header' });
    this.tabHeaderInnerTitleEl = createDiv({ cls: 'workspace-tab-header-inner-title', parent: this.tabHeaderEl });
  }

  /**
   * Abre un archivo en esta hoja de trabajo.
   * Determina automáticamente el tipo de vista registrado para la extensión del archivo,
   * inicializa la vista y carga el archivo invocando su ciclo de vida `onLoadFile`.
   */
  async openFile(file: TFile, _openState?: any): Promise<void> {
    const ext = file.extension.toLowerCase();
    const targetViewType = this.app.workspace.getViewTypeForExtension(ext);

    // Si no tenemos vista o es de distinto tipo, cambiar el estado de la hoja
    if (!this.view || this.view.getViewType() !== targetViewType) {
      await this.setViewState({ type: targetViewType, active: true });
    }

    // Si la vista maneja archivos de texto (TextFileView o con onLoadFile), cargar el contenido
    if (this.view && typeof (this.view as any).onLoadFile === 'function') {
      await (this.view as any).onLoadFile(file);
    }

    // Actualizar título de la pestaña
    if (this.view) {
      this.tabHeaderInnerTitleEl.textContent = this.view.getDisplayText();
    }

    // Notificar al espacio de trabajo
    this.app.workspace.trigger('file-open', file);
    this.app.workspace.trigger('active-leaf-change', this);
  }

  /**
   * Cambia el tipo de vista o estado montado en esta hoja.
   */
  async setViewState(viewState: ViewState): Promise<void> {
    // Si ya había una vista montada, descargarla y limpiar su contenedor
    if (this.view) {
      this.view.unload();
      empty(this.containerEl);
      this.view = null;
    }

    const creator = this.app.workspace.getViewCreator(viewState.type);
    if (!creator) {
      throw new Error(`[WorkspaceLeaf] No hay creador de vista registrado para el tipo: ${viewState.type}`);
    }

    const view = creator(this);
    this.view = view;
    this.containerEl.appendChild(view.containerEl);

    // Ciclo de vida: cargar e inicializar vista
    view.load();
    await view.onOpen();

    if (viewState.state) {
      await view.setState(viewState.state);
    }

    this.tabHeaderInnerTitleEl.textContent = view.getDisplayText();
    this.app.workspace.trigger('layout-change');

    if (viewState.active) {
      this.app.workspace.setActiveLeaf(this);
    }
  }

  /**
   * Obtiene la configuración de estado actual de la vista en la hoja.
   */
  getViewState(): ViewState {
    return {
      type: this.view ? this.view.getViewType() : 'empty',
      state: this.view ? this.view.getState() : {},
    };
  }

  /**
   * Retorna el texto representativo de la vista actual para la pestaña.
   */
  getDisplayText(): string {
    return this.view ? this.view.getDisplayText() : 'Empty';
  }

  /**
   * Retorna la raíz o contenedor padre.
   */
  getRoot(): any {
    return this.parent || this;
  }

  /**
   * Cierra y desacopla la hoja de trabajo, descargando su vista y liberando el DOM.
   */
  detach(): void {
    if (this.view) {
      this.view.unload();
      this.view = null;
    }
    this.containerEl.remove();
    this.tabHeaderEl.remove();
    this.app.workspace.removeLeaf(this);
  }

  /**
   * Fija o desfija la pestaña para evitar cierre accidental.
   */
  setPinned(pinned: boolean): void {
    this.pinned = pinned;
  }
}
