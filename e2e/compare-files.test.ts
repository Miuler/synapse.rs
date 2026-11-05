// «Comparar archivos» del explorador: solo aparece con dos archivos seleccionados y abre su diff.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

let s: Session;

// Con pausas: abrir el primer archivo lo localiza en el árbol y eso reinicia la selección.
async function select(paths: string[]) {
  for (const [i, p] of paths.entries()) {
    await s.browser.execute(
      (path: string, ctrl: boolean) => {
        const el = document.querySelector(`.file-tree-item[data-path="${path}"]`)!;
        el.dispatchEvent(new MouseEvent('click', { bubbles: true, ctrlKey: ctrl }));
      },
      p,
      i > 0,
    );
    await s.browser.pause(600);
  }
}

async function menuHasCompare(path: string): Promise<boolean> {
  // Evento directo: el clic derecho de WebDriver reduce la selección al elemento pulsado.
  await s.browser.execute((p: string) => {
    const el = document.querySelector(`.file-tree-item[data-path="${p}"]`)!;
    const { x, y } = el.getBoundingClientRect();
    el.dispatchEvent(
      new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: x + 5, clientY: y + 5 }),
    );
  }, path);
  await s.browser.$('.context-menu-item').waitForExist({ timeout: 5_000 });
  await s.browser.pause(200);
  const found = (await s.browser.execute(() =>
    [...document.querySelectorAll('.context-menu-item')].some((i) => i.textContent?.includes('Comparar archivos')),
  )) as boolean;
  return found;
}

const closeMenu = async () => {
  await s.browser.keys(['Escape']);
  await s.browser.$('.context-menu-item').waitForExist({ timeout: 5_000, reverse: true });
};

beforeAll(async () => {
  s = await startApp({
    'a.txt': 'igual\nviejo\n',
    'b.txt': 'igual\nnuevo\n',
    'c.txt': 'otro\n',
  });
  await s.browser.$('.file-tree-item[data-path="a.txt"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(() => s?.close());

it('no aparece con uno o tres archivos', async () => {
  await select(['a.txt']);
  expect(await menuHasCompare('a.txt')).toBe(false);
  await closeMenu();
  await select(['a.txt', 'b.txt', 'c.txt']);
  expect(await menuHasCompare('a.txt')).toBe(false);
  await closeMenu();
});

it('con dos archivos abre el diff entre ellos', async () => {
  await select(['a.txt', 'b.txt']);
  expect(await menuHasCompare('b.txt')).toBe(true);
  await s.browser.execute(() => {
    const item = [...document.querySelectorAll('.context-menu-item')].find((i) =>
      i.textContent?.includes('Comparar archivos'),
    ) as HTMLElement;
    item.click();
  });
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
  expect(text).toContain('a.txt → b.txt');
});
