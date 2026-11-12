// Apariencia por bóveda: fragmentos CSS (`.synapse/snippets/*.css`) que se activan uno a uno y un
// tema (`.synapse/themes/*.css`). Lo elegido se guarda en `.synapse/appearance.json`, así cada
// bóveda tiene su propio aspecto. El CSS se inyecta en un único <style> al final del <head>, para
// que gane sobre los estilos de la app.
import { vaultRepository } from '@shared/repositories';
import { refreshThemeIcons } from '@shared/lib/theme-icons.svelte';
import papelCss from '../themes/papel.css?raw';
import neonCss from '../themes/neon.css?raw';
import islasCss from '../themes/islas.css?raw';
import rosaCss from '../themes/rosa.css?raw';

export const SNIPPETS_DIR = '.synapse/snippets';
export const THEMES_DIR = '.synapse/themes';
const CONFIG_PATH = '.synapse/appearance.json';
const STYLE_ID = 'synapse-custom-css';

/** Temas que vienen con la app. Un archivo de la bóveda con el mismo nombre los reemplaza. */
export const BUILTIN_THEMES: Record<string, { label: string; css: string }> = {
  islas: { label: 'Islas', css: islasCss },
  neon: { label: 'Neón', css: neonCss },
  'neon-islas': { label: 'Neón Islas', css: `${neonCss}\n${islasCss}` },
  papel: { label: 'Papel', css: papelCss },
  'papel-islas': { label: 'Papel Islas', css: `${papelCss}\n${islasCss}` },
  rosa: { label: 'Rosa', css: rosaCss },
  'rosa-islas': { label: 'Rosa Islas', css: `${rosaCss}\n${islasCss}` },
};

interface AppearanceConfig {
  theme: string | null;
  snippets: string[];
}

const cssFiles = async (dir: string) =>
  (await vaultRepository.listVaultDirectory(dir)).files
    .map((f) => f.split('/').pop() ?? f)
    .filter((f) => f.toLowerCase().endsWith('.css'))
    .map((f) => f.slice(0, -4))
    .sort((a, b) => a.localeCompare(b));

class Appearance {
  /** Fragmentos y temas disponibles (nombre sin `.css`). */
  snippets = $state<string[]>([]);
  themes = $state<string[]>([]);
  /** Temas que están como archivo en la bóveda (los demás son integrados). */
  vaultThemes = $state<string[]>([]);
  /** Lo elegido en esta bóveda. */
  enabledSnippets = $state<string[]>([]);
  theme = $state<string | null>(null);

  /** Lee lo disponible y lo elegido en la bóveda abierta y lo aplica. */
  async load() {
    const [snippets, themes, raw] = await Promise.all([
      cssFiles(SNIPPETS_DIR),
      cssFiles(THEMES_DIR),
      vaultRepository.readNote(CONFIG_PATH).catch(() => null),
    ]);
    let config: AppearanceConfig = { theme: null, snippets: [] };
    try {
      if (raw?.content) config = { ...config, ...JSON.parse(raw.content) };
    } catch {
      console.warn(`${CONFIG_PATH} no es JSON válido; se ignora`);
    }
    this.snippets = snippets;
    this.vaultThemes = themes;
    this.themes = [...new Set([...Object.keys(BUILTIN_THEMES), ...themes])].sort((a, b) => a.localeCompare(b));
    this.theme = config.theme && this.themes.includes(config.theme) ? config.theme : null;
    this.enabledSnippets = config.snippets.filter((s) => snippets.includes(s));
    await this.apply();
  }

  async setTheme(name: string | null) {
    this.theme = name;
    await this.saveAndApply();
  }

  async toggleSnippet(name: string, enabled: boolean) {
    this.enabledSnippets = enabled
      ? [...this.enabledSnippets.filter((s) => s !== name), name].sort((a, b) => a.localeCompare(b))
      : this.enabledSnippets.filter((s) => s !== name);
    await this.saveAndApply();
  }

  private async saveAndApply() {
    const config: AppearanceConfig = { theme: this.theme, snippets: this.enabledSnippets };
    await vaultRepository.saveNote({
      relativePath: CONFIG_PATH,
      title: 'appearance.json',
      content: JSON.stringify(config, null, 2) + '\n',
      encoding: 'UTF-8',
    });
    await this.apply();
  }

  /** Nombre visible: el integrado tiene etiqueta propia salvo que la bóveda lo reemplace. */
  themeLabel(name: string) {
    return !this.vaultThemes.includes(name) && BUILTIN_THEMES[name] ? BUILTIN_THEMES[name].label : name;
  }

  /** Inyecta el tema y después los fragmentos, para que un fragmento pueda retocar el tema. */
  private async apply() {
    const theme = this.theme;
    const builtin = theme && !this.vaultThemes.includes(theme) ? BUILTIN_THEMES[theme] : undefined;
    const paths = [
      ...(theme && !builtin ? [`${THEMES_DIR}/${theme}.css`] : []),
      ...this.enabledSnippets.map((s) => `${SNIPPETS_DIR}/${s}.css`),
    ];
    const parts = await Promise.all(
      paths.map(async (p) => {
        const note = await vaultRepository.readNote(p).catch(() => null);
        return note?.content ? `/* ${p} */\n${note.content}` : '';
      }),
    );
    if (builtin) parts.unshift(`/* tema integrado: ${theme} */\n${builtin.css}`);
    let style = document.getElementById(STYLE_ID) as HTMLStyleElement | null;
    if (!style) {
      style = document.createElement('style');
      style.id = STYLE_ID;
    }
    // Siempre al final del <head>, por si la app agregó estilos después
    document.head.appendChild(style);
    style.textContent = parts.filter(Boolean).join('\n\n');
    refreshThemeIcons();
  }
}

export const appearance = new Appearance();
