// Apariencia por bóveda: fragmentos CSS (`.synapse/snippets/*.css`) que se activan uno a uno y un
// tema (`.synapse/themes/*.css`). Lo elegido se guarda en `.synapse/appearance.json`, así cada
// bóveda tiene su propio aspecto. El CSS se inyecta en un único <style> al final del <head>, para
// que gane sobre los estilos de la app.
import { vaultRepository } from '@shared/repositories';
import { refreshThemeIcons } from '@shared/lib/theme-icons.svelte';

export const SNIPPETS_DIR = '.synapse/snippets';
export const THEMES_DIR = '.synapse/themes';
const CONFIG_PATH = '.synapse/appearance.json';
const STYLE_ID = 'synapse-custom-css';

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
    this.themes = themes;
    this.theme = config.theme && themes.includes(config.theme) ? config.theme : null;
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

  /** Inyecta el tema y después los fragmentos, para que un fragmento pueda retocar el tema. */
  private async apply() {
    const paths = [
      ...(this.theme ? [`${THEMES_DIR}/${this.theme}.css`] : []),
      ...this.enabledSnippets.map((s) => `${SNIPPETS_DIR}/${s}.css`),
    ];
    const parts = await Promise.all(
      paths.map(async (p) => {
        const note = await vaultRepository.readNote(p).catch(() => null);
        return note?.content ? `/* ${p} */\n${note.content}` : '';
      }),
    );
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
