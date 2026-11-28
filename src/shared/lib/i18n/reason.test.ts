import { afterEach, expect, it } from 'bun:test';
import { i18n } from './i18n.svelte';
import { reasonText } from './reason';

const initial = i18n.locale;
afterEach(() => i18n.setLocale(initial));

it('traduce el motivo solo y el que sigue a un valor', () => {
  i18n.setLocale('es');
  expect(reasonText('missing revision')).toBe('falta la revisión');
  expect(reasonText('op://boveda/elemento/campo: empty value')).toBe('op://boveda/elemento/campo: vacío');
  expect(reasonText('Bug: type declares no states')).toBe('Bug: el tipo no declara estados');
  expect(reasonText('config.jsonc cannot be read: fix it before saving')).toBe(
    'config.jsonc no se puede leer: corrígelo antes de guardar',
  );
  expect(reasonText('t1.md (Equipo.board.md): belongs to that board and is changed from it')).toBe(
    't1.md (Equipo.board.md): pertenece a ese tablero y se cambia desde él',
  );
  // El motivo más largo no se confunde con el que es su comienzo
  expect(reasonText('nota.md: name already taken in the destination')).toBe('nota.md: ese nombre ya está en uso en el destino');
  expect(reasonText('no merge in progress')).toBe('no hay una fusión en curso');
  expect(reasonText('feature: branch has unmerged commits')).toBe('feature: la rama tiene commits sin fusionar');
  expect(reasonText('range contains merge commits: cannot squash into one commit')).toBe(
    'el rango incluye fusiones: no se pueden resumir en un solo commit',
  );
  expect(reasonText('/usr/bin/true: not a working Git executable (`git --version` failed)')).toBe(
    '/usr/bin/true: no es un ejecutable Git válido (falló `git --version`)',
  );
  expect(reasonText('agent session is no longer open')).toBe('la sesión del agente ya no está abierta');
  expect(reasonText('Document XML is corrupted')).toBe('el XML del documento está dañado');
  i18n.setLocale('en');
  expect(reasonText('duplicate name except for case')).toBe('same name as another file except for letter case');
  expect(reasonText('feature: branch has unmerged commits')).toBe('feature: branch has unmerged commits');
  expect(reasonText('no merge in progress')).toBe('no merge in progress');
});

it('deja como está lo que no es un motivo propio', () => {
  i18n.setLocale('es');
  for (const detail of ['', 'HTTP 500', '/a/b.md: Permission denied', 'the file is empty value']) {
    expect(reasonText(detail)).toBe(detail);
  }
});
