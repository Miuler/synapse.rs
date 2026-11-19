// Diagramas draw.io: los .drawio se abren en el editor embebido y al guardar se escribe el XML; los
// .drawio.png se ven como imagen y se editan desde el menú contextual o el visor, reescribiendo el PNG.
// Se usa un draw.io falso local que habla el protocolo embed (init/load/save/export) y guarda solo.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { readFileSync, statSync, writeFileSync } from 'node:fs';
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

/** PNG de 1×1 con un texto `mxfile` incrustado, como hace draw.io en el formato xmlpng. */
function pngWithText(text: string, key = 'mxfile'): Buffer {
  const ihdr = Buffer.from([0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0]);
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('tEXt', Buffer.from(`${key}\0${text}`, 'latin1')),
    chunk('IDAT', deflateSync(Buffer.from([0, 255, 255, 255]))),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

const savedPng = `data:image/png;base64,${pngWithText('diagrama-editado').toString('base64')}`;

/** SVG con el diagrama en `content`, como exporta draw.io en el formato xmlsvg. */
const svgWithText = (text: string) =>
  `<svg xmlns="http://www.w3.org/2000/svg" content="&lt;mxfile&gt;${text}&lt;/mxfile&gt;" width="10" height="10"><rect width="10" height="10"/></svg>`;
const savedSvg = `data:image/svg+xml;base64,${Buffer.from(svgWithText('diagrama-editado')).toString('base64')}`;

const fakePage = `<!doctype html><html><body><script>
let mode = null, xml = '', pngOk = false;
const send = (m) => parent.postMessage(JSON.stringify(m), '*');
window.addEventListener('message', (e) => {
  const m = JSON.parse(e.data);
  if (m.action === 'load') {
    if (m.xmlpng) { mode = 'png'; pngOk = m.xmlpng.startsWith('data:image/png;base64,'); }
    else if (m.xml.includes('<svg')) { mode = 'svg'; pngOk = m.xml.includes('mxfile'); }
    else { mode = 'xml'; xml = m.xml; }
    send({ event: 'load' });
    setTimeout(() => send({ event: 'save', xml: mode === 'xml' ? xml.replace('</root>', '<mxCell id="editado" parent="1"/></root>') : '' }), 300);
  }
  if (m.action === 'export') send({ event: 'export', format: m.format, data: !pngOk ? 'data:,' : m.format === 'xmlsvg' ? ${JSON.stringify(savedSvg)} : ${JSON.stringify(savedPng)} });
});
send({ event: 'init' });
</script></body></html>`;

const server = Bun.serve({
  port: 0,
  fetch: () => new Response(fakePage, { headers: { 'content-type': 'text/html' } }),
});

const XML =
  '<mxfile><diagram name="P"><mxGraphModel><root><mxCell id="0"/><mxCell id="1" parent="0"/></root></mxGraphModel></diagram></mxfile>';

beforeAll(async () => {
  s = await startApp({
    'flujo.drawio': XML,
    'menu.drawio.png': 'x',
    'visor.drawio.png': 'x',
    'disfrazado.png': 'x',
    'normal.png': 'x',
    'plano.svg': svgWithText('original'),
    'arquitectura.png': 'x',
  });
  // PNG con un diagrama de draw.io real (codificado como lo hace draw.io) para la búsqueda
  const model =
    '<mxfile><diagram name="Arquitectura"><mxGraphModel><root><mxCell id="0"/><mxCell id="2" value="Pasarela Quetzalcoatl" vertex="1"/></root></mxGraphModel></diagram></mxfile>';
  writeFileSync(join(s.vault, 'arquitectura.png'), pngWithText(encodeURIComponent(model)));
  for (const f of ['menu.drawio.png', 'visor.drawio.png', 'disfrazado.png']) writeFileSync(join(s.vault, f), pngWithText('original'));
  writeFileSync(join(s.vault, 'normal.png'), pngWithText('GIMP', 'Software'));
  await s.browser.$('.file-tree-item[data-path="flujo.drawio"]').waitForExist({ timeout: 30_000 });
  await s.browser.execute((url) => {
    localStorage.setItem('synapse_settings', JSON.stringify({ drawioUrl: url }));
  }, `http://127.0.0.1:${server.port}/`);
  await s.browser.refresh();
  await s.browser.$('.file-tree-item[data-path="flujo.drawio"]').waitForExist({ timeout: 30_000 });
}, 120_000);

afterAll(async () => {
  await s?.close();
  server.stop(true);
});

const pngText = (rel: string) => readFileSync(join(s.vault, rel)).toString('latin1');

it('un .drawio se abre en draw.io y al guardar se escribe el XML', async () => {
  await s.browser.$('.file-tree-item[data-path="flujo.drawio"]').click();
  await s.browser.$('.drawio-viewer[data-drawio-path="flujo.drawio"] iframe').waitForExist({ timeout: 15_000 });
  await s.browser.waitUntil(async () => readFileSync(join(s.vault, 'flujo.drawio'), 'utf8').includes('id="editado"'), {
    timeout: 15_000,
    timeoutMsg: 'no se guardó el XML',
  });
}, 60_000);

it('un .drawio.png se ve como imagen y se edita desde el menú contextual', async () => {
  await s.browser.$('.file-tree-item[data-path="menu.drawio.png"]').click();
  await s.browser.$('.drawio-edit-btn').waitForExist({ timeout: 15_000 });
  expect(await s.browser.$('.drawio-viewer[data-drawio-path="menu.drawio.png"]').isExisting()).toBe(false);

  await s.browser.$('.file-tree-item[data-path="menu.drawio.png"]').click({ button: 'right' });
  const item = s.browser.$('//div[@role="menuitem"][contains(., "Abrir con draw.io")]');
  await item.waitForExist({ timeout: 5_000 });
  await item.click();
  await s.browser.waitUntil(async () => pngText('menu.drawio.png').includes('diagrama-editado'), {
    timeout: 15_000,
    timeoutMsg: 'no se reescribió el PNG',
  });
  expect(pngText('menu.drawio.png').startsWith('\x89PNG')).toBe(true);

  // Volver a abrirlo desde el explorador solo activa la pestaña: el editor sigue, sin redibujarse
  const viewer = '.drawio-viewer[data-drawio-path="menu.drawio.png"]';
  await s.browser.execute((sel: string) => document.querySelector(sel)?.setAttribute('data-marca', 'mismo'), viewer);
  await s.browser.$('.file-tree-item[data-path="menu.drawio.png"]').click();
  await s.browser.$('.tab-title-container.active[data-path="menu.drawio.png"]').waitForExist({ timeout: 5_000 });
  expect(await s.browser.execute((sel: string) => document.querySelector(sel)?.getAttribute('data-marca') ?? null, viewer)).toBe('mismo');
  // Lectura (cabecera) lo devuelve al visor de imagen
  await s.browser.$('.editor-header .view-toggle-btn').click();
  await s.browser.waitUntil(async () => !(await s.browser.$(viewer).isExisting()), {
    timeout: 5_000,
    timeoutMsg: 'siguió abierto en draw.io',
  });
  await s.browser.$('.tab-pane:not(.hidden) .drawio-edit-btn').waitForExist({ timeout: 5_000 });
}, 60_000);

it('el visor de imágenes ofrece editar un .drawio.png con draw.io', async () => {
  await s.browser.$('.file-tree-item[data-path="visor.drawio.png"]').click();
  const btn = s.browser.$('.tab-pane:not(.hidden) .drawio-edit-btn');
  await btn.waitForExist({ timeout: 15_000 });
  await btn.click();
  await s.browser.waitUntil(async () => pngText('visor.drawio.png').includes('diagrama-editado'), {
    timeout: 15_000,
    timeoutMsg: 'no se reescribió el PNG',
  });
  // El cambio en disco que deja su propio guardado no vuelve a montar (redibujar) el editor
  const viewer = '.drawio-viewer[data-drawio-path="visor.drawio.png"]';
  await s.browser.execute((sel: string) => document.querySelector(sel)?.setAttribute('data-marca', 'mismo'), viewer);
  // Comprobación de que algo NO pasa: se da tiempo al vigilante de archivos para que avise
  await s.browser.pause(2_000);
  expect(await s.browser.execute((sel: string) => document.querySelector(sel)?.getAttribute('data-marca') ?? null, viewer)).toBe('mismo');
}, 60_000);

it('un PNG con un diagrama de draw.io dentro se edita con el botón Lectura/Edición, aunque no se llame .drawio.png', async () => {
  const toggle = () => s.browser.$('.editor-header .view-toggle-btn');
  // Un PNG normal no ofrece editar
  await s.browser.$('.file-tree-item[data-path="normal.png"]').click();
  await s.browser.$('.tab-title-container[data-path="normal.png"]').waitForExist({ timeout: 10_000 });
  await s.browser.$('.tab-pane:not(.hidden) .image-viewer-container').waitForExist({ timeout: 10_000 });
  expect(await toggle().isExisting()).toBe(false);
  // El que lleva `mxfile` sí: el botón muestra «Lectura» y al pulsarlo abre draw.io
  await s.browser.$('.file-tree-item[data-path="disfrazado.png"]').click();
  await toggle().waitForExist({ timeout: 10_000 });
  expect(await toggle().getText()).toContain('Lectura');
  await s.browser.$('.tab-pane:not(.hidden) .drawio-edit-btn').waitForExist({ timeout: 10_000 });
  await toggle().click();
  await s.browser.$('.drawio-viewer[data-drawio-path="disfrazado.png"]').waitForExist({ timeout: 10_000 });
  expect(await toggle().getText()).toContain('Edición');
  await s.browser.waitUntil(async () => pngText('disfrazado.png').includes('diagrama-editado'), {
    timeout: 15_000,
    timeoutMsg: 'no se reescribió el PNG',
  });
}, 60_000);

it('un SVG con un diagrama de draw.io dentro se edita y se guarda como SVG', async () => {
  const toggle = () => s.browser.$('.editor-header .view-toggle-btn');
  await s.browser.$('.file-tree-item[data-path="plano.svg"]').click();
  await toggle().waitForExist({ timeout: 10_000 });
  await toggle().click();
  await s.browser.$('.drawio-viewer[data-drawio-path="plano.svg"]').waitForExist({ timeout: 10_000 });
  await s.browser.waitUntil(async () => readFileSync(join(s.vault, 'plano.svg'), 'utf8').includes('diagrama-editado'), {
    timeout: 15_000,
    timeoutMsg: 'no se reescribió el SVG',
  });
  expect(readFileSync(join(s.vault, 'plano.svg'), 'utf8').startsWith('<svg')).toBe(true);
}, 60_000);

it('el texto de un diagrama de draw.io dentro de un PNG se encuentra con la búsqueda de texto completo', async () => {
  const search = () =>
    s.browser.execute(async () => {
      const r = await (window as any).__TAURI_INTERNALS__.invoke('full_text_search', { query: 'Quetzalcoatl', limit: 10 });
      return (r.hits as { note_path: string; kind: string }[]).map((h) => `${h.note_path}:${h.kind}`);
    }) as Promise<string[]>;
  await s.browser.waitUntil(async () => (await search()).includes('arquitectura.png:drawio'), {
    timeout: 30_000,
    timeoutMsg: 'la búsqueda no encontró el texto del diagrama',
  });
  // Y la imagen queda marcada como de draw.io al indexar: la cabecera ofrece Lectura/Edición
  await s.browser.$('.file-tree-item[data-path="arquitectura.png"]').click();
  await s.browser.$('.editor-header .view-toggle-btn').waitForExist({ timeout: 10_000 });
}, 60_000);

it('una copia de solo lectura de una imagen en draw.io no se puede editar a la vez: lleva al editor', async () => {
  const toggle = () => s.browser.$('.editor-header .view-toggle-btn');
  // disfrazado.png quedó abierto en el editor de draw.io; desde el explorador, una copia de lectura
  await s.browser.execute(() => {
    const el = document.querySelector('.file-tree-item[data-path="disfrazado.png"]')!;
    const { x, y } = el.getBoundingClientRect();
    el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: x + 5, clientY: y + 5 }));
  });
  await s.browser.$('.open-readonly-copy').waitForExist({ timeout: 5_000 });
  await s.browser.execute(() => (document.querySelector('.open-readonly-copy') as HTMLElement).click());
  await s.browser.$('.tab-title-container.active[data-path="copy:2:disfrazado.png"]').waitForExist({ timeout: 10_000 });
  expect(await toggle().getText()).toContain('Lectura');
  // Pasar a edición en la copia lleva a la pestaña que ya edita la imagen
  await toggle().click();
  await s.browser.$('.tab-title-container.active[data-path="disfrazado.png"]').waitForExist({ timeout: 5_000 });
  expect(await s.browser.$$('.drawio-viewer[data-drawio-path="disfrazado.png"]').length).toBe(1);
}, 60_000);

it('al guardar en draw.io, otra pestaña con la imagen se recarga sin perder el zoom ni la posición', async () => {
  const toggle = () => s.browser.$('.editor-header .view-toggle-btn');
  const copy = 'copy:2:disfrazado.png';
  const view = () =>
    s.browser.execute(() => {
      const pane = [...document.querySelectorAll('.tab-pane')].find((p) => (p as HTMLElement).offsetParent !== null)!;
      const img = pane.querySelector('img') as HTMLImageElement | null;
      return {
        zoom: pane.querySelector('.zoom-level-btn')?.textContent?.trim(),
        transform: (pane.querySelector('.image-viewport') as HTMLElement | null)?.style.transform,
        src: img?.getAttribute('src'),
      };
    });
  // En la copia (visor de imagen): acercar y desplazar
  await s.browser.$(`.tab-title-container[data-path="${copy}"]`).click();
  await s.browser.$(`.tab-title-container.active[data-path="${copy}"]`).waitForExist({ timeout: 5_000 });
  await s.browser.$('.tab-pane:not(.hidden) .zoom-level-btn').waitForExist({ timeout: 10_000 });
  const zoom0 = (await view()).zoom;
  await s.browser.execute(() => {
    const pane = document.querySelector('.tab-pane:not(.hidden)')!;
    const zoomIn = pane.querySelector('[aria-label^="Acercar"]') as HTMLElement;
    zoomIn.click();
    zoomIn.click();
    pane.querySelector('.image-viewer-container')?.dispatchEvent(new WheelEvent('wheel', { deltaY: 40, bubbles: true, cancelable: true }));
  });
  await s.browser.waitUntil(async () => (await view()).zoom !== zoom0 && !!(await view()).transform, {
    timeout: 5_000,
    timeoutMsg: 'no se aplicó el zoom',
  });
  const before = await view();
  // El original sale y vuelve a entrar al editor: el draw.io falso guarda al cargar
  await s.browser.$('.tab-title-container[data-path="disfrazado.png"]').click();
  await s.browser.$('.tab-title-container.active[data-path="disfrazado.png"]').waitForExist({ timeout: 5_000 });
  await toggle().click();
  await s.browser.waitUntil(async () => (await toggle().getText()).includes('Lectura'), { timeout: 5_000 });
  await s.browser.$(`.tab-title-container[data-path="${copy}"]`).click();
  await s.browser.$(`.tab-title-container.active[data-path="${copy}"]`).waitForExist({ timeout: 5_000 });
  const afterClose = await view();
  expect(afterClose.zoom).toBe(before.zoom);
  expect(afterClose.transform).toBe(before.transform);
  await s.browser.$('.tab-title-container[data-path="disfrazado.png"]').click();
  await s.browser.$('.tab-title-container.active[data-path="disfrazado.png"]').waitForExist({ timeout: 5_000 });
  const mtime0 = statSync(join(s.vault, 'disfrazado.png')).mtimeMs;
  await toggle().click();
  await s.browser.$('.drawio-viewer[data-drawio-path="disfrazado.png"]').waitForExist({ timeout: 10_000 });
  // El draw.io falso guarda a los 300 ms de cargar: esperar a que reescriba el PNG
  await s.browser.waitUntil(async () => statSync(join(s.vault, 'disfrazado.png')).mtimeMs !== mtime0, {
    timeout: 10_000,
    timeoutMsg: 'draw.io no reescribió el PNG',
  });
  await s.browser.$(`.tab-title-container[data-path="${copy}"]`).click();
  await s.browser.$(`.tab-title-container.active[data-path="${copy}"]`).waitForExist({ timeout: 5_000 });
  await s.browser.waitUntil(async () => (await view()).src !== afterClose.src, {
    timeout: 10_000,
    timeoutMsg: 'la copia no releyó la imagen guardada',
  });
  const after = await view();
  expect(after.src).not.toBe(afterClose.src); // releyó la imagen guardada
  expect(after.zoom).toBe(before.zoom);
  expect(after.transform).toBe(before.transform);
}, 90_000);
