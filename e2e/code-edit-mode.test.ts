// Archivos de código (xml, json, …): Lectura no deja escribir pero sí buscar (Ctrl+F); Edición deja escribir.
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

const text = () => s.browser.execute(() => document.querySelector('.tab-pane:not(.hidden) .cm-content')?.textContent ?? '');
/** Escribe una «Z» al inicio del editor y dice si el texto cambió. */
async function typingChanges(): Promise<boolean> {
  const before = await text();
  await s.browser.$('.tab-pane:not(.hidden) .cm-content').click();
  await s.browser.keys(['Control', 'Home']);
  await s.browser.keys('Z');
  // Justificada: en solo lectura se comprueba que el texto NO cambia, y no hay evento que esperar
  await s.browser.pause(200);
  const after = await text();
  if (after !== before) await s.browser.keys(['Backspace']);
  return after !== before;
}

it('en Lectura no se escribe pero Ctrl+F abre el buscador', async () => {
  await s.browser.$('.file-tree-item[data-path="datos.xml"]').click();
  await s.browser.$('.tab-title-container.active[data-path="datos.xml"]').waitForExist({ timeout: 10_000 });
  await s.browser.$('.tab-pane:not(.hidden) .cm-content').waitForExist({ timeout: 10_000 });
  expect(await s.browser.$('.editor-header .view-toggle-btn').getText()).toContain('Lectura');
  expect(await typingChanges()).toBe(false);
  await s.browser.keys(['Control', 'f']);
  await s.browser.$('.tab-pane:not(.hidden) .cm-search').waitForExist({ timeout: 5_000, timeoutMsg: 'Ctrl+F no abrió el buscador en lectura' });
  await s.browser.keys(['Escape']);
}, 60_000);

it('al pasar a Edición se puede escribir, y al volver a Lectura no', async () => {
  const toggle = s.browser.$('.editor-header .view-toggle-btn');
  await toggle.click();
  await s.browser.waitUntil(async () => (await toggle.getText()).includes('Edición'), { timeout: 5_000 });
  expect(await typingChanges()).toBe(true);
  await toggle.click();
  await s.browser.waitUntil(async () => (await toggle.getText()).includes('Lectura'), { timeout: 5_000 });
  expect(await typingChanges()).toBe(false);
}, 60_000);
