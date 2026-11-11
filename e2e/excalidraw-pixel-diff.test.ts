// Diff de píxeles de un dibujo de Excalidraw: las dos versiones (disco y git) se exportan a imagen
// con el mismo marco y se resaltan los píxeles distintos.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;

const rect = (id: string, x: number, color: string) => ({
  id,
  type: 'rectangle',
  x,
  y: 0,
  width: 100,
  height: 80,
  angle: 0,
  strokeColor: color,
  backgroundColor: color,
  fillStyle: 'solid',
  strokeWidth: 2,
  strokeStyle: 'solid',
  roughness: 0,
  opacity: 100,
  seed: 1,
  version: 1,
  versionNonce: 1,
  isDeleted: false,
  groupIds: [],
  boundElements: null,
  updated: 1,
  link: null,
  locked: false,
});
const drawing = (elements: unknown[]) =>
  JSON.stringify({ type: 'excalidraw', version: 2, source: 'e2e', elements, appState: {}, files: {} });

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-exdiff-'));
  const vault = join(base, 'vault');
  mkdirSync(vault, { recursive: true });
  writeFileSync(join(vault, 'dibujo.excalidraw'), drawing([rect('a', 0, '#1971c2')]));
  const git = (...args: string[]) =>
    execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', ...args], { cwd: vault });
  git('init', '-q');
  git('add', '.');
  git('commit', '-qm', 'inicio');
  // Se agrega un elemento a la derecha: el marco crece, pero el rectángulo original no cambia
  writeFileSync(join(vault, 'dibujo.excalidraw'), drawing([rect('a', 0, '#1971c2'), rect('b', 200, '#e03131')]));

  s = await startAppOnVault(vault);
  await s.browser.$('.file-tree-item[data-path="dibujo.excalidraw"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

it('el modo Diferencias resalta solo lo que cambió del dibujo', async () => {
  await s.browser.$('.file-tree-item[data-path="dibujo.excalidraw"]').click({ button: 'right' });
  const item = s.browser.$('.context-menu-item*=Diff visual');
  await item.waitForDisplayed({ timeout: 5_000 });
  await item.click();
  const diffBtn = s.browser.$('.toggle-btn*=Diferencias');
  await diffBtn.waitForDisplayed({ timeout: 10_000 });
  await diffBtn.click();
  await s.browser.$('.diff-img').waitForExist({ timeout: 20_000 });

  const stats = (await s.browser.execute(() => document.querySelector('.stat-pill')?.textContent ?? '')) as string;
  const pct = Number(/\(([\d.,]+)%\)/.exec(stats)?.[1]?.replace(',', '.'));
  // Cambió el rectángulo nuevo (~100×80 de un marco de ~340×120): ni todo ni nada
  expect(pct).toBeGreaterThan(5);
  expect(pct).toBeLessThan(40);
}, 60_000);
