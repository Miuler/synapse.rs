// En el Kanban, una tarjeta se arrastra a otra columna (cambia su estado) o se mueve con el teclado.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

const board = `---
type: board
default_view: kanban
columns:
  - { id: todo, label: "Por hacer", color: "var(--text-muted)" }
  - { id: doing, label: "En progreso", color: "var(--accent)" }
  - { id: done, label: "Hecho", color: "#22c55e" }
---

| id | Tarea | Estado | Inicio | Duración | Depende | Progreso |
|---|---|---|---|---|---|---|
| t1 | Primera | todo | 2026-01-01 | 1d | - | 0% |
| t2 | Segunda | todo | 2026-01-01 | 1d | - | 0% |
`;

let s: Session;

beforeAll(async () => {
  s = await startApp({ 'equipo.board.md': board });
  await s.browser.$('.file-tree-item[data-path="equipo.board.md"]').waitForExist({ timeout: 30_000 });
  await s.browser.$('.file-tree-item[data-path="equipo.board.md"]').click();
  await s.browser.$('.kanban-view .card').waitForExist({ timeout: 15_000 });
}, 120_000);
afterAll(() => s?.close());

const file = () => readFileSync(join(s.vault, 'equipo.board.md'), 'utf8');
/** Los títulos de cada columna, en orden. */
const columns = () =>
  s.browser.execute(() =>
    [...document.querySelectorAll('.tab-pane:not(.hidden) .kanban-col')].map((col) =>
      [...col.querySelectorAll('.col-cards .card-title')].map((t) => t.textContent!.trim()),
    ),
  ) as Promise<string[][]>;
const until = (check: () => boolean | Promise<boolean>, what: string) =>
  s.browser.waitUntil(async () => check(), { timeout: 10_000, timeoutMsg: what });

/** El centro de un elemento, en píxeles de la ventana. */
const center = (selector: string, text?: string) =>
  s.browser.execute(
    (selector, text) => {
      const el = [...document.querySelectorAll(selector)].find((e) => !text || e.textContent?.includes(text))!;
      const r = el.getBoundingClientRect();
      return { x: Math.round(r.x + r.width / 2), y: Math.round(r.y + r.height / 2) };
    },
    selector,
    text ?? '',
  ) as Promise<{ x: number; y: number }>;

async function drag(from: { x: number; y: number }, to: { x: number; y: number }) {
  await s.browser
    .action('pointer', { parameters: { pointerType: 'mouse' } })
    .move({ x: from.x, y: from.y })
    .down()
    .pause(100)
    .move({ duration: 150, x: Math.round((from.x + to.x) / 2), y: Math.round((from.y + to.y) / 2) })
    .move({ duration: 150, x: to.x, y: to.y })
    .pause(100)
    .up()
    .perform();
}

it('arrastrar una tarjeta a otra columna cambia su estado y lo guarda', async () => {
  const doing = await center('.tab-pane:not(.hidden) .kanban-col', 'En progreso');
  await drag(await center('.tab-pane:not(.hidden) .card', 'Primera'), doing);
  await until(async () => (await columns())[1].includes('Primera'), 'la tarjeta no cambió de columna');
  expect(await columns()).toEqual([['Segunda'], ['Primera'], []]);
  await until(() => file().includes('| t1 | Primera | doing |'), 'no se guardó el estado');
  expect(await s.browser.execute(() => !!document.querySelector('.kanban-ghost'))).toBe(false);
}, 60_000);

it('soltar fuera de las columnas o pulsar Escape no cambia nada', async () => {
  const card = await center('.tab-pane:not(.hidden) .card', 'Segunda');
  const toolbar = await center('.tab-pane:not(.hidden) .view-switcher');
  await drag(card, toolbar);
  expect(await columns()).toEqual([['Segunda'], ['Primera'], []]);

  const done = await center('.tab-pane:not(.hidden) .kanban-col', 'Hecho');
  const pointer = () => s.browser.action('pointer', { parameters: { pointerType: 'mouse' } });
  await pointer().move({ x: card.x, y: card.y }).down().move({ duration: 150, x: done.x, y: done.y }).perform(true);
  await s.browser.$('.kanban-ghost').waitForExist({ timeout: 5_000 });
  expect(await s.browser.execute(() => window.getSelection()?.toString() ?? '')).toBe('');
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/kanban-arrastre.png`);
  await s.browser.keys(['Escape']);
  await s.browser.releaseActions();
  await s.browser.$('.kanban-ghost').waitForExist({ reverse: true, timeout: 5_000 });
  expect(await columns()).toEqual([['Segunda'], ['Primera'], []]);
}, 60_000);

it('con la tarjeta enfocada, Shift+flechas la cambian de columna y conserva el foco', async () => {
  await s.browser.execute(() =>
    ([...document.querySelectorAll('.tab-pane:not(.hidden) .card')].find((c) => c.textContent?.includes('Segunda')) as HTMLElement).focus(),
  );
  await s.browser.keys(['Shift', 'ArrowRight']);
  await until(async () => (await columns())[1].includes('Segunda'), 'no pasó a la derecha');
  await s.browser.keys(['Shift', 'ArrowRight']);
  await until(async () => (await columns())[2].includes('Segunda'), 'no conservó el foco');
  await s.browser.keys(['Shift', 'ArrowLeft']);
  await until(() => file().includes('| t2 | Segunda | doing |'), 'no se guardó');
}, 60_000);

it('el botón de comentarios de la tarjeta sigue funcionando', async () => {
  await s.browser.$('.tab-pane:not(.hidden) .card .comment-btn').click();
  await s.browser.$('.tab-pane:not(.hidden) .comments-panel').waitForExist({ timeout: 5_000 });
}, 30_000);
