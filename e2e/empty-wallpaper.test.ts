// Pestaña vacía: el fondo por defecto (según el tema) va en el binario y el WebView lo puede
// decodificar (WebKitGTK no siempre trae AVIF, por eso es WebP).
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-wallpaper-'));
  const vault = join(base, 'vault');
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  writeFileSync(join(vault, 'nota.md'), '# Nota\n');
  s = await startAppOnVault(vault);
  await s.browser.$('.empty-workspace').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

it('la pestaña vacía muestra el fondo por defecto', async () => {
  const result = await s.browser.execute(async () => {
    const bg = getComputedStyle(document.querySelector('.empty-workspace')!).backgroundImage;
    const src = /url\("?([^")]+)/.exec(bg)?.[1];
    if (!src) return 'sin fondo';
    return new Promise<string>((resolve) => {
      const img = new Image();
      img.onload = () => resolve(img.naturalWidth > 0 ? 'ok' : 'vacía');
      img.onerror = () => resolve('no se pudo decodificar ' + src);
      img.src = src;
    });
  });
  expect(result).toBe('ok');
});
