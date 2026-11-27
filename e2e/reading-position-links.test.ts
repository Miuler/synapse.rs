// Al volver a lectura se conserva la posición aunque la nota tarde en pintarse (muchos enlaces e
// imágenes), y no se vuelve al destino de una navegación anterior.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { paneReady, startApp, type Session } from './harness';

const links = Array.from({ length: 150 }, (_, i) => `Ver [[otra-${i}]] y ![[falta-${i}.png]].`).join('\n\n');
// Las imágenes quedan por encima de la sección que se mira: al cargar, la empujan hacia abajo
const images = Array.from({ length: 6 }, (_, i) => `![[foto-${i}.svg]]`).join('\n\n');
const parts = Array.from({ length: 40 }, (_, i) => `## Parte ${i + 1}\n\n${`Texto de la parte ${i + 1}. `.repeat(20)}\n`).join('\n');
const svg = '<svg xmlns="http://www.w3.org/2000/svg" width="400" height="500"><rect width="400" height="500" fill="teal"/></svg>';
let s: Session;

beforeAll(async () => {
  const photos = Object.fromEntries(Array.from({ length: 6 }, (_, i) => [`foto-${i}.svg`, svg]));
  s = await startApp({
    'larga.md': `# Inicio\n\n${links}\n\n${images}\n\n${parts}`,
    'enlaces.md': `# Inicio\n\n${links}\n\n${parts}`,
    ...photos,
  });
  await open('larga.md');
}, 120_000);
afterAll(() => s?.close());

async function open(name: string) {
  const item = s.browser.$(`.file-tree-item[data-path="${name}"]`);
  await item.waitForExist({ timeout: 30_000 });
  await item.click();
  await paneReady(s);
  await s.browser.waitUntil(async () => (await headings()) === 40, { timeout: 20_000 });
}
const headings = () =>
  s.browser.execute(() => document.querySelectorAll('.tab-pane:not(.hidden) .markdown-body h2').length) as Promise<number>;
const editing = () => s.browser.execute(() => !document.querySelector('.tab-pane:not(.hidden) .markdown-body')) as Promise<boolean>;
async function toggle(toEditing: boolean) {
  await s.browser.keys('F2');
  await s.browser.waitUntil(async () => (await editing()) === toEditing, { timeout: 10_000 });
}
/** Título que está arriba de la vista, en el modo que se vea. */
const topPart = () =>
  s.browser.execute(() => {
    const pane = document.querySelector('.tab-pane:not(.hidden)')!;
    const y = pane.querySelector('.editor-wrapper')!.getBoundingClientRect().top + 16;
    const reading = pane.querySelector('.markdown-body');
    const rows = [...(reading ? reading.children : pane.querySelectorAll('.cm-line'))];
    const row = rows.find((r) => r.getClientRects().length && r.getBoundingClientRect().bottom > y);
    return row?.textContent?.trim() ?? '';
  }) as Promise<string>;
const atTop = (text: string, msg: string) =>
  s.browser.waitUntil(async () => (await topPart()).replace(/^#+\s*/, '').startsWith(text), { timeout: 8_000, timeoutMsg: msg });
/** Lleva ese título arriba, en lectura o en edición. */
const show = (text: string) =>
  s.browser.execute((text: string) => {
    const pane = document.querySelector('.tab-pane:not(.hidden)')!;
    const all = [...pane.querySelectorAll('.markdown-body h2, .cm-line')];
    all.find((e) => e.textContent?.trim().replace(/^#+\s*/, '') === text)!.scrollIntoView({ block: 'start' });
  }, text);

/** En edición solo existen las líneas cercanas: baja hasta que aparezca la del título. */
const showEditing = (text: string) =>
  s.browser.waitUntil(
    () =>
      s.browser.execute((text: string) => {
        const pane = document.querySelector('.tab-pane:not(.hidden)')!;
        const line = [...pane.querySelectorAll('.cm-line')].find((e) => e.textContent?.trim().replace(/^#+\s*/, '') === text);
        if (line) line.scrollIntoView({ block: 'start' });
        else {
          const scroller = pane.querySelector('.cm-scroller')!;
          scroller.scrollTop += scroller.clientHeight / 2;
        }
        return !!line;
      }, text) as Promise<boolean>,
    { timeout: 15_000, timeoutMsg: `no aparece ${text} editando` },
  );

it('conserva la posición al volver a lectura', async () => {
  await show('Parte 20');
  await atTop('Parte 20', 'no llegó a la parte 20');
  await s.browser.pause(300);
  await toggle(true);
  await atTop('Parte 20', 'la edición no arrancó en la parte 20');
  await s.browser.pause(300);
  await toggle(false);
  await s.browser.waitUntil(async () => (await headings()) === 40, { timeout: 20_000 });
  await atTop('Parte 20', 'la lectura no volvió a la parte 20');
  // Sigue ahí cuando las imágenes terminan de cargar
  await s.browser.pause(1_500);
  expect(await topPart()).toStartWith('Parte 20');
}, 90_000);

it('tras ir a un título, volver de edición respeta dónde se quedó', async () => {
  if (await editing()) await toggle(false);
  await open('enlaces.md');
  await s.browser.$('.tab-pane:not(.hidden) .editor-outline-btn').click();
  await s.browser.$('.heading-outline').waitForDisplayed({ timeout: 5_000 });
  await s.browser.$('.heading-outline [aria-label="Expandir todo"]').click();
  await s.browser.waitUntil(
    () =>
      s.browser.execute(() => {
        const text = [...document.querySelectorAll('.heading-outline .outline-text')].find((e) => e.textContent?.trim() === 'Parte 10');
        text?.closest('button')?.click();
        return !!text;
      }) as Promise<boolean>,
    { timeout: 5_000, timeoutMsg: 'no está en el índice' },
  );
  await atTop('Parte 10', 'el índice no llevó a la parte 10');
  await s.browser.pause(800);
  await toggle(true);
  await s.browser.pause(300);
  await showEditing('Parte 30');
  await atTop('Parte 30', 'no llegó a la parte 30 editando');
  await s.browser.pause(300);
  await toggle(false);
  await s.browser.waitUntil(async () => (await headings()) === 40, { timeout: 20_000 });
  await s.browser.pause(1_500);
  expect(await topPart()).toStartWith('Parte 30');
}, 90_000);
