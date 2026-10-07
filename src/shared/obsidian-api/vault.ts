import type { VaultFile as RepositoryVaultFile, VaultRepository } from '@shared/repositories';
import type { SynapseVaultLike, TFileLike, VaultFile } from './types';

function fileFromMetadata(file: RepositoryVaultFile, vault: SynapseVaultLike): TFileLike {
  const path = file.path;
  const name = path.split('/').pop() ?? path;
  const dot = name.lastIndexOf('.');
  return {
    path,
    name,
    basename: dot > 0 ? name.slice(0, dot) : name,
    extension: dot > 0 ? name.slice(dot + 1) : '',
    parent: path.includes('/') ? path.slice(0, path.lastIndexOf('/')) : undefined,
    stat: file.stat,
    vault,
  };
}

function pathOf(file: TFileLike | string): string {
  return typeof file === 'string' ? file : file.path;
}

export class SynapseVault implements SynapseVaultLike {
  constructor(private readonly repository: VaultRepository) {}

  async getFiles(): Promise<VaultFile[]> {
    const files = await this.repository.getVaultFiles();
    return files.map((file) => fileFromMetadata(file, this));
  }

  async getMarkdownFiles(): Promise<VaultFile[]> {
    const files = await this.getFiles();
    return files.filter((file) => /\.(md|markdown)$/i.test(file.path));
  }

  async read(file: TFileLike | string): Promise<string> {
    const note = await this.repository.readNote(pathOf(file));
    if (!note) throw new Error(`Archivo no encontrado: ${pathOf(file)}`);
    return note.content ?? '';
  }

  cachedRead(file: TFileLike | string): Promise<string> {
    return this.read(file);
  }

  async modify(file: TFileLike | string, data: string): Promise<void> {
    const path = pathOf(file);
    const current = await this.repository.readNote(path);
    await this.repository.saveNote({
      relativePath: path,
      title: current?.title ?? path.split('/').pop() ?? path,
      content: data,
      encoding: current?.encoding,
    });
  }

  async exists(path: string): Promise<boolean> {
    return (await this.repository.readNote(path)) !== null;
  }

  getResourcePath(file: TFileLike | string): string {
    const path = pathOf(file);
    const note = { relative_path: path };
    return this.repository.resolveAssetUrl(note.relative_path);
  }
}
