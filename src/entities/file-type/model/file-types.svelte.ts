import configRaw from '../../../../config.jsonc?raw';

export interface SupportedFileTypes {
  images: string[];
  markdown: string[];
  diagrams: string[];
  drawings: string[];
  pdf: string[];
  code: string[];
  word?: string[];
  excel?: string[];
}

export interface IgnoredConfig {
  directories: string[];
  suffixes: string[];
  prefixes: string[];
  files: string[];
}

export interface EditorConfig {
  auto_save: boolean;
  auto_save_delay_ms: number;
  drawing_auto_save_delay_ms?: number;
  drawing_change_debounce_ms?: number;
  fs_watcher_recent_save_grace_ms?: number;
}

export interface DiffConfig {
  max_expand_lines: number;
}

export interface AppConfig {
  supported_files: SupportedFileTypes;
  ignored: IgnoredConfig;
  editor?: EditorConfig;
  diff?: DiffConfig;
}

function parseJsonc<T>(raw: string): T {
  const cleaned = raw
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/\/\/[^\n\r]*/g, '')
    .replace(/,\s*([\]}])/g, '$1');
  return JSON.parse(cleaned);
}

const parsedConfig = parseJsonc<AppConfig>(configRaw);
export const DEFAULT_APP_CONFIG: AppConfig = parsedConfig;
export const DEFAULT_SUPPORTED_FILE_TYPES: SupportedFileTypes = parsedConfig.supported_files;
export const DEFAULT_IGNORED_CONFIG: IgnoredConfig = parsedConfig.ignored;

/**
 * Entidad pura que gestiona el estado y las reglas de dominio
 * sobre los tipos y extensiones de archivo soportados.
 * No tiene dependencias de servicios externos ni de infraestructura (I/O).
 */
class FileTypesManager {
  fileTypes = $state<SupportedFileTypes>(DEFAULT_SUPPORTED_FILE_TYPES);
  isLoaded = $state<boolean>(false);

  /**
   * Actualiza el estado de la entidad con nuevos tipos soportados provistos externamente.
   */
  setSupportedFileTypes(types: SupportedFileTypes): void {
    if (types && Array.isArray(types.images)) {
      this.fileTypes = types;
      this.isLoaded = true;
    }
  }

  getAllExtensions(): string[] {
    const list: string[] = [];
    const groups = [
      this.fileTypes.images,
      this.fileTypes.markdown,
      this.fileTypes.diagrams,
      this.fileTypes.drawings,
      this.fileTypes.pdf,
      this.fileTypes.code,
      this.fileTypes.word,
      this.fileTypes.excel,
    ];
    for (const group of groups) {
      if (!Array.isArray(group)) continue;
      for (const ext of group) {
        if (!list.includes(ext)) {
          list.push(ext);
        }
      }
    }
    return list;
  }

  private matchesExtension(path: string | null | undefined, extensions: string[] | undefined): boolean {
    if (!path || !Array.isArray(extensions)) return false;
    const lower = path.toLowerCase().split('?')[0].split('#')[0];
    return extensions.some((ext) => {
      const cleanExt = ext.toLowerCase().replace(/^\./, '');
      return lower.endsWith(`.${cleanExt}`);
    });
  }

  isImageFile(path: string | null | undefined): boolean {
    return this.matchesExtension(path, this.fileTypes.images);
  }

  isSvgFile(path: string | null | undefined): boolean {
    if (!path) return false;
    const lower = path.toLowerCase().split('?')[0].split('#')[0];
    return lower.endsWith('.svg');
  }

  isMarkdownFile(path: string | null | undefined): boolean {
    if (!path) return false;
    if (path.startsWith('synapse://')) return false;
    const lower = path.toLowerCase().split('?')[0].split('#')[0];
    if (lower.endsWith('.excalidraw.md') || lower.endsWith('.excalidraw.excalidraw')) {
      return false;
    }
    return this.matchesExtension(path, this.fileTypes.markdown);
  }

  isDiagramFile(path: string | null | undefined): boolean {
    return this.matchesExtension(path, this.fileTypes.diagrams);
  }

  isD2File(path: string | null | undefined): boolean {
    if (!path) return false;
    const lower = path.toLowerCase().split('?')[0].split('#')[0];
    return lower.endsWith('.d2');
  }

  isMermaidFile(path: string | null | undefined): boolean {
    if (!path) return false;
    const lower = path.toLowerCase().split('?')[0].split('#')[0];
    return lower.endsWith('.mmd') || lower.endsWith('.mermaid');
  }

  isDrawingFile(path: string | null | undefined): boolean {
    if (!path) return false;
    const lower = path.toLowerCase().split('?')[0].split('#')[0];
    if (
      lower.endsWith('.excalidraw') ||
      lower.endsWith('.excalidraw.json') ||
      lower.endsWith('.excalidraw.md') ||
      lower.endsWith('.excalidraw.excalidraw')
    ) {
      return true;
    }
    return this.matchesExtension(path, this.fileTypes.drawings);
  }

  isPdfFile(path: string | null | undefined): boolean {
    return this.matchesExtension(path, this.fileTypes.pdf);
  }

  isCodeFile(path: string | null | undefined): boolean {
    return this.matchesExtension(path, this.fileTypes.code);
  }

  isWordFile(path: string | null | undefined): boolean {
    return this.matchesExtension(path, this.fileTypes.word || ['docx', 'doc', 'docm', 'dot', 'dotx']);
  }

  isExcelFile(path: string | null | undefined): boolean {
    return this.matchesExtension(path, this.fileTypes.excel || ['xlsx', 'xls', 'xlsm', 'xlt', 'xltx']);
  }

  isOfficeFile(path: string | null | undefined): boolean {
    return this.isWordFile(path) || this.isExcelFile(path);
  }

  isSupportedFile(path: string | null | undefined): boolean {
    return (
      this.isImageFile(path) ||
      this.isMarkdownFile(path) ||
      this.isDiagramFile(path) ||
      this.isDrawingFile(path) ||
      this.isPdfFile(path) ||
      this.isCodeFile(path) ||
      this.isOfficeFile(path)
    );
  }
}

export const fileTypesManager = new FileTypesManager();

export const isImageFile = (path: string | null | undefined) => fileTypesManager.isImageFile(path);
export const isSvgFile = (path: string | null | undefined) => fileTypesManager.isSvgFile(path);
export const isMarkdownFile = (path: string | null | undefined) => fileTypesManager.isMarkdownFile(path);
export const isDiagramFile = (path: string | null | undefined) => fileTypesManager.isDiagramFile(path);
export const isD2File = (path: string | null | undefined) => fileTypesManager.isD2File(path);
export const isMermaidFile = (path: string | null | undefined) => fileTypesManager.isMermaidFile(path);
export const isDrawingFile = (path: string | null | undefined) => fileTypesManager.isDrawingFile(path);
export const isPdfFile = (path: string | null | undefined) => fileTypesManager.isPdfFile(path);
export const isCodeFile = (path: string | null | undefined) => fileTypesManager.isCodeFile(path);
export const isWordFile = (path: string | null | undefined) => fileTypesManager.isWordFile(path);
export const isExcelFile = (path: string | null | undefined) => fileTypesManager.isExcelFile(path);
export const isOfficeFile = (path: string | null | undefined) => fileTypesManager.isOfficeFile(path);
export const isSupportedFile = (path: string | null | undefined) => fileTypesManager.isSupportedFile(path);
