// El botón del láser (diagramas e imágenes): el punto es redondo (no lo estira la regla del «100%») y toma el color del
// tema (`--laser`), igual que el trazo.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

let s: Session;
beforeAll(async () => {
  s = await startApp({
    'flujo.mmd': 'graph LR\n  A --> B\n',
    'foto.svg': '<svg xmlns="http://www.w3.org/2000/svg" width="300" height="200"><rect width="300" height="200" fill="white"/></svg>',
  });
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

/** Píxeles pintados en el lienzo del láser visible. */
const painted = () =>
  s.browser.execute(() => {
    const c = [...document.querySelectorAll<HTMLCanvasElement>('.laser-canvas')].find((e) => e.offsetParent !== null)!;
    if (!c.width || !c.height) return 0;
    const d = c.getContext('2d')!.getImageData(0, 0, c.width, c.height).data;
    let n = 0;
    for (let i = 3; i < d.length; i += 4) if (d[i]) n++;
    return n;
  }) as Promise<number>;
const mouse = (type: string, dx: number, dy: number) =>
  s.browser.execute(
    (type, dx, dy) => {
      const el = document.querySelector('.image-viewer-container')!;
      const r = el.getBoundingClientRect();
      el.dispatchEvent(new MouseEvent(type, { bubbles: true, button: 0, clientX: r.left + r.width / 2 + dx, clientY: r.top + r.height / 2 + dy }));
    },
    type,
    dx,
    dy,
  );
const drag = async () => {
  await mouse('mousedown', -60, -30);
  for (let i = 0; i <= 6; i++) await mouse('mousemove', -60 + i * 20, -30 + i * 10);
};
const transform = () =>
  s.browser.execute(() => (document.querySelector('.image-viewport') as HTMLElement).style.transform) as Promise<string>;

it('en el visor de imágenes, arrastrar con el láser dibuja un trazo que se apaga y no mueve la imagen', async () => {
  await s.browser.$('.file-tree-item[data-path="foto.svg"]').click();
  const button = s.browser.$('.image-controls .laser-toggle-btn');
  await button.waitForExist({ timeout: 10_000 });
  // Sin láser, arrastrar mueve la imagen
  await drag();
  await mouse('mouseup', 60, 30);
  const moved = await transform();
  expect(moved).toBe('translate(120px, 60px)');
  expect(await painted()).toBe(0);
  // Con láser, dibuja
  await button.click();
  expect(await button.getAttribute('aria-pressed')).toBe('true');
  await drag();
  await s.browser.waitUntil(async () => (await painted()) > 200, { timeout: 5_000, timeoutMsg: 'no dibujó el trazo' });
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/laser-imagen.png`);
  expect(await transform()).toBe(moved);
  await mouse('mouseup', 60, 30);
  await s.browser.waitUntil(async () => (await painted()) === 0, { timeout: 5_000, timeoutMsg: 'el trazo no se apagó' });
  // Al desactivarlo se borra y vuelve a mover
  await drag();
  await button.click();
  await s.browser.waitUntil(async () => (await painted()) === 0, { timeout: 1_000, interval: 50, timeoutMsg: 'no se borró al desactivarlo' });
}, 60_000);
