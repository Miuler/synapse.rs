// Sincronización de un tablero con Azure DevOps en dos acciones separadas: «Traer» y «Enviar».
// El tablero guarda, por tarea vinculada, lo último que ambos lados tenían en común (`links`): así se
// distingue qué cambió aquí y qué cambió allí, y no se repiten envíos.
import type { BoardDocument, BoardTask } from './board.svelte';

export interface AzureLink {
  id: number;
  /** Título y estado remotos de la última sincronización. */
  title: string;
  state: string;
  /** Columna en la que quedó la tarea en esa sincronización. */
  column: string;
  /** Fechas (`timestamp`) de los comentarios ya enviados. */
  sent?: string[];
}

export interface AzureSyncConfig {
  project_url: string;
  /** WIQL propio; vacío, los work items abiertos relacionados con quien sincroniza. */
  query?: string;
  /** Tipo de los work items que se crean (`Task` por defecto). */
  work_item_type?: string;
  /** A quién se asignan los que se crean (correo); vacío, a nadie. */
  assigned_to?: string;
  /** Estados de cada columna: el primero es el que se envía, todos valen al traer. */
  states?: Record<string, string[]>;
  links?: Record<string, AzureLink>;
}

export interface WorkItem {
  id: number;
  rev: number;
  title: string;
  state: string;
  work_item_type: string;
}

export const DEFAULT_QUERY = `SELECT [System.Id] FROM WorkItems
WHERE [System.TeamProject] = @project
  AND [System.State] NOT IN ('Closed', 'Cancelled', 'Removed', 'Done')
  AND ([System.AssignedTo] = @me OR [System.ChangedBy] = @me OR [System.CreatedBy] = @me)
ORDER BY [System.ChangedDate] DESC`;

export const DEFAULT_STATES: Record<string, string[]> = {
  todo: ['To Do', 'New', 'Proposed'],
  doing: ['In Progress', 'Doing', 'Active', 'Committed', 'Resolved'],
  done: ['Done', 'Closed', 'Completed'],
};

export const DEFAULT_WORK_ITEM_TYPE = 'Task';

function statesOf(azure: AzureSyncConfig, column: string): string[] {
  return azure.states?.[column] ?? DEFAULT_STATES[column] ?? [];
}

function columnOf(doc: BoardDocument, state: string): string | undefined {
  return doc.columns.find((c) => statesOf(doc.azure!, c.id).some((s) => s.toLowerCase() === state.toLowerCase()))?.id;
}

/** Un título tal como queda en una celda de la tabla. */
const cellText = (title: string) => title.replace(/\s*[\r\n]+\s*/g, ' ').trim();

export interface PullResult {
  doc: BoardDocument;
  added: number;
  updated: number;
  /** Tareas con cambios distintos en los dos lados: se conserva lo local. */
  conflicts: string[];
  /** Work items que Azure DevOps ya no devuelve: se desvinculan, la tarea se queda. */
  missing: number[];
}

/**
 * Aplica al tablero lo traído de Azure DevOps. `items` son los de la consulta y los ya vinculados;
 * `requested`, los ids por los que se preguntó (un vínculo creado mientras tanto no se toca).
 */
export function applyPull(doc: BoardDocument, items: WorkItem[], requested: number[], today: string): PullResult {
  const azure = doc.azure!;
  const byId = new Map(items.map((item) => [item.id, item]));
  const links: Record<string, AzureLink> = {};
  const conflicts: string[] = [];
  const missing: number[] = [];
  let updated = 0;

  const tasks = doc.tasks.map((task) => {
    const link = azure.links?.[task.id];
    if (!link) return task;
    const item = byId.get(link.id);
    if (!item) {
      if (requested.includes(link.id)) missing.push(link.id);
      else links[task.id] = link;
      return task;
    }
    const next: BoardTask = { ...task };
    const base: AzureLink = { ...link };
    let conflict = false;

    const title = cellText(item.title);
    if (task.title === title) base.title = title;
    else if (title !== link.title) {
      if (task.title === link.title) next.title = base.title = title;
      else conflict = true;
    }

    if (item.state !== link.state) {
      const column = columnOf(doc, item.state);
      if (column === task.status) {
        base.state = item.state;
        base.column = column;
      } else if (task.status === link.column) {
        // Un estado sin columna deja la tarjeta donde está
        next.status = base.column = column ?? task.status;
        base.state = item.state;
      } else conflict = true;
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
    let id = `ado-${item.id}`;
    while (used.has(id)) id += '_';
    used.add(id);
    const title = cellText(item.title);
    const status = columnOf(doc, item.state) ?? doc.columns[0]?.id ?? 'todo';
    tasks.push({ id, title, status, start: today, duration: 1, dependsOn: '-', progress: 0, comments: [] });
    links[id] = { id: item.id, title, state: item.state, column: status };
    added++;
  }

  return { doc: { ...doc, tasks, azure: { ...azure, links } }, added, updated, conflicts, missing };
}

/** Lo que «Enviar» cambiaría en un work item ya vinculado. */
export interface PushChange {
  taskId: string;
  item: WorkItem;
  title?: string;
  state?: string;
  /** Columna de la tarea al preparar el envío: la que pide ese estado. */
  column: string;
  comments: { timestamp: string; text: string }[];
  /** El mismo campo cambió también en Azure DevOps: solo se envía si se marca. */
  conflict: boolean;
}

export interface PushPlan {
  changes: PushChange[];
  /** Tareas sin vínculo: se crean solo las que se marquen. */
  unlinked: BoardTask[];
  /** Vinculadas a un work item que ya no se encuentra. */
  missing: { taskId: string; id: number }[];
  /** Cambiaron de columna a una que no tiene estado en Azure DevOps. */
  unmapped: string[];
}

export function buildPushPlan(doc: BoardDocument, items: WorkItem[]): PushPlan {
  const azure = doc.azure!;
  const byId = new Map(items.map((item) => [item.id, item]));
  const plan: PushPlan = { changes: [], unlinked: [], missing: [], unmapped: [] };

  for (const task of doc.tasks) {
    const link = azure.links?.[task.id];
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
      const states = statesOf(azure, task.status);
      if (states.length === 0) plan.unmapped.push(task.id);
      else if (!states.some((s) => s.toLowerCase() === item.state.toLowerCase())) {
        change.state = states[0];
        change.conflict ||= item.state !== link.state;
      }
    }

    if (change.title !== undefined || change.state !== undefined || change.comments.length > 0) {
      plan.changes.push(change);
    }
  }
  return plan;
}

/** Escrituras en Azure DevOps. */
export interface AzureWriter {
  update(id: number, rev: number, changes: { title?: string; state?: string }): Promise<WorkItem>;
  create(type: string, title: string): Promise<WorkItem>;
  comment(id: number, text: string): Promise<void>;
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
  writer: AzureWriter,
  read: () => BoardDocument,
  save: (taskId: string, link: AzureLink) => void,
  changes: PushChange[],
  create: string[],
): Promise<PushResult> {
  const result: PushResult = { updated: 0, created: 0, comments: 0, errors: [] };

  async function sendComments(taskId: string, link: AzureLink, comments: PushChange['comments']) {
    for (const c of comments) {
      await writer.comment(link.id, c.text);
      link.sent = [...(link.sent ?? []), c.timestamp];
      save(taskId, { ...link });
      result.comments++;
    }
  }

  for (const change of changes) {
    try {
      const current = read().azure?.links?.[change.taskId];
      if (!current) continue;
      const link = { ...current };
      if (change.title !== undefined || change.state !== undefined) {
        const item = await writer.update(change.item.id, change.item.rev, { title: change.title, state: change.state });
        if (change.title !== undefined) link.title = cellText(item.title);
        if (change.state !== undefined) {
          link.state = item.state;
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
      if (!task || doc.azure?.links?.[taskId]) continue;
      let item = await writer.create(doc.azure?.work_item_type?.trim() || DEFAULT_WORK_ITEM_TYPE, task.title);
      // Un work item nace en su estado inicial; la columna de la tarea se envía después
      const initial = columnOf(doc, item.state) ?? doc.columns[0]?.id ?? 'todo';
      const link: AzureLink = { id: item.id, title: cellText(item.title), state: item.state, column: initial };
      save(taskId, { ...link });
      result.created++;
      const state = statesOf(doc.azure!, task.status)[0];
      if (state && task.status !== initial) {
        item = await writer.update(item.id, item.rev, { state });
        link.state = item.state;
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
