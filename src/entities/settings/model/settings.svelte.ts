export type MermaidRendererType = 'mermaidjs' | 'merman';

export interface AppSettings {
  mermaidRenderer: MermaidRendererType;
}

const STORAGE_KEY = 'synapse_settings';

export const DEFAULT_SETTINGS: AppSettings = {
  mermaidRenderer: 'mermaidjs', // Por defecto el visor es Mermaid.js
};

function loadInitialSettings(): AppSettings {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const parsed = JSON.parse(stored);
      if (parsed.mermaidRenderer === 'merman' || parsed.mermaidRenderer === 'mermaidjs') {
        return {
          mermaidRenderer: parsed.mermaidRenderer,
        };
      }
    }
  } catch (e) {
    // Si localStorage no está disponible o falla el parseo, usar valores por defecto
  }

  // Por defecto el visor es Mermaid.js
  return { ...DEFAULT_SETTINGS };
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

  private persist() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(this.settings));
    } catch (e) {
      console.warn('No se pudo persistir la configuración en localStorage:', e);
    }
  }
}

export const appSettings = new SettingsManager();
