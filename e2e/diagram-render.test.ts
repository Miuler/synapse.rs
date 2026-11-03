// Render de diagramas al editar: no se encola un render por tecla, Ctrl+Enter fuerza el redibujado y
// la barra de estado muestra el render en curso y lo que tardó. El D2 usa TALA, que es lento.
// Con E2E_D2=/ruta/diagrama.d2 se prueba con un diagrama propio en vez del generado.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { readFileSync } from 'node:fs';
import { startApp, type Session } from './harness';

function talaD2(): string {
  const parts = ['direction: right', 'vars: {', '  d2-config: {', '    layout-engine: tala', '  }', '}', ''];
  for (let i = 0; i < 8; i++) {
    parts.push(`zona_${i}: {`, `  web_${i}: Web ${i}`, `  api_${i}: API ${i} { shape: hexagon }`, `  db_${i}: BD ${i} { shape: cylinder }`, `  web_${i} -> api_${i} -> db_${i}`, '}');
  }
  for (let i = 0; i < 7; i++) parts.push(`zona_${i}.api_${i} -> zona_${i + 1}.api_${i + 1}`);
  return parts.join('\n') + '\n';
}

const D2 = process.env.E2E_D2 ? readFileSync(process.env.E2E_D2, 'utf8') : talaD2();

let s: Session;
const status = () =>
  s.browser.execute(() => document.querySelector('.diagram-render-status')?.textContent?.trim() ?? '') as Promise<string>;
/** Texto del diagrama visible (las etiquetas del SVG). */
const previewText = () =>
  s.browser.execute(() =>
    [...document.querySelectorAll('.preview-pane')]
      .filter((p) => (p as HTMLElement).offsetParent)
      .map((p) => p.textContent ?? '')
      .join(' '),
  ) as Promise<string>;
const svgHas = async (text: string) => (await previewText()).includes(text);

beforeAll(async () => {
  s = await startApp({ 'complejo.d2': D2, 'flujo.mmd': 'flowchart TD\n  A[Inicio] --> B[Fin]\n' });
  await s.browser.execute(() => localStorage.setItem('synapse_settings', JSON.stringify({ vimMode: false })));
  await s.browser.refresh();
  const item = s.browser.$('.file-tree-item[data-path="complejo.d2"]');
  await item.waitForExist({ timeout: 30_000 });
  await item.click();
  await s.browser.pause(500);
  const content = s.browser.$('.cm-editor .cm-content');
  if (!(await content.isExisting())) await s.browser.keys(['F2']);
  await content.waitForExist({ timeout: 10_000 });
}, 120_000);

afterAll(async () => {
  await s?.close();
});

it('la barra de estado muestra el render y lo que tardó', async () => {
  await s.browser.waitUntil(async () => /^D2: [\d,]+ s$/.test(await status()), { timeout: 60_000, timeoutMsg: `estado: ${await status()}` });
  console.log('primer render:', await status());
}, 90_000);

it('al escribir rápido se dibuja el texto final sin esperar un render por tecla', async () => {
  await s.browser.$('.cm-content .cm-line').click();
  // Línea nueva: el archivo puede terminar en un comentario.
  await s.browser.keys(['Control', 'End']);
  await s.browser.keys(['Enter']);
  const start = Date.now();
  await s.browser.keys('nodo_final_ok'.split(''));
  await s.browser.waitUntil(() => svgHas('nodo_final_ok'), { timeout: 60_000 });
  const elapsed = Date.now() - start;
  console.log('hasta ver el texto final:', elapsed, 'ms;', await status());
  // Con un render por tecla (13 teclas) serían 13 renders seguidos.
  const oneRender = parseFloat((await status()).replace(/[^\d,]/g, '').replace(',', '.')) * 1000;
  expect(elapsed).toBeLessThan(4 * oneRender + 3000);
}, 90_000);

it('Ctrl+Enter fuerza el redibujado', async () => {
  await s.browser.waitUntil(async () => (await status()).startsWith('D2:'), { timeout: 60_000 });
  await s.browser.keys(['Control', 'Enter']);
  await s.browser.waitUntil(async () => (await status()).startsWith('Renderizando D2'), { timeout: 3_000, interval: 50 });
  await s.browser.waitUntil(async () => (await status()).startsWith('D2:'), { timeout: 60_000 });
  // No inserta un salto de línea.
  const lines = (await s.browser.execute(() => document.querySelectorAll('.cm-content .cm-line').length)) as number;
  expect(lines).toBeGreaterThan(0);
}, 90_000);

it('en Mermaid también se ve el estado y Ctrl+Enter redibuja', async () => {
  await s.browser.$('.file-tree-item[data-path="flujo.mmd"]').click();
  await s.browser.pause(500);
  const content = s.browser.$('.cm-editor .cm-content');
  const visible = () =>
    s.browser.execute(() => !![...document.querySelectorAll('.cm-content')].find((e) => (e as HTMLElement).offsetParent)) as Promise<boolean>;
  if (!(await visible())) await s.browser.keys(['F2']);
  await s.browser.waitUntil(visible, { timeout: 10_000 });
  await s.browser.waitUntil(async () => (await status()).startsWith('Mermaid:'), { timeout: 30_000 });
  const el = await s.browser.execute(() => [...document.querySelectorAll('.cm-content')].find((e) => (e as HTMLElement).offsetParent) as HTMLElement);
  await s.browser.$(el).click();
  await s.browser.keys(['Control', 'Enter']);
  await s.browser.waitUntil(async () => (await status()).startsWith('Mermaid:'), { timeout: 10_000 });
}, 60_000);
