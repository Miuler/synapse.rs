import type { AgentOption } from './types';

/**
 * Modelo recordado que ya no está en la lista porque antes llevaba el esfuerzo en el identificador
 * (`gemini-4-neon-high`): se separa en modelo y esfuerzo. `null` si no corresponde a ninguno.
 */
export function savedModelChoice(
  saved: string,
  models: AgentOption[],
  efforts: AgentOption[],
): { model: string; effort: string | null } | null {
  if (models.some((m) => m.id === saved)) return { model: saved, effort: null };
  for (const m of models) {
    const effort = efforts.find((e) => saved === `${m.id}-${e.id}`);
    if (effort) return { model: m.id, effort: effort.id };
  }
  return null;
}
