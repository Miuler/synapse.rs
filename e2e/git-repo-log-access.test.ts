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

it('sobre un archivo no aparece «Git Log…» (tiene «Historial de Git»)', async () => {
  await s.browser.execute(() => {
    const el = document.querySelector('.file-tree-item[data-path="nota.md"]')!;
    const { x, y } = el.getBoundingClientRect();
    el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: x + 5, clientY: y + 5 }));
  });
  await s.browser.$('.context-menu-item').waitForExist({ timeout: 5_000 });
  expect(await s.browser.$('.context-menu-item.vault-git-log').isExisting()).toBe(false);
  await s.browser.keys('Escape');
});
