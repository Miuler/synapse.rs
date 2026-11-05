// Ctrl+Shift+F abre la búsqueda global con una nota abierta, y Ctrl+R abre reemplazar en un editor.
// Las propiedades de la nota muestran las listas en línea.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { Key } from 'webdriverio';
import { startApp, type Session } from './harness';

let s: Session;
const focusedName = () => s.browser.execute(() => (document.activeElement as HTMLInputElement)?.name ?? '') as Promise<string>;

beforeAll(async () => {
  s = await startApp({ 'nota.md': '---\ntags: [uno, dos, tres]\n---\n\nHola\n', 'grafo.d2': 'a -> b\n' });
  const item = s.browser.$('.file-tree-item[data-path="nota.md"]');
  await item.waitForExist({ timeout: 30_000 });
  await item.click();
}, 120_000);

afterAll(async () => {
  await s?.close();
});

it('muestra cada etiqueta de una lista en línea', async () => {
  await s.browser.waitUntil(
    () => s.browser.execute(() => /uno[\s\S]*dos[\s\S]*tres/.test(document.body.innerText) && !document.body.innerText.includes('[uno')) as Promise<boolean>,
    { timeout: 10_000, timeoutMsg: 'etiquetas sin separar' },
  );
}, 30_000);

it('Ctrl+Shift+F abre la búsqueda en el contenido con una nota abierta', async () => {
  await s.browser.keys([Key.Ctrl, Key.Shift, 'f']);
  const input = s.browser.$('input[placeholder*="Ctrl+Shift+F"]');
  await input.waitForExist({ timeout: 5_000 });
  await s.browser.keys(['Escape']);
  await input.waitForExist({ timeout: 5_000, reverse: true });
}, 30_000);

it('Ctrl+R en un editor abre reemplazar y el contenido no se recarga', async () => {
  await s.browser.$('.file-tree-item[data-path="grafo.d2"]').click();
  await s.browser.pause(500);
  const shown = () => s.browser.execute(() => [...document.querySelectorAll('.cm-content')].some((c) => c.getClientRects().length > 0)) as Promise<boolean>;
  if (!(await shown())) await s.browser.keys(['F2']);
  await s.browser.waitUntil(shown, { timeout: 10_000 });
  const lines = await s.browser.$$('.cm-content .cm-line');
  for (const line of lines) if (await line.isDisplayed()) { await line.click(); break; }
  await s.browser.keys([Key.Ctrl, 'r']);
  await s.browser.waitUntil(async () => (await focusedName()) === 'replace', { timeout: 5_000, timeoutMsg: 'sin foco en reemplazar' });
  expect(await focusedName()).toBe('replace');
}, 60_000);
