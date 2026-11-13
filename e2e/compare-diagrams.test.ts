// Diffs visuales de dibujos y diagramas: comparar dos Excalidraw o dos Mermaid (2 Paneles, Texto y
// Diferencias píxel por píxel) y el diff de Git de un Mermaid con modo Diferencias.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;

const rect = (id: string, x: number, color: string) => ({
  id, type: 'rectangle', x, y: 0, width: 120, height: 80, angle: 0, strokeColor: color, backgroundColor: color,
  fillStyle: 'solid', strokeWidth: 2, strokeStyle: 'solid', roughness: 0, opacity: 100, groupIds: [], frameId: null,
  roundness: null, seed: 1, version: 1, versionNonce: 1, isDeleted: false, boundElements: null, updated: 1, link: null,
  locked: false,
});
const drawing = (...elements: object[]) =>
  JSON.stringify({ type: 'excalidraw', version: 2, source: 'e2e', elements, appState: {}, files: {} });

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-diagrams-'));
  const vault = join(base, 'vault');
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  writeFileSync(join(vault, 'a.excalidraw'), drawing(rect('r1', 0, '#1e88e5')));
  writeFileSync(join(vault, 'b.excalidraw'), drawing(rect('r1', 0, '#1e88e5'), rect('r2', 200, '#e53935')));
  writeFileSync(join(vault, 'a.mmd'), 'graph TD\n  A --> B\n');
  writeFileSync(join(vault, 'b.mmd'), 'graph TD\n  A --> B\n  B --> C\n');
  writeFileSync(join(vault, 'flujo.mmd'), 'graph TD\n  X --> Y\n');
  const git = (...args: string[]) =>
    execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', ...args], { cwd: vault });
  git('init', '-q');
  git('add', '.');
  git('commit', '-qm', 'inicio');
  // Segundo commit con un cambio en flujo.mmd, para ver su parche desde el historial
  writeFileSync(join(vault, 'flujo.mmd'), 'graph TD\n  X --> Y\n  Y --> Z\n');
  git('commit', '-qam', 'agrega Z');
  // Un cambio en el stage y otro distinto sin preparar
  writeFileSync(join(vault, 'flujo.mmd'), 'graph TD\n  X --> Y\n  Y --> Z\n  Z --> W\n');
  git('add', 'flujo.mmd');
  writeFileSync(join(vault, 'flujo.mmd'), 'graph TD\n  X --> Y\n  Y --> Z\n  Z --> W\n  W --> V\n');
  s = await startAppOnVault(vault);
  await s.browser.$('.file-tree-item[data-path="flujo.mmd"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

async function contextMenu(path: string) {
  await s.browser.execute((p: string) => {
    const el = document.querySelector(`.file-tree-item[data-path="${p}"]`)!;
    const { x, y } = el.getBoundingClientRect();
    el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: x + 5, clientY: y + 5 }));
  }, path);
  await s.browser.$('.context-menu-item').waitForExist({ timeout: 5_000 });
}

async function clickMenu(text: string) {
  await s.browser.execute((t: string) => {
    ([...document.querySelectorAll('.context-menu-item')].find((i) => i.textContent?.includes(t)) as HTMLElement).click();
  }, text);
}

async function compare(a: string, b: string) {
  // Abrir el primer archivo puede reiniciar la selección: se reintenta hasta que el menú ofrezca comparar
  for (let attempt = 0; attempt < 3; attempt++) {
    for (const [i, p] of [a, b].entries()) {
      await s.browser.execute(
        (path: string, ctrl: boolean) => {
          document.querySelector(`.file-tree-item[data-path="${path}"]`)!.dispatchEvent(new MouseEvent('click', { bubbles: true, ctrlKey: ctrl }));
        },
        p,
        i > 0,
      );
      await s.browser.pause(800);
    }
    await contextMenu(b);
    const ready = await s.browser.execute(() =>
      [...document.querySelectorAll('.context-menu-item')].some((i) => i.textContent?.includes('Comparar archivos')),
    );
    if (ready) break;
    await s.browser.keys(['Escape']);
    await s.browser.pause(400);
  }
  await clickMenu('Comparar archivos');
  await s.browser.$('.view-mode-toggle').waitForExist({ timeout: 10_000 });
}

/** Pulsa el botón del panel visible cuyo texto contiene `text`; devuelve si existía. */
const clickButton = (text: string) =>
  s.browser.execute((t: string) => {
    const btn = [...document.querySelectorAll('.tab-pane:not(.hidden) button')].find((b) =>
      b.textContent?.includes(t),
    ) as HTMLElement | undefined;
    btn?.click();
    return !!btn;
  }, text) as Promise<boolean>;

const visibleText = () =>
  s.browser.execute(() => document.querySelector('.tab-pane:not(.hidden)')?.textContent ?? '') as Promise<string>;

async function expectPixelDiff() {
  expect(await clickButton('Diferencias')).toBe(true);
  await s.browser.waitUntil(async () => /píxeles distintos/.test(await visibleText()), {
    timeout: 20_000,
    timeoutMsg: 'no se calculó el diff de píxeles',
  });
  expect(await visibleText()).not.toMatch(/\b0 píxeles distintos/);
}

async function expectSideBySide(a: string, b: string) {
  expect(await clickButton('2 Paneles')).toBe(true);
  await s.browser.waitUntil(
    async () => {
      const tags = (await s.browser.execute(() =>
        [...document.querySelectorAll('.tab-pane:not(.hidden) .pane-tag')].map((t) => t.textContent?.trim() ?? ''),
      )) as string[];
      return tags.some((t) => t.includes(a)) && tags.some((t) => t.includes(b));
    },
    { timeout: 10_000, timeoutMsg: `no se ven ${a} y ${b} lado a lado` },
  );
}

it('dos Excalidraw se comparan lado a lado y píxel por píxel', async () => {
  await compare('a.excalidraw', 'b.excalidraw');
  await expectSideBySide('a.excalidraw', 'b.excalidraw');
  await expectPixelDiff();
}, 60_000);

it('dos Mermaid se comparan lado a lado, como texto y píxel por píxel', async () => {
  await compare('a.mmd', 'b.mmd');
  await expectSideBySide('a.mmd', 'b.mmd');
  expect(
    await s.browser.execute(() =>
      [...document.querySelectorAll('.tab-pane:not(.hidden) .view-mode-toggle button')].some((b) => b.textContent?.includes('Texto')),
    ),
  ).toBe(true);
  await expectPixelDiff();
}, 60_000);

it('el diff de Git de un Mermaid tiene modo Diferencias', async () => {
  await contextMenu('flujo.mmd');
  await clickMenu('Diff visual');
  await s.browser.$('.tab-pane:not(.hidden) .view-mode-toggle').waitForExist({ timeout: 10_000 });
  await expectPixelDiff();
}, 60_000);

it('el diff visual en stage compara el stage contra HEAD', async () => {
  await contextMenu('flujo.mmd');
  await clickMenu('Diff visual --staged');
  await s.browser.$('.tab-pane:not(.hidden) .view-mode-toggle').waitForExist({ timeout: 10_000 });
  expect(await clickButton('2 Paneles')).toBe(true);
  await s.browser.waitUntil(
    async () => {
      const tags = (await s.browser.execute(() =>
        [...document.querySelectorAll('.tab-pane:not(.hidden) .pane-tag')].map((t) => t.textContent?.trim() ?? ''),
      )) as string[];
      return tags.some((t) => t.includes('Stage')) && tags.some((t) => t.includes('HEAD'));
    },
    { timeout: 10_000, timeoutMsg: 'el stage no se compara contra HEAD' },
  );
  // Texto: solo el cambio del stage (Z --> W), no el que está sin preparar (W --> V)
  expect(await clickButton('Texto')).toBe(true);
  await s.browser.waitUntil(async () => (await visibleText()).includes('Z --> W'), { timeout: 10_000 });
  expect(await visibleText()).not.toContain('W --> V');
}, 60_000);

it('el historial muestra fechas y abre el parche de un commit, también en píxeles', async () => {
  await contextMenu('flujo.mmd');
  await clickMenu('Historial de Git');
  const dialog = s.browser.$('.patch-btn');
  await dialog.waitForExist({ timeout: 10_000 });
  expect(await s.browser.execute(() => document.body.textContent?.includes('Invalid Date'))).toBe(false);
  // El primero es el commit más reciente («agrega Z»)
  await s.browser.execute(() => (document.querySelector('.patch-btn') as HTMLElement).click());
  await s.browser.waitUntil(async () => (await visibleText()).includes('COMMIT:'), {
    timeout: 10_000,
    timeoutMsg: 'no se abrió el parche del commit',
  });
  expect(await clickButton('Texto')).toBe(true);
  await s.browser.waitUntil(async () => (await visibleText()).includes('Y --> Z'), { timeout: 10_000 });
  expect(await visibleText()).not.toContain('Z --> W');
  await expectPixelDiff();
}, 60_000);
