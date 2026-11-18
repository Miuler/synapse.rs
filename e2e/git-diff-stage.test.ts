// «Diff» de una imagen compara lo local con el stage (como `git diff`), no con HEAD; y si todos los
// cambios ya están en stage, avisa y ofrece abrir «Diff --staged».
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { deflateSync } from 'node:zlib';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;
let vault: string;
const git = (...args: string[]) =>
  execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', ...args], { cwd: vault });

/** PNG de 40×30 de un solo color (para que la versión vieja y la nueva difieran). */
function png(r: number, g: number, b: number): Buffer {
  const w = 40;
  const h = 30;
  const crcTable = Array.from({ length: 256 }, (_, n) => {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    return c >>> 0;
  });
  const crc = (buf: Buffer) => {
    let c = 0xffffffff;
    for (const byte of buf) c = crcTable[(c ^ byte) & 0xff] ^ (c >>> 8);
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
  ihdr.set([8, 2, 0, 0, 0], 8);
  const row = Buffer.concat([Buffer.from([0]), Buffer.from(Array.from({ length: w }, () => [r, g, b]).flat())]);
  const raw = Buffer.concat(Array.from({ length: h }, () => row));
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ihdr), chunk('IDAT', deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]);
}

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-diffstage-'));
  vault = join(base, 'vault');
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  writeFileSync(join(vault, 'foto.png'), png(200, 40, 40)); // HEAD: rojo
  git('init', '-q');
  git('add', '.');
  git('commit', '-qm', 'inicio');
  writeFileSync(join(vault, 'foto.png'), png(40, 40, 200)); // stage: azul
  git('add', 'foto.png');
  writeFileSync(join(vault, 'foto.png'), png(40, 200, 40)); // local: verde
  s = await startAppOnVault(vault);
  await s.browser.$('.file-tree-item[data-path="foto.png"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

async function openDiff(label: RegExp) {
  await s.browser.execute(() => {
    const el = document.querySelector('.file-tree-item[data-path="foto.png"]')!;
    const { x, y } = el.getBoundingClientRect();
    el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: x + 5, clientY: y + 5 }));
  });
  await s.browser.$('.context-menu-item').waitForExist({ timeout: 5_000 });
  const picked = await s.browser.execute((src: string) => {
    const re = new RegExp(src);
    const item = [...document.querySelectorAll('.context-menu-item')].find((i) => re.test(i.textContent?.trim() ?? ''));
    (item as HTMLElement | undefined)?.click();
    return item?.textContent?.trim() ?? null;
  }, label.source);
  expect(picked).not.toBeNull();
}

/** Color del centro de la imagen «antes» del panel 2 Paneles visible. */
const beforePixel = () =>
  s.browser.execute(async () => {
    const pane = [...document.querySelectorAll('.diff-mode-pane')].find((p) => (p as HTMLElement).offsetParent !== null)!;
    const imgs = [...pane.querySelectorAll('img')] as HTMLImageElement[];
    const img = imgs[imgs.length - 1];
    await img.decode().catch(() => {});
    const c = document.createElement('canvas');
    c.width = img.naturalWidth;
    c.height = img.naturalHeight;
    const ctx = c.getContext('2d')!;
    ctx.drawImage(img, 0, 0);
    return [...ctx.getImageData(5, 5, 1, 1).data.slice(0, 3)];
  }) as Promise<number[]>;

it('«Diff» de una imagen compara lo local con el stage', async () => {
  await openDiff(/^Diff( visual| \(| Ctrl|$)|^Ver cambios/);
  await s.browser.$('.diff-toolbar').waitForExist({ timeout: 10_000 });
  await s.browser.waitUntil(
    async () => ((await s.browser.execute(() => document.querySelector('.diff-body')?.textContent ?? '')) as string).includes('Stage / Index'),
    { timeout: 10_000, timeoutMsg: 'el lado «antes» no es el stage' },
  );
  // El «antes» es el azul del stage, no el rojo de HEAD
  await s.browser.waitUntil(async () => {
    const [r, , b] = await beforePixel();
    return b > 150 && r < 100;
  }, { timeout: 10_000, timeoutMsg: 'la imagen «antes» no es la del stage' });
  expect(await s.browser.$('.diff-only-staged').isExisting()).toBe(false);
});

it('con todo en stage, avisa y abre «Diff --staged»', async () => {
  git('add', 'foto.png');
  // La pestaña del diff sigue abierta: se recarga
  await s.browser.execute(() => (document.querySelector('button[title="Recargar diferencias (F5)"]') as HTMLElement).click());
  await s.browser.$('.diff-only-staged').waitForExist({ timeout: 10_000 });
  await s.browser.execute(() => (document.querySelector('.diff-only-staged-btn') as HTMLElement).click());
  await s.browser.$('.tab-title-container[data-path="diff-staged:foto.png"]').waitForExist({ timeout: 10_000 });
});
