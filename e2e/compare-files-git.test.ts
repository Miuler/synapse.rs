// En una bóveda con Git, «Diff» del menú con dos archivos seleccionados los compara entre sí
// (antes hacía el diff de Git de uno solo y mostraba «Sin diferencias»).
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-git-diff-'));
  const vault = join(base, 'vault');
  // Con .synapse la app abre la bóveda sin preguntar
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  writeFileSync(join(vault, 'AGENTS.md'), 'igual\nviejo\n');
  writeFileSync(join(vault, 'README.md'), 'igual\nnuevo\n');
  const git = (...args: string[]) =>
    execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', ...args], { cwd: vault });
  git('init', '-q');
  git('add', '.');
  git('commit', '-qm', 'inicio');
  s = await startAppOnVault(vault);
  await s.browser.$('.file-tree-item[data-path="README.md"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

it('«Diff» con dos archivos seleccionados compara uno contra el otro', async () => {
  for (const [i, p] of ['AGENTS.md', 'README.md'].entries()) {
    await s.browser.execute(
      (path: string, ctrl: boolean) => {
        document
          .querySelector(`.file-tree-item[data-path="${path}"]`)!
          .dispatchEvent(new MouseEvent('click', { bubbles: true, ctrlKey: ctrl }));
      },
      p,
      i > 0,
    );
    await s.browser.pause(600);
  }
  await s.browser.execute(() => {
    const el = document.querySelector('.file-tree-item[data-path="README.md"]')!;
    const { x, y } = el.getBoundingClientRect();
    el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: x + 5, clientY: y + 5 }));
  });
  const item = s.browser.$('.diff-two-files');
  await item.waitForExist({ timeout: 5_000 });
  // Ya no se ofrece el diff de Git de un solo archivo
  const labels = (await s.browser.execute(() =>
    [...document.querySelectorAll('.context-menu-item')].map((i) => i.textContent?.trim()),
  )) as string[];
  expect(labels).not.toContain('Diff');
  await s.browser.execute(() => (document.querySelector('.diff-two-files') as HTMLElement).click());
  await s.browser.waitUntil(
    () => s.browser.execute(() => !!document.querySelector('.diff-viewer-wrapper')) as Promise<boolean>,
    { timeout: 10_000 },
  );
  await s.browser.pause(800);
  const text = (await s.browser.execute(
    () => [...document.querySelectorAll('.diff-viewer-wrapper')].map((e) => e.textContent).join('\n'),
  )) as string;
  expect(text).toContain('viejo');
  expect(text).toContain('nuevo');
});
