import configRaw from '../../../../config.jsonc?raw';

export interface SupportedFileTypes {
  images: string[];
  markdown: string[];
  diagrams: string[];
  drawings: string[];
  code: string[];
}

export interface IgnoredConfig {
  directories: string[];
  suffixes: string[];
  prefixes: string[];
  files: string[];
}

export interface AppConfig {
  supported_files: SupportedFileTypes;
  ignored: IgnoredConfig;
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
      this.fileTypes.code,
    ];
    for (const group of groups) {
      for (const ext of group) {
        if (!list.includes(ext)) {
          list.push(ext);
        }
      }
    }
    return list;
  }

  private matchesExtension(path: string | null | undefined, extensions: string[]): boolean {
    if (!path) return false;
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
    return this.matchesExtension(path, this.fileTypes.markdown);
  }

  isDiagramFile(path: string | null | undefined): boolean {
    return this.matchesExtension(path, this.fileTypes.diagrams);
  }

  isDrawingFile(path: string | null | undefined): boolean {
    return this.matchesExtension(path, this.fileTypes.drawings);
  }

  isCodeFile(path: string | null | undefined): boolean {
    return this.matchesExtension(path, this.fileTypes.code);
  }

  isSupportedFile(path: string | null | undefined): boolean {
    return (
      this.isImageFile(path) ||
      this.isMarkdownFile(path) ||
      this.isDiagramFile(path) ||
      this.isDrawingFile(path) ||
      this.isCodeFile(path)
    );
  }
}

export const fileTypesManager = new FileTypesManager();

export const isImageFile = (path: string | null | undefined) => fileTypesManager.isImageFile(path);
export const isSvgFile = (path: string | null | undefined) => fileTypesManager.isSvgFile(path);
export const isMarkdownFile = (path: string | null | undefined) => fileTypesManager.isMarkdownFile(path);
export const isDiagramFile = (path: string | null | undefined) => fileTypesManager.isDiagramFile(path);
export const isDrawingFile = (path: string | null | undefined) => fileTypesManager.isDrawingFile(path);
export const isCodeFile = (path: string | null | undefined) => fileTypesManager.isCodeFile(path);
export const isSupportedFile = (path: string | null | undefined) => fileTypesManager.isSupportedFile(path);
