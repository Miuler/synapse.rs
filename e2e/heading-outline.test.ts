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
