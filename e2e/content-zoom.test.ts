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
const press = (k: string) => s.browser.keys([Key.Ctrl, k]);
/** Espera a que el tamaño del texto cumpla `ok` (el cambio no es inmediato). */
const sizeIs = (ok: (v: number) => boolean, what: string) =>
  s.browser.waitUntil(async () => ok(await size()), { timeout: 3_000, timeoutMsg: `el tamaño del texto no quedó ${what}` });
/** Hay un editor o una lectura visible para medir. */
const shown = () =>
  s.browser.waitUntil(
    () => s.browser.execute(() => [...document.querySelectorAll('.cm-line, .markdown-body')].some((e) => e.getClientRects().length > 0)) as Promise<boolean>,
    { timeout: 10_000, timeoutMsg: 'no se mostró el contenido' },
  );

beforeAll(async () => {
  s = await startApp({ 'n.md': '# Título\n\ntexto\n', 'c.txt': 'codigo\n' });
  await s.browser.$('.file-tree-item[data-path="c.txt"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(() => s?.close());

it('funciona en el editor de código', async () => {
  await s.browser.$('.file-tree-item[data-path="c.txt"]').click();
  await s.browser.$('.cm-line').waitForExist({ timeout: 10_000 });
  await shown();
  const base = await size();
  await press('=');
  await sizeIs((v) => v > base, 'más grande');
  await press('0');
  await sizeIs((v) => v === base, 'como al inicio');
  await press('-');
  await sizeIs((v) => v < base, 'más chico');
  await press('0');
  await wheel(-100);
  await sizeIs((v) => v > base, 'más grande');
  await press('0');
  await sizeIs((v) => v === base, 'como al inicio');
});

it('funciona en la nota Markdown', async () => {
  await s.browser.$('.file-tree-item[data-path="n.md"]').click();
  await s.browser.$('.tab-title-container[data-path="n.md"]').waitForExist({ timeout: 10_000 });
  await shown();
  const base = await size();
  await wheel(-100);
  await sizeIs((v) => v > base, 'más grande');
  await press('0');
  await sizeIs((v) => v === base, 'como al inicio');
});
