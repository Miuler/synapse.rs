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
  // Los diagramas se abren en vista previa: F2 muestra el editor de código.
  const content = s.browser.$('.cm-editor .cm-content');
  if (!(await content.isExisting())) await s.browser.keys(['F2']);
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

it('los atajos de Vim zc, zo, zR y zM pliegan y despliegan', async () => {
  await s.browser.execute(() => {
    localStorage.setItem('synapse_settings', JSON.stringify({ vimMode: true }));
  });
  await s.browser.refresh();
  const item = s.browser.$('.file-tree-item[data-path="arquitectura.d2"]');
  await item.waitForExist({ timeout: 30_000 });
  await item.click();
  const content = s.browser.$('.cm-editor .cm-content');
  await s.browser.pause(500);
  if (!(await content.isExisting())) await s.browser.keys(['F2']);
  await content.waitForExist({ timeout: 10_000 });
  await content.click();
  const placeholders = () => s.browser.execute(() => document.querySelectorAll('.cm-foldPlaceholder').length) as Promise<number>;
  // Cursor en la primera línea: zc pliega el bloque exterior, zo lo despliega.
  await s.browser.$('.cm-content .cm-line').click();
  await s.browser.keys(['Escape']);
  await s.browser.keys(['z', 'c']);
  await s.browser.waitUntil(async () => (await placeholders()) === 1, { timeout: 5_000 });
  await s.browser.keys(['z', 'o']);
  await s.browser.waitUntil(async () => (await placeholders()) === 0, { timeout: 5_000 });
  // zM pliega todo y zR lo despliega todo.
  await s.browser.keys(['z', 'M']);
  await s.browser.waitUntil(async () => (await placeholders()) > 0, { timeout: 5_000 });
  await s.browser.keys(['z', 'R']);
  await s.browser.waitUntil(async () => (await placeholders()) === 0, { timeout: 5_000 });
});

it('zc con el cursor dentro de un bloque lo pliega, y repetido pliega el exterior', async () => {
  const placeholders = () => s.browser.execute(() => document.querySelectorAll('.cm-foldPlaceholder').length) as Promise<number>;
  // Cursor en "shape: cylinder", dentro de db { } que a su vez está dentro de server { }.
  const inner = await s.browser.execute(
    () => [...document.querySelectorAll('.cm-content .cm-line')].find((l) => l.textContent?.includes('cylinder')) as HTMLElement,
  );
  await s.browser.$(inner).click();
  await s.browser.keys(['Escape', 'z', 'c']);
  await s.browser.waitUntil(async () => (await placeholders()) === 1, { timeout: 5_000 });
  let lines = await visibleLines();
  expect(lines.join('\n')).not.toContain('cylinder');
  expect(lines.join('\n')).toContain('shape: rectangle');
  await s.browser.keys(['z', 'c']);
  await s.browser.waitUntil(async () => !(await visibleLines()).join('\n').includes('rectangle'), { timeout: 5_000 });
  lines = await visibleLines();
  expect(lines).toContain('client -> server');
  await s.browser.keys(['z', 'R']);
});

it('zM pliega todos los niveles: zo abre el exterior y el interno sigue plegado; zO abre todo', async () => {
  const placeholders = () => s.browser.execute(() => document.querySelectorAll('.cm-foldPlaceholder').length) as Promise<number>;
  const all = async () => (await visibleLines()).join('\n');
  await s.browser.$('.cm-content .cm-line').click();
  await s.browser.keys(['Escape', 'z', 'M']);
  await s.browser.waitUntil(async () => !(await all()).includes('rectangle'), { timeout: 5_000 });
  // zo en "server: {": se ve el contenido del exterior, pero db sigue plegado.
  await s.browser.keys(['z', 'o']);
  await s.browser.waitUntil(async () => (await all()).includes('rectangle'), { timeout: 5_000 });
  expect(await all()).not.toContain('cylinder');
  expect(await placeholders()).toBe(1);
  // zM de nuevo y zO: se abre todo de una vez.
  await s.browser.keys(['z', 'M']);
  await s.browser.waitUntil(async () => !(await all()).includes('rectangle'), { timeout: 5_000 });
  await s.browser.keys(['z', 'O']);
  await s.browser.waitUntil(async () => (await all()).includes('cylinder'), { timeout: 5_000 });
  expect(await placeholders()).toBe(0);
  // zC dentro de db pliega server con db dentro: zo en server deja db plegado.
  const inner = await s.browser.execute(
    () => [...document.querySelectorAll('.cm-content .cm-line')].find((l) => l.textContent?.includes('rectangle')) as HTMLElement,
  );
  await s.browser.$(inner).click();
  await s.browser.keys(['Escape', 'z', 'C']);
  await s.browser.waitUntil(async () => !(await all()).includes('rectangle'), { timeout: 5_000 });
  await s.browser.keys(['z', 'o']);
  await s.browser.waitUntil(async () => (await all()).includes('rectangle'), { timeout: 5_000 });
  expect(await all()).not.toContain('cylinder');
  await s.browser.keys(['z', 'R']);
});
