// Plegado entre llaves en el editor de código de D2: el margen ofrece plegar el bloque y al
// hacerlo sus líneas se ocultan tras el marcador.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

let s: Session;
const visibleLines = () =>
  s.browser.execute(() => [...document.querySelectorAll('.cm-content .cm-line')].map((l) => l.textContent)) as Promise<string[]>;

beforeAll(async () => {
  s = await startApp({ 'arquitectura.d2': 'server: {\n  shape: rectangle\n  db: {\n    shape: cylinder\n  }\n}\nclient -> server\n' });
  const item = s.browser.$('.file-tree-item[data-path="arquitectura.d2"]');
  await item.waitForExist({ timeout: 30_000 });
  await item.click();
  await s.browser.$('.tab-title-container[data-path="arquitectura.d2"]').waitForExist({ timeout: 10_000 });
  // Los diagramas se abren en vista previa: Ctrl+E muestra el editor de código.
  const content = s.browser.$('.cm-editor .cm-content');
  if (!(await content.isExisting())) await s.browser.keys(['Control', 'e']);
  await content.waitForExist({ timeout: 10_000 });
}, 120_000);

afterAll(async () => {
  await s?.close();
});

it('el margen ofrece plegar los bloques con llaves', async () => {
  const markers = (await s.browser.execute(
    () => document.querySelectorAll('.cm-foldGutter span[title="Plegar"]').length,
  )) as number;
  expect(markers).toBe(2);
});

it('plegar el bloque exterior oculta sus líneas', async () => {
  const marker = s.browser.$('.cm-foldGutter span[title="Plegar"]');
  await marker.click();
  await s.browser.$('.cm-foldPlaceholder').waitForExist({ timeout: 5_000 });
  const lines = await visibleLines();
  expect(lines[0]).toContain('server: {');
  expect(lines[0]).toContain('}');
  expect(lines.join('\n')).not.toContain('shape: cylinder');
  expect(lines).toContain('client -> server');
});
