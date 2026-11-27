// Sincronización de un tablero con un gestor de issues en dos acciones separadas: «Traer» y «Enviar».
// El tablero guarda, por tarea vinculada, lo último que ambos lados tenían en común (`links`): así se
// distingue qué cambió aquí y qué cambió allí, y no se repiten envíos. De qué proveedor son las
// tareas solo se sabe por lo que él declara.
import type { BoardDocument, BoardTask } from './board.svelte';

export interface IssueLink {
  /** Identificador en el proveedor. */
  id: string;
  /** Título y estado remotos de la última sincronización. */
  title: string;
  state: string;
  /** Columna en la que quedó la tarea en esa sincronización. */
  column: string;
  /** Fechas (`timestamp`) de los comentarios ya enviados. */
  sent?: string[];
}

/** De dónde vienen las tareas de un tablero. */
export interface IssueSource {
  provider: string;
  /** Id de la conexión; vacío, la que cubra el ámbito. */
  connection: string;
  /** Proyecto o repositorio, en la forma que entienda el proveedor. */
  scope: string;
  /** Consulta propia; vacía, la que el proveedor usa por defecto. */
  query?: string;
  /** Tipo de las tareas que se crean. */
  issue_type?: string;
  /** A quién se asignan las que se crean; vacío, a nadie. */
  assignee?: string;
  /** Estados de cada columna: el primero es el que se envía, todos valen al traer. */
  states?: Record<string, string[]>;
  links?: Record<string, IssueLink>;
  /** Lo leído del archivo, para devolverle lo que no se interpreta. */
  original?: Record<string, unknown>;
}

/** La parte del modelo común que el tablero usa. */
export interface Issue {
  id: string;
  type: string;
  title: string;
  /** `category` es la que declara el proveedor para ese estado. */
  state: { name: string; category?: 'todo' | 'doing' | 'done' | 'cancelled' };
  /** Versión leída: una escritura solo se admite sobre ella. */
  revision: string | null;
}

/** Lo que un proveedor declara de sí mismo. */
export interface IssueProvider {
  id: string;
  name: string;
  query_language: string;
  default_query: string;
  default_issue_type: string;
  default_states: Record<string, string[]>;
  /** Prefijo del identificador de las tareas que se traen (`prefijo-id`). */
  task_prefix: string;
  scope_example: string;
}

function statesOf(source: IssueSource, provider: IssueProvider, column: string): string[] {
  return source.states?.[column] ?? provider.default_states[column] ?? [];
}

/**
 * La columna de un estado remoto: la que lo tenga configurado en el tablero; si no, la de la categoría
 * que declara el proveedor; y, como respaldo, la que lo tenga entre sus nombres habituales.
 */
function columnOf(doc: BoardDocument, provider: IssueProvider, state: Issue['state']): string | undefined {
  const wanted = state.name.toLowerCase();
  const listing = (states: (column: string) => string[] | undefined) =>
    doc.columns.find((c) => states(c.id)?.some((s) => s.toLowerCase() === wanted))?.id;
  return (
    listing((column) => doc.source!.states?.[column]) ??
    doc.columns.find((c) => c.id === state.category)?.id ??
    listing((column) => statesOf(doc.source!, provider, column))
  );
}

/** Un título tal como queda en una celda de la tabla. */
const cellText = (title: string) => title.replace(/\s*[\r\n]+\s*/g, ' ').trim();

export interface PullResult {
  doc: BoardDocument;
  added: number;
  updated: number;
  /** Tareas con cambios distintos en los dos lados: se conserva lo local. */
  conflicts: string[];
  /** Tareas que el proveedor ya no devuelve: se desvinculan, la fila se queda. */
  missing: string[];
}

/**
 * Aplica al tablero lo traído del proveedor. `items` son las tareas de la consulta y las ya
 * vinculadas; `requested`, los ids por los que se preguntó (un vínculo creado mientras tanto no se toca).
 */
export function applyPull(
  doc: BoardDocument,
  provider: IssueProvider,
  items: Issue[],
  requested: string[],
  today: string,
): PullResult {
  const source = doc.source!;
  const byId = new Map(items.map((item) => [item.id, item]));
  const links: Record<string, IssueLink> = {};
  const conflicts: string[] = [];
  const missing: string[] = [];
  let updated = 0;

  const tasks = doc.tasks.map((task) => {
    const link = source.links?.[task.id];
    if (!link) return task;
    const item = byId.get(link.id);
    if (!item) {
      if (requested.includes(link.id)) missing.push(link.id);
      else links[task.id] = link;
      return task;
    }
    const next: BoardTask = { ...task };
    const base: IssueLink = { ...link };
    let conflict = false;

    const title = cellText(item.title);
    if (task.title === title) base.title = title;
    else if (title !== link.title) {
      if (task.title === link.title) next.title = base.title = title;
      else conflict = true;
    }

    const column = columnOf(doc, provider, item.state);
    if (item.state.name !== link.state) {
      if (column === task.status) {
        base.state = item.state.name;
        base.column = column;
      } else if (task.status === link.column) {
        // Un estado sin columna deja la tarjeta donde está
        next.status = base.column = column ?? task.status;
        base.state = item.state.name;
      } else conflict = true;
    } else if (column && column !== task.status && task.status === link.column) {
      // El estado no cambió, pero ahora se sabe su columna y aquí nadie movió la tarjeta
      next.status = base.column = column;
    }

    if (conflict) conflicts.push(task.id);
    if (next.title !== task.title || next.status !== task.status) updated++;
    links[task.id] = base;
    return next;
  });

  const linked = new Set(Object.values(links).map((l) => l.id));
  const used = new Set(tasks.map((t) => t.id));
  let added = 0;
  for (const item of items) {
    if (linked.has(item.id) || missing.includes(item.id)) continue;
    let id = `${provider.task_prefix}-${item.id}`;
    while (used.has(id)) id += '_';
    used.add(id);
    const title = cellText(item.title);
    const status = columnOf(doc, provider, item.state) ?? doc.columns[0]?.id ?? 'todo';
    tasks.push({ id, title, status, start: today, duration: 1, dependsOn: '-', progress: 0, comments: [] });
    links[id] = { id: item.id, title, state: item.state.name, column: status };
    added++;
  }

  return { doc: { ...doc, tasks, source: { ...source, links } }, added, updated, conflicts, missing };
}

/** Lo que «Enviar» cambiaría en una tarea ya vinculada. */
export interface PushChange {
  taskId: string;
  item: Issue;
  title?: string;
  state?: string;
  /** Columna de la tarea al preparar el envío: la que pide ese estado. */
  column: string;
  comments: { timestamp: string; text: string }[];
  /** El mismo campo cambió también en el proveedor: solo se envía si se marca. */
  conflict: boolean;
}

export interface PushPlan {
  changes: PushChange[];
  /** Tareas sin vínculo: se crean solo las que se marquen. */
  unlinked: BoardTask[];
  /** Vinculadas a una tarea que ya no se encuentra. */
  missing: { taskId: string; id: string }[];
  /** Cambiaron de columna a una que no tiene estado en el proveedor. */
  unmapped: string[];
}

export function buildPushPlan(doc: BoardDocument, provider: IssueProvider, items: Issue[]): PushPlan {
  const source = doc.source!;
  const byId = new Map(items.map((item) => [item.id, item]));
  const plan: PushPlan = { changes: [], unlinked: [], missing: [], unmapped: [] };

  for (const task of doc.tasks) {
    const link = source.links?.[task.id];
    if (!link) {
      plan.unlinked.push(task);
      continue;
    }
    const item = byId.get(link.id);
    if (!item) {
      plan.missing.push({ taskId: task.id, id: link.id });
      continue;
    }
    const change: PushChange = {
      taskId: task.id,
      item,
      column: task.status,
      comments: task.comments.filter((c) => !link.sent?.includes(c.timestamp)),
      conflict: false,
    };

    const title = cellText(item.title);
    if (task.title !== link.title && task.title !== title) {
      change.title = task.title;
      change.conflict ||= title !== link.title;
    }

    if (task.status !== link.column) {
      const states = statesOf(source, provider, task.status);
      if (states.length === 0) plan.unmapped.push(task.id);
      // Si su estado remoto ya es de esta columna, no hay nada que cambiar
      else if (columnOf(doc, provider, item.state) !== task.status) {
        change.state = states[0];
        change.conflict ||= item.state.name !== link.state;
      }
    }

    if (change.title !== undefined || change.state !== undefined || change.comments.length > 0) {
      plan.changes.push(change);
    }
  }
  return plan;
}

/** Escrituras en el proveedor. */
export interface IssueWriter {
  update(id: string, revision: string | null, changes: { title?: string; state?: string }): Promise<Issue>;
  create(type: string, title: string): Promise<Issue>;
  comment(id: string, text: string): Promise<void>;
}
export interface PushResult {
  updated: number;
  created: number;
  comments: number;
  errors: { taskId: string; error: unknown }[];
}

/**
 * Envía los cambios elegidos y crea las tareas marcadas. `save` guarda cada vínculo en cuanto la
 * escritura se confirma, para que un fallo a medias no repita lo ya hecho.
 */
export async function runPush(
  provider: IssueProvider,
  writer: IssueWriter,
  read: () => BoardDocument,
  save: (taskId: string, link: IssueLink) => void,
  changes: PushChange[],
  create: string[],
): Promise<PushResult> {
  const result: PushResult = { updated: 0, created: 0, comments: 0, errors: [] };

  async function sendComments(taskId: string, link: IssueLink, comments: PushChange['comments']) {
    for (const c of comments) {
      await writer.comment(link.id, c.text);
      link.sent = [...(link.sent ?? []), c.timestamp];
      save(taskId, { ...link });
      result.comments++;
    }
  }

  for (const change of changes) {
    try {
      const current = read().source?.links?.[change.taskId];
      if (!current) continue;
      const link = { ...current };
      if (change.title !== undefined || change.state !== undefined) {
        const item = await writer.update(change.item.id, change.item.revision, { title: change.title, state: change.state });
        if (change.title !== undefined) link.title = cellText(item.title);
        if (change.state !== undefined) {
          link.state = item.state.name;
          link.column = change.column;
        }
        save(change.taskId, { ...link });
        result.updated++;
      }
      await sendComments(change.taskId, link, change.comments);
    } catch (error) {
      result.errors.push({ taskId: change.taskId, error });
    }
  }

  for (const taskId of create) {
    try {
      const doc = read();
      const task = doc.tasks.find((t) => t.id === taskId);
      if (!task || doc.source?.links?.[taskId]) continue;
      let item = await writer.create(doc.source?.issue_type?.trim() || provider.default_issue_type, task.title);
      // Nace en su estado inicial; la columna de la tarea se envía después
      const initial = columnOf(doc, provider, item.state) ?? doc.columns[0]?.id ?? 'todo';
      const link: IssueLink = { id: item.id, title: cellText(item.title), state: item.state.name, column: initial };
      save(taskId, { ...link });
      result.created++;
      const state = statesOf(doc.source!, provider, task.status)[0];
      if (state && task.status !== initial) {
        item = await writer.update(item.id, item.revision, { state });
        link.state = item.state.name;
        link.column = task.status;
        save(taskId, { ...link });
      }
      await sendComments(taskId, link, task.comments);
    } catch (error) {
      result.errors.push({ taskId, error });
    }
  }

  return result;
}
