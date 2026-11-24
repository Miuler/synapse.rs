// Mejoras del visor de diff de CodeMirror: navegación entre bloques, búsqueda (Ctrl+F),
// controles de revertir y edición en el árbol de trabajo con guardado a disco.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;
let vault: string;

const ORIGINAL = [
  '# Documento',
  '',
  'Línea uno original',
  'Línea 2',
  'Línea 3',
  'Línea 4',
  'Línea 5',
  'Línea 6',
  'Línea 7',
  'Línea 8',
  'Línea 9',
  'Línea diez original',
  '',
].join('\n');

const MODIFIED = [
  '# Documento',
  '',
  'Línea uno modificada',
  'Línea 2',
  'Línea 3',
  'Línea 4',
  'Línea 5',
  'Línea 6',
  'Línea 7',
  'Línea 8',
  'Línea 9',
  'Línea diez modificada',
  '',
].join('\n');

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-cmdiff-'));
  vault = join(base, 'vault');
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  writeFileSync(join(vault, 'nota.md'), ORIGINAL);
  const git = (...args: string[]) =>
    execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', ...args], { cwd: vault });
  git('init', '-q');
  git('add', '.');
  git('commit', '-qm', 'inicio');
  writeFileSync(join(vault, 'nota.md'), MODIFIED);
  s = await startAppOnVault(vault);
  await s.browser.$('.file-tree-item[data-path="nota.md"]').waitForExist({ timeout: 30_000 });
}, 120_000);

afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

it('navega entre bloques, abre búsqueda y revierte un bloque guardando en disco', async () => {
  // Abrir diff de trabajo desde el menú contextual
  await s.browser.execute(() => {
    const el = document.querySelector('.file-tree-item[data-path="nota.md"]')!;
    const { x, y } = el.getBoundingClientRect();
    el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: x + 5, clientY: y + 5 }));
  });
  await s.browser.$('.context-menu-item').waitForExist({ timeout: 5_000 });
  await s.browser.execute(() => {
    const item = [...document.querySelectorAll('.context-menu-item')].find((i) => /ver cambios|diff/i.test(i.textContent ?? ''));
    (item as HTMLElement).click();
  });

  await s.browser.$('.cm-diff-host').waitForExist({ timeout: 10_000 });
  await s.browser.$('.chunk-counter').waitForExist({ timeout: 5_000 });

  // 1. Navegación entre los 2 bloques de cambio
  const initialCounter = await s.browser.execute(() => document.querySelector('.chunk-counter')?.textContent?.trim());
  expect(initialCounter).toBe('2');

  await s.browser.execute(() => (document.querySelector('button[aria-label="Siguiente cambio"]') as HTMLElement).click());
  await s.browser.waitUntil(
    async () => (await s.browser.execute(() => document.querySelector('.chunk-counter')?.textContent?.trim())) === '1/2',
    { timeout: 3_000 },
  );

  await s.browser.execute(() => (document.querySelector('button[aria-label="Siguiente cambio"]') as HTMLElement).click());
  await s.browser.waitUntil(
    async () => (await s.browser.execute(() => document.querySelector('.chunk-counter')?.textContent?.trim())) === '2/2',
    { timeout: 3_000 },
  );

  await s.browser.execute(() => (document.querySelector('button[aria-label="Cambio anterior"]') as HTMLElement).click());
  await s.browser.waitUntil(
    async () => (await s.browser.execute(() => document.querySelector('.chunk-counter')?.textContent?.trim())) === '1/2',
    { timeout: 3_000 },
  );

  // 2. Búsqueda integrada (@codemirror/search)
  await s.browser.execute(() => (document.querySelector('button[aria-label="Buscar en el diff"]') as HTMLElement).click());
  await s.browser.$('.cm-diff-host .cm-search').waitForExist({ timeout: 5_000 });

  // 3. Revertir el primer bloque y comprobar que se guarda en disco
  const revertBtn = s.browser.$('.cm-diff-revert-btn');
  await revertBtn.waitForExist({ timeout: 5_000 });
  await revertBtn.click();

  await s.browser.waitUntil(
    async () => {
      const onDisk = readFileSync(join(vault, 'nota.md'), 'utf8');
      return onDisk.includes('Línea uno original') && onDisk.includes('Línea diez modificada');
    },
    { timeout: 5_000, timeoutMsg: 'El bloque revertido no se guardó en disco' },
  );

  // Al quedar solo 1 bloque de cambio, el contador se actualiza a 1
  await s.browser.waitUntil(
    async () => {
      const txt = await s.browser.execute(() => document.querySelector('.chunk-counter')?.textContent?.trim() ?? '');
      return txt === '1' || txt === '1/1';
    },
    { timeout: 3_000 },
  );

  // 4. En vista unificada aparecen los botones en español («Revertir» / «Mantener»)
  await s.browser.execute(() => {
    const btn = [...document.querySelectorAll('.toggle-btn')].find((b) => b.textContent?.trim() === 'Unificada') as HTMLElement;
    btn.click();
  });
  await s.browser.$('.cm-diff-merge-reject').waitForExist({ timeout: 5_000 });
  const rejectLabel = await s.browser.execute(() => document.querySelector('.cm-diff-merge-reject')?.textContent?.trim());
  expect(rejectLabel).toBe('Revertir');
});
