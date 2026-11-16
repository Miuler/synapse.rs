// Visor de diff: las vistas ya calculadas («2 Paneles», «Diferencias», texto) quedan en memoria al
// cambiar de modo. Pestañas: «Duplicar en vista dividida» abre la misma pestaña en el otro panel.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { deflateSync } from 'node:zlib';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;

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
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-diffkeep-'));
  const vault = join(base, 'vault');
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  writeFileSync(join(vault, 'foto.png'), png(200, 40, 40));
  writeFileSync(join(vault, 'nota.md'), '# Nota\n\ncontenido\n');
  const git = (...args: string[]) =>
    execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', ...args], { cwd: vault });
  git('init', '-q');
  git('add', '.');
  git('commit', '-qm', 'inicio');
  writeFileSync(join(vault, 'foto.png'), png(40, 40, 200));
  s = await startAppOnVault(vault);
  await s.browser.$('.file-tree-item[data-path="foto.png"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

const clickMode = (label: string) =>
  s.browser.execute((l: string) => {
    ([...document.querySelectorAll('.diff-toolbar .toggle-btn')].find((b) => b.textContent?.trim() === l) as HTMLElement).click();
  }, label);

it('las vistas del diff ya calculadas se mantienen al cambiar de modo', async () => {
  // Diff de la imagen modificada (contra HEAD), desde el menú contextual del explorador
  await s.browser.execute(() => {
    const el = document.querySelector('.file-tree-item[data-path="foto.png"]')!;
    const { x, y } = el.getBoundingClientRect();
    el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: x + 5, clientY: y + 5 }));
  });
  await s.browser.$('.context-menu-item').waitForExist({ timeout: 5_000 });
  await s.browser.execute(() => {
    const item = [...document.querySelectorAll('.context-menu-item')].find((i) => /ver cambios|diff/i.test(i.textContent ?? ''));
    (item as HTMLElement).click();
  });
  await s.browser.$('.diff-toolbar').waitForExist({ timeout: 10_000 });

  await clickMode('Diferencias');
  await s.browser.$('.image-highlight-root').waitForExist({ timeout: 10_000 });
  await s.browser.execute(() => document.querySelector('.image-highlight-root')!.setAttribute('data-marca', 'calculado'));
  await clickMode('2 Paneles');
  await s.browser.$('.visual-diff-root').waitForExist({ timeout: 10_000 });
  await s.browser.execute(() => document.querySelector('.visual-diff-root')!.setAttribute('data-marca', 'calculado'));

  // Al volver a «Diferencias» y a «2 Paneles», son los mismos elementos (no se rehicieron)
  await clickMode('Diferencias');
  expect(await s.browser.execute(() => document.querySelector('.image-highlight-root')?.getAttribute('data-marca'))).toBe('calculado');
  await clickMode('2 Paneles');
  expect(await s.browser.execute(() => document.querySelector('.visual-diff-root')?.getAttribute('data-marca'))).toBe('calculado');
  // Solo se ve la vista activa
  const visibles = await s.browser.execute(() =>
    [...document.querySelectorAll('.diff-mode-pane')].filter((p) => getComputedStyle(p).display !== 'none').length,
  );
  expect(visibles).toBe(1);
});

it('duplicar una pestaña la abre también en el otro panel', async () => {
  await s.browser.$('.file-tree-item[data-path="nota.md"]').click();
  await s.browser.$('.tab-title-container[data-path="nota.md"]').waitForExist({ timeout: 10_000 });
  await s.browser.execute(() => {
    const el = document.querySelector('.tab-title-container[data-path="nota.md"]')!;
    const { x, y } = el.getBoundingClientRect();
    el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: x + 5, clientY: y + 5 }));
  });
  await s.browser.$('.tab-duplicate').waitForExist({ timeout: 5_000 });
  await s.browser.execute(() => (document.querySelector('.tab-duplicate') as HTMLElement).click());
  // La misma pestaña en los dos paneles
  await s.browser.waitUntil(
    async () => (await s.browser.$$('.tab-title-container[data-path="nota.md"]').length) === 2,
    { timeout: 10_000, timeoutMsg: 'la pestaña no se duplicó en el otro panel' },
  );
  expect(await s.browser.$$('.synapse-pane').length).toBe(2);
});
