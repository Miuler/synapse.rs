import { invokeTauri } from '@shared/api';

/**
 * Synapse espera una respuesta del usuario (un permiso, una pregunta): si la ventana no está al
 * frente, la marca como urgente y avisa con una notificación del sistema. Sin foco robado.
 */
export function requestAttention(title: string, body: string): void {
  invokeTauri('request_attention', { title, body }).catch(() => {});
}
