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
  // Soltar de verdad: `releaseActions` dejaba el botón pulsado para las pruebas siguientes
  await pointer().up().perform();
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

// El diálogo se pinta fuera de la pestaña
const dialog = () => s.browser.$('.task-dialog');
const closeDialog = async () => {
  await s.browser.$('.task-dialog .resizable-dialog-close').click();
  await dialog().waitForExist({ reverse: true, timeout: 5_000 });
};

it('el botón de comentarios de la tarjeta abre el diálogo de la tarea', async () => {
  await s.browser.$('.tab-pane:not(.hidden) .card .comment-btn').click();
  await dialog().waitForExist({ timeout: 5_000 });
  await closeDialog();
}, 30_000);

const card = (title: string) => center('.tab-pane:not(.hidden) .card', title);
const focusCard = (title: string) =>
  s.browser.execute((title) => {
    const cards = [...document.querySelectorAll<HTMLElement>('.tab-pane:not(.hidden) .card')];
    cards.find((c) => c.textContent?.includes(title))!.focus();
  }, title);
/** Los ids de las filas, en el orden en que se guardaron. */
const rows = () => [...file().matchAll(/^\| (t\d) \|/gm)].map((m) => m[1]);

it('arrastrar una tarjeta encima de otra la pone primero y guarda ese orden', async () => {
  expect((await columns())[1]).toEqual(['Primera', 'Segunda']);
  const first = await card('Primera');
  const hold = () => s.browser.action('pointer', { parameters: { pointerType: 'mouse' } });
  const from = await card('Segunda');
  await hold().move({ x: from.x, y: from.y }).down().move({ duration: 150, x: first.x, y: first.y - 15 }).perform(true);
  await s.browser.$('.card.drop-before').waitForExist({ timeout: 5_000 });
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/kanban-orden.png`);
  await hold().up().perform();
  await until(async () => (await columns())[1][0] === 'Segunda', 'no cambió el orden');
  await until(() => rows().join() === 't2,t1', 'no se guardó el orden');

  // Y de vuelta, soltándola debajo de la otra
  const below = await card('Primera');
  await drag(await card('Segunda'), { x: below.x, y: below.y + 25 });
  await until(() => rows().join() === 't1,t2', 'no volvió abajo');
}, 60_000);

it('con la tarjeta enfocada, Shift+↑ y Shift+↓ cambian cuál va primero', async () => {
  await focusCard('Segunda');
  await s.browser.keys(['Shift', 'ArrowUp']);
  await until(async () => (await columns())[1][0] === 'Segunda', 'no subió');
  await s.browser.keys(['Shift', 'ArrowDown']);
  await until(async () => (await columns())[1][0] === 'Primera', 'no bajó o perdió el foco');
}, 60_000);

it('el doble clic abre el diálogo de la tarea sin moverla, y Escape lo cierra', async () => {
  const before = await columns();
  await s.browser.$('.tab-pane:not(.hidden) .card[data-task="t2"] .card-title').doubleClick();
  await dialog().waitForExist({ timeout: 5_000 });
  const text = (selector: string) => s.browser.$(`.task-dialog ${selector}`).getText();
  expect(await text('.resizable-dialog-title')).toBe('Segunda');
  const value = (selector: string) => s.browser.$(`.task-dialog ${selector}`).getValue();
  expect(await value('.detail-status')).toBe('doing');
  expect(await value('.detail-start input')).toBe('2026-01-01');
  expect(await value('.detail-duration input')).toBe('1');
  expect(await value('.detail-progress input')).toBe('0');
  // Una tabla no guarda tipo ni asignado
  expect(await s.browser.$('.task-dialog .detail-assignee').isExisting()).toBe(false);
  expect(await columns()).toEqual(before);

  // Con un comentario a medio escribir, Escape no cierra; al enviarlo se guarda
  const input = s.browser.$('.task-dialog .comment-form input');
  await input.click();
  await s.browser.keys('Hola');
  await s.browser.keys(['Escape']);
  await s.browser.pause(300);
  expect(await dialog().isExisting()).toBe(true);
  await s.browser.keys(['Enter']);
  await until(async () => (await text('.comments-list')).includes('Hola'), 'no se añadió el comentario');
  await until(() => /^### t2\n- `[^`]+` — Hola$/m.test(file()), 'no se guardó el comentario');
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/kanban-detalle.png`);

  await s.browser.keys(['Escape']);
  await dialog().waitForExist({ reverse: true, timeout: 5_000 });
}, 60_000);

it('Enter sobre la tarjeta enfocada abre el diálogo y al cerrarlo vuelve el foco', async () => {
  await focusCard('Primera');
  await s.browser.keys(['Enter']);
  await dialog().waitForExist({ timeout: 5_000 });
  expect(await s.browser.$('.task-dialog .resizable-dialog-title').getText()).toBe('Primera');
  await s.browser.keys(['Escape']);
  await dialog().waitForExist({ reverse: true, timeout: 5_000 });
  await until(
    () => s.browser.execute(() => (document.activeElement as HTMLElement)?.dataset.task === 't1'),
    'el foco no volvió a la tarjeta',
  );
}, 30_000);

it('en la tabla el doble clic en el id, y en el Gantt en el nombre, abren el diálogo', async () => {
  const view = (n: number) => s.browser.$(`.tab-pane:not(.hidden) .view-switcher button:nth-child(${n})`).click();
  await view(1);
  await s.browser.$('.tab-pane:not(.hidden) .table-view td.col-id code').doubleClick();
  await dialog().waitForExist({ timeout: 5_000 });
  await closeDialog();

  await view(3);
  const label = s.browser.$('.tab-pane:not(.hidden) .gantt-row-label');
  await label.click();
  await s.browser.pause(600);
  expect(await dialog().isExisting()).toBe(false);
  await label.doubleClick();
  await dialog().waitForExist({ timeout: 5_000 });
  await closeDialog();
}, 60_000);

const colWidth = (id: string) =>
  s.browser.execute(
    (id) => (document.querySelector(`.tab-pane:not(.hidden) [data-column="${id}"]`) as HTMLElement).offsetWidth,
    id,
  );

it('el borde de una columna cambia su ancho, lo guarda y se restablece con doble clic', async () => {
  await s.browser.$('.tab-pane:not(.hidden) .view-switcher button:nth-child(2)').click();
  await s.browser.$('.tab-pane:not(.hidden) .col-resizer').waitForExist({ timeout: 5_000 });
  const normal = await colWidth('todo');
  const handle = '.tab-pane:not(.hidden) [data-column="todo"] .col-resizer';
  const from = await center(handle);
  await drag(from, { x: from.x + 120, y: from.y });
  await until(async () => (await colWidth('todo')) === normal + 120, 'no se ensanchó');
  await until(() => file().includes(`column_widths:\n  todo: ${normal + 120}\n`), 'no se guardó el ancho');
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/kanban-ancho.png`);

  // Con el teclado
  await s.browser.execute((selector) => (document.querySelector(selector) as HTMLElement).focus(), handle);
  await s.browser.keys(['ArrowLeft']);
  await until(async () => (await colWidth('todo')) === normal + 100, 'la flecha no lo estrechó');

  // Se conserva al volver a la vista, y las tarjetas se siguen arrastrando a la columna
  await s.browser.$('.tab-pane:not(.hidden) .view-switcher button:nth-child(1)').click();
  await s.browser.$('.tab-pane:not(.hidden) .view-switcher button:nth-child(2)').click();
  await s.browser.$(handle).waitForExist({ timeout: 5_000 });
  expect(await colWidth('todo')).toBe(normal + 100);
  const count = (await columns())[0].length;
  await drag(await card('Primera'), await center('.tab-pane:not(.hidden) [data-column="todo"] .col-header'));
  await until(async () => (await columns())[0].length === count + 1, 'ya no se arrastran las tarjetas');

  await s.browser.$(handle).doubleClick();
  await until(async () => (await colWidth('todo')) === normal, 'no se restableció');
  await until(() => !file().includes('column_widths'), 'quedó la clave');
}, 60_000);

const tableIds = () =>
  s.browser.execute(() =>
    [...document.querySelectorAll('.tab-pane:not(.hidden) .table-view tbody td.col-id')].map((c) => c.textContent!.trim()),
  ) as Promise<string[]>;

it('la tabla se ordena por cualquier columna sin cambiar el orden del archivo', async () => {
  await s.browser.$('.tab-pane:not(.hidden) .view-switcher button:nth-child(1)').click();
  await s.browser.$('.tab-pane:not(.hidden) .table-view').waitForExist({ timeout: 5_000 });
  const initial = await tableIds();
  const saved = file();
  const header = (key: string) => s.browser.$(`.tab-pane:not(.hidden) .sort-btn[data-sort="${key}"]`);
  const sorted = [...initial].sort();

  await header('id').click();
  await until(async () => (await tableIds()).join() === sorted.join(), 'no ordenó ascendente');
  await header('id').click();
  await until(async () => (await tableIds()).join() === [...sorted].reverse().join(), 'no ordenó descendente');
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/tabla-orden.png`);

  // Se conserva al cambiar de vista
  await s.browser.$('.tab-pane:not(.hidden) .view-switcher button:nth-child(2)').click();
  await s.browser.$('.tab-pane:not(.hidden) .view-switcher button:nth-child(1)').click();
  await s.browser.$('.tab-pane:not(.hidden) .table-view').waitForExist({ timeout: 5_000 });
  expect(await tableIds()).toEqual([...sorted].reverse());

  // Al escribir, la fila no salta de sitio
  await header('title').click();
  await until(async () => (await header('title').getText()).length > 0, 'sin cabecera');
  const byTitle = await tableIds();
  await s.browser.execute(() => {
    const input = document.querySelector<HTMLInputElement>('.tab-pane:not(.hidden) .table-view tbody input[type="text"]')!;
    input.value = 'Zeta';
    input.dispatchEvent(new Event('change', { bubbles: true }));
  });
  await until(() => file().includes('| Zeta |'), 'no se guardó');
  expect(await tableIds()).toEqual(byTitle);

  // Tercer clic: el orden del tablero, que no cambió
  await header('title').click();
  await header('title').click();
  await until(async () => (await tableIds()).join() === initial.join(), 'no volvió al orden del tablero');
  expect([...file().matchAll(/^\| (t\d) \|/gm)].map((m) => m[1])).toEqual(
    [...saved.matchAll(/^\| (t\d) \|/gm)].map((m) => m[1]),
  );
}, 60_000);
