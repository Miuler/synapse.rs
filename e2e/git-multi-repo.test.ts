// Varios repositorios Git en una bóveda: la rama junto a la carpeta-repositorio, el estado de sus
// archivos, el diálogo de ramas de ese repositorio y un commit que va a ese repositorio y no a la raíz.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;
let vault: string;

const git = (cwd: string, ...args: string[]) =>
  execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', ...args], { cwd }).toString();

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-multirepo-'));
  vault = join(base, 'vault');
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  mkdirSync(join(vault, 'proyecto'), { recursive: true });
  writeFileSync(join(vault, 'raiz.md'), '# Raíz\n');
  writeFileSync(join(vault, 'proyecto', 'nota.md'), '# Nota\n');
  // La bóveda es un repositorio (main) y `proyecto/` otro, en la rama feature-x
  git(vault, 'init', '-q', '-b', 'main');
  writeFileSync(join(vault, '.gitignore'), 'proyecto/\n');
  git(vault, 'add', '.');
  git(vault, 'commit', '-qm', 'raíz');
  git(join(vault, 'proyecto'), 'init', '-q', '-b', 'feature-x');
  git(join(vault, 'proyecto'), 'add', '.');
  git(join(vault, 'proyecto'), 'commit', '-qm', 'inicio proyecto');
  // La app corre con un HOME temporal: la identidad va en el propio repositorio
  git(join(vault, 'proyecto'), 'config', 'user.name', 'e2e');
  git(join(vault, 'proyecto'), 'config', 'user.email', 'e2e@local');
  writeFileSync(join(vault, 'proyecto', 'nota.md'), '# Nota cambiada\n');

  s = await startAppOnVault(vault);
  await s.browser.$('.file-tree-item[data-path="proyecto"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

const row = (path: string) => s.browser.$(`.file-tree-item[data-path="${path}"]`);

it('la carpeta-repositorio muestra su rama y que tiene cambios', async () => {
  const badge = row('proyecto').$('.folder-git-branch');
  await badge.waitForExist({ timeout: 15_000, timeoutMsg: 'no apareció la rama junto a la carpeta' });
  expect(await badge.getProperty('textContent')).toContain('feature-x');
  expect(await badge.getAttribute('class')).toContain('dirty');
  // La raíz no lleva insignia en el árbol
  expect(await row('raiz.md').$('.folder-git-branch').isExisting()).toBe(false);
}, 60_000);

it('el archivo modificado del repositorio anidado tiene estado de git', async () => {
  await row('proyecto').click();
  await row('proyecto/nota.md').waitForExist({ timeout: 10_000 });
  await s.browser.waitUntil(async () => row('proyecto/nota.md').$('.vault-explorer-git-badges').isExisting(), {
    timeout: 10_000,
    timeoutMsg: 'el archivo no muestra estado de git',
  });
}, 60_000);

it('al abrir un archivo del repositorio, la cabecera muestra su rama', async () => {
  await row('proyecto/nota.md').click();
  await s.browser.waitUntil(async () => (await s.browser.execute(() => document.querySelector('.git-branch-badge .git-branch-name')?.textContent ?? '')) === 'feature-x', {
    timeout: 10_000,
    timeoutMsg: 'la cabecera no muestra la rama del repositorio del archivo activo',
  });
  await row('raiz.md').click();
  await s.browser.waitUntil(async () => (await s.browser.execute(() => document.querySelector('.git-branch-badge .git-branch-name')?.textContent ?? '')) === 'main', { timeout: 10_000 });
}, 60_000);

it('el diálogo de ramas abierto desde la carpeta es el de ese repositorio', async () => {
  await row('proyecto').$('.folder-git-branch').click();
  const dialog = s.browser.$('.git-branch-dialog');
  await dialog.waitForExist({ timeout: 10_000 });
  await s.browser.waitUntil(async () => ((await dialog.getProperty('textContent')) as string).includes('feature-x'), { timeout: 10_000 });
  expect(await dialog.$('.git-repo-select select').getValue()).toBe('proyecto');
  await s.browser.keys(['Escape']);
  await dialog.waitForExist({ reverse: true, timeout: 5_000 });
}, 60_000);

it('el commit desde el menú de la carpeta va a ese repositorio, no a la raíz', async () => {
  await row('proyecto').click({ button: 'right' });
  const item = s.browser.$('.context-menu-item*=Commit en');
  await item.waitForExist({ timeout: 5_000 });
  await item.click();
  const dialog = s.browser.$('.git-commit-dialog');
  await dialog.waitForExist({ timeout: 10_000 });
  const dialogText = () => dialog.getProperty('textContent') as Promise<string>;
  await s.browser.waitUntil(async () => (await dialogText()).includes('nota.md'), { timeout: 10_000, timeoutMsg: 'el diálogo no lista nota.md' });
  expect(await dialogText()).not.toContain('raiz.md');
  await dialog.$('.git-commit-textarea').setValue('cambio en proyecto');
  await dialog.$('.git-commit-btn.submit').click();
  await dialog.waitForExist({ reverse: true, timeout: 15_000 }).catch(async (e) => {
    throw new Error(`${e.message}: ${await dialogText()}`);
  });

  expect(git(join(vault, 'proyecto'), 'log', '-1', '--format=%s').trim()).toBe('cambio en proyecto');
  expect(git(vault, 'log', '-1', '--format=%s').trim()).toBe('raíz');
  await s.browser.waitUntil(async () => !((await row('proyecto').$('.folder-git-branch').getAttribute('class')) ?? '').includes('dirty'), {
    timeout: 10_000,
    timeoutMsg: 'la carpeta sigue marcada con cambios tras el commit',
  });
}, 60_000);
