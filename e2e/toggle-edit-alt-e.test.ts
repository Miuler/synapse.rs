// Alt+E alterna entre lectura y edición (igual que F2): una pulsación entra a edición y otra vuelve a lectura.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { Key } from 'webdriverio';
import { startApp, type Session, paneReady } from './harness';

let s: Session;
const editorVisible = () =>
  s.browser.execute(() => [...document.querySelectorAll('.cm-editor')].some((c) => c.getClientRects().length > 0)) as Promise<boolean>;

beforeAll(async () => {
  s = await startApp({ 'nota.md': '# Nota\n\nTexto.\n' });
  const item = s.browser.$('.file-tree-item[data-path="nota.md"]');
  await item.waitForExist({ timeout: 30_000 });
  await item.click();
  await paneReady(s);
}, 120_000);

afterAll(async () => {
  await s?.close();
});

it('Alt+E entra al modo edición y una segunda pulsación vuelve a lectura', async () => {
  const wasEditing = await editorVisible();
  await s.browser.keys([Key.Alt, 'e']);
  await s.browser.waitUntil(async () => (await editorVisible()) !== wasEditing, { timeout: 10_000, timeoutMsg: 'Alt+E no cambió de modo' });
  await s.browser.keys([Key.Alt, 'e']);
  await s.browser.waitUntil(async () => (await editorVisible()) === wasEditing, { timeout: 10_000, timeoutMsg: 'la segunda pulsación no volvió al modo inicial' });
  expect(await editorVisible()).toBe(wasEditing);
}, 60_000);
