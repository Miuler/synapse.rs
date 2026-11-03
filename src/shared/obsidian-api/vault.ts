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
  /** A partir de cuántos archivos cambiados a la vez el vigilante trabaja en bloque. */
  static readonly BULK_CHANGE_THRESHOLD = 20;
  readonly adapter: DataAdapter;
  private readonly rootFolder: TFolder;
  private metadataCache?: SynapseMetadataCache;
  private readonly recentMutations: Map<string, number> = new Map();
  private unlistenWatcher?: () => void;
  private _cachedFiles: TFile[] = [];
  private _filesLoaded = false;

  // Índice síncrono de la bóveda (ruta → metadatos, sin contenido): espejo del DashMap de Rust
  // para que las consultas por ruta respondan sin cruzar el IPC de Tauri, como en Obsidian.
  private readonly index = new Map<string, RepositoryVaultFile>();
  private readonly folderPaths = new Set<string>();
  private indexLoaded = false;
  private readonly indexLoading: Promise<void>;

  /**
   * @param indexSource Origen de la lista de archivos para el índice síncrono. La app lo conecta a
   * la lista de la bóveda que ya cargó el explorador, para no pedir otra igual por IPC. Sin él, se
   * pide al backend.
   */
  constructor(
    private readonly repository: VaultRepository,
    private readonly indexSource?: () => Promise<RepositoryVaultFile[]>
  ) {
    super();
    this.rootFolder = new TFolder(this, '');

    // Adaptador de datos con propagación reactiva de mutaciones directas hacia el bus de eventos
    this.adapter = new SynapseDataAdapter(repository, (type, path, extra) => {
      this.handleAdapterMutation(type, path, extra);
    });

    // Iniciar escucha del watcher de cambios en disco si el repositorio lo soporta
    this.setupWatcher();
    this.indexLoading = this.loadIndex();
  }

  /**
   * Carga el índice de rutas de la bóveda. Se espera antes de cargar plugins para que las
   * consultas por ruta sean síncronas desde el primer momento.
   */
  private async loadIndex(): Promise<void> {
    if (!this.indexSource && typeof this.repository.getVaultFiles !== 'function') return;
    try {
      const files = this.indexSource ? await this.indexSource() : await this.repository.getVaultFiles();
      this.index.clear();
      this.folderPaths.clear();
      for (const file of files) this.upsertIndex(file);
      this._cachedFiles = files.map((file) => tFileFromMetadata(file, this));
      this._filesLoaded = true;
      this.indexLoaded = true;
    } catch (e) {
      console.warn('[SynapseVault] No se pudo cargar el índice de la bóveda:', e);
    }
  }

  /** Si el índice síncrono ya está cargado. */
  isIndexReady(): boolean {
    return this.indexLoaded;
  }

  /** Resuelve cuando el índice síncrono está listo (o no se pudo cargar). */
  whenIndexReady(): Promise<void> {
    return this.indexLoading;
  }

  private upsertIndex(file: RepositoryVaultFile): void {
    const path = pathOf(file.path);
    this.index.set(path, file);
    let dir = path;
    while (dir.includes('/')) {
      dir = dir.slice(0, dir.lastIndexOf('/'));
      this.folderPaths.add(dir);
    }
  }

  private removeFromIndex(path: string): void {
    this.index.delete(path);
    if (this.folderPaths.delete(path)) {
      const prefix = `${path}/`;
      for (const key of [...this.index.keys()]) if (key.startsWith(prefix)) this.index.delete(key);
      for (const dir of [...this.folderPaths]) if (dir.startsWith(prefix)) this.folderPaths.delete(dir);
    }
  }

  /** Registra en el índice un archivo creado localmente y luego toma los metadatos reales. */
  private indexLocalFile(file: TFile): void {
    if (!this.indexLoaded) return;
    const name = file.path.split('/').pop() ?? file.path;
    const dot = name.lastIndexOf('.');
    this.upsertIndex({
      path: file.path,
      name,
      basename: dot > 0 ? name.slice(0, dot) : name,
      extension: dot > 0 ? name.slice(dot + 1) : '',
      stat: file.stat,
    } as RepositoryVaultFile);
    this.refreshIndexEntry(file.path);
  }

  private refreshIndexEntry(path: string): void {
    if (!this.indexLoaded || typeof this.repository.getNoteMeta !== 'function') return;
    this.repository
      .getNoteMeta(path)
      .then((meta) => {
        if (meta) this.upsertIndex(meta);
      })
      .catch(() => {});
  }

  private lookupIndex(path: string): TAbstractFile | null {
    const meta = this.index.get(path);
    if (meta) return tFileFromMetadata(meta, this);
    if (!this.folderPaths.has(path)) return null;

    const folder = new TFolder(this, path);
    const prefix = `${path}/`;
    const children: TAbstractFile[] = [];
    for (const dir of this.folderPaths) {
      if (dir.startsWith(prefix) && !dir.slice(prefix.length).includes('/')) children.push(new TFolder(this, dir));
    }
    for (const filePath of this.index.keys()) {
      if (filePath.startsWith(prefix) && !filePath.slice(prefix.length).includes('/')) children.push(new TFile(this, filePath));
    }
    folder.children = children;
    return folder;
  }

  /**
   * Obtiene la configuración de una opción interna de la bóveda (ej. 'vimMode', 'theme').
   */
  getConfig(key: string): any {
    switch (key) {
      case 'vimMode':
        return false;
      case 'theme':
        return 'obsidian';
      case 'legacyEditor':
        return false;
      case 'livePreview':
        return true;
      default:
        return undefined;
    }
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
          this.removeFromIndex(clean);
          this.trigger('delete', file);
          this.metadataCache?.deleteCache(clean);
        }

        // 2. Archivos modificados o creados externamente
        const changed = paths.map((p) => pathOf(p)).filter((p) => !this.isRecentlyMutated(p));
        // Cambio masivo (escaneo inicial de la bóveda, git checkout...): una sola consulta en bloque
        // para el índice y sin releer cada nota; la caché de metadatos se calcula al pedirla.
        const bulk = changed.length > SynapseVault.BULK_CHANGE_THRESHOLD;
        if (bulk && this.indexLoaded && typeof this.repository.getVaultFiles === 'function') {
          const files = await this.repository.getVaultFiles().catch(() => null);
          if (files) for (const f of files) this.upsertIndex(f);
        }

        for (const clean of changed) {
          if (!bulk && this.indexLoaded && typeof this.repository.getNoteMeta === 'function') {
            const meta = await this.repository.getNoteMeta(clean).catch(() => null);
            if (meta) this.upsertIndex(meta);
          }
          const file = await this.getAbstractFileByPath(clean);
          if (file) {
            this.trigger('modify', file);

            // Si es markdown, sincronizar caché de metadatos (en cambios masivos, solo la que ya
            // estaba calculada)
            const cached = !!this.metadataCache?.getFileCacheSync(clean);
            if (file instanceof TFile && /\.(md|markdown)$/i.test(clean) && (!bulk || cached)) {
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
   * Busca un archivo o carpeta por ruta. Con el índice cargado es síncrono, como en Obsidian;
   * mientras se carga, consulta al índice DashMap de Rust y devuelve una promesa.
   */
  getAbstractFileByPath(path: string): TAbstractFile | null | Promise<TAbstractFile | null> {
    if (!path) return null;
    const cleanPath = path.replace(/^\/+/, '');
    if (!cleanPath) return this.rootFolder;
    if (this.indexLoaded) return this.lookupIndex(cleanPath);
    return this.fetchAbstractFile(cleanPath);
  }

  private async fetchAbstractFile(cleanPath: string): Promise<TAbstractFile | null> {

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
    const list: any = this.indexLoaded
      ? [...this.index.values()].map((file) => tFileFromMetadata(file, this))
      : [...this._cachedFiles];
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

    this.indexLocalFile(file);
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

    this.indexLocalFile(file);
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
    if (this.indexLoaded) this.folderPaths.add(cleanPath);
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
    this.removeFromIndex(path);

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
    this.removeFromIndex(path);

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
    if (this.indexLoaded) {
      const wasFolder = this.folderPaths.has(oldPath);
      const moved = this.index.get(oldPath);
      this.removeFromIndex(oldPath);
      if (moved) this.upsertIndex({ ...moved, path: cleanNewPath });
      if (wasFolder) {
        this.folderPaths.add(cleanNewPath);
        // Los hijos de la carpeta se reconstruyen desde Rust
        void this.loadIndex();
      }
      this.refreshIndexEntry(cleanNewPath);
    }

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
    if (copied instanceof TFile) this.indexLocalFile(copied);
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

  /**
   * Retorna el nombre de la bóveda actual (ej. nombre de la carpeta raíz).
   */
  getName(): string {
    return 'Synapse Vault';
  }

  /**
   * Itera recursivamente sobre los hijos de una carpeta invocando el callback para cada nodo.
   */
  static recurseChildren(folder: any, cb: (file: any) => any): void {
    if (!folder) return;
    if (Array.isArray(folder.children)) {
      for (const child of folder.children) {
        cb(child);
        if (child.children) {
          SynapseVault.recurseChildren(child, cb);
        }
      }
    }
  }

  /**
   * Versión de instancia de recurseChildren.
   */
  recurseChildren(folder: any, cb: (file: any) => any): void {
    SynapseVault.recurseChildren(folder, cb);
  }
}
