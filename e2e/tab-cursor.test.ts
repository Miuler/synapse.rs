// Al cambiar de pestaña y volver, el editor conserva el cursor y recupera el foco; y si el archivo
// cambia en disco, el contenido nuevo se aplica sin mandar el cursor al inicio.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { startApp, type Session } from './harness';

/** Diagramas largos (cientos de líneas, con scroll) para que un salto del cursor se note. */
function longD2(): string {
  const parts = ['direction: right', 'vars: {', '  d2-config: {', '    layout-engine: elk', '  }', '}', ''];
  for (let i = 0; i < 40; i++) {
    parts.push(`servicio_${i}: {`, `  label: "Servicio ${i}"`, '  shape: rectangle', `  api_${i}: {`, '    shape: hexagon', '  }', `  db_${i}: {`, '    shape: cylinder', '  }', `  api_${i} -> db_${i}: consulta`, '}', '');
  }
  for (let i = 0; i < 39; i++) parts.push(`servicio_${i}.api_${i} -> servicio_${i + 1}.api_${i + 1}: llama`);
  return parts.join('\n') + '\n';
}

function longMermaid(): string {
  const parts = ['flowchart TD'];
  for (let i = 0; i < 60; i++) {
    parts.push(`  subgraph grupo${i} [Grupo ${i}]`, `    nodo${i}a[Inicio ${i}] --> nodo${i}b{¿Válido?}`, `    nodo${i}b -->|sí| nodo${i}c[Procesar ${i}]`, `    nodo${i}b -->|no| nodo${i}d[Rechazar ${i}]`, '  end');
  }
  for (let i = 0; i < 59; i++) parts.push(`  nodo${i}c --> nodo${i + 1}a`);
  return parts.join('\n') + '\n';
}

let s: Session;

beforeAll(async () => {
  s = await startApp({
    'uno.md': '# Uno\n\nprimera línea\nsegunda línea\ntercera línea\n',
    'dos.md': '# Dos\n\notra nota\n',
    'grafo.d2': 'a -> b\nb -> c\nc -> d\n',
    'largo.d2': longD2(),
    'flujo.mmd': longMermaid(),
  });
  await s.browser.$('.file-tree-item[data-path="uno.md"]').waitForExist({ timeout: 30_000 });
}, 120_000);

afterAll(async () => {
  await s?.close();
});

/** Texto del editor visible (el de la pestaña activa). */
const visibleText = () =>
  s.browser.execute(() => {
    const content = [...document.querySelectorAll('.cm-content')].find((e) => (e as HTMLElement).offsetParent);
    return content ? [...content.querySelectorAll('.cm-line')].map((l) => l.textContent).join('\n') : '';
  }) as Promise<string>;

async function openTab(path: string) {
  await s.browser.$(`.file-tree-item[data-path="${path}"]`).click();
  await s.browser.pause(600);
}

async function editAtEnd(path: string) {
  await openTab(path);
  const visible = () =>
    s.browser.execute(() => !![...document.querySelectorAll('.cm-content')].find((e) => (e as HTMLElement).offsetParent)) as Promise<boolean>;
  if (!(await visible())) await s.browser.keys(['F2']);
  await s.browser.waitUntil(visible, { timeout: 10_000 });
  const content = await s.browser.execute(() => [...document.querySelectorAll('.cm-content')].find((e) => (e as HTMLElement).offsetParent) as HTMLElement);
  await s.browser.$(content).click();
  await s.browser.keys(['Control', 'End']);
  await s.browser.keys(['x']);
  await s.browser.pause(300);
}

for (const path of ['uno.md', 'grafo.d2']) {
  it(`${path}: al volver a la pestaña el foco y el cursor siguen donde estaban`, async () => {
    await editAtEnd(path);
    expect(await visibleText()).toContain('x');
    await openTab('dos.md');
    await s.browser.pause(1000);
    await s.browser.$(`.tab-title-container[data-path="${path}"]`).click();
    await s.browser.pause(800);
    // Lo que se escribe al volver, sin hacer clic en el editor, sigue a la `x`.
    await s.browser.keys(['y']);
    await s.browser.waitUntil(async () => (await visibleText()).includes('y'), { timeout: 5_000 });
    expect(await visibleText()).toContain('xy');
  }, 60_000);
}

it('un cambio en disco se aplica sin mover el cursor al inicio', async () => {
  await s.browser.keys(['Control', 's']);
  await s.browser.pause(3000);
  // Otro programa cambia la primera línea del diagrama (la pestaña está guardada, se recarga).
  writeFileSync(join(s.vault, 'grafo.d2'), 'z -> b\nb -> c\nc -> d\nxy');
  await s.browser.waitUntil(async () => (await visibleText()).startsWith('z -> b'), { timeout: 15_000 });
  await s.browser.keys(['w']);
  await s.browser.waitUntil(async () => (await visibleText()).includes('w'), { timeout: 5_000 });
  expect(await visibleText()).toContain('xyw');
}, 60_000);

/** Scroll del editor visible (el suyo o el del contenedor que lo desplace). */
const scrollTop = () =>
  s.browser.execute(() => {
    let el = [...document.querySelectorAll('.cm-content')].find((e) => (e as HTMLElement).offsetParent) as HTMLElement | null;
    let total = 0;
    while (el) {
      total += el.scrollTop;
      el = el.parentElement;
    }
    return Math.round(total);
  }) as Promise<number>;

for (const path of ['largo.d2', 'flujo.mmd']) {
  it(`${path} (largo): el cursor y el scroll se conservan a mitad del archivo`, async () => {
    await openTab(path);
    const visible = () =>
      s.browser.execute(() => !![...document.querySelectorAll('.cm-content')].find((e) => (e as HTMLElement).offsetParent)) as Promise<boolean>;
    if (!(await visible())) await s.browser.keys(['F2']);
    await s.browser.waitUntil(visible, { timeout: 10_000 });
    const content = await s.browser.execute(() => [...document.querySelectorAll('.cm-content')].find((e) => (e as HTMLElement).offsetParent) as HTMLElement);
    await s.browser.$(content).click();
    // A mitad del archivo: unas 200 líneas abajo, al final de la línea.
    await s.browser.keys(['Control', 'Home']);
    // Una tecla por llamada: WebDriver junta las repetidas dentro de una misma secuencia.
    for (let i = 0; i < 200; i++) await s.browser.keys(['ArrowDown']);
    await s.browser.keys(['End', 'x']);
    await s.browser.pause(300);
    const scrolled = await scrollTop();

    expect(scrolled).toBeGreaterThan(500);

    await openTab('dos.md');
    await s.browser.pause(1000);
    await s.browser.$(`.tab-title-container[data-path="${path}"]`).click();
    await s.browser.pause(800);
    expect(Math.abs((await scrollTop()) - scrolled)).toBeLessThan(40);
    await s.browser.keys(['y']);
    await s.browser.waitUntil(async () => (await visibleText()).includes('xy'), { timeout: 5_000 });
    expect((await visibleText()).startsWith('y')).toBe(false);
  }, 90_000);
}
