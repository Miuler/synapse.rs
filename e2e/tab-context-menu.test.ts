// Menú contextual de la pestaña: renombrar (lo hace el breadcrumb de esa pestaña) y copiar rutas.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { startApp, type Session } from './harness';

let s: Session;

beforeAll(async () => {
  s = await startApp({ 'uno.md': '# Uno\n', 'dos.md': '# Dos\n' });
  await s.browser.$('.file-tree-item[data-path="uno.md"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(() => s?.close());

it('el menú de la pestaña tiene renombrar y copiar rutas, y renombra la pestaña elegida', async () => {
  await s.browser.$('.file-tree-item[data-path="uno.md"]').click();
  await s.browser.$('.file-tree-item[data-path="dos.md"]').click();
  // Pestaña «uno» (no activa): clic derecho
  const tab = s.browser.$('.synapse-tab*=uno');
  await tab.waitForExist({ timeout: 10_000 });
  await tab.click({ button: 'right' });
  const menu = s.browser.$('.tab-context-menu');
  await menu.waitForDisplayed({ timeout: 5_000 });
  const labels = await menu.getText();
  for (const l of ['Abrir en nueva ventana', 'Abrir con la app por defecto', 'Mostrar en carpeta', 'Renombrar archivo', 'Copiar ruta relativa', 'Copiar ruta completa']) {
    expect(labels).toContain(l);
  }
  await s.browser.$('.tab-rename').click();

  // Se activa «uno» y su breadcrumb abre la edición del nombre
  const input = s.browser.$('.breadcrumb-edit-wrapper input');
  await input.waitForDisplayed({ timeout: 5_000 });
  expect(await input.getValue()).toBe('uno.md');
  await input.setValue('renombrada.md');
  await s.browser.keys('Enter');
  await s.browser.waitUntil(() => existsSync(join(s.vault, 'renombrada.md')), {
    timeout: 5_000,
    timeoutMsg: 'no se renombró el archivo',
  });
  expect(existsSync(join(s.vault, 'uno.md'))).toBe(false);
});
