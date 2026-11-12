// Apariencia por bóveda: fragmentos CSS (`.synapse/snippets/*.css`) que se activan uno a uno y un
// tema (`.synapse/themes/*.css`). Lo elegido se guarda en `.synapse/appearance.json`, así cada
// bóveda tiene su propio aspecto. El CSS se inyecta en un único <style> al final del <head>, para
// que gane sobre los estilos de la app.
import { vaultRepository } from '@shared/repositories';
import { invokeTauri } from '@shared/repositories/tauri';
import { refreshThemeIcons } from '@shared/lib/theme-icons.svelte';

export const SNIPPETS_DIR = '.synapse/snippets';
export const THEMES_DIR = '.synapse/themes';
const CONFIG_PATH = '.synapse/appearance.json';
const STYLE_ID = 'synapse-custom-css';

// Los temas salen de tres lugares; ante el mismo nombre gana el último:
//   1. Integrados: cada `.css` de `../themes/` entra al compilar, sin registrarlo aquí.
//   2. Globales: `.css` de `~/.config/synapsers/themes` (siempre) y de las carpetas extra de
//      `appearance.theme_dirs` del config.jsonc del usuario, leídos al iniciar y con «Reescanear».
//   3. De la bóveda: `.synapse/themes/*.css`.
// Metadatos en un comentario al inicio del archivo (integrados y globales):
//   @nombre Neón   nombre visible (si falta, se usa el del archivo)
//   @capa          no es un tema completo sino una capa que se combina con cada tema («Neón Islas»)
//   @base neon     parte de otro tema y le agrega lo suyo encima
const builtinFiles = Object.fromEntries(
  Object.entries(
    import.meta.glob('../themes/*.css', { query: '?raw', import: 'default', eager: true }) as Record<string, string>,
  ).map(([path, css]) => [(path.split('/').pop() ?? path).replace(/\.css$/i, ''), css]),
);

interface CatalogTheme {
  label: string;
  css: string;
}

interface GlobalTheme {
  id: string;
  css: string;
  dir: string;
}

/** Arma la lista de temas a partir de `id → CSS`, resolviendo `@nombre`, `@base` y `@capa`. */
function buildThemes(files: Record<string, string>): Record<string, CatalogTheme> {
  const themes: Record<string, CatalogTheme & { layer: boolean }> = {};
  for (const [id, css] of Object.entries(files)) {
    const label = css.match(/@nombre\s+(.+?)\s*(?:\*\/|$)/m)?.[1] ?? id;
    themes[id] = { label, css, layer: /@capa\b/.test(css) };
  }
  // @base: antepone el CSS del tema base (sin cadenas de bases, para no complicarlo)
  for (const t of Object.values(themes)) {
    const base = t.css.match(/@base\s+([\w-]+)/)?.[1];
    if (base && files[base]) t.css = `${files[base]}\n${t.css}`;
  }
  const result: Record<string, CatalogTheme> = {};
  const layers = Object.entries(themes).filter(([, t]) => t.layer);
  for (const [id, t] of Object.entries(themes)) {
    result[id] = { label: t.label, css: t.css };
    if (t.layer) continue;
    for (const [layerId, layer] of layers) {
      result[`${id}-${layerId}`] = { label: `${t.label} ${layer.label}`, css: `${t.css}\n${layer.css}` };
    }
  }
  return result;
}

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
  /** Temas que están como archivo en la bóveda (los demás son integrados o globales). */
  vaultThemes = $state<string[]>([]);
  /** Carpeta de temas globales que se lee siempre (`~/.config/synapsers/themes`). */
  defaultThemeDir = $state('');
  /** Carpetas extra de temas globales, de `appearance.theme_dirs` en el config.jsonc del usuario. */
  themeDirs = $state<string[]>([]);
  /** Temas globales leídos en el último escaneo. */
  globalThemes = $state<GlobalTheme[]>([]);
  private globalsLoaded = false;
  /** Integrados más globales, ya combinados con sus capas y bases. */
  private catalog = $derived(
    buildThemes({ ...builtinFiles, ...Object.fromEntries(this.globalThemes.map((t) => [t.id, t.css])) }),
  );
  /** Lo elegido en esta bóveda. */
  enabledSnippets = $state<string[]>([]);
  theme = $state<string | null>(null);

  /** Lee lo disponible y lo elegido en la bóveda abierta y lo aplica. */
  async load() {
    if (!this.globalsLoaded) await this.scanGlobalThemes();
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
    this.themes = [...new Set([...Object.keys(this.catalog), ...themes])].sort((a, b) => a.localeCompare(b));
    this.theme = config.theme && this.themes.includes(config.theme) ? config.theme : null;
    this.enabledSnippets = config.snippets.filter((s) => snippets.includes(s));
    await this.apply();
  }

  /** Vuelve a leer las carpetas de temas globales (sin aplicar nada). */
  private async scanGlobalThemes() {
    try {
      [this.defaultThemeDir, this.themeDirs, this.globalThemes] = await Promise.all([
        invokeTauri<string>('get_default_theme_dir'),
        invokeTauri<string[]>('get_theme_dirs'),
        invokeTauri<GlobalTheme[]>('list_global_themes'),
      ]);
      this.globalsLoaded = true;
    } catch (e) {
      console.warn('No se pudieron leer los temas globales', e);
    }
  }

  /** Reescanea las carpetas globales y la bóveda, y vuelve a aplicar lo elegido. */
  async rescan() {
    await this.scanGlobalThemes();
    await this.load();
  }

  /** Guarda las carpetas de temas globales en el config.jsonc del usuario y reescanea. */
  async setThemeDirs(dirs: string[]) {
    this.themeDirs = await invokeTauri<string[]>('save_theme_dirs', { dirs });
    await this.rescan();
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

  /** Nombre visible: el integrado o global tiene etiqueta propia salvo que la bóveda lo reemplace. */
  themeLabel(name: string) {
    return !this.vaultThemes.includes(name) && this.catalog[name] ? this.catalog[name].label : name;
  }

  /** Inyecta el tema y después los fragmentos, para que un fragmento pueda retocar el tema. */
  private async apply() {
    const theme = this.theme;
    const builtin = theme && !this.vaultThemes.includes(theme) ? this.catalog[theme] : undefined;
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
    if (builtin) parts.unshift(`/* tema: ${theme} */\n${builtin.css}`);
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
