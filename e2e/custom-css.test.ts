// Temas y fragmentos CSS de la bóveda: se eligen en Configuración, se aplican sin reiniciar y quedan
// guardados en .synapse/appearance.json.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { startApp, type Session } from './harness';

let s: Session;

beforeAll(async () => {
  s = await startApp({
    'nota.md': '# Hola\n',
    '.synapse/themes/rojo.css': ':root, html[data-theme] { --accent: rgb(255, 0, 0); }\n',
    '.synapse/snippets/titulos.css': '.markdown-body h1 { letter-spacing: 7px; }\n',
  });
  await s.browser.$('.file-tree-item[data-path="nota.md"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(() => s?.close());

const accent = () =>
  s.browser.execute(() => getComputedStyle(document.documentElement).getPropertyValue('--accent').trim()) as Promise<string>;

it('elegir un tema y activar un fragmento cambia el aspecto y se guarda', async () => {
  const before = await accent();
  expect(before).not.toBe('rgb(255, 0, 0)');

  await s.browser.$('[aria-label="Configuración"]').click();
  await s.browser.$('.settings-panel').waitForDisplayed({ timeout: 5_000 });
  await s.browser.$('.custom-theme-select option[value="rojo"]').waitForExist({ timeout: 5_000 });
  await s.browser.$('.custom-theme-select').selectByAttribute('value', 'rojo');
  await s.browser.waitUntil(async () => (await accent()) === 'rgb(255, 0, 0)', {
    timeout: 5_000,
    timeoutMsg: 'el tema no cambió --accent',
  });

  await s.browser.$('.snippet-toggle[data-snippet="titulos"]').click();
  await s.browser.waitUntil(
    async () =>
      (await s.browser.execute(() => document.getElementById('synapse-custom-css')?.textContent ?? '')).includes('letter-spacing: 7px'),
    { timeout: 5_000, timeoutMsg: 'el fragmento no se inyectó' },
  );

  const config = JSON.parse(readFileSync(join(s.vault, '.synapse/appearance.json'), 'utf8'));
  expect(config).toEqual({ theme: 'rojo', snippets: ['titulos'] });

  // Volver a «Ninguno» quita el tema
  await s.browser.$('.custom-theme-select').selectByAttribute('value', '');
  await s.browser.waitUntil(async () => (await accent()) === before, { timeout: 5_000 });
});

it('un fragmento cambia títulos en lectura y edición, la fuente del texto y el ícono de carpeta', async () => {
  const svg = encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"><rect width="1" height="1" fill="red"/></svg>');
  const css = `:root { --h1-color: rgb(0, 128, 0); --h1-size: 40px; --font-text: "Courier New"; --icon-folder: url("data:image/svg+xml,${svg}"); }\n`;
  writeFileSync(join(s.vault, '.synapse/snippets/estilo.css'), css);
  mkdirSync(join(s.vault, 'carpeta'), { recursive: true });
  writeFileSync(join(s.vault, 'carpeta/x.md'), 'x\n');
  await s.browser.$('.appearance-reload').click();
  await s.browser.$('.snippet-toggle[data-snippet="estilo"]').waitForExist({ timeout: 5_000 });
  await s.browser.$('.snippet-toggle[data-snippet="estilo"]').click();
  await s.browser.keys('Escape');

  await s.browser.$('.file-tree-item[data-path="nota.md"]').click();
  // Lectura
  await s.browser.$('.md-view-btn').click();
  await s.browser.$('.view-item.mode-reading').click();
  await s.browser.$('.markdown-body h1').waitForExist({ timeout: 10_000 });
  const reading = (await s.browser.execute(() => {
    const h = getComputedStyle(document.querySelector('.markdown-body h1')!);
    return { color: h.color, size: h.fontSize, font: getComputedStyle(document.querySelector('.markdown-body')!).fontFamily };
  })) as { color: string; size: string; font: string };
  expect(reading.color).toBe('rgb(0, 128, 0)');
  expect(reading.size).toBe('40px');
  expect(reading.font).toContain('Courier New');
  // Edición
  await s.browser.$('.md-view-btn').click();
  await s.browser.$('.view-item.mode-live').click();
  await s.browser.waitUntil(
    async () =>
      (await s.browser.execute(() => {
        // El estilo del título envuelve toda la línea («# Hola»), con la marca «#» en un span interno
        const span = [...document.querySelectorAll('.cm-line span')].find((e) => e.textContent?.trim().endsWith('Hola'));
        return span ? getComputedStyle(span).color : null;
      })) === 'rgb(0, 128, 0)',
    { timeout: 5_000, timeoutMsg: 'el título del editor no tomó --h1-color' },
  );
  // Ícono de carpeta reemplazado
  await s.browser.waitUntil(
    async () => (await s.browser.$('.file-tree-item[data-path="carpeta"] .synapse-folder-custom').isExisting()),
    { timeout: 10_000, timeoutMsg: 'el ícono de carpeta no se reemplazó' },
  );
});

it('el tema integrado «Papel» se aplica sin archivo en la bóveda y las zonas públicas existen', async () => {
  // Quitar el fragmento del caso anterior para ver el ícono del tema
  await s.browser.$('[aria-label="Configuración"]').click();
  await s.browser.$('.settings-panel').waitForDisplayed({ timeout: 5_000 });
  await s.browser.$('.snippet-toggle[data-snippet="estilo"]').click();
  await s.browser.$('.appearance-reload').click();
  await s.browser.$('.custom-theme-select option[value="papel"]').waitForExist({ timeout: 5_000 });
  await s.browser.$('.custom-theme-select').selectByAttribute('value', 'papel');
  await s.browser.waitUntil(
    async () =>
      (await s.browser.execute(() => getComputedStyle(document.documentElement).getPropertyValue('--bg-primary').trim())) ===
      ((await s.browser.execute(() => document.documentElement.dataset.theme)) === 'dark' ? '#1e1d1a' : '#f7f1e3'),
    { timeout: 5_000, timeoutMsg: 'el tema Papel no cambió --bg-primary' },
  );
  await s.browser.keys('Escape');
  const zones = (await s.browser.execute(() =>
    ['.synapse-app', '.synapse-ribbon', '.synapse-sidebar', '.synapse-explorer-item', '.synapse-header', '.synapse-tab', '.synapse-main', '.synapse-note-title', '.synapse-editor', '.synapse-statusbar'].filter(
      (sel) => !document.querySelector(sel),
    ),
  )) as string[];
  expect(zones).toEqual([]);
});
