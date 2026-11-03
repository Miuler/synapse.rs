// Archivos .puml: se renderizan en el servidor PlantUML configurado (aquí uno falso local), los errores
// marcan la línea en el editor, el SVG recibido se limpia de scripts y se pueden exportar.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { decode } from 'plantuml-encoder';
import { startApp, type Session } from './harness';

let s: Session;

/** Servidor falso: devuelve el código recibido en un SVG, o un error de sintaxis si contiene MAL. */
const server = Bun.serve({
  port: 0,
  fetch(req) {
    const encoded = new URL(req.url).pathname.split('/svg/')[1];
    const code = decode(encoded);
    const svgHeaders = { 'content-type': 'image/svg+xml' };
    if (code.includes('MAL')) {
      const line = code.split('\n').findIndex((l) => l.includes('MAL')) + 1;
      return new Response('<svg xmlns="http://www.w3.org/2000/svg"/>', {
        status: 400,
        headers: { ...svgHeaders, 'x-plantuml-diagram-error': 'Syntax Error?', 'x-plantuml-diagram-error-line': String(line) },
      });
    }
    const text = code.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
    return new Response(
      `<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 100" width="400" height="100" onload="window.__pwned=1"><script>window.__pwned=1</script><a href="https://example.test"><text class="recibido" x="5" y="50" font-size="12">${text}</text></a></svg>`,
      { headers: svgHeaders },
    );
  },
});

beforeAll(async () => {
  s = await startApp({ 'secuencia.puml': '@startuml\nAlice -> Bob: Hola\n@enduml\n', 'suelto.plantuml': 'Ana -> Luis\n' });
  await s.browser.$('.file-tree-item[data-path="secuencia.puml"]').waitForExist({ timeout: 30_000 });
  await s.browser.execute((url) => {
    localStorage.setItem('synapse_settings', JSON.stringify({ plantumlServer: url }));
  }, `http://127.0.0.1:${server.port}`);
  await s.browser.refresh();
  await s.browser.$('.file-tree-item[data-path="secuencia.puml"]').waitForExist({ timeout: 30_000 });
}, 120_000);

afterAll(async () => {
  await s?.close();
  server.stop(true);
});

const received = () =>
  s.browser.execute(() => [...document.querySelectorAll('svg text.recibido')].map((t) => t.textContent).join('\n')) as Promise<string>;

it('muestra el diagrama devuelto por el servidor y limpia el SVG', async () => {
  await s.browser.$('.file-tree-item[data-path="secuencia.puml"]').click();
  await s.browser.waitUntil(async () => (await received()).includes('Alice -> Bob'), { timeout: 15_000 });
  const clean = await s.browser.execute(() => ({
    pwned: (window as any).__pwned ?? null,
    scripts: document.querySelectorAll('svg script').length,
    links: document.querySelectorAll('svg a').length,
  }));
  expect(clean).toEqual({ pwned: null, scripts: 0, links: 0 } as any);
}, 60_000);

it('al editar se vuelve a renderizar y un error marca el mensaje del servidor', async () => {
  const content = s.browser.$('.cm-editor .cm-content');
  if (!(await content.isExisting())) await s.browser.keys(['F2']);
  await content.waitForExist({ timeout: 10_000 });
  await s.browser.$('.cm-content .cm-line').click();
  await s.browser.keys(['End', 'Enter']);
  // Sin espacios ni letras repetidas: WebDriver las junta dentro de una misma secuencia.
  await s.browser.keys('Bob->Ana'.split(''));
  await s.browser.waitUntil(async () => (await received()).includes('Bob->Ana'), {
    timeout: 15_000,
    timeoutMsg: 'sin render tras editar',
  });

  await s.browser.keys('MAL'.split(''));
  await s.browser.waitUntil(
    () => s.browser.execute(() => document.body.textContent?.includes('Syntax Error?')) as Promise<boolean>,
    { timeout: 15_000, timeoutMsg: 'no apareció el error' },
  );
  // El @startuml de la línea 1 y lo escrito en la 2: el diagnóstico cae en la línea 2.
  await s.browser.waitUntil(
    () => s.browser.execute(() => document.querySelectorAll('.cm-lintRange-error, .cm-lint-marker-error').length > 0) as Promise<boolean>,
    { timeout: 10_000, timeoutMsg: 'sin diagnóstico en el editor' },
  );
  for (let i = 0; i < 3; i++) await s.browser.keys(['Backspace']);
  await s.browser.waitUntil(
    () => s.browser.execute(() => !document.body.textContent?.includes('Syntax Error?')) as Promise<boolean>,
    { timeout: 15_000, timeoutMsg: 'no apareció el error' },
  );
}, 60_000);

it('código sin @startuml también se renderiza', async () => {
  await s.browser.$('.file-tree-item[data-path="suelto.plantuml"]').click();
  await s.browser.waitUntil(async () => (await received()).includes('Ana -> Luis'), { timeout: 15_000 });
  expect(await received()).toContain('@startuml');
}, 60_000);

const PNG_SIGNATURE = Buffer.from([0x89, 0x50, 0x4e, 0x47]);

it('exporta a SVG y PNG desde el explorador', async () => {
  for (const [label, rel] of [['Exportar como SVG', 'suelto.svg'], ['Exportar como PNG', 'suelto.png']]) {
    await s.browser.$('.file-tree-item[data-path="suelto.plantuml"]').click({ button: 'right' });
    const item = s.browser.$(`//div[@role="menuitem"][contains(., "${label}")]`);
    await item.waitForExist({ timeout: 5_000 });
    await item.click();
    await s.browser.waitUntil(async () => existsSync(join(s.vault, rel)), { timeout: 20_000 });
  }
  const svg = readFileSync(join(s.vault, 'suelto.svg'), 'utf8');
  expect(svg).toContain('<svg');
  expect(svg).not.toContain('<script');
  expect(readFileSync(join(s.vault, 'suelto.png')).subarray(0, 4).equals(PNG_SIGNATURE)).toBe(true);
}, 60_000);
