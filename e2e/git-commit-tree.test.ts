// Diálogo de commit: botón para ver los archivos en árbol de carpetas, con selección por carpeta.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-commit-tree-'));
  const vault = join(base, 'vault');
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  writeFileSync(join(vault, 'nota.md'), '# Nota\n');
  writeFileSync(join(vault, '.gitignore'), '.synapse/\n');
  const git = (...args: string[]) =>
    execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', ...args], { cwd: vault });
  git('init', '-q');
  git('add', '.');
  git('commit', '-qm', 'inicio');
  writeFileSync(join(vault, 'nota.md'), '# Nota\n\ncambio\n');
  for (const f of ['src/components/a.md', 'src/components/b.md', 'src/software/c.md', 'docs/guias/d.md']) {
    mkdirSync(join(vault, f, '..'), { recursive: true });
    writeFileSync(join(vault, f), `# ${f}\n`);
  }
  s = await startAppOnVault(vault);
  await s.browser.$('.file-tree-item[data-path="nota.md"]').waitForExist({ timeout: 30_000 });
  await s.browser.execute(() => localStorage.removeItem('synapse.gitCommitDialog.view'));
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

async function openCommit() {
  await s.browser.execute(() => {
    const el = document.querySelector('.file-tree-item[data-path="nota.md"]')!;
    const { x, y } = el.getBoundingClientRect();
    el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: x + 5, clientY: y + 5 }));
  });
  await s.browser.$('.context-menu-item').waitForExist({ timeout: 5_000 });
  await s.browser.execute(() => {
    ([...document.querySelectorAll('.context-menu-item')].find((i) => i.textContent?.includes('Hacer commit')) as HTMLElement).click();
  });
  await s.browser.$('.git-commit-dialog .git-commit-file-item').waitForExist({ timeout: 5_000 });
}

const folders = () =>
  s.browser.execute(() =>
    [...document.querySelectorAll('.git-commit-dialog .git-commit-folder-item')].map((f) => f.querySelector('.git-commit-folder-name')!.textContent),
  ) as Promise<string[]>;
const selected = () =>
  s.browser.execute(() => {
    const t = document.querySelector('.git-commit-dialog .git-commit-files-title')!.textContent ?? '';
    return Number(/\((\d+) de/.exec(t)?.[1]);
  }) as Promise<number>;

it('el botón cambia a árbol, las carpetas se pliegan y su casilla selecciona todo su contenido', async () => {
  await openCommit();
  expect(await folders()).toEqual([]);
  await s.browser.$('.git-commit-dialog .git-commit-view-btn').click();
  await s.browser.$('.git-commit-dialog .git-commit-folder-item').waitForExist({ timeout: 5_000 });
  // docs/guias se compacta en una sola fila; src tiene dos subcarpetas
  expect(await folders()).toEqual(['docs/guias/', 'src/', 'components/', 'software/']);

  // Solo nota.md venía seleccionado (se abrió desde ella): la casilla de src marca sus 3 archivos
  expect(await selected()).toBe(1);
  await s.browser.$('.git-commit-dialog [data-folder="src"] input[type="checkbox"]').click();
  expect(await selected()).toBe(4);
  // Quitar uno deja la carpeta en estado intermedio
  await s.browser.execute(() =>
    ([...document.querySelectorAll('.git-commit-dialog .git-commit-file-item')].find((l) => l.getAttribute('title') === 'src/software/c.md')!.querySelector('input') as HTMLInputElement).click(),
  );
  expect(await selected()).toBe(3);
  expect(await s.browser.execute(() => (document.querySelector('.git-commit-dialog [data-folder="src"] input') as HTMLInputElement).indeterminate)).toBe(true);

  // Plegar src oculta sus subcarpetas y archivos
  await s.browser.$('.git-commit-dialog [data-folder="src"] .git-commit-folder-chevron').click();
  expect(await folders()).toEqual(['docs/guias/', 'src/']);

  // La vista se recuerda al volver a abrir
  await s.browser.keys('Escape');
  await s.browser.$('.git-commit-dialog').waitForExist({ timeout: 5_000, reverse: true });
  await openCommit();
  await s.browser.$('.git-commit-dialog .git-commit-folder-item').waitForExist({ timeout: 5_000 });
  await s.browser.keys('Escape');
}, 60_000);
