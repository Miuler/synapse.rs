// Explorador con Git: expandir/contraer todo (con y sin filtro), archivos borrados tachados y el
// filtro de Git mostrando todo lo que entraría en un commit, aunque no sea un tipo de archivo soportado.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;

const files: Record<string, string> = {
  'a/uno.md': '# Uno\n',
  'a/sub/dos.md': '# Dos\n',
  'b/tres.md': '# Tres\n',
  'c/cuatro.md': '# Cuatro\n',
};

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-git-'));
  const vault = join(base, 'vault');
  for (const [p, content] of Object.entries(files)) {
    mkdirSync(dirname(join(vault, p)), { recursive: true });
    writeFileSync(join(vault, p), content);
  }
  // Con .synapse la app abre la bóveda sin preguntar
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  const git = (...args: string[]) =>
    execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', ...args], { cwd: vault });
  git('init', '-q');
  git('add', '.');
  git('commit', '-qm', 'inicio');
  rmSync(join(vault, 'b/tres.md'));
  writeFileSync(join(vault, 'a/datos.bin'), 'binario');
  writeFileSync(join(vault, 'a/sub/dos.md'), '# Dos cambiado\n');

  s = await startAppOnVault(vault);
  await s.browser.$('.file-tree-item[data-path="c"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

const row = (path: string) => s.browser.$(`.file-tree-item[data-path="${path}"]`);
const expanded = async (path: string) => (await row(path).getAttribute('aria-expanded')) === 'true';

it('el archivo borrado sigue en el árbol, tachado', async () => {
  await row('b').click();
  await row('b/tres.md').waitForExist({ timeout: 10_000 });
  expect(await row('b/tres.md').getAttribute('class')).toContain('git-deleted');
  // Sin filtro no se ven los tipos que el explorador no muestra
  await row('a').click();
  await row('a/uno.md').waitForExist({ timeout: 10_000 });
  expect(await row('a/datos.bin').isExisting()).toBe(false);
});

it('expandir y contraer todo sin filtro', async () => {
  await s.browser.$('[aria-label="Expandir todas las carpetas"]').click();
  await row('a/sub/dos.md').waitForExist({ timeout: 10_000 });
  expect(await expanded('c')).toBe(true);
  await s.browser.$('[aria-label="Contraer todas las carpetas"]').click();
  await s.browser.waitUntil(async () => !(await row('a/uno.md').isExisting()), { timeout: 5_000 });
  expect(await expanded('c')).toBe(false);
});

it('con el filtro de Git se ve todo lo del commit y expandir/contraer solo toca el filtro', async () => {
  await row('c').click(); // c fuera del filtro, abierta
  await row('c/cuatro.md').waitForExist({ timeout: 10_000 });

  await s.browser.$('.git-filter-btn').click();
  await s.browser.waitUntil(async () => !(await row('c').isExisting()), { timeout: 10_000 });
  await s.browser.$('[aria-label="Expandir todas las carpetas"]').click();
  await row('a/datos.bin').waitForExist({ timeout: 10_000 });
  await row('a/sub/dos.md').waitForExist({ timeout: 10_000 });
  expect(await row('b/tres.md').getAttribute('class')).toContain('git-deleted');

  await s.browser.$('[aria-label="Contraer todas las carpetas"]').click();
  await s.browser.waitUntil(async () => !(await row('a/datos.bin').isExisting()), { timeout: 5_000 });

  // Al quitar el filtro, c sigue abierta como estaba
  await s.browser.$('.git-filter-btn').click();
  await row('c/cuatro.md').waitForExist({ timeout: 10_000 });
  expect(await expanded('a')).toBe(false);
});
