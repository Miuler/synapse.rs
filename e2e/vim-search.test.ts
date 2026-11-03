// Búsqueda de Vim (`/`) en el editor de código: el texto escrito en el cuadro debe verse (color
// distinto del fondo) en tema claro y oscuro.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

let s: Session;

beforeAll(async () => {
  s = await startApp({ 'diagrama.mmd': 'graph TD\n  A --> B\n  B --> C\n' });
  await s.browser.$('.file-tree-item[data-path="diagrama.mmd"]').waitForExist({ timeout: 30_000 });
}, 120_000);

afterAll(async () => {
  await s?.close();
});

async function vimSearchPanel(theme: 'light' | 'dark') {
  await s.browser.execute((t: string) => {
    localStorage.setItem('synapse_settings', JSON.stringify({ vimMode: true, theme: t }));
  }, theme);
  await s.browser.refresh();
  const item = s.browser.$('.file-tree-item[data-path="diagrama.mmd"]');
  await item.waitForExist({ timeout: 30_000 });
  await item.click();
  // Los diagramas se abren en vista previa: Ctrl+E muestra el editor de código.
  await s.browser.$('.tab-title-container[data-path="diagrama.mmd"]').waitForExist({ timeout: 10_000 });
  const content = s.browser.$('.cm-editor .cm-content');
  if (!(await content.isExisting())) await s.browser.keys(['Control', 'e']);
  await content.waitForExist({ timeout: 10_000 });
  await content.click();
  await s.browser.keys(['Escape', '/']);
  const input = s.browser.$('.cm-vim-panel input');
  await input.waitForExist({ timeout: 5_000 });
  await s.browser.keys('graph'.split(''));
  return (await s.browser.execute(() => {
    const input = document.querySelector('.cm-vim-panel input') as HTMLInputElement;
    const panel = document.querySelector('.cm-vim-panel') as HTMLElement;
    let bgEl: HTMLElement | null = panel;
    let bg = 'rgba(0, 0, 0, 0)';
    while (bgEl && (bg === 'rgba(0, 0, 0, 0)' || bg === 'transparent')) {
      bg = getComputedStyle(bgEl).backgroundColor;
      bgEl = bgEl.parentElement;
    }
    return { value: input.value, color: getComputedStyle(input).color, bg, width: input.getBoundingClientRect().width };
  })) as { value: string; color: string; bg: string; width: number };
}

for (const theme of ['light', 'dark'] as const) {
  it(`el texto de la búsqueda de Vim se ve (tema ${theme})`, async () => {
    const r = await vimSearchPanel(theme);
    expect(r.value).toBe('graph');
    expect(r.width).toBeGreaterThan(50);
    expect(r.color).not.toBe(r.bg);
    await s.browser.keys(['Escape']);
  });
}
