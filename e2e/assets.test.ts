// Carga de imágenes de la bóveda en el webview: protocolo de assets de Tauri frente a data URL en
// base64 por IPC. Usa rutas con emojis, espacios, '@' y acentos, que son las que motivaron el uso
// de data URL.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { mkdirSync, writeFileSync } from 'fs';
import { dirname, join } from 'path';
import { deflateSync } from 'zlib';
import { startApp, type Session } from './harness';

/** PNG válido de `w`×`h` con ruido (poco comprimible, para que pese como una captura real). */
function noisyPng(w: number, h: number): Buffer {
  const crcTable = Array.from({ length: 256 }, (_, n) => {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    return c >>> 0;
  });
  const crc = (buf: Buffer) => {
    let c = 0xffffffff;
    for (const b of buf) c = crcTable[(c ^ b) & 0xff] ^ (c >>> 8);
    return (c ^ 0xffffffff) >>> 0;
  };
  const chunk = (type: string, data: Buffer) => {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    const td = Buffer.concat([Buffer.from(type), data]);
    const c = Buffer.alloc(4);
    c.writeUInt32BE(crc(td));
    return Buffer.concat([len, td, c]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr[8] = 8; // profundidad
  ihdr[9] = 2; // RGB
  const raw = Buffer.alloc((w * 3 + 1) * h);
  let seed = 12345;
  for (let y = 0; y < h; y++) {
    raw[y * (w * 3 + 1)] = 0;
    for (let x = 0; x < w * 3; x++) {
      seed = (seed * 1103515245 + 12345) & 0x7fffffff;
      raw[y * (w * 3 + 1) + 1 + x] = seed & 0xff;
    }
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', deflateSync(raw, { level: 1 })),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

const IMAGES = [
  'imagen simple.png',
  '👷 Trabajo/Diagramas/captura con espacios.png',
  'Clientes/usuario@correo.com/árbol ñandú.png',
  'Proyectos/BCP - TTIB (2026)/pago #1 [final].png',
];

let s: Session;

beforeAll(async () => {
  s = await startApp({
    'nota.md': '# Nota\n',
    'con-imagenes.md': '# Con imágenes\n\n![simple](imagen%20simple.png)\n\n![[👷 Trabajo/Diagramas/captura con espacios.png]]\n',
  });
  const png = noisyPng(900, 900); // ~2,4 MB
  for (const rel of IMAGES) {
    const p = join(s.vault, rel);
    mkdirSync(dirname(p), { recursive: true });
    writeFileSync(p, png);
  }
  await s.browser.$('.file-tree-item').waitForExist({ timeout: 60_000 });
}, 120_000);

afterAll(async () => {
  await s?.close();
});

it('carga cada imagen por protocolo de assets y por data URL', async () => {
  const results = (await s.browser.executeAsync(
    (vault: string, rels: string[], done: (r: unknown) => void) => {
      const internals = (window as any).__TAURI_INTERNALS__;
      const load = (src: string) =>
        new Promise<{ ok: boolean; ms: number }>((resolve) => {
          const t = performance.now();
          const img = new Image();
          img.onload = () => resolve({ ok: img.naturalWidth > 0, ms: performance.now() - t });
          img.onerror = () => resolve({ ok: false, ms: performance.now() - t });
          img.src = src;
        });
      (async () => {
        const out: unknown[] = [];
        for (const rel of rels) {
          const abs = `${vault}/${rel}`;
          const asset = await load(internals.convertFileSrc(abs, 'asset'));
          const t = performance.now();
          const dataUrl = await internals.invoke('read_asset_data_url', { src: rel, baseFile: null });
          const ipcMs = performance.now() - t;
          const data = dataUrl ? await load(dataUrl) : { ok: false, ms: 0 };
          out.push({ rel, asset, data: { ok: data.ok, ms: ipcMs + data.ms } });
        }
        done(out);
      })();
    },
    s.vault,
    IMAGES
  )) as { rel: string; asset: { ok: boolean; ms: number }; data: { ok: boolean; ms: number } }[];

  for (const r of results) {
    console.log(
      `${r.rel}\n    asset: ${r.asset.ok ? 'OK' : 'FALLA'} ${r.asset.ms.toFixed(0)} ms   data URL: ${r.data.ok ? 'OK' : 'FALLA'} ${r.data.ms.toFixed(0)} ms`
    );
  }
  // El protocolo de assets carga todas, incluidas las rutas con emojis, '@', acentos y '#'
  expect(results.every((r) => r.asset.ok)).toBe(true);
});

const ipcCalls = async () =>
  (await s.browser.execute(() => ((window as any).__synapseIpcLog ?? []).map((e: { cmd: string }) => e.cmd))) as string[];

it('una nota con imágenes las muestra por el protocolo de assets, sin base64 por IPC', async () => {
  const before = (await ipcCalls()).filter((c) => c === 'read_asset_data_url').length;
  await s.browser.$('.file-tree-item.file[data-path="con-imagenes.md"]').click();
  await s.browser.waitUntil(
    async () =>
      ((await s.browser.execute(
        () => new Set([...document.querySelectorAll('img')].filter((i) => i.naturalWidth > 0 && /asset/.test(i.src)).map((i) => i.src)).size
      )) as number) >= 2,
    { timeout: 20_000, timeoutMsg: 'la nota no mostró sus 2 imágenes por el protocolo de assets' }
  );
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/nota-con-imagenes.png`);
  const after = (await ipcCalls()).filter((c) => c === 'read_asset_data_url').length;
  expect(after - before).toBe(0);
});

it('el visor de imágenes abre una imagen con ruta difícil por el protocolo de assets', async () => {
  await s.browser.$('.file-tree-item.folder[data-path="Clientes"]').click();
  await s.browser.$('.file-tree-item.folder[data-path="Clientes/usuario@correo.com"]').waitForExist({ timeout: 10_000 });
  await s.browser.$('.file-tree-item.folder[data-path="Clientes/usuario@correo.com"]').click();
  const file = s.browser.$('.file-tree-item.file[data-path="Clientes/usuario@correo.com/árbol ñandú.png"]');
  await file.waitForExist({ timeout: 10_000 });
  await file.click();
  await s.browser.waitUntil(
    async () =>
      (await s.browser.execute(() =>
        [...document.querySelectorAll('img')].some((i) => i.naturalWidth === 900 && /asset/.test(i.src))
      )) as boolean,
    { timeout: 20_000, timeoutMsg: 'el visor no mostró la imagen por el protocolo de assets' }
  );
});
