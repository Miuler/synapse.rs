// Al cambiar de modo en medio de una tabla larga se conserva la fila visible, no el principio de la tabla.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { paneReady, startApp, type Session } from './harness';

const table = (name: string) =>
  [`| ${name} | Valor |`, '|---|---|', ...Array.from({ length: 80 }, (_, i) => `| ${name}-${i + 1} | igual |`)].join('\n');
const NOTE = `# Nota\n\nTexto.\n\n${table('uno')}\n\n## Otra\n\n${table('dos')}\n\nFin.\n`;
let s: Session;

beforeAll(async () => {
  s = await startApp({ 'nota.md': NOTE });
  const item = s.browser.$('.file-tree-item[data-path="nota.md"]');
  await item.waitForExist({ timeout: 30_000 });
  await item.click();
  await paneReady(s);
  await s.browser.waitUntil(() => s.browser.execute(() => document.querySelectorAll('.markdown-body table').length === 2) as Promise<boolean>, { timeout: 15_000 });
}, 120_000);
afterAll(() => s?.close());

/** Número de la primera fila visible de la tabla `name`, en el modo que se esté viendo. */
const firstRow = (name: string) =>
  s.browser.execute((n: string) => {
    const pane = document.querySelector('.tab-pane:not(.hidden)')!;
    const top = pane.querySelector('.editor-wrapper')!.getBoundingClientRect().top + 6;
    const reading = pane.querySelector('.markdown-body');
    const rows = reading ? reading.querySelectorAll('tr') : pane.querySelectorAll('.cm-line');
    const row = [...rows].find((r) => r.getClientRects().length && r.getBoundingClientRect().bottom > top);
    return Number(new RegExp(`${n}-(\\d+)`).exec(row?.textContent ?? '')?.[1] ?? -1);
  }, name) as Promise<number>;

const editing = () => s.browser.execute(() => !document.querySelector('.tab-pane:not(.hidden) .markdown-body')) as Promise<boolean>;
async function toggle(toEditing: boolean) {
  await s.browser.keys('F2');
  await s.browser.waitUntil(async () => (await editing()) === toEditing, { timeout: 10_000 });
}
const near = (name: string, row: number) =>
  s.browser.waitUntil(async () => Math.abs((await firstRow(name)) - row) <= 2, {
    timeout: 5_000,
    timeoutMsg: `no quedó cerca de ${name}-${row}`,
  });

for (const [name, row] of [['uno', 40], ['dos', 55]] as const) {
  it(`lectura → edición → lectura en la fila ${row} de la tabla «${name}»`, async () => {
    if (await editing()) await toggle(false);
    await s.browser.execute((text: string) => {
      const tr = [...document.querySelectorAll('.markdown-body tr')].find((r) => r.textContent?.includes(text))!;
      tr.scrollIntoView({ block: 'start' });
    }, `${name}-${row}`);
    await near(name, row);
    await s.browser.pause(200);
    await toggle(true);
    await near(name, row);
    await s.browser.pause(300);
    await toggle(false);
    await near(name, row);
  }, 60_000);
}

it('fuera de una tabla sigue igual', async () => {
  if (await editing()) await toggle(false);
  await s.browser.execute(() => document.querySelector('.markdown-body h2')!.scrollIntoView({ block: 'start' }));
  await s.browser.pause(300);
  await toggle(true);
  await s.browser.waitUntil(
    () =>
      s.browser.execute(() => {
        const pane = document.querySelector('.tab-pane:not(.hidden)')!;
        const top = pane.querySelector('.editor-wrapper')!.getBoundingClientRect().top + 14;
        return [...pane.querySelectorAll('.cm-line')].find((l) => l.getBoundingClientRect().bottom > top)?.textContent?.includes('Otra') ?? false;
      }) as Promise<boolean>,
    { timeout: 5_000, timeoutMsg: 'no arrancó en el título' },
  );
}, 60_000);
