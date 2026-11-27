// Menciones a otras conversaciones y bóvedas en el chat del agente: `#2` (conversación de esta
// ventana), `@Bóveda` y `@Bóveda#2` (otra bóveda de Synapse). Se insertan como texto; al enviar,
// se le explica al agente a quién apunta cada una para que use `send_to_conversation`.
import type { ConversationSummary, Mention } from '@entities/acp-agent';

export type VaultState = 'active' | 'busy' | 'off';

export interface VaultCandidate {
  name: string;
  path: string;
  state: VaultState;
  conversations: ConversationSummary[];
}

export const STATUS_ICON: Record<ConversationSummary['status'], string> = {
  working: '🧠',
  permission: '⚠',
  idle: '⏸',
  error: '✖',
};

export const VAULT_ICON: Record<VaultState, string> = { active: '🟢', busy: '🟡', off: '⚪' };

/** Nombre de bóveda como token sin espacios (`@Mi_Bóveda`). */
export const vaultToken = (name: string) => `@${name.replace(/\s+/g, '_')}`;

/** Referencia a un archivo de la bóveda en el texto (`@docs/nota.md`); además va adjunto. */
export const fileMention = (relPath: string): Mention => ({
  token: `@${relPath.replace(/\s+/g, '_')}`,
  to: relPath,
  title: `el archivo «${relPath}»`,
  file: true,
});

/** Carpetas de la bóveda que contienen alguna de estas rutas, de la más corta a la más larga. */
export function foldersOf(paths: Iterable<string>): string[] {
  const folders = new Set<string>();
  for (let path of paths) {
    for (let cut = path.lastIndexOf('/'); cut > 0; cut = path.lastIndexOf('/')) {
      path = path.slice(0, cut);
      // Ya vista: sus carpetas superiores también
      if (folders.has(path)) break;
      folders.add(path);
    }
  }
  return [...folders].sort((a, b) => a.length - b.length || a.localeCompare(b));
}

/** Referencia a una carpeta (`@docs/guias/`, la barra final la distingue); además va adjunta. */
export const folderMention = (relPath: string): Mention => ({
  token: `@${relPath.replace(/\s+/g, '_')}/`,
  to: relPath,
  title: `la carpeta «${relPath}»`,
  file: true,
});

/** Si las letras de `query` aparecen en `text` en ese orden, aunque separadas. */
export function fuzzy(text: string, query: string): boolean {
  let i = 0;
  for (const c of text) if (c === query[i]) i++;
  return i === query.length;
}

/** Qué tan bien casa una ruta (en minúsculas) con lo escrito: 0 es lo mejor. */
function matchRank(path: string, query: string): number {
  const name = path.slice(path.lastIndexOf('/') + 1);
  if (name === query || name.slice(0, name.lastIndexOf('.')) === query) return 0;
  if (name.startsWith(query)) return 1;
  if (name.includes(query)) return 2;
  return path.includes(query) ? 3 : 4;
}

/**
 * Los `limit` elementos cuya ruta mejor casa con `query` (en minúsculas): primero por su nombre,
 * luego por la ruta y al final por letras sueltas; a igualdad, la ruta más corta. Sin texto, en su orden.
 */
export function bestMatches<T>(items: Iterable<T>, pathOf: (item: T) => string, query: string, limit: number): T[] {
  if (!query) return [...items].slice(0, limit);
  const found: { item: T; rank: number; length: number }[] = [];
  for (const item of items) {
    const path = pathOf(item).toLowerCase();
    if (fuzzy(path, query)) found.push({ item, rank: matchRank(path, query), length: path.length });
  }
  return found
    .sort((a, b) => a.rank - b.rank || a.length - b.length)
    .slice(0, limit)
    .map((f) => f.item);
}

/** «hace 3 min», «hace 2 h»… */
export function ago(ms: number | undefined, now = Date.now()): string {
  if (!ms) return '';
  const min = Math.round((now - ms) / 60_000);
  if (min < 1) return 'ahora';
  if (min < 60) return `hace ${min} min`;
  const h = Math.round(min / 60);
  if (h < 24) return `hace ${h} h`;
  return `hace ${Math.round(h / 24)} d`;
}

/** Tooltip de una conversación: su última respuesta y la última pregunta. */
export function conversationTooltip(c: ConversationSummary): string {
  const parts = [`${c.handle} · ${c.title} · ${c.agent}`];
  if (c.last_reply) parts.push(`Última respuesta: ${c.last_reply}`);
  if (c.last_question) parts.push(`Última pregunta: ${c.last_question}`);
  return parts.join('\n\n');
}

/** Une las bóvedas abiertas (activas u ocupadas) con las conocidas apagadas, sin la propia. */
export function mergeVaults(
  open: { vault: string; path: string; conversations: ConversationSummary[] }[],
  known: { name: string; path: string }[],
  ownPath: string | null,
): VaultCandidate[] {
  const same = (a: string, b: string | null) => !!b && a.replace(/\/+$/, '') === b.replace(/\/+$/, '');
  const result: VaultCandidate[] = open
    .filter((v) => !same(v.path, ownPath))
    .map((v) => ({
      name: v.vault,
      path: v.path,
      state: v.conversations.some((c) => c.status === 'working' || c.status === 'permission') ? 'busy' : 'active',
      conversations: v.conversations,
    }));
  for (const k of known) {
    if (same(k.path, ownPath) || result.some((r) => same(r.path, k.path))) continue;
    result.push({ name: k.name, path: k.path, state: 'off', conversations: [] });
  }
  return result;
}

/** Nota para el agente con lo que significa cada mención que sigue en el texto. */
export function mentionsPrompt(text: string, mentions: Mention[]): { prompt: string; used: Mention[] } {
  const used = mentions.filter(
    (m, i) => mentions.findIndex((x) => x.token === m.token) === i && new RegExp(`(^|\\s)${escape(m.token)}(?=$|[\\s.,;:!?])`).test(text),
  );
  if (!used.length) return { prompt: text, used };
  const lines = used.map((m) =>
    m.file ? `- ${m.token}: ${m.title} (adjunto)` : `- ${m.token}: ${m.title}. Para escribirle usa send_to_conversation con to="${m.to}".`,
  );
  return { prompt: `${text}\n\n[Menciones de Synapse]\n${lines.join('\n')}`, used };
}

/** Parte un texto en trozos normales y menciones (para mostrarlas como chips). */
export function splitMentions(text: string, mentions: Mention[] | undefined): ({ text: string } | { mention: Mention })[] {
  if (!mentions?.length) return [{ text }];
  const re = new RegExp(`(${mentions.map((m) => escape(m.token)).sort((a, b) => b.length - a.length).join('|')})`, 'g');
  return text
    .split(re)
    .filter((part) => part !== '')
    .map((part) => {
      const m = mentions.find((x) => x.token === part);
      return m ? { mention: m } : { text: part };
    });
}

const escape = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
