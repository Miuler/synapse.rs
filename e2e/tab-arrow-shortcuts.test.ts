// Ctrl+←/→ cambian de pestaña fuera de los campos de texto; al escribir, la tecla es del editor.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { Key } from 'webdriverio';
import { startApp, type Session, paneReady } from './harness';

let s: Session;

beforeAll(async () => {
  s = await startApp({
    'uno.md': '# Uno\n\nprimera nota\n',
    'dos.md': '# Dos\n\nsegunda nota\n',
  });
  for (const path of ['uno.md', 'dos.md']) {
    const item = s.browser.$(`.file-tree-item[data-path="${path}"]`);
    await item.waitForExist({ timeout: 30_000 });
    // Doble clic: cada nota queda en su propia pestaña
    await item.doubleClick();
    await paneReady(s);
  }
}, 120_000);

afterAll(async () => {
  await s?.close();
});

const heading = () =>
  s.browser.execute(
    () =>
      [...document.querySelectorAll<HTMLElement>('.markdown-body h1, .cm-content')]
        .find((e) => e.offsetParent !== null)
        ?.textContent?.match(/Uno|Dos/)?.[0] ?? '',
  ) as Promise<string>;

async function waitForNote(name: string) {
  await s.browser.waitUntil(async () => (await heading()) === name, {
    timeout: 10_000,
    timeoutMsg: `no se ve la nota ${name}`,
  });
}

it('leyendo, Ctrl+← y Ctrl+→ cambian de pestaña', async () => {
  await waitForNote('Dos');
  await s.browser.keys([Key.Ctrl, Key.ArrowLeft]);
  await waitForNote('Uno');
  await s.browser.keys([Key.Ctrl, Key.ArrowRight]);
  await waitForNote('Dos');
}, 60_000);

it('escribiendo, Ctrl+← se queda en el editor', async () => {
  await s.browser.keys([Key.F2]);
  // Cada pestaña tiene su editor: se pone el foco en el que está a la vista
  await s.browser.waitUntil(
    () =>
      s.browser.execute(() => {
        const content = [...document.querySelectorAll<HTMLElement>('.cm-content')].find((e) => e.offsetParent !== null);
        content?.focus();
        return !!content && document.activeElement === content;
      }) as Promise<boolean>,
    { timeout: 10_000, timeoutMsg: 'sin editor a la vista' },
  );
  await s.browser.keys([Key.Ctrl, Key.ArrowLeft]);
  await s.browser.pause(500);
  expect(await heading()).toBe('Dos');
  // El atajo de siempre sigue cambiando de pestaña también al escribir
  await s.browser.keys([Key.Ctrl, Key.PageUp]);
  await waitForNote('Uno');
}, 60_000);
