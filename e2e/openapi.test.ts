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
  // Especificación repartida en archivos: los `$ref` relativos se siguen dentro de la bóveda
  mkdirSync(join(vault, 'api', 'paths'), { recursive: true });
  writeFileSync(join(vault, 'api', 'main.yaml'),
    "openapi: 3.0.1\ninfo:\n  title: API Repartida\n  version: 1.0.0\npaths:\n  /transferencias:\n    $ref: ./paths/transferencias.yaml\n  /falta:\n    $ref: ./paths/no-existe.yaml\n");
  writeFileSync(join(vault, 'api', 'paths', 'transferencias.yaml'),
    "get:\n  summary: Buscar transferencias\n  responses:\n    '200':\n      description: OK\n");
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

it('Scalar usa los colores del tema de la app en claro y en oscuro', async () => {
  const colors = () =>
    s.browser.execute(() => {
      const app = document.querySelector('.tab-pane:not(.hidden) .openapi-pane .scalar-app') as HTMLElement;
      const probe = document.createElement('div');
      probe.style.background = 'var(--bg-primary)';
      document.body.appendChild(probe);
      const expected = getComputedStyle(probe).backgroundColor;
      probe.remove();
      return { scalar: getComputedStyle(app).getPropertyValue('--scalar-background-1').trim(), resolved: getComputedStyle(app).backgroundColor, expected,
        dark: document.body.classList.contains('dark-mode') };
    });
  const original = await s.browser.execute(() => document.documentElement.getAttribute('data-theme'));
  for (const theme of ['light', 'dark']) {
    await s.browser.execute((t) => document.documentElement.setAttribute('data-theme', t), theme);
    await s.browser.waitUntil(async () => (await colors()).dark === (theme === 'dark'), { timeout: 5_000, timeoutMsg: `Scalar no pasó a ${theme}` });
    const c = await colors();
    expect(c.scalar).not.toBe('');
    expect(c.resolved).toBe(c.expected);
  }
  await s.browser.execute((t) => document.documentElement.setAttribute('data-theme', t ?? 'light'), original);
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
  // Resaltado YAML: la clave `openapi` va en su propio span con estilo
  const keySpans = await s.browser.execute(() =>
    [...document.querySelectorAll('.tab-pane:not(.hidden) .cm-line span')].filter((e) => e.textContent === 'openapi').length,
  );
  expect(keySpans).toBeGreaterThan(0);
  expect(await s.browser.$('.tab-pane:not(.hidden) .openapi-pane').isExisting()).toBe(false);
}, 60_000);

it('sigue los $ref a otros archivos y avisa de los que faltan; se actualiza al cambiar uno', async () => {
  await s.browser.execute(() => document.querySelector('.file-tree-item[data-path="api"]')?.dispatchEvent(new MouseEvent('click', { bubbles: true })));
  await s.browser.$('.file-tree-item[data-path="api/main.yaml"]').waitForExist({ timeout: 10_000 });
  await s.browser.$('.file-tree-item[data-path="api/main.yaml"]').click();
  await s.browser.waitUntil(async () => (await paneText()).includes('Buscar transferencias'), { timeout: 20_000, timeoutMsg: 'no se resolvió el $ref externo' });
  expect(await paneText()).toContain('sin resolver');
  writeFileSync(join(vault, 'api', 'paths', 'transferencias.yaml'),
    "get:\n  summary: Listar transferencias nuevas\n  responses:\n    '200':\n      description: OK\n");
  await s.browser.waitUntil(async () => (await paneText()).includes('Listar transferencias nuevas'), { timeout: 20_000, timeoutMsg: 'no se actualizó al cambiar el archivo referenciado' });
}, 60_000);

it('Ctrl+P → «Zoom» abre el selector; ↑/↓ + Enter cambian el texto de Scalar, Esc lo deja como estaba', async () => {
  const zoom = () => s.browser.execute(() => getComputedStyle(document.documentElement).getPropertyValue('--content-zoom').trim());
  const openZoom = async () => {
    await s.browser.keys(['Control', 'p']);
    const input = await s.browser.$('.synapse-command-palette input');
    await input.waitForExist({ timeout: 15_000 });
    await input.setValue('Zoom: tamaño');
    await s.browser.waitUntil(
      async () => (await s.browser.execute(() => !!document.querySelector('.synapse-command-palette .command-item'))) as boolean,
      { timeout: 15_000 },
    );
    await s.browser.keys(['Enter']);
    await s.browser.$('.zoom-dialog').waitForExist({ timeout: 5_000 });
  };
  const fontBefore = (await s.browser.execute(() => {
    const app = document.querySelector('.tab-pane:not(.hidden) .openapi-pane .scalar-app') as HTMLElement;
    return getComputedStyle(app.querySelector('h1, h2, .section-header') ?? app).fontSize;
  })) as string;
  await openZoom();
  expect(await s.browser.execute(() => document.querySelector('.zoom-list [aria-selected="true"]')?.textContent?.trim())).toBe('100%');
  await s.browser.keys(['ArrowDown']);
  await s.browser.keys(['ArrowDown']);
  await s.browser.keys(['Enter']);
  await s.browser.$('.zoom-dialog').waitForExist({ timeout: 5_000, reverse: true });
  expect(await zoom()).toBe('1.25');
  const fontAfter = (await s.browser.execute(() => {
    const app = document.querySelector('.tab-pane:not(.hidden) .openapi-pane .scalar-app') as HTMLElement;
    return getComputedStyle(app.querySelector('h1, h2, .section-header') ?? app).fontSize;
  })) as string;
  expect(parseFloat(fontAfter)).toBeGreaterThan(parseFloat(fontBefore));
  // Esc deshace la vista previa
  await openZoom();
  await s.browser.keys(['ArrowUp']);
  expect(await zoom()).toBe('1.1');
  await s.browser.keys(['Escape']);
  await s.browser.$('.zoom-dialog').waitForExist({ timeout: 5_000, reverse: true });
  expect(await zoom()).toBe('1.25');
  await s.browser.execute(() => document.documentElement.style.setProperty('--content-zoom', '1'));
}, 60_000);
