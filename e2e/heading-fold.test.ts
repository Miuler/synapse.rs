// Plegado por títulos en la vista de lectura y en el editor de Markdown.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

let s: Session;
beforeAll(async () => {
  s = await startApp({
    'guia.md': '# Inicio\n\nintro\n\n## Parte A\n\ntexto a\n\n### Detalle A1\n\ntexto a1\n\n## Parte B\n\ntexto b\n',
  });
  await s.browser.$('.file-tree-item[data-path="guia.md"]').waitForExist({ timeout: 30_000 });
  await s.browser.$('.file-tree-item[data-path="guia.md"]').click();
  await s.browser.$('.markdown-body .reading-fold').waitForExist({ timeout: 15_000 });
}, 120_000);
afterAll(() => s?.close());

const visible = () =>
  s.browser.execute(() =>
    [...document.querySelectorAll('.markdown-body > *')]
      .filter((e) => e.getClientRects().length)
      .map((e) => e.textContent?.trim()),
  ) as Promise<string[]>;
const toggle = (title: string, altKey = false) =>
  s.browser.execute(
    (t, alt) => {
      const h = [...document.querySelectorAll('.markdown-body > :is(h1,h2,h3)')].find((e) => e.textContent?.trim() === t)!;
      h.querySelector('.reading-fold')!.dispatchEvent(new MouseEvent('click', { bubbles: true, altKey: alt }));
    },
    title,
    altKey,
  );
const keys = (...k: string[]) =>
  s.browser.execute((list) => {
    for (const key of list) document.querySelector('.synapse-reading')!.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }));
  }, k);
const ALL = ['Inicio', 'intro', 'Parte A', 'texto a', 'Detalle A1', 'texto a1', 'Parte B', 'texto b'];

it('la flecha pliega un título con sus subtítulos y lo vuelve a abrir', async () => {
  expect(await visible()).toEqual(ALL);
  await toggle('Parte A');
  expect(await visible()).toEqual(['Inicio', 'intro', 'Parte A', 'Parte B', 'texto b']);
  if (process.env.SHOT_DIR) await s.browser.saveScreenshot(`${process.env.SHOT_DIR}/fold.png`);
  await toggle('Parte A');
  expect(await visible()).toEqual(ALL);
});

it('Alt pliega todos los del nivel; zM y zR, todos', async () => {
  await toggle('Parte A', true);
  expect(await visible()).toEqual(['Inicio', 'intro', 'Parte A', 'Parte B']);
  await keys('z', 'R');
  expect(await visible()).toEqual(ALL);
  await keys('z', 'M');
  expect(await visible()).toEqual(['Inicio']);
});

it('buscar despliega lo que oculta la coincidencia', async () => {
  await keys('/');
  await s.browser.$('.reading-search-input').waitForExist({ timeout: 5_000 });
  await s.browser.$('.reading-search-input').setValue('texto a1');
  await s.browser.waitUntil(async () => (await visible()).includes('texto a1'), { timeout: 5_000, timeoutMsg: 'no se desplegó' });
  await s.browser.keys('Escape');
  await keys('z', 'R');
});

it('lo plegado se conserva al pasar a edición y volver; el editor también pliega títulos', async () => {
  await toggle('Parte A');
  await s.browser.keys('F2');
  await s.browser.$('.cm-foldGutter span[title="Fold line"], .cm-foldGutter span[title="Plegar"]').waitForExist({ timeout: 10_000 });
  const lines = () => s.browser.execute(() => document.querySelectorAll('.tab-pane:not(.hidden) .cm-line').length) as Promise<number>;
  const before = await lines();
  await s.browser.execute(() => {
    const marks = [...document.querySelectorAll('.tab-pane:not(.hidden) .cm-foldGutter span[title]')].filter((e) => e.getClientRects().length && (e.parentElement as HTMLElement).style.visibility !== 'hidden');
    (marks[0] as HTMLElement).click();
  });
  await s.browser.waitUntil(async () => (await lines()) < before, { timeout: 5_000, timeoutMsg: 'el editor no plegó el título' });
  await s.browser.keys('F2');
  await s.browser.$('.markdown-body .reading-fold').waitForExist({ timeout: 10_000 });
  expect(await visible()).not.toContain('texto a1');
});
