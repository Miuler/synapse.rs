// Autocompletado con las palabras del documento en el editor de D2: un nombre definido en el
// archivo aparece como sugerencia aunque el lenguaje no lo conozca.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

let s: Session;

beforeAll(async () => {
  s = await startApp({ 'arquitectura.d2': 'servidor_web -> base_datos\n' });
  const item = s.browser.$('.file-tree-item[data-path="arquitectura.d2"]');
  await item.waitForExist({ timeout: 30_000 });
  await item.click();
  await s.browser.pause(500);
  const content = s.browser.$('.cm-editor .cm-content');
  if (!(await content.isExisting())) await s.browser.keys(['F2']);
  await content.waitForExist({ timeout: 10_000 });
  await s.browser.execute(() => localStorage.setItem('synapse_settings', JSON.stringify({ vimMode: false })));
}, 120_000);

afterAll(async () => {
  await s?.close();
});

it('sugiere un nombre definido en el documento y lo inserta', async () => {
  await s.browser.$('.cm-content .cm-line').click();
  await s.browser.keys(['Control', 'End', 'Enter']);
  await s.browser.keys('serv'.split(''));
  const tooltip = s.browser.$('.cm-tooltip-autocomplete');
  await tooltip.waitForExist({ timeout: 5_000 });
  expect(await tooltip.getText()).toContain('servidor_web');
  await s.browser.keys(['Enter']);
  const text = (await s.browser.execute(() => document.querySelector('.cm-content')!.textContent)) as string;
  expect(text).toContain('servidor_web -> base_datosservidor_web');
});
