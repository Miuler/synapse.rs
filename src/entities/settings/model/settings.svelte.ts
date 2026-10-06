import { invokeTauri, isTauriEnvironment } from '@shared/repositories/tauri';

export type MermaidRendererType = 'mermaidjs' | 'merman';
export type ThemeMode = 'system' | 'light' | 'dark';
export type ResolvedTheme = 'light' | 'dark';

export interface AppSettings {
  mermaidRenderer: MermaidRendererType;
  lastOpenedFolder?: string;
  theme: ThemeMode;
}

const STORAGE_KEY = 'synapse_settings';

export const DEFAULT_SETTINGS: AppSettings = {
  mermaidRenderer: 'mermaidjs', // Por defecto el visor es Mermaid.js
  lastOpenedFolder: undefined,
  theme: 'system', // Por defecto sigue el tema del sistema operativo
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
      if (parsed.theme === 'system' || parsed.theme === 'light' || parsed.theme === 'dark') {
        settings.theme = parsed.theme;
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
  systemTheme = $state<ResolvedTheme>('light');

  constructor() {
    if (typeof window !== 'undefined') {
      if (window.matchMedia) {
        const mq = window.matchMedia('(prefers-color-scheme: dark)');
        this.systemTheme = mq.matches ? 'dark' : 'light';
        mq.addEventListener('change', (e) => {
          this.systemTheme = e.matches ? 'dark' : 'light';
          this.applyTheme();
        });
      }

      // Consulta al backend en Linux/Tauri (Hyprland / Hyde / XDG Desktop Portal / gsettings)
      this.checkSystemTheme();

      // Al recuperar el foco de la ventana (p.ej. al cambiar de tema en terminal o Hyde)
      window.addEventListener('focus', () => {
        if (this.settings.theme === 'system') {
          this.checkSystemTheme();
        }
      });

      // Polling ligero cada 3s para sincronización en tiempo real
      window.setInterval(() => {
        if (this.settings.theme === 'system') {
          this.checkSystemTheme();
        }
      }, 3000);
    }
    this.applyTheme();
  }

  async checkSystemTheme(): Promise<void> {
    if (isTauriEnvironment()) {
      try {
        const theme = await invokeTauri<string>('get_system_theme');
        if (theme === 'dark' || theme === 'light') {
          if (this.systemTheme !== theme) {
            this.systemTheme = theme;
            this.applyTheme();
          }
          return;
        }
      } catch (e) {
        // Fallback al navegador
      }
    }

    if (typeof window !== 'undefined' && window.matchMedia) {
      const mq = window.matchMedia('(prefers-color-scheme: dark)');
      const theme = mq.matches ? 'dark' : 'light';
      if (this.systemTheme !== theme) {
        this.systemTheme = theme;
        this.applyTheme();
      }
    }
  }

  get theme(): ThemeMode {
    return this.settings.theme;
  }

  get resolvedTheme(): ResolvedTheme {
    return this.settings.theme === 'system' ? this.systemTheme : this.settings.theme;
  }

  setTheme(theme: ThemeMode) {
    this.settings.theme = theme;
    this.persist();
    if (theme === 'system') {
      this.checkSystemTheme();
    }
    this.applyTheme();
  }

  toggleTheme() {
    const nextTheme: ThemeMode = this.resolvedTheme === 'dark' ? 'light' : 'dark';
    this.setTheme(nextTheme);
  }

  cycleTheme() {
    if (this.settings.theme === 'system') {
      this.setTheme('dark');
    } else if (this.settings.theme === 'dark') {
      this.setTheme('light');
    } else {
      this.setTheme('system');
    }
  }

  applyTheme() {
    if (typeof document === 'undefined') return;
    const resolved = this.resolvedTheme;
    document.documentElement.setAttribute('data-theme', resolved);
    document.documentElement.style.colorScheme = resolved;
  }

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
