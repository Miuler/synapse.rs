// Sin Git que funcione (aquí: config.jsonc apunta a un git que no existe), al abrir una bóveda que es
// un repositorio se ofrece elegir el ejecutable; Ajustes → Git muestra el estado y permite volver
// al del sistema, que queda guardado en config.jsonc.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;
let configDir: string;

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-git-missing-'));
  const vault = join(base, 'vault');
  configDir = join(base, 'config');
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  mkdirSync(configDir, { recursive: true });
  writeFileSync(join(vault, '.gitignore'), '.synapse/\n');
  writeFileSync(join(vault, 'nota.md'), '# Nota\n');
  const git = (...args: string[]) =>
    execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', ...args], { cwd: vault });
  git('init', '-q', '-b', 'main');
  git('add', '.');
  git('commit', '-qm', 'inicio');
  writeFileSync(join(configDir, 'config.jsonc'), '{ "git": { "path": "/no/existe/git" } }\n');
  const prev = process.env.SYNAPSE_CONFIG_DIR;
  process.env.SYNAPSE_CONFIG_DIR = configDir;
  try {
    s = await startAppOnVault(vault);
  } finally {
    if (prev === undefined) delete process.env.SYNAPSE_CONFIG_DIR;
    else process.env.SYNAPSE_CONFIG_DIR = prev;
  }
  await s.browser.$('.file-tree-item[data-path="nota.md"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

it('sin Git avisa con un diálogo para elegir el ejecutable, y la bóveda se sigue viendo como repositorio', async () => {
  await s.browser.$('.git-missing-dialog').waitForExist({ timeout: 15_000, timeoutMsg: 'no apareció el aviso de Git' });
  const buttons = (await s.browser.execute(() =>
    [...document.querySelectorAll('.git-missing-dialog button')].map((b) => b.textContent?.trim() ?? ''),
  )) as string[];
  expect(buttons.some((b) => b.includes('Elegir ejecutable'))).toBe(true);
  // El estado se lee con gix: la rama se ve aunque no haya git
  expect(await s.browser.execute(() => document.querySelector('.git-branch-badge')?.textContent ?? '')).toContain('main');
  await s.browser.execute(() =>
    ([...document.querySelectorAll('.git-missing-dialog button')].find((b) => b.textContent?.includes('Ahora no')) as HTMLElement).click(),
  );
  await s.browser.$('.git-missing-dialog').waitForExist({ timeout: 5_000, reverse: true });
}, 60_000);

it('Ajustes → Git muestra que falta y «Usar el del sistema» lo arregla y lo guarda', async () => {
  const before = (await s.browser.execute(() => (window as any).__TAURI_INTERNALS__.invoke('get_git_info'))) as { version?: string | null; configuredPath?: string };
  expect(before.version ?? null).toBeNull();
  expect(before.configuredPath).toBe('/no/existe/git');
  const after = (await s.browser.execute(() => (window as any).__TAURI_INTERNALS__.invoke('set_git_path', { path: null }))) as { version?: string | null };
  expect(after.version).toStartWith('git version');
  const saved = readFileSync(join(configDir, 'config.jsonc'), 'utf8');
  expect(saved).not.toContain('/no/existe/git');
}, 30_000);
