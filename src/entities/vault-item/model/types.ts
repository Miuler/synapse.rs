export type VaultItemKind = 'markdown' | 'mermaid' | 'excalidraw' | 'image' | 'pdf' | 'graph' | 'other';

export type VaultSyncState = 'local' | 'ghost' | 'cached' | 'pinned';

export interface GraphNode {
  id: string;
  label: string;
  group: string;
  val: number;
}

export interface GraphEdge {
  source: string;
  target: string;
}

export interface VaultGraphData {
  nodes: GraphNode[];
  links: GraphEdge[];
}

export interface NoteRelations {
  up?: string;
  related?: string[];
  [key: string]: unknown;
}

export interface NoteFrontmatter {
  id?: string;
  title?: string;
  created?: string;
  updated?: string;
  type?: string;
  tags?: string[];
  aliases?: string[];
  relations?: NoteRelations;
  sync?: {
    state?: string;
    version?: number;
  };
  [key: string]: unknown;
}

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
  frontmatter?: NoteFrontmatter;
  lastReloaded?: number;
  excalidrawViewer?: 'native' | 'obsidian';
}
