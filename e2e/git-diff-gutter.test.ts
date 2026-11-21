// Indicadores de diff de Git en el gutter de CodeMirror (líneas añadidas y modificadas respecto a HEAD).
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;

function git(cwd: string, args: string[]) {
  execFileSync('git', args, { cwd, stdio: 'ignore' });
}

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-gitgutter-'));
  const vault = join(base, 'vault');
  mkdirSync(join(vault, '.synapse'), { recursive: true });

  git(vault, ['init']);
  git(vault, ['config', 'user.email', 'test@example.com']);
  git(vault, ['config', 'user.name', 'Test User']);

  writeFileSync(join(vault, 'nota.md'), 'linea 1\nlinea 2\nlinea 3\n');
  git(vault, ['add', 'nota.md']);
  git(vault, ['commit', '--no-gpg-sign', '-m', 'init']);

  // Modifica línea 2 y añade línea 4
  writeFileSync(join(vault, 'nota.md'), 'linea 1\nlinea 2 editada\nlinea 3\nlinea nueva\n');
  // Archivo sin seguimiento (untracked)
  writeFileSync(join(vault, 'nueva.md'), 'hola\nmundo\n');

  s = await startAppOnVault(vault);
  await s.browser.$('.file-tree-item[data-path="nota.md"]').waitForExist({ timeout: 30_000 });
}, 120_000);

afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

it('muestra marcadores de línea modificada y añadida en nota.md', async () => {
  await s.browser.$('.file-tree-item[data-path="nota.md"]').click();
  await s.browser.$('.tab-title-container.active[data-path="nota.md"]').waitForExist({ timeout: 10_000 });

  // Pasar a modo Edición para montar CodeMirror si abre en Lectura
  const toggle = s.browser.$('.editor-header .view-toggle-btn');
  if (await toggle.isExisting()) {
    const label = await toggle.getText();
    if (label.includes('Lectura')) {
      await toggle.click();
    }
  }

  await s.browser.$('.tab-pane:not(.hidden) .cm-git-diff-gutter').waitForExist({ timeout: 10_000 });
  await s.browser.waitUntil(
    async () =>
      s.browser.execute(() => {
        const pane = document.querySelector('.tab-pane:not(.hidden)');
        if (!pane) return false;
        const mod = pane.querySelectorAll('.cm-git-diff-modified').length;
        const add = pane.querySelectorAll('.cm-git-diff-added').length;
        return mod >= 1 && add >= 1;
      }),
    { timeout: 10_000, timeoutMsg: 'No aparecieron los marcadores .cm-git-diff-modified y .cm-git-diff-added' },
  );

  const counts = await s.browser.execute(() => {
    const pane = document.querySelector('.tab-pane:not(.hidden)');
    return {
      modified: pane?.querySelectorAll('.cm-git-diff-modified').length ?? 0,
      added: pane?.querySelectorAll('.cm-git-diff-added').length ?? 0,
    };
  });
  expect(counts.modified).toBe(1);
  expect(counts.added).toBe(1);
}, 60_000);

it('al pasar por la marca, un tooltip muestra lo que había en Git y resalta lo que cambió', async () => {
  const marker = () =>
    s.browser.execute((ev) => {
      const m = document.querySelector('.tab-pane:not(.hidden) .cm-git-diff-modified') as HTMLElement;
      m.dispatchEvent(new MouseEvent(ev, { bubbles: true, clientX: m.getBoundingClientRect().x + 1, clientY: m.getBoundingClientRect().y + 2 }));
    }, 'mousemove');
  await marker();
  await s.browser.$('.cm-git-diff-tip').waitForExist({ timeout: 5_000, timeoutMsg: 'no apareció el tooltip' });
  const tip = (await s.browser.execute(() => {
    const t = document.querySelector('.cm-git-diff-tip')!;
    return {
      head: t.querySelector('.cm-git-diff-tip-head')?.textContent,
      old: t.querySelector('.cm-git-diff-tip-old')?.textContent,
      now: t.querySelector('.cm-git-diff-tip-new')?.textContent,
      ins: [...t.querySelectorAll('.cm-git-diff-tip-ins')].map((e) => e.textContent),
    };
  })) as { head: string; old: string; now: string; ins: string[] };
  expect(tip.head).toContain('Modificado');
  expect(tip.old).toBe('−linea 2');
  expect(tip.now).toBe('+linea 2 editada');
  expect(tip.ins).toEqual([' editada']);
  // Aparece pegado a la marca: justo debajo de su línea y al inicio del texto (no al final de la línea)
  const gap = (await s.browser.execute(() => {
    const m = document.querySelector('.tab-pane:not(.hidden) .cm-git-diff-modified')!.getBoundingClientRect();
    const t = document.querySelector('.cm-git-diff-tip')!.getBoundingClientRect();
    return { dy: t.top - m.bottom, dx: t.left - m.right };
  })) as { dy: number; dx: number };
  expect(gap.dy).toBeGreaterThan(-5);
  expect(gap.dy).toBeLessThan(30);
  expect(gap.dx).toBeLessThan(80);
  // Al salir del margen se oculta
  await s.browser.execute(() => {
    const g = document.querySelector('.tab-pane:not(.hidden) .cm-git-diff-gutter') as HTMLElement;
    g.dispatchEvent(new MouseEvent('mouseleave', { bubbles: false }));
  });
  await s.browser.$('.cm-git-diff-tip').waitForExist({ timeout: 5_000, reverse: true, timeoutMsg: 'el tooltip no se ocultó' });
}, 30_000);

it('muestra todas las líneas como añadidas en archivo untracked (nueva.md)', async () => {
  await s.browser.$('.file-tree-item[data-path="nueva.md"]').click();
  await s.browser.$('.tab-title-container.active[data-path="nueva.md"]').waitForExist({ timeout: 10_000 });

  const toggle = s.browser.$('.editor-header .view-toggle-btn');
  if (await toggle.isExisting()) {
    const label = await toggle.getText();
    if (label.includes('Lectura')) {
      await toggle.click();
    }
  }

  await s.browser.$('.tab-pane:not(.hidden) .cm-git-diff-gutter').waitForExist({ timeout: 10_000 });
  await s.browser.waitUntil(
    async () =>
      s.browser.execute(() => {
        const pane = document.querySelector('.tab-pane:not(.hidden)');
        return (pane?.querySelectorAll('.cm-git-diff-added').length ?? 0) >= 2;
      }),
    { timeout: 10_000, timeoutMsg: 'No aparecieron los marcadores .cm-git-diff-added en nueva.md' },
  );
}, 60_000);
