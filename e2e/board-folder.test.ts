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
  expect(await s.browser.$('.task-dialog .detail-assignee input').getValue()).toBe('Ana');
  await s.browser.$('.task-dialog .comment-form input').click();
  await s.browser.keys('hola');
  await s.browser.keys(['Enter']);
  await until(() => / comment\n  hola\n/.test(read('Equipo.board/ado-1.md')), 'no se guardó el comentario');
  await shot('carpeta-detalle');
  await s.browser.$('.task-dialog .resizable-dialog-close').click();
  await s.browser.$('.task-dialog').waitForExist({ reverse: true, timeout: 5_000 });
}, 90_000);

/** Pone un valor en un campo del detalle y lo confirma, como al salir de él. */
const set = (selector: string, value: string) =>
  s.browser.execute(
    (selector, value) => {
      const field = document.querySelector(`.task-dialog ${selector}`) as HTMLInputElement;
      field.value = value;
      field.dispatchEvent(new Event('change', { bubbles: true }));
    },
    selector,
    value,
  );
const write = (selector: string, value: string) =>
  s.browser.execute(
    (selector, value) => {
      const field = document.querySelector(`.task-dialog ${selector}`) as HTMLTextAreaElement;
      field.value = value;
      field.dispatchEvent(new Event('input', { bubbles: true }));
    },
    selector,
    value,
  );
const gone = (selector: string) => s.browser.$(selector).waitForExist({ reverse: true, timeout: 10_000 });

it('desde la tabla y el Gantt se abre el mismo detalle, y todo lo editado va a los archivos', async () => {
  const view = (n: number) => s.browser.$(`${pane} .view-switcher button:nth-child(${n})`).click();
  await view(3);
  // En el Gantt se abre con doble clic en el nombre
  await s.browser.$(`${pane} .gantt-row-label`).waitForExist({ timeout: 5_000 });
  await s.browser.execute(() => {
    const label = [...document.querySelectorAll('.tab-pane:not(.hidden) .gantt-row-label')].find((l) => l.textContent?.includes('Primera'))!;
    label.dispatchEvent(new MouseEvent('click', { bubbles: true, detail: 2 }));
  });
  await s.browser.$('.task-dialog .detail-title').waitForExist({ timeout: 5_000 });
  await s.browser.$('.task-dialog .resizable-dialog-close').click();
  await gone('.task-dialog');
  await view(1);
  await s.browser.$(`${pane} .table-view td.col-id code`).waitForExist({ timeout: 5_000 });
  await s.browser.execute(() => {
    const cell = [...document.querySelectorAll('.tab-pane:not(.hidden) .table-view td.col-id code')].find((c) => c.textContent === 'ado-1')!;
    cell.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
  });
  await s.browser.$('.task-dialog .description').waitForExist({ timeout: 10_000 });
  const task = () => read('Equipo.board/ado-1.md');

  await set('.detail-title', 'Primera editada');
  await set('.detail-type input', 'Task');
  await set('.detail-assignee input', 'Eva');
  await set('.detail-start input', '2026-03-04');
  await set('.detail-duration input', '5');
  await set('.detail-depends select', 't1');
  await set('.detail-progress input', '250');
  await set('select.detail-status', 'todo');
  for (const line of ['# Primera editada', '- type: Task', '- assignee: Eva', '- start: 2026-03-04', '- duration: 5d', '- depends: t1', '- progress: 100%', '- status: todo']) {
    await until(() => task().includes(`${line}\n`), `no se guardó ${line}`);
  }

  // La descripción: lo escrito no se pierde al cerrar sin guardar
  await s.browser.$('.task-dialog button[aria-label="Editar la descripción"]').click();
  await s.browser.$('.task-dialog .editor textarea').waitForExist({ timeout: 5_000 });
  await write('.editor textarea', 'Nueva descripción\n\ncon dos párrafos');
  await s.browser.$('.task-dialog .resizable-dialog-close').click();
  await s.browser.$('.confirm-dialog-btn.cancel').waitForExist({ timeout: 5_000 });
  await s.browser.$('.confirm-dialog-btn.cancel').click();
  await gone('.confirm-dialog');
  await s.browser.$('.task-dialog .editor-save').click();
  await until(() => task().includes('## Description\n\n> Nueva descripción\n>\n> con dos párrafos\n'), 'no se guardó la descripción');
  await gone('.task-dialog .editor');

  // Un comentario propio se cambia sin más
  await s.browser.$('.task-dialog .comment-item button[aria-label="Editar el comentario"]').click();
  await s.browser.$('.task-dialog .editor textarea').waitForExist({ timeout: 5_000 });
  await write('.editor textarea', 'hola de nuevo');
  await s.browser.$('.task-dialog .editor-save').click();
  await until(() => / comment\n  hola de nuevo\n/.test(task()), 'no se guardó el comentario');
  await gone('.task-dialog .editor');

  // El histórico avisa antes; cancelar no abre nada
  const history = '.task-dialog .history button[aria-label="Editar la entrada del histórico"]';
  const before = task();
  await s.browser.$(history).click();
  await s.browser.$('.confirm-dialog').waitForExist({ timeout: 5_000 });
  await shot('carpeta-aviso-historico');
  await s.browser.$('.confirm-dialog-btn.cancel').click();
  await gone('.confirm-dialog');
  expect(await s.browser.$('.task-dialog .editor').isExisting()).toBe(false);
  expect(task()).toBe(before);

  await s.browser.$(history).click();
  await s.browser.$('.confirm-dialog-btn.confirm').waitForExist({ timeout: 5_000 });
  await s.browser.$('.confirm-dialog-btn.confirm').click();
  await s.browser.$('.task-dialog .history .editor textarea').waitForExist({ timeout: 5_000 });
  await write('.editor textarea', 'Estado: corregido');
  await s.browser.$('.task-dialog .editor-save').click();
  await until(() => /history · Luis · id:rev-1 · edited\n  Estado: corregido\n/.test(task()), 'no se marcó la entrada');
  await until(async () => (await s.browser.$('.task-dialog .history .edited').isExisting()), 'no se ve la marca');
  await shot('carpeta-editado');
  await s.browser.$('.task-dialog .resizable-dialog-close').click();
  await gone('.task-dialog');
  // Lo no tocado sigue en el archivo
  for (const kept of ['- prioridad: alta', '- key: AB#1']) expect(task()).toContain(kept);

  // Se devuelve a como lo esperan las pruebas siguientes
  writeFileSync(join(s.vault, 'Equipo.board/ado-1.md'), task().replace('# Primera editada', '# Primera'));
  await view(2);
  await until(async () => (await columns())[0].includes('Primera'), 'no volvió el Kanban');
}, 120_000);

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
