// Con el teclado: Alt+2 abre y cierra el agente; el atajo del explorador lo muestra con el foco en el árbol, las flechas mueven la
// selección y Enter abre el archivo. Vale para «ubicar» (Ctrl+Alt+1) y para mostrarlo (Alt+1).
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

let s: Session;

beforeAll(async () => {
  s = await startApp({ 'a.md': '# A\n', 'b.md': '# B\n', 'c.md': '# C\n' });
  await s.browser.$('.file-tree-item[data-path="a.md"]').waitForExist({ timeout: 30_000 });
  await s.browser.$('.file-tree-item[data-path="a.md"]').click();
  await s.browser.$('.tab-title-container[data-path="a.md"]').waitForExist({ timeout: 10_000 });
}, 120_000);
afterAll(() => s?.close());

const focusInExplorer = () =>
  s.browser.execute(() => !!document.activeElement?.closest('.synapse-explorer')) as Promise<boolean>;
/** Saca el foco del explorador (como si el usuario estuviera en el editor). */
const focusOutside = () =>
  s.browser.execute(() => {
    (document.activeElement as HTMLElement | null)?.blur();
    const main = document.querySelector('.synapse-main') as HTMLElement;
    main.tabIndex = -1;
    main.focus();
  });

async function openWithEnterBelowCurrent() {
  await s.browser.keys('ArrowDown');
  await s.browser.keys('Enter');
}

it('Ctrl+Alt+1 ubica el archivo y deja el foco en el árbol para usar flechas y Enter', async () => {
  // El foco está en el editor
  await focusOutside();
  expect(await focusInExplorer()).toBe(false);
  await s.browser.keys(['Control', 'Alt', '1']);
  await s.browser.waitUntil(focusInExplorer, { timeout: 5_000, timeoutMsg: 'el árbol no quedó con el foco' });
  await openWithEnterBelowCurrent();
  await s.browser.$('.tab-title-container[data-path="b.md"]').waitForExist({ timeout: 5_000 });

});

it('Alt+1 muestra el explorador con el foco en el árbol', async () => {
  // Ocultarlo y volver a mostrarlo desde el editor
  await focusOutside();
  await s.browser.keys(['Alt', '1']);
  await s.browser.waitUntil(
    async () => !(await s.browser.execute(() => getComputedStyle(document.querySelector('.synapse-explorer')!).display !== 'none')),
    { timeout: 5_000 },
  );
  await s.browser.keys(['Alt', '1']);
  await s.browser.waitUntil(focusInExplorer, { timeout: 5_000, timeoutMsg: 'Alt+1 no dejó el foco en el árbol' });
  await openWithEnterBelowCurrent();
  await s.browser.$('.tab-title-container[data-path="c.md"]').waitForExist({ timeout: 5_000, timeoutMsg: 'Enter no abrió c.md' });
});

it('Shift+Escape cierra el último panel lateral que se abrió', async () => {
  const explorerOpen = () =>
    s.browser.execute(() => getComputedStyle(document.querySelector('.synapse-explorer')!).display !== 'none') as Promise<boolean>;
  const agentOpen = () => s.browser.execute(() => !!document.querySelector('.synapse-agent-panel')) as Promise<boolean>;
  /** Pulsa Shift+Escape y espera a que se cierre el panel que se espera cerrar. */
  const shiftEsc = async (closes: () => Promise<boolean>) => {
    await focusOutside();
    await s.browser.keys(['Shift', 'Escape']);
    await s.browser.waitUntil(async () => !(await closes()), { timeout: 5_000, timeoutMsg: 'Shift+Escape no cerró el panel' });
  };

  // Explorador y después el agente: primero se cierra el agente, luego el explorador
  if (!(await explorerOpen())) await s.browser.keys(['Alt', '1']);
  await s.browser.waitUntil(explorerOpen, { timeout: 5_000 });
  await s.browser.$('.editor-header .agent-toggle').click();
  await s.browser.waitUntil(agentOpen, { timeout: 5_000 });
  await shiftEsc(agentOpen);
  expect(await explorerOpen()).toBe(true);
  await shiftEsc(explorerOpen);

  // Al revés: el agente y después el explorador; se cierra primero el explorador
  await s.browser.$('.editor-header .agent-toggle').click();
  await s.browser.waitUntil(agentOpen, { timeout: 5_000 });
  await focusOutside();
  await s.browser.keys(['Alt', '1']);
  await s.browser.waitUntil(explorerOpen, { timeout: 5_000 });
  await shiftEsc(explorerOpen);
  expect(await agentOpen()).toBe(true);
  await shiftEsc(agentOpen);
});

it('Alt+2 abre y cierra el panel del agente', async () => {
  const agentOpen = () => s.browser.execute(() => !!document.querySelector('.synapse-agent-panel')) as Promise<boolean>;
  if (await agentOpen()) {
    await s.browser.$('.editor-header .agent-toggle').click();
    await s.browser.waitUntil(async () => !(await agentOpen()), { timeout: 5_000 });
  }
  await focusOutside();
  await s.browser.keys(['Alt', '2']);
  await s.browser.waitUntil(agentOpen, { timeout: 5_000, timeoutMsg: 'Alt+2 no abrió el agente' });
  // Con el foco dentro del panel también lo cierra
  await s.browser.keys(['Alt', '2']);
  await s.browser.waitUntil(async () => !(await agentOpen()), { timeout: 5_000, timeoutMsg: 'Alt+2 no cerró el agente' });
});
