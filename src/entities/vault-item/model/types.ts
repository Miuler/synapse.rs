export type VaultItemKind = 'markdown' | 'mermaid' | 'excalidraw' | 'image' | 'pdf' | 'graph' | 'other';

export type VaultSyncState = 'local' | 'ghost' | 'cached' | 'pinned';

export interface GraphNode {
  id: string;
  label: string;
  group: string;
  val: number;
  is_ghost?: boolean;
}

export interface GraphEdge {
  source: string;
  target: string;
}

export interface VaultGraphData {
  nodes: GraphNode[];
  links: GraphEdge[];
}

export interface TagCountItem {
  tag: string;
  count: number;
}

export interface BacklinkItem {
  source_path: string;
  source_title: string;
  snippet: string;
}

export interface OutlinkItem {
  target: string;
  alias?: string | null;
  resolved_path?: string | null;
  is_embed: boolean;
}

export interface UnlinkedMentionItem {
  source_path: string;
  source_title: string;
  snippet: string;
}

export interface NoteBacklinksReport {
  target_path: string;
  backlinks: BacklinkItem[];
  outlinks: OutlinkItem[];
  unlinked_mentions: UnlinkedMentionItem[];
}

export interface FolderMeta {
  relative_path: string;
  name: string;
  parent?: string | null;
  notes_count: number;
  subfolders_count: number;
  dominant_tags: string[];
  sample_notes: string[];
}

export interface QuickOpenSearchResult {
  results: Array<{
    text: string;
    score: number;
    match_indices: number[];
    note_path?: string;
    is_recent?: boolean;
    last_opened_nanos?: number;
    tags?: string[];
  }>;
  folders?: FolderMeta[];
  total_files: number;
  matched_files: number;
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
