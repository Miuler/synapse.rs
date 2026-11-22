// Índice OKF: Synapse mantiene un index.md (OKF v0.3) en cada carpeta; un index.md propio pasa a
// index.deprecado.md; al crear una nota se actualiza; en Synapse se abre en solo lectura.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { startApp, type Session } from './harness';

let s: Session;
const read = (rel: string) => (existsSync(join(s.vault, rel)) ? readFileSync(join(s.vault, rel), 'utf8') : '');

beforeAll(async () => {
  process.env.SYNAPSE_OKF_INDEX = '1';
  try {
    s = await startApp({
      'nota.md': '# Nota\n',
      'proyecto/plan.md': '---\ntype: note\ndescription: El plan del trimestre\n---\n# Plan\n',
      'proyecto/index.md': '# Índice escrito a mano\n',
    });
  } finally {
    delete process.env.SYNAPSE_OKF_INDEX;
  }
  await s.browser.$('.file-tree-item[data-path="nota.md"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
});

it('genera index.md en la raíz y en cada carpeta, y conserva el propio como deprecado', async () => {
  await s.browser.waitUntil(async () => read('proyecto/index.md').includes('okf-index'), { timeout: 20_000, timeoutMsg: 'no se generó proyecto/index.md' });
  expect(read('proyecto/index.md')).toContain('* [Plan](plan.md) - El plan del trimestre');
  expect(read('proyecto/index.deprecado.md')).toBe('# Índice escrito a mano\n');
  expect(read('index.md')).toStartWith('---\nokf_version: "0.3"\n');
  expect(read('index.md')).toContain('* [proyecto](proyecto/)');
}, 40_000);

it('al crear una nota se actualiza el índice de su carpeta', async () => {
  writeFileSync(join(s.vault, 'proyecto/nueva.md'), '# Nueva idea\n');
  await s.browser.waitUntil(async () => read('proyecto/index.md').includes('* [Nueva idea](nueva.md)'), {
    timeout: 20_000,
    timeoutMsg: 'el índice no se actualizó al crear la nota',
  });
}, 30_000);

it('en Synapse el index.md generado se abre en solo lectura', async () => {
  await s.browser.execute(() => (document.querySelector('.file-tree-item[data-path="index.md"]') as HTMLElement)?.click());
  await s.browser.$('.markdown-reading-wrapper').waitForExist({ timeout: 10_000 });
  await s.browser.keys(['F2']);
  await s.browser.pause(800);
  // Sigue en lectura: no aparece el editor
  expect(
    await s.browser.execute(() => {
      const c = document.querySelector('.tab-pane:not(.hidden) .cm-content') as HTMLElement | null;
      return !!c && c.offsetParent !== null;
    }),
  ).toBe(false);
  // Sin el botón de lectura/escritura
  expect(await s.browser.execute(() => !!document.querySelector('.view-toggle-btn'))).toBe(false);
  // Un enlace a una carpeta abre el index.md de esa carpeta
  await s.browser.execute(() => {
    const a = [...document.querySelectorAll('.markdown-reading-wrapper .markdown-body a')].find((x) => x.textContent?.trim() === 'proyecto') as HTMLElement;
    a.click();
  });
  await s.browser.waitUntil(
    async () =>
      (await s.browser.execute(() => document.querySelector('.tab-title-container.active')?.getAttribute('data-path'))) === 'proyecto/index.md',
    { timeout: 10_000, timeoutMsg: 'el enlace a la carpeta no abrió su index.md' },
  );
  // La pestaña del index.md se llama como su carpeta y lleva icono de carpeta
  const tab = (await s.browser.execute(() => {
    const t = document.querySelector('.tab-title-container[data-path="proyecto/index.md"]')!;
    return { text: t.textContent?.trim(), folderIcon: !!t.querySelector('.synapse-file-folder') };
  })) as { text: string; folderIcon: boolean };
  expect(tab.text).toContain('proyecto');
  expect(tab.text).not.toContain('index');
  expect(tab.folderIcon).toBe(true);
}, 30_000);

it('clic en una carpeta abre su index.md (la flecha la despliega); con «Desplegarla» se despliega', async () => {
  const folder = () => s.browser.$('.file-tree-item.folder[data-path="proyecto"]');
  const active = () => s.browser.execute(() => document.querySelector('.tab-title-container.active')?.getAttribute('data-path')) as Promise<string>;
  const childVisible = () => s.browser.$('.file-tree-item[data-path="proyecto/plan.md"]').isExisting();
  await s.browser.execute(() => (document.querySelector('.file-tree-item[data-path="nota.md"]') as HTMLElement).click());
  await s.browser.waitUntil(async () => (await active()) === 'nota.md', { timeout: 10_000 });
  // Partir con la carpeta plegada
  if (await childVisible()) {
    await s.browser.execute(() => (document.querySelector('.file-tree-item.folder[data-path="proyecto"] .chevron-toggle') as HTMLElement).click());
    await s.browser.waitUntil(async () => !(await childVisible()), { timeout: 5_000 });
  }
  // Clic en la carpeta: su índice, sin desplegarla
  await s.browser.execute(() => (document.querySelector('.file-tree-item.folder[data-path="proyecto"]') as HTMLElement).click());
  await s.browser.waitUntil(async () => (await active()) === 'proyecto/index.md', { timeout: 10_000, timeoutMsg: 'el clic no abrió el index.md' });
  expect(await childVisible()).toBe(false);
  // La flecha la despliega
  await s.browser.execute(() => (document.querySelector('.file-tree-item.folder[data-path="proyecto"] .chevron-toggle') as HTMLElement).click());
  await s.browser.waitUntil(childVisible, { timeout: 5_000, timeoutMsg: 'la flecha no desplegó la carpeta' });
  // Desplegada, su index.md no se lista dentro (la carpeta ya lo abre)
  expect(await s.browser.$('.file-tree-item[data-path="proyecto/index.md"]').isExisting()).toBe(false);
  await s.browser.execute(() => (document.querySelector('.file-tree-item.folder[data-path="proyecto"] .chevron-toggle') as HTMLElement).click());
  await s.browser.waitUntil(async () => !(await childVisible()), { timeout: 5_000 });
  // Ajuste «Desplegarla»: el clic despliega (como antes)
  await s.browser.execute(() => {
    const raw = JSON.parse(localStorage.getItem('synapse_settings') ?? '{}');
    localStorage.setItem('synapse_settings', JSON.stringify({ ...raw, folderClick: 'expand' }));
  });
  await s.browser.refresh();
  await folder().waitForExist({ timeout: 30_000 });
  await s.browser.execute(() => (document.querySelector('.file-tree-item.folder[data-path="proyecto"]') as HTMLElement).click());
  await s.browser.waitUntil(childVisible, { timeout: 5_000, timeoutMsg: 'con «Desplegarla» no se desplegó' });
}, 60_000);

it('el índice de una carpeta tiene «+ Nuevo…» y «+ Carpeta» para crear dentro de ella', async () => {
  const active = () => s.browser.execute(() => document.querySelector('.tab-title-container.active')?.getAttribute('data-path')) as Promise<string>;
  await s.browser.execute(() => {
    const raw = JSON.parse(localStorage.getItem('synapse_settings') ?? '{}');
    localStorage.setItem('synapse_settings', JSON.stringify({ ...raw, folderClick: 'index' }));
  });
  await s.browser.refresh();
  await s.browser.$('.file-tree-item.folder[data-path="proyecto"]').waitForExist({ timeout: 30_000 });
  await s.browser.execute(() => (document.querySelector('.file-tree-item.folder[data-path="proyecto"]') as HTMLElement).click());
  await s.browser.waitUntil(async () => (await active()) === 'proyecto/index.md', { timeout: 10_000 });
  await s.browser.$('.folder-index-new').waitForExist({ timeout: 10_000, timeoutMsg: 'no aparecen los botones del índice' });
  // Nueva nota dentro de la carpeta
  await s.browser.execute(() => {
    const sel = document.querySelector('.tab-pane:not(.hidden) .folder-index-new') as HTMLSelectElement;
    sel.value = 'markdown';
    sel.dispatchEvent(new Event('change', { bubbles: true }));
  });
  await s.browser.waitUntil(async () => /^proyecto\/Nuevo Archivo \d+\.md$/.test(await active()), {
    timeout: 10_000,
    timeoutMsg: 'no creó la nota dentro de la carpeta',
  });
  // Nueva subcarpeta (el nombre se pide con prompt)
  await s.browser.execute(() => (document.querySelector('.file-tree-item.folder[data-path="proyecto"]') as HTMLElement).click());
  await s.browser.waitUntil(async () => (await active()) === 'proyecto/index.md', { timeout: 10_000 });
  await s.browser.$('.tab-pane:not(.hidden) .folder-index-new-folder').waitForExist({ timeout: 10_000 });
  await s.browser.execute(() => {
    window.prompt = () => 'nueva-sub';
    (document.querySelector('.tab-pane:not(.hidden) .folder-index-new-folder') as HTMLElement).click();
  });
  await s.browser.waitUntil(async () => existsSync(join(s.vault, 'proyecto/nueva-sub')), { timeout: 10_000, timeoutMsg: 'no creó la subcarpeta' });
}, 60_000);
