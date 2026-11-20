// Explorador fijado (ocupa su espacio) o suelto (flota sobre el editor sin desplazarlo, y se oculta
// al abrir un archivo, con Esc o al hacer clic fuera). La elección se recuerda.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

let s: Session;

beforeAll(async () => {
  s = await startApp({ 'uno.md': '# Uno\n', 'dos.md': '# Dos\n' });
  await s.browser.$('.file-tree-item[data-path="uno.md"]').waitForExist({ timeout: 30_000 });
  await s.browser.execute(() => {
    for (const k of Object.keys(localStorage)) if (k.startsWith('synapse.explorer.pinned')) localStorage.removeItem(k);
  });
}, 120_000);
afterAll(() => s?.close());

const rects = () =>
  s.browser.execute(() => {
    const r = (q: string) => document.querySelector(q)?.getBoundingClientRect();
    const side = document.querySelector('.synapse-explorer') as HTMLElement;
    return {
      ribbon: Math.round(r('.synapse-ribbon')!.right),
      main: Math.round(r('.synapse-main')!.left),
      sideLeft: Math.round(r('.synapse-explorer')!.left),
      sideRight: Math.round(r('.synapse-explorer')!.right),
      visible: !!side && getComputedStyle(side).display !== 'none',
      floating: side.classList.contains('is-floating'),
    };
  }) as Promise<{ ribbon: number; main: number; sideLeft: number; sideRight: number; visible: boolean; floating: boolean }>;

const filesButton = 'button[aria-label^="Explorador de archivos"]';

it('fijado ocupa espacio; suelto flota sobre el editor y se oculta al abrir un archivo', async () => {
  // Por defecto, fijado: el editor empieza donde termina el explorador
  let r = await rects();
  expect(r.floating).toBe(false);
  expect(r.main).toBeGreaterThanOrEqual(r.sideRight - 2);

  // Soltarlo: el editor ocupa el espacio y el explorador flota pegado a la barra lateral
  await s.browser.$('.sidebar-pin-btn').click();
  await s.browser.waitUntil(async () => (await rects()).floating, { timeout: 5_000, timeoutMsg: 'no se soltó' });
  r = await rects();
  expect(r.floating).toBe(true);
  expect(r.visible).toBe(true);
  expect(Math.abs(r.sideLeft - r.ribbon)).toBeLessThanOrEqual(2);
  expect(r.main).toBeLessThan(r.sideRight - 50);

  // Abrir un archivo lo oculta
  await s.browser.$('.file-tree-item[data-path="dos.md"]').click();
  await s.browser.waitUntil(async () => !(await rects()).visible, { timeout: 5_000, timeoutMsg: 'no se ocultó al abrir un archivo' });

  // Se vuelve a mostrar desde la barra lateral, flotando; Esc lo oculta
  await s.browser.$(filesButton).click();
  await s.browser.waitUntil(async () => (await rects()).visible, { timeout: 5_000 });
  await s.browser.$('.synapse-explorer').click();
  await s.browser.keys('Escape');
  await s.browser.waitUntil(async () => !(await rects()).visible, { timeout: 5_000, timeoutMsg: 'Esc no lo ocultó' });

  // Clic fuera (en el editor) también lo oculta
  await s.browser.$(filesButton).click();
  await s.browser.waitUntil(async () => (await rects()).visible, { timeout: 5_000 });
  await s.browser.$('.synapse-main').click();
  await s.browser.waitUntil(async () => !(await rects()).visible, { timeout: 5_000, timeoutMsg: 'clic fuera no lo ocultó' });

  // La elección se recuerda; fijarlo de nuevo vuelve al comportamiento de siempre
  // Se recuerda por bóveda (la clave lleva su ruta)
  expect(
    await s.browser.execute(() =>
      Object.keys(localStorage).filter((k) => k.startsWith('synapse.explorer.pinned::')).map((k) => localStorage.getItem(k)),
    ),
  ).toEqual(['0']);
  await s.browser.$(filesButton).click();
  await s.browser.waitUntil(async () => (await rects()).visible, { timeout: 5_000 });
  await s.browser.$('.sidebar-pin-btn').click();
  await s.browser.waitUntil(async () => !(await rects()).floating, { timeout: 5_000, timeoutMsg: 'no se fijó' });
  r = await rects();
  expect(r.floating).toBe(false);
  expect(r.main).toBeGreaterThanOrEqual(r.sideRight - 2);
});
