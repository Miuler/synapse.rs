// Diagramas draw.io: los .drawio se abren en el editor embebido y al guardar se escribe el XML; los
// .drawio.png se ven como imagen y se editan desde el menú contextual o el visor, reescribiendo el PNG.
// Se usa un draw.io falso local que habla el protocolo embed (init/load/save/export) y guarda solo.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { readFileSync, writeFileSync } from 'node:fs';
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
function pngWithText(text: string): Buffer {
  const ihdr = Buffer.from([0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0]);
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('tEXt', Buffer.from(`mxfile\0${text}`, 'latin1')),
    chunk('IDAT', deflateSync(Buffer.from([0, 255, 255, 255]))),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

const savedPng = `data:image/png;base64,${pngWithText('diagrama-editado').toString('base64')}`;

const fakePage = `<!doctype html><html><body><script>
let mode = null, xml = '', pngOk = false;
const send = (m) => parent.postMessage(JSON.stringify(m), '*');
window.addEventListener('message', (e) => {
  const m = JSON.parse(e.data);
  if (m.action === 'load') {
    if (m.xmlpng) { mode = 'png'; pngOk = m.xmlpng.startsWith('data:image/png;base64,'); }
    else { mode = 'xml'; xml = m.xml; }
    send({ event: 'load' });
    setTimeout(() => send({ event: 'save', xml: mode === 'xml' ? xml.replace('</root>', '<mxCell id="editado" parent="1"/></root>') : '' }), 300);
  }
  if (m.action === 'export') send({ event: 'export', format: 'xmlpng', data: pngOk ? ${JSON.stringify(savedPng)} : 'data:,' });
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
  s = await startApp({ 'flujo.drawio': XML, 'menu.drawio.png': 'x', 'visor.drawio.png': 'x' });
  for (const f of ['menu.drawio.png', 'visor.drawio.png']) writeFileSync(join(s.vault, f), pngWithText('original'));
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
}, 60_000);
