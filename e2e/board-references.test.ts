// Una nota muestra un tablero y sus tareas sin abrirlo: bloques `board` en sus cuatro vistas y
// enlaces a tareas, que siguen los cambios del tablero. Lo que no existe se explica en su sitio, y un
// título con HTML se ve como texto.
import { writeFileSync } from 'node:fs';
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
  - { id: "done", label: "Hecho", color: "" }
source:
  provider: azure_devops
  scope: https://dev.azure.com/org/Demo
  synced_at: "2026-11-27T10:00:00"
---

## Order

- ado-1
- ado-2
- t1
`;

const task = (title: string, status: string, data: string[] = []) =>
  `# ${title}\n\n- status: ${status}\n${data.map((line) => `- ${line}\n`).join('')}`;
const HOSTILE = '<img src=x onerror="window.__pwned=1"> título';
const block = (lines: string) => `\`\`\`board\nboard: Equipo.board\n${lines}\n\`\`\``;

const note = `# Estado

El despliegue depende de [[Equipo.board/ado-1]] y de [[Equipo.board/t1|la limpieza]].

${block('view: summary')}

${block('filter:\n  type: [Bug]\n  assignee: ana')}

${block('view: table\ngroup_by: assignee')}

${block('view: kanban')}

Falta [[Equipo.board/ado-99]].

\`\`\`board
board: Nada.board
\`\`\`

${block('filter:\n  prioridad: alta')}

Fin de la nota.
`;

let s: Session;
beforeAll(async () => {
  s = await startApp({
    'nota.md': note,
    'Equipo.board.md': board,
    'Equipo.board/ado-1.md': task('Publicar', 'doing', [
      'type: Bug',
      'assignee: Ana',
      'key: AB#1',
      'sync: {"id":"1","title":"Publicar","state":"Active","column":"doing"}',
    ]),
    'Equipo.board/ado-2.md': task(HOSTILE, 'done', ['type: Task', 'assignee: Luis', 'key: AB#2']),
    'Equipo.board/t1.md': task('Limpiar', 'todo'),
  });
  await s.browser.$('.file-tree-item[data-path="nota.md"]').waitForExist({ timeout: 30_000 });
  await s.browser.$('.file-tree-item[data-path="nota.md"]').click();
  await s.browser.$('.markdown-body .board-ref-kanban').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(() => s?.close());

const until = (check: () => boolean | Promise<boolean>, what: string) =>
  s.browser.waitUntil(async () => check(), { timeout: 15_000, timeoutMsg: what });
/** El texto de cada bloque, con los espacios juntos. */
const blocks = () =>
  s.browser.execute(() =>
    [...document.querySelectorAll('.tab-pane:not(.hidden) .markdown-body .board-ref')].map((b) =>
      (b as HTMLElement).innerText.replace(/\s+/g, ' ').trim(),
    ),
  ) as Promise<string[]>;
const links = () =>
  s.browser.execute(() =>
    [...document.querySelectorAll('.tab-pane:not(.hidden) .markdown-body > p a.board-task-ref')].map((a) =>
      (a as HTMLElement).innerText.replace(/\s+/g, ' ').trim(),
    ),
  ) as Promise<string[]>;

it('un enlace a una tarea muestra su referencia, título y estado', async () => {
  await until(async () => (await links())[0]?.includes('Active') ?? false, 'no se pintó el enlace');
  const [first, own, missing] = await links();
  expect(first).toBe('AB#1 Publicar Active');
  expect(own).toBe('la limpieza Por hacer');
  expect(missing).toContain('No existe la tarea «ado-99»');
});

it('las cuatro vistas muestran lo que piden, con la fecha de los datos', async () => {
  await until(async () => (await blocks()).every((text) => text.length > 0), 'faltan bloques');
  const [summary, list, table, kanban] = await blocks();
  expect(summary).toContain('Datos del 2026-11-27 10:00');
  expect(summary).toContain('33 % hecho: 1 de 3');
  expect(list).toContain('AB#1');
  expect(list).toContain('Publicar');
  expect(list).not.toContain('Limpiar');
  for (const text of ['Referencia', 'Asignado a', 'Ana', 'Luis', 'Limpiar']) expect(table).toContain(text);
  expect(kanban).toContain('Por hacer (1)');
  expect(kanban).toContain('En progreso (1)');
  expect(kanban).toContain('Hecho (1)');
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/referencias.png`);
});

it('lo que no se puede mostrar se explica y el resto de la nota sigue', async () => {
  const all = await blocks();
  expect(all[4]).toContain('No existe el tablero «Nada.board»');
  expect(all[5]).toContain('Filtro desconocido: «prioridad». Se admite: state, type, assignee, key.');
  expect(await s.browser.$('.tab-pane:not(.hidden) .markdown-body').getText()).toContain('Fin de la nota.');
});

it('un título con HTML se ve como texto y no ejecuta nada', async () => {
  expect((await blocks())[2]).toContain(HOSTILE);
  expect(
    await s.browser.execute(() => [!!(window as any).__pwned, document.querySelectorAll('.board-ref img').length]),
  ).toEqual([false, 0]);
});

it('al cambiar el tablero, la nota muestra lo nuevo', async () => {
  writeFileSync(join(s.vault, 'Equipo.board/ado-1.md'), task('Publicado ya', 'done', ['type: Bug', 'assignee: Ana', 'key: AB#1']));
  await until(async () => (await links())[0] === 'AB#1 Publicado ya Hecho', 'el enlace no se actualizó');
  await until(async () => (await blocks())[0].includes('67 % hecho: 2 de 3'), 'el resumen no se actualizó');
});

const live = '.tab-pane:not(.hidden) .cm-content';
/** El texto de los bloques y de los enlaces a tareas de la vista en vivo. */
const liveBlocks = () =>
  s.browser.execute(() =>
    [...document.querySelectorAll('.tab-pane:not(.hidden) .cm-board-ref')].map((b) => (b as HTMLElement).innerText.replace(/\s+/g, ' ').trim()),
  ) as Promise<string[]>;
const liveLinks = () =>
  s.browser.execute(() =>
    [...document.querySelectorAll('.tab-pane:not(.hidden) .cm-line a.board-task-ref')].map((a) => (a as HTMLElement).innerText.replace(/\s+/g, ' ').trim()),
  ) as Promise<string[]>;

it('en la vista en vivo se ven los mismos tableros y enlaces', async () => {
  // El modo se elige en la barra de estado
  await s.browser.$('.md-view-btn').click();
  await s.browser.$('.view-item.mode-live').click();
  await until(async () => (await liveLinks())[0] === 'AB#1 Publicado ya Hecho', 'no se pintó el enlace en vivo');
  await until(async () => (await liveBlocks())[0]?.includes('67 % hecho: 2 de 3') ?? false, 'no se pintó el bloque en vivo');
  expect((await liveLinks())[1]).toBe('la limpieza Por hacer');
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/referencias-vivo.png`);
}, 60_000);

it('al entrar en un bloque se edita su texto y al salir se vuelve a ver el tablero', async () => {
  const code = () =>
    s.browser.execute(() =>
      [...document.querySelectorAll('.tab-pane:not(.hidden) .cm-line')].map((l) => l.textContent).join('\n'),
    ) as Promise<string>;
  expect(await code()).not.toContain('view: summary');
  const before = (await liveBlocks()).length;
  await s.browser.$('.tab-pane:not(.hidden) .cm-board-ref .board-ref-percent').click();
  await until(async () => (await code()).includes('view: summary'), 'no se abrió el texto del bloque');
  expect((await liveBlocks()).length).toBe(before - 1);

  // Lo que se escribe cambia lo que se ve
  await s.browser.keys(['End']);
  await s.browser.keys([' ', 'e', 'x']);
  await s.browser.keys(['Control', 'Home']);
  await s.browser.keys(['Control']);
  await until(async () => (await liveBlocks())[0]?.includes('No existe el tablero «Equipo.board ex»') ?? false, 'no cambió el bloque');
}, 90_000);

it('pulsar un enlace de un bloque abre la tarea', async () => {
  await s.browser.execute(() => {
    const key = [...document.querySelectorAll('.tab-pane:not(.hidden) .cm-board-ref a.board-ref-key')][0] as HTMLElement;
    key.click();
  });
  await until(
    () => s.browser.execute(() => !!document.querySelector('.tab-title-container.active[data-path="Equipo.board/ado-1.md"]')) as Promise<boolean>,
    'no se abrió la tarea',
  );
}, 60_000);

