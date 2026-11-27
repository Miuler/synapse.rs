// La pestaña vacía lista los 3 últimos archivos abiertos (sección plegable, que se recuerda) y su
// botón de crear ofrece los tipos de archivo.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { Key } from 'webdriverio';
import { startApp, type Session } from './harness';

let s: Session;

beforeAll(async () => {
  s = await startApp({ 'a.md': '# A\n', 'b.md': '# B\n', 'carpeta/c.md': '# C\n', 'd.md': '# D\n' });
  await s.browser.$('.file-tree-item[data-path="d.md"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
});

const active = () =>
  s.browser.execute(() => document.querySelector('.tab-title-container.active')?.getAttribute('data-path') ?? '') as Promise<string>;

const recents = () =>
  s.browser.execute(() =>
    [...document.querySelectorAll<HTMLElement>('.recent-file')].filter((e) => e.offsetParent !== null).map((e) => e.dataset.path),
  ) as Promise<string[]>;

async function openFromTree(path: string) {
  await s.browser.execute((p) => (document.querySelector(`.file-tree-item[data-path="${p}"]`) as HTMLElement).click(), path);
  await s.browser.waitUntil(async () => (await active()) === path, { timeout: 10_000, timeoutMsg: `no se abrió ${path}` });
}

async function newEmptyTab() {
  await s.browser.keys([Key.Ctrl, 't']);
  await s.browser.waitUntil(async () => (await active()).startsWith('empty:'), { timeout: 10_000 });
  await s.browser.$('.empty-recents').waitForExist({ timeout: 10_000, timeoutMsg: 'no aparecen los recientes' });
}

it('lista los 3 últimos abiertos y abre el que se elige', async () => {
  await openFromTree('a.md');
  await openFromTree('b.md');
  await s.browser.execute(() => (document.querySelector('.file-tree-item.folder[data-path="carpeta"] .chevron-toggle') as HTMLElement).click());
  await s.browser.$('.file-tree-item[data-path="carpeta/c.md"]').waitForExist({ timeout: 5_000 });
  await openFromTree('carpeta/c.md');
  await openFromTree('d.md');
  await newEmptyTab();
  expect(await recents()).toEqual(['d.md', 'carpeta/c.md', 'b.md']);
  await s.browser.$('.recent-file[data-path="b.md"]').click();
  await s.browser.waitUntil(async () => (await active()) === 'b.md', { timeout: 10_000 });
}, 90_000);

it('plegada, se recuerda así', async () => {
  await newEmptyTab();
  await s.browser.$('.tab-pane:not(.hidden) .recents-toggle').click();
  await s.browser.waitUntil(async () => (await recents()).length === 0, { timeout: 5_000 });
  await s.browser.pause(2000);
  await s.browser.refresh();
  // Las pestañas vacías no se restauran: se abre otra
  await s.browser.$('.file-tree-item[data-path="d.md"]').waitForExist({ timeout: 30_000 });
  await s.browser.$('.tab-title-container.active').waitForExist({ timeout: 10_000 });
  await newEmptyTab();
  const toggle = s.browser.$('.tab-pane:not(.hidden) .recents-toggle');
  expect(await toggle.getAttribute('aria-expanded')).toBe('false');
  expect(await recents()).toEqual([]);
  await toggle.click();
  await s.browser.waitUntil(async () => (await recents()).length === 3, { timeout: 5_000 });
}, 90_000);

it('la cantidad de recientes se cambia en Configuración y 0 oculta la lista', async () => {
  if (!(await active()).startsWith('empty:')) await newEmptyTab();
  const setCount = async (count: number) => {
    await s.browser.$('[aria-label="Configuración"]').click();
    const field = s.browser.$('input[aria-label="Archivos recientes en la pestaña vacía"]');
    await field.waitForDisplayed({ timeout: 5_000 });
    await field.setValue(count);
    await s.browser.keys(Key.Tab);
    await s.browser.keys(Key.Escape);
    await s.browser.$('.settings-panel').waitForExist({ reverse: true, timeout: 5_000 });
  };
  await setCount(2);
  await s.browser.waitUntil(async () => (await recents()).length === 2, { timeout: 5_000 });
  await setCount(0);
  await s.browser.waitUntil(async () => !(await s.browser.$('.empty-recents').isExisting()), { timeout: 5_000 });
  await setCount(3);
  await s.browser.waitUntil(async () => (await recents()).length === 3, { timeout: 5_000 });
}, 60_000);

it('«Crear nuevo archivo» ofrece los tipos y crea el elegido', async () => {
  if (!(await active()).startsWith('empty:')) await newEmptyTab();
  await s.browser.$('.tab-pane:not(.hidden) .create-btn').click();
  await s.browser.$('.empty-new-item[data-kind="mermaid"]').waitForDisplayed({ timeout: 5_000 });
  expect(await s.browser.$$('.empty-new-item').length).toBe(7);
  await s.browser.$('.empty-new-item[data-kind="mermaid"]').click();
  await s.browser.waitUntil(async () => (await active()).endsWith('.mmd'), { timeout: 10_000, timeoutMsg: 'no creó el diagrama' });
}, 60_000);

it('en el panel derecho de la vista dividida, el archivo nuevo queda en ese panel', async () => {
  await openFromTree('a.md');
  await s.browser.$('button[title="Dividir panel verticalmente"]').click();
  await s.browser.$('.split-resizer').waitForExist({ timeout: 5_000, timeoutMsg: 'no se dividió la vista' });
  // El último «+» es el de la barra del panel derecho
  await s.browser.execute(() => ([...document.querySelectorAll<HTMLElement>('.new-tab-btn')].at(-1) as HTMLElement).click());
  // Pestañas de esa barra: las del primer ancestro de ese «+» que las contiene
  const rightTabs = () =>
    s.browser.execute(() => {
      let bar = [...document.querySelectorAll('.new-tab-btn')].at(-1)!.parentElement!;
      while (!bar.querySelector('.tab-title-container')) bar = bar.parentElement!;
      return [...bar.querySelectorAll('.tab-title-container')].map((t) => t.getAttribute('data-path') ?? '');
    }) as Promise<string[]>;
  await s.browser.waitUntil(async () => (await rightTabs()).some((p) => p.startsWith('empty:')), { timeout: 5_000 });
  const before = (await rightTabs()).length;
  for (const button of await s.browser.$$('.create-btn')) {
    if (await button.isDisplayed()) await button.click();
  }
  await s.browser.$('.empty-new-item[data-kind="d2"]').waitForDisplayed({ timeout: 5_000 });
  await s.browser.$('.empty-new-item[data-kind="d2"]').click();
  await s.browser.waitUntil(async () => (await rightTabs()).some((p) => p.endsWith('.d2')), {
    timeout: 10_000,
    timeoutMsg: 'el archivo no quedó en el panel derecho',
  });
  // Ocupa la pestaña vacía: no se añade otra
  expect((await rightTabs()).length).toBe(before);
  expect((await rightTabs()).some((p) => p.startsWith('empty:'))).toBe(false);
}, 60_000);
