// Archivos de código (xml, json, …): el botón Lectura/Edición cambia si el editor deja escribir.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-codeedit-'));
  const vault = join(base, 'vault');
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  writeFileSync(join(vault, 'datos.xml'), '<?xml version="1.0"?>\n<a/>\n');
  s = await startAppOnVault(vault);
  await s.browser.$('.file-tree-item[data-path="datos.xml"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

const editable = () =>
  s.browser.execute(() => {
    const pane = [...document.querySelectorAll('.tab-pane')].find((p) => (p as HTMLElement).offsetParent !== null);
    return pane?.querySelector('.cm-content')?.getAttribute('contenteditable');
  });

it('pasar a Edición deja escribir en un archivo de código, y volver a Lectura no', async () => {
  await s.browser.$('.file-tree-item[data-path="datos.xml"]').click();
  await s.browser.$('.tab-title-container.active[data-path="datos.xml"]').waitForExist({ timeout: 10_000 });
  const toggle = s.browser.$('.editor-header .view-toggle-btn');
  await s.browser.waitUntil(async () => (await editable()) === 'false', { timeout: 10_000, timeoutMsg: 'en lectura se puede escribir' });
  await toggle.click();
  await s.browser.waitUntil(async () => (await editable()) === 'true', { timeout: 5_000, timeoutMsg: 'en edición no se puede escribir' });
  await toggle.click();
  await s.browser.waitUntil(async () => (await editable()) === 'false', { timeout: 5_000, timeoutMsg: 'volvió a lectura pero sigue editable' });
  expect(await toggle.getText()).toContain('Lectura');
}, 60_000);
