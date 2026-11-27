import { parse as parseYaml, stringify as stringifyYaml } from 'yaml';
import type { ActivityEntry } from './folder-board';
import type { IssueSource } from './issue-sync';
import { LEGACY_KEY, readLegacySource, writeLegacySource } from './legacy-source';

export type BoardViewMode = 'table' | 'kanban' | 'gantt';

export interface BoardColumn {
  id: string;
  label: string;
  color: string;
}

export interface BoardComment {
  timestamp: string;
  text: string;
  /** Su id en el proveedor: ya está allí (solo en el formato de carpeta). */
  remote?: string;
  author?: string;
  /** Cambiado aquí: el proveedor sigue teniendo el original. */
  edited?: boolean;
}

export interface BoardTask {
  id: string;
  title: string;
  status: string;
  start: string; // YYYY-MM-DD
  duration: number; // días
  dependsOn: string; // id de otra tarea o '-'
  progress: number; // 0-100
  comments: BoardComment[];
  /** Solo los guarda el formato de carpeta. */
  type?: string;
  assignee?: string;
  /** Referencia en el proveedor (`AB#123`). */
  key?: string;
  url?: string;
  /** Lo recién traído del proveedor, hasta que se guarda en el archivo de la tarea. */
  incoming?: {
    entries: ActivityEntry[];
    description?: string;
    /** Huella de la descripción a la que sustituye. */
    over?: string;
  };
}

export interface BoardDocument {
  defaultView: BoardViewMode;
  columns: BoardColumn[];
  tasks: BoardTask[];
  /**
   * Ancho en píxeles de las columnas del Kanban que no usan el normal (clave `column_widths`). Va aparte
   * de `columns` para que una versión anterior, que solo entiende `id`, `label` y `color`, no las pierda.
   */
  columnWidths?: Record<string, number>;
  /** Sincronización con un gestor de issues. */
  source?: IssueSource;
  /** Qué datos se ven en las tarjetas y en la tabla (formato de carpeta); sin decirlo, los habituales. */
  fields?: { card?: string[]; table?: string[] };
  /** Bloques del frontmatter que el tablero no interpreta: se guardan tal cual. */
  extraFrontmatter?: string;
}

export const DEFAULT_COLUMNS: BoardColumn[] = [
  { id: 'todo', label: 'Por hacer', color: 'var(--text-muted)' },
  { id: 'doing', label: 'En progreso', color: 'var(--accent)' },
  { id: 'done', label: 'Hecho', color: '#22c55e' },
];

const OWN_KEYS = ['type', 'default_view', 'columns'];

export const COLUMN_WIDTH = { min: 180, max: 720 };
export const clampColumnWidth = (width: number) =>
  Math.round(Math.min(COLUMN_WIDTH.max, Math.max(COLUMN_WIDTH.min, width)));

function parseColumnWidths(value: object): Record<string, number> {
  const widths = Object.entries(value).filter(([, width]) => typeof width === 'number' && width > 0);
  return Object.fromEntries(widths.map(([id, width]) => [id, clampColumnWidth(width as number)]));
}

/** Separa lo que va en YAML libre (origen, anchos) y lo que no es del tablero, bloque a bloque. */
function parseOtherKeys(fmText: string): Pick<BoardDocument, 'source' | 'columnWidths' | 'extraFrontmatter'> {
  const blocks: { key: string; lines: string[] }[] = [];
  for (const line of fmText.split(/\r?\n/)) {
    const key = line.match(/^([^\s#-][^:]*):/)?.[1];
    if (key !== undefined || blocks.length === 0) blocks.push({ key: key ?? '', lines: [line] });
    else blocks[blocks.length - 1].lines.push(line);
  }

  let source: IssueSource | undefined;
  let columnWidths: BoardDocument['columnWidths'];
  const extra: string[] = [];
  for (const block of blocks) {
    if (OWN_KEYS.includes(block.key)) continue;
    const text = block.lines.join('\n');
    if (block.key === LEGACY_KEY || block.key === 'column_widths') {
      try {
        const value = parseYaml(text)?.[block.key];
        if (value && typeof value === 'object') {
          if (block.key === LEGACY_KEY) source = readLegacySource(value);
          else columnWidths = parseColumnWidths(value);
          continue;
        }
      } catch {
        // YAML a medio escribir: se conserva como texto
      }
    }
    extra.push(text);
  }
  return { source, columnWidths, extraFrontmatter: extra.join('\n') || undefined };
}

function parseFrontmatter(raw: string): {
  defaultView: BoardViewMode;
  columns: BoardColumn[];
  body: string;
  source?: IssueSource;
  columnWidths?: Record<string, number>;
  extraFrontmatter?: string;
} {
  const fmMatch = raw.match(/^---\r?\n([\s\S]*?)\r?\n---\r?\n?([\s\S]*)$/);
  if (!fmMatch) {
    return { defaultView: 'kanban', columns: [...DEFAULT_COLUMNS], body: raw };
  }

  const fmText = fmMatch[1];
  const body = fmMatch[2];

  let defaultView: BoardViewMode = 'kanban';
  const viewMatch = fmText.match(/^default_view:\s*(table|kanban|gantt)/m);
  if (viewMatch) {
    defaultView = viewMatch[1] as BoardViewMode;
  }

  const columns: BoardColumn[] = [];
  const colRegex = /-\s*\{\s*id:\s*([^,}]+),\s*label:\s*"([^"]+)",\s*color:\s*"([^"]+)"\s*\}/g;
  let m: RegExpExecArray | null;
  while ((m = colRegex.exec(fmText)) !== null) {
    columns.push({ id: m[1].trim(), label: m[2].trim(), color: m[3].trim() });
  }

  return {
    defaultView,
    columns: columns.length > 0 ? columns : [...DEFAULT_COLUMNS],
    body,
    ...parseOtherKeys(fmText),
  };
}

function parseCommentsSection(body: string): Map<string, BoardComment[]> {
  const map = new Map<string, BoardComment[]>();
  const idx = body.search(/^##\s+Comentarios\s*$/m);
  if (idx === -1) return map;

  const section = body.slice(idx);
  const lines = section.split(/\r?\n/);
  let currentId: string | null = null;

  for (const line of lines) {
    const h3 = line.match(/^###\s+(.+?)\s*$/);
    if (h3) {
      currentId = h3[1].trim();
      if (!map.has(currentId)) map.set(currentId, []);
      continue;
    }
    if (currentId) {
      const item = line.match(/^-\s+`([^`]+)`\s+[—-]\s+(.+)$/);
      if (item) {
        map.get(currentId)!.push({
          timestamp: item[1].trim(),
          text: item[2].trim(),
        });
      }
    }
  }
  return map;
}

export function parseBoardMarkdown(raw: string): BoardDocument {
  const { defaultView, columns, body, source, columnWidths, extraFrontmatter } = parseFrontmatter(raw);
  const commentsMap = parseCommentsSection(body);

  const tablePart = body.split(/^##\s+Comentarios\s*$/m)[0];
  const lines = tablePart.split(/\r?\n/).map((l) => l.trim()).filter((l) => l.startsWith('|'));

  const tasks: BoardTask[] = [];
  // Saltar cabecera (0) y separador (1)
  for (let i = 2; i < lines.length; i++) {
    const cells = lines[i]
      .split(/(?<!\\)\|/)
      .slice(1, -1)
      .map((c) => c.trim().replace(/\\\|/g, '|'));
    if (cells.length < 7) continue;

    const id = cells[0] || `t-${i - 1}`;
    const title = cells[1] || 'Sin título';
    const status = cells[2] || columns[0]?.id || 'todo';
    const start = /^\d{4}-\d{2}-\d{2}$/.test(cells[3]) ? cells[3] : new Date().toISOString().slice(0, 10);
    const durationMatch = cells[4].match(/(\d+)/);
    const duration = durationMatch ? Math.max(1, parseInt(durationMatch[1], 10)) : 1;
    const dependsOn = cells[5] && cells[5] !== '-' ? cells[5] : '-';
    const progressMatch = cells[6].match(/(\d+)/);
    const progress = progressMatch ? Math.min(100, Math.max(0, parseInt(progressMatch[1], 10))) : 0;

    tasks.push({
      id,
      title,
      status,
      start,
      duration,
      dependsOn,
      progress,
      comments: commentsMap.get(id) ?? [],
    });
  }

  return { defaultView, columns, tasks, source, columnWidths, extraFrontmatter };
}

export function serializeBoardMarkdown(doc: BoardDocument): string {
  const colsYaml = doc.columns
    .map((c) => `  - { id: ${c.id}, label: "${c.label.replace(/"/g, '\\"')}", color: "${c.color}" }`)
    .join('\n');

  const sourceYaml = doc.source && stringifyYaml(writeLegacySource(doc.source), { lineWidth: 0 }).trimEnd();

  const widthsYaml =
    Object.keys(doc.columnWidths ?? {}).length > 0 &&
    stringifyYaml({ column_widths: doc.columnWidths }).trimEnd();

  const frontmatter = [
    '---',
    'type: board',
    `default_view: ${doc.defaultView}`,
    'columns:',
    colsYaml,
    ...(widthsYaml ? [widthsYaml] : []),
    ...(sourceYaml ? [sourceYaml] : []),
    ...(doc.extraFrontmatter ? [doc.extraFrontmatter] : []),
    '---',
  ].join('\n');

  const tableHeader = [
    '| id | Tarea | Estado | Inicio | Duración | Depende | Progreso |',
    '|---|---|---|---|---|---|---|',
  ];

  const tableRows = doc.tasks.map((t) => {
    const safeTitle = t.title.replace(/\|/g, '\\|');
    const dep = t.dependsOn.trim() || '-';
    return `| ${t.id} | ${safeTitle} | ${t.status} | ${t.start} | ${t.duration}d | ${dep} | ${t.progress}% |`;
  });

  const commentBlocks: string[] = [];
  for (const t of doc.tasks) {
    if (t.comments.length === 0) continue;
    commentBlocks.push(`### ${t.id}`);
    for (const c of t.comments) {
      commentBlocks.push(`- \`${c.timestamp}\` — ${c.text}`);
    }
    commentBlocks.push('');
  }

  const parts = [frontmatter, '', ...tableHeader, ...tableRows];
  if (commentBlocks.length > 0) {
    parts.push('', '## Comentarios', '', ...commentBlocks);
  } else {
    parts.push('');
  }

  return parts.join('\n');
}

/**
 * Pone la tarea en la columna `status`, delante de `before`; sin `before`, al final de la columna.
 * El orden de las filas es el orden del tablero. Si la columna queda igual, devuelve el mismo array.
 */
export function placeTask(tasks: BoardTask[], id: string, status: string, before: string | null): BoardTask[] {
  const from = tasks.findIndex((t) => t.id === id);
  if (from === -1 || before === id) return tasks;
  const task = tasks[from];
  const rest = tasks.filter((_, i) => i !== from);
  let at = rest.findIndex((t) => t.id === before);
  if (at === -1) {
    const last = rest.map((t) => t.status).lastIndexOf(status);
    // En una columna vacía conserva su fila
    at = last === -1 ? from : last + 1;
  }
  rest.splice(at, 0, task.status === status ? task : { ...task, status });
  const order = (list: BoardTask[]) => list.filter((t) => t.status === status).map((t) => t.id).join('\n');
  return task.status === status && order(rest) === order(tasks) ? tasks : rest;
}

export type TaskSortKey = 'id' | 'type' | 'assignee' | 'title' | 'status' | 'start' | 'duration' | 'dependsOn' | 'progress';
export interface TaskSort {
  key: TaskSortKey;
  descending: boolean;
}

/** Copia ordenada para mirar; sin `sort`, el orden del archivo. El estado sigue el orden de las columnas. */
export function sortTasks(tasks: BoardTask[], sort: TaskSort | null, columns: BoardColumn[]): BoardTask[] {
  if (!sort) return tasks;
  const { key } = sort;
  const value = (t: BoardTask) =>
    key === 'status' ? columns.findIndex((c) => c.id === t.status) : key === 'dependsOn' && t[key] === '-' ? '' : (t[key] ?? '');
  const direction = sort.descending ? -1 : 1;
  return [...tasks].sort((a, b) => {
    const [x, y] = [value(a), value(b)];
    // Lo vacío va al final en los dos sentidos
    if (x === '' || y === '') return Number(x === '') - Number(y === '');
    // `numeric`: t2 va antes que t10
    const result =
      typeof x === 'number' ? x - (y as number) : x.localeCompare(y as string, undefined, { numeric: true });
    return result * direction;
  });
}

/** Un tablero nuevo, en el formato de carpeta y sin tareas: cada una tendrá su archivo. */
export function createDefaultBoardMarkdown(): string {
  const columns = DEFAULT_COLUMNS.map(
    (c) => `  - { id: ${JSON.stringify(c.id)}, label: ${JSON.stringify(c.label)}, color: ${JSON.stringify(c.color)} }`,
  );
  return ['---', 'type: board', 'format: 2', 'default_view: kanban', 'columns:', ...columns, '---', '', '## Order', ''].join('\n');
}
