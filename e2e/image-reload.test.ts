// Una imagen abierta se refresca cuando cambia en disco (también en sus copias de pestaña).
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;
let vault: string;
const svg = (w: number) => `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="20"><rect width="${w}" height="20" fill="red"/></svg>`;

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-imgreload-'));
  vault = join(base, 'vault');
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  writeFileSync(join(vault, 'dibujo.svg'), svg(40));
  s = await startAppOnVault(vault);
  await s.browser.$('.file-tree-item[data-path="dibujo.svg"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

const widths = () =>
  s.browser.execute(() => [...document.querySelectorAll('.tab-pane img')].map((i) => (i as HTMLImageElement).naturalWidth)) as Promise<number[]>;

it('la pestaña de una imagen se refresca al cambiar el archivo', async () => {
  await s.browser.$('.file-tree-item[data-path="dibujo.svg"]').click();
  await s.browser.waitUntil(async () => (await widths()).includes(40), { timeout: 10_000, timeoutMsg: 'no cargó la imagen' });
  writeFileSync(join(vault, 'dibujo.svg'), svg(90));
  await s.browser.waitUntil(async () => { const w = await widths(); return w.length > 0 && w.every((x) => x === 90); }, {
    timeout: 15_000,
    timeoutMsg: 'la pestaña siguió mostrando la imagen vieja',
  });
}, 60_000);
