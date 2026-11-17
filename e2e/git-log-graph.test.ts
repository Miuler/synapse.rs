// Historial de Git de un archivo con el grafo de ramas: dos ramas crean el mismo archivo por su
// cuenta y se fusionan; el commit de la otra rama sale en su propia columna y marcado.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-loggraph-'));
  const vault = join(base, 'vault');
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  const git = (...args: string[]) =>
    execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', ...args], { cwd: vault });
  git('init', '-q', '-b', 'main');
  writeFileSync(join(vault, 'otro.md'), 'base\n');
  git('add', '.');
  git('commit', '-qm', 'base');
  git('checkout', '-qb', 'lado');
  writeFileSync(join(vault, 'nota.md'), '# Lado\n');
  git('add', '.');
  git('commit', '-qm', 'crea en lado');
  git('checkout', '-q', 'main');
  writeFileSync(join(vault, 'nota.md'), '# Principal\n');
  git('add', '.');
  git('commit', '-qm', 'crea en principal');
  git('merge', '-q', '-s', 'ours', '--no-edit', 'lado');
  writeFileSync(join(vault, 'nota.md'), '# Principal\n\nmás\n');
  git('commit', '-qam', 'edita');
  s = await startAppOnVault(vault);
  await s.browser.$('.file-tree-item[data-path="nota.md"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

it('el historial dibuja las ramas y marca qué commit creó el archivo', async () => {
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
  const rows = (await s.browser.execute(() =>
    [...document.querySelectorAll('.git-log-dialog .git-log-row')].map((r) => ({
      msg: r.querySelector('.col-message')?.textContent?.replace(/\s+/g, ' ').trim() ?? '',
      left: parseFloat((r.querySelector('.graph-dot') as HTMLElement | null)?.style.left ?? '-1'),
      origin: !!r.querySelector('.graph-dot.origin'),
      patch: !!r.querySelector('.patch-btn'),
    })),
  )) as { msg: string; left: number; origin: boolean; patch: boolean }[];
  const row = (m: string) => rows.find((r) => r.msg.endsWith(m))!;

  expect(rows.length).toBe(4);
  expect(row('edita').msg).not.toContain('nuevo');
  expect(rows.find((r) => r.msg.includes('Merge'))!.msg).toContain('fusión');
  // Los dos commits que crearon el archivo: «nuevo», punto hueco y sin «Ver cambios»
  for (const m of ['crea en principal', 'crea en lado']) {
    expect(row(m).msg).toContain('nuevo');
    expect(row(m).origin).toBe(true);
    expect(row(m).patch).toBe(false);
  }
  // Uno de ellos va en otra columna y se marca como de otra rama
  const lefts = new Set(rows.map((r) => r.left));
  expect(lefts.size).toBe(2);
  expect(rows.filter((r) => r.msg.includes('otra rama')).length).toBe(1);
});
