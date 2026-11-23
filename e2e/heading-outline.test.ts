// Clic en el título de la nota: índice de títulos contraído (solo primer nivel), expandir/contraer
// todo e ir a un subtítulo.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

let s: Session;
const filler = Array.from({ length: 80 }, (_, i) => `línea ${i}`).join('\n\n');

beforeAll(async () => {
  s = await startApp({
    'guia.md': `# Inicio\n\n## Parte A\n\n${filler}\n\n### Detalle A1\n\ntexto final\n\n${filler}\n\n# Cierre\n\n## Parte B\n\n${filler}\n`,
  });
  await s.browser.$('.file-tree-item[data-path="guia.md"]').waitForExist({ timeout: 30_000 });
  await s.browser.$('.file-tree-item[data-path="guia.md"]').click();
  await s.browser.$('.editor-title-input').waitForExist({ timeout: 10_000 });
}, 120_000);
afterAll(() => s?.close());

const headings = () =>
  s.browser.execute(() =>
    [...document.querySelectorAll('.heading-outline .outline-text')].map((e) => e.textContent),
  ) as Promise<string[]>;

it('clic en el título abre el índice contraído; expandir y contraer todo', async () => {
  await s.browser.$('.editor-title-input').click();
  await s.browser.$('.heading-outline').waitForDisplayed({ timeout: 5_000 });
  expect(await headings()).toEqual(['Inicio', 'Cierre']);
  await s.browser.$('.heading-outline [aria-label="Expandir todo"]').click();
  expect(await headings()).toEqual(['Inicio', 'Parte A', 'Detalle A1', 'Cierre', 'Parte B']);
  await s.browser.$('.heading-outline [aria-label="Contraer todo"]').click();
  expect(await headings()).toEqual(['Inicio', 'Cierre']);
});

it('elegir un subtítulo cierra el índice y lleva a él', async () => {
  await s.browser.$('.heading-outline [aria-label="Expandir todo"]').click();
  await s.browser.execute(() => {
    const btn = [...document.querySelectorAll('.heading-outline .outline-heading')].find((b) => b.textContent?.includes('Detalle A1'));
    (btn as HTMLElement).click();
  });
  await s.browser.waitUntil(async () => !(await s.browser.$('.heading-outline').isExisting()), { timeout: 5_000 });
  await s.browser.waitUntil(
    () =>
      s.browser.execute(() => {
        const el = [...document.querySelectorAll('.tab-pane:not(.hidden) h3, .tab-pane:not(.hidden) .cm-line')].find((e) =>
          e.textContent?.includes('Detalle A1'),
        );
        if (!el) return false;
        const r = el.getBoundingClientRect();
        return r.top >= 0 && r.top < window.innerHeight;
      }) as Promise<boolean>,
    { timeout: 5_000, timeoutMsg: 'el subtítulo no quedó a la vista' },
  );
});

const headingPath = () =>
  s.browser.execute(() =>
    [...document.querySelectorAll('.tab-pane:not(.hidden) .editor-heading-crumb')].map((e) => e.textContent?.trim()),
  ) as Promise<string[]>;

it('junto al título se ve la ruta de títulos de donde está la vista', async () => {
  await s.browser.waitUntil(async () => (await headingPath()).join('>') === 'Inicio>Parte A>Detalle A1', {
    timeout: 5_000,
    timeoutMsg: 'ruta de títulos: no llegó a Inicio>Parte A>Detalle A1',
  });
  // Al hacer clic en un tramo se va a ese título
  await s.browser.execute(() => {
    const crumb = [...document.querySelectorAll('.tab-pane:not(.hidden) .editor-heading-crumb')].find(
      (e) => e.textContent?.trim() === 'Inicio',
    );
    (crumb as HTMLElement).click();
  });
  await s.browser.waitUntil(async () => (await headingPath()).join('>') === 'Inicio', {
    timeout: 5_000,
    timeoutMsg: 'tras el clic la ruta no volvió a Inicio',
  });
});

const outlineState = () =>
  s.browser.execute(() => ({
    items: [...document.querySelectorAll('.heading-outline .outline-text')].map((e) => e.textContent),
    current: document.querySelector('.heading-outline .outline-heading.current .outline-text')?.textContent ?? null,
  })) as Promise<{ items: string[]; current: string | null }>;

it('la búsqueda rápida filtra títulos y Enter va al primero', async () => {
  await s.browser.$('.editor-outline-btn').click();
  await s.browser.$('.heading-outline .outline-search-input').waitForDisplayed({ timeout: 5_000 });
  // Sin letras dobles: WebKitWebDriver a veces pierde la segunda de un par repetido
  await s.browser.keys('detal');
  await s.browser.waitUntil(async () => (await outlineState()).items.join('>') === 'Inicio>Parte A>Detalle A1', {
    timeout: 5_000,
    timeoutMsg: 'la búsqueda no filtró los títulos',
  });
  await s.browser.keys('Enter');
  await s.browser.waitUntil(async () => (await headingPath()).join('>') === 'Inicio>Parte A>Detalle A1', {
    timeout: 5_000,
    timeoutMsg: 'Enter no llevó a Detalle A1',
  });
  const text = await s.browser.execute(() => document.querySelector('.tab-pane:not(.hidden) .editor-heading-path')?.textContent?.replace(/\s+/g, ' ').trim());
  expect(text).toBe("(Inicio›Parte A›Detalle A1)");
});

it('el índice abre desplegado en la ruta actual y resalta el título', async () => {
  await s.browser.$('.editor-outline-btn').click();
  await s.browser.$('.heading-outline').waitForDisplayed({ timeout: 5_000 });
  const state = await outlineState();
  expect(state.items).toEqual(['Inicio', 'Parte A', 'Detalle A1', 'Cierre']);
  expect(state.current).toBe('Detalle A1');
  await s.browser.keys('Escape');
});

it('Ctrl+F12 abre y cierra el índice de la nota en un diálogo', async () => {
  if (await s.browser.$('.heading-outline').isExisting()) await s.browser.keys(['Escape']);
  await s.browser.waitUntil(async () => !(await s.browser.$('.heading-outline').isExisting()), { timeout: 5_000 });
  await s.browser.keys(['Control', 'F12']);
  await s.browser.$('.outline-dialog .heading-outline').waitForDisplayed({ timeout: 5_000 });
  await s.browser.keys(['Control', 'F12']);
  await s.browser.waitUntil(async () => !(await s.browser.$('.heading-outline').isExisting()), { timeout: 5_000 });
});

it('las flechas recorren las entradas visibles, → despliega y Enter va a la elegida', async () => {
  await s.browser.keys(['Control', 'F12']);
  await s.browser.$('.outline-dialog .heading-outline').waitForDisplayed({ timeout: 5_000 });
  await s.browser.$('.heading-outline [aria-label="Contraer todo"]').click();
  await s.browser.$('.heading-outline .outline-search-input').click();
  const active = () =>
    s.browser.execute(() => document.querySelector('.heading-outline .outline-heading.active .outline-text')?.textContent ?? null) as Promise<string | null>;
  // Contraído: la activa (Detalle A1) ya no se ve, así que ↓ empieza por la primera
  await s.browser.keys('ArrowDown');
  expect(await active()).toBe('Inicio');
  await s.browser.keys('ArrowDown');
  expect(await active()).toBe('Cierre');
  await s.browser.keys('ArrowRight');
  await s.browser.keys('ArrowDown');
  expect(await active()).toBe('Parte B');
  await s.browser.keys('ArrowUp');
  await s.browser.keys('ArrowUp');
  expect(await active()).toBe('Inicio');
  await s.browser.keys('ArrowDown');
  await s.browser.keys('ArrowDown');
  await s.browser.keys('Enter');
  await s.browser.waitUntil(async () => (await headingPath()).join('>') === 'Cierre>Parte B', {
    timeout: 5_000,
    timeoutMsg: 'Enter no llevó a Parte B',
  });
});

async function dragResizeHandle(selector: string, dx: number, dy: number) {
  await s.browser.execute(
    (sel: string, deltaX: number, deltaY: number) => {
      const handle = document.querySelector(sel) as HTMLElement;
      if (!handle) throw new Error(`No existe handle: ${sel}`);
      const r = handle.getBoundingClientRect();
      const x = r.left + r.width / 2;
      const y = r.top + r.height / 2;
      handle.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 0, clientX: x, clientY: y }));
      window.dispatchEvent(new PointerEvent('pointermove', { clientX: x + deltaX, clientY: y + deltaY }));
      window.dispatchEvent(new PointerEvent('pointerup', { clientX: x + deltaX, clientY: y + deltaY }));
    },
    selector,
    dx,
    dy,
  );
}

it('el popover del índice se redimensiona arrastrando su esquina y recuerda el tamaño', async () => {
  await s.browser.execute(() => localStorage.removeItem('synapse.outlinePopover.size'));
  if (await s.browser.$('.heading-outline').isExisting()) await s.browser.keys(['Escape']);
  await s.browser.waitUntil(async () => !(await s.browser.$('.heading-outline').isExisting()), { timeout: 5_000 });

  await s.browser.$('.editor-outline-btn').click();
  await s.browser.$('.heading-outline .dialog-resize.se').waitForDisplayed({ timeout: 5_000 });

  const before = (await s.browser.execute(() => {
    const r = document.querySelector('.heading-outline')!.getBoundingClientRect();
    return { width: r.width, height: r.height };
  })) as { width: number; height: number };

  await dragResizeHandle('.heading-outline .dialog-resize.se', 120, 90);

  const after = (await s.browser.execute(() => {
    const r = document.querySelector('.heading-outline')!.getBoundingClientRect();
    return { width: r.width, height: r.height };
  })) as { width: number; height: number };

  expect(after.width).toBeGreaterThan(before.width + 80);
  expect(after.height).toBeGreaterThan(before.height + 50);

  await s.browser.keys(['Escape']);
  await s.browser.waitUntil(async () => !(await s.browser.$('.heading-outline').isExisting()), { timeout: 5_000 });

  await s.browser.$('.editor-outline-btn').click();
  await s.browser.$('.heading-outline').waitForDisplayed({ timeout: 5_000 });

  const reopened = (await s.browser.execute(() => {
    const r = document.querySelector('.heading-outline')!.getBoundingClientRect();
    return { width: r.width, height: r.height };
  })) as { width: number; height: number };

  expect(Math.round(reopened.width)).toBe(Math.round(after.width));
  expect(Math.round(reopened.height)).toBe(Math.round(after.height));

  await s.browser.keys(['Escape']);
  await s.browser.waitUntil(async () => !(await s.browser.$('.heading-outline').isExisting()), { timeout: 5_000 });
});

it('el diálogo Ctrl+F12 del índice se redimensiona arrastrando un borde y recuerda el tamaño', async () => {
  await s.browser.execute(() => localStorage.removeItem('synapse.outlineDialog.size'));
  await s.browser.keys(['Control', 'F12']);
  await s.browser.$('.outline-dialog .dialog-resize.e').waitForDisplayed({ timeout: 5_000 });
  await Bun.sleep(250);

  const beforeWidth = (await s.browser.execute(
    () => document.querySelector('.outline-dialog')!.getBoundingClientRect().width,
  )) as number;

  await dragResizeHandle('.outline-dialog .dialog-resize.e', 60, 0);

  const afterWidth = (await s.browser.execute(
    () => document.querySelector('.outline-dialog')!.getBoundingClientRect().width,
  )) as number;

  expect(afterWidth).toBeGreaterThan(beforeWidth + 80);

  await s.browser.keys(['Control', 'F12']);
  await s.browser.waitUntil(async () => !(await s.browser.$('.outline-dialog').isExisting()), { timeout: 5_000 });

  await s.browser.keys(['Control', 'F12']);
  await s.browser.$('.outline-dialog').waitForDisplayed({ timeout: 5_000 });
  await Bun.sleep(250);

  const reopenedWidth = (await s.browser.execute(
    () => document.querySelector('.outline-dialog')!.getBoundingClientRect().width,
  )) as number;

  expect(Math.round(reopenedWidth)).toBe(Math.round(afterWidth));
});
