import { TextFileView } from './view';
import type { WorkspaceLeaf } from './leaf';
import { createDiv, createEl, empty } from './dom';
import {
  parseExcalidrawContent,
  serializeExcalidrawMarkdown,
  extractWikilinksFromElements,
  type ExcalidrawScene,
} from './excalidraw-parser';
import { ExcalidrawAutomate } from './excalidraw-automate';

export const VIEW_TYPE_EXCALIDRAW = 'excalidraw';

/**
 * Vista de edición y visualización nativa de Excalidraw (ExcalidrawView).
 * Hereda de `TextFileView` para gestionar automáticamente la carga, persistencia
 * bidireccional y ciclo de vida de los diagramas, ya sea en formato JSON o Markdown con frontmatter.
 */
export class ExcalidrawView extends TextFileView {
  scene: ExcalidrawScene = {
    type: 'excalidraw',
    version: 2,
    source: 'https://github.com/zsviczian/obsidian-excalidraw-plugin',
    elements: [],
    appState: { viewBackgroundColor: '#ffffff' },
    files: {},
  };

  isMarkdown = true;
  viewMode: 'visual' | 'text' = 'visual';
  frontmatter: Record<string, any> = {};

  private readonly canvasContainerEl: HTMLElement;
  private readonly textContainerEl: HTMLTextAreaElement;
  private readonly statusNoticeEl: HTMLElement;

  constructor(leaf: WorkspaceLeaf) {
    super(leaf);
    if (typeof (this.containerEl as any).addClass === 'function') {
      (this.containerEl as any).addClass('excalidraw-view');
    } else {
      this.containerEl.classList?.add('excalidraw-view');
    }

    // Contenedor visual del dibujo
    this.canvasContainerEl = createDiv({
      cls: 'excalidraw-canvas-container',
      parent: this.contentEl,
    });
    this.canvasContainerEl.style.width = '100%';
    this.canvasContainerEl.style.height = '100%';
    this.canvasContainerEl.style.display = 'flex';
    this.canvasContainerEl.style.flexDirection = 'column';

    // Contenedor de edición en modo texto (raw markdown / json)
    this.textContainerEl = createEl('textarea', {
      cls: 'excalidraw-raw-textarea',
      parent: this.contentEl,
    }) as HTMLTextAreaElement;
    this.textContainerEl.style.width = '100%';
    this.textContainerEl.style.height = '100%';
    this.textContainerEl.style.fontFamily = 'monospace';
    this.textContainerEl.style.display = 'none';

    this.textContainerEl.addEventListener('input', () => {
      this.data = this.textContainerEl.value;
      this.requestSave();
    });

    // Barra de estado informativa
    this.statusNoticeEl = createDiv({
      cls: 'excalidraw-status-bar',
      parent: this.contentEl,
    });
    this.statusNoticeEl.style.fontSize = '12px';
    this.statusNoticeEl.style.padding = '4px 8px';
    this.statusNoticeEl.style.opacity = '0.7';

    // Registrar acciones en la cabecera de la pestaña
    this.addAction('file-code', 'Conmutar entre lienzo visual y código Markdown', () => {
      this.toggleViewMode();
    });

    this.addAction('image', 'Exportar a SVG transparente', async () => {
      const svg = this.exportSvg();
      if (this.file) {
        const svgPath = this.file.path.replace(/\.(excalidraw\.md|md|excalidraw)$/i, '.svg');
        await this.app.vault.create(svgPath, svg);
      }
    });
  }

  getViewType(): string {
    return VIEW_TYPE_EXCALIDRAW;
  }

  getDisplayText(): string {
    return this.file ? this.file.basename : 'Excalidraw';
  }

  /**
   * Serializa los datos actuales listos para guardar en disco según el formato del archivo.
   */
  getViewData(): string {
    if (this.viewMode === 'text') {
      return this.textContainerEl.value;
    }

    if (this.isMarkdown) {
      return serializeExcalidrawMarkdown(this.scene, this.data);
    }

    return JSON.stringify(this.scene, null, 2);
  }

  /**
   * Carga los datos crudos en la vista, parsea el documento y actualiza la representación visual.
   */
  setViewData(data: string, clear: boolean): void {
    if (clear) {
      this.clear();
    }

    this.data = data;
    const doc = parseExcalidrawContent(data);
    this.isMarkdown = doc.isMarkdown;
    this.scene = doc.scene;
    this.frontmatter = doc.frontmatter || {};

    this.textContainerEl.value = data;
    this.renderCanvas();
    this.updateStatus();
  }

  /**
   * Limpia el estado interno.
   */
  clear(): void {
    this.scene = {
      type: 'excalidraw',
      version: 2,
      source: 'https://github.com/zsviczian/obsidian-excalidraw-plugin',
      elements: [],
      appState: { viewBackgroundColor: '#ffffff' },
      files: {},
    };
    this.frontmatter = {};
    empty(this.canvasContainerEl);
    this.textContainerEl.value = '';
  }

  /**
   * Conmuta fluidamente entre modo visual de dibujo y modo texto (raw markdown/JSON).
   */
  toggleViewMode(): 'visual' | 'text' {
    if (this.viewMode === 'visual') {
      // Pasar a modo texto
      this.textContainerEl.value = this.getViewData();
      this.canvasContainerEl.style.display = 'none';
      this.textContainerEl.style.display = 'block';
      this.viewMode = 'text';
    } else {
      // Volver a modo visual: parsear el texto editado
      const rawText = this.textContainerEl.value;
      const doc = parseExcalidrawContent(rawText);
      this.scene = doc.scene;
      this.isMarkdown = doc.isMarkdown;
      this.frontmatter = doc.frontmatter || {};
      this.renderCanvas();

      this.textContainerEl.style.display = 'none';
      this.canvasContainerEl.style.display = 'flex';
      this.viewMode = 'visual';
    }

    this.updateStatus();
    return this.viewMode;
  }

  /**
   * Retorna la escena completa con sus elementos, appState y files.
   */
  getScene(): ExcalidrawScene {
    return this.scene;
  }

  /**
   * Actualiza la escena y solicita guardado automático.
   */
  updateScene(partial: Partial<ExcalidrawScene>): void {
    this.scene = {
      ...this.scene,
      ...partial,
      elements: partial.elements || this.scene.elements,
      appState: { ...this.scene.appState, ...(partial.appState || {}) },
      files: { ...this.scene.files, ...(partial.files || {}) },
    };
    this.renderCanvas();
    this.requestSave();
  }

  /**
   * Extrae todos los wikilinks presentes en los elementos de texto del dibujo.
   */
  getWikilinks(): string[] {
    return extractWikilinksFromElements(this.scene.elements);
  }

  /**
   * Genera una representación SVG vectorial transparente del lienzo actual.
   */
  exportSvg(): string {
    const ea = new ExcalidrawAutomate(this.app);
    return ea.toSVG(this.scene.elements);
  }

  /**
   * Genera un búfer binario PNG del lienzo actual.
   */
  async exportPng(): Promise<ArrayBuffer> {
    const ea = new ExcalidrawAutomate(this.app);
    return await ea.toPNG(this.scene.elements);
  }

  private renderCanvas(): void {
    empty(this.canvasContainerEl);

    // Renderizar representación SVG dentro del contenedor del canvas
    const ea = new ExcalidrawAutomate(this.app);
    const svgContent = ea.toSVG(this.scene.elements);

    const svgWrapper = createDiv({ cls: 'excalidraw-svg-stage', parent: this.canvasContainerEl });
    svgWrapper.innerHTML = svgContent;
    svgWrapper.style.width = '100%';
    svgWrapper.style.height = '100%';
    svgWrapper.style.display = 'flex';
    svgWrapper.style.alignItems = 'center';
    svgWrapper.style.justifyContent = 'center';
  }

  private updateStatus(): void {
    const elemCount = (this.scene.elements || []).filter((e) => !e.isDeleted).length;
    const links = this.getWikilinks();
    const linksText = links.length > 0 ? ` | Enlaces: ${links.join(', ')}` : '';
    this.statusNoticeEl.textContent = `Modo: ${this.viewMode.toUpperCase()} | Elementos: ${elemCount}${linksText}`;
  }
}
