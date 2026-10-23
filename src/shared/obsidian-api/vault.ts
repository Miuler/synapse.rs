import type { VaultFile as RepositoryVaultFile, VaultRepository } from '@shared/repositories';
import { SynapseDataAdapter } from './adapter';
import { Events } from './events';
import {
  TAbstractFile,
  TFile,
  TFolder,
  type DataAdapter,
  type SynapseVaultLike,
} from './types';
import type { SynapseMetadataCache } from './metadata-cache';

function pathOf(file: TAbstractFile | string): string {
  return typeof file === 'string' ? file.replace(/^\/+/, '') : file.path;
}

function tFileFromMetadata(file: RepositoryVaultFile, vault: SynapseVaultLike): TFile {
  return new TFile(
    vault,
    file.path,
    file.stat,
    file.view_mode
  );
}

export class SynapseVault extends Events implements SynapseVaultLike {
  readonly adapter: DataAdapter;
  private readonly rootFolder: TFolder;
  private metadataCache?: SynapseMetadataCache;
  private readonly recentMutations: Map<string, number> = new Map();
  private unlistenWatcher?: () => void;
  private _cachedFiles: TFile[] = [];
  private _filesLoaded = false;

  constructor(private readonly repository: VaultRepository) {
    super();
    this.rootFolder = new TFolder(this, '');

    // Adaptador de datos con propagación reactiva de mutaciones directas hacia el bus de eventos
    this.adapter = new SynapseDataAdapter(repository, (type, path, extra) => {
      this.handleAdapterMutation(type, path, extra);
    });

    // Iniciar escucha del watcher de cambios en disco si el repositorio lo soporta
    this.setupWatcher();
  }

  /**
   * Conecta la caché reactiva de metadatos a la bóveda para mantenerla sincronizada con las mutaciones.
   */
  setMetadataCache(cache: SynapseMetadataCache): void {
    this.metadataCache = cache;
  }

  /**
   * Registra una mutación local interna para evitar eventos duplicados cuando el watcher del SO notifique.
   */
  private recordMutation(path: string): void {
    const clean = pathOf(path);
    this.recentMutations.set(clean, Date.now());
  }

  private isRecentlyMutated(path: string): boolean {
    const clean = pathOf(path);
    const last = this.recentMutations.get(clean) || 0;
    return Date.now() - last < 2000;
  }

  /**
   * Gestiona mutaciones originadas directamente a través de app.vault.adapter
   */
  private async handleAdapterMutation(
    type: 'write' | 'writeBinary' | 'remove' | 'rename',
    path: string,
    extra?: any
  ): Promise<void> {
    const clean = pathOf(path);
    if (this.isRecentlyMutated(clean)) return;

    if (type === 'write' || type === 'writeBinary') {
      const file = (await this.getAbstractFileByPath(clean)) || new TFile(this, clean);
      this.trigger('modify', file);
      if (typeof extra === 'string' && this.metadataCache && /\.(md|markdown)$/i.test(clean)) {
        const cache = this.metadataCache.computeAndStoreCache(clean, extra);
        if (file instanceof TFile) {
          this.metadataCache.trigger('changed', file, extra, cache);
          this.metadataCache.trigger('resolve', file);
        }
      }
    } else if (type === 'remove') {
      const file = new TFile(this, clean);
      this.trigger('delete', file);
      this.metadataCache?.deleteCache(clean);
    } else if (type === 'rename') {
      const oldClean = pathOf(extra);
      const file = (await this.getAbstractFileByPath(clean)) || new TFile(this, clean);
      this.trigger('rename', file, oldClean);
      this.metadataCache?.renameCache(oldClean, clean);
    }
  }

  /**
   * Configura la escucha del watcher de archivos (Rust notify)
   */
  private async setupWatcher(): Promise<void> {
    if (typeof this.repository.onFilesChanged !== 'function') return;

    try {
      const unlisten = await this.repository.onFilesChanged(async (payload) => {
        const { paths, deleted } = payload;

        // 1. Archivos eliminados
        for (const delPath of deleted) {
          const clean = pathOf(delPath);
          if (this.isRecentlyMutated(clean)) continue;

          const file = new TFile(this, clean);
          this.trigger('delete', file);
          this.metadataCache?.deleteCache(clean);
        }

        // 2. Archivos modificados o creados externamente
        for (const changedPath of paths) {
          const clean = pathOf(changedPath);
          if (this.isRecentlyMutated(clean)) continue;

          const file = await this.getAbstractFileByPath(clean);
          if (file) {
            this.trigger('modify', file);

            // Si es markdown, sincronizar caché de metadatos
            if (file instanceof TFile && /\.(md|markdown)$/i.test(clean)) {
              this.repository.readNote(clean).then((note) => {
                if (note && this.metadataCache) {
                  const content = note.content ?? '';
                  const cache = this.metadataCache.computeAndStoreCache(clean, content);
                  this.metadataCache.trigger('changed', file, content, cache);
                  this.metadataCache.trigger('resolve', file);
                }
              }).catch(() => {});
            }
          }
        }
      });

      this.unlistenWatcher = typeof unlisten === 'function' ? unlisten : undefined;
    } catch (e) {
      console.warn('[SynapseVault] No se pudo inicializar listener del watcher:', e);
    }
  }

  /**
   * Destruye suscripciones activas del watcher.
   */
  destroy(): void {
    if (this.unlistenWatcher) {
      this.unlistenWatcher();
      this.unlistenWatcher = undefined;
    }
    this.trigger('closed');
  }

  /**
   * Retorna la carpeta raíz de la bóveda.
   */
  getRoot(): TFolder {
    return this.rootFolder;
  }

  /**
   * Consulta puntual O(1) directamente al índice DashMap de Rust.
   * Retorna una instancia de TFile o TFolder.
   */
  async getAbstractFileByPath(path: string): Promise<TAbstractFile | null> {
    if (!path) return null;
    const cleanPath = path.replace(/^\/+/, '');
    if (!cleanPath) return this.rootFolder;

    const meta = await this.repository.getNoteMeta(cleanPath);
    if (meta) {
      return tFileFromMetadata(meta, this);
    }

    const children = await this.repository.getDirectoryChildren(cleanPath);
    if (children.length > 0) {
      const folder = new TFolder(this, cleanPath);
      folder.children = children.map((c) =>
        c.is_folder
          ? new TFolder(this, c.relative_path)
          : new TFile(this, c.relative_path)
      );
      return folder;
    }

    return null;
  }

  /**
   * Retorna todos los archivos presentes en el índice de la bóveda como instancias de TFile.
   * Soporta tanto uso síncrono directo (retorna Array con métodos como .some()) como invocación con `await`.
   */
  getFiles(): TFile[] {
    const list: any = [...this._cachedFiles];
    list.then = (onFulfilled?: (value: TFile[]) => any, onRejected?: (reason: any) => any) => {
      return this.repository.getVaultFiles().then(
        (files) => {
          this._cachedFiles = files.map((file) => tFileFromMetadata(file, this));
          this._filesLoaded = true;
          return onFulfilled ? onFulfilled(this._cachedFiles) : this._cachedFiles;
        },
        onRejected
      );
    };
    return list;
  }

  /**
   * Retorna únicamente los archivos Markdown (.md, .markdown) como instancias de TFile.
   */
  getMarkdownFiles(): TFile[] {
    const list: any = this._cachedFiles.filter((file) => /\.(md|markdown)$/i.test(file.path));
    list.then = (onFulfilled?: (value: TFile[]) => any, onRejected?: (reason: any) => any) => {
      return Promise.resolve(this.getFiles()).then((files) => {
        const md = files.filter((file) => /\.(md|markdown)$/i.test(file.path));
        return onFulfilled ? onFulfilled(md) : md;
      }, onRejected);
    };
    return list;
  }

  /**
   * Retorna todos los archivos y carpetas cargados en la bóveda.
   */
  getAllLoadedFiles(): TAbstractFile[] {
    const folderPaths = new Set<string>();
    for (const file of this._cachedFiles) {
      let p = file.path.includes('/') ? file.path.slice(0, file.path.lastIndexOf('/')) : '';
      while (p) {
        folderPaths.add(p);
        p = p.includes('/') ? p.slice(0, p.lastIndexOf('/')) : '';
      }
    }
    const folders = Array.from(folderPaths).map((p) => new TFolder(this, p));
    const all: any = [this.rootFolder, ...folders, ...this._cachedFiles];
    all.then = (onFulfilled?: (value: TAbstractFile[]) => any, onRejected?: (reason: any) => any) => {
      return Promise.resolve(this.getFiles()).then(() => {
        const fps = new Set<string>();
        for (const file of this._cachedFiles) {
          let p = file.path.includes('/') ? file.path.slice(0, file.path.lastIndexOf('/')) : '';
          while (p) {
            fps.add(p);
            p = p.includes('/') ? p.slice(0, p.lastIndexOf('/')) : '';
          }
        }
        const fs = Array.from(fps).map((p) => new TFolder(this, p));
        const res = [this.rootFolder, ...fs, ...this._cachedFiles];
        return onFulfilled ? onFulfilled(res) : res;
      }, onRejected);
    };
    return all;
  }

  /**
   * Lee el contenido del archivo bajo demanda como texto UTF-8.
   */
  async read(file: TFile | string): Promise<string> {
    const path = pathOf(file);
    const content = await this.adapter.read(path);

    // Si aún no está en caché de metadatos, poblarla
    if (this.metadataCache && /\.(md|markdown)$/i.test(path)) {
      if (!this.metadataCache.getFileCacheSync(path)) {
        this.metadataCache.computeAndStoreCache(path, content);
      }
    }

    return content;
  }

  /**
   * Lee el contenido del archivo bajo demanda como datos binarios (ArrayBuffer).
   */
  async readBinary(file: TFile | string): Promise<ArrayBuffer> {
    const path = pathOf(file);
    return this.adapter.readBinary(path);
  }

  /**
   * Lectura con caché delegada.
   */
  cachedRead(file: TFile | string): Promise<string> {
    return this.read(file);
  }

  /**
   * Modifica el contenido de texto de un archivo existente y dispara 'modify' y 'changed'.
   */
  async modify(file: TFile | string, data: string): Promise<void> {
    const path = pathOf(file);
    this.recordMutation(path);
    await this.adapter.write(path, data);

    const tFile = file instanceof TFile ? file : (await this.getAbstractFileByPath(path)) || new TFile(this, path);
    this.trigger('modify', tFile);

    if (this.metadataCache && /\.(md|markdown)$/i.test(path)) {
      const cache = this.metadataCache.computeAndStoreCache(path, data);
      if (tFile instanceof TFile) {
        this.metadataCache.trigger('changed', tFile, data, cache);
        this.metadataCache.trigger('resolve', tFile);
      }
    }
  }

  /**
   * Modifica el contenido binario de un archivo existente y dispara 'modify'.
   */
  async modifyBinary(file: TFile | string, data: ArrayBuffer | Uint8Array): Promise<void> {
    const path = pathOf(file);
    this.recordMutation(path);
    await this.adapter.writeBinary(path, data);

    const tFile = file instanceof TFile ? file : (await this.getAbstractFileByPath(path)) || new TFile(this, path);
    this.trigger('modify', tFile);
  }

  /**
   * Crea un nuevo archivo de texto, dispara 'create' y 'changed', y retorna su instancia TFile.
   */
  async create(path: string, data: string): Promise<TFile> {
    const cleanPath = pathOf(path);
    this.recordMutation(cleanPath);
    await this.adapter.write(cleanPath, data);

    const abstractFile = await this.getAbstractFileByPath(cleanPath);
    const file: TFile =
      abstractFile instanceof TFile
        ? abstractFile
        : new TFile(this, cleanPath, { ctime: Date.now(), mtime: Date.now(), size: data.length });

    this.trigger('create', file);

    if (this.metadataCache && /\.(md|markdown)$/i.test(cleanPath)) {
      const cache = this.metadataCache.computeAndStoreCache(cleanPath, data);
      this.metadataCache.trigger('changed', file, data, cache);
      this.metadataCache.trigger('resolve', file);
    }

    return file;
  }

  /**
   * Crea un nuevo archivo binario, dispara 'create' y retorna su instancia TFile.
   */
  async createBinary(path: string, data: ArrayBuffer | Uint8Array): Promise<TFile> {
    const cleanPath = pathOf(path);
    this.recordMutation(cleanPath);
    await this.adapter.writeBinary(cleanPath, data);

    const abstractFile = await this.getAbstractFileByPath(cleanPath);
    const byteLen = data instanceof ArrayBuffer ? data.byteLength : data.length;
    const file: TFile =
      abstractFile instanceof TFile
        ? abstractFile
        : new TFile(this, cleanPath, { ctime: Date.now(), mtime: Date.now(), size: byteLen });

    this.trigger('create', file);
    return file;
  }

  /**
   * Crea una nueva carpeta en la bóveda, dispara 'create' y retorna su instancia TFolder.
   */
  async createFolder(path: string): Promise<TFolder> {
    const cleanPath = pathOf(path);
    this.recordMutation(cleanPath);
    await this.adapter.mkdir(cleanPath);

    const folder = new TFolder(this, cleanPath);
    this.trigger('create', folder);
    return folder;
  }

  /**
   * Elimina un archivo o carpeta de la bóveda y dispara 'delete'.
   */
  async delete(file: TAbstractFile | string, _force?: boolean): Promise<void> {
    const path = pathOf(file);
    this.recordMutation(path);
    await this.adapter.remove(path);

    const target = file instanceof TAbstractFile ? file : new TFile(this, path);
    this.trigger('delete', target);
    this.metadataCache?.deleteCache(path);
  }

  /**
   * Envía un archivo o carpeta a la papelera y dispara 'delete'.
   */
  async trash(file: TAbstractFile | string, system: boolean): Promise<void> {
    const path = pathOf(file);
    this.recordMutation(path);
    if (system) {
      await this.adapter.trashSystem(path);
    } else {
      await this.adapter.trashLocal(path);
    }

    const target = file instanceof TAbstractFile ? file : new TFile(this, path);
    this.trigger('delete', target);
    this.metadataCache?.deleteCache(path);
  }

  /**
   * Renombra o mueve un archivo o carpeta dentro de la bóveda y dispara 'rename'.
   */
  async rename(file: TAbstractFile | string, newPath: string): Promise<void> {
    const oldPath = pathOf(file);
    const cleanNewPath = pathOf(newPath);
    this.recordMutation(oldPath);
    this.recordMutation(cleanNewPath);
    await this.adapter.rename(oldPath, cleanNewPath);

    const target = (await this.getAbstractFileByPath(cleanNewPath)) || new TFile(this, cleanNewPath);
    this.trigger('rename', target, oldPath);
    this.metadataCache?.renameCache(oldPath, cleanNewPath);
  }

  /**
   * Copia un archivo a una nueva ruta de destino y dispara 'create'.
   */
  async copy(file: TFile | string, newPath: string): Promise<TFile> {
    const sourcePath = pathOf(file);
    const cleanNewPath = pathOf(newPath);
    this.recordMutation(cleanNewPath);
    await this.adapter.copy(sourcePath, cleanNewPath);

    const copied = (await this.getAbstractFileByPath(cleanNewPath)) || new TFile(this, cleanNewPath);
    this.trigger('create', copied);

    return copied instanceof TFile ? copied : new TFile(this, cleanNewPath);
  }

  /**
   * Comprueba en O(1) si un archivo o carpeta existe.
   */
  async exists(path: string): Promise<boolean> {
    const cleanPath = pathOf(path);
    return this.adapter.exists(cleanPath);
  }

  /**
   * Obtiene la URL local de Tauri para renderizar el recurso en el WebView.
   */
  getResourcePath(file: TFile | string): string {
    const path = pathOf(file);
    return this.adapter.getResourcePath(path);
  }
}
