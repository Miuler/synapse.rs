import type { DocxDiffEntry } from '../api/docx-diff.api';

/** Bloques iguales que se dejan a la vista junto a cada cambio. */
export const CONTEXT = 2;

export type DocxGroup = { type: 'entries'; start: number; entries: DocxDiffEntry[] } | { type: 'fold'; start: number; entries: DocxDiffEntry[] };

/** Pliega los tramos largos sin cambios, dejando contexto junto a los cambios. */
export function groupEntries(entries: DocxDiffEntry[]): DocxGroup[] {
  const keep = new Array<boolean>(entries.length).fill(false);
  entries.forEach((e, i) => {
    if (e.kind === 'equal') return;
    for (let k = Math.max(0, i - CONTEXT); k <= Math.min(entries.length - 1, i + CONTEXT); k++) keep[k] = true;
  });
  const groups: DocxGroup[] = [];
  for (let i = 0; i < entries.length; ) {
    let j = i;
    while (j < entries.length && keep[j] === keep[i]) j++;
    // Plegar uno o dos bloques no ahorra nada
    const fold = !keep[i] && j - i > 2;
    const last = groups.at(-1);
    if (!fold && last?.type === 'entries') last.entries.push(...entries.slice(i, j));
    else groups.push({ type: fold ? 'fold' : 'entries', start: i, entries: entries.slice(i, j) });
    i = j;
  }
  return groups;
}

export function countChanges(entries: DocxDiffEntry[]) {
  const count = (kind: string) => entries.filter((e) => e.kind === kind).length;
  return { added: count('added'), removed: count('removed'), modified: count('modified') };
}
