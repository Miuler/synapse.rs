import { invokeTauri } from '@shared/api';

/**
 * Synapse espera una respuesta del usuario (un permiso, una pregunta): si la ventana no está al
 * frente, la marca como urgente y avisa con una notificación del sistema. Sin foco robado.
 */
export function requestAttention(title: string, body: string): void {
  invokeTauri('request_attention', { title, body }).catch(() => {});
}

/**
 * Avisa con una notificación del sistema solo si ninguna ventana de esta instancia tiene el foco (el
 * usuario está en otra aplicación o en otra instancia de Synapse). No marca la ventana como urgente.
 */
export function notifyIfUnfocused(title: string, body: string): void {
  invokeTauri('notify_if_unfocused', { title, body }).catch(() => {});
}

/** Qué muestra la notificación de fin de turno de un agente; se elige en Configuración. */
export type AgentNoticeContent = 'answer' | 'query';

let noticeContent: AgentNoticeContent = 'answer';

export const agentNoticeContent = (): AgentNoticeContent => noticeContent;

export function setAgentNoticeContent(content: AgentNoticeContent): void {
  noticeContent = content;
}
