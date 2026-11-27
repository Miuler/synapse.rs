import { invokeTauri, isTauriEnvironment } from '@shared/repositories/tauri';
import { i18n, type SupportedLocale } from '@shared/lib/i18n';
import { setAgentCompletionSound, type AgentCompletionSound } from '@shared/lib/sound';
import { setAgentNoticeContent, type AgentNoticeContent } from '@shared/lib/attention';

export type MermaidRendererType = 'mermaidjs' | 'merman';
export type ThemeMode = 'system' | 'light' | 'dark';
export type ResolvedTheme = 'light' | 'dark';
export type ExcalidrawViewerType = 'native' | 'obsidian';
export type PropertiesViewMode = 'expanded' | 'collapsed' | 'hidden';
/** Clic en una carpeta del explorador: abre su `index.md` (por defecto) o la despliega */
export type FolderClickMode = 'index' | 'expand';
export type { SupportedLocale, AgentCompletionSound, AgentNoticeContent };

export const EMPTY_TAB_PARTS = ['search', 'recents', 'create', 'tips'] as const;
export type EmptyTabPart = (typeof EMPTY_TAB_PARTS)[number];
/** Valor de «fondo» para no mostrar ninguno (sin valor se usa el de la app). */
export const NO_WALLPAPER = 'none';

export interface AppSettings {
  mermaidRenderer: MermaidRendererType;
  lastOpenedFolder?: string;
  theme: ThemeMode;
  defaultExcalidrawViewer: ExcalidrawViewerType;
  vimMode: boolean;
  mcpServerEnabled: boolean;
  /** Consultar al iniciar si hay una versión nueva en GitHub (latest.json de la última release). */
  autoCheckUpdates: boolean;
  locale: SupportedLocale;
  propertiesViewMode: PropertiesViewMode;
  folderClick: FolderClickMode;
  plantumlServer: string;
  /** Dirección del editor de draw.io en modo embed (por defecto el público; puede ser uno propio sin red). */
  drawioUrl: string;
  contentZoom: number;
  /** Zoom de toda la ventana (zoom del webview), aparte del de contenido. */
  uiZoom: number;
  /** Zoom de los visores de Word y de hojas de cálculo (crece todo, como en un PDF). */
  docxZoom: number;
  sheetZoom: number;
  /** Zoom del chat del agente (mensajes y caja de texto). */
  chatZoom: number;
  /** Sonido al terminar un turno del agente ACP. */
  agentCompletionSound: AgentCompletionSound;
  /** Texto de la notificación de fin de turno: la respuesta final del agente o la consulta. */
  agentNoticeContent: AgentNoticeContent;
  /** Opacidad de la marca de agua con el logo del agente en el chat, de 0 (oculta) a 40 %. */
  agentWatermark: number;
  /** Lista de archivos recientes de la pestaña vacía, plegada. */
  recentsCollapsed: boolean;
  /** Cuántos archivos recientes lista la pestaña vacía. */
  recentsCount: number;
  /** Qué muestra la pestaña vacía sobre el fondo. */
  emptyTab: Record<EmptyTabPart, boolean>;
  /** Grosor del puntero láser en px (radio del punto; el rayo es algo más ancho), de 1 a 8. */
  laserSize: number;
  /** Imagen de fondo de la pestaña vacía elegida en cada bóveda: ruta de la bóveda → ruta absoluta. */
  wallpapers?: Record<string, string>;
}

/** Visores con zoom propio (crece todo su contenido) */
export type ZoomTarget = 'docx' | 'sheet' | 'chat';

const STORAGE_KEY = 'synapse_settings';

/** Había ajustes guardados al arrancar (antes de que la app escriba nada): distingue una
 *  actualización de una instalación nueva. */
export const hadStoredSettingsAtStartup: boolean = (() => {
  try {
    return typeof localStorage !== 'undefined' && localStorage.getItem(STORAGE_KEY) !== null;
  } catch {
    return false;
  }
})();

export const DEFAULT_SETTINGS: AppSettings = {
  mermaidRenderer: 'mermaidjs', // Por defecto el visor es Mermaid.js
  lastOpenedFolder: undefined,
  theme: 'system', // Por defecto sigue el tema del sistema operativo
  defaultExcalidrawViewer: 'native', // Por defecto los archivos .excalidraw se abren con Excalidraw nativo
  vimMode: false, // Por defecto modo Vim desactivado
  mcpServerEnabled: false, // Por defecto servidor MCP desactivado
  autoCheckUpdates: true, // Por defecto busca actualizaciones al iniciar (se puede desactivar)
  locale: 'en', // Idioma base; al arrancar se usa el detectado
  propertiesViewMode: 'expanded', // Por defecto visible y expandido
  folderClick: 'index', // Clic en carpeta: abre su index.md
  plantumlServer: 'https://www.plantuml.com/plantuml', // Servidor público de PlantUML
  drawioUrl: 'https://embed.diagrams.net/', // Editor público de draw.io en modo embed
  contentZoom: 1, // Tamaño del texto del editor y la previsualización
  uiZoom: 1, // Zoom de toda la interfaz
  docxZoom: 1, // Zoom del visor de Word
  sheetZoom: 1, // Zoom del visor de hojas de cálculo
  chatZoom: 1, // Zoom del chat del agente
  agentCompletionSound: 'burbuja_fm', // Sonido al terminar un turno del agente ACP
  agentNoticeContent: 'answer',
  agentWatermark: 10, // Opacidad (%) de la marca de agua del agente en el chat
  recentsCollapsed: false,
  recentsCount: 3,
  emptyTab: { search: true, recents: true, create: true, tips: true },
  laserSize: 2, // Grosor (px) del puntero láser
};

function clampWatermark(value: number): number {
  return Math.min(40, Math.max(0, Math.round(value)));
}

/** La app guarda como mucho los 15 últimos abiertos. */
export const MAX_RECENTS_COUNT = 15;

function clampRecentsCount(value: number): number {
  return Math.min(MAX_RECENTS_COUNT, Math.max(1, Math.round(value)));
}

function clampLaserSize(value: number): number {
  return Math.min(8, Math.max(1, Math.round(value * 2) / 2));
}

function clampZoom(zoom: number): number {
  return Math.min(3, Math.max(0.5, Math.round(zoom * 20) / 20));
}

function loadInitialSettings(): AppSettings {
  const settings: AppSettings = { ...DEFAULT_SETTINGS, locale: i18n.locale };
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
      if (parsed.defaultExcalidrawViewer === 'native' || parsed.defaultExcalidrawViewer === 'obsidian') {
        settings.defaultExcalidrawViewer = parsed.defaultExcalidrawViewer;
      }
      if (typeof parsed.contentZoom === 'number' && Number.isFinite(parsed.contentZoom)) {
        settings.contentZoom = clampZoom(parsed.contentZoom);
      }
      if (typeof parsed.uiZoom === 'number' && Number.isFinite(parsed.uiZoom)) {
        settings.uiZoom = clampZoom(parsed.uiZoom);
      }
      for (const key of ['docxZoom', 'sheetZoom', 'chatZoom'] as const) {
        const z = parsed[key];
        if (typeof z === 'number' && Number.isFinite(z)) settings[key] = clampZoom(z);
      }
      if (typeof parsed.vimMode === 'boolean') {
        settings.vimMode = parsed.vimMode;
      }
      if (typeof parsed.mcpServerEnabled === 'boolean') {
        settings.mcpServerEnabled = parsed.mcpServerEnabled;
      }
      if (typeof parsed.autoCheckUpdates === 'boolean') {
        settings.autoCheckUpdates = parsed.autoCheckUpdates;
      }
      if (parsed.locale === 'es' || parsed.locale === 'en') {
        settings.locale = parsed.locale;
      }
      if (parsed.propertiesViewMode === 'expanded' || parsed.propertiesViewMode === 'collapsed' || parsed.propertiesViewMode === 'hidden') {
        settings.propertiesViewMode = parsed.propertiesViewMode;
      }
      if (parsed.folderClick === 'index' || parsed.folderClick === 'expand') {
        settings.folderClick = parsed.folderClick;
      }
      if (
        parsed.agentCompletionSound === 'burbuja_fm' ||
        parsed.agentCompletionSound === 'sonar_organico' ||
        parsed.agentCompletionSound === 'none'
      ) {
        settings.agentCompletionSound = parsed.agentCompletionSound;
      }
      if (parsed.agentNoticeContent === 'answer' || parsed.agentNoticeContent === 'query') {
        settings.agentNoticeContent = parsed.agentNoticeContent;
      }
      if (typeof parsed.agentWatermark === 'number' && Number.isFinite(parsed.agentWatermark)) {
        settings.agentWatermark = clampWatermark(parsed.agentWatermark);
      }
      if (typeof parsed.recentsCollapsed === 'boolean') {
        settings.recentsCollapsed = parsed.recentsCollapsed;
      }
      if (typeof parsed.recentsCount === 'number' && Number.isFinite(parsed.recentsCount)) {
        settings.recentsCount = clampRecentsCount(parsed.recentsCount);
      }
      if (parsed.emptyTab && typeof parsed.emptyTab === 'object') {
        settings.emptyTab = Object.fromEntries(
          EMPTY_TAB_PARTS.map((part) => [part, parsed.emptyTab[part] !== false]),
        ) as Record<EmptyTabPart, boolean>;
      }
      if (typeof parsed.laserSize === 'number' && Number.isFinite(parsed.laserSize)) {
        settings.laserSize = clampLaserSize(parsed.laserSize);
      }
      if (parsed.wallpapers && typeof parsed.wallpapers === 'object') {
        settings.wallpapers = Object.fromEntries(
          Object.entries(parsed.wallpapers).filter(([k, v]) => k && typeof v === 'string' && v.trim()),
        ) as Record<string, string>;
      }
      // Antes el fondo era general: pasa a la bóveda que estaba abierta
      if (typeof parsed.wallpaper === 'string' && parsed.wallpaper.trim() && settings.lastOpenedFolder) {
        settings.wallpapers = { [settings.lastOpenedFolder]: parsed.wallpaper.trim(), ...settings.wallpapers };
      }
      if (typeof parsed.plantumlServer === 'string' && /^https?:\/\//i.test(parsed.plantumlServer.trim())) {
        settings.plantumlServer = parsed.plantumlServer.trim();
      }
      if (typeof parsed.drawioUrl === 'string' && /^https?:\/\//i.test(parsed.drawioUrl.trim())) {
        settings.drawioUrl = parsed.drawioUrl.trim();
      }
    }
  } catch (e) {
    // Si localStorage no está disponible o falla el parseo, usar valores por defecto
  }

  return settings;
}

class SettingsManager {
  settings = $state<AppSettings>(loadInitialSettings());
  systemTheme = $state<ResolvedTheme>('light');
  private isCheckingTheme = false;

  constructor() {
    if (this.settings.locale) {
      i18n.setLocale(this.settings.locale);
    }
    setAgentCompletionSound(this.settings.agentCompletionSound || 'burbuja_fm');
    setAgentNoticeContent(this.settings.agentNoticeContent);
    this.applyLaserSize();

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

      // Polling ligero para sincronización en tiempo real cuando la ventana está visible
      window.setInterval(() => {
        if (this.settings.theme === 'system' && !document.hidden) {
          this.checkSystemTheme();
        }
      }, 5000);
    }
    this.applyTheme();
  }

  async checkSystemTheme(): Promise<void> {
    if (this.isCheckingTheme) return;
    this.isCheckingTheme = true;
    try {
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
    } finally {
      this.isCheckingTheme = false;
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

  get locale(): SupportedLocale {
    return this.settings.locale || 'en';
  }

  setLocale(locale: SupportedLocale) {
    this.settings.locale = locale;
    i18n.setLocale(locale);
    this.persist();
  }

  toggleLocale(): SupportedLocale {
    const next: SupportedLocale = this.locale === 'es' ? 'en' : 'es';
    this.setLocale(next);
    return next;
  }

  get mermaidRenderer(): MermaidRendererType {
    return this.settings.mermaidRenderer;
  }

  setPlantumlServer(server: string) {
    this.settings.plantumlServer = server;
    this.persist();
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

  get defaultExcalidrawViewer(): ExcalidrawViewerType {
    return this.settings.defaultExcalidrawViewer || 'native';
  }

  setDefaultExcalidrawViewer(viewer: ExcalidrawViewerType) {
    this.settings.defaultExcalidrawViewer = viewer;
    this.persist();
  }

  toggleDefaultExcalidrawViewer() {
    this.settings.defaultExcalidrawViewer =
      this.settings.defaultExcalidrawViewer === 'native' ? 'obsidian' : 'native';
    this.persist();
  }

  get lastOpenedFolder(): string | undefined {
    return this.settings.lastOpenedFolder;
  }

  setLastOpenedFolder(folder: string | undefined) {
    this.settings.lastOpenedFolder = folder;
    this.persist();
  }

  get contentZoom(): number {
    return this.settings.contentZoom;
  }

  setContentZoom(zoom: number) {
    const next = clampZoom(zoom);
    if (next === this.settings.contentZoom) return;
    this.settings = { ...this.settings, contentZoom: next };
    this.persist();
  }

  get uiZoom(): number {
    return this.settings.uiZoom;
  }

  setUiZoom(zoom: number) {
    const next = clampZoom(zoom);
    if (next === this.settings.uiZoom) return;
    this.settings = { ...this.settings, uiZoom: next };
    this.persist();
  }

  get docxZoom(): number {
    return this.settings.docxZoom;
  }

  get sheetZoom(): number {
    return this.settings.sheetZoom;
  }

  get chatZoom(): number {
    return this.settings.chatZoom;
  }

  /** Zoom del visor de Word (`docx`), de hojas de cálculo (`sheet`) o del chat del agente (`chat`), de 50 % a 300 %. */
  setViewerZoom(viewer: ZoomTarget, zoom: number) {
    const key = viewer === 'docx' ? 'docxZoom' : viewer === 'sheet' ? 'sheetZoom' : 'chatZoom';
    const next = clampZoom(zoom);
    if (next === this.settings[key]) return;
    this.settings = { ...this.settings, [key]: next };
    this.persist();
  }

  /** Fondo propio de la bóveda abierta, si se eligió uno. */
  get wallpaper(): string | undefined {
    const vault = this.settings.lastOpenedFolder;
    return vault ? this.settings.wallpapers?.[vault] : undefined;
  }

  setWallpaper(path: string | undefined) {
    const vault = this.settings.lastOpenedFolder;
    if (!vault) return;
    const wallpapers = { ...this.settings.wallpapers };
    if (path?.trim()) wallpapers[vault] = path.trim();
    else delete wallpapers[vault];
    this.settings = { ...this.settings, wallpapers };
    this.persist();
  }

  get plantumlServer(): string {
    return this.settings.plantumlServer;
  }

  get vimMode(): boolean {
    return this.settings.vimMode ?? false;
  }

  setVimMode(enabled: boolean) {
    this.settings.vimMode = enabled;
    this.persist();
  }

  toggleVimMode(): boolean {
    const next = !this.vimMode;
    this.setVimMode(next);
    return next;
  }

  get mcpServerEnabled(): boolean {
    return this.settings.mcpServerEnabled ?? false;
  }

  setMcpServerEnabled(enabled: boolean) {
    this.settings.mcpServerEnabled = enabled;
    this.persist();
  }

  get autoCheckUpdates(): boolean {
    return this.settings.autoCheckUpdates ?? true;
  }

  setAutoCheckUpdates(enabled: boolean) {
    this.settings.autoCheckUpdates = enabled;
    this.persist();
  }

  get folderClick(): FolderClickMode {
    return this.settings.folderClick || 'index';
  }

  setFolderClick(mode: FolderClickMode) {
    this.settings = { ...this.settings, folderClick: mode };
    this.persist();
  }

  get agentCompletionSound(): AgentCompletionSound {
    return this.settings.agentCompletionSound || 'burbuja_fm';
  }

  setAgentCompletionSound(sound: AgentCompletionSound) {
    this.settings = { ...this.settings, agentCompletionSound: sound };
    setAgentCompletionSound(sound);
    this.persist();
  }

  get agentNoticeContent(): AgentNoticeContent {
    return this.settings.agentNoticeContent;
  }

  setAgentNoticeContent(content: AgentNoticeContent) {
    this.settings = { ...this.settings, agentNoticeContent: content };
    setAgentNoticeContent(content);
    this.persist();
  }

  get agentWatermark(): number {
    return this.settings.agentWatermark ?? 10;
  }

  setAgentWatermark(percent: number) {
    const next = clampWatermark(percent);
    if (next === this.agentWatermark) return;
    this.settings = { ...this.settings, agentWatermark: next };
    this.persist();
  }

  get recentsCollapsed(): boolean {
    return this.settings.recentsCollapsed;
  }

  setRecentsCollapsed(collapsed: boolean) {
    this.settings = { ...this.settings, recentsCollapsed: collapsed };
    this.persist();
  }

  get recentsCount(): number {
    return this.settings.recentsCount;
  }

  setRecentsCount(count: number) {
    if (!Number.isFinite(count)) return;
    this.settings = { ...this.settings, recentsCount: clampRecentsCount(count) };
    this.persist();
  }

  get emptyTab(): Record<EmptyTabPart, boolean> {
    return this.settings.emptyTab;
  }

  setEmptyTab(parts: Partial<Record<EmptyTabPart, boolean>>) {
    this.settings = { ...this.settings, emptyTab: { ...this.settings.emptyTab, ...parts } };
    this.persist();
  }

  get laserSize(): number {
    return this.settings.laserSize ?? 2;
  }

  setLaserSize(px: number) {
    const next = clampLaserSize(px);
    if (next === this.laserSize) return;
    this.settings = { ...this.settings, laserSize: next };
    this.applyLaserSize();
    this.persist();
  }

  /** El lienzo del láser (capa `shared`) lee el grosor de `--laser-size`. */
  applyLaserSize() {
    if (typeof document === 'undefined') return;
    document.documentElement.style.setProperty('--laser-size', String(this.laserSize));
  }

  get propertiesViewMode(): PropertiesViewMode {
    return this.settings.propertiesViewMode || 'expanded';
  }

  setPropertiesViewMode(mode: PropertiesViewMode) {
    this.settings.propertiesViewMode = mode;
    this.persist();
  }

  toggleMcpServerEnabled(): boolean {
    const next = !this.mcpServerEnabled;
    this.setMcpServerEnabled(next);
    return next;
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
