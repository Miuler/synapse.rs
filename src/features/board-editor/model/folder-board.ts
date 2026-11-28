// El formato de carpeta (`format: 2`): `Nombre.board.md` define el tablero y cada tarea tiene su
// archivo. Rust lee y escribe los archivos; aquí se convierten al documento que usan las vistas y se
// guarda solo lo que cambió.
import { fingerprint, mergeActivity } from './activity';
import { defaultColumns, type BoardColumn, type BoardDocument, type BoardTask, type BoardViewMode } from './board.svelte';
import type { IssueLink, IssueSource } from './issue-sync';

export interface ActivityEntry {
  at: string;
  kind: 'comment' | 'history';
  author: string | null;
  id: string | null;
  edited: boolean;
  text: string;
}

export type TaskSync = IssueLink;

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
  const columns = board.columns.length ? board.columns : defaultColumns();
  const links: Record<string, IssueLink> = {};
  const list = tasks.map((card) => {
    if (card.sync) {
      links[card.id] = card.sync;
    }
    const task: BoardTask = {
      id: card.id,
      title: card.title,
      status: card.status || columns[0].id,
      start: card.start || new Date().toLocaleDateString('sv-SE'),
      duration: card.duration,
      dependsOn: card.dependsOn,
      progress: card.progress,
      comments: card.comments.map((c) => ({
        timestamp: c.at,
        text: c.text,
        ...(c.id && { remote: c.id }),
        ...(c.author && { author: c.author }),
        ...(c.edited && { edited: true }),
      })),
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
  const key = (c: BoardTask['comments'][number]) => `${c.timestamp}\n${c.text}`;
  const known = new Map((before?.task.comments ?? []).map((c) => [key(c), c]));
  const added = after.task.comments.filter((c) => !known.has(key(c)));
  // Los recién enviados ya tienen id en el proveedor, y los editados dejan de diferir
  for (const c of after.task.comments) {
    const old = known.get(key(c));
    if (!c.remote || !old || (old.remote && !(old.edited && !c.edited))) continue;
    const at = next.activity.findIndex(
      (e) => e.kind === 'comment' && (!e.id || e.id === c.remote) && e.at === c.timestamp && e.text === c.text,
    );
    if (at >= 0) next.activity = next.activity.with(at, { ...next.activity[at], id: c.remote, edited: false });
  }
  if (added.length) {
    next.activity = [
      ...next.activity,
      ...added.map((c) => ({ at: c.timestamp, kind: 'comment' as const, author: null, id: c.remote ?? null, edited: false, text: c.text })),
    ];
  }
  const incoming = after.task.incoming;
  if (incoming && incoming !== before?.task.incoming) {
    next.activity = mergeActivity(next.activity, incoming.entries, after.link?.sent);
    // Solo si la de aquí sigue siendo la que se iba a sustituir
    if (incoming.description !== undefined && fingerprint(next.description) === incoming.over) next.description = incoming.description;
  }
  if (!before || !same(before.link, after.link)) next.sync = after.link ? { rev: next.sync?.rev, ...after.link } : null;
  return next;
}

/** Lo que se iba a cambiar ya no está como se veía. */
const changedMeanwhile = (detail: string) => Object.assign(new Error(detail), { kind: 'conflict', detail });

/** La descripción nueva, si la del archivo sigue siendo la que se estaba editando. */
export function editDescription(file: TaskFile, previous: string, text: string): TaskFile {
  if (file.description !== previous) throw changedMeanwhile('description');
  // Se anota de cuál se partió: así traer distingue que aquí cambió
  const sync = file.sync && { ...file.sync, description: file.sync.description ?? fingerprint(previous) };
  return { ...file, description: text, sync };
}

/** Si la descripción de aquí no es la última que el proveedor y el tablero tenían en común. */
export function ownDescription({ sync, description }: TaskFile): boolean {
  if (!sync) return false;
  return sync.description === undefined ? !!description : fingerprint(description) !== sync.description;
}

/** Marcada para que el próximo «Traer» ponga la descripción del proveedor en lugar de la de aquí. */
export function takeProviderDescription(file: TaskFile, previous: string): TaskFile {
  if (file.description !== previous || !file.sync) throw changedMeanwhile('description');
  // Como si aquí no hubiera cambiado y su actividad estuviera sin leer
  const { rev: _, ...sync } = file.sync;
  return { ...file, sync: { ...sync, description: fingerprint(previous) } };
}

/** Si está pendiente de que «Traer» la sustituya. */
export const awaitsProviderDescription = ({ sync, description }: TaskFile) =>
  !!sync && !sync.rev && sync.description === fingerprint(description);

export type EntryRef = Pick<ActivityEntry, 'kind' | 'id' | 'at' | 'text'>;

/**
 * El texto nuevo de un comentario o de una entrada del histórico. Lo que también está en el
 * proveedor queda marcado como editado: traer no lo pisa.
 */
export function editEntry(file: TaskFile, entry: EntryRef, text: string): TaskFile {
  const at = file.activity.findIndex(
    (e) => e.kind === entry.kind && e.id === entry.id && e.at === entry.at && e.text === entry.text,
  );
  if (at < 0) throw changedMeanwhile(entry.kind);
  const old = file.activity[at];
  const edited = old.edited || !!old.id || old.kind === 'history';
  return { ...file, activity: file.activity.with(at, { ...old, text, edited }) };
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
