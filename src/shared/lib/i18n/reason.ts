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
  'path outside the vault': 'reasons.outsideVault',
  "path with '..' is not allowed": 'reasons.dotDot',
  'path cannot be empty': 'reasons.emptyPath',
  'belongs to that board and is changed from it': 'reasons.boardOwned',
  'a board with a task folder keeps the .board.md extension': 'reasons.boardExtension',
  'name already taken': 'reasons.nameTaken',
  'name already taken in the destination': 'reasons.nameTakenThere',
  'item does not exist': 'reasons.noItem',
  'destination folder does not exist': 'reasons.noDestination',
  'cannot copy the vault root': 'reasons.copyRoot',
  'cannot copy a folder into itself': 'reasons.copyIntoItself',
  'cannot move the vault root': 'reasons.moveRoot',
  'cannot move a folder into itself': 'reasons.moveIntoItself',
  'cannot delete the vault root': 'reasons.deleteRoot',
  'cannot write to the vault root': 'reasons.writeRoot',
  'invalid item name': 'reasons.invalidName',
  'item has no parent folder': 'reasons.noParent',
  'name cannot be empty': 'reasons.emptyName',
  'name cannot contain path separators': 'reasons.separators',
  'unsupported item type': 'reasons.itemType',
  'could not copy the folder': 'reasons.copyFolder',
  'could not copy the file': 'reasons.copyFile',
  'could not copy the board folder': 'reasons.copyBoard',
  'could not delete the folder': 'reasons.deleteFolder',
  'could not delete the file': 'reasons.deleteFile',
  'could not create the folder': 'reasons.createFolder',
  'could not write the file': 'reasons.writeFile',
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
