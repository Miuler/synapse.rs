// Excalidraw: con zoom, el cuadro de edición de texto no se amplía con `scale()` (WebKitGTK lo
// rasteriza a escala 1 y se ve borroso), sino con su tamaño de letra real, en el mismo sitio.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

let s: Session;
const SCENE = JSON.stringify({
  type: 'excalidraw',
  version: 2,
  source: 'synapse',
  elements: [],
  appState: { viewBackgroundColor: '#ffffff', zoom: { value: 3 } },
  files: {},
});

beforeAll(async () => {
  s = await startApp({ 'boceto.excalidraw': SCENE });
  await s.browser.$('.file-tree-item[data-path="boceto.excalidraw"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(() => s?.close());

it('el texto que se edita con zoom usa su tamaño real y no una escala', async () => {
  await s.browser.$('.file-tree-item[data-path="boceto.excalidraw"]').click();
  await s.browser.$('.excalidraw canvas.interactive').waitForExist({ timeout: 20_000 });
  const toggle = s.browser.$('.editor-header .view-toggle-btn');
  if ((await toggle.getText()).includes('Lectura')) await toggle.click();
  await s.browser.waitUntil(async () => (await toggle.getText()).includes('Edición'), { timeout: 5_000 });
  const editor = s.browser.$('textarea.excalidraw-wysiwyg');
  await s.browser.waitUntil(
    async () => {
      if (await editor.isExisting()) return true;
      await s.browser.$('.excalidraw canvas.interactive').doubleClick();
      return editor.isExisting();
    },
    { timeout: 10_000, interval: 500, timeoutMsg: 'el doble clic no abrió el editor de texto' },
  );
  await s.browser.keys('Hola');
  const style = (await s.browser.execute(() => {
    const e = document.querySelector('textarea.excalidraw-wysiwyg') as HTMLTextAreaElement;
    return { transform: e.style.transform, fontSize: e.style.fontSize, value: e.value };
  })) as { transform: string; fontSize: string; value: string };
  expect(style.transform).not.toContain('scale');
  expect(style.fontSize).toBe('60px'); // 20 px de la escena × zoom 3
  expect(style.value).toBe('Hola');
}, 60_000);
