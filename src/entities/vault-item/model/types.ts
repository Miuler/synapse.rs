export type VaultItemKind = 'markdown' | 'mermaid' | 'excalidraw' | 'image' | 'pdf' | 'other';

export interface VaultItem {
  id: string;
  title: string;
  relative_path: string;
  abs_path?: string;
  kind?: VaultItemKind;
}

export interface TabItem {
  path: string;
  title: string;
  abs_path?: string;
  isDirty?: boolean;
}

export interface OpenedNote {
  relative_path: string;
  abs_path?: string;
  title: string;
  content: string;
  savedContent: string;
  encoding: string;
  isLoading: boolean;
}
