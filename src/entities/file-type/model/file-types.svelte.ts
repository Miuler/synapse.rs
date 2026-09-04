import { invokeTauri, isTauriEnvironment } from '@shared/api';

export interface SupportedFileTypes {
  images: string[];
  markdown: string[];
  diagrams: string[];
  drawings: string[];
  code: string[];
}

export const DEFAULT_SUPPORTED_FILE_TYPES: SupportedFileTypes = {
  images: ['png', 'webp', 'jpg', 'jpeg', 'gif', 'bmp', 'svg', 'ico', 'avif'],
  markdown: ['md', 'markdown'],
  diagrams: ['mmd', 'mermaid'],
  drawings: ['excalidraw', 'excalidraw.json'],
  code: ['rs'],
};

class FileTypesManager {
  fileTypes = $state<SupportedFileTypes>(DEFAULT_SUPPORTED_FILE_TYPES);
  isLoadedFromTauri = $state<boolean>(false);

  constructor() {
    this.initFromTauri();
  }

  async initFromTauri(): Promise<SupportedFileTypes> {
    if (isTauriEnvironment()) {
      try {
        const types = await invokeTauri<SupportedFileTypes>('get_supported_file_types');
        if (types && Array.isArray(types.images)) {
          this.fileTypes = types;
          this.isLoadedFromTauri = true;
          return types;
        }
      } catch (err) {
        console.warn('No se pudo cargar SupportedFileTypes desde Tauri, usando defaults:', err);
      }
    }
    return this.fileTypes;
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
