import type { SynapseApp } from './app';

export interface DailyNotesPluginOptions {
  format: string;
  folder: string;
  template: string;
}

export interface InternalPluginInstance<T = any> {
  options: T;
  getFormat?(): string;
  [key: string]: any;
}

export interface InternalPluginEntry<T = any> {
  id: string;
  enabled: boolean;
  instance: InternalPluginInstance<T>;
  [key: string]: any;
}

/**
 * Gestor y registro de plugins internos / nativos de Obsidian (InternalPlugins).
 * Expuesto en `app.internalPlugins` para compatibilidad con plugins como Calendar,
 * Periodic Notes, obsidian-daily-notes-interface, etc.
 */
export class SynapseInternalPluginsManager {
  readonly app: SynapseApp;
  readonly plugins: Record<string, InternalPluginEntry> = {};

  constructor(app: SynapseApp) {
    this.app = app;
    this.registerDefaultCorePlugins();
  }

  private registerDefaultCorePlugins(): void {
    // 1. Core plugin: daily-notes
    const dailyNotesOptions: DailyNotesPluginOptions = {
      format: 'YYYY-MM-DD',
      folder: '',
      template: '',
    };

    const dailyNotesEntry: InternalPluginEntry<DailyNotesPluginOptions> = {
      id: 'daily-notes',
      enabled: true,
      instance: {
        options: dailyNotesOptions,
        getFormat: () => dailyNotesOptions.format,
      },
    };

    this.plugins['daily-notes'] = dailyNotesEntry;

    // 2. Core plugin: workspaces
    const workspacesEntry: InternalPluginEntry = {
      id: 'workspaces',
      enabled: true,
      instance: {
        options: {},
        activeWorkspace: 'default',
        getWorkspaces: () => ['default'],
        saveWorkspace: () => {},
        loadWorkspace: () => {},
      },
    };
    this.plugins['workspaces'] = workspacesEntry;

    // 3. Core plugin: file-recovery con protección contra recursión infinita
    const activeFileRecoveryPaths = new Set<string>();
    let currentOnFileChanged = (_file: any) => Promise.resolve();
    let currentForceAdd = (_path: any, _data: any) => Promise.resolve();

    const safeFileRecoveryInstance: InternalPluginInstance = {
      options: {},
      get onFileChanged() {
        return async (file: any) => {
          const path =
            file && typeof file.path === 'string'
              ? file.path
              : typeof file === 'string'
                ? file
                : '__synapse_anon_file__';
          if (activeFileRecoveryPaths.has(path)) {
            return;
          }
          activeFileRecoveryPaths.add(path);
          try {
            return await currentOnFileChanged(file);
          } finally {
            activeFileRecoveryPaths.delete(path);
          }
        };
      },
      set onFileChanged(fn: any) {
        if (typeof fn === 'function') {
          currentOnFileChanged = fn;
        }
      },
      get forceAdd() {
        return async (path: any, data: any) => {
          const key = typeof path === 'string' ? path : '__synapse_anon_file__';
          if (activeFileRecoveryPaths.has(key)) {
            return;
          }
          activeFileRecoveryPaths.add(key);
          try {
            return await currentForceAdd(path, data);
          } finally {
            activeFileRecoveryPaths.delete(key);
          }
        };
      },
      set forceAdd(fn: any) {
        if (typeof fn === 'function') {
          currentForceAdd = fn;
        }
      },
    };

    this.plugins['file-recovery'] = {
      id: 'file-recovery',
      enabled: true,
      instance: safeFileRecoveryInstance,
    };

    // 4. Stubs para otros core plugins habituales
    for (const id of ['backlink', 'outgoing-link', 'tag-pane', 'page-preview', 'graph']) {
      this.plugins[id] = {
        id,
        enabled: true,
        instance: {
          options: {},
        },
      };
    }

    // 5. Intentar cargar configuración guardada previamente desde .obsidian/daily-notes.json o .synapse/daily-notes.json
    this.loadDailyNotesSettingsAsync();
  }

  private async loadDailyNotesSettingsAsync(): Promise<void> {
    try {
      const candidates = ['.obsidian/daily-notes.json', '.synapse/daily-notes.json'];
      for (const path of candidates) {
        if (await this.app.vault.adapter.exists(path)) {
          const raw = await this.app.vault.adapter.read(path);
          const parsed = JSON.parse(raw);
          if (parsed && typeof parsed === 'object') {
            const current = this.plugins['daily-notes'].instance.options;
            if (typeof parsed.format === 'string' && parsed.format.trim()) {
              current.format = parsed.format.trim();
            }
            if (typeof parsed.folder === 'string') {
              current.folder = parsed.folder.trim();
            }
            if (typeof parsed.template === 'string') {
              current.template = parsed.template.trim();
            }
            break;
          }
        }
      }
    } catch (e) {
      // Ignorar fallos de lectura y mantener opciones por defecto válidas
    }
  }

  /**
   * Obtiene la entrada del plugin interno por su ID (ej. 'daily-notes').
   */
  getPluginById(id: string): InternalPluginEntry | undefined {
    return this.plugins[id];
  }

  /**
   * Obtiene la instancia activa del plugin interno si está habilitado.
   */
  getEnabledPluginById(id: string): InternalPluginInstance | null {
    const entry = this.plugins[id];
    if (entry && entry.enabled) {
      return entry.instance;
    }
    return null;
  }
}
