// Explorador virtualizado: con muchas notas en una carpeta solo se pintan las filas visibles, y el
// desplazamiento, «ubicar archivo» y contraer la carpeta siguen funcionando.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

const TOTAL = 400;
let s: Session;
const name = (i: number) => `nota-${String(i).padStart(3, '0')}.md`;
const rowCount = () => s.browser.execute(() => document.querySelectorAll('.file-tree-item').length) as Promise<number>;
const hasRow = (path: string) =>
  s.browser.execute((p: string) => !!document.querySelector(`.file-tree-item[data-path="${CSS.escape(p)}"]`), path) as Promise<boolean>;

beforeAll(async () => {
  const files: Record<string, string> = { 'inicio.md': '# Inicio\n' };
  for (let i = 0; i < TOTAL; i++) files[`muchas/${name(i)}`] = `# Nota ${i}\n`;
  s = await startApp(files);
  await s.browser.$('.file-tree-item.folder[data-path="muchas"]').waitForExist({ timeout: 30_000 });
  await s.browser.$('.file-tree-item.folder[data-path="muchas"]').click();
  await s.browser.$(`.file-tree-item[data-path="muchas/${name(0)}"]`).waitForExist({ timeout: 10_000 });
}, 120_000);

afterAll(async () => {
  await s?.close();
});

it('solo pinta las filas visibles', async () => {
  const rows = await rowCount();
  expect(rows).toBeGreaterThan(10);
  expect(rows).toBeLessThan(TOTAL / 2);
  expect(await hasRow(`muchas/${name(TOTAL - 1)}`)).toBe(false);
});

it('al desplazarse hasta el final aparece la última nota', async () => {
  await s.browser.execute(() => {
    const el = document.querySelector('.sidebar-content') as HTMLElement;
    el.scrollTop = el.scrollHeight;
  });
  await s.browser.waitUntil(() => hasRow(`muchas/${name(TOTAL - 1)}`), { timeout: 5_000 });
  expect(await hasRow(`muchas/${name(0)}`)).toBe(false);
});

it('ubicar archivo lleva a una nota que no estaba en el DOM', async () => {
  await s.browser.execute(() => {
    (document.querySelector('.sidebar-content') as HTMLElement).scrollTop = 0;
  });
  const target = `muchas/${name(250)}`;
  await s.browser.waitUntil(async () => !(await hasRow(target)), { timeout: 5_000 });
  // Abrir la nota con el Quick Open y ubicarla en el árbol (Alt+L).
  await s.browser.keys(['Control', 'o']);
  const input = s.browser.$('.quick-open-command-root input');
  await input.waitForExist({ timeout: 10_000 });
  await input.setValue('nota-250');
  await s.browser.waitUntil(
    async () => (await s.browser.$$('.quick-open-results-container [data-command-item]').length) > 0,
    { timeout: 10_000 },
  );
  await s.browser.keys(['Enter']);
  await s.browser.$(`.tab-title-container[data-path="${target}"]`).waitForExist({ timeout: 10_000 });
  await s.browser.keys(['Alt', 'l']);
  await s.browser.waitUntil(() => hasRow(target), { timeout: 5_000 });
  const visible = (await s.browser.execute((p: string) => {
    const row = document.querySelector(`.file-tree-item[data-path="${CSS.escape(p)}"]`)!.getBoundingClientRect();
    const view = (document.querySelector('.sidebar-content') as HTMLElement).getBoundingClientRect();
    return row.top >= view.top && row.bottom <= view.bottom;
  }, target)) as boolean;
  expect(visible).toBe(true);
});

it('contraer la carpeta deja solo las filas de primer nivel', async () => {
  await s.browser.execute(() => {
    (document.querySelector('.sidebar-content') as HTMLElement).scrollTop = 0;
  });
  await s.browser.$('.file-tree-item.folder[data-path="muchas"]').click();
  await s.browser.waitUntil(async () => (await rowCount()) === 2, { timeout: 5_000 });
});

it('el menú contextual se abre sobre una fila', async () => {
  const row = s.browser.$('.file-tree-item[data-path="inicio.md"]');
  await row.click({ button: 'right' });
  await s.browser.waitUntil(
    () => s.browser.execute(() => !!document.querySelector('[role="menu"]')) as Promise<boolean>,
    { timeout: 5_000 },
  );
  await s.browser.keys(['Escape']);
});
