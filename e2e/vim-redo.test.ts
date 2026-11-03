// En el editor de D2 con Vim, `u` deshace y Ctrl+R rehace (no recarga el archivo).
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

let s: Session;
const text = () =>
  s.browser.execute(() => [...document.querySelectorAll('.cm-content .cm-line')].map((l) => l.textContent).join('\n')) as Promise<string>;

beforeAll(async () => {
  s = await startApp({ 'grafo.d2': 'a -> b\n' });
  const item = s.browser.$('.file-tree-item[data-path="grafo.d2"]');
  await item.waitForExist({ timeout: 30_000 });
  await s.browser.execute(() => localStorage.setItem('synapse_settings', JSON.stringify({ vimMode: true })));
  await s.browser.refresh();
  await item.waitForExist({ timeout: 30_000 });
  await item.click();
  await s.browser.pause(500);
  const content = s.browser.$('.cm-editor .cm-content');
  if (!(await content.isExisting())) await s.browser.keys(['F2']);
  await content.waitForExist({ timeout: 10_000 });
  await s.browser.$('.cm-content .cm-line').click();
  expect(await s.browser.$('.cm-vimMode').isExisting()).toBe(true);
}, 120_000);

afterAll(async () => {
  await s?.close();
});

it('u deshace y Ctrl+R rehace', async () => {
  // Una tecla por llamada: en una misma secuencia WebDriver junta las repetidas (las dos Escape).
  for (const key of ['Escape', 'A', ' ', 'c', 'Escape']) await s.browser.keys([key]);
  await s.browser.waitUntil(async () => (await text()).includes('a -> b c'), { timeout: 5_000 });
  await s.browser.keys(['u']);
  await s.browser.waitUntil(async () => !(await text()).includes('a -> b c'), { timeout: 5_000 });
  await s.browser.keys(['Control', 'r']);
  await s.browser.waitUntil(async () => (await text()).includes('a -> b c'), { timeout: 5_000 });
  // Sigue en el editor con el cambio rehecho (una recarga lo habría descartado o sacado del editor).
  await s.browser.pause(1000);
  expect(await text()).toContain('a -> b c');
});
