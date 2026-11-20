// Panel del agente fijado (ocupa su espacio a la derecha) o suelto (flota sobre el editor y se
// oculta al hacer clic fuera o con Esc), como el explorador. La elección se recuerda por bóveda.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

let s: Session;

beforeAll(async () => {
  s = await startApp({ 'uno.md': '# Uno\n' });
  await s.browser.$('.file-tree-item[data-path="uno.md"]').waitForExist({ timeout: 30_000 });
  await s.browser.execute(() => (document.querySelector('.file-tree-item[data-path="uno.md"]') as HTMLElement).click());
}, 120_000);
afterAll(() => s?.close());

const state = () =>
  s.browser.execute(() => {
    const panel = document.querySelector('.synapse-agent-panel') as HTMLElement | null;
    const main = document.querySelector('.synapse-main') as HTMLElement;
    return {
      open: !!panel,
      overlay: !!panel?.classList.contains('is-overlay'),
      mainRight: Math.round(main.getBoundingClientRect().right),
      panelLeft: panel ? Math.round(panel.getBoundingClientRect().left) : null,
      win: window.innerWidth,
    };
  }) as Promise<{ open: boolean; overlay: boolean; mainRight: number; panelLeft: number | null; win: number }>;
const openPanel = async () => {
  await s.browser.execute(() => (document.querySelector('.editor-header .agent-toggle') as HTMLElement).click());
  await s.browser.$('.synapse-agent-panel').waitForExist({ timeout: 5_000 });
};

it('fijado: el panel ocupa su espacio y el editor se encoge', async () => {
  await openPanel();
  const st = await state();
  expect(st.overlay).toBe(false);
  expect(st.mainRight).toBeLessThanOrEqual(st.panelLeft! + 1);
}, 30_000);

it('suelto: flota sobre el editor y se oculta al hacer clic fuera; se recuerda por bóveda', async () => {
  await s.browser.execute(() => (document.querySelector('.acp-pin-btn') as HTMLElement).click());
  await s.browser.waitUntil(async () => (await state()).overlay, { timeout: 5_000, timeoutMsg: 'no pasó a flotar' });
  const st = await state();
  expect(st.mainRight).toBeGreaterThan(st.panelLeft! + 50); // el editor no se encoge
  const saved = (await s.browser.execute(() =>
    Object.keys(localStorage).filter((k) => k.startsWith('synapse.agent.pinned::')).map((k) => localStorage.getItem(k)),
  )) as string[];
  expect(saved).toEqual(['0']);
  // Clic en el editor: se oculta
  await s.browser.execute(() => {
    const el = document.querySelector('.synapse-main') as HTMLElement;
    el.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, cancelable: true }));
  });
  await s.browser.waitUntil(async () => !(await state()).open, { timeout: 5_000, timeoutMsg: 'no se ocultó al hacer clic fuera' });
  // Al volver a abrirlo sigue suelto
  await openPanel();
  expect((await state()).overlay).toBe(true);
  // Esc con el foco en el panel lo oculta
  await s.browser.execute(() => (document.querySelector('.synapse-agent-panel button') as HTMLElement).focus());
  await s.browser.keys('Escape');
  await s.browser.waitUntil(async () => !(await state()).open, { timeout: 5_000, timeoutMsg: 'Esc no lo ocultó' });
}, 30_000);
