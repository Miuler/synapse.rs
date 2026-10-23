import type { VaultFile as RepositoryVaultFile, VaultRepository } from '@shared/repositories';
import { SynapseDataAdapter } from './adapter';
import {
  TAbstractFile,
  TFile,
  TFolder,
  type DataAdapter,
  type SynapseVaultLike,
} from './types';

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

export class SynapseVault implements SynapseVaultLike {
  readonly adapter: DataAdapter;
  private readonly rootFolder: TFolder;

  constructor(private readonly repository: VaultRepository) {
    this.adapter = new SynapseDataAdapter(repository);
    this.rootFolder = new TFolder(this, '');
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
   */
  async getFiles(): Promise<TFile[]> {
    const files = await this.repository.getVaultFiles();
    return files.map((file) => tFileFromMetadata(file, this));
  }

  /**
   * Retorna únicamente los archivos Markdown (.md, .markdown) como instancias de TFile.
   */
  async getMarkdownFiles(): Promise<TFile[]> {
    const files = await this.getFiles();
    return files.filter((file) => /\.(md|markdown)$/i.test(file.path));
  }

  /**
   * Retorna todos los archivos y carpetas cargados en la bóveda.
   */
  async getAllLoadedFiles(): Promise<TAbstractFile[]> {
    const files = await this.getFiles();
    const folderPaths = new Set<string>();

    for (const file of files) {
      let p = file.path.includes('/') ? file.path.slice(0, file.path.lastIndexOf('/')) : '';
      while (p) {
        folderPaths.add(p);
        p = p.includes('/') ? p.slice(0, p.lastIndexOf('/')) : '';
      }
    }

    const folders = Array.from(folderPaths).map((p) => new TFolder(this, p));
    return [this.rootFolder, ...folders, ...files];
  }

  /**
   * Lee el contenido del archivo bajo demanda como texto UTF-8.
   */
  async read(file: TFile | string): Promise<string> {
    const path = pathOf(file);
    return this.adapter.read(path);
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
   * Modifica el contenido de texto de un archivo existente.
   */
  async modify(file: TFile | string, data: string): Promise<void> {
    const path = pathOf(file);
    await this.adapter.write(path, data);
  }

  /**
   * Modifica el contenido binario de un archivo existente.
   */
  async modifyBinary(file: TFile | string, data: ArrayBuffer | Uint8Array): Promise<void> {
    const path = pathOf(file);
    await this.adapter.writeBinary(path, data);
  }

  /**
   * Crea un nuevo archivo de texto y retorna su instancia TFile.
   */
  async create(path: string, data: string): Promise<TFile> {
    const cleanPath = pathOf(path);
    await this.adapter.write(cleanPath, data);

    const file = await this.getAbstractFileByPath(cleanPath);
    if (file instanceof TFile) return file;

    return new TFile(this, cleanPath, { ctime: Date.now(), mtime: Date.now(), size: data.length });
  }

  /**
   * Crea un nuevo archivo binario y retorna su instancia TFile.
   */
  async createBinary(path: string, data: ArrayBuffer | Uint8Array): Promise<TFile> {
    const cleanPath = pathOf(path);
    await this.adapter.writeBinary(cleanPath, data);

    const file = await this.getAbstractFileByPath(cleanPath);
    if (file instanceof TFile) return file;

    const byteLen = data instanceof ArrayBuffer ? data.byteLength : data.length;
    return new TFile(this, cleanPath, { ctime: Date.now(), mtime: Date.now(), size: byteLen });
  }

  /**
   * Crea una nueva carpeta en la bóveda y retorna su instancia TFolder.
   */
  async createFolder(path: string): Promise<TFolder> {
    const cleanPath = pathOf(path);
    await this.adapter.mkdir(cleanPath);
    return new TFolder(this, cleanPath);
  }

  /**
   * Elimina un archivo o carpeta de la bóveda.
   */
  async delete(file: TAbstractFile | string, _force?: boolean): Promise<void> {
    const path = pathOf(file);
    await this.adapter.remove(path);
  }

  /**
   * Envía un archivo o carpeta a la papelera.
   */
  async trash(file: TAbstractFile | string, system: boolean): Promise<void> {
    const path = pathOf(file);
    if (system) {
      await this.adapter.trashSystem(path);
    } else {
      await this.adapter.trashLocal(path);
    }
  }

  /**
   * Renombra o mueve un archivo o carpeta dentro de la bóveda.
   */
  async rename(file: TAbstractFile | string, newPath: string): Promise<void> {
    const oldPath = pathOf(file);
    const cleanNewPath = pathOf(newPath);
    await this.adapter.rename(oldPath, cleanNewPath);
  }

  /**
   * Copia un archivo a una nueva ruta de destino.
   */
  async copy(file: TFile | string, newPath: string): Promise<TFile> {
    const sourcePath = pathOf(file);
    const cleanNewPath = pathOf(newPath);
    await this.adapter.copy(sourcePath, cleanNewPath);

    const copied = await this.getAbstractFileByPath(cleanNewPath);
    if (copied instanceof TFile) return copied;

    return new TFile(this, cleanNewPath);
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
