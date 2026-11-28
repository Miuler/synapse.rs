// La carpeta de un tablero se puede ver pero no cambiar: sus archivos llevan un candado que no se
// quita, y al eliminar el tablero se pregunta si su carpeta se elimina también.
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

const TASK = '# Tarea protegida\n\n- status: todo\n';
let s: Session;

beforeAll(async () => {
  s = await startApp({
    'Equipo.board.md': '---\ntype: board\nformat: 2\n---\n\n## Order\n\n- t1\n',
    'Equipo.board/t1.md': TASK,
    'nota.md': '# Nota\n',
  });
  await s.browser.$('.file-tree-item[data-path="nota.md"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(() => s?.close());

const item = (path: string) => s.browser.$(`.file-tree-item[data-path="${path}"]`);
const task = () => join(s.vault, 'Equipo.board/t1.md');
const menu = () =>
  s.browser.execute(() =>
    [...document.querySelectorAll('.context-menu-item')].map((i) => ({
      text: i.textContent?.trim() ?? '',
      disabled: i.hasAttribute('data-disabled'),
    })),
  ) as Promise<{ text: string; disabled: boolean }[]>;
const choose = (text: string) =>
  s.browser.execute(
    (text) => ([...document.querySelectorAll('.context-menu-item')].find((i) => i.textContent?.trim().startsWith(text)) as HTMLElement).click(),
    text,
  );
async function openMenu(path: string) {
  await item(path).click({ button: 'right' });
  await s.browser.$('.context-menu-item').waitForExist({ timeout: 5_000 });
}
const shot = async (name: string) => {
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/${name}.png`);
};

it('un archivo de la carpeta se abre bloqueado y no se puede desbloquear', async () => {
  await item('Equipo.board').click();
  await item('Equipo.board/t1.md').waitForExist({ timeout: 10_000 });
  await item('Equipo.board/t1.md').click();
  await s.browser.$('.tab-lock-badge').waitForExist({ timeout: 10_000 });
  await openMenu('Equipo.board/t1.md');
  await shot('candado-menu');
  const items = await menu();
  expect(items).toContainEqual({ text: 'Protegido por su tablero', disabled: true });
  expect(items.map((i) => i.text).filter((t) => /desbloquear|bloquear archivo/i.test(t))).toEqual([]);
  await s.browser.keys(['Escape']);

  // El candado normal sigue funcionando en las demás notas
  await openMenu('nota.md');
  expect(await menu()).toContainEqual({ text: 'Bloquear archivo', disabled: false });
  await s.browser.keys(['Escape']);
}, 90_000);

it('el explorador no puede eliminarlo', async () => {
  await openMenu('Equipo.board/t1.md');
  await choose('Borrar');
  await s.browser.$('.confirm-dialog-btn.confirm').waitForExist({ timeout: 10_000 });
  await s.browser.$('.confirm-dialog-btn.confirm').click();
  await s.browser.waitUntil(
    () => s.browser.execute(() => document.body.textContent?.includes('pertenece a ese tablero') ?? false) as Promise<boolean>,
    { timeout: 10_000, timeoutMsg: 'no se explicó el rechazo' },
  );
  await shot('candado-eliminar');
  expect(readFileSync(task(), 'utf8')).toBe(TASK);
  await s.browser.keys(['Escape']);
  await s.browser.$('.confirm-dialog').waitForExist({ reverse: true, timeout: 10_000 });
}, 90_000);

it('al eliminar el tablero se pregunta por su carpeta y se eliminan los dos', async () => {
  await openMenu('Equipo.board.md');
  await choose('Borrar');
  const box = s.browser.$('.confirm-dialog .board-folder input');
  await box.waitForExist({ timeout: 10_000 });
  expect(await box.isSelected()).toBe(true);
  await shot('candado-eliminar-tablero');
  await s.browser.$('.confirm-dialog-btn.confirm').click();
  await s.browser.waitUntil(() => !existsSync(join(s.vault, 'Equipo.board.md')) && !existsSync(join(s.vault, 'Equipo.board')), {
    timeout: 15_000,
    timeoutMsg: 'no se eliminaron',
  });
}, 90_000);
