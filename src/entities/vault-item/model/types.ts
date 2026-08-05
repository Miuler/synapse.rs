export type VaultItemKind = 'markdown' | 'mermaid' | 'excalidraw' | 'image' | 'pdf' | 'other';

export interface VaultItem {
  id: string;
  title: string;
  content: string;
  relative_path: string;
  kind?: VaultItemKind;
}

export interface TabItem {
  path: string;
  title: string;
  isDirty?: boolean;
}
