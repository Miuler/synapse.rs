export type VaultItemKind = 'markdown' | 'mermaid' | 'excalidraw' | 'image' | 'pdf' | 'other';

export type VaultSyncState = 'local' | 'ghost' | 'cached' | 'pinned';

export interface VaultItem {
  id: string;
  title: string;
  relative_path: string;
  abs_path?: string;
  kind?: VaultItemKind;
  view_mode?: string;
  sync_state?: VaultSyncState;
}

export interface TabItem {
  path: string;
  title: string;
  abs_path?: string;
  isDirty?: boolean;
  isLocked?: boolean;
  kind?: VaultItemKind;
  sync_state?: VaultSyncState;
}

export type MarkdownViewMode = 'live' | 'source' | 'reading';

export interface OpenedNote {
  relative_path: string;
  abs_path?: string;
  title: string;
  content: string;
  savedContent: string;
  encoding: string;
  isLoading: boolean;
  isLoaded?: boolean;
  viewMode?: MarkdownViewMode;
  sync_state?: VaultSyncState;
  lastReloaded?: number;
  excalidrawViewer?: 'native' | 'obsidian';
}
