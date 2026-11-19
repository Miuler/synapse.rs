// Visor de diagramas (Mermaid): la rueda hace zoom con el cursor como ancla y, con cualquier zoom,
// arrastrar mueve el diagrama exactamente lo mismo que el mouse.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

let s: Session;

beforeAll(async () => {
  s = await startApp({ 'flujo.mmd': 'graph LR\n  A[Inicio] --> B[Paso] --> C[Fin]\n' });
  await s.browser.$('.file-tree-item[data-path="flujo.mmd"]').waitForExist({ timeout: 30_000 });
  await s.browser.$('.file-tree-item[data-path="flujo.mmd"]').click();
  await s.browser.$('.preview-pane svg').waitForExist({ timeout: 20_000 });
  // Esperar a que el diagrama deje de moverse (el visor lo encaja tras el render)
  let last = '';
  await s.browser.waitUntil(
    async () => {
      const now = JSON.stringify(
        await s.browser.execute(() => {
          const r = document.querySelector('.preview-pane svg')!.getBoundingClientRect();
          return [r.left, r.top, r.width, r.height].map(Math.round);
        }),
      );
      const stable = now === last;
      last = now;
      return stable;
    },
    { timeout: 10_000, interval: 150, timeoutMsg: 'el diagrama no se quedó quieto' },
  );
}, 120_000);
afterAll(() => s?.close());

/** Punto del diagrama (unidades del SVG) que está bajo la posición de pantalla (x, y). */
const svgPointAt = (x: number, y: number) =>
  s.browser.execute(
    (cx: number, cy: number) => {
      const svg = document.querySelector('.preview-pane svg') as SVGSVGElement;
      const p = new DOMPoint(cx, cy).matrixTransform(svg.getScreenCTM()!.inverse());
      return { x: p.x, y: p.y };
    },
    x,
    y,
  ) as Promise<{ x: number; y: number }>;

/** Posición de pantalla de un punto del diagrama. */
const screenOf = (p: { x: number; y: number }) =>
  s.browser.execute(
    (px: number, py: number) => {
      const svg = document.querySelector('.preview-pane svg') as SVGSVGElement;
      const q = new DOMPoint(px, py).matrixTransform(svg.getScreenCTM()!);
      return { x: q.x, y: q.y };
    },
    p.x,
    p.y,
  ) as Promise<{ x: number; y: number }>;

const pane = () =>
  s.browser.execute(() => {
    const r = document.querySelector('.preview-pane')!.getBoundingClientRect();
    return { left: r.left, top: r.top, width: r.width, height: r.height };
  }) as Promise<{ left: number; top: number; width: number; height: number }>;

it('el zoom con la rueda deja quieto el punto bajo el cursor', async () => {
  const r = await pane();
  const mx = r.left + r.width * 0.3;
  const my = r.top + r.height * 0.35;
  const before = await svgPointAt(mx, my);
  const d0 = (await screenOf({ x: before.x + 10, y: before.y })).x - mx;
  await s.browser.execute(
    (x: number, y: number) => {
      const el = document.querySelector('.preview-pane')!;
      for (let i = 0; i < 3; i++) el.dispatchEvent(new WheelEvent('wheel', { deltaY: -120, clientX: x, clientY: y, bubbles: true, cancelable: true }));
    },
    mx,
    my,
  );
  // El zoom se aplica en el siguiente frame: esperar a que cambie la escala
  await s.browser.waitUntil(
    async () => (await screenOf({ x: before.x + 10, y: before.y })).x - mx > d0 * 1.05,
    { timeout: 5_000, timeoutMsg: 'la rueda no hizo zoom' },
  );
  const after = await svgPointAt(mx, my);
  expect(Math.abs(after.x - before.x)).toBeLessThan(0.5);
  expect(Math.abs(after.y - before.y)).toBeLessThan(0.5);
});

it('con zoom, el diagrama se mueve lo mismo que el mouse', async () => {
  const r = await pane();
  const x0 = r.left + r.width / 2;
  const y0 = r.top + r.height / 2;
  const anchor = await svgPointAt(x0, y0);
  const startScreen = await screenOf(anchor);
  await s.browser.execute(
    (x: number, y: number) => {
      const el = document.querySelector('.preview-pane')!;
      const ev = (type: string, cx: number, cy: number) =>
        el.dispatchEvent(new PointerEvent(type, { bubbles: true, clientX: cx, clientY: cy, pointerId: 1, button: 0 }));
      ev('pointerdown', x, y);
      ev('pointermove', x + 100, y + 60);
      ev('pointerup', x + 100, y + 60);
    },
    x0,
    y0,
  );
  await s.browser.waitUntil(
    async () => Math.round((await screenOf(anchor)).x - startScreen.x) === 100,
    { timeout: 5_000, timeoutMsg: 'el arrastre no movió el diagrama' },
  );
  const endScreen = await screenOf(anchor);
  expect(Math.round(endScreen.x - startScreen.x)).toBe(100);
  expect(Math.round(endScreen.y - startScreen.y)).toBe(60);
});
