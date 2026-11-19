// Tablas en el editor de Markdown: las líneas se decoran (no se reemplazan), así que la numeración
// queda alineada con el texto de debajo y el cursor entra en la tabla fila por fila.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session, paneReady } from './harness';

const DOC = ['# Contratos', '', '| API | CUD |', '|---|---|', '| [[core.yaml]] | x |', '| otra | y |', '', 'debajo 1', 'debajo 2', ''].join('\n');

let s: Session;
beforeAll(async () => {
  s = await startApp({ 'tabla.md': DOC });
  const item = s.browser.$('.file-tree-item[data-path="tabla.md"]');
  await item.waitForExist({ timeout: 30_000 });
  await item.click();
  await paneReady(s);
  // Markdown se abre en Lectura (con un editor oculto en el DOM): F2 muestra el editor
  const shown = () =>
    s.browser.execute(() => [...document.querySelectorAll('.cm-content')].some((c) => c.getClientRects().length > 0)) as Promise<boolean>;
  if (!(await shown())) await s.browser.keys(['F2']);
  await s.browser.waitUntil(shown, { timeout: 15_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
});

const activeLine = () =>
  s.browser.execute(() => document.querySelector('.cm-content .cm-activeLine')?.textContent ?? null) as Promise<string | null>;

it('las líneas de la tabla siguen en el documento, decoradas como tabla', async () => {
  const info = (await s.browser.execute(() => ({
    table: [...document.querySelectorAll('.cm-content .cm-md-table-line')].map((l) => l.textContent),
    widget: !!document.querySelector('.cm-table-widget-container'),
  }))) as { table: string[]; widget: boolean };
  expect(info.widget).toBe(false);
  expect(info.table.length).toBe(4);
  expect(info.table[0]).toContain('API');
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/markdown-table-lines.png`);
});

it('la numeración de las líneas de debajo coincide con su texto', async () => {
  const offsets = (await s.browser.execute(() => {
    const lines = [...document.querySelectorAll('.cm-content .cm-line')] as HTMLElement[];
    const nums = [...document.querySelectorAll('.cm-lineNumbers .cm-gutterElement')].filter((g) => /^\d+$/.test(g.textContent ?? '')) as HTMLElement[];
    return [8, 9].map((n) => {
      const g = nums.find((x) => x.textContent === String(n))!;
      return Math.abs(g.getBoundingClientRect().top - lines[n - 1].getBoundingClientRect().top);
    });
  })) as number[];
  for (const d of offsets) expect(d).toBeLessThan(3);
});

it('con ↑ desde debajo de la tabla el cursor entra en su última fila', async () => {
  // Cursor al inicio de «debajo 1» desde la vista del editor (un clic sintético no es fiable)
  await s.browser.execute(() => {
    const content = [...document.querySelectorAll('.cm-content')].find((c) => c.getClientRects().length > 0) as any;
    const view = content.cmView?.view ?? content.cmTile?.view ?? content.parentElement.parentElement.cmView?.view;
    const line = view.state.doc.line(8);
    view.dispatch({ selection: { anchor: line.from } });
    view.focus();
  });

  await s.browser.waitUntil(async () => (await activeLine()) === 'debajo 1', { timeout: 15_000 });
  await s.browser.keys(['ArrowUp']);
  await s.browser.waitUntil(async () => (await activeLine()) === '', { timeout: 15_000 });
  await s.browser.keys(['ArrowUp']);
  await s.browser.waitUntil(async () => (await activeLine()) === '| otra | y |', { timeout: 15_000 });
  await s.browser.keys(['ArrowUp']);
  await s.browser.waitUntil(async () => (await activeLine())?.includes('core.yaml') ?? false, { timeout: 15_000 });
});

it('el botón «Editor visual» abre el editor de celdas', async () => {
  await s.browser.execute(() => (document.querySelector('.cm-md-table-visual-btn') as HTMLElement).click());
  await s.browser.$('.cm-table-widget-container').waitForExist({ timeout: 15_000 });
});
