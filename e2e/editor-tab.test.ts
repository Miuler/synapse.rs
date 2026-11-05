// Tab inserta 4 espacios sin perder el foco, en los editores de código y Markdown, y respeta los modos de Vim.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

let s: Session;
const state = () =>
  s.browser.execute(() => {
    const content = [...document.querySelectorAll('.cm-content')].find((c) => c.getClientRects().length > 0)!;
    return {
      first: content.querySelector('.cm-line')!.textContent,
      focused: content.contains(document.activeElement),
    };
  }) as Promise<{ first: string; focused: boolean }>;

async function open(path: string) {
  await s.browser.$(`.file-tree-item[data-path="${path}"]`).click();
  await s.browser.pause(500);
  const shown = () =>
    s.browser.execute(() => [...document.querySelectorAll('.cm-content')].some((c) => c.getClientRects().length > 0)) as Promise<boolean>;
  if (!(await shown())) await s.browser.keys(['F2']);
  await s.browser.waitUntil(shown, { timeout: 10_000 });
  for (const line of await s.browser.$$('.cm-content .cm-line')) {
    if (await line.isDisplayed()) {
      await line.click();
      break;
    }
  }
  await s.browser.keys(['Home']);
}

beforeAll(async () => {
  s = await startApp({ 'grafo.d2': 'a -> b\n', 'nota.md': 'hola\n', 'vim.d2': 'c -> d\n' });
  await s.browser.$('.file-tree-item[data-path="grafo.d2"]').waitForExist({ timeout: 30_000 });
}, 120_000);

afterAll(async () => {
  await s?.close();
});

it('en el editor de código inserta 4 espacios y Shift+Tab los quita', async () => {
  await open('grafo.d2');
  await s.browser.keys(['Tab']);
  await s.browser.waitUntil(async () => (await state()).first === '    a -> b', { timeout: 5_000 });
  expect((await state()).focused).toBe(true);
  await s.browser.keys(['Shift', 'Tab']);
  await s.browser.keys(['Shift']);
  await s.browser.waitUntil(async () => (await state()).first === 'a -> b', { timeout: 5_000 });
}, 60_000);

it('en el editor de Markdown inserta 4 espacios', async () => {
  await open('nota.md');
  await s.browser.keys(['Tab']);
  await s.browser.waitUntil(async () => (await state()).first === '    hola', { timeout: 5_000 });
  expect((await state()).focused).toBe(true);
}, 60_000);

it('con Vim solo inserta en modo inserción y no pierde el foco', async () => {
  await s.browser.execute(() => {
    const raw = JSON.parse(localStorage.getItem('synapse_settings') || '{}');
    localStorage.setItem('synapse_settings', JSON.stringify({ ...raw, vimMode: true }));
  });
  await s.browser.refresh();
  await s.browser.$('.file-tree-item[data-path="vim.d2"]').waitForExist({ timeout: 30_000 });
  await open('vim.d2');
  await s.browser.keys(['Escape']);
  await s.browser.keys(['Tab']);
  await s.browser.pause(300);
  expect(await state()).toEqual({ first: 'c -> d', focused: true });
  await s.browser.keys(['i']);
  await s.browser.keys(['Tab']);
  await s.browser.waitUntil(async () => (await state()).first.includes('    '), { timeout: 5_000 });
  expect((await state()).focused).toBe(true);
}, 60_000);
