// «Comparar archivos» con dos imágenes: lado a lado y diferencias píxel por píxel.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { deflateSync } from 'node:zlib';
import { startApp, type Session } from './harness';

let s: Session;

/** PNG RGB de `w`×`h` relleno de un color, con un cuadrado de otro color si se indica. */
function png(w: number, h: number, rgb: [number, number, number], square?: [number, number, number]): Buffer {
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
  ihdr[8] = 8;
  ihdr[9] = 2;
  const rows: number[] = [];
  for (let y = 0; y < h; y++) {
    rows.push(0);
    for (let x = 0; x < w; x++) {
      const inSquare = square && x >= 4 && x < 12 && y >= 4 && y < 12;
      rows.push(...(inSquare ? square! : rgb));
    }
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', deflateSync(Buffer.from(rows))),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

beforeAll(async () => {
  s = await startApp({ 'nota.md': 'x\n' });
  writeFileSync(join(s.vault, 'antes.png'), png(16, 16, [255, 255, 255]));
  writeFileSync(join(s.vault, 'despues.png'), png(16, 16, [255, 255, 255], [0, 0, 0]));
  await s.browser.$('.file-tree-item[data-path="nota.md"]').waitForExist({ timeout: 30_000 });
  await s.browser.$('.file-tree-item[data-path="despues.png"]').waitForExist({ timeout: 15_000 });
}, 120_000);
afterAll(() => s?.close());

it('dos imágenes se comparan lado a lado y píxel por píxel', async () => {
  for (const [i, p] of ['antes.png', 'despues.png'].entries()) {
    await s.browser.execute(
      (path: string, ctrl: boolean) => {
        document
          .querySelector(`.file-tree-item[data-path="${path}"]`)!
          .dispatchEvent(new MouseEvent('click', { bubbles: true, ctrlKey: ctrl }));
      },
      p,
      i > 0,
    );
    await s.browser.pause(600);
  }
  await s.browser.execute(() => {
    const el = document.querySelector('.file-tree-item[data-path="despues.png"]')!;
    const { x, y } = el.getBoundingClientRect();
    el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: x + 5, clientY: y + 5 }));
  });
  await s.browser.$('.context-menu-item').waitForExist({ timeout: 5_000 });
  await s.browser.execute(() => {
    ([...document.querySelectorAll('.context-menu-item')].find((i) => i.textContent?.includes('Comparar archivos')) as HTMLElement).click();
  });

  // Lado a lado: un panel por archivo
  const toggle = s.browser.$('.view-mode-toggle');
  await toggle.waitForExist({ timeout: 10_000 });
  await s.browser.$('button*=2 Paneles').click();
  await s.browser.waitUntil(
    async () => {
      const tags = (await s.browser.execute(() =>
        [...document.querySelectorAll('.pane-tag')].map((t) => t.textContent?.trim()),
      )) as string[];
      return tags.some((t) => t?.includes('despues.png')) && tags.some((t) => t?.includes('antes.png'));
    },
    { timeout: 10_000, timeoutMsg: 'no se ven los dos archivos lado a lado' },
  );

  // Píxel por píxel: el cuadrado de 8×8 cambió
  await s.browser.$('button*=Diferencias').click();
  await s.browser.waitUntil(
    async () => ((await s.browser.execute(() => document.body.textContent ?? '')) as string).includes('64 píxeles distintos'),
    { timeout: 10_000, timeoutMsg: 'no se calculó el diff de píxeles' },
  );
});
