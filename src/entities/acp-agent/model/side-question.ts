import type { AgentOption, ChatItem, PermissionOption } from './types';

/** Lo que recibe el agente con `/btw`: la pregunta y la indicación de no actuar. */
export function btwPrompt(question: string): string {
  return (
    '[Pregunta aparte del usuario, hecha desde Synapse mientras otra sesión sigue con el trabajo de la conversación anterior]\n' +
    'Responde directamente y en breve, con lo que ya sabes de esa conversación. No continúes ese trabajo, ' +
    'no hagas planes y no modifiques archivos: los permisos que pidas se rechazarán.\n\n' +
    question
  );
}

/** Nota para el agente cuando la pregunta aparte pasa a ser una conversación normal. */
export const PROMOTED_NOTE =
  '[Synapse] La pregunta aparte es ahora una conversación normal: ya puedes usar herramientas y pedir permisos.';

/** Modo del agente que no modifica nada, si ofrece alguno. */
export function readOnlyMode(modes: AgentOption[]): string | null {
  return modes.find((m) => /^(plan|ask|read[-_]?only)$/i.test(m.id))?.id ?? null;
}

/** Opción con la que se rechaza un permiso (`null`: cancelarlo). */
export function rejectOption(options: PermissionOption[]): string | null {
  const reject = options.filter((o) => o.kind.startsWith('reject'));
  return (reject.find((o) => o.kind === 'reject_once') ?? reject[0])?.id ?? null;
}

/** Mensajes que hereda una bifurcación: sin permisos ni preguntas que aún espera la original. */
export function forkItems(items: ChatItem[]): ChatItem[] {
  return items.filter((i) => !((i.type === 'permission' || i.type === 'elicitation') && i.answered === null));
}

/**
 * Lo que una pregunta aparte hereda de su conversación: los mensajes hasta `until` (el último que
 * había al preguntar) y la marca de bifurcación.
 */
export function asideBase(items: ChatItem[], until: number, handle: string, title: string): ChatItem[] {
  const base = forkItems(items.filter((i) => i.id <= until));
  return base.length ? [...base, { type: 'fork', id: until + 1, handle, title }] : [];
}

/** Título: la primera pregunta; en una bifurcación, la primera tras bifurcar (o la de origen). */
export function conversationTitle(items: ChatItem[]): string {
  const at = items.findLastIndex((i) => i.type === 'fork');
  const fork = items[at];
  const first = items.slice(at + 1).find((i) => i.type === 'user');
  const text =
    first?.type === 'user' ? first.text : fork?.type === 'fork' ? `Bifurcada de ${fork.title || fork.handle}` : '';
  return text.replace(/\s+/g, ' ').trim().slice(0, 80);
}
