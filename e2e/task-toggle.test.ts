// Ctrl+Enter (atajo `toggle_task`) marca y desmarca la tarea de la línea del cursor en el editor de
// Markdown, también con Vim en modo normal e inserción (sin crear una línea nueva).
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session, paneReady } from './harness';

let s: Session;
const lines = () =>
  s.browser.execute(() => [...document.querySelectorAll('.cm-content .cm-line')].map((l) => l.textContent)) as Promise<string[]>;

beforeAll(async () => {
  s = await startApp({ 'tareas.md': '- [ ] Pendiente 00\n- [x] LISTO 02\nTexto suelto\n' });
  const item = s.browser.$('.file-tree-item[data-path="tareas.md"]');
  await item.waitForExist({ timeout: 30_000 });
  await s.browser.execute(() => localStorage.setItem('synapse_settings', JSON.stringify({ vimMode: true })));
  await s.browser.refresh();
  await item.waitForExist({ timeout: 30_000 });
  await s.browser.execute(() => (document.querySelector('.file-tree-item[data-path="tareas.md"]') as HTMLElement).click());
  await paneReady(s);
  // Las notas se abren en lectura: F2 pasa a edición
  await s.browser.$('.markdown-reading-wrapper').waitForExist({ timeout: 10_000 });
  await s.browser.keys(['F2']);
  await s.browser.waitUntil(
    async () => (await s.browser.execute(() => { const c = document.querySelector('.cm-content') as HTMLElement | null; return !!c && c.offsetParent !== null; })) as boolean,
    { timeout: 10_000, timeoutMsg: 'no se abrió el editor' },
  );
  await s.browser.execute(() => (document.querySelector('.cm-content') as HTMLElement).focus());
  expect(await s.browser.$('.cm-vimMode').isExisting()).toBe(true);
}, 120_000);

afterAll(async () => {
  await s?.close();
});

const ctrlEnter = () => s.browser.keys(['Control', 'Enter']);

it('en modo normal de Vim marca y desmarca la línea del cursor', async () => {
  await s.browser.keys(['Escape']);
  for (const key of ['g', 'g']) await s.browser.keys([key]);
  await ctrlEnter();
  await s.browser.waitUntil(async () => (await lines())[0] === '- [x] Pendiente 00', { timeout: 5_000, timeoutMsg: 'no se marcó' });
  expect((await lines()).length).toBe(4);
  await ctrlEnter();
  await s.browser.waitUntil(async () => (await lines())[0] === '- [ ] Pendiente 00', { timeout: 5_000, timeoutMsg: 'no se desmarcó' });
}, 30_000);

it('en modo inserción también, y una línea normal pasa a ser tarea', async () => {
  for (const key of ['j', 'j', 'i']) await s.browser.keys([key]);
  await ctrlEnter();
  await s.browser.waitUntil(async () => (await lines())[2] === '- [ ] Texto suelto', { timeout: 5_000, timeoutMsg: 'no se convirtió en tarea' });
  expect((await lines()).length).toBe(4);
  await s.browser.keys(['Escape']);
}, 30_000);
