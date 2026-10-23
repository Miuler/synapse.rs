import type { VaultNote, VaultRepository, VaultFile as RepositoryVaultFile } from '@shared/repositories';
import { Events, type EventRef } from './events';
import { Component } from './component';
import { View, ItemView, TextFileView, DefaultMarkdownView } from './view';
import { WorkspaceLeaf, type ViewState } from './leaf';
import { Plugin, type PluginManifest } from './plugin';
import {
  Setting,
  PluginSettingTab,
  SynapseSettingManager,
  TextComponent,
  TextAreaComponent,
  ToggleComponent,
  DropdownComponent,
  SliderComponent,
  ButtonComponent,
  ExtraButtonComponent,
  ColorComponent,
} from './setting';
import { type Command, SynapseCommandManager } from './command';
import { SynapseRibbonManager, SynapseStatusBarManager, type RibbonIconItem } from './ribbon';
import { Notice, Modal, Menu, MenuItem } from './ui';
import {
  normalizePath,
  parseYaml,
  stringifyYaml,
  requestUrl,
  Platform,
  moment,
  type MomentInstance,
} from './utils';
import { getObsidianBundle, setupObsidianGlobalEnvironment } from './bridge';
import { PluginLoader, type PluginEntry } from './loader';

export {
  Events,
  type EventRef,
  Component,
  View,
  ItemView,
  TextFileView,
  DefaultMarkdownView,
  WorkspaceLeaf,
  type ViewState,
  Plugin,
  type PluginManifest,
  Setting,
  PluginSettingTab,
  SynapseSettingManager,
  TextComponent,
  TextAreaComponent,
  ToggleComponent,
  DropdownComponent,
  SliderComponent,
  ButtonComponent,
  ExtraButtonComponent,
  ColorComponent,
  type Command,
  SynapseCommandManager,
  SynapseRibbonManager,
  SynapseStatusBarManager,
  type RibbonIconItem,
  Notice,
  Modal,
  Menu,
  MenuItem,
  normalizePath,
  parseYaml,
  stringifyYaml,
  requestUrl,
  Platform,
  moment,
  type MomentInstance,
  getObsidianBundle,
  setupObsidianGlobalEnvironment,
  PluginLoader,
  type PluginEntry,
};

export interface VaultFileStat {
  ctime: number | null;
  mtime: number;
  size: number;
}

export interface DataWriteOptions {
  ctime?: number;
  mtime?: number;
}

export interface Stat {
  type: 'file' | 'folder';
  ctime: number;
  mtime: number;
  size: number;
}

export interface ListedFiles {
  files: string[];
  folders: string[];
}

export abstract class TAbstractFile {
  vault: SynapseVaultLike;
  path: string;
  name: string;
  parent: TFolder | null = null;

  constructor(vault: SynapseVaultLike, path: string) {
    this.vault = vault;
    this.path = path.replace(/^\/+/, '');
    this.name = this.path.split('/').pop() || this.path;
  }
}

export class TFile extends TAbstractFile {
  stat: VaultFileStat;
  basename: string;
  extension: string;
  view_mode?: string;

  constructor(
    vault: SynapseVaultLike,
    path: string,
    stat?: VaultFileStat,
    view_mode?: string
  ) {
    super(vault, path);
    const dot = this.name.lastIndexOf('.');
    this.basename = dot > 0 ? this.name.slice(0, dot) : this.name;
    this.extension = dot > 0 ? this.name.slice(dot + 1) : '';
    this.stat = stat || { ctime: null, mtime: Date.now(), size: 0 };
    this.view_mode = view_mode;
  }
}

export class TFolder extends TAbstractFile {
  children: TAbstractFile[] = [];

  constructor(vault: SynapseVaultLike, path: string) {
    super(vault, path);
  }

  isRoot(): boolean {
    return this.path === '' || this.path === '/';
  }
}

export type TFileLike = TFile;
export type TFolderLike = TFolder;
export type TAbstractFileLike = TAbstractFile;
export type VaultFile = TFile;

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

export type CachedMetadata = CachedMetadataLike;

export interface DataAdapter {
  getName(): string;
  exists(normalizedPath: string, sensitive?: boolean): Promise<boolean>;
  stat(normalizedPath: string): Promise<Stat | null>;
  list(normalizedPath: string): Promise<ListedFiles>;
  read(normalizedPath: string): Promise<string>;
  readBinary(normalizedPath: string): Promise<ArrayBuffer>;
  write(normalizedPath: string, data: string, options?: DataWriteOptions): Promise<void>;
  writeBinary(normalizedPath: string, data: ArrayBuffer | Uint8Array, options?: DataWriteOptions): Promise<void>;
  append(normalizedPath: string, data: string, options?: DataWriteOptions): Promise<void>;
  process(normalizedPath: string, fn: (data: string) => string, options?: DataWriteOptions): Promise<string>;
  getResourcePath(normalizedPath: string): string;
  mkdir(normalizedPath: string): Promise<void>;
  trashSystem(normalizedPath: string): Promise<boolean>;
  trashLocal(normalizedPath: string): Promise<void>;
  rmdir(normalizedPath: string, recursive: boolean): Promise<void>;
  remove(normalizedPath: string): Promise<void>;
  rename(normalizedPath: string, normalizedNewPath: string): Promise<void>;
  copy(normalizedPath: string, normalizedNewPath: string): Promise<void>;
  getBasePath(): Promise<string | null>;
}

export interface SynapseVaultLike {
  adapter: DataAdapter;
  getAbstractFileByPath(path: string): Promise<TAbstractFile | null>;
  getRoot(): TFolder;
  getFiles(): Promise<TFile[]>;
  getMarkdownFiles(): Promise<TFile[]>;
  getAllLoadedFiles(): Promise<TAbstractFile[]>;
  read(file: TFile | string): Promise<string>;
  cachedRead(file: TFile | string): Promise<string>;
  readBinary(file: TFile | string): Promise<ArrayBuffer>;
  modify(file: TFile | string, data: string): Promise<void>;
  modifyBinary(file: TFile | string, data: ArrayBuffer | Uint8Array): Promise<void>;
  create(path: string, data: string): Promise<TFile>;
  createBinary(path: string, data: ArrayBuffer | Uint8Array): Promise<TFile>;
  createFolder(path: string): Promise<TFolder>;
  delete(file: TAbstractFile | string, force?: boolean): Promise<void>;
  trash(file: TAbstractFile | string, system: boolean): Promise<void>;
  rename(file: TAbstractFile | string, newPath: string): Promise<void>;
  copy(file: TFile | string, newPath: string): Promise<TFile>;
  exists(path: string): Promise<boolean>;
  getResourcePath(file: TFile | string): string;

  // Event methods
  on(name: 'create', callback: (file: TAbstractFile) => any, ctx?: any): EventRef;
  on(name: 'modify', callback: (file: TAbstractFile) => any, ctx?: any): EventRef;
  on(name: 'delete', callback: (file: TAbstractFile) => any, ctx?: any): EventRef;
  on(name: 'rename', callback: (file: TAbstractFile, oldPath: string) => any, ctx?: any): EventRef;
  on(name: 'closed', callback: () => any, ctx?: any): EventRef;
  on(name: 'raw', callback: (path: string) => any, ctx?: any): EventRef;
  on(name: string, callback: (...data: any[]) => any, ctx?: any): EventRef;
  off(name: string, callback: (...data: any[]) => any): void;
  offref(ref: EventRef): void;
  trigger(name: string, ...data: any[]): void;
}

export interface SynapseWorkspaceLike {
  activeLeaf: WorkspaceLeaf | null;
  getActiveFile(): Promise<TFile | null>;
  openLinkText(linktext: string, sourcePath: string, newLeaf?: boolean): Promise<TFile | null>;
  registerView(type: string, viewCreator: (leaf: WorkspaceLeaf) => View): void;
  registerExtensions(extensions: string[], viewType: string): void;
  getViewCreator(type: string): ((leaf: WorkspaceLeaf) => View) | undefined;
  getViewTypeForExtension(extension: string): string;
  getViewTypeForFile(file: TFile): string;
  getLeaf(newLeaf?: boolean | 'split' | 'tab' | 'window'): WorkspaceLeaf;
  createLeafBySplit(leaf: WorkspaceLeaf, direction?: 'horizontal' | 'vertical', before?: boolean): WorkspaceLeaf;
  setActiveLeaf(leaf: WorkspaceLeaf, params?: { focus?: boolean }): void;
  getLeavesOfType(type: string): WorkspaceLeaf[];
  getActiveViewOfType<T extends View>(type: new (...args: any[]) => T): T | null;
  getLayout(): any;

  // Event methods
  on(name: 'file-open', callback: (file: TFile | null) => any, ctx?: any): EventRef;
  on(name: 'active-leaf-change', callback: (leaf: WorkspaceLeaf | null) => any, ctx?: any): EventRef;
  on(name: 'layout-change', callback: () => any, ctx?: any): EventRef;
  on(name: 'quick-preview', callback: (file: TFile, data: string) => any, ctx?: any): EventRef;
  on(name: 'editor-change', callback: (editor: any, info: any) => any, ctx?: any): EventRef;
  on(name: 'resize', callback: () => any, ctx?: any): EventRef;
  on(name: string, callback: (...data: any[]) => any, ctx?: any): EventRef;
  off(name: string, callback: (...data: any[]) => any): void;
  offref(ref: EventRef): void;
  trigger(name: string, ...data: any[]): void;
}

export interface SynapseMetadataCacheLike {
  getFileCache(file: TFile | string): CachedMetadataLike | null | Promise<CachedMetadataLike | null>;
  getFirstLinkpathDest(linkpath: string, sourcePath: string): Promise<TFile | null>;
  fileToLinktext(file: TFile | { path: string }, sourcePath?: string): string;

  // Event methods
  on(name: 'changed', callback: (file: TFile, data: string, cache: CachedMetadataLike) => any, ctx?: any): EventRef;
  on(name: 'resolve', callback: (file: TFile) => any, ctx?: any): EventRef;
  on(name: 'resolved', callback: () => any, ctx?: any): EventRef;
  on(name: string, callback: (...data: any[]) => any, ctx?: any): EventRef;
  off(name: string, callback: (...data: any[]) => any): void;
  offref(ref: EventRef): void;
  trigger(name: string, ...data: any[]): void;
}

export interface SynapseFileManagerLike {
  processFrontMatter(
    file: TFile | string,
    fn: (frontmatter: Record<string, any>) => void
  ): Promise<void>;
  getAvailablePathForAttachment(filename: string, extension: string): Promise<string>;
  generateMarkdownLink(
    file: TFile | { path: string },
    sourcePath: string,
    subpath?: string,
    alias?: string
  ): string;
}

export type { VaultNote, VaultRepository, RepositoryVaultFile };
