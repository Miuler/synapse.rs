// Un tablero de carpeta (`format: 2`): cada tarea es un archivo. Las vistas muestran sus datos, los
// cambios se escriben en el archivo que toca y lo que cambia por fuera se vuelve a leer.
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

const board = `---
type: board
format: 2
default_view: kanban
columns:
  - { id: "todo", label: "Por hacer", color: "var(--text-muted)" }
  - { id: "doing", label: "En progreso", color: "var(--accent)" }
tags:
  - mio
---

## Order

- ado-1
- t1
`;

const first = `# Primera

- status: todo
- type: Bug
- assignee: Ana
- key: AB#1
- start: 2026-01-01
- duration: 1d
- depends: -
- progress: 0%
- prioridad: alta

## Description

> Texto largo
> en dos líneas

## Activity

- \`2026-01-02T10:00:00\` history · Luis · id:rev-1
  Estado: New → To Do
`;

let s: Session;
beforeAll(async () => {
  s = await startApp({
    'Equipo.board.md': board,
    'Equipo.board/ado-1.md': first,
    'Equipo.board/t1.md': '# Segunda\n\n- status: todo\n',
  });
  await s.browser.$('.file-tree-item[data-path="Equipo.board.md"]').waitForExist({ timeout: 30_000 });
  await s.browser.$('.file-tree-item[data-path="Equipo.board.md"]').click();
  await s.browser.$('.kanban-view .card').waitForExist({ timeout: 15_000 });
}, 120_000);
afterAll(() => s?.close());

const read = (path: string) => readFileSync(join(s.vault, path), 'utf8');
const until = (check: () => boolean | Promise<boolean>, what: string) =>
  s.browser.waitUntil(async () => check(), { timeout: 15_000, timeoutMsg: what });
const pane = '.tab-pane:not(.hidden)';
const columns = () =>
  s.browser.execute(() =>
    [...document.querySelectorAll('.tab-pane:not(.hidden) .kanban-col')].map((col) =>
      [...col.querySelectorAll('.card-title')].map((t) => t.textContent!.trim()),
    ),
  ) as Promise<string[][]>;
const focus = (id: string) =>
  s.browser.execute((id) => (document.querySelector(`.tab-pane:not(.hidden) .card[data-task="${id}"]`) as HTMLElement).focus(), id);
const shot = async (name: string) => {
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/${name}.png`);
};

it('muestra las tareas de sus archivos con referencia, tipo y asignado', async () => {
  expect(await columns()).toEqual([['Primera', 'Segunda'], []]);
  const card = await s.browser.$(`${pane} .card[data-task="ado-1"]`).getText();
  for (const text of ['AB#1', 'Bug', 'Ana']) expect(card).toContain(text);
  await shot('carpeta-kanban');
}, 60_000);

it('reordenar toca solo el tablero y cambiar de columna solo la tarea', async () => {
  const task = read('Equipo.board/ado-1.md');
  await focus('t1');
  await s.browser.keys(['Shift', 'ArrowUp']);
  await until(() => read('Equipo.board.md').includes('## Order\n\n- t1\n- ado-1\n'), 'no se guardó el orden');
  expect(read('Equipo.board/ado-1.md')).toBe(task);
  expect(read('Equipo.board.md')).toContain('tags:\n  - mio\n');

  const order = read('Equipo.board.md');
  await focus('ado-1');
  await s.browser.keys(['Shift', 'ArrowRight']);
  await until(() => read('Equipo.board/ado-1.md').includes('- status: doing'), 'no se guardó la columna');
  const moved = read('Equipo.board/ado-1.md');
  // Lo demás del archivo sigue igual
  for (const kept of ['- prioridad: alta', '> Texto largo\n> en dos líneas', 'Estado: New → To Do', '- key: AB#1']) expect(moved).toContain(kept);
  expect(read('Equipo.board.md')).toBe(order);
}, 90_000);

it('el detalle muestra descripción e histórico, y un comentario va al archivo de la tarea', async () => {
  await s.browser.$(`${pane} .card[data-task="ado-1"] .card-title`).doubleClick();
  await s.browser.$('.task-dialog .description').waitForExist({ timeout: 10_000 });
  expect(await s.browser.$('.task-dialog .description').getText()).toBe('Texto largo\nen dos líneas');
  expect(await s.browser.$('.task-dialog .history').getText()).toContain('Estado: New → To Do');
  expect(await s.browser.$('.task-dialog .detail-assignee').getText()).toBe('Ana');
  await s.browser.$('.task-dialog .comment-form input').click();
  await s.browser.keys('hola');
  await s.browser.keys(['Enter']);
  await until(() => / comment\n  hola\n/.test(read('Equipo.board/ado-1.md')), 'no se guardó el comentario');
  await shot('carpeta-detalle');
  await s.browser.$('.task-dialog .resizable-dialog-close').click();
  await s.browser.$('.task-dialog').waitForExist({ reverse: true, timeout: 5_000 });
}, 90_000);

it('añadir crea un archivo y eliminar lo quita', async () => {
  await s.browser.$(`${pane} .add-card-btn`).click();
  await until(() => existsSync(join(s.vault, 'Equipo.board/t3.md')), 'no se creó el archivo');
  await until(() => read('Equipo.board.md').includes('- t3\n'), 'no entró en el orden');
  await s.browser.$(`${pane} .view-switcher button:nth-child(1)`).click();
  await s.browser.$(`${pane} .table-view`).waitForExist({ timeout: 5_000 });
  expect(await s.browser.$(`${pane} .table-view`).getText()).toContain('AB#1');
  await shot('carpeta-tabla');
  await s.browser.execute(() => {
    const row = [...document.querySelectorAll('.tab-pane:not(.hidden) .table-view tbody tr')].find((r) => r.querySelector('code')?.textContent === 't3')!;
    (row.querySelector('.icon-btn.danger') as HTMLElement).click();
  });
  await until(() => !existsSync(join(s.vault, 'Equipo.board/t3.md')) && !read('Equipo.board.md').includes('- t3'), 'no se eliminó');
  await s.browser.$(`${pane} .view-switcher button:nth-child(2)`).click();
}, 90_000);

it('lo que cambia por fuera se vuelve a leer, y un archivo roto se avisa', async () => {
  writeFileSync(join(s.vault, 'Equipo.board/t1.md'), '# Cambiada fuera\n\n- status: todo\n');
  await until(async () => (await columns())[0].includes('Cambiada fuera'), 'no se releyó');
  writeFileSync(join(s.vault, 'Equipo.board/rota.md'), '# R\n- sync: {\n');
  await until(
    () => s.browser.execute(() => document.querySelector('.tab-pane:not(.hidden) .board-problem')?.textContent?.includes('rota.md') ?? false),
    'no se avisó del archivo roto',
  );
}, 90_000);

it('el Gantt pinta las tareas de los archivos', async () => {
  await s.browser.$(`${pane} .view-switcher button:nth-child(3)`).click();
  await s.browser.$(`${pane} .gantt-view`).waitForExist({ timeout: 5_000 });
  const titles = await s.browser.execute(() =>
    [...document.querySelectorAll('.tab-pane:not(.hidden) .gantt-row-label .task-title')].map((t) => t.textContent!.trim()),
  );
  expect(titles).toEqual(['Cambiada fuera', 'Primera']);
  await shot('carpeta-gantt');
}, 60_000);

it('la vista Raw muestra el archivo del tablero tal como quedó', async () => {
  await s.browser.$(`${pane} .view-raw`).click();
  await until(
    () =>
      s.browser.execute(() =>
        [...document.querySelectorAll('.tab-pane:not(.hidden) .board-raw .cm-line')].map((l) => l.textContent).join('\n'),
      ).then((shown) => String(shown).trimEnd() === read('Equipo.board.md').trimEnd()),
    'la vista Raw no refleja el archivo',
  );
}, 60_000);
