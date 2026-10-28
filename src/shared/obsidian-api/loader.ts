import { getObsidianBundle } from './bridge';
import type { SynapseApp } from './app';
import type { Plugin, PluginManifest } from './plugin';
import { createEl } from './dom';

export interface PluginEntry {
  manifest: PluginManifest;
  instance: Plugin;
  dir: string;
  styleEl?: HTMLElement;
  enabled: boolean;
}

/**
 * Cargador dinámico de plugins de Obsidian (PluginLoader).
 * Descubre, inyecta estilos CSS, evalúa código CommonJS/ESM empaquetado,
 * e inicia el ciclo de vida de los plugins de la comunidad.
 */
export class PluginLoader {
  readonly app: SynapseApp;
  readonly loadedPlugins = new Map<string, PluginEntry>();
  readonly plugins: Record<string, any> = {};

  constructor(app: SynapseApp) {
    this.app = app;
  }

  /**
   * Escanea la carpeta de plugins `.synapse/plugins`
   * en la bóveda actual buscando archivos `manifest.json`.
   */
  async scanPlugins(): Promise<Array<{ dir: string; manifest: PluginManifest }>> {
    const results: Array<{ dir: string; manifest: PluginManifest }> = [];
    const searchDirs = ['.synapse/plugins'];

    for (const baseDir of searchDirs) {
      const exists = await this.app.vault.adapter.exists(baseDir);
      if (!exists) continue;

      const listed = await this.app.vault.adapter.list(baseDir);
      for (const folder of listed.folders) {
        const manifestPath = `${folder}/manifest.json`;
        if (await this.app.vault.adapter.exists(manifestPath)) {
          try {
            const raw = await this.app.vault.adapter.read(manifestPath);
            const manifest: PluginManifest = JSON.parse(raw);
            if (manifest.id) {
              results.push({ dir: folder, manifest });
            }
          } catch (e) {
            console.warn(`[PluginLoader] Error al parsear ${manifestPath}:`, e);
          }
        }
      }
    }

    return results;
  }

  /**
   * Carga, inyecta y activa un plugin desde su carpeta en la bóveda.
   */
  async loadPluginFromDir(pluginDir: string): Promise<PluginEntry> {
    const cleanDir = pluginDir.replace(/^\/+|\/+$/g, '');
    const manifestPath = `${cleanDir}/manifest.json`;
    const mainJsPath = `${cleanDir}/main.js`;
    const stylesPath = `${cleanDir}/styles.css`;

    if (!(await this.app.vault.adapter.exists(manifestPath))) {
      throw new Error(`[PluginLoader] No se encontró manifest.json en ${cleanDir}`);
    }
    if (!(await this.app.vault.adapter.exists(mainJsPath))) {
      throw new Error(`[PluginLoader] No se encontró main.js en ${cleanDir}`);
    }

    const rawManifest = await this.app.vault.adapter.read(manifestPath);
    const manifest: PluginManifest = JSON.parse(rawManifest);

    // Si ya estaba cargado, descargarlo primero
    if (this.loadedPlugins.has(manifest.id)) {
      await this.unloadPlugin(manifest.id);
    }

    // 1. Inyectar estilos CSS si existen
    let styleEl: HTMLElement | undefined;
    if (await this.app.vault.adapter.exists(stylesPath)) {
      try {
        const css = await this.app.vault.adapter.read(stylesPath);
        styleEl = createEl('style', {
          text: css,
          attr: { id: `synapse-plugin-${manifest.id}` },
        });
        if (typeof document !== 'undefined' && document.head) {
          document.head.appendChild(styleEl);
        }
      } catch (e) {
        console.warn(`[PluginLoader] Error al inyectar estilos de ${manifest.id}:`, e);
      }
    }

    // 2. Leer y evaluar main.js con el shim del módulo 'obsidian'
    const code = await this.app.vault.adapter.read(mainJsPath);
    const obsidianBundle = getObsidianBundle(this.app);

    const targetGlobal: any = typeof window !== 'undefined' ? window : (globalThis as any);
    const customRequire = (moduleId: string) => {
      if (moduleId === 'obsidian') return obsidianBundle;
      if (moduleId === 'moment') return obsidianBundle.moment;
      if (targetGlobal && typeof targetGlobal.require === 'function') {
        return targetGlobal.require(moduleId);
      }
      throw new Error(`[Plugin ${manifest.id}] Cannot find module '${moduleId}'`);
    };

    const moduleObj = { exports: {} as any };
    const exportsObj = moduleObj.exports;

    try {
      // Wrapper de evaluación de módulo CommonJS estándar
      const factory = new Function('require', 'module', 'exports', code);
      factory(customRequire, moduleObj, exportsObj);
    } catch (err) {
      styleEl?.remove();
      throw new Error(`[PluginLoader] Error al evaluar main.js de ${manifest.id}: ${err}`);
    }

    const PluginClass = moduleObj.exports.default || moduleObj.exports;
    if (typeof PluginClass !== 'function') {
      styleEl?.remove();
      throw new Error(`[PluginLoader] El plugin ${manifest.id} no exportó una clase de plugin válida`);
    }

    // 3. Instanciar e iniciar ciclo de vida
    const instance: Plugin = new PluginClass(this.app, manifest);
    this.plugins[manifest.id] = instance;

    const entry: PluginEntry = {
      manifest,
      instance,
      dir: cleanDir,
      styleEl,
      enabled: true,
    };

    this.loadedPlugins.set(manifest.id, entry);

    await instance.load();

    return entry;
  }

  /**
   * Descarga y desactiva un plugin en ejecución, limpiando estilos y liberando recursos.
   */
  async unloadPlugin(pluginId: string): Promise<void> {
    const entry = this.loadedPlugins.get(pluginId);
    if (!entry) return;

    try {
      await entry.instance.unload();
    } catch (e) {
      console.error(`[PluginLoader] Error al descargar ${pluginId}:`, e);
    }

    delete this.plugins[pluginId];

    if (entry.styleEl) {
      entry.styleEl.remove();
    }

    this.loadedPlugins.delete(pluginId);
  }

  /**
   * Carga un plugin específico por su ID buscando en la carpeta de plugins de Synapse.
   */
  async loadPlugin(pluginId: string): Promise<PluginEntry> {
    const searchDirs = ['.synapse/plugins'];
    for (const baseDir of searchDirs) {
      const targetDir = `${baseDir}/${pluginId}`;
      const manifestPath = `${targetDir}/manifest.json`;
      if (await this.app.vault.adapter.exists(manifestPath)) {
        return this.loadPluginFromDir(targetDir);
      }
    }
    throw new Error(`[PluginLoader] No se encontró el plugin '${pluginId}' en .synapse/plugins`);
  }

  /**
   * Escanea y carga todos los plugins disponibles o habilitados en la bóveda actual.
   */
  async loadAllPlugins(): Promise<PluginEntry[]> {
    const discovered = await this.scanPlugins();
    const loaded: PluginEntry[] = [];
    for (const item of discovered) {
      try {
        const entry = await this.loadPluginFromDir(item.dir);
        loaded.push(entry);
        console.log(`[PluginLoader] Plugin cargado exitosamente: ${item.manifest.name} (${item.manifest.id})`);
      } catch (err) {
        console.warn(`[PluginLoader] Error al cargar plugin desde ${item.dir}:`, err);
      }
    }
    return loaded;
  }

  /**
   * Determina si un plugin está actualmente cargado y activo en memoria.
   */
  isPluginLoaded(pluginId: string): boolean {
    return this.loadedPlugins.has(pluginId);
  }

  /**
   * Obtiene la instancia activa de un plugin por su ID.
   */
  getPlugin(pluginId: string): Plugin | undefined {
    return this.loadedPlugins.get(pluginId)?.instance;
  }

  /**
   * Lista todos los plugins cargados actualmente.
   */
  getLoadedPlugins(): PluginEntry[] {
    return Array.from(this.loadedPlugins.values());
  }
}
