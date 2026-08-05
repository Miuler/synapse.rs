export interface NoteItem {
  id: string;
  title: string;
  content: string;
  relative_path: string;
}

export interface TabItem {
  path: string;
  title: string;
  isDirty?: boolean;
}
