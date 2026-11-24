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
