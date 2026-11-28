import { t } from './i18n.svelte';

/** Los motivos fijos que Rust pone en el detalle de un error: la frase en inglés es la clave. */
const REASONS = {
  'empty value': 'reasons.emptyValue',
  'no answer': 'reasons.noAnswer',
  'type declares no states': 'reasons.noStates',
  'duplicate name except for case': 'reasons.duplicateName',
  'no vault open': 'reasons.noVault',
  'missing revision': 'reasons.missingRevision',
  'this server does not support changing the type': 'reasons.typeChange',
  'response without work item': 'reasons.noWorkItem',
  'config.jsonc cannot be read: fix it before saving': 'reasons.configUnreadable',
  'no keyring on this system': 'reasons.noKeyring',
} as const;

/**
 * El detalle de un error en el idioma de la interfaz: el motivo solo o tras «valor: ». Cualquier
 * otro (del sistema, del servidor) se devuelve como está.
 */
export function reasonText(detail: string): string {
  for (const [phrase, key] of Object.entries(REASONS)) {
    if (detail === phrase) return t(key);
    if (detail.endsWith(`: ${phrase}`)) return detail.slice(0, -phrase.length) + t(key);
  }
  return detail;
}
