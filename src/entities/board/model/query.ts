// Lo que una nota pide de un tablero (el bloque `board` y el enlace a una tarea) y cómo se resuelve
// contra sus datos. Son funciones puras: usan lo que hay en los archivos del tablero, sin red.
import { parse as parseYaml } from 'yaml';

export const VIEWS = ['summary', 'list', 'table', 'kanban'] as const;
export const FILTERS = ['state', 'type', 'assignee', 'key'] as const;
export const GROUPS = ['state', 'type', 'assignee'] as const;
export const FIELDS = ['key', 'title', 'state', 'type', 'assignee', 'progress', 'start', 'duration', 'updated'] as const;
export const CATEGORIES = ['todo', 'doing', 'done', 'cancelled'] as const;
const KEYS = ['board', 'view', 'filter', 'group_by', 'sort', 'limit', 'fields'] as const;

export type ReferenceView = (typeof VIEWS)[number];
export type ReferenceField = (typeof FIELDS)[number];
export type Category = (typeof CATEGORIES)[number];
type Filter = (typeof FILTERS)[number];

export interface BoardQuery {
  board: string;
  view: ReferenceView;
  filter: Partial<Record<Filter, string[]>>;
  groupBy?: (typeof GROUPS)[number];
  /** Sin él, el orden del tablero. */
  sort?: { field: ReferenceField; descending: boolean };
  limit?: number;
  fields: ReferenceField[];
}

/** Por qué no se puede mostrar una referencia; el texto lo pone quien la pinta. */
export interface ReferenceError {
  kind:
    | 'yaml'
    | 'missing_board_key'
    | 'unknown_key'
    | 'unknown_filter'
    | 'bad_value'
    | 'missing_board'
    | 'ambiguous_board'
    | 'table_board'
    | 'unreadable'
    | 'missing_task';
  detail: string;
  /** Lo que sí se admite en ese sitio. */
  allowed?: readonly string[];
}

export type Result<T> = { ok: true; value: T } | { ok: false; error: ReferenceError };

const fail = (kind: ReferenceError['kind'], detail: string, allowed?: readonly string[]) =>
  ({ ok: false, error: { kind, detail, ...(allowed && { allowed }) } }) as const;

const isOne = <T extends string>(allowed: readonly T[], value: unknown): value is T =>
  allowed.includes(value as T);

/** Un valor o una lista de valores, como textos. */
function values(value: unknown): string[] | null {
  const list = Array.isArray(value) ? value : [value];
  if (!list.every((v) => typeof v === 'string' || typeof v === 'number')) return null;
  return list.map((v) => String(v).trim()).filter(Boolean);
}

/** El contenido de un bloque `board`. */
export function parseBoardBlock(source: string): Result<BoardQuery> {
  let raw: unknown;
  try {
    raw = parseYaml(source);
  } catch (error) {
    return fail('yaml', String(error instanceof Error ? error.message : error).split('\n')[0]);
  }
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return fail('missing_board_key', '');
  const block = raw as Record<string, unknown>;
  const unknown = Object.keys(block).find((key) => !isOne(KEYS, key));
  if (unknown !== undefined) return fail('unknown_key', unknown, KEYS);
  if (typeof block.board !== 'string' || !block.board.trim()) return fail('missing_board_key', '');

  const view = block.view ?? 'list';
  if (!isOne(VIEWS, view)) return fail('bad_value', `view: ${view}`, VIEWS);

  const filter: BoardQuery['filter'] = {};
  if (block.filter !== undefined && block.filter !== null) {
    if (typeof block.filter !== 'object' || Array.isArray(block.filter)) return fail('bad_value', 'filter', FILTERS);
    for (const [key, value] of Object.entries(block.filter)) {
      if (!isOne(FILTERS, key)) return fail('unknown_filter', key, FILTERS);
      const list = values(value);
      if (!list) return fail('bad_value', `filter.${key}`);
      if (list.length) filter[key] = list;
    }
  }

  const query: BoardQuery = {
    board: block.board.trim(),
    view,
    filter,
    fields: view === 'table' ? ['key', 'title', 'type', 'state', 'assignee'] : ['key', 'title', 'state'],
  };
  if (block.group_by !== undefined) {
    if (!isOne(GROUPS, block.group_by)) return fail('bad_value', `group_by: ${block.group_by}`, GROUPS);
    query.groupBy = block.group_by;
  }
  if (block.sort !== undefined) {
    const [field, direction = 'asc', ...rest] = String(block.sort).trim().split(/\s+/);
    if (!isOne(FIELDS, field) || !isOne(['asc', 'desc'], direction) || rest.length) {
      return fail('bad_value', `sort: ${block.sort}`, FIELDS);
    }
    query.sort = { field, descending: direction === 'desc' };
  }
  if (block.limit !== undefined) {
    if (!Number.isInteger(block.limit) || (block.limit as number) < 1) return fail('bad_value', `limit: ${block.limit}`);
    query.limit = block.limit as number;
  }
  if (block.fields !== undefined) {
    const list = values(block.fields);
    const wrong = list?.find((field) => !isOne(FIELDS, field));
    if (!list?.length || wrong !== undefined) return fail('bad_value', `fields: ${wrong ?? ''}`, FIELDS);
    query.fields = list as ReferenceField[];
  }
  return { ok: true, value: query };
}

/** `[[Equipo.board/ado-1|alias]]`: el tablero y la tarea que nombra un enlace; `null` si no es de una. */
export function taskReference(target: string): { board: string; task: string } | null {
  const path = target.split('|')[0].split('#')[0].trim();
  const found = path.match(/^(.*[^/]\.board)\/([A-Za-z0-9_-]+)(?:\.md)?$/);
  return found && found[2].toLowerCase() !== 'index' ? { board: found[1], task: found[2] } : null;
}

/** Los datos de un tablero que usa una referencia (los de `board_read`). */
export interface ReferencedBoard {
  board: {
    columns: { id: string; label: string; color: string }[];
    source: { synced_at?: string } | null;
  };
  tasks: {
    id: string;
    title: string;
    status: string;
    type: string | null;
    assignee: string | null;
    key: string | null;
    url: string | null;
    start: string;
    duration: number;
    progress: number;
    sync: { state: string; column: string; updated?: string } | null;
  }[];
}

export interface Row {
  id: string;
  key: string;
  title: string;
  /** El estado del proveedor mientras la tarea siga en su columna; si no, el nombre de la columna. */
  state: string;
  column: string;
  category: Category;
  type: string;
  assignee: string;
  progress: number;
  start: string;
  duration: number;
  updated: string;
  url: string | null;
}

const NAMES: Record<Category, string[]> = {
  todo: ['todo', 'to do', 'new', 'proposed', 'por hacer'],
  doing: ['doing', 'in progress', 'active', 'committed', 'resolved', 'en progreso', 'en curso'],
  done: ['done', 'closed', 'completed', 'hecho'],
  cancelled: ['cancelled', 'canceled', 'removed', 'cancelado'],
};
const named = (...texts: (string | undefined)[]) => {
  for (const text of texts) {
    const found = CATEGORIES.find((category) => NAMES[category].includes(text?.toLowerCase() ?? ''));
    if (found) return found;
  }
  return undefined;
};

/** Todas las tareas de un tablero, en su orden, con su estado y categoría. */
export function boardRows({ board, tasks }: ReferencedBoard): Row[] {
  const columns = board.columns;
  return tasks.map((task) => {
    const at = Math.max(0, columns.findIndex((c) => c.id === task.status));
    const column = columns[at];
    const remote = task.sync && (!task.status || task.sync.column === column?.id) ? task.sync.state : '';
    // Por el nombre de la columna o del estado; si no dicen nada, por su sitio en el tablero
    const category =
      named(column?.id, column?.label, remote) ??
      (at === 0 ? 'todo' : at === columns.length - 1 ? 'done' : 'doing');
    const label = column?.label ?? task.status;
    return {
      id: task.id,
      key: task.key || task.id,
      title: task.title,
      state: remote || label,
      column: column?.id ?? task.status,
      category,
      type: task.type ?? '',
      assignee: task.assignee ?? '',
      progress: task.progress,
      start: task.start,
      duration: task.duration,
      updated: task.sync?.updated ?? '',
      url: task.url,
    };
  });
}

export interface Summary {
  total: number;
  counts: Record<Category, number>;
  /** Porcentaje hecho de lo que no se canceló. */
  progress: number;
}

export function summarize(rows: Row[]): Summary {
  const counts = { todo: 0, doing: 0, done: 0, cancelled: 0 };
  for (const row of rows) counts[row.category]++;
  const live = rows.length - counts.cancelled;
  return { total: rows.length, counts, progress: live ? Math.round((counts.done / live) * 100) : 0 };
}

export interface Group {
  /** Vacío: sin agrupar, o las que no tienen ese dato. */
  label: string;
  rows: Row[];
}

export interface Resolved {
  view: ReferenceView;
  fields: ReferenceField[];
  groups: Group[];
  /** De las que cumplen el filtro, antes del límite. */
  summary: Summary;
  /** Cuándo se trajeron del proveedor; vacío, nunca. */
  syncedAt: string;
}

const matches = (row: Row, columnLabel: string, filter: BoardQuery['filter']) =>
  FILTERS.every((key) => {
    const wanted = filter[key]?.map((v) => v.toLowerCase());
    if (!wanted) return true;
    const has = {
      // Vale la categoría, la columna (id o nombre) o el estado del proveedor
      state: [row.category, row.column, columnLabel, row.state],
      type: [row.type],
      assignee: [row.assignee],
      key: [row.key, row.id],
    }[key];
    return has.some((value) => wanted.includes(value.toLowerCase()));
  });

function sorted(rows: Row[], sort: NonNullable<BoardQuery['sort']>, columns: string[]): Row[] {
  const value = (row: Row) => (sort.field === 'state' ? columns.indexOf(row.column) : row[sort.field]);
  const direction = sort.descending ? -1 : 1;
  return [...rows].sort((a, b) => {
    const [x, y] = [value(a), value(b)];
    // Lo vacío va al final en los dos sentidos
    if (x === '' || y === '') return Number(x === '') - Number(y === '');
    const result = typeof x === 'number' ? x - (y as number) : x.localeCompare(y as string, undefined, { numeric: true });
    return result * direction;
  });
}

/** Lo que muestra un bloque sobre los datos de su tablero. */
export function resolveQuery(data: ReferencedBoard, query: BoardQuery): Resolved {
  const { columns } = data.board;
  const label = (id: string) => columns.find((c) => c.id === id)?.label ?? id;
  const found = boardRows(data).filter((row) => matches(row, label(row.column), query.filter));
  const ordered = query.sort ? sorted(found, query.sort, columns.map((c) => c.id)) : found;
  const rows = query.limit ? ordered.slice(0, query.limit) : ordered;

  let groups: Group[];
  const by = query.view === 'kanban' ? 'state' : query.groupBy;
  if (!by) groups = [{ label: '', rows }];
  else if (by === 'state') {
    // En el orden de las columnas; el Kanban las enseña todas
    groups = columns.map((c) => ({ label: c.label, rows: rows.filter((row) => row.column === c.id) }));
    const loose = rows.filter((row) => !columns.some((c) => c.id === row.column));
    if (loose.length) groups.push({ label: '', rows: loose });
    if (query.view !== 'kanban') groups = groups.filter((group) => group.rows.length);
  } else {
    // Como en el filtro, las mayúsculas no distinguen; el grupo se llama como la primera que aparece
    const named = new Map<string, Group>();
    for (const row of rows) {
      const key = row[by].toLowerCase();
      if (!named.has(key)) named.set(key, { label: row[by], rows: [] });
      named.get(key)!.rows.push(row);
    }
    groups = [...named.values()].sort((a, b) => Number(!a.label) - Number(!b.label) || a.label.localeCompare(b.label));
  }
  return {
    view: query.view,
    fields: query.fields,
    groups,
    summary: summarize(found),
    syncedAt: data.board.source?.synced_at ?? '',
  };
}

/** La tarea que nombra un enlace. */
export function resolveTask(data: ReferencedBoard, id: string): Result<Row> {
  const found = boardRows(data).find((row) => row.id.toLowerCase() === id.toLowerCase());
  return found ? { ok: true, value: found } : fail('missing_task', id);
}
