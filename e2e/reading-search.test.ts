// La búsqueda del modo lectura no debe bloquear al escribir en una nota grande, y navega por las coincidencias.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

let s: Session;
const NOTE = Array.from({ length: 300 }, (_, i) => `Párrafo ${i} con varias palabras para buscar dentro de la nota.`).join('\n\n');
const input = () => s.browser.$('.reading-search-input');
const marks = () => s.browser.execute(() => document.querySelectorAll('mark.reading-search-highlight').length) as Promise<number>;

beforeAll(async () => {
  s = await startApp({ 'grande.md': NOTE });
  const item = s.browser.$('.file-tree-item[data-path="grande.md"]');
  await item.waitForExist({ timeout: 30_000 });
  await item.click();
  await s.browser.$('.markdown-body p').waitForExist({ timeout: 30_000 });
  await s.browser.pause(500);
  await s.browser.keys(['Control', 'f']);
  await s.browser.keys(['Control']);
  await input().waitForExist({ timeout: 5_000 });
}, 120_000);

afterAll(async () => {
  await s?.close();
});

it('escribir no bloquea la interfaz y resalta el resultado final', async () => {
  // Mide el mayor tiempo sin que el hilo principal responda mientras se escribe.
  await s.browser.execute(() => {
    const w = window as any;
    w.__maxGap = 0;
    let last = performance.now();
    w.__gapTimer = setInterval(() => {
      const now = performance.now();
      w.__maxGap = Math.max(w.__maxGap, now - last);
      last = now;
    }, 10);
  });
  await s.browser.keys(['a']);
  await s.browser.waitUntil(async () => (await marks()) > 300, { timeout: 60_000, timeoutMsg: 'sin resaltados' });
  await s.browser.keys(['r']);
  await s.browser.waitUntil(async () => (await marks()) === 900, { timeout: 60_000, timeoutMsg: 'resultado final incorrecto' });
  await s.browser.pause(300);
  const maxGap = (await s.browser.execute(() => {
    clearInterval((window as any).__gapTimer);
    return (window as any).__maxGap;
  })) as number;
  expect(maxGap).toBeLessThan(2_000);
}, 120_000);

it('Enter avanza a la siguiente coincidencia', async () => {
  const active = () =>
    s.browser.execute(() => {
      const all = [...document.querySelectorAll('mark.reading-search-highlight')];
      return all.findIndex((m) => m.classList.contains('reading-search-active'));
    }) as Promise<number>;
  expect(await active()).toBe(0);
  await s.browser.keys(['Enter']);
  await s.browser.waitUntil(async () => (await active()) === 1, { timeout: 5_000, timeoutMsg: 'no avanzó' });
}, 30_000);
