// La descripción y la actividad (comentarios e histórico) que «Traer» guarda en el archivo de cada
// tarea de un tablero de carpeta. Solo se piden las de las tareas que cambiaron en el proveedor, y
// nunca pisan lo cambiado aquí.
import type { BoardDocument } from './board.svelte';
import type { ActivityEntry, TaskFile } from './folder-board';
import type { Issue, IssueLink } from './issue-sync';

/** Una entrada tal como la entrega el proveedor. */
export interface RemoteEntry {
  id: string;
  kind: 'comment' | 'history';
  author: { name: string } | null;
  at: string;
  text: string;
}

const trimmed = (text: string) => text.replace(/[ \t]+$/gm, '').trim();

/** Huella de un texto (FNV-1a); no distingue los espacios al final de las líneas. */
export function fingerprint(text: string): string {
  let hash = 0x811c9dc5;
  for (const char of trimmed(text)) hash = Math.imul(hash ^ char.codePointAt(0)!, 0x01000193);
  return (hash >>> 0).toString(16).padStart(8, '0');
}

/** Un texto sin las marcas que cambian al ir y volver del proveedor. */
const plain = (text: string) => text.replace(/\\(?=[^\w\s])/g, '').replace(/\s+/g, ' ').trim();

/**
 * La actividad de una tarea con lo traído. Una entrada editada aquí no se toca; un comentario que
 * Synapse envió sin conocer su id (`sent`) se reconoce por su texto y no se repite.
 */
export function mergeActivity(local: ActivityEntry[], remote: ActivityEntry[], sent: string[] = []): ActivityEntry[] {
  const next = [...local];
  for (const entry of remote) {
    const known = next.findIndex((e) => e.kind === entry.kind && e.id === entry.id);
    if (known >= 0) {
      if (!next[known].edited) next[known] = entry;
      continue;
    }
    const echo = next.findIndex(
      (e) => entry.kind === 'comment' && e.kind === 'comment' && !e.id && sent.includes(e.at) && plain(e.text) === plain(entry.text),
    );
    // Conserva su fecha: con ella consta como enviado
    if (echo >= 0) next[echo] = { ...next[echo], id: entry.id, author: next[echo].author ?? entry.author };
    else next.push(entry);
  }
  return next.sort((a, b) => a.at.localeCompare(b.at));
}

/**
 * Qué hacer con la descripción traída. `base` es la huella de la última que ambos lados tenían: si
 * aquí no cambió desde entonces, se toma la nueva; si cambió en los dos lados, se queda la de aquí.
 */
export function mergeDescription(local: string, remote: string, base?: string): { base?: string; take?: string; conflict?: boolean } {
  const [here, there] = [fingerprint(local), fingerprint(remote)];
  if (base === there) return { base };
  if (here === there) return { base: there };
  if (base === here || (base === undefined && !trimmed(local))) return { base: there, take: remote };
  return { base, conflict: true };
}

/**
 * Lo que cambia cuando cambia la actividad: la revisión y, si el proveedor lo dice, cuántos
 * comentarios hay.
 */
const version = (item: Issue) => [item.revision, item.extra?.comment_count].filter(Boolean).join('+');

/** Las tareas vinculadas que cambiaron en el proveedor desde que se leyó su actividad. */
export function changedIssues(doc: BoardDocument, items: Issue[]): string[] {
  const links = Object.values(doc.source?.links ?? {});
  const versions = new Map(items.map((item) => [item.id, version(item)]));
  return links.filter((link) => versions.has(link.id) && (!link.rev || link.rev !== versions.get(link.id))).map((link) => link.id);
}

/**
 * Deja en cada tarea lo traído para su archivo y anota en su vínculo hasta dónde se leyó. `files` son
 * los archivos de las que ya existían. Devuelve las que ganan algo y las que cambiaron en los dos lados.
 */
export function receive(
  doc: BoardDocument,
  items: Issue[],
  activity: Record<string, RemoteEntry[]>,
  files: Record<string, TaskFile>,
): { doc: BoardDocument; changed: string[]; conflicts: string[] } {
  const byId = new Map(items.map((item) => [item.id, item]));
  const links: Record<string, IssueLink> = { ...doc.source?.links };
  const changed: string[] = [];
  const conflicts: string[] = [];
  const tasks = doc.tasks.map((task) => {
    const link = links[task.id];
    const item = link && byId.get(link.id);
    const found = link && activity[link.id];
    // Sin su actividad no se anota la revisión: se vuelve a pedir la próxima vez
    if (!link || !item || !found) return task;
    const file = files[task.id];
    const local = file?.description ?? '';
    const entries: ActivityEntry[] = found.map((e) => ({
      at: e.at, kind: e.kind, author: e.author?.name ?? null, id: e.id, edited: false, text: e.text,
    }));
    const description = mergeDescription(local, item.description ?? '', link.description);
    if (description.conflict) conflicts.push(task.id);
    links[task.id] = {
      ...link,
      ...(version(item) && { rev: version(item) }),
      ...(description.base && { description: description.base }),
    };
    const merged = file && mergeActivity(file.activity, entries, link.sent);
    if (file && (description.take !== undefined || JSON.stringify(merged) !== JSON.stringify(file.activity))) changed.push(task.id);
    return {
      ...task,
      incoming: { entries, ...(description.take !== undefined && { description: description.take, over: fingerprint(local) }) },
    };
  });
  return { doc: { ...doc, tasks, source: doc.source && { ...doc.source, links } }, changed, conflicts };
}
