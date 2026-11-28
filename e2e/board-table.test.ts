// La Tabla con muchas tareas: las filas entran por tandas y la lista de «Depende», que ofrece todas
// las demás tareas, solo se crea en el selector que se usa.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

const COUNT = 250;
const ids = Array.from({ length: COUNT }, (_, i) => `t${i + 1}`);
const files: Record<string, string> = {
  'Grande.board.md': `---\ntype: board\nformat: 2\ndefault_view: table\ncolumns:\n  - { id: "todo", label: "Por hacer", color: "" }\n---\n\n## Order\n\n${ids.map((id) => `- ${id}`).join('\n')}\n`,
};
for (const [i, id] of ids.entries()) {
  files[`Grande.board/${id}.md`] = `# Tarea ${i + 1}\n\n- status: todo\n- depends: ${i ? ids[i - 1] : '-'}\n`;
}

let s: Session;
beforeAll(async () => {
  s = await startApp(files);
  await s.browser.$('.file-tree-item[data-path="Grande.board.md"]').waitForExist({ timeout: 30_000 });
  await s.browser.$('.file-tree-item[data-path="Grande.board.md"]').click();
  await s.browser.$('.table-view tbody tr').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(() => s?.close());

const until = (check: () => boolean | Promise<boolean>, what: string) =>
  s.browser.waitUntil(async () => check(), { timeout: 15_000, timeoutMsg: what });
const table = () =>
  s.browser.execute(() => {
    const rows = [...document.querySelectorAll('.tab-pane:not(.hidden) .table-view tbody tr')];
    const selects = rows.map((row) => row.querySelector('select.depends') as HTMLSelectElement);
    return {
      rows: rows.length,
      options: selects.reduce((sum, select) => sum + select.options.length, 0),
      values: [selects[0]?.value, selects[1]?.value, selects.at(-1)?.value],
    };
  }) as Promise<{ rows: number; options: number; values: string[] }>;

it('muestra todas las filas, cada una con su dependencia, sin crear todas las opciones', async () => {
  await until(async () => (await table()).rows === COUNT, 'no entraron todas las filas');
  const { options, values } = await table();
  expect(values).toEqual(['-', 't1', `t${COUNT - 1}`]);
  // Cada selector: «-» y, si depende de otra, esa
  expect(options).toBe(COUNT * 2 - 1);
});

it('al usar un selector ofrece las demás tareas y guarda la elegida', async () => {
  const offered = await s.browser.execute(() => {
    const select = document.querySelector('.tab-pane:not(.hidden) .table-view tbody tr select.depends') as HTMLSelectElement;
    select.focus();
    return { count: select.options.length, value: select.value };
  });
  expect(offered).toEqual({ count: COUNT, value: '-' });
  await s.browser.execute(() => {
    const select = document.querySelector('.tab-pane:not(.hidden) .table-view tbody tr select.depends') as HTMLSelectElement;
    select.value = 't200';
    select.dispatchEvent(new Event('change', { bubbles: true }));
  });
  await until(() => readFileSync(join(s.vault, 'Grande.board/t1.md'), 'utf8').includes('- depends: t200\n'), 'no se guardó');
  // Otro selector toma la lista y el anterior conserva lo elegido
  await s.browser.execute(() =>
    (document.querySelectorAll('.tab-pane:not(.hidden) .table-view tbody tr select.depends')[1] as HTMLSelectElement).focus(),
  );
  await until(async () => (await table()).values[0] === 't200', 'se perdió lo elegido');
  expect((await table()).options).toBeLessThan(COUNT * 3 + 5);
});

it('ordenar mantiene todas las filas', async () => {
  await s.browser.$('.tab-pane:not(.hidden) .sort-btn[data-sort="title"]').click();
  await s.browser.$('.tab-pane:not(.hidden) .sort-btn[data-sort="title"]').click();
  await until(
    () => s.browser.execute(() => document.querySelector('.tab-pane:not(.hidden) .table-view tbody tr code')?.textContent === 't250') as Promise<boolean>,
    'no se ordenó',
  );
  expect((await table()).rows).toBe(COUNT);
});
