// Vault que es un único repositorio: «Git Log…» en el clic derecho del espacio vacío del explorador,
// del nombre del vault y del badge de la rama.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-repo-log-access-'));
  const vault = join(base, 'vault');
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  writeFileSync(join(vault, '.gitignore'), '.synapse/\n');
  writeFileSync(join(vault, 'nota.md'), '# Nota\n');
  const git = (...args: string[]) =>
    execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', ...args], { cwd: vault });
  git('init', '-q', '-b', 'main');
  git('add', '.');
  git('commit', '-qm', 'commit inicial del vault');
  s = await startAppOnVault(vault);
  await s.browser.$('.git-branch-badge').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

for (const [where, selector] of [
  ['el espacio vacío del explorador', '.sidebar-content'],
  ['el nombre del vault', '.rust-badge-btn'],
  ['el badge de la rama', '.git-branch-badge'],
] as const) {
  it(`clic derecho en ${where} ofrece «Git Log…» y abre el log del repositorio`, async () => {
    await s.browser.execute((sel: string) => {
      const el = document.querySelector(sel)!;
      const r = el.getBoundingClientRect();
      // En el espacio vacío: abajo del todo, lejos de los archivos
      const y = sel === '.sidebar-content' ? r.bottom - 10 : r.top + r.height / 2;
      el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: r.left + 10, clientY: y }));
    }, selector);
    await s.browser.$('.context-menu-item.vault-git-log').waitForExist({ timeout: 5_000, timeoutMsg: `sin «Git Log…» en ${where}` });
    const labels = (await s.browser.execute(() =>
      [...document.querySelectorAll('.context-menu-item')].map((i) => i.textContent?.trim()),
    )) as string[];
    expect(labels).toContain('Ramas…');
    await s.browser.execute(() => (document.querySelector('.context-menu-item.vault-git-log') as HTMLElement).click());
    await s.browser.$('.git-repo-log-dialog').waitForExist({ timeout: 5_000 });
    await s.browser.waitUntil(
      async () => ((await s.browser.execute(() => document.querySelector('.git-repo-log-dialog')?.textContent ?? '')) as string).includes('commit inicial del vault'),
      { timeout: 10_000, timeoutMsg: 'el log no muestra el commit' },
    );
    await s.browser.keys('Escape');
    await s.browser.$('.git-repo-log-dialog').waitForExist({ timeout: 5_000, reverse: true });
  }, 30_000);
}

it('sobre un archivo no aparece «Git Log…» y las opciones de Git salen juntas arriba', async () => {
  await s.browser.execute(() => {
    const el = document.querySelector('.file-tree-item[data-path="nota.md"]')!;
    const { x, y } = el.getBoundingClientRect();
    el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: x + 5, clientY: y + 5 }));
  });
  await s.browser.$('.context-menu-item').waitForExist({ timeout: 5_000 });
  expect(await s.browser.$('.context-menu-item.vault-git-log').isExisting()).toBe(false);

  const labels = (await s.browser.execute(() =>
    [...document.querySelectorAll('.vault-context-menu .context-menu-item')].map((i) => i.textContent?.trim() ?? ''),
  )) as string[];

  const diffIdx = labels.findIndex((l) => l === 'Diff');
  const diffStagedIdx = labels.findIndex((l) => l === 'Diff --staged');
  const historyIdx = labels.findIndex((l) => l.includes('Historial de Git'));
  const openWinIdx = labels.findIndex((l) => l.includes('Abrir en nueva ventana'));
  const newFolderIdx = labels.findIndex((l) => l === 'Nueva carpeta');

  expect(diffIdx).toBeGreaterThan(0);
  expect(diffStagedIdx).toBe(diffIdx + 1);
  expect(historyIdx).toBe(diffStagedIdx + 1);
  expect(historyIdx).toBeLessThan(openWinIdx);
  expect(openWinIdx).toBeLessThan(newFolderIdx);
  await s.browser.keys('Escape');
});

it('en la raíz con cambios sin commitear las opciones de Git aparecen juntas arriba', async () => {
  writeFileSync(join(base, 'vault', 'nota.md'), '# Nota\n\ncambio sin commit\n');
  await s.browser.$('.sidebar-content').waitForExist({ timeout: 5_000 });
  // Refrescar estado Git vía Ctrl+R desde el menú del espacio vacío
  await s.browser.execute(() => {
    const el = document.querySelector('.sidebar-content')!;
    const r = el.getBoundingClientRect();
    el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: r.left + 10, clientY: r.bottom - 10 }));
  });
  await s.browser.$('.context-menu-item').waitForExist({ timeout: 5_000 });
  await s.browser.execute(() => {
    ([...document.querySelectorAll('.context-menu-item')].find((i) => i.textContent?.includes('Recargar')) as HTMLElement).click();
  });
  await Bun.sleep(400);

  await s.browser.execute(() => {
    const el = document.querySelector('.sidebar-content')!;
    const r = el.getBoundingClientRect();
    el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: r.left + 10, clientY: r.bottom - 10 }));
  });
  await s.browser.$('.context-menu-item.vault-git-log').waitForExist({ timeout: 5_000 });

  const labels = (await s.browser.execute(() =>
    [...document.querySelectorAll('.vault-context-menu .context-menu-item')].map((i) => i.textContent?.trim() ?? ''),
  )) as string[];

  const logIdx = labels.findIndex((l) => l.includes('Git Log'));
  const branchesIdx = labels.findIndex((l) => l.includes('Ramas'));
  const commitIdx = labels.findIndex((l) => l.includes('Hacer commit'));
  const newFolderIdx = labels.findIndex((l) => l === 'Nueva carpeta');

  expect(logIdx).toBeGreaterThan(0);
  expect(branchesIdx).toBe(logIdx + 1);
  expect(commitIdx).toBe(branchesIdx + 1);
  expect(commitIdx).toBeLessThan(newFolderIdx);
  await s.browser.keys('Escape');
});
