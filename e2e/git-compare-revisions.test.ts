// Historial de Git: «Diff entre versiones» cambia las acciones por casillas, deja elegir exactamente
// dos commits y abre el diff entre esas dos versiones del archivo (de la más antigua a la más nueva).
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-compare-'));
  const vault = join(base, 'vault');
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  const git = (...args: string[]) =>
    execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', ...args], { cwd: vault });
  git('init', '-q');
  for (const [i, line] of ['primera línea', 'segunda línea', 'tercera línea'].entries()) {
    writeFileSync(join(vault, 'nota.md'), `# Nota\n\n${line}\n`);
    git('add', '.');
    git('commit', '-qm', `versión ${i + 1}`);
  }
  s = await startAppOnVault(vault);
  await s.browser.$('.file-tree-item[data-path="nota.md"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

const count = (sel: string) => s.browser.execute((q: string) => document.querySelectorAll(q).length, sel) as Promise<number>;

it('elige dos versiones y abre su diff', async () => {
  // Historial de Git desde el menú contextual de la nota
  await s.browser.execute(() => {
    const el = document.querySelector('.file-tree-item[data-path="nota.md"]')!;
    const { x, y } = el.getBoundingClientRect();
    el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: x + 5, clientY: y + 5 }));
  });
  await s.browser.$('.context-menu-item').waitForExist({ timeout: 5_000 });
  await s.browser.execute(() => {
    ([...document.querySelectorAll('.context-menu-item')].find((i) => i.textContent?.includes('Historial de Git')) as HTMLElement).click();
  });
  await s.browser.$('.git-log-dialog .git-log-row').waitForExist({ timeout: 10_000 });
  expect(await count('.git-log-dialog .git-log-row')).toBe(3);
  expect(await count('.git-log-dialog .col-actions .action-btn')).toBeGreaterThan(0);

  // Modo «Diff entre versiones»: las acciones se cambian por casillas
  await s.browser.$('.git-log-dialog .compare-toggle').click();
  expect(await count('.git-log-dialog td.col-actions')).toBe(0);
  expect(await count('.git-log-dialog .compare-check')).toBe(3);
  const compare = s.browser.$('.git-log-dialog .compare-go-btn');
  expect(await compare.isEnabled()).toBe(false);

  // Solo dos: con dos marcadas, la tercera queda deshabilitada
  const checks = await s.browser.$$('.git-log-dialog .compare-check');
  await checks[0].click(); // versión 3 (la más nueva)
  expect(await compare.isEnabled()).toBe(false);
  await checks[2].click(); // versión 1 (la más antigua)
  expect(await compare.isEnabled()).toBe(true);
  expect(await checks[1].isEnabled()).toBe(false);

  // Volver a desactivar el modo restaura las acciones y limpia la selección
  await s.browser.$('.git-log-dialog .compare-toggle').click();
  expect(await count('.git-log-dialog td.col-actions')).toBe(3);
  await s.browser.$('.git-log-dialog .compare-toggle').click();
  expect(await s.browser.$('.git-log-dialog .compare-go-btn').isEnabled()).toBe(false);
  const again = await s.browser.$$('.git-log-dialog .compare-check');
  await again[0].click();
  await again[2].click();
  await s.browser.$('.git-log-dialog .compare-go-btn').click();

  // Se abre el diff de la versión 1 a la versión 3
  await s.browser.$('.git-log-dialog').waitForExist({ timeout: 5_000, reverse: true });
  await s.browser.waitUntil(
    async () => ((await s.browser.execute(() => document.querySelector('.diff-file-badge')?.textContent ?? '')) as string).includes('VERSIONES'),
    { timeout: 10_000, timeoutMsg: 'no se abrió el diff entre versiones' },
  );
  await s.browser.waitUntil(
    async () => {
      const body = (await s.browser.execute(() => document.querySelector('.tab-pane:not(.hidden)')?.textContent ?? '')) as string;
      return body.includes('primera línea') && body.includes('tercera línea');
    },
    { timeout: 10_000, timeoutMsg: 'el diff no muestra las dos versiones' },
  );
});
