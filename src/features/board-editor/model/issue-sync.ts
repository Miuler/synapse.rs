// Sincronización de un tablero con un gestor de issues en dos acciones separadas: «Traer» y «Enviar».
// El tablero guarda, por tarea vinculada, lo último que ambos lados tenían en común (`links`): así se
// distingue qué cambió aquí y qué cambió allí, y no se repiten envíos. De qué proveedor son las
// tareas solo se sabe por lo que él declara.
import { fingerprint, version } from './activity';
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
  /** Ya no la devuelve la consulta del tablero: sigue en él, marcada. */
  outside?: boolean;
  /** Versión de la que se guardó la descripción y la actividad (formato de carpeta). */
  rev?: string;
  /** Cuándo cambió en el proveedor, según la última vez que se trajo (formato de carpeta). */
  updated?: string;
  /** Huella de la última descripción que ambos lados tenían. */
  description?: string;
  /** El tipo que había antes de cambiarlo aquí (formato de carpeta). */
  type?: string;
  /** Como `type`, para el asignado; vacío, no tenía. */
  assignee?: string;
}

export type Criterion = 'type' | 'state' | 'assignee' | 'iteration' | 'area' | 'tag';
/** Dentro de un criterio basta un valor; entre criterios se exigen todos. */
export type IssueFilter = Partial<Record<Criterion, string[]>>;

/** De dónde vienen las tareas de un tablero. */
export interface IssueSource {
  provider: string;
  /** Id de la conexión; vacío, la que cubra el ámbito. */
  connection: string;
  /** Proyecto o repositorio, en la forma que entienda el proveedor. */
  scope: string;
  /** Consulta propia; vacía, la que el proveedor usa por defecto. */
  query?: string;
  /** Criterios elegidos. */
  filter?: IssueFilter;
  /** La consulta que salió de ellos: si `query` ya no es esta, se editó a mano y manda ella. */
  filter_query?: string;
  /** Tipo de las tareas que se crean. */
  issue_type?: string;
  /** A quién se asignan las que se crean; vacío, a nadie. */
  assignee?: string;
  /** Estados de cada columna: el primero es el que se envía, todos valen al traer. */
  states?: Record<string, string[]>;
  links?: Record<string, IssueLink>;
  /** Cuándo se trajeron las tareas por última vez (formato de carpeta, UTC). */
  synced_at?: string;
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
  assignee?: { name: string; id: string } | null;
  key?: string;
  url?: string | null;
  /** Markdown. */
  description?: string | null;
  updated_at?: string | null;
  /** Lo que no cabe en el modelo común. */
  extra?: Record<string, string>;
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
  filter_criteria?: Criterion[];
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

/** Lo que de una tarea solo dice el proveedor. */
function remoteFields(item: Issue): Partial<BoardTask> {
  const fields: Partial<BoardTask> = { type: item.type || undefined, assignee: item.assignee?.name, key: item.key, url: item.url ?? undefined };
  return Object.fromEntries(Object.entries(fields).filter(([, value]) => value !== undefined));
}

/** El filtro por criterios sigue valiendo mientras su consulta no se haya editado a mano. */
export function usesFilter(source: IssueSource): source is IssueSource & { filter: IssueFilter } {
  return !!source.filter && !!source.query && source.query.trim() === source.filter_query?.trim();
}

/** Un título tal como queda en una celda de la tabla. */
const cellText = (title: string) => title.replace(/\s*[\r\n]+\s*/g, ' ').trim();

export interface PullResult {
  doc: BoardDocument;
  added: number;
  updated: number;
  /** Tareas con cambios distintos en los dos lados: se conserva lo local. */
  conflicts: string[];
  /** Las que cambiaron de título o columna. */
  changed: string[];
  /** Tareas que el proveedor ya no devuelve: se desvinculan, la fila se queda. */
  missing: string[];
  /** Tareas vinculadas que la consulta ya no devuelve. */
  outside: string[];
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
  /** Lo que devolvió la consulta; sin saberlo, ninguna se marca. */
  matched?: string[],
): PullResult {
  const source = doc.source!;
  const byId = new Map(items.map((item) => [item.id, item]));
  const links: Record<string, IssueLink> = {};
  const conflicts: string[] = [];
  const missing: string[] = [];
  const outside: string[] = [];
  const changed: string[] = [];

  const tasks = doc.tasks.map((task) => {
    const link = source.links?.[task.id];
    if (!link) return task;
    const item = byId.get(link.id);
    if (!item) {
      if (requested.includes(link.id)) missing.push(link.id);
      else links[task.id] = link;
      return task;
    }
    const next: BoardTask = { ...task, ...remoteFields(item) };
    // Sin asignar en el proveedor, deja de estarlo aquí
    if (item.assignee === null) delete next.assignee;
    const base: IssueLink = { ...link };
    let conflict = false;

    // Lo cambiado aquí, que tiene su valor anterior en el vínculo, no se pisa
    for (const field of ['type', 'assignee'] as const) {
      if (link[field] === undefined) continue;
      const [local, remote] = [task[field] ?? '', next[field] ?? ''];
      if (local === remote || local === link[field]) {
        delete base[field];
        continue;
      }
      if (task[field] === undefined) delete next[field];
      else next[field] = task[field];
      if (remote !== link[field]) conflict = true;
    }

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

    if (matched) {
      // Las consultas dejan fuera lo terminado: su columna ya dice por qué no vino
      const closed = item.state.category === 'done' || item.state.category === 'cancelled';
      if (closed || matched.includes(link.id)) delete base.outside;
      else {
        base.outside = true;
        outside.push(task.id);
      }
    }
    if (conflict) conflicts.push(task.id);
    if (next.title !== task.title || next.status !== task.status) changed.push(task.id);
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
    tasks.push({ id, title, status, start: today, duration: 1, dependsOn: '-', progress: 0, comments: [], ...remoteFields(item) });
    links[id] = { id: item.id, title, state: item.state.name, column: status };
    added++;
  }

  return { doc: { ...doc, tasks, source: { ...source, links } }, added, updated: changed.length, changed, conflicts, missing, outside };
}

/** Lo que «Enviar» cambiaría en una tarea ya vinculada. */
export interface PushChange {
  taskId: string;
  item: Issue;
  title?: string;
  state?: string;
  type?: string;
  /** Vacío, quitar la asignación. */
  assignee?: string;
  /** `lossy`: al reescribirla se pierde formato del proveedor, así que solo se envía si se marca. */
  description?: { text: string; lossy: boolean };
  /** Comentarios del proveedor editados aquí; solo se envían si se marca. */
  edits?: { remote: string; timestamp: string; text: string }[];
  /** La tarea cambió en el proveedor desde el último «Traer». */
  stale?: boolean;
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
  /** Cambios que el proveedor no admite: no se envían. */
  unsupported: { taskId: string; field: string }[];
}

/** Si un cambio lleva algo que enviar. */
export const hasChanges = (c: PushChange) =>
  [c.title, c.state, c.type, c.assignee, c.description].some((v) => v !== undefined) ||
  c.comments.length > 0 ||
  !!c.edits?.length;

/** Lo que de cada tarea no está en el documento y lo que el proveedor admite cambiar. */
export interface PushContext {
  /** Descripción de cada tarea (formato de carpeta). */
  descriptions?: Record<string, string>;
  /** Sin decirlo, todo. */
  editable?: string[];
}

export function buildPushPlan(
  doc: BoardDocument,
  provider: IssueProvider,
  items: Issue[],
  { descriptions = {}, editable }: PushContext = {},
): PushPlan {
  const source = doc.source!;
  const byId = new Map(items.map((item) => [item.id, item]));
  const plan: PushPlan = { changes: [], unlinked: [], missing: [], unmapped: [], unsupported: [] };

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
      // Lo que ya está en el proveedor tiene su id
      comments: task.comments.filter((c) => !c.remote && !link.sent?.includes(c.timestamp)),
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

    /** Si el proveedor admite cambiar `field`; si no, queda anotado. */
    const can = (field: string) => {
      if (!editable || editable.includes(field)) return true;
      plan.unsupported.push({ taskId: task.id, field });
      return false;
    };
    // El tipo y el asignado cambiados aquí tienen su valor anterior en el vínculo
    if (link.type !== undefined && task.type && task.type !== link.type && task.type !== item.type && can('type')) {
      change.type = task.type;
      change.conflict ||= item.type !== link.type;
    }
    if (link.assignee !== undefined) {
      const [local, remote] = [task.assignee ?? '', item.assignee?.name ?? ''];
      if (local !== link.assignee && local !== remote && can('assignee')) {
        change.assignee = local;
        change.conflict ||= remote !== link.assignee;
      }
    }
    const description = descriptions[task.id];
    if (link.description !== undefined && description !== undefined) {
      const [local, remote] = [fingerprint(description), fingerprint(item.description ?? '')];
      if (local !== link.description && local !== remote && can('description')) {
        change.description = { text: description, lossy: item.extra?.description_lossy === 'true' };
        change.conflict ||= remote !== link.description;
      }
    }
    // Solo se reescribe lo que salió de aquí: lo demás puede ser de otra persona
    const edited = task.comments.filter((c) => c.remote && c.edited);
    const edits = edited.flatMap((c) =>
      link.sent?.includes(c.timestamp) ? [{ remote: c.remote!, timestamp: c.timestamp, text: c.text }] : [],
    );
    if (edits.length < edited.length) plan.unsupported.push({ taskId: task.id, field: 'foreignComment' });
    if (edits.length && can('comment')) {
      change.edits = edits;
      if (link.rev && link.rev !== version(item)) change.stale = true;
    }

    if (hasChanges(change)) plan.changes.push(change);
  }
  return plan;
}

/** Los campos de una tarea existente que se pueden escribir. */
export interface IssueFields {
  title?: string;
  state?: string;
  type?: string;
  assignee?: string;
  /** Markdown. */
  description?: string;
}

/** Escrituras en el proveedor. */
export interface IssueWriter {
  update(id: string, revision: string | null, changes: IssueFields): Promise<Issue>;
  create(type: string, title: string, extra?: { assignee?: string; description?: string }): Promise<Issue>;
  /** Devuelve el id del comentario creado, si el proveedor lo dice. */
  comment(id: string, text: string): Promise<string | null | void>;
  editComment(id: string, comment: string, text: string): Promise<unknown>;
}

/** Un comentario del tablero que ya quedó en el proveedor. */
export interface SentComment {
  timestamp: string;
  text: string;
  remote: string;
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
  save: (taskId: string, link: IssueLink, comment?: SentComment, description?: { text: string; over: string }) => void,
  changes: PushChange[],
  create: string[],
  descriptions: Record<string, string> = {},
): Promise<PushResult> {
  const result: PushResult = { updated: 0, created: 0, comments: 0, errors: [] };

  async function sendComments(taskId: string, link: IssueLink, comments: PushChange['comments']) {
    for (const c of comments) {
      const remote = await writer.comment(link.id, c.text);
      link.sent = [...(link.sent ?? []), c.timestamp];
      save(taskId, { ...link }, remote ? { timestamp: c.timestamp, text: c.text, remote } : undefined);
      result.comments++;
    }
  }

  for (const change of changes) {
    try {
      const current = read().source?.links?.[change.taskId];
      if (!current) continue;
      const link = { ...current };
      const fields: IssueFields = Object.fromEntries(
        Object.entries({
          title: change.title,
          state: change.state,
          type: change.type,
          assignee: change.assignee,
          description: change.description?.text,
        }).filter(([, value]) => value !== undefined),
      );
      if (Object.keys(fields).length) {
        const item = await writer.update(change.item.id, change.item.revision, fields);
        if (change.title !== undefined) link.title = cellText(item.title);
        if (change.state !== undefined) {
          link.state = item.state.name;
          link.column = change.column;
        }
        // Ya coinciden: no queda nada pendiente de ellos
        if (change.type !== undefined) delete link.type;
        if (change.assignee !== undefined) delete link.assignee;
        // Aquí queda como la guardó el proveedor: si no, parecería cambiada otra vez
        let kept: { text: string; over: string } | undefined;
        if (change.description) {
          const sent = change.description.text;
          kept = { text: item.description ?? sent, over: fingerprint(sent) };
          link.description = fingerprint(kept.text);
        }
        save(change.taskId, { ...link }, undefined, kept);
        result.updated++;
      }
      await sendComments(change.taskId, link, change.comments);
      for (const edit of change.edits ?? []) {
        await writer.editComment(link.id, edit.remote, edit.text);
        save(change.taskId, { ...link }, edit);
        result.comments++;
      }
    } catch (error) {
      result.errors.push({ taskId: change.taskId, error });
    }
  }

  for (const taskId of create) {
    try {
      const doc = read();
      const task = doc.tasks.find((t) => t.id === taskId);
      if (!task || doc.source?.links?.[taskId]) continue;
      const description = descriptions[taskId] || undefined;
      let item = await writer.create(
        task.type || doc.source?.issue_type?.trim() || provider.default_issue_type,
        task.title,
        { assignee: task.assignee, description },
      );
      // Nace en su estado inicial; la columna de la tarea se envía después
      const initial = columnOf(doc, provider, item.state) ?? doc.columns[0]?.id ?? 'todo';
      const link: IssueLink = {
        id: item.id,
        title: cellText(item.title),
        state: item.state.name,
        column: initial,
        ...(description && { description: fingerprint(description) }),
      };
      save(taskId, { ...link });
      result.created++;
      const state = statesOf(doc.source!, provider, task.status)[0];
      if (state && task.status !== initial) {
        item = await writer.update(item.id, item.revision, { state });
        link.state = item.state.name;
        link.column = task.status;
        save(taskId, { ...link });
      }
      // Los traídos de una tarea de antes no son comentarios por enviar
      await sendComments(taskId, link, task.comments.filter((c) => !c.remote));
    } catch (error) {
      result.errors.push({ taskId, error });
    }
  }

  return result;
}
