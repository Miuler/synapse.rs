// Shift+F6 en el editor de código de D2: selecciona la palabra bajo el cursor y sus coincidencias
// exactas, y lo que se escribe las cambia todas (como renombrar en JetBrains).
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

const DOC = 'server -> db\nservers: x\nclient -> server\n';

let s: Session;
const editorText = () =>
  s.browser.execute(() => [...document.querySelectorAll('.cm-content .cm-line')].map((l) => l.textContent).join('\n')) as Promise<string>;

async function openEditor() {
  const item = s.browser.$('.file-tree-item[data-path="arquitectura.d2"]');
  await item.waitForExist({ timeout: 30_000 });
  await item.click();
  const content = s.browser.$('.cm-editor .cm-content');
  await s.browser.pause(500);
  if (!(await content.isExisting())) await s.browser.keys(['F2']);
  await content.waitForExist({ timeout: 10_000 });
}

async function clickFirstLine() {
  await s.browser.$('.cm-content .cm-line').click();
  // Cursor al inicio de la línea, sobre "server".
  await s.browser.keys(['Home']);
}

beforeAll(async () => {
  s = await startApp({ 'arquitectura.d2': DOC });
  await openEditor();
}, 120_000);

afterAll(async () => {
  await s?.close();
});

it('Shift+F6 y escribir cambia todas las coincidencias exactas', async () => {
  await clickFirstLine();
  await s.browser.keys(['Shift', 'F6']);
  await s.browser.keys(['Shift']);
  await s.browser.keys('web'.split(''));
  await s.browser.waitUntil(async () => (await editorText()).startsWith('web -> db'), { timeout: 5_000 });
  expect(await editorText()).toBe('web -> db\nservers: x\nclient -> web\n'.replace(/\n$/, '\n'));
  // Un solo Ctrl+Z deshace el cambio en todas.
  await s.browser.keys(['Escape']);
  await s.browser.keys(['Control', 'z']);
  await s.browser.waitUntil(async () => (await editorText()).startsWith('server -> db'), { timeout: 5_000 });
  expect(await editorText()).toContain('client -> server');
});

it('con Vim: Shift+F6 y c cambia todas las coincidencias', async () => {
  await s.browser.execute(() => {
    localStorage.setItem('synapse_settings', JSON.stringify({ vimMode: true }));
  });
  await s.browser.refresh();
  await openEditor();
  await clickFirstLine();
  await s.browser.keys(['Escape']);
  await s.browser.keys(['0']);
  await s.browser.keys(['Shift', 'F6']);
  await s.browser.keys(['Shift']);
  await s.browser.keys(['c', 'w', 'e', 'b', 'Escape']);
  await s.browser.waitUntil(async () => (await editorText()).startsWith('web -> db'), { timeout: 5_000 });
  expect(await editorText()).toContain('client -> web');
  expect(await editorText()).toContain('servers: x');
});
