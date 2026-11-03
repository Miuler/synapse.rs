// Atajos del editor de código: F2 alterna vista previa y edición, Ctrl+E/Ctrl+Y desplazan la
// pantalla una línea y Ctrl+D duplica la línea, con y sin Vim.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

const DOC = Array.from({ length: 200 }, (_, i) => `nodo${i} -> nodo${i + 1}`).join('\n') + '\n';

let s: Session;
const scrollTop = () => s.browser.execute(() => document.querySelector('.cm-scroller')!.scrollTop) as Promise<number>;
const lines = () =>
  s.browser.execute(() => [...document.querySelectorAll('.cm-content .cm-line')].map((l) => l.textContent)) as Promise<string[]>;

async function openEditor() {
  const item = s.browser.$('.file-tree-item[data-path="grafo.d2"]');
  await item.waitForExist({ timeout: 30_000 });
  await item.click();
  await s.browser.pause(500);
  const content = s.browser.$('.cm-editor .cm-content');
  // Los diagramas se abren en vista previa: F2 muestra el editor.
  if (!(await content.isExisting())) await s.browser.keys(['F2']);
  await content.waitForExist({ timeout: 10_000 });
  await s.browser.$('.cm-content .cm-line').click();
}

async function checkKeys() {
  const before = await scrollTop();
  await s.browser.keys(['Control', 'e']);
  await s.browser.keys(['Control', 'e']);
  await s.browser.waitUntil(async () => (await scrollTop()) > before, { timeout: 5_000 });
  const down = await scrollTop();
  await s.browser.keys(['Control', 'y']);
  await s.browser.waitUntil(async () => (await scrollTop()) < down, { timeout: 5_000 });
  // Seguimos en el editor: Ctrl+E no cambió a la vista previa.
  expect(await s.browser.$('.cm-editor .cm-content').isExisting()).toBe(true);

  await s.browser.execute(() => (document.querySelector('.cm-scroller')!.scrollTop = 0));
  await s.browser.$('.cm-content .cm-line').click();
  await s.browser.keys(['Control', 'd']);
  await s.browser.waitUntil(async () => (await lines())[1] === 'nodo0 -> nodo1', { timeout: 5_000 });
  expect((await lines()).slice(0, 3)).toEqual(['nodo0 -> nodo1', 'nodo0 -> nodo1', 'nodo1 -> nodo2']);
  await s.browser.keys(['Escape']);
  await s.browser.keys(['Control', 'z']);
  await s.browser.waitUntil(async () => (await lines())[1] === 'nodo1 -> nodo2', { timeout: 5_000 });
}

beforeAll(async () => {
  s = await startApp({ 'grafo.d2': DOC });
  await openEditor();
}, 120_000);

afterAll(async () => {
  await s?.close();
});

it('Ctrl+E, Ctrl+Y y Ctrl+D sin Vim', async () => {
  await s.browser.execute(() => localStorage.setItem('synapse_settings', JSON.stringify({ vimMode: false })));
  await s.browser.refresh();
  await openEditor();
  expect(await s.browser.$('.cm-vimMode').isExisting()).toBe(false);
  await checkKeys();
});

it('Ctrl+E, Ctrl+Y y Ctrl+D con Vim', async () => {
  await s.browser.execute(() => localStorage.setItem('synapse_settings', JSON.stringify({ vimMode: true })));
  await s.browser.refresh();
  await openEditor();
  expect(await s.browser.$('.cm-vimMode').isExisting()).toBe(true);
  await s.browser.keys(['Escape']);
  await checkKeys();
});

it('F2 vuelve a la vista previa', async () => {
  await s.browser.keys(['F2']);
  await s.browser.waitUntil(async () => !(await s.browser.$('.cm-editor .cm-content').isExisting()), { timeout: 5_000 });
});
