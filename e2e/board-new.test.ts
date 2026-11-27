// Un tablero se crea desde el explorador (o la paleta), se abre en su editor y guarda sus cambios.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

let s: Session;

beforeAll(async () => {
  s = await startApp({ 'nota.md': '# Nota\n' });
  await s.browser.$('.file-tree-item[data-path="nota.md"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(() => s?.close());

const cards = () =>
  s.browser.execute(() => [...document.querySelectorAll('.tab-pane:not(.hidden) .kanban-view .card-title')].map((e) => e.textContent?.trim())) as Promise<string[]>;

it('«Nuevo tablero» del explorador crea un .board.md con su Kanban y guarda lo que se añade', async () => {
  await s.browser.$('.file-tree-item[data-path="nota.md"]').click({ button: 'right' });
  await s.browser.$('.context-menu-item').waitForExist({ timeout: 5_000 });
  await s.browser.execute(() =>
    ([...document.querySelectorAll('.context-menu-item')].find((i) => i.textContent?.includes('Nuevo tablero')) as HTMLElement).click(),
  );
  await s.browser.$('.tab-pane:not(.hidden) .kanban-view').waitForExist({ timeout: 15_000 });
  // Nace en el formato de carpeta, sin tareas y sin avisos de error ni de conversión
  expect(await cards()).toEqual([]);
  const file = () => readFileSync(join(s.vault, 'Tablero.board.md'), 'utf8');
  expect(file()).toContain('format: 2\n');
  expect(await s.browser.$('.tab-pane:not(.hidden) .board-problem').isExisting()).toBe(false);
  expect(await s.browser.$('.tab-pane:not(.hidden) .board-convert').isExisting()).toBe(false);

  await s.browser.$('.tab-pane:not(.hidden) .add-card-btn').click();
  await s.browser.waitUntil(async () => (await cards()).includes('Nueva tarea'), { timeout: 5_000 });
  await s.browser.waitUntil(async () => file().includes('## Order\n\n- t1\n'), { timeout: 10_000, timeoutMsg: 'no se guardó la tarea' });
  expect(readFileSync(join(s.vault, 'Tablero.board/t1.md'), 'utf8')).toContain('# Nueva tarea\n\n- status: todo\n');
}, 90_000);

it('la vista Raw muestra el archivo tal como se guarda, sin dejar editarlo', async () => {
  const file = () => readFileSync(join(s.vault, 'Tablero.board.md'), 'utf8');
  const shown = () =>
    s.browser.execute(() => [...document.querySelectorAll('.tab-pane:not(.hidden) .board-raw .cm-line')].map((l) => l.textContent).join('\n')) as Promise<string>;
  const saved = file();
  await s.browser.$('.tab-pane:not(.hidden) .view-raw').click();
  await s.browser.waitUntil(async () => (await shown()).includes('- t1'), { timeout: 10_000, timeoutMsg: 'no se ve el archivo' });
  expect((await shown()).trimEnd()).toBe(saved.trimEnd());
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/board-raw.png`);
  await s.browser.$('.tab-pane:not(.hidden) .board-raw .cm-content').click();
  await s.browser.keys(['x', 'y', 'z']);
  await s.browser.pause(500);
  expect(await shown()).not.toContain('xyz');
  expect(file()).toBe(saved);
  // Elegirla no cambia la vista con la que abre el tablero
  expect(file()).toContain('default_view: kanban');
  await s.browser.$('.tab-pane:not(.hidden) .view-switcher button:nth-child(2)').click();
  await s.browser.$('.tab-pane:not(.hidden) .kanban-view').waitForExist({ timeout: 5_000 });
}, 90_000);

it('también se crea desde la paleta de comandos', async () => {
  await s.browser.$('.tab-pane:not(.hidden) .view-switcher').click();
  await s.browser.keys(['Control', 'p']);
  const input = await s.browser.$('.synapse-command-palette input');
  await input.waitForExist({ timeout: 15_000 });
  await input.setValue('nuevo tablero');
  await s.browser.waitUntil(
    async () => (await s.browser.execute(() => document.querySelector('.synapse-command-palette .command-item')?.textContent ?? '') as string).includes('tablero'),
    { timeout: 15_000, timeoutMsg: 'la paleta no ofrece el tablero' },
  );
  await s.browser.keys(['Enter']);
  await s.browser.waitUntil(
    () =>
      s.browser.execute(() => !!document.querySelector('.tab-title-container.active[data-path$=".board.md"]:not([data-path="Tablero.board.md"])') && !!document.querySelector('.tab-pane:not(.hidden) .kanban-view')) as Promise<boolean>,
    { timeout: 15_000, timeoutMsg: 'no se abrió el tablero nuevo' },
  );
  // La paleta abre la pestaña por otro camino: el archivo ya tiene que existir al leerlo
  await s.browser.pause(1_500);
  expect(await s.browser.$('.tab-pane:not(.hidden) .board-problem').isExisting()).toBe(false);
  await s.browser.$('.tab-pane:not(.hidden) .add-card-btn').click();
  await s.browser.waitUntil(async () => (await cards()).includes('Nueva tarea'), { timeout: 5_000 });
}, 90_000);
