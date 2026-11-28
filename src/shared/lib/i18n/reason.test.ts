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
  i18n.setLocale('en');
  expect(reasonText('duplicate name except for case')).toBe('same name as another file except for letter case');
});

it('deja como está lo que no es un motivo propio', () => {
  i18n.setLocale('es');
  for (const detail of ['', 'HTTP 500', '/a/b.md: Permission denied', 'the file is empty value']) {
    expect(reasonText(detail)).toBe(detail);
  }
});
