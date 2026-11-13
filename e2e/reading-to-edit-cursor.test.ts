// Al pasar de lectura a edición, el cursor queda en la primera línea que se veía en lectura,
// y al volver a lectura se conserva la parte del documento que se veía al editar.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

let s: Session;
const body = Array.from({ length: 80 }, (_, i) => `línea ${i}`).join('\n\n');

beforeAll(async () => {
  s = await startApp({ 'larga.md': `---\ntags: [x]\n---\n# Inicio\n\n${body}\n\n## Final\n\n${body}\n` });
  await s.browser.$('.file-tree-item[data-path="larga.md"]').waitForExist({ timeout: 30_000 });
  await s.browser.$('.file-tree-item[data-path="larga.md"]').click();
  await s.browser.$('.markdown-body').waitForExist({ timeout: 10_000 });
}, 120_000);
afterAll(() => s?.close());

async function setMode(mode: 'live' | 'reading') {
  await s.browser.$('.md-view-btn').click();
  await s.browser.$(`.view-item.mode-${mode}`).click();
}

const cursorLineText = () =>
  s.browser.execute(() => {
    const sel = window.getSelection();
    const line = sel?.anchorNode && (sel.anchorNode.nodeType === 1 ? (sel.anchorNode as Element) : sel.anchorNode.parentElement)?.closest('.cm-line');
    return line?.textContent ?? null;
  }) as Promise<string | null>;

it('el cursor va a la primera línea visible de la lectura', async () => {
  await s.browser.execute(() => {
    const p = [...document.querySelectorAll('.markdown-body p')].find((e) => e.textContent?.trim() === 'línea 40');
    p?.scrollIntoView({ block: 'start' });
  });
  await Bun.sleep(300);
  await setMode('live');
  await s.browser.waitUntil(async () => (await cursorLineText()) === 'línea 40', {
    timeout: 5_000,
    timeoutMsg: 'el cursor no quedó en «línea 40»',
  });
});

it('con texto repetido usa el título bajo el que estaba', async () => {
  await setMode('reading');
  await s.browser.$('.markdown-body').waitForExist({ timeout: 10_000 });
  await s.browser.execute(() => {
    const final = [...document.querySelectorAll('.markdown-body h2')].find((e) => e.textContent?.trim() === 'Final');
    let el = final?.nextElementSibling;
    for (let i = 0; i < 10 && el; i++) el = el.nextElementSibling; // «línea 10» bajo «Final»
    el?.scrollIntoView({ block: 'start' });
  });
  await Bun.sleep(300);
  await setMode('live');
  await s.browser.waitUntil(async () => (await cursorLineText()) === 'línea 10', { timeout: 5_000 });
  // Es la segunda «línea 10» (bajo «Final», línea 188 del archivo), no la primera (línea 26)
  const pos = await s.browser.execute(() =>
    [...document.querySelectorAll('span')].map((e) => e.textContent ?? '').find((t) => /^Lín \d+, Col \d+$/.test(t)),
  );
  expect(pos).toBe('Lín 188, Col 1');
});

it('al volver a lectura conserva la posición de la edición', async () => {
  // Seguimos en edición con «línea 10» bajo «Final» arriba
  await s.browser.execute(() => {
    const line = [...document.querySelectorAll('.cm-line')].find((e) => e.textContent === 'línea 30');
    line?.scrollIntoView({ block: 'start' });
  });
  await Bun.sleep(300);
  await setMode('reading');
  await s.browser.waitUntil(
    async () =>
      (await s.browser.execute(() => {
        const root = document.querySelector('.markdown-body');
        if (!root) return null;
        const top = root.closest('.editor-wrapper')!.getBoundingClientRect().top + 4;
        const first = [...root.children].find((b) => b.getBoundingClientRect().bottom > top);
        // Debe ser la «línea 30» bajo «Final», no la del principio
        const final = [...root.querySelectorAll('h2')].find((e) => e.textContent?.trim() === 'Final');
        const after = final && first ? final.compareDocumentPosition(first) & Node.DOCUMENT_POSITION_FOLLOWING : 0;
        return after ? first?.textContent?.trim() : null;
      })) === 'línea 30',
    { timeout: 5_000, timeoutMsg: 'la lectura no quedó en «línea 30» bajo «Final»' },
  );
});
