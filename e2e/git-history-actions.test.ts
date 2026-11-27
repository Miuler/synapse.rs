// En el Git Log: squash desde un commit (sin tocar lo sin confirmar), deshacerlo, revert y reset.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let vault: string;

const git = (...args: string[]) =>
  execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', ...args], { cwd: vault }).toString().trim();
const note = () => readFileSync(join(vault, 'nota.md'), 'utf8');
const log = () => git('log', '--format=%s').split('\n');

beforeAll(async () => {
  vault = mkdtempSync(join(tmpdir(), 'synapse-e2e-history-'));
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  git('init', '-q', '-b', 'main');
  git('config', 'user.name', 'e2e');
  git('config', 'user.email', 'e2e@local');
  git('config', 'commit.gpgsign', 'false');
  writeFileSync(join(vault, '.gitignore'), '.synapse/\n');
  writeFileSync(join(vault, 'nota.md'), '# Nota\n');
  git('add', '.');
  git('commit', '-qm', 'inicio');
  for (const text of ['dos', 'tres']) {
    writeFileSync(join(vault, 'nota.md'), `# Nota\n\n${text}\n`);
    git('commit', '-qam', text);
  }
  // Un commit de otra rama, creado sin cambiar de rama
  git('branch', 'otra', git('commit-tree', 'HEAD~2^{tree}', '-p', 'HEAD~2', '-m', 'aparte'));
  writeFileSync(join(vault, 'borrador.md'), 'sin confirmar\n');
  s = await startAppOnVault(vault);
  await s.browser.$('.git-branch-badge').waitForExist({ timeout: 30_000 });
  await s.browser.keys(['Control', 'p']);
  const input = await s.browser.$('.synapse-command-palette input');
  await input.waitForExist({ timeout: 15_000 });
  await input.setValue('Log de todas las ramas');
  await s.browser.$('.synapse-command-palette .command-item').waitForExist({ timeout: 15_000 });
  await s.browser.keys(['Enter']);
  await s.browser.$('.git-repo-log-dialog .repo-log-row').waitForExist({ timeout: 15_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(vault, { recursive: true, force: true });
});

const until = (check: () => boolean | Promise<boolean>, what: string) =>
  s.browser.waitUntil(async () => check(), { timeout: 15_000, timeoutMsg: what });

/** Abre el menú del commit con ese mensaje. */
async function openMenu(message: string) {
  await until(
    () =>
      s.browser.execute((message) => {
        const row = [...document.querySelectorAll('.git-repo-log-dialog .repo-log-row')].find(
          (r) => r.querySelector('.message-text')?.textContent?.trim() === message,
        );
        row?.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: 300, clientY: 200 }));
        return !!row;
      }, message) as Promise<boolean>,
    `el log no muestra «${message}»`,
  );
}

/** Abre el menú del commit con ese mensaje y elige una acción. */
async function choose(message: string, action: string) {
  await openMenu(message);
  const item = await s.browser.$(`.repo-log-menu [data-action="${action}"]`);
  await item.waitForEnabled({ timeout: 10_000 });
  await s.browser.execute((action) => (document.querySelector(`.repo-log-menu [data-action="${action}"]`) as HTMLElement).click(), action);
}
const press = async (selector: string) => {
  await s.browser.$(selector).waitForEnabled({ timeout: 10_000 });
  await s.browser.execute((selector) => (document.querySelector(selector) as HTMLElement).click(), selector);
};

it('une dos commits en uno sin tocar lo sin confirmar, y se deshace', async () => {
  const before = git('rev-parse', 'HEAD');
  await choose('dos', 'squash');
  const message = await s.browser.$('.repo-log-squash textarea');
  await message.waitForExist({ timeout: 10_000 });
  expect(await message.getValue()).toBe('dos\n\ntres');
  await message.setValue('dos y tres');
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/git-squash.png`);
  await press('[data-action="confirm-squash"]');
  await until(() => log().join() === 'dos y tres,inicio', 'no se unieron');
  expect(note()).toContain('tres');
  expect(git('status', '--porcelain')).toBe('?? borrador.md');
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/git-deshacer.png`);
  await press('[data-action="undo-history"]');
  await until(() => git('rev-parse', 'HEAD') === before, 'no se deshizo');
}, 60_000);

it('revierte un commit y hace reset hard, que se puede deshacer', async () => {
  await choose('tres', 'revert');
  await press('.repo-log-prompt [data-action="confirm"]');
  await until(() => log().length === 4, 'no se revirtió');
  expect(note()).not.toContain('tres');

  const before = git('rev-parse', 'HEAD');
  writeFileSync(join(vault, 'nota.md'), '# a medias\n');
  await choose('inicio', 'reset-hard');
  await press('.repo-log-prompt [data-action="confirm"]');
  await until(() => log().join() === 'inicio', 'no se hizo el reset');
  expect(note()).toBe('# Nota\n');
  expect(readFileSync(join(vault, 'borrador.md'), 'utf8')).toBe('sin confirmar\n');
  await press('[data-action="undo-history"]');
  await until(() => git('rev-parse', 'HEAD') === before && note() === '# a medias\n', 'no volvieron la rama y los cambios');
}, 60_000);

it('un commit de otra rama no se ofrece para revertir, pero sí para llevar allí la rama', async () => {
  await openMenu('aparte');
  await s.browser.$('.repo-log-menu [data-action="reset-soft"]').waitForExist({ timeout: 10_000 });
  expect(await s.browser.$('.repo-log-menu [data-action="revert"]').isExisting()).toBe(false);
  await openMenu('inicio');
  await s.browser.$('.repo-log-menu [data-action="revert"]').waitForExist({ timeout: 10_000 });
  await s.browser.execute(() => document.querySelector('.git-repo-log-dialog .repo-log-row')!.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true })));
}, 60_000);

it('el reset soft mueve la rama y deja los cambios preparados', async () => {
  git('checkout', '-q', '--', 'nota.md');
  const content = note();
  await choose('inicio', 'reset-soft');
  await press('.repo-log-prompt [data-action="confirm"]');
  await until(() => log().join() === 'inicio', 'no se hizo el reset');
  expect(note()).toBe(content);
  await press('[data-action="undo-history"]');
  await until(() => log().length > 1, 'no se deshizo');
  expect(git('status', '--porcelain')).toBe('?? borrador.md');
}, 60_000);
