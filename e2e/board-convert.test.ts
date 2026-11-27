// Un tablero con tabla se puede pasar a un archivo por tarea: solo si se confirma, sin perder lo que
// tenía y sin que un guardado pendiente vuelva a escribir la tabla encima.
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

const board = `---
type: board
default_view: kanban
columns:
  - { id: todo, label: "Por hacer", color: "var(--text-muted)" }
  - { id: doing, label: "En progreso", color: "var(--accent)" }
azure_devops:
  project_url: https://dev.azure.com/org/Demo
  query: SELECT 1
  links:
    ado-7:
      id: 7
      title: Siete
      state: To Do
      column: todo
      sent: ["2026-01-02T10:00:00"]
---

| id | Tarea | Estado | Inicio | Duración | Depende | Progreso | Prioridad |
|---|---|---|---|---|---|---|---|
| ado-7 | Siete | todo | 2026-01-01 | 3d | - | 40% | alta |
| t2 | Dos | todo | 2026-01-02 | 1d | ado-7 | 0% | |

## Comentarios

### ado-7
- \`2026-01-02T10:00:00\` — ya enviado
`;

// Sin columnas propias: al editar un tablero con tabla, la interfaz solo escribe las suyas
const plain = `---
type: board
---

| id | Tarea | Estado | Inicio | Duración | Depende | Progreso |
|---|---|---|---|---|---|---|
| t1 | Uno | todo | 2026-01-01 | 1d | - | 0% |
`;

const broken = `---
type: board
---

| id | Tarea | Estado | Inicio | Duración | Depende | Progreso |
|---|---|---|---|---|---|---|
| con espacio | Mal | todo | 2026-01-01 | 1d | - | 0% |
`;

let s: Session;
beforeAll(async () => {
  s = await startApp({ 'Equipo.board.md': board, 'Rapido.board.md': plain, 'Roto.board.md': broken });
  await s.browser.$('.file-tree-item[data-path="Equipo.board.md"]').waitForExist({ timeout: 30_000 });
  await s.browser.$('.file-tree-item[data-path="Equipo.board.md"]').click();
  await s.browser.$('.kanban-view .card').waitForExist({ timeout: 15_000 });
}, 120_000);
afterAll(() => s?.close());

const pane = '.tab-pane:not(.hidden)';
const read = (path: string) => readFileSync(join(s.vault, path), 'utf8');
const until = (check: () => boolean | Promise<boolean>, what: string) =>
  s.browser.waitUntil(async () => check(), { timeout: 15_000, timeoutMsg: what });
const columns = () =>
  s.browser.execute(() =>
    [...document.querySelectorAll('.tab-pane:not(.hidden) .kanban-col')].map((col) =>
      [...col.querySelectorAll('.card-title')].map((t) => t.textContent!.trim()),
    ),
  ) as Promise<string[][]>;
const shot = async (name: string) => {
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/${name}.png`);
};
const ask = async () => {
  await s.browser.$(`${pane} .board-convert button`).click();
  await s.browser.$('.confirm-dialog').waitForExist({ timeout: 10_000 });
};
const answer = async (button: 'confirm' | 'cancel') => {
  await s.browser.$(`.confirm-dialog-btn.${button}`).click();
  await s.browser.$('.confirm-dialog').waitForExist({ reverse: true, timeout: 10_000 });
};

it('sin confirmar no cambia nada', async () => {
  await ask();
  await shot('convertir-confirmacion');
  await answer('cancel');
  await s.browser.pause(1_000);
  expect(read('Equipo.board.md')).toBe(board);
  expect(readdirSync(s.vault).filter((f) => f.startsWith('Equipo')).sort()).toEqual(['Equipo.board.md']);
}, 60_000);

it('al confirmar, cada tarea pasa a su archivo sin perder nada y queda una copia', async () => {
  await ask();
  await answer('confirm');
  await until(() => existsSync(join(s.vault, 'Equipo.board/t2.md')), 'no se convirtió');
  await until(async () => !(await s.browser.$(`${pane} .board-convert`).isExisting()), 'sigue ofreciendo convertir');
  await until(async () => (await columns()).map((c) => c.join()).join('|') === 'Siete,Dos|', 'las columnas no muestran las tareas');
  const file = read('Equipo.board.md');
  expect(file).toContain('format: 2\n');
  expect(file).toContain('## Order\n\n- ado-7\n- t2\n');
  expect(file).toContain('provider: azure_devops');
  expect(file).toContain('query: SELECT 1');
  expect(file).not.toContain('| ado-7 |');
  const first = read('Equipo.board/ado-7.md');
  for (const kept of ['# Siete', '- duration: 3d', '- Prioridad: alta', '"sent":["2026-01-02T10:00:00"]', 'ya enviado']) {
    expect(first).toContain(kept);
  }
  expect(read('Equipo.board.md.bak')).toBe(board);
  await shot('convertir-hecho');
}, 90_000);

it('después se trabaja sobre los archivos', async () => {
  await s.browser.$(`${pane} .card[data-task="ado-7"] .card-title`).doubleClick();
  await s.browser.$('.task-dialog .comment-item').waitForExist({ timeout: 10_000 });
  expect(await s.browser.$('.task-dialog .comment-text').getText()).toBe('ya enviado');
  await s.browser.$('.task-dialog .resizable-dialog-close').click();
  await s.browser.$('.task-dialog').waitForExist({ reverse: true, timeout: 5_000 });
  await s.browser.$(`${pane} .add-card-btn`).click();
  await until(() => existsSync(join(s.vault, 'Equipo.board/t3.md')), 'no se creó el archivo');
  await s.browser.$(`${pane} .view-raw`).click();
  await until(
    () =>
      s.browser.execute(() => [...document.querySelectorAll('.tab-pane:not(.hidden) .board-raw .cm-line')].some((l) => l.textContent === 'format: 2')),
    'Raw no muestra el formato nuevo',
  );
}, 90_000);

it('un cambio sin guardar se convierte con lo demás y nadie vuelve a escribir la tabla', async () => {
  await s.browser.$('.file-tree-item[data-path="Rapido.board.md"]').click();
  await s.browser.$(`${pane} .card[data-task="t1"]`).waitForExist({ timeout: 15_000 });
  await s.browser.execute(() => (document.querySelector('.tab-pane:not(.hidden) .card[data-task="t1"]') as HTMLElement).focus());
  await s.browser.keys(['Shift', 'ArrowRight']);
  // Sin esperar al autoguardado
  await ask();
  await answer('confirm');
  await until(() => existsSync(join(s.vault, 'Rapido.board/t1.md')), 'no se convirtió');
  // Más que la espera del autoguardado
  await s.browser.pause(2_000);
  expect(read('Rapido.board.md')).toContain('format: 2\n');
  expect(read('Rapido.board.md')).not.toContain('| t1 |');
  expect(read('Rapido.board/t1.md')).toContain('- status: doing');
  expect(read('Rapido.board.md.bak')).toContain('| t1 | Uno | doing |');
}, 90_000);

it('con identificadores que no sirven de archivo explica por qué y no toca nada', async () => {
  await s.browser.$('.file-tree-item[data-path="Roto.board.md"]').click();
  await s.browser.$(`${pane} .board-convert button`).waitForExist({ timeout: 15_000 });
  await ask();
  await answer('confirm');
  await until(
    () => s.browser.execute(() => document.querySelector('.tab-pane:not(.hidden) .board-problem')?.textContent?.includes('con espacio') ?? false),
    'no se explicó el motivo',
  );
  expect(read('Roto.board.md')).toBe(broken);
  expect(readdirSync(s.vault).filter((f) => f.startsWith('Roto')).sort()).toEqual(['Roto.board.md']);
}, 60_000);
