// Rama local por detrás de su remota (tras un `git fetch`): el badge muestra ↓N, el diálogo de ramas
// la actualiza con «Actualizar desde origin/main» (fast-forward) y el menú del Git Log ofrece
// «Avanzar «main» hasta aquí» en un commit posterior; el menú sale junto al puntero.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;
let origen: string;
let vault: string;

const git = (cwd: string, ...args: string[]) =>
  execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', ...args], { cwd }).toString().trim();
const commitEnOrigen = (texto: string) => {
  writeFileSync(join(origen, 'nota.md'), `# Nota\n\n${texto}\n`);
  git(origen, 'commit', '-qam', texto);
};

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-ff-'));
  origen = join(base, 'origen');
  vault = join(base, 'vault');
  mkdirSync(origen, { recursive: true });
  git(origen, 'init', '-q', '-b', 'main');
  writeFileSync(join(origen, '.gitignore'), '.synapse/\n');
  writeFileSync(join(origen, 'nota.md'), '# Nota\n');
  git(origen, 'add', '.');
  git(origen, 'commit', '-qm', 'inicio');
  git(base, 'clone', '-q', origen, vault);
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  commitEnOrigen('dos');
  commitEnOrigen('tres');
  git(vault, 'fetch', '-q');
  s = await startAppOnVault(vault);
  await s.browser.$('.git-branch-badge').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

const badgeTrack = () =>
  s.browser.execute(() => document.querySelector('.git-branch-badge .git-branch-track')?.textContent?.trim() ?? '') as Promise<string>;

it('el badge muestra ↓2 y «Actualizar desde origin/main» avanza la rama y los archivos', async () => {
  await s.browser.waitUntil(async () => (await badgeTrack()) === '↓2', { timeout: 15_000, timeoutMsg: 'el badge no muestra ↓2' });
  await s.browser.execute(() => (document.querySelector('.git-branch-badge') as HTMLElement).click());
  const btn = await s.browser.$('.git-branch-dialog .git-row-btn.fast-forward');
  await btn.waitForExist({ timeout: 10_000 });
  expect(await btn.getText()).toContain('origin/main');
  await s.browser.execute(() => (document.querySelector('.git-branch-dialog .git-row-btn.fast-forward') as HTMLElement).click());
  await s.browser.waitUntil(async () => git(vault, 'rev-parse', 'main') === git(vault, 'rev-parse', 'origin/main'), {
    timeout: 10_000,
    timeoutMsg: 'main no avanzó',
  });
  expect(readFileSync(join(vault, 'nota.md'), 'utf8')).toContain('tres');
  await s.browser.$('.git-branch-dialog .git-row-btn.fast-forward').waitForExist({ timeout: 10_000, reverse: true });
  await s.browser.keys(['Escape']);
  await s.browser.waitUntil(async () => (await badgeTrack()) === '', { timeout: 10_000, timeoutMsg: 'el badge sigue con ↓' });
}, 60_000);

it('en el Git Log, «Avanzar «main» hasta aquí» en un commit posterior; el menú sale junto al puntero', async () => {
  commitEnOrigen('cuatro');
  git(vault, 'fetch', '-q');
  await s.browser.keys(['Control', 'p']);
  const input = await s.browser.$('.synapse-command-palette input');
  await input.waitForExist({ timeout: 15_000 });
  await input.setValue('Log de todas las ramas');
  await s.browser.waitUntil(
    async () => (await s.browser.execute(() => !!document.querySelector('.synapse-command-palette .command-item'))) as boolean,
    { timeout: 15_000 },
  );
  await s.browser.keys(['Enter']);
  await s.browser.$('.git-repo-log-dialog .repo-log-row').waitForExist({ timeout: 15_000 });
  const msgs = () =>
    s.browser.execute(() =>
      [...document.querySelectorAll('.git-repo-log-dialog .repo-log-row .message-text')].map((m) => m.textContent?.trim()),
    ) as Promise<string[]>;
  await s.browser.waitUntil(async () => (await msgs()).includes('cuatro'), {
    timeout: 10_000,
    timeoutMsg: 'el log no muestra «cuatro»',
  }).catch(async (e) => {
    console.log('mensajes del log:', await msgs());
    throw e;
  });
  const click = (msg: string) =>
    s.browser.execute((msg) => {
      const row = [...document.querySelectorAll('.git-repo-log-dialog .repo-log-row')].find(
        (r) => r.querySelector('.message-text')?.textContent?.trim() === msg,
      ) as HTMLElement;
      const { x, y } = row.getBoundingClientRect();
      const at = { x: Math.round(x + 60), y: Math.round(y + 8) };
      row.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: at.x, clientY: at.y }));
      return at;
    }, msg) as Promise<{ x: number; y: number }>;

  // Un commit anterior: no se ofrece
  await click('dos');
  await s.browser.$('.repo-log-menu [data-action="new-tag"]').waitForExist({ timeout: 10_000 });
  await s.browser.pause(500);
  expect(await s.browser.$('.repo-log-menu [data-action="fast-forward"]').isExisting()).toBe(false);
  await s.browser.execute(() => document.querySelector('.git-repo-log-dialog .repo-log-row')!.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true })));

  const at = await click('cuatro');
  const ff = await s.browser.$('.repo-log-menu [data-action="fast-forward"]');
  await ff.waitForExist({ timeout: 10_000 });
  await s.browser.waitUntil(async () => (await ff.isEnabled()), { timeout: 5_000 });
  // El menú sale donde se hizo clic (no desplazado por el diálogo)
  await s.browser.pause(100);
  const box = (await s.browser.execute(() => {
    const r = document.querySelector('.repo-log-menu')!.getBoundingClientRect();
    return { x: r.left, y: r.top };
  })) as { x: number; y: number };
  expect(Math.abs(box.x - at.x)).toBeLessThan(3);
  expect(Math.abs(box.y - at.y)).toBeLessThan(3);
  expect(await ff.getText()).toContain('main');
  await s.browser.execute(() => (document.querySelector('.repo-log-menu [data-action="fast-forward"]') as HTMLElement).click());
  await s.browser.waitUntil(async () => git(vault, 'rev-parse', 'main') === git(vault, 'rev-parse', 'origin/main'), {
    timeout: 10_000,
    timeoutMsg: 'main no avanzó desde el log',
  });
  expect(readFileSync(join(vault, 'nota.md'), 'utf8')).toContain('cuatro');
}, 60_000);
