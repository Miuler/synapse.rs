import { Component } from './component';
import type { SynapseApp } from './app';
import type { Command } from './command';
import type { PluginSettingTab } from './setting';
import type { WorkspaceLeaf } from './leaf';
import type { View } from './view';

/**
 * Manifiesto canónico de un plugin de Obsidian (manifest.json).
 */
export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  minAppVersion?: string;
  description?: string;
  author?: string;
  authorUrl?: string;
  isDesktopOnly?: boolean;
  [key: string]: any;
}

/**
 * Clase base canónica Plugin de Obsidian.
 * Todo plugin de la comunidad (incluyendo `obsidian-excalidraw-plugin`) hereda de esta clase.
 */
export abstract class Plugin extends Component {
  readonly app: SynapseApp;
  readonly manifest: PluginManifest;

  constructor(app: SynapseApp, manifest: PluginManifest) {
    super();
    this.app = app;
    this.manifest = manifest;
  }

  /**
   * Carga los datos persistidos de configuración del plugin (`data.json`).
   * Busca prioritariamente en `.synapse/plugins/<id>/data.json` y de forma secundaria en `.obsidian/plugins/<id>/data.json`.
   */
  async loadData(): Promise<any> {
    const primaryPath = `.synapse/plugins/${this.manifest.id}/data.json`;
    const fallbackPath = `.obsidian/plugins/${this.manifest.id}/data.json`;

    if (await this.app.vault.adapter.exists(primaryPath)) {
      try {
        const text = await this.app.vault.adapter.read(primaryPath);
        return JSON.parse(text);
      } catch (e) {
        console.warn(`[Plugin ${this.manifest.id}] Error al leer data.json primario:`, e);
      }
    }

    if (await this.app.vault.adapter.exists(fallbackPath)) {
      try {
        const text = await this.app.vault.adapter.read(fallbackPath);
        return JSON.parse(text);
      } catch (e) {
        console.warn(`[Plugin ${this.manifest.id}] Error al leer data.json fallback:`, e);
      }
    }

    return null;
  }

  /**
   * Guarda los datos de configuración del plugin en `.synapse/plugins/<id>/data.json`.
   */
  async saveData(data: any): Promise<void> {
    const dir = `.synapse/plugins/${this.manifest.id}`;
    if (!(await this.app.vault.adapter.exists(dir))) {
      await this.app.vault.adapter.mkdir(dir);
    }
    const path = `${dir}/data.json`;
    const json = JSON.stringify(data, null, 2);
    await this.app.vault.adapter.write(path, json);
  }

  /**
   * Registra un comando en la paleta de comandos de Synapse.
   * Se desregistra automáticamente cuando el plugin se descarga.
   */
  addCommand(command: Command): Command {
    const commandId = command.id.includes(':') ? command.id : `${this.manifest.id}:${command.id}`;
    const fullCommand: Command = {
      ...command,
      id: commandId,
    };

    this.app.commands.addCommand(fullCommand);
    this.register(() => {
      this.app.commands.removeCommand(commandId);
    });

    return fullCommand;
  }

  /**
   * Registra una pestaña de configuración en el diálogo de ajustes de Synapse.
   * Se remueve automáticamente al descargar el plugin.
   */
  addSettingTab(settingTab: PluginSettingTab): void {
    this.app.setting.addSettingTab(settingTab);
    this.register(() => {
      this.app.setting.removeSettingTab(settingTab);
    });
  }

  /**
   * Añade un icono de acceso directo en la barra lateral Ribbon de Synapse.
   * Se remueve automáticamente al descargar el plugin.
   */
  addRibbonIcon(icon: string, title: string, callback: (evt: MouseEvent) => any): HTMLElement {
    const el = this.app.ribbon.addRibbonIcon(icon, title, callback);
    this.register(() => {
      this.app.ribbon.removeRibbonIcon(el);
    });
    return el;
  }

  /**
   * Añade un indicador o widget en la barra de estado inferior (StatusBar) de Synapse.
   * Se remueve automáticamente al descargar el plugin.
   */
  addStatusBarItem(): HTMLElement {
    const el = this.app.statusBar.addStatusBarItem();
    this.register(() => {
      this.app.statusBar.removeStatusBarItem(el);
    });
    return el;
  }

  /**
   * Registra un tipo de vista personalizado en el espacio de trabajo.
   */
  registerView(type: string, viewCreator: (leaf: WorkspaceLeaf) => View): void {
    this.app.workspace.registerView(type, viewCreator);
  }

  /**
   * Mapea extensiones de archivo hacia un tipo de vista registrado.
   */
  registerExtensions(extensions: string[], viewType: string): void {
    this.app.workspace.registerExtensions(extensions, viewType);
  }

  /**
   * Registra un post-procesador de Markdown que se ejecuta al renderizar bloques de lectura o embeds.
   */
  registerMarkdownPostProcessor(postProcessor: (el: HTMLElement, ctx: any) => any): any {
    return postProcessor;
  }

  /**
   * Registra un procesador de bloque de código Markdown (ej. ```excalidraw).
   */
  registerMarkdownCodeBlockProcessor(
    _language: string,
    handler: (source: string, el: HTMLElement, ctx: any) => any
  ): any {
    return handler;
  }
}
