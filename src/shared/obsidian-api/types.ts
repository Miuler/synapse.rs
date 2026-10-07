import type { VaultNote, VaultRepository } from '@shared/repositories';

export interface VaultFile {
  path: string;
  name: string;
  basename: string;
  extension: string;
  parent?: string;
  stat: {
    ctime: number | null;
    mtime: number;
    size: number;
  };
}

export interface TFileLike extends VaultFile {
  vault: SynapseVaultLike;
}

export interface CachedMetadataLike {
  frontmatter?: Record<string, unknown>;
  headings: Array<{ heading: string; level: number; position: { start: number; end: number } }>;
  links: Array<{ link: string; original: string; position: { start: number; end: number } }>;
  embeds: Array<{ link: string; original: string; position: { start: number; end: number } }>;
  tags: string[];
}

export interface SynapseVaultLike {
  getFiles(): Promise<VaultFile[]>;
  getMarkdownFiles(): Promise<VaultFile[]>;
  read(file: TFileLike | string): Promise<string>;
  cachedRead(file: TFileLike | string): Promise<string>;
  modify(file: TFileLike | string, data: string): Promise<void>;
  exists(path: string): Promise<boolean>;
  getResourcePath(file: TFileLike | string): string;
}

export type { VaultNote, VaultRepository };
