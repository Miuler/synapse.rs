// Los index.md llevan el fondo de la pestaña vacía como marca de agua; un tema ajusta cuánto se
// difumina y las demás notas no lo llevan.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { Key } from 'webdriverio';
import { startApp, type Session } from './harness';

let s: Session;

const INDEX = ['# Índice', '', ...Array.from({ length: 40 }, (_, i) => `- Elemento ${i + 1} del índice con algo de texto`), ''].join('\n');

beforeAll(async () => {
  s = await startApp({ 'index.md': INDEX, 'nota.md': '# Nota\n' });
  await s.browser.$('.file-tree-item[data-path="index.md"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
});

async function open(path: string) {
  await s.browser.execute((p) => (document.querySelector(`.file-tree-item[data-path="${p}"]`) as HTMLElement).click(), path);
  await s.browser.waitUntil(
    async () =>
      (await s.browser.execute(() => document.querySelector('.tab-title-container.active')?.getAttribute('data-path'))) === path,
    { timeout: 10_000, timeoutMsg: `no se abrió ${path}` },
  );
  await s.browser.$('.tab-pane:not(.hidden) .editor-wrapper').waitForExist({ timeout: 10_000 });
}

/** La capa de fondo de la nota visible. */
const layer = () =>
  s.browser.execute(() => {
    const wrapper = document.querySelector('.tab-pane:not(.hidden) .editor-wrapper')!;
    const before = getComputedStyle(wrapper, '::before');
    const reading = wrapper.querySelector('.markdown-reading-wrapper');
    return {
      image: before.backgroundImage,
      blur: before.filter,
      opacity: before.opacity,
      reading: reading ? getComputedStyle(reading).backgroundColor : null,
    };
  }) as Promise<{ image: string; blur: string; opacity: string; reading: string | null }>;

it('el index.md lleva la marca de agua, en lectura y en edición', async () => {
  await open('index.md');
  await s.browser.$('.tab-pane:not(.hidden) .markdown-reading-wrapper li').waitForExist({ timeout: 10_000 });
  const reading = await layer();
  expect(reading.image).toContain('url(');
  expect(reading.blur).toBe('blur(6px)');
  expect(reading.opacity).toBe('0.1');
  expect(reading.reading).toBe('rgba(0, 0, 0, 0)');
  await s.browser.execute(() => document.documentElement.setAttribute('data-theme', 'dark'));
  expect((await layer()).image).toContain('url(');
  await s.browser.keys(Key.F2);
  await s.browser.$('.tab-pane:not(.hidden) .cm-content').waitForDisplayed({ timeout: 10_000 });
  expect((await layer()).image).toContain('url(');
  await s.browser.execute(() => document.documentElement.setAttribute('data-theme', 'light'));
}, 60_000);

it('un tema cambia el desenfoque y la opacidad', async () => {
  await s.browser.execute(() => {
    const style = document.createElement('style');
    style.id = 'tema-de-prueba';
    style.textContent = 'html[data-theme] { --index-wallpaper-blur: 0px; --index-wallpaper-opacity: 0.3; }';
    document.head.append(style);
  });
  const themed = await layer();
  expect(themed.blur).toBe('blur(0px)');
  expect(themed.opacity).toBe('0.3');
  await s.browser.execute(() => document.getElementById('tema-de-prueba')!.remove());
}, 30_000);

it('una nota que no es index.md no la lleva', async () => {
  await open('nota.md');
  expect((await layer()).image).toBe('none');
}, 30_000);

it('con fondo «Ninguno» el index.md tampoco', async () => {
  await s.browser.keys([Key.Ctrl, 't']);
  await s.browser.$('.tab-pane:not(.hidden) .customize-btn').waitForExist({ timeout: 10_000 });
  await s.browser.$('.tab-pane:not(.hidden) .customize-btn').click();
  await s.browser.$('.customize-panel button[data-wallpaper="none"]').click();
  await s.browser.keys(Key.Escape);
  await s.browser.$('.customize-panel').waitForExist({ reverse: true, timeout: 5_000 });
  await open('index.md');
  expect((await layer()).image).toBe('none');
}, 30_000);
