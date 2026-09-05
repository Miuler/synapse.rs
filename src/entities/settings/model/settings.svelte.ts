export type MermaidRendererType = 'mermaidjs' | 'merman';

export interface AppSettings {
  mermaidRenderer: MermaidRendererType;
  lastOpenedFolder?: string;
}

const STORAGE_KEY = 'synapse_settings';

export const DEFAULT_SETTINGS: AppSettings = {
  mermaidRenderer: 'mermaidjs', // Por defecto el visor es Mermaid.js
  lastOpenedFolder: undefined,
};

function loadInitialSettings(): AppSettings {
  const settings: AppSettings = { ...DEFAULT_SETTINGS };
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const parsed = JSON.parse(stored);
      if (parsed.mermaidRenderer === 'merman' || parsed.mermaidRenderer === 'mermaidjs') {
        settings.mermaidRenderer = parsed.mermaidRenderer;
      }
      if (typeof parsed.lastOpenedFolder === 'string' && parsed.lastOpenedFolder.trim().length > 0) {
        settings.lastOpenedFolder = parsed.lastOpenedFolder.trim();
      }
    }
  } catch (e) {
    // Si localStorage no está disponible o falla el parseo, usar valores por defecto
  }

  // Por defecto el visor es Mermaid.js
  return settings;
}

class SettingsManager {
  settings = $state<AppSettings>(loadInitialSettings());

  get mermaidRenderer(): MermaidRendererType {
    return this.settings.mermaidRenderer;
  }

  setMermaidRenderer(renderer: MermaidRendererType) {
    this.settings.mermaidRenderer = renderer;
    this.persist();
  }

  toggleMermaidRenderer() {
    this.settings.mermaidRenderer =
      this.settings.mermaidRenderer === 'mermaidjs' ? 'merman' : 'mermaidjs';
    this.persist();
  }

  get lastOpenedFolder(): string | undefined {
    return this.settings.lastOpenedFolder;
  }

  setLastOpenedFolder(folder: string | undefined) {
    this.settings.lastOpenedFolder = folder;
    this.persist();
  }

  private persist() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(this.settings));
    } catch (e) {
      console.warn('No se pudo persistir la configuración en localStorage:', e);
    }
  }
}

export const appSettings = new SettingsManager();
