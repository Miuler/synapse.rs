// Historial de Git de un archivo que se renombró: «Ver archivo» de un commit anterior al renombre
// muestra su contenido (antes decía «El archivo no existía en la revisión»).
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-rename-'));
  const vault = join(base, 'vault');
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  mkdirSync(join(vault, 'components'), { recursive: true });
  const git = (...args: string[]) =>
    execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', ...args], { cwd: vault });
  git('init', '-q');
  writeFileSync(join(vault, 'components', 'diagrama.md'), '# Diagrama\n\nversión antigua\n');
  git('add', '.');
  git('commit', '-qm', 'agrega el diagrama');
  mkdirSync(join(vault, 'src'), { recursive: true });
  git('mv', 'components', 'src/components');
  git('commit', '-qm', 'mueve a src');
  writeFileSync(join(vault, 'src', 'components', 'diagrama.md'), '# Diagrama\n\nversión nueva\n');
  git('commit', '-qam', 'actualiza');
  s = await startAppOnVault(vault);
  await s.browser.$('.file-tree-item[data-path="src"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

it('«Ver archivo» de un commit anterior al renombre muestra el contenido de entonces', async () => {
  await s.browser.$('.file-tree-item[data-path="src"]').click();
  await s.browser.$('.file-tree-item[data-path="src/components"]').waitForExist({ timeout: 5_000 });
  await s.browser.$('.file-tree-item[data-path="src/components"]').click();
  await s.browser.$('.file-tree-item[data-path="src/components/diagrama.md"]').waitForExist({ timeout: 5_000 });
  await s.browser.execute(() => {
    const el = document.querySelector('.file-tree-item[data-path="src/components/diagrama.md"]')!;
    const { x, y } = el.getBoundingClientRect();
    el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: x + 5, clientY: y + 5 }));
  });
  await s.browser.$('.context-menu-item').waitForExist({ timeout: 5_000 });
  await s.browser.execute(() => {
    ([...document.querySelectorAll('.context-menu-item')].find((i) => i.textContent?.includes('Historial de Git')) as HTMLElement).click();
  });
  // El historial sigue el renombre: tres commits
  await s.browser.waitUntil(async () => (await s.browser.$$('.git-log-dialog .git-log-row').length) === 3, {
    timeout: 10_000,
    timeoutMsg: 'el historial no siguió el renombre',
  });
  // El más antiguo (último de la lista) es de cuando estaba en components/
  const rows = await s.browser.$$('.git-log-dialog .git-log-row');
  await rows[2].$('.view-btn').click();
  await s.browser.waitUntil(
    async () => ((await s.browser.execute(() => document.querySelector('.tab-pane:not(.hidden)')?.textContent ?? '')) as string).includes('versión antigua'),
    { timeout: 10_000, timeoutMsg: 'no se mostró el contenido del commit anterior al renombre' },
  );
  const body = (await s.browser.execute(() => document.querySelector('.tab-pane:not(.hidden)')?.textContent ?? '')) as string;
  expect(body).not.toContain('no existía');
});

it('la pestaña de una versión se puede mover a una ventana nueva y allí muestra la misma versión', async () => {
  const revTab = (await s.browser.execute(() =>
    [...document.querySelectorAll('.tab-title-container')].map((t) => t.getAttribute('data-path')).find((p) => p?.startsWith('rev:')),
  )) as string;
  expect(revTab).toBeTruthy();
  await s.browser.execute((p: string) => {
    const el = document.querySelector(`.tab-title-container[data-path="${p}"]`)!;
    const { x, y } = el.getBoundingClientRect();
    el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: x + 5, clientY: y + 5 }));
  }, revTab);
  await s.browser.$('.context-menu-item').waitForExist({ timeout: 5_000 });
  const labels = (await s.browser.execute(() =>
    [...document.querySelectorAll('.context-menu-item')].map((i) => i.textContent?.trim()),
  )) as string[];
  expect(labels).toContain('Abrir en nueva ventana');
  expect(labels).toContain('Mover a nueva ventana');
  const main = await s.browser.getWindowHandle();
  await s.browser.execute(() => {
    ([...document.querySelectorAll('.context-menu-item')].find((i) => i.textContent?.trim() === 'Mover a nueva ventana') as HTMLElement).click();
  });
  // Se cierra aquí y aparece la ventana nueva con la misma versión
  await s.browser.waitUntil(async () => (await s.browser.getWindowHandles()).length === 2, {
    timeout: 15_000,
    timeoutMsg: 'no se abrió la ventana nueva',
  });
  expect(await s.browser.$(`.tab-title-container[data-path="${revTab}"]`).isExisting()).toBe(false);
  const other = (await s.browser.getWindowHandles()).find((h) => h !== main)!;
  await s.browser.switchToWindow(other);
  await s.browser.waitUntil(
    async () => ((await s.browser.execute(() => document.querySelector('.tab-pane:not(.hidden)')?.textContent ?? '')) as string).includes('versión antigua'),
    { timeout: 20_000, timeoutMsg: 'la ventana nueva no mostró la versión' },
  );
  await s.browser.closeWindow();
  await s.browser.switchToWindow(main);
}, 60_000);
