// Ctrl++, Ctrl+-, Ctrl+0 y Ctrl+rueda cambian el tamaño del texto del editor y de la lectura.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { Key } from 'webdriverio';
import { startApp, type Session } from './harness';

let s: Session;
const size = () =>
  s.browser.execute(() => {
    const el = [...document.querySelectorAll('.cm-line, .markdown-body')].find((e) => e.getClientRects().length > 0)!;
    return parseFloat(getComputedStyle(el).fontSize);
  }) as Promise<number>;
const wheel = (deltaY: number) =>
  s.browser.execute((d: number) => {
    const el = [...document.querySelectorAll('.cm-editor, .markdown-body')].find((e) => e.getClientRects().length > 0)!;
    el.dispatchEvent(new WheelEvent('wheel', { bubbles: true, cancelable: true, ctrlKey: true, deltaY: d }));
  }, deltaY);
const press = async (k: string) => {
  await s.browser.keys([Key.Ctrl, k]);
  await s.browser.pause(200);
};

beforeAll(async () => {
  s = await startApp({ 'n.md': '# Título\n\ntexto\n', 'c.txt': 'codigo\n' });
  await s.browser.$('.file-tree-item[data-path="c.txt"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(() => s?.close());

it('funciona en el editor de código', async () => {
  await s.browser.$('.file-tree-item[data-path="c.txt"]').click();
  await s.browser.$('.cm-line').waitForExist({ timeout: 10_000 });
  await s.browser.pause(500);
  const base = await size();
  await press('=');
  expect(await size()).toBeGreaterThan(base);
  await press('0');
  expect(await size()).toBe(base);
  await press('-');
  expect(await size()).toBeLessThan(base);
  await press('0');
  await wheel(-100);
  await s.browser.pause(200);
  expect(await size()).toBeGreaterThan(base);
  await press('0');
  expect(await size()).toBe(base);
});

it('funciona en la nota Markdown', async () => {
  await s.browser.$('.file-tree-item[data-path="n.md"]').click();
  await s.browser.pause(1000);
  const base = await size();
  await wheel(-100);
  await s.browser.pause(200);
  expect(await size()).toBeGreaterThan(base);
  await press('0');
  expect(await size()).toBe(base);
});
