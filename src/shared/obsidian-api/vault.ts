import type { VaultFile as RepositoryVaultFile, VaultRepository } from '@shared/repositories';
import type { SynapseVaultLike, TFileLike } from './types';

function pathOf(file: TFileLike | string): string {
  return typeof file === 'string' ? file : file.path;
}

function fileFromMetadata(file: RepositoryVaultFile, vault: SynapseVaultLike): TFileLike {
  const path = file.path;
  const name = file.name || path.split('/').pop() || path;
  const dot = name.lastIndexOf('.');
  return {
    path,
    name,
    basename: file.basename || (dot > 0 ? name.slice(0, dot) : name),
    extension: file.extension || (dot > 0 ? name.slice(dot + 1) : ''),
    parent: file.parent ?? (path.includes('/') ? path.slice(0, path.lastIndexOf('/')) : undefined),
    stat: file.stat,
    view_mode: file.view_mode,
    vault,
  };
}

export class SynapseVault implements SynapseVaultLike {
  constructor(private readonly repository: VaultRepository) {}

  /**
   * Consulta puntual O(1) directamente al índice DashMap de Rust.
   */
  async getAbstractFileByPath(path: string): Promise<TFileLike | null> {
    if (!path) return null;
    const cleanPath = path.replace(/^\/+/, '');
    const meta = await this.repository.getNoteMeta(cleanPath);
    if (!meta) return null;
    return fileFromMetadata(meta, this);
  }

  /**
   * Retorna todos los archivos presentes en el índice de la bóveda
   * recibidos por streaming sin asignaciones intermedias de Vec en Rust.
   */
  async getFiles(): Promise<TFileLike[]> {
    const files = await this.repository.getVaultFiles();
    return files.map((file) => fileFromMetadata(file, this));
  }

  /**
   * Retorna únicamente los archivos Markdown (.md, .markdown).
   */
  async getMarkdownFiles(): Promise<TFileLike[]> {
    const files = await this.getFiles();
    return files.filter((file) => /\.(md|markdown)$/i.test(file.path));
  }

  /**
   * Lee el contenido del archivo bajo demanda desde el disco duro.
   */
  async read(file: TFileLike | string): Promise<string> {
    const path = pathOf(file);
    const note = await this.repository.readNote(path);
    if (!note) throw new Error(`Archivo no encontrado: ${path}`);
    return note.content ?? '';
  }

  /**
   * En Synapse, cachedRead delega a la lectura del repositorio / caché.
   */
  cachedRead(file: TFileLike | string): Promise<string> {
    return this.read(file);
  }

  /**
   * Modifica el contenido de un archivo existente y actualiza DashMap.
   */
  async modify(file: TFileLike | string, data: string): Promise<void> {
    const path = pathOf(file);
    const current = await this.repository.readNote(path);
    const name = path.split('/').pop() || path;
    const dot = name.lastIndexOf('.');
    const title = current?.title || (dot > 0 ? name.slice(0, dot) : name);

    await this.repository.saveNote({
      relativePath: path,
      title,
      content: data,
      encoding: current?.encoding,
    });
  }

  /**
   * Crea un nuevo archivo con el contenido especificado.
   */
  async create(path: string, data: string): Promise<TFileLike> {
    const cleanPath = path.replace(/^\/+/, '');
    const name = cleanPath.split('/').pop() || cleanPath;
    const dot = name.lastIndexOf('.');
    const title = dot > 0 ? name.slice(0, dot) : name;

    await this.repository.saveNote({
      relativePath: cleanPath,
      title,
      content: data,
    });

    const meta = await this.getAbstractFileByPath(cleanPath);
    if (meta) return meta;

    return {
      path: cleanPath,
      name,
      basename: title,
      extension: dot > 0 ? name.slice(dot + 1) : '',
      parent: cleanPath.includes('/') ? cleanPath.slice(0, cleanPath.lastIndexOf('/')) : undefined,
      stat: { ctime: Date.now(), mtime: Date.now(), size: data.length },
      vault: this,
    };
  }

  /**
   * Elimina un archivo de la bóveda.
   */
  async delete(file: TFileLike | string): Promise<void> {
    const path = pathOf(file);
    await this.repository.deleteItem(path);
  }

  /**
   * Renombra o mueve un archivo dentro de la bóveda.
   */
  async rename(file: TFileLike | string, newPath: string): Promise<void> {
    const path = pathOf(file);
    const newName = newPath.split('/').pop() || newPath;
    await this.repository.renameItem(path, newName);
  }

  /**
   * Copia un archivo a un nuevo destino.
   */
  async copy(file: TFileLike | string, newPath: string): Promise<TFileLike> {
    const path = pathOf(file);
    const destDir = newPath.includes('/') ? newPath.slice(0, newPath.lastIndexOf('/')) : '';
    const results = await this.repository.copyItems([path], destDir);
    const copiedPath = results[0] || newPath;
    const meta = await this.getAbstractFileByPath(copiedPath);
    if (meta) return meta;

    const name = copiedPath.split('/').pop() || copiedPath;
    const dot = name.lastIndexOf('.');
    return {
      path: copiedPath,
      name,
      basename: dot > 0 ? name.slice(0, dot) : name,
      extension: dot > 0 ? name.slice(dot + 1) : '',
      parent: destDir || undefined,
      stat: { ctime: Date.now(), mtime: Date.now(), size: 0 },
      vault: this,
    };
  }

  /**
   * Comprueba en O(1) si un archivo existe consultando el DashMap de Rust.
   */
  async exists(path: string): Promise<boolean> {
    const meta = await this.getAbstractFileByPath(path);
    return meta !== null;
  }

  /**
   * Obtiene la URL local de Tauri para renderizar un recurso en el WebView.
   */
  getResourcePath(file: TFileLike | string): string {
    const path = pathOf(file);
    return this.repository.resolveAssetUrl(path);
  }
}
