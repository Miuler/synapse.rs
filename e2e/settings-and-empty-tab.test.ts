// La pestaña vacía trae el buscador de Ctrl+O con el foco puesto, y el botón de
// configuración abre un panel que, entre otras cosas, guarda atajos en shortcuts.jsonc.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { Key } from 'webdriverio';
import { startApp, type Session } from './harness';

let s: Session;

beforeAll(async () => {
  s = await startApp({ 'plan anual.md': '# Plan anual\n\ntexto\n', 'otra.md': '# Otra\n' });
  await s.browser.$('.file-tree-item[data-path="otra.md"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(() => s?.close());

it('la pestaña vacía tiene el buscador con el foco', async () => {
  const input = s.browser.$('.empty-search .search-box-input');
  await input.waitForExist({ timeout: 10_000 });
  await s.browser.waitUntil(
    () => s.browser.execute(() => document.activeElement?.closest('.empty-search') != null) as Promise<boolean>,
    { timeout: 5_000, timeoutMsg: 'el buscador no tiene el foco' },
  );
  // Sin texto no se muestra la lista
  expect(await s.browser.$('.empty-search .palette-item').isExisting()).toBe(false);

  await s.browser.keys('plan');
  await s.browser.$('.empty-search .palette-item').waitForExist({ timeout: 5_000 });
  await s.browser.keys(Key.Enter);
  await s.browser.$('.markdown-body, .cm-editor').waitForExist({ timeout: 10_000 });
});

it('el botón de configuración abre el panel y guarda un atajo', async () => {
  await s.browser.$('[aria-label="Configuración"]').click();
  await s.browser.$('.settings-panel').waitForDisplayed({ timeout: 5_000 });

  const buttons = await s.browser.$$('.settings-nav-item');
  for (const b of buttons) {
    if ((await b.getText()) === 'Atajos de teclado') await b.click();
  }
  const field = s.browser.$('.shortcut-input');
  await field.waitForExist({ timeout: 5_000 });
  await field.click();
  await s.browser.keys([Key.Ctrl, 'a']);
  await s.browser.keys('Ctrl+O, Alt+O');
  await s.browser.keys(Key.Enter);

  const file = join(s.home, '.config', 'synapsers', 'shortcuts.jsonc');
  await s.browser.waitUntil(() => readFileSync(file, 'utf8').includes('"quick_open": "Ctrl+O, Alt+O"'), {
    timeout: 5_000,
    timeoutMsg: 'el atajo no se guardó en shortcuts.jsonc',
  });
  // Se conservan los comentarios del archivo
  expect(readFileSync(file, 'utf8')).toContain('// ');
});
