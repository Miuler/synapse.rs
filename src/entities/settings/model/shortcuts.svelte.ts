import { invokeTauri, isTauriEnvironment } from '@shared/repositories/tauri';

export interface WelcomeShortcutsConfig {
  new_workspace: string;
  open_folder: string;
  clone_git: string;
  search_workspaces: string;
}

export interface WorkspaceShortcutsConfig {
  quick_open: string;
  command_palette: string;
  new_file: string;
  new_tab: string;
  save_file: string;
  close_tab: string;
  split_view: string;
  toggle_sidebar: string;
  locate_active_file: string;
  search_fulltext: string;
  open_vault_folder: string;
  reload_active_file: string;
  reload_vault: string;
  nav_back: string;
  nav_forward: string;
  toggle_theme: string;
  toggle_devtools: string;
  toggle_edit_mode: string;
  next_tab: string;
  prev_tab: string;
  git_push: string;
}

export interface ShortcutsConfig {
  welcome: WelcomeShortcutsConfig;
  workspace: WorkspaceShortcutsConfig;
}

export const DEFAULT_SHORTCUTS_CONFIG: ShortcutsConfig = {
  welcome: {
    new_workspace: 'Ctrl+Shift+N',
    open_folder: 'Ctrl+Shift+O',
    clone_git: 'Ctrl+Shift+G',
    search_workspaces: 'Ctrl+F',
  },
  workspace: {
    quick_open: 'Ctrl+O',
    command_palette: 'Ctrl+P',
    new_file: 'Ctrl+N',
    new_tab: 'Ctrl+T',
    save_file: 'Ctrl+S',
    close_tab: 'Ctrl+W, Ctrl+F4',
    split_view: 'Ctrl+\\',
    toggle_sidebar: 'Alt+1, Ctrl+B',
    locate_active_file: 'Ctrl+Alt+1, Alt+L',
    search_fulltext: 'Ctrl+Shift+F',
    open_vault_folder: 'Ctrl+Shift+O',
    reload_active_file: 'Ctrl+R',
    reload_vault: 'Ctrl+Shift+R',
    nav_back: 'Alt+ArrowLeft',
    nav_forward: 'Alt+ArrowRight',
    toggle_theme: 'Ctrl+Shift+T',
    toggle_devtools: 'F12, Ctrl+Shift+I',
    toggle_edit_mode: 'Ctrl+E',
    next_tab: 'Ctrl+PageDown, Ctrl+Tab',
    prev_tab: 'Ctrl+PageUp, Ctrl+Shift+Tab',
    git_push: 'Ctrl+Shift+K',
  },
};

interface ParsedCombo {
  wantCtrl: boolean;
  wantAlt: boolean;
  wantShift: boolean;
  targetKey: string;
}

const comboCache = new Map<string, ParsedCombo[]>();

function parseCombo(comboStr: string): ParsedCombo | null {
  const parts = comboStr
    .split('+')
    .map((p) => p.trim())
    .filter(Boolean);
  if (parts.length === 0) return null;

  let wantCtrl = false;
  let wantAlt = false;
  let wantShift = false;
  let targetKey = '';

  for (const part of parts) {
    const lower = part.toLowerCase();
    if (lower === 'ctrl' || lower === 'cmd' || lower === 'meta' || lower === 'control') {
      wantCtrl = true;
    } else if (lower === 'alt' || lower === 'option') {
      wantAlt = true;
    } else if (lower === 'shift' || lower === 'mayús' || lower === 'mayus') {
      wantShift = true;
    } else {
      targetKey = lower;
    }
  }

  if (!targetKey) return null;

  return { wantCtrl, wantAlt, wantShift, targetKey };
}

function getParsedCombos(spec: string): ParsedCombo[] {
  let cached = comboCache.get(spec);
  if (!cached) {
    cached = spec
      .split(',')
      .map((c) => c.trim())
      .filter(Boolean)
      .map(parseCombo)
      .filter((c): c is ParsedCombo => c !== null);
    comboCache.set(spec, cached);
  }
  return cached;
}

function matchesSingleCombo(e: KeyboardEvent, combo: ParsedCombo): boolean {
  const hasCtrl = e.ctrlKey || e.metaKey;
  if (hasCtrl !== combo.wantCtrl) return false;
  if (e.altKey !== combo.wantAlt) return false;
  if (e.shiftKey !== combo.wantShift) return false;

  const targetKey = combo.targetKey;
  const eventKey = e.key.toLowerCase();
  const eventCode = (e.code || '').toLowerCase();

  // Normalizar flechas
  if (targetKey === 'left' || targetKey === 'arrowleft') {
    return eventKey === 'arrowleft';
  }
  if (targetKey === 'right' || targetKey === 'arrowright') {
    return eventKey === 'arrowright';
  }
  if (targetKey === 'up' || targetKey === 'arrowup') {
    return eventKey === 'arrowup';
  }
  if (targetKey === 'down' || targetKey === 'arrowdown') {
    return eventKey === 'arrowdown';
  }

  // Normalizar PageUp / PageDown (ej. Ctrl+PageUp, Ctrl+PgUp, Ctrl+PageDown, Ctrl+PgDn)
  if (targetKey === 'pageup' || targetKey === 'pgup' || targetKey === 'prior') {
    return eventKey === 'pageup' || eventCode === 'pageup';
  }
  if (targetKey === 'pagedown' || targetKey === 'pgdn' || targetKey === 'next') {
    return eventKey === 'pagedown' || eventCode === 'pagedown';
  }

  // Normalizar dígitos (ej. Alt+1 o Ctrl+Alt+1 donde Shift o teclado puede alterar e.key)
  if (/^\d$/.test(targetKey)) {
    return (
      eventKey === targetKey ||
      eventCode === `digit${targetKey}` ||
      eventCode === `numpad${targetKey}`
    );
  }

  // Normalizar Tab / Backtab (en Linux/WebKit/X11, Shift+Tab genera e.key = "Backtab")
  if (targetKey === 'tab') {
    return eventKey === 'tab' || eventKey === 'backtab' || eventCode === 'tab';
  }

  // Normalizar barra invertida (Ctrl+\ o Ctrl+|)
  if (targetKey === '\\' || targetKey === 'backslash') {
    return eventKey === '\\' || eventKey === '|' || eventCode === 'backslash';
  }

  return eventKey === targetKey;
}

/**
 * Evalúa si un `KeyboardEvent` coincide con una definición de atajo (que puede contener
 * múltiples combinaciones separadas por coma, ej. `"Alt+1, Ctrl+B"`).
 */
export function matchesShortcut(e: KeyboardEvent, shortcutSpec: string | undefined | null): boolean {
  if (!shortcutSpec) return false;
  const combos = getParsedCombos(shortcutSpec);
  return combos.some((combo) => matchesSingleCombo(e, combo));
}

class ShortcutsConfigManager {
  config = $state<ShortcutsConfig>({
    welcome: { ...DEFAULT_SHORTCUTS_CONFIG.welcome },
    workspace: { ...DEFAULT_SHORTCUTS_CONFIG.workspace },
  });
  private loadPromise: Promise<void> | null = null;

  get welcome(): WelcomeShortcutsConfig {
    return this.config.welcome;
  }

  get workspace(): WorkspaceShortcutsConfig {
    return this.config.workspace;
  }

  async loadFromBackend(force = false): Promise<void> {
    if (!isTauriEnvironment()) return;
    if (this.loadPromise && !force) {
      return this.loadPromise;
    }
    this.loadPromise = (async () => {
      try {
        const loaded = await invokeTauri<ShortcutsConfig>('get_shortcuts_config');
        if (loaded && loaded.welcome && loaded.workspace) {
          this.config = {
            welcome: { ...DEFAULT_SHORTCUTS_CONFIG.welcome, ...loaded.welcome },
            workspace: { ...DEFAULT_SHORTCUTS_CONFIG.workspace, ...loaded.workspace },
          };
        }
      } catch (err) {
        console.warn('No se pudo cargar shortcuts.jsonc desde Rust, usando valores por defecto:', err);
      }
    })();
    return this.loadPromise;
  }
}

export const shortcutsConfig = new ShortcutsConfigManager();
