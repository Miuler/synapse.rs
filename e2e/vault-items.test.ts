// Flujos que dependen de la lista de la bóveda (store de la entidad vault-item): explorador,
// Quick Open, pestañas, título editable y creación de notas.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { existsSync } from 'fs';
import { join } from 'path';
import { startApp, type Session } from './harness';

let s: Session;
const shots = process.env.E2E_SHOTS;
const shot = async (name: string) => {
  if (shots) await s.browser.saveScreenshot(join(shots, `${name}.png`));
};
const explorerFile = (path: string) => s.browser.$(`.file-tree-item.file[data-path="${path}"]`);

beforeAll(async () => {
  s = await startApp({
    'bienvenida.md': '# Bienvenida\nHola desde la bóveda de prueba.\n',
    'docs/arquitectura.md': '---\ntags: [rust]\n---\n# Arquitectura\nCapas.\n',
    'docs/notas/idea.md': '# Idea\nUna idea suelta.\n',
  });
  await explorerFile('bienvenida.md').waitForExist({ timeout: 30_000 });
}, 120_000);

afterAll(async () => {
  await s?.close();
});

it('el explorador muestra las notas de la bóveda', async () => {
  expect(await explorerFile('bienvenida.md').isExisting()).toBe(true);
  expect(await s.browser.$('.file-tree-item.folder[data-path="docs"]').isExisting()).toBe(true);
});

it('abrir una nota crea su pestaña y muestra su título', async () => {
  await explorerFile('bienvenida.md').click();
  const tab = s.browser.$('.tab-title-container[data-path="bienvenida.md"]');
  await tab.waitForExist({ timeout: 10_000 });
  const title = s.browser.$('.editor-title-input');
  await title.waitForExist({ timeout: 10_000 });
  expect(await title.getValue()).toBe('Bienvenida');
  await shot('01-nota-abierta');
});

it('el Quick Open encuentra notas en subcarpetas', async () => {
  await s.browser.keys(['Control', 'o']);
  const input = s.browser.$('.quick-open-command-root input');
  await input.waitForExist({ timeout: 10_000 });
  await input.setValue('idea');
  await s.browser.waitUntil(async () => (await s.browser.$('.quick-open-results-container').getText()).includes('idea'), {
    timeout: 10_000,
    timeoutMsg: 'el Quick Open no mostró docs/notas/idea.md',
  });
  await shot('02-quick-open');
  await s.browser.keys(['Enter']);
  await s.browser.$('.tab-title-container[data-path="docs/notas/idea.md"]').waitForExist({ timeout: 10_000 });
});

it('el Quick Open agrupa recientes solo sin búsqueda', async () => {
  const dividers = async () =>
    (await s.browser.execute(() =>
      [...document.querySelectorAll('.quick-open-results-container .section-divider-label')].map((d) => d.textContent?.trim() ?? '')
    )) as string[];
  await s.browser.keys(['Control', 'o']);
  const input = s.browser.$('.quick-open-command-root input');
  await input.waitForExist({ timeout: 10_000 });
  await s.browser.waitUntil(async () => (await dividers()).length > 0, { timeout: 10_000, timeoutMsg: 'sin separadores en la lista sin búsqueda' });
  const grouped = (await dividers()).map((t) => t.toLowerCase());
  expect(grouped[0]).toBe('recientes');
  expect(grouped.filter((t) => t === 'archivos').length).toBeLessThanOrEqual(1);
  await input.setValue('a');
  // Al buscar: sin "Recientes" y como mucho un "Archivos" (debajo de las carpetas, si las hay)
  await s.browser.waitUntil(async () => !(await dividers()).some((t) => t.toLowerCase() === 'recientes'), {
    timeout: 10_000,
    timeoutMsg: 'quedó el separador Recientes al buscar',
  });
  const searching = (await dividers()).map((t) => t.toLowerCase());
  expect(searching.filter((t) => t === 'archivos').length).toBeLessThanOrEqual(1);
  await shot('02b-quick-open-busqueda');
  await s.browser.keys(['Escape']);
});

it('cambiar el título renombra el archivo y el explorador se actualiza', async () => {
  await explorerFile('bienvenida.md').click();
  const title = s.browser.$('.editor-title-input');
  await s.browser.waitUntil(async () => (await title.getValue()) === 'Bienvenida', { timeout: 10_000 });
  // Pasar a edición si el título es de solo lectura
  if (await title.getAttribute('readonly') !== null) {
    await s.browser.keys(['Control', 'e']);
    await s.browser.waitUntil(async () => (await title.getAttribute('readonly')) === null, { timeout: 5_000 });
  }
  await title.click();
  await s.browser.keys(['Control', 'a']);
  await s.browser.keys('Portada'.split(''));
  await s.browser.keys(['Enter']);
  await explorerFile('Portada.md').waitForExist({ timeout: 15_000, timeoutMsg: 'el explorador no mostró Portada.md' });
  expect(existsSync(join(s.vault, 'Portada.md'))).toBe(true);
  expect(existsSync(join(s.vault, 'bienvenida.md'))).toBe(false);
  await shot('03-titulo-cambiado');
});

it('crear una nota nueva la agrega a la lista', async () => {
  const before = (await s.browser.$$('.tab-title-container')).length;
  await s.browser.keys(['Control', 'n']);
  await s.browser.waitUntil(async () => (await s.browser.$$('.tab-title-container')).length > before, {
    timeout: 10_000,
    timeoutMsg: 'no se abrió la pestaña de la nota nueva',
  });
  await shot('04-nota-nueva');
});

it('la consola no muestra avisos de reactividad de Svelte', async () => {
  const log = await s.consoleLog();
  expect(log).not.toContain('binding_property_non_reactive');
  expect(log).not.toContain('state_unsafe_mutation');
});
