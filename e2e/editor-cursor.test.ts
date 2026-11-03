// El cursor del editor de código (D2/Mermaid) debe verse en tema oscuro: su color no puede ser
// el negro por defecto de CodeMirror sobre el fondo oscuro.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

let s: Session;

beforeAll(async () => {
  s = await startApp({ 'diagrama.d2': 'a -> b\n' });
  await s.browser.$('.file-tree-item[data-path="diagrama.d2"]').waitForExist({ timeout: 30_000 });
  await s.browser.execute(() => {
    localStorage.setItem('synapse_settings', JSON.stringify({ theme: 'dark' }));
  });
  await s.browser.refresh();
  const item = s.browser.$('.file-tree-item[data-path="diagrama.d2"]');
  await item.waitForExist({ timeout: 30_000 });
  await item.click();
  await s.browser.$('.tab-title-container[data-path="diagrama.d2"]').waitForExist({ timeout: 10_000 });
  const content = s.browser.$('.cm-editor .cm-content');
  await s.browser.pause(500);
  if (!(await content.isExisting())) await s.browser.keys(['Control', 'e']);
  await content.waitForExist({ timeout: 10_000 });
  await content.click();
}, 120_000);

afterAll(async () => {
  await s?.close();
});

it('el cursor tiene el color del texto del tema oscuro', async () => {
  await s.browser.$('.cm-cursor').waitForExist({ timeout: 5_000 });
  const r = (await s.browser.execute(() => {
    const cursor = document.querySelector('.cm-cursor') as HTMLElement;
    const text = getComputedStyle(document.documentElement).getPropertyValue('--text-primary').trim();
    return { cursor: getComputedStyle(cursor).borderLeftColor, text };
  })) as { cursor: string; text: string };
  expect(r.cursor).not.toBe('rgb(0, 0, 0)');
});
