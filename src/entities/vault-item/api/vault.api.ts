import { invokeTauri } from '@shared/api';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { convertFileSrc } from '@tauri-apps/api/core';

import type { NoteFrontmatter, VaultSyncState } from '../model/types';

export interface VaultNoteDto {
  relative_path: string;
  abs_path?: string;
  title: string;
  content: string;
  encoding?: string;
  view_mode?: string;
  sync_state?: VaultSyncState;
  frontmatter?: NoteFrontmatter;
}

export interface VaultFileStatDto {
  mtime_ms: number;
  size_bytes: number;
}

export interface VaultFileDto {
  path: string;
  title: string;
  stat?: VaultFileStatDto;
  last_opened_nanos?: number;
  view_mode?: string;
  sync_state?: VaultSyncState;
}

export interface VaultEntryNodeDto {
  name: string;
  relative_path: string;
  is_folder: boolean;
  title?: string;
  sync_state?: VaultSyncState;
}

export interface SaveNoteParams {
  relativePath: string;
  title: string;
  content: string;
  encoding?: string;
}

export interface SelectVaultFolderResult {
  folder_path: string;
  notes: VaultNoteDto[];
}

export const vaultApi = {
  async getNotes(): Promise<VaultNoteDto[]> {
    return await invokeTauri<VaultNoteDto[]>('get_vault_notes');
  },

  async getVaultFiles(): Promise<VaultFileDto[]> {
    return await invokeTauri<VaultFileDto[]>('get_vault_files');
  },

  async getNoteMeta(relativePath: string): Promise<VaultFileDto | null> {
    return await invokeTauri<VaultFileDto | null>('get_note_meta', { relativePath });
  },

  async getDirectoryChildren(parentPath?: string): Promise<VaultEntryNodeDto[]> {
    return await invokeTauri<VaultEntryNodeDto[]>('get_vault_directory_children', { parentPath });
  },

  async getVaultFilesCount(): Promise<number> {
    return await invokeTauri<number>('get_vault_files_count');
  },

  async getRecentNotes(limit = 15): Promise<string[]> {
    return await invokeTauri<string[]>('get_recent_notes_command', { limit });
  },

  async readNote(relativePath: string): Promise<VaultNoteDto | null> {
    return await invokeTauri<VaultNoteDto>('read_note_content', { relativePath });
  },

  async saveNote(params: SaveNoteParams): Promise<void> {
    await invokeTauri('save_note_content', {
      relativePath: params.relativePath,
      title: params.title,
      content: params.content,
      encoding: params.encoding,
    });
  },

  async recordNoteOpened(relativePath: string): Promise<void> {
    await invokeTauri('record_note_opened', { relativePath });
  },

  async setNoteViewMode(relativePath: string, viewMode: string): Promise<void> {
    await invokeTauri('set_note_view_mode', { relativePath, viewMode });
  },

  async deleteItem(relativePath: string): Promise<void> {
    await invokeTauri('delete_vault_item', { relativePath });
  },

  async renameItem(relativePath: string, newName: string): Promise<string> {
    return await invokeTauri<string>('rename_vault_item', { relativePath, newName });
  },

  async copyItems(paths: string[], destDir: string): Promise<string[]> {
    return await invokeTauri<string[]>('copy_vault_items', { paths, destDir });
  },

  async reloadVaultItems(paths: string[]): Promise<string[]> {
    return await invokeTauri<string[]>('reload_vault_items', { paths });
  },

  async createFolder(relativePath: string): Promise<void> {
    await invokeTauri('create_vault_folder', { relativePath });
  },

  async resolveAssetFilePath(src: string, baseFile?: string | null): Promise<string | null> {
    return await invokeTauri<string | null>('resolve_asset_file_path', { src, baseFile });
  },

  async readAssetDataUrl(src: string, baseFile?: string | null): Promise<string | null> {
    return await invokeTauri<string | null>('read_asset_data_url', { src, baseFile });
  },

  async saveBinaryAsset(relativePath: string, base64Data: string): Promise<void> {
    await invokeTauri('save_binary_asset', { relativePath, base64Data });
  },

  async readBinary(relativePath: string): Promise<ArrayBuffer> {
    const bytes = await invokeTauri<number[]>('read_binary_file', { relativePath });
    return new Uint8Array(bytes).buffer;
  },

  async writeBinary(relativePath: string, data: ArrayBuffer | Uint8Array): Promise<void> {
    const bytes = data instanceof Uint8Array ? Array.from(data) : Array.from(new Uint8Array(data));
    await invokeTauri('write_binary_file', { relativePath, data: bytes });
  },

  async resolveVaultLink(link: string, baseFile?: string | null): Promise<string | null> {
    return await invokeTauri<string | null>('resolve_vault_link', { link, baseFile });
  },

  async resolveVaultLinks(links: string[], baseFile?: string | null): Promise<Record<string, string>> {
    return await invokeTauri<Record<string, string>>('resolve_vault_links', { links, baseFile });
  },

  async renderMarkdownWikilinks(content: string, baseFile?: string | null): Promise<string> {
    return await invokeTauri<string>('render_markdown_wikilinks', { content, baseFile });
  },

  async onFilesChanged(callback: (payload: { paths: string[]; deleted: string[] }) => void): Promise<UnlistenFn> {
    return await listen<{ paths: string[]; deleted: string[] }>('vault:files-changed', (event) => {
      callback(event.payload);
    });
  },

  async getActiveVaultPath(): Promise<string | null> {
    return await invokeTauri<string | null>('get_active_vault_path');
  },

  async setActiveVaultPath(path: string): Promise<void> {
    await invokeTauri<void>('set_active_vault_path', { newPath: path });
  },

  async selectVaultFolder(startingDirectory?: string): Promise<SelectVaultFolderResult | null> {
    return await invokeTauri<SelectVaultFolderResult | null>('select_vault_folder', {
      startingDirectory: startingDirectory ?? null,
      starting_directory: startingDirectory ?? null,
    });
  },

  resolveAssetUrl(path: string): string {
    return convertFileSrc(path, 'asset');
  },
};
