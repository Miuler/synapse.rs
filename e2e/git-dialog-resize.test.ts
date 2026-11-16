// Los diálogos de commit, historial y ramas de Git abren grandes, se agrandan arrastrando un borde
// y recuerdan el tamaño al volver a abrirlos.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-dialog-resize-'));
  const vault = join(base, 'vault');
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  writeFileSync(join(vault, 'nota.md'), '# Nota\n');
  const git = (...args: string[]) =>
    execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', ...args], { cwd: vault });
  git('init', '-q');
  git('add', '.');
  git('commit', '-qm', 'inicio');
  writeFileSync(join(vault, 'nota.md'), '# Nota\n\ncambio\n');
  s = await startAppOnVault(vault);
  await s.browser.$('.file-tree-item[data-path="nota.md"]').waitForExist({ timeout: 30_000 });
  // Sin tamaños guardados de corridas anteriores
  await s.browser.execute(() => {
    localStorage.removeItem('synapse.gitCommitDialog.size');
    localStorage.removeItem('synapse.gitLogDialog.size');
    localStorage.removeItem('synapse.gitBranchDialog.size');
  });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

async function openFromMenu(text: string, dialog: string) {
  if (text === 'ramas') {
    await s.browser.execute(() => (document.querySelector('.git-branch-badge') as HTMLElement).click());
    await s.browser.$(dialog).waitForExist({ timeout: 5_000 });
    await Bun.sleep(300);
    return;
  }
  await s.browser.execute(() => {
    const el = document.querySelector('.file-tree-item[data-path="nota.md"]')!;
    const { x, y } = el.getBoundingClientRect();
    el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: x + 5, clientY: y + 5 }));
  });
  await s.browser.$('.context-menu-item').waitForExist({ timeout: 5_000 });
  await s.browser.execute((t: string) => {
    ([...document.querySelectorAll('.context-menu-item')].find((i) => i.textContent?.includes(t)) as HTMLElement).click();
  }, text);
  await s.browser.$(dialog).waitForExist({ timeout: 5_000 });
  await Bun.sleep(300); // animación de entrada
}

const width = (dialog: string) =>
  s.browser.execute((d: string) => document.querySelector(d)!.getBoundingClientRect().width, dialog) as Promise<number>;

/** Arrastra el borde derecho del diálogo `dx` píxeles. */
async function dragRightEdge(dialog: string, dx: number) {
  await s.browser.execute(
    (d: string, delta: number) => {
      const handle = document.querySelector(`${d} .dialog-resize.e`) as HTMLElement;
      const r = handle.getBoundingClientRect();
      const x = r.left + r.width / 2;
      const y = r.top + r.height / 2;
      handle.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 0, clientX: x, clientY: y }));
      window.dispatchEvent(new PointerEvent('pointermove', { clientX: x + delta, clientY: y }));
      window.dispatchEvent(new PointerEvent('pointerup', { clientX: x + delta, clientY: y }));
    },
    dialog,
    dx,
  );
}

for (const [name, menu, dialog] of [
  ['commit', 'Hacer commit', '.git-commit-dialog'],
  ['historial', 'Historial de Git', '.git-log-dialog'],
  ['ramas', 'ramas', '.git-branch-dialog'],
] as const) {
  it(`el diálogo de ${name} se redimensiona y recuerda el tamaño`, async () => {
    await openFromMenu(menu, dialog);
    const before = await width(dialog);
    const viewport = (await s.browser.execute(() => window.innerWidth)) as number;
    expect(before).toBeGreaterThan(Math.min(700, viewport - 40));
    await dragRightEdge(dialog, -50);
    const after = await width(dialog);
    // Centrado: el borde se mueve 50 px y el ancho cambia el doble
    expect(Math.round(before - after)).toBe(100);
    await s.browser.keys('Escape');
    await s.browser.$(dialog).waitForExist({ timeout: 5_000, reverse: true });
    await openFromMenu(menu, dialog);
    expect(Math.round(await width(dialog))).toBe(Math.round(after));
    await s.browser.keys('Escape');
    await s.browser.$(dialog).waitForExist({ timeout: 5_000, reverse: true });
  });
}

it('el historial de Git se mueve arrastrando la cabecera y vuelve al centro con doble clic', async () => {
  const dialog = '.git-log-dialog';
  await openFromMenu('Historial de Git', dialog);
  const pos = () =>
    s.browser.execute((d: string) => {
      const r = document.querySelector(d)!.getBoundingClientRect();
      return { x: Math.round(r.left), y: Math.round(r.top) };
    }, dialog) as Promise<{ x: number; y: number }>;
  const before = await pos();
  // Arrastrar la cabecera 60 px a la izquierda y 40 px hacia abajo
  await s.browser.execute((d: string) => {
    const header = document.querySelector(`${d} .resizable-dialog-header`) as HTMLElement;
    const r = header.getBoundingClientRect();
    const x = r.left + 200;
    const y = r.top + r.height / 2;
    header.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 0, clientX: x, clientY: y }));
    window.dispatchEvent(new PointerEvent('pointermove', { clientX: x - 60, clientY: y + 40 }));
    window.dispatchEvent(new PointerEvent('pointerup', { clientX: x - 60, clientY: y + 40 }));
  }, dialog);
  await Bun.sleep(100);
  const moved = await pos();
  expect(moved.x - before.x).toBe(-60);
  expect(moved.y - before.y).toBe(40);

  // Al volver a abrirlo sigue donde se dejó
  await s.browser.keys('Escape');
  await s.browser.$(dialog).waitForExist({ timeout: 5_000, reverse: true });
  await openFromMenu('Historial de Git', dialog);
  expect(await pos()).toEqual(moved);

  // Doble clic en la cabecera: vuelve al centro
  await s.browser.execute((d: string) => {
    const title = document.querySelector(`${d} .resizable-dialog-title`) as HTMLElement;
    title.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
  }, dialog);
  await Bun.sleep(100);
  expect(await pos()).toEqual(before);
  await s.browser.keys('Escape');
});
