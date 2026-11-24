// Paleta de comandos: se encuentran también en inglés.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

let s: Session;
beforeAll(async () => {
  s = await startApp({ 'nota.md': '# Nota\n' });
  await s.browser.$('.file-tree-item[data-path="nota.md"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(() => s?.close());

it('«upgrade» deja primero «Buscar actualizaciones»', async () => {
  await s.browser.keys(['Control', 'p']);
  const input = await s.browser.$('.palette-container input');
  await input.waitForExist({ timeout: 5_000 });
  await input.setValue('upgrade');
  await s.browser.waitUntil(
    async () => (await s.browser.$('.command-item[data-selected] .command-name').getText()).includes('Buscar actualizaciones'),
    { timeout: 5_000, timeoutMsg: 'el primer resultado no es «Buscar actualizaciones»' },
  );
  expect(await s.browser.$('.command-item[data-selected] .command-name').getText()).toContain('Buscar actualizaciones');
  // Flechas y vaciar la búsqueda siguen funcionando
  const name = () => s.browser.$('.command-item[data-selected] .command-name').getText();
  const first = await name();
  await s.browser.keys('ArrowDown');
  await s.browser.waitUntil(async () => (await name()) !== first, { timeout: 5_000, timeoutMsg: 'la flecha no mueve la selección' });
  await input.setValue('cerrar pestaña');
  await s.browser.waitUntil(async () => (await name()) === 'Cerrar pestaña actual', { timeout: 5_000 });
  await s.browser.keys('Enter');
  await s.browser.$('.palette-container').waitForExist({ reverse: true, timeout: 5_000 });
});

it('en inglés, los nombres salen en inglés y se encuentran también en español', async () => {
  await s.browser.keys(['Control', 'p']);
  const spanish = await s.browser.$('.palette-container input');
  await spanish.waitForExist({ timeout: 5_000 });
  await spanish.setValue('switch language');
  await s.browser.waitUntil(
    async () => (await s.browser.$('.command-item[data-selected] .command-name').getText()).startsWith('Cambiar idioma'),
    { timeout: 5_000 },
  );
  await s.browser.keys('Enter');
  await s.browser.$('.palette-container').waitForExist({ reverse: true, timeout: 5_000 });
  await s.browser.keys(['Control', 'p']);
  const input = await s.browser.$('.palette-container input');
  await input.waitForExist({ timeout: 5_000 });
  expect(await input.getAttribute('placeholder')).toBe('Type a command or search...');
  await input.setValue('actualizaciones');
  await s.browser.waitUntil(
    async () => (await s.browser.$('.command-item[data-selected] .command-name').getText()) === 'Synapse: Check for updates',
    { timeout: 5_000, timeoutMsg: 'no se encontró en español con la interfaz en inglés' },
  );
});
