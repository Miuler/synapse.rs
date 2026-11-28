// «Traer», se pida desde el tablero o desde una nota que lo muestra: pregunta al proveedor, aplica lo
// que cambió y, en un tablero de carpeta, guarda además la descripción y la actividad.
import { t } from '@shared/lib/i18n';
import { fetchActivity, fetchIssues, filterQuery, type IssueError } from '../api/issues.api';
import { changedIssues, receive } from './activity';
import type { BoardDocument } from './board.svelte';
import type { TaskFile } from './folder-board';
import { applyPull, usesFilter, type IssueProvider, type IssueSource } from './issue-sync';

export interface PulledBoard {
  /** El tablero tal como está ahora: se vuelve a leer después de cada llamada. */
  read: () => BoardDocument;
  write: (next: BoardDocument) => void;
  /** Solo en los tableros de carpeta. */
  readTask?: (id: string) => Promise<{ task: TaskFile }>;
}

export function issueErrorText(error: unknown, provider: string): string {
  const e = error as Partial<IssueError>;
  if (!e?.kind) return String(error instanceof Error ? error.message : error);
  return t(`issues.errors.${e.kind}`, { provider, detail: e.detail ?? '' });
}

/** Los ids vinculados a tareas que siguen en el tablero. */
export function linkedIds(doc: BoardDocument): string[] {
  return doc.tasks.flatMap((task) => doc.source?.links?.[task.id]?.id ?? []);
}

/** Devuelve qué contar; nada si el origen cambió mientras se traía. */
export async function pullIssues(
  source: IssueSource,
  declared: IssueProvider,
  { read, write, readTask }: PulledBoard,
): Promise<string[]> {
  let query = source.query?.trim() || declared.default_query;
  // «La iteración en curso» cambia con los días: la consulta del filtro se rehace al traer
  if (usesFilter(source)) {
    const compiled = await filterQuery(source, source.filter);
    const doc = read();
    if (compiled !== query && doc.source && usesFilter(doc.source)) {
      write({ ...doc, source: { ...doc.source, query: compiled, filter_query: compiled } });
    }
    query = compiled;
  }
  const { items, requested, matched } = await fetchIssues(source, query, linkedIds(read()));
  if (read().source?.scope !== source.scope) return [];
  const today = new Date().toLocaleDateString('sv-SE');
  const apply = () => applyPull(read(), declared, items, requested, today, matched);
  // En un tablero de carpeta, además, la actividad de lo que cambió
  let activity: Awaited<ReturnType<typeof fetchActivity>> | undefined;
  const files: Record<string, TaskFile> = {};
  if (readTask) {
    const ids = changedIssues(apply().doc, items);
    activity = ids.length ? await fetchActivity(source, ids) : { items: {}, failed: [] };
    const existing = Object.entries(read().source?.links ?? {}).filter(([, link]) => activity!.items[link.id]);
    const present = new Set(read().tasks.map((task) => task.id));
    await Promise.all(
      existing.filter(([id]) => present.has(id)).map(async ([id]) => (files[id] = (await readTask(id)).task)),
    );
    if (read().source?.scope !== source.scope) return [];
  }
  const result = apply();
  let { doc } = result;
  let changed = result.changed;
  let conflicts = result.conflicts;
  if (activity) {
    const received = receive(doc, items, activity.items, files);
    doc = received.doc;
    doc = { ...doc, source: { ...doc.source!, synced_at: new Date().toISOString().slice(0, 19) } };
    changed = [...new Set([...changed, ...received.changed])];
    conflicts = [...new Set([...conflicts, ...received.conflicts])];
  }
  write(doc);
  const failed = activity?.failed ?? [];
  return [
    t('issues.pulled', { added: result.added, updated: changed.length }),
    ...(failed.length
      ? [t('issues.activityFailed', { count: failed.length, detail: issueErrorText(failed[0].error, declared.name) })]
      : []),
    ...(conflicts.length ? [t('issues.conflicts', { tasks: conflicts.join(', ') })] : []),
    ...(result.missing.length ? [t('issues.missing', { ids: result.missing.join(', ') })] : []),
    ...(result.outside.length ? [t('issues.outsideCount', { tasks: result.outside.join(', ') })] : []),
  ];
}
