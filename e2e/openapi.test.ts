// Archivos OpenAPI: en Lectura se ven con Scalar; en Edición, el YAML en el editor de código.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;
let vault: string;

const SPEC = `openapi: 3.0.3
info:
  title: API de Prueba Synapse
  version: 1.0.0
paths:
  /mascotas:
    get:
      summary: Listar mascotas
      responses:
        '200':
          description: OK
`;

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-openapi-'));
  vault = join(base, 'vault');
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  writeFileSync(join(vault, 'openapi.yaml'), SPEC);
  s = await startAppOnVault(vault);
  await s.browser.$('.file-tree-item[data-path="openapi.yaml"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

const paneText = () =>
  s.browser.execute(() => (document.querySelector('.tab-pane:not(.hidden) .openapi-pane') as HTMLElement | null)?.innerText ?? '');

it('en Lectura muestra el visor Scalar', async () => {
  await s.browser.$('.file-tree-item[data-path="openapi.yaml"]').click();
  await s.browser.$('.tab-title-container.active[data-path="openapi.yaml"]').waitForExist({ timeout: 10_000 });
  await s.browser.$('.tab-pane:not(.hidden) .openapi-pane .scalar-app').waitForExist({ timeout: 20_000, timeoutMsg: 'Scalar no se montó' });
  await s.browser.waitUntil(async () => (await paneText()).includes('API de Prueba Synapse'), { timeout: 20_000 });
  expect(await paneText()).toContain('Listar mascotas');
  // Scalar va empaquetado: nada de CDN, fuentes ni telemetría de terceros
  const external = await s.browser.execute(() =>
    performance.getEntriesByType('resource').map((e) => e.name)
      .filter((u) => /^https?:/.test(u) && !/^https?:\/\/(localhost|127\.0\.0\.1|tauri\.localhost|ipc\.localhost|asset\.localhost)\b/.test(u)),
  );
  expect(external).toEqual([]);
}, 60_000);

it('al cambiar el archivo en disco el visor se actualiza', async () => {
  writeFileSync(join(vault, 'openapi.yaml'), SPEC.replace('API de Prueba Synapse', 'API Renombrada'));
  await s.browser.waitUntil(async () => (await paneText()).includes('API Renombrada'), { timeout: 20_000, timeoutMsg: 'el visor no recargó' });
}, 60_000);

it('en Edición muestra el YAML en el editor de código', async () => {
  const toggle = s.browser.$('.editor-header .view-toggle-btn');
  await toggle.click();
  await s.browser.waitUntil(async () => (await toggle.getText()).includes('Edición'), { timeout: 5_000 });
  await s.browser.$('.tab-pane:not(.hidden) .cm-content').waitForExist({ timeout: 10_000 });
  const code = await s.browser.execute(() => document.querySelector('.tab-pane:not(.hidden) .cm-content')?.textContent ?? '');
  expect(code).toContain('openapi: 3.0.3');
  expect(await s.browser.$('.tab-pane:not(.hidden) .openapi-pane').isExisting()).toBe(false);
}, 60_000);
