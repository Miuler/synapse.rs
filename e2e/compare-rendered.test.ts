// Diff de píxeles entre archivos que se pueden convertir en imagen aunque sean de tipos distintos
// (imagen contra Excalidraw), dos Mermaid y dos draw.io (con un draw.io falso local que exporta un
// SVG distinto según el XML).
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { deflateSync } from 'node:zlib';
import { startApp, type Session } from './harness';

let s: Session;

function crc32(buf: Buffer): number {
  let c = ~0;
  for (const b of buf) {
    c ^= b;
    for (let k = 0; k < 8; k++) c = (c >>> 1) ^ (0xedb88320 & -(c & 1));
  }
  return ~c >>> 0;
}

function chunk(type: string, data: Buffer): Buffer {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, 'latin1'), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([len, body, crc]);
}

/** PNG de un color liso. */
function solidPng(w: number, h: number, [r, g, b]: number[]): Buffer {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr.set([8, 2, 0, 0, 0], 8);
  const row = Buffer.concat([Buffer.from([0]), Buffer.from(Array.from({ length: w }, () => [r, g, b]).flat())]);
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', deflateSync(Buffer.concat(Array.from({ length: h }, () => row)))),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

const drawing = JSON.stringify({
  type: 'excalidraw',
  version: 2,
  source: 'e2e',
  elements: [
    {
      id: 'a', type: 'rectangle', x: 0, y: 0, width: 100, height: 80, angle: 0, strokeColor: '#1971c2',
      backgroundColor: '#1971c2', fillStyle: 'solid', strokeWidth: 2, strokeStyle: 'solid', roughness: 0,
      opacity: 100, seed: 1, version: 1, versionNonce: 1, isDeleted: false, groupIds: [], boundElements: null,
      updated: 1, link: null, locked: false,
    },
  ],
  appState: {},
  files: {},
});

// draw.io falso: al exportar devuelve un SVG rojo o azul según lo que diga el XML cargado
const fakePage = `<!doctype html><html><body><script>
let xml = '';
const send = (m) => parent.postMessage(JSON.stringify(m), '*');
window.addEventListener('message', (e) => {
  const m = JSON.parse(e.data);
  if (m.action === 'load') { xml = m.xml || ''; send({ event: 'load' }); }
  if (m.action === 'export') {
    const color = xml.includes('rojo') ? '#e03131' : '#1971c2';
    const svg = '<svg xmlns="http://www.w3.org/2000/svg" width="120" height="80"><rect width="60" height="80" fill="' + color + '"/></svg>';
    send({ event: 'export', format: m.format, data: 'data:image/svg+xml;base64,' + btoa(svg) });
  }
});
send({ event: 'init' });
</script></body></html>`;

const server = Bun.serve({
  port: 0,
  fetch: () => new Response(fakePage, { headers: { 'content-type': 'text/html' } }),
});

const drawio = (label: string) =>
  `<mxfile><diagram name="P"><mxGraphModel><root><mxCell id="0"/><mxCell id="1" parent="0"/><mxCell id="2" value="${label}" vertex="1" parent="1"/></root></mxGraphModel></diagram></mxfile>`;

beforeAll(async () => {
  s = await startApp({
    'foto.png': 'x',
    'dibujo.excalidraw': drawing,
    'a.mmd': 'graph TD\n  A --> B\n',
    'b.mmd': 'graph TD\n  A --> B\n  B --> C\n',
    'azul.drawio': drawio('azul'),
    'rojo.drawio': drawio('rojo'),
  });
  writeFileSync(join(s.vault, 'foto.png'), solidPng(140, 120, [255, 255, 255]));
  await s.browser.$('.file-tree-item[data-path="foto.png"]').waitForExist({ timeout: 30_000 });
  await s.browser.execute((url) => {
    localStorage.setItem('synapse_settings', JSON.stringify({ drawioUrl: url }));
  }, `http://127.0.0.1:${server.port}/`);
  await s.browser.refresh();
  await s.browser.$('.file-tree-item[data-path="foto.png"]').waitForExist({ timeout: 30_000 });
}, 120_000);

afterAll(async () => {
  await s?.close();
  server.stop(true);
});

const selected = () =>
  s.browser.execute(() => [...document.querySelectorAll('.file-tree-item.selected')].map((e) => e.getAttribute('data-path')).sort()) as Promise<string[]>;

/** Selecciona los dos archivos y abre «Comparar archivos». */
async function compare(a: string, b: string) {
  for (const [i, p] of [a, b].entries()) {
    await s.browser.execute(
      (path: string, ctrl: boolean) => {
        document.querySelector(`.file-tree-item[data-path="${path}"]`)!.dispatchEvent(new MouseEvent('click', { bubbles: true, ctrlKey: ctrl }));
      },
      p,
      i > 0,
    );
    if (i === 0) await s.browser.$(`.tab-title-container.active[data-path="${p}"]`).waitForExist({ timeout: 10_000 });
    const want = [a, b].slice(0, i + 1).sort().join(',');
    await s.browser.waitUntil(async () => (await selected()).join(',') === want, { timeout: 5_000, timeoutMsg: `selección ≠ ${want}` });
  }
  await s.browser.execute((p: string) => {
    const el = document.querySelector(`.file-tree-item[data-path="${p}"]`)!;
    const { x, y } = el.getBoundingClientRect();
    el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: x + 5, clientY: y + 5 }));
  }, b);
  await s.browser.$('.context-menu-item*=Comparar archivos').waitForExist({ timeout: 5_000 });
  await s.browser.$('.context-menu-item*=Comparar archivos').click();
}

/** Pulsa un modo de la barra del diff visible (cuando aparece). */
async function clickMode(label: string) {
  await s.browser.waitUntil(
    async () =>
      (await s.browser.execute((l: string) => {
        const btn = [...document.querySelectorAll<HTMLElement>('.tab-pane:not(.hidden) .toggle-btn')].find((b) => b.textContent?.includes(l));
        btn?.click();
        return !!btn;
      }, label)) as boolean,
    { timeout: 10_000, timeoutMsg: `no apareció el modo ${label}` },
  );
}

/** Pulsa «Diferencias» en el diff visible y devuelve el porcentaje de píxeles distintos. */
async function pixelDiffPercent(): Promise<number> {
  await clickMode('Diferencias');
  await s.browser.$('.tab-pane:not(.hidden) .diff-img').waitForExist({ timeout: 30_000 });
  const stats = (await s.browser.execute(
    () => document.querySelector('.tab-pane:not(.hidden) .diff-mode-pane:not(.hidden) .stat-pill')?.textContent ?? '',
  )) as string;
  return Number(/\(([\d.,]+)%\)/.exec(stats)?.[1]?.replace(',', '.'));
}

it('una imagen contra un dibujo de Excalidraw se compara píxel a píxel', async () => {
  await compare('foto.png', 'dibujo.excalidraw');
  const pct = await pixelDiffPercent();
  // El rectángulo azul sobre blanco: ni todo ni nada
  expect(pct).toBeGreaterThan(5);
  expect(pct).toBeLessThan(95);
  // «2 Paneles» muestra los dos lados ya convertidos en imagen
  await clickMode('2 Paneles');
  await s.browser.waitUntil(
    async () => (await s.browser.execute(() => document.querySelectorAll('.tab-pane:not(.hidden) .visual-diff-root img').length)) >= 2,
    { timeout: 20_000, timeoutMsg: 'no se vieron las dos imágenes' },
  );
}, 90_000);

it('dos Mermaid se comparan píxel a píxel', async () => {
  await compare('a.mmd', 'b.mmd');
  expect(await pixelDiffPercent()).toBeGreaterThan(0);
}, 90_000);

it('dos draw.io se renderizan con draw.io y se comparan píxel a píxel, sin perder el diff de texto', async () => {
  await compare('azul.drawio', 'rojo.drawio');
  const pct = await pixelDiffPercent();
  // Media imagen cambia de azul a rojo
  expect(pct).toBeGreaterThan(30);
  expect(pct).toBeLessThan(70);
  await clickMode('Texto');
}, 90_000);
