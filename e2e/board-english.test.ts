// Con la interfaz en inglés el editor de tableros no muestra textos en español: se revisan lo visible
// y los atributos que se leen (título, etiqueta accesible, marcador) en cada vista.
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

// La cabecera de la tabla es parte del formato del archivo: no se ve en ninguna vista salvo «Raw»
const table = `---
type: board
default_view: table
columns:
  - { id: todo, label: "To do", color: "var(--text-muted)" }
  - { id: done, label: "Done", color: "#22c55e" }
---

| id | Tarea | Estado | Inicio | Duración | Depende | Progreso |
|---|---|---|---|---|---|---|
| t1 | First | todo | 2026-01-01 | 1d | - | 0% |
| t2 | Second | done | 2026-01-02 | 2d | t1 | 50% |
`;

let s: Session;
beforeAll(async () => {
  s = await startApp({
    'team.board.md': table,
    // Las notas muestran tableros de carpeta: uno de tabla solo diría que hay que convertirlo
    'items.board.md': '---\ntype: board\nformat: 2\ncolumns:\n  - { id: "todo", label: "To do", color: "" }\n---\n\n## Order\n\n- t1\n',
    'items.board/t1.md': '# Only\n\n- status: todo\n',
    // Dos tareas que en otros sistemas serían el mismo archivo: Rust avisa de la segunda
    'twins.board.md': '---\ntype: board\nformat: 2\ncolumns:\n  - { id: "todo", label: "To do", color: "" }\n---\n',
    'twins.board/T1.md': '# Upper\n\n- status: todo\n',
    'twins.board/t1.md': '# Lower\n\n- status: todo\n',
    'plain.md': `# Plain\n\n${['table', 'summary', 'kanban'].map((v) => `\`\`\`board\nboard: items.board\nview: ${v}\n\`\`\``).join('\n\n')}\n\n\`\`\`board\nboard: missing.board\n\`\`\`\n`,
  });
  await s.browser.$('.file-tree-item[data-path="plain.md"]').waitForExist({ timeout: 30_000 });
  await s.browser.keys(['Control', 'p']);
  const palette = await s.browser.$('.palette-container input');
  await palette.waitForExist({ timeout: 5_000 });
  await palette.setValue('switch language');
  await s.browser.waitUntil(
    async () => (await s.browser.$('.command-item[data-selected] .command-name').getText()).startsWith('Cambiar idioma'),
    { timeout: 5_000 },
  );
  await s.browser.keys('Enter');
  await s.browser.$('.palette-container').waitForExist({ reverse: true, timeout: 5_000 });
  // La nota se abre ya en inglés: lo pintado antes de cambiar de idioma no se vuelve a pintar
  await s.browser.$('.file-tree-item[data-path="plain.md"]').click();
}, 120_000);
afterAll(() => s?.close());

const pane = '.tab-pane:not(.hidden)';
const until = (check: () => boolean | Promise<boolean>, what: string) =>
  s.browser.waitUntil(async () => check(), { timeout: 15_000, timeoutMsg: what });

/** Lo que de `selector` se ve o se lee en español. */
const spanish = (selector: string) =>
  s.browser.execute((selector) => {
    const found = [...document.querySelectorAll<HTMLElement>(selector)];
    if (!found.length) return [`no existe ${selector}`];
    const texts = found.flatMap((root) => [
      ...root.innerText.split('\n'),
      ...[root, ...root.querySelectorAll('*')].flatMap((el) =>
        ['title', 'aria-label', 'placeholder'].map((name) => el.getAttribute(name) ?? ''),
      ),
    ]);
    const words = /[áéíóúñ¿¡]|\b(Tabla|Tarea|Tareas|Estado|Inicio|Depende|Progreso|Eliminar|Comentario|Comentarios|Editar|Cerrar|Guardar|Cancelar|Traer|Enviar|Columna|Nuevo|Nueva|Por hacer|Hecho|Sin|Ver|Actualizar|Tipo|Asignado|Conexión)\b/i;
    return [...new Set(texts.filter((text) => words.test(text)))];
  }, selector) as Promise<string[]>;

const view = (index: number) =>
  s.browser.execute((i) => (document.querySelectorAll('.tab-pane:not(.hidden) .view-switcher button')[i] as HTMLElement).click(), index);

it('los bloques de tablero en una nota', async () => {
  await s.browser.$('.markdown-body .board-ref-table').waitForExist({ timeout: 30_000 });
  await s.browser.$('.markdown-body .board-ref-summary').waitForExist({ timeout: 10_000 });
  await s.browser.$('.markdown-body .board-ref-kanban').waitForExist({ timeout: 10_000 });
  await s.browser.$('.markdown-body .board-ref-error').waitForExist({ timeout: 10_000 });
  expect(await spanish('.markdown-body [class*="board-ref"]')).toEqual([]);
}, 60_000);

it('tabla, Kanban y Gantt, con una tarea abierta', async () => {
  await s.browser.$('.file-tree-item[data-path="team.board.md"]').click();
  await s.browser.$(`${pane} .board-editor table`).waitForExist({ timeout: 15_000 });
  expect(await s.browser.$(`${pane} .board-editor thead`).getText()).toContain('Task');
  expect(await spanish(`${pane} .board-editor`)).toEqual([]);

  await view(1);
  await s.browser.$(`${pane} .kanban-view .card`).waitForExist({ timeout: 10_000 });
  expect(await spanish(`${pane} .board-editor`)).toEqual([]);

  await s.browser.$(`${pane} .card .comment-btn`).click();
  await s.browser.$('.task-dialog').waitForExist({ timeout: 10_000 });
  expect(await spanish('.task-dialog')).toEqual([]);
  await s.browser.keys(['Escape']);
  await s.browser.$('.task-dialog').waitForExist({ reverse: true, timeout: 5_000 });

  await view(2);
  await s.browser.$(`${pane} .gantt-view`).waitForExist({ timeout: 10_000 });
  expect(await spanish(`${pane} .board-editor`)).toEqual([]);
}, 90_000);

it('la configuración del origen', async () => {
  await s.browser.$(`${pane} .issues-settings`).click();
  await s.browser.$('.source-panel').waitForExist({ timeout: 15_000 });
  expect(await spanish('.source-panel')).toEqual([]);
  await s.browser.$('.source-panel input[name="mode"][value="criteria"]').click();
  expect(await spanish('.source-panel')).toEqual([]);
  await s.browser.$('.source-panel button.cancel').click();
  await s.browser.$('.source-panel').waitForExist({ reverse: true, timeout: 5_000 });
}, 60_000);

it('un tablero y una tarea nuevos nacen con nombres en inglés', async () => {
  await s.browser.$('.file-tree-item[data-path="plain.md"]').click({ button: 'right' });
  await s.browser.$('.context-menu-item').waitForExist({ timeout: 5_000 });
  await s.browser.execute(() =>
    ([...document.querySelectorAll('.context-menu-item')].find((i) => i.textContent?.includes('New board')) as HTMLElement).click(),
  );
  const created = join(s.vault, 'Board.board.md');
  await until(() => existsSync(created), 'no se creó el tablero');
  expect(readFileSync(created, 'utf8')).toContain('label: "To do"');
  await until(async () => (await s.browser.$(`${pane} .kanban-view`).isExisting()), 'no se abrió');
  // Al crearlo puede preguntar de dónde salen las tareas
  if (await s.browser.$('.source-panel').isExisting()) await s.browser.$('.source-panel button.cancel').click();
  await s.browser.execute(() => (document.querySelector('.tab-pane:not(.hidden) .add-card-btn') as HTMLElement).click());
  await until(() => existsSync(join(s.vault, 'Board.board/t1.md')), 'no se creó la tarea');
  expect(readFileSync(join(s.vault, 'Board.board/t1.md'), 'utf8')).toContain('# New task\n');
  expect(await spanish(`${pane} .board-editor`)).toEqual([]);
}, 90_000);

it('un tablero de carpeta, el detalle de su tarea y el aviso de convertir', async () => {
  await s.browser.$('.file-tree-item[data-path="items.board.md"]').click();
  await s.browser.$(`${pane} .card[data-task="t1"]`).waitForExist({ timeout: 15_000 });
  await s.browser.$(`${pane} .card[data-task="t1"] .comment-btn`).click();
  await s.browser.$('.task-dialog').waitForExist({ timeout: 10_000 });
  expect(await s.browser.$('.task-dialog').getText()).toContain('Description');
  expect(await spanish('.task-dialog')).toEqual([]);
  await s.browser.$('.task-dialog .resizable-dialog-close').click();
  await s.browser.$('.task-dialog').waitForExist({ reverse: true, timeout: 5_000 });
  await view(0);
  await s.browser.$(`${pane} .board-editor table`).waitForExist({ timeout: 10_000 });
  expect(await spanish(`${pane} .board-editor`)).toEqual([]);

  await s.browser.$('.file-tree-item[data-path="team.board.md"]').click();
  await s.browser.$(`${pane} .board-convert button`).click();
  await s.browser.$('.confirm-dialog').waitForExist({ timeout: 10_000 });
  expect(await spanish('.confirm-dialog')).toEqual([]);
  await s.browser.$('.confirm-dialog-btn.cancel').click();
  await s.browser.$('.confirm-dialog').waitForExist({ reverse: true, timeout: 5_000 });
}, 90_000);

it('el motivo que escribe Rust sale en inglés', async () => {
  await s.browser.$('.file-tree-item[data-path="twins.board.md"]').click();
  await until(
    async () => (await s.browser.$(`${pane} .board-editor`).getText()).includes('except for letter case'),
    'no salió el aviso de la tarea repetida',
  );
  expect(await spanish(`${pane} .board-editor`)).toEqual([]);
}, 60_000);

it('el rechazo del explorador a tocar un archivo del tablero sale en inglés', async () => {
  const task = '.file-tree-item[data-path="items.board/t1.md"]';
  await s.browser.$('.file-tree-item[data-path="items.board"]').click();
  await s.browser.$(task).waitForExist({ timeout: 10_000 });
  await s.browser.$(task).click({ button: 'right' });
  await s.browser.$('.context-menu-item').waitForExist({ timeout: 5_000 });
  // El menú del explorador todavía está en español
  await s.browser.execute(() =>
    ([...document.querySelectorAll('.context-menu-item')].find((i) => i.textContent?.trim().startsWith('Borrar')) as HTMLElement).click(),
  );
  await s.browser.$('.confirm-dialog-btn.confirm').waitForExist({ timeout: 10_000 });
  await s.browser.$('.confirm-dialog-btn.confirm').click();
  await s.browser.$('.vault-toast').waitForExist({ timeout: 10_000 });
  expect(await s.browser.$('.vault-toast').getText()).toBe(
    'Could not delete: t1.md (items.board.md): belongs to that board and is changed from it',
  );
  expect(existsSync(join(s.vault, 'items.board/t1.md'))).toBe(true);
  await s.browser.keys(['Escape']);
  await s.browser.$('.confirm-dialog').waitForExist({ reverse: true, timeout: 10_000 });
}, 90_000);

it('la configuración de conexiones', async () => {
  await s.browser.$('[aria-label="Settings"]').click();
  await s.browser.$('.settings-nav-item').waitForExist({ timeout: 15_000 });
  await s.browser.execute(() =>
    ([...document.querySelectorAll('.settings-nav-item')].find((b) => b.textContent?.trim() === 'Connections') as HTMLElement).click(),
  );
  await s.browser.$('.connections').waitForExist({ timeout: 15_000 });
  expect(await spanish('.connections')).toEqual([]);
  await s.browser.$('.connections .add').click();
  await s.browser.$('.connections form').waitForExist({ timeout: 5_000 });
  expect(await spanish('.connections')).toEqual([]);
}, 60_000);
