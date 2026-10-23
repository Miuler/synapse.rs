import type { VaultRepository } from '@shared/repositories';
import type { DataAdapter, DataWriteOptions, ListedFiles, Stat } from './types';

export type AdapterMutationCallback = (
  type: 'write' | 'writeBinary' | 'remove' | 'rename',
  path: string,
  extra?: any
) => void;

export class SynapseDataAdapter implements DataAdapter {
  constructor(
    private readonly repository: VaultRepository,
    private readonly onMutation?: AdapterMutationCallback
  ) {}

  getName(): string {
    return 'SynapseDataAdapter';
  }

  async exists(normalizedPath: string, _sensitive?: boolean): Promise<boolean> {
    const clean = normalizedPath.replace(/^\/+|\/+$/g, '');
    if (!clean) return true; // la raíz siempre existe

    // 1. ¿Es un archivo en el DashMap?
    const meta = await this.repository.getNoteMeta(clean);
    if (meta) return true;

    // 2. ¿Es una carpeta? (comprobamos en sus hijos o en su padre)
    const children = await this.repository.getDirectoryChildren(clean);
    if (children.length > 0) return true;

    const parent = clean.includes('/') ? clean.slice(0, clean.lastIndexOf('/')) : '';
    const parentChildren = await this.repository.getDirectoryChildren(parent || undefined);
    return parentChildren.some((c) => c.relative_path === clean);
  }

  async stat(normalizedPath: string): Promise<Stat | null> {
    const clean = normalizedPath.replace(/^\/+|\/+$/g, '');
    if (!clean) {
      return { type: 'folder', ctime: 0, mtime: 0, size: 0 };
    }
    const meta = await this.repository.getNoteMeta(clean);
    if (meta) {
      return {
        type: 'file',
        ctime: meta.stat.ctime ?? 0,
        mtime: meta.stat.mtime,
        size: meta.stat.size,
      };
    }
    if (await this.exists(clean)) {
      return { type: 'folder', ctime: 0, mtime: 0, size: 0 };
    }
    return null;
  }

  async list(normalizedPath: string): Promise<ListedFiles> {
    const clean = normalizedPath.replace(/^\/+/, '');
    const children = await this.repository.getDirectoryChildren(clean || undefined);
    const files: string[] = [];
    const folders: string[] = [];

    for (const child of children) {
      if (child.is_folder) {
        folders.push(child.relative_path);
      } else {
        files.push(child.relative_path);
      }
    }

    return { files, folders };
  }

  async read(normalizedPath: string): Promise<string> {
    const clean = normalizedPath.replace(/^\/+/, '');
    const note = await this.repository.readNote(clean);
    if (!note) throw new Error(`Archivo no encontrado: ${clean}`);
    return note.content ?? '';
  }

  async readBinary(normalizedPath: string): Promise<ArrayBuffer> {
    const clean = normalizedPath.replace(/^\/+/, '');
    return this.repository.readBinary(clean);
  }

  async write(normalizedPath: string, data: string, _options?: DataWriteOptions): Promise<void> {
    const clean = normalizedPath.replace(/^\/+/, '');
    const current = await this.repository.readNote(clean);
    const name = clean.split('/').pop() || clean;
    const dot = name.lastIndexOf('.');
    const title = current?.title || (dot > 0 ? name.slice(0, dot) : name);

    await this.repository.saveNote({
      relativePath: clean,
      title,
      content: data,
      encoding: current?.encoding,
    });
    this.onMutation?.('write', clean, data);
  }

  async writeBinary(
    normalizedPath: string,
    data: ArrayBuffer | Uint8Array,
    _options?: DataWriteOptions
  ): Promise<void> {
    const clean = normalizedPath.replace(/^\/+/, '');
    await this.repository.writeBinary(clean, data);
    this.onMutation?.('writeBinary', clean, data);
  }

  async append(normalizedPath: string, data: string, options?: DataWriteOptions): Promise<void> {
    const existing = await this.read(normalizedPath).catch(() => '');
    await this.write(normalizedPath, existing + data, options);
  }

  async process(
    normalizedPath: string,
    fn: (data: string) => string,
    options?: DataWriteOptions
  ): Promise<string> {
    const existing = await this.read(normalizedPath);
    const processed = fn(existing);
    await this.write(normalizedPath, processed, options);
    return processed;
  }

  getResourcePath(normalizedPath: string): string {
    const clean = normalizedPath.replace(/^\/+/, '');
    return this.repository.resolveAssetUrl(clean);
  }

  async mkdir(normalizedPath: string): Promise<void> {
    const clean = normalizedPath.replace(/^\/+/, '');
    if (!clean) return;
    await this.repository.createFolder(clean);
  }

  async trashSystem(normalizedPath: string): Promise<boolean> {
    const clean = normalizedPath.replace(/^\/+/, '');
    await this.repository.deleteItem(clean);
    this.onMutation?.('remove', clean);
    return true;
  }

  async trashLocal(normalizedPath: string): Promise<void> {
    const clean = normalizedPath.replace(/^\/+/, '');
    await this.repository.deleteItem(clean);
    this.onMutation?.('remove', clean);
  }

  async rmdir(normalizedPath: string, _recursive: boolean): Promise<void> {
    const clean = normalizedPath.replace(/^\/+/, '');
    await this.repository.deleteItem(clean);
    this.onMutation?.('remove', clean);
  }

  async remove(normalizedPath: string): Promise<void> {
    const clean = normalizedPath.replace(/^\/+/, '');
    await this.repository.deleteItem(clean);
    this.onMutation?.('remove', clean);
  }

  async rename(normalizedPath: string, normalizedNewPath: string): Promise<void> {
    const clean = normalizedPath.replace(/^\/+/, '');
    const cleanNew = normalizedNewPath.replace(/^\/+/, '');
    const newName = cleanNew.split('/').pop() || cleanNew;
    await this.repository.renameItem(clean, newName);
    this.onMutation?.('rename', cleanNew, clean);
  }

  async copy(normalizedPath: string, normalizedNewPath: string): Promise<void> {
    const clean = normalizedPath.replace(/^\/+/, '');
    const cleanNew = normalizedNewPath.replace(/^\/+/, '');
    const destDir = cleanNew.includes('/') ? cleanNew.slice(0, cleanNew.lastIndexOf('/')) : '';
    await this.repository.copyItems([clean], destDir);
  }

  async getBasePath(): Promise<string | null> {
    return this.repository.getActiveVaultPath();
  }
}
