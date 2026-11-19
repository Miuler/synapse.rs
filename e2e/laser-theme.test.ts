// El botón del láser: el punto es redondo (no lo estira la regla del «100%») y toma el color del
// tema (`--laser`), igual que el trazo.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

let s: Session;
beforeAll(async () => {
  s = await startApp({ 'flujo.mmd': 'graph LR\n  A --> B\n' });
  await s.browser.$('.file-tree-item[data-path="flujo.mmd"]').waitForExist({ timeout: 30_000 });
  await s.browser.$('.file-tree-item[data-path="flujo.mmd"]').click();
  await s.browser.$('.laser-toggle-btn').waitForExist({ timeout: 20_000 });
}, 120_000);
afterAll(() => s?.close());

const dot = () =>
  s.browser.execute(() => {
    const el = document.querySelector('.laser-toggle-btn .laser-dot') as HTMLElement;
    const r = el.getBoundingClientRect();
    return { w: Math.round(r.width), h: Math.round(r.height), bg: getComputedStyle(el).backgroundColor };
  });

it('el punto del láser es redondo y al activarlo usa el color del tema', async () => {
  const off = await dot();
  expect(off.w).toBe(off.h);
  // Un tema que cambia el láser a verde
  await s.browser.execute(() => document.documentElement.style.setProperty('--laser', 'rgb(0, 200, 0)'));
  await s.browser.$('.laser-toggle-btn').click();
  await s.browser.waitUntil(async () => (await dot()).bg === 'rgb(0, 200, 0)', {
    timeout: 5_000,
    timeoutMsg: 'el punto activo no tomó --laser',
  });
  const on = await dot();
  expect(on.w).toBe(on.h);
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/laser.png`);
});
