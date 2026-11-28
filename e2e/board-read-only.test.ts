// Un tablero se abre en edición; en modo lectura solo se mira: nada se mueve, se añade, se edita, se
// borra, se comenta ni se sincroniza, y los archivos no cambian.
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

const board = `---
type: board
format: 2
default_view: kanban
columns:
  - { id: "todo", label: "Por hacer", color: "" }
  - { id: "doing", label: "En progreso", color: "" }
source:
  provider: azure_devops
  scope: https://dev.azure.com/org/Demo
---

## Order

- t1
- t2
`;
const files: Record<string, string> = {
  'Equipo.board.md': board,
  'Equipo.board/t1.md': '# Primera\n\n- status: todo\n\n## Activity\n\n- `2026-01-01T10:00:00` comment\n  hola\n',
  'Equipo.board/t2.md': '# Segunda\n\n- status: todo\n',
  'Tabla.board.md': '---\ntype: board\n---\n\n| id | Tarea | Estado | Inicio | Duración | Depende | Progreso |\n|---|---|---|---|---|---|---|\n| t1 | Uno | todo | 2026-01-01 | 1d | - | 0% |\n',
};

let s: Session;
beforeAll(async () => {
  s = await startApp(files);
  await s.browser.$('.file-tree-item[data-path="Equipo.board.md"]').waitForExist({ timeout: 30_000 });
  await s.browser.$('.file-tree-item[data-path="Equipo.board.md"]').click();
  await s.browser.$('.kanban-view .card').waitForExist({ timeout: 15_000 });
}, 120_000);
afterAll(() => s?.close());

const pane = '.tab-pane:not(.hidden)';
const until = (check: () => boolean | Promise<boolean>, what: string) =>
  s.browser.waitUntil(async () => check(), { timeout: 15_000, timeoutMsg: what });
const read = (path: string) => readFileSync(join(s.vault, path), 'utf8');
const snapshot = () =>
  [read('Equipo.board.md'), ...readdirSync(join(s.vault, 'Equipo.board')).sort().map((f) => read(`Equipo.board/${f}`))].join('\n');
const setMode = async (mode: 'live' | 'reading') => {
  await s.browser.$('.md-view-btn').click();
  await s.browser.$(`.view-item.mode-${mode}`).click();
};
const count = (selector: string) => s.browser.execute((q) => document.querySelectorAll(q).length, selector) as Promise<number>;
const moveFirst = async () => {
  await s.browser.execute(() => (document.querySelector('.tab-pane:not(.hidden) .card[data-task="t1"]') as HTMLElement).focus());
  await s.browser.keys(['Shift', 'ArrowRight']);
};

it('se abre en edición', async () => {
  expect(await count(`${pane} .board-readonly`)).toBe(0);
  expect(await count(`${pane} .add-card-btn`)).toBe(2);
});

it('en lectura el Kanban y la tabla solo se miran', async () => {
  await setMode('reading');
  await s.browser.$(`${pane} .board-readonly`).waitForExist({ timeout: 10_000 });
  const before = snapshot();
  expect([await count(`${pane} .add-card-btn`), await count(`${pane} .col-resizer`)]).toEqual([0, 0]);
  expect([await s.browser.$(`${pane} .issues-pull`).isEnabled(), await s.browser.$(`${pane} .issues-push`).isEnabled(), await s.browser.$(`${pane} .issues-settings`).isEnabled()]).toEqual([false, false, false]);
  await moveFirst();
  await s.browser.$(`${pane} .view-switcher button:nth-child(1)`).click();
  await s.browser.$(`${pane} .table-view tbody tr`).waitForExist({ timeout: 5_000 });
  expect(await count(`${pane} .table-view tbody :is(input, select):not(:disabled)`)).toBe(0);
  expect([await count(`${pane} .table-view .icon-btn.danger`), await count(`${pane} .add-row-btn`)]).toEqual([0, 0]);
  await s.browser.pause(800);
  expect(snapshot()).toBe(before);
}, 60_000);

it('el detalle se ve pero no se cambia', async () => {
  await s.browser.$(`${pane} .view-switcher button:nth-child(2)`).click();
  await s.browser.$(`${pane} .card[data-task="t1"] .card-title`).waitForExist({ timeout: 5_000 });
  await s.browser.$(`${pane} .card[data-task="t1"] .card-title`).doubleClick();
  await s.browser.$('.task-dialog .comment-item').waitForExist({ timeout: 10_000 });
  expect(await count('.task-dialog .task-columns :is(input, select, textarea):not(:disabled)')).toBe(0);
  expect([await count('.task-dialog .pencil'), await count('.task-dialog .comment-form')]).toEqual([0, 0]);
  await s.browser.$('.task-dialog .resizable-dialog-close').click();
  await s.browser.$('.task-dialog').waitForExist({ reverse: true, timeout: 5_000 });
}, 60_000);

it('un tablero con tabla en lectura no se puede convertir', async () => {
  await s.browser.$('.file-tree-item[data-path="Tabla.board.md"]').click();
  await s.browser.$(`${pane} .board-convert button`).waitForExist({ timeout: 15_000 });
  await setMode('reading');
  await until(async () => !(await s.browser.$(`${pane} .board-convert button`).isEnabled()), 'dejó convertir en lectura');
}, 60_000);

it('al volver a edición se puede cambiar otra vez', async () => {
  await s.browser.$('.file-tree-item[data-path="Equipo.board.md"]').click();
  await s.browser.$(`${pane} .board-readonly`).waitForExist({ timeout: 10_000 });
  await setMode('live');
  await s.browser.$(`${pane} .add-card-btn`).waitForExist({ timeout: 10_000 });
  await moveFirst();
  await until(() => read('Equipo.board/t1.md').includes('- status: doing'), 'no se movió');
}, 60_000);
