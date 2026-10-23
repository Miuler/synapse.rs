import type { VaultNote, VaultRepository, VaultFile as RepositoryVaultFile } from '@shared/repositories';

export interface VaultFileStat {
  ctime: number | null;
  mtime: number;
  size: number;
}

export interface TAbstractFileLike {
  path: string;
  name: string;
  parent?: string;
  vault: SynapseVaultLike;
}

export interface TFileLike extends TAbstractFileLike {
  basename: string;
  extension: string;
  stat: VaultFileStat;
  view_mode?: string;
}

export interface TFolderLike extends TAbstractFileLike {
  children: TAbstractFileLike[];
  isRoot(): boolean;
}

export type VaultFile = TFileLike;

export interface HeadingCache {
  heading: string;
  level: number;
  position: { start: number; end: number };
}

export interface LinkCache {
  link: string;
  original: string;
  displayText?: string;
  position: { start: number; end: number };
}

export interface EmbedCache {
  link: string;
  original: string;
  displayText?: string;
  position: { start: number; end: number };
}

export interface TagCache {
  tag: string;
  position?: { start: number; end: number };
}

export interface CachedMetadataLike {
  frontmatter?: Record<string, unknown>;
  headings: HeadingCache[];
  links: LinkCache[];
  embeds: EmbedCache[];
  tags: string[];
}

export interface SynapseVaultLike {
  getAbstractFileByPath(path: string): Promise<TFileLike | null>;
  getFiles(): Promise<TFileLike[]>;
  getMarkdownFiles(): Promise<TFileLike[]>;
  read(file: TFileLike | string): Promise<string>;
  cachedRead(file: TFileLike | string): Promise<string>;
  modify(file: TFileLike | string, data: string): Promise<void>;
  create(path: string, data: string): Promise<TFileLike>;
  delete(file: TFileLike | string): Promise<void>;
  rename(file: TFileLike | string, newPath: string): Promise<void>;
  copy(file: TFileLike | string, newPath: string): Promise<TFileLike>;
  exists(path: string): Promise<boolean>;
  getResourcePath(file: TFileLike | string): string;
}

export interface SynapseWorkspaceLike {
  getActiveFile(): Promise<TFileLike | null>;
  openLinkText(linktext: string, sourcePath: string): Promise<TFileLike | null>;
}

export interface SynapseMetadataCacheLike {
  getFileCache(file: TFileLike | string): Promise<CachedMetadataLike | null>;
  getFirstLinkpathDest(linkpath: string, sourcePath: string): Promise<TFileLike | null>;
  fileToLinktext(file: TFileLike | { path: string }, sourcePath?: string): string;
}

export interface SynapseFileManagerLike {
  processFrontMatter(
    file: TFileLike | string,
    fn: (frontmatter: Record<string, any>) => void
  ): Promise<void>;
  getAvailablePathForAttachment(filename: string, extension: string): Promise<string>;
  generateMarkdownLink(
    file: TFileLike | { path: string },
    sourcePath: string,
    subpath?: string,
    alias?: string
  ): string;
}

export type { VaultNote, VaultRepository, RepositoryVaultFile };
