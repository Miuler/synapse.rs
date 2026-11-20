// Novedades: el comando de la paleta abre el CHANGELOG empaquetado en una pestaña en memoria
// (no es un archivo del vault), y Ajustes tiene la sección de actualizaciones.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-news-'));
  const vault = join(base, 'vault');
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  writeFileSync(join(vault, 'nota.md'), '# Nota\n');
  s = await startAppOnVault(vault);
  await s.browser.$('.file-tree-item[data-path="nota.md"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

const paneText = () =>
  s.browser.execute(() => (document.querySelector('.tab-pane:not(.hidden)') as HTMLElement | null)?.innerText ?? '') as Promise<string>;

it('«Synapse: Novedades» abre el changelog en una pestaña que no está en el vault', async () => {
  await s.browser.$('body').click();
  await s.browser.keys(['Control', 'p']);
  const input = await s.browser.$('.synapse-command-palette input');
  await input.waitForExist({ timeout: 15_000 });
  await input.setValue('Synapse: Novedades');
  await s.browser.waitUntil(
    async () => (await s.browser.execute(() => !!document.querySelector('.synapse-command-palette .command-item'))) as boolean,
    { timeout: 15_000 },
  );
  await s.browser.keys(['Enter']);
  await s.browser.$('.tab-title-container.active[data-path="synapse://news"]').waitForExist({ timeout: 10_000 });
  await s.browser.waitUntil(async () => (await paneText()).includes('Novedades de Synapse'), { timeout: 10_000 });
  // Empieza por la versión instalada (la del binario, que puede ir por detrás del CHANGELOG)
  const version = (await s.browser.execute(() =>
    (window as any).__TAURI_INTERNALS__.invoke('plugin:app|version'),
  )) as string;
  const text = await paneText();
  expect(text.split('\n').find((l) => /^\[\d/.test(l))).toStartWith(`[${version}]`);
  expect(text).not.toContain('Sin publicar');
  const title = (await s.browser.execute(
    () => document.querySelector('.tab-title-container.active')?.textContent?.trim() ?? '',
  )) as string;
  expect(title).toContain('Novedades');
  expect(await s.browser.$('.file-tree-item[data-path^="synapse://"]').isExisting()).toBe(false);
}, 60_000);

it('al abrir por primera vez una versión nueva, abre sola las novedades desde la anterior', async () => {
  // Instalación nueva (perfil vacío): no se abrió nada solo
  expect(
    await s.browser.execute(() => [...document.querySelectorAll('.tab-title-container')].some((t) => t.getAttribute('data-path')?.startsWith('synapse://news:'))),
  ).toBe(false);
  await s.browser.execute(() => {
    localStorage.setItem('synapse.lastSeenVersion', '0.1.0-alpha');
    location.reload();
  });
  await s.browser.$('.tab-title-container[data-path="synapse://news:0.1.0-alpha"]').waitForExist({ timeout: 30_000, timeoutMsg: 'no se abrieron las novedades' });
  await s.browser.waitUntil(async () => (await paneText()).includes('Novedades de Synapse desde 0.1.0-alpha'), { timeout: 10_000 });
  // Y queda registrada la versión actual: no se repite
  const version = (await s.browser.execute(() => (window as any).__TAURI_INTERNALS__.invoke('plugin:app|version'))) as string;
  expect(await s.browser.execute(() => localStorage.getItem('synapse.lastSeenVersion'))).toBe(version);
}, 90_000);
