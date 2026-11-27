// El formato de carpeta (`format: 2`): `Nombre.board.md` define el tablero y cada tarea tiene su
// archivo. Rust lee y escribe los archivos; aquí se convierten al documento que usan las vistas y se
// guarda solo lo que cambió.
import { DEFAULT_COLUMNS, type BoardColumn, type BoardDocument, type BoardTask, type BoardViewMode } from './board.svelte';
import type { IssueLink, IssueSource } from './issue-sync';

export interface ActivityEntry {
  at: string;
  kind: 'comment' | 'history';
  author: string | null;
  id: string | null;
  edited: boolean;
  text: string;
}

export type TaskSync = IssueLink & { rev?: string };

interface TaskData {
  title: string;
  status: string;
  type: string | null;
  assignee: string | null;
  key: string | null;
  url: string | null;
  start: string;
  duration: number;
  dependsOn: string;
  progress: number;
  sync: TaskSync | null;
  extra: [string, string][];
}

export type TaskCard = TaskData & { id: string; hash: string; comments: ActivityEntry[] };

export type TaskFile = TaskData & {
  description: string;
  activity: ActivityEntry[];
  preamble: string;
  sections: string;
};

export interface BoardFile {
  defaultView: string;
  columns: BoardColumn[];
  columnWidths: Record<string, number>;
  source: Omit<IssueSource, 'links'> | null;
  fields: { card?: string[]; table?: string[] };
  extraFrontmatter: string;
  order: string[];
  body: string;
}

export interface BoardSnapshot {
  board: BoardFile;
  hash: string;
  tasks: TaskCard[];
  problems: { file: string; detail: string }[];
}

export interface BoardApi {
  read(path: string): Promise<BoardSnapshot>;
  write(path: string, board: BoardFile, hash: string): Promise<string>;
  readTask(path: string, id: string): Promise<{ task: TaskFile; hash: string }>;
  writeTask(path: string, id: string, task: TaskFile, hash: string | null): Promise<string>;
  deleteTask(path: string, id: string, hash: string): Promise<void>;
}

/** Si el contenido de un `.board.md` es el de un tablero de carpeta. */
export function isFolderBoard(content: string): boolean {
  const frontmatter = content.match(/^---\r?\n([\s\S]*?)\r?\n---/)?.[1] ?? '';
  return /^format:\s*2\s*$/m.test(frontmatter);
}

const VIEWS: BoardViewMode[] = ['table', 'kanban', 'gantt'];
/** Los datos de una tarea que el documento puede no tener. */
const OPTIONAL = ['type', 'assignee', 'key', 'url'] as const;
const REQUIRED = ['title', 'status', 'start', 'duration', 'dependsOn', 'progress'] as const;

export function folderDocument({ board, tasks }: BoardSnapshot): BoardDocument {
  const columns = board.columns.length ? board.columns : [...DEFAULT_COLUMNS];
  const links: Record<string, IssueLink> = {};
  const list = tasks.map((card) => {
    if (card.sync) {
      const { rev: _, ...link } = card.sync;
      links[card.id] = link;
    }
    const task: BoardTask = {
      id: card.id,
      title: card.title,
      status: card.status || columns[0].id,
      start: card.start || new Date().toLocaleDateString('sv-SE'),
      duration: card.duration,
      dependsOn: card.dependsOn,
      progress: card.progress,
      comments: card.comments.map((c) => ({ timestamp: c.at, text: c.text })),
    };
    for (const key of OPTIONAL) if (card[key]) task[key] = card[key]!;
    return task;
  });
  const defaultView = VIEWS.find((view) => view === board.defaultView) ?? 'kanban';
  return {
    defaultView,
    columns,
    columnWidths: board.columnWidths,
    fields: board.fields,
    tasks: list,
    source: board.source ? { ...board.source, connection: board.source.connection ?? '', links } : undefined,
  };
}

/** El archivo del tablero que corresponde a `doc`, conservando lo que el documento no tiene. */
export function boardFile(previous: BoardFile, doc: BoardDocument): BoardFile {
  let source: BoardFile['source'] = null;
  if (doc.source) {
    const { links: _, original: __, ...rest } = doc.source;
    source = rest;
  }
  return {
    ...previous,
    defaultView: doc.defaultView,
    columns: doc.columns,
    columnWidths: doc.columnWidths ?? {},
    source,
    order: doc.tasks.map((task) => task.id),
  };
}

const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);

export interface TaskChanges {
  created: string[];
  updated: string[];
  removed: string[];
}

export function taskChanges(before: BoardDocument, after: BoardDocument): TaskChanges {
  const old = new Map(before.tasks.map((task) => [task.id, task]));
  const changes: TaskChanges = { created: [], updated: [], removed: [] };
  for (const task of after.tasks) {
    const previous = old.get(task.id);
    if (!previous) changes.created.push(task.id);
    else if (!same(previous, task) || !same(before.source?.links?.[task.id], after.source?.links?.[task.id])) {
      changes.updated.push(task.id);
    }
    old.delete(task.id);
  }
  changes.removed = [...old.keys()];
  return changes;
}

const emptyTask = (): TaskFile => ({
  title: '',
  status: '',
  type: null,
  assignee: null,
  key: null,
  url: null,
  start: '',
  duration: 1,
  dependsOn: '-',
  progress: 0,
  sync: null,
  extra: [],
  description: '',
  activity: [],
  preamble: '',
  sections: '',
});

/**
 * Lleva a `file` lo que cambió entre `before` y `after`. Lo que no cambió en Synapse queda como esté
 * en el archivo, aunque se haya editado por fuera. Sin `before`, la tarea es nueva.
 */
export function applyTask(
  file: TaskFile | null,
  before: { task: BoardTask; link?: IssueLink } | null,
  after: { task: BoardTask; link?: IssueLink },
): TaskFile {
  const next: TaskFile = { ...(file ?? emptyTask()) };
  const changed = (key: keyof BoardTask) => !before || !same(before.task[key], after.task[key]);
  for (const key of REQUIRED) if (changed(key)) Object.assign(next, { [key]: after.task[key] });
  for (const key of OPTIONAL) if (changed(key)) next[key] = after.task[key] ?? null;
  const known = new Set((before?.task.comments ?? []).map((c) => `${c.timestamp}\n${c.text}`));
  const added = after.task.comments.filter((c) => !known.has(`${c.timestamp}\n${c.text}`));
  if (added.length) {
    next.activity = [
      ...next.activity,
      ...added.map((c) => ({ at: c.timestamp, kind: 'comment' as const, author: null, id: null, edited: false, text: c.text })),
    ];
  }
  if (!before || !same(before.link, after.link)) next.sync = after.link ? { rev: next.sync?.rev, ...after.link } : null;
  return next;
}

/** Guarda lo que cambió entre `before` y `after`; devuelve cómo quedó el archivo del tablero. */
export async function persist(
  api: BoardApi,
  path: string,
  file: { board: BoardFile; hash: string },
  before: BoardDocument,
  after: BoardDocument,
): Promise<{ board: BoardFile; hash: string }> {
  const changes = taskChanges(before, after);
  const state = (doc: BoardDocument, id: string) => ({ task: doc.tasks.find((t) => t.id === id)!, link: doc.source?.links?.[id] });
  // Las tareas antes que el orden: así el tablero nunca nombra un archivo que aún no existe
  for (const id of changes.created) {
    await api.writeTask(path, id, applyTask(null, null, state(after, id)), null);
  }
  for (const id of changes.updated) {
    const stored = await api.readTask(path, id);
    await api.writeTask(path, id, applyTask(stored.task, state(before, id), state(after, id)), stored.hash);
  }
  for (const id of changes.removed) {
    const stored = await api.readTask(path, id);
    await api.deleteTask(path, id, stored.hash);
  }
  const board = boardFile(file.board, after);
  if (same(board, boardFile(file.board, before))) return file;
  return { board, hash: await api.write(path, board, file.hash) };
}
