// Exportar diagramas desde el menú contextual del explorador: Excalidraw, Mermaid y D2 a PNG y SVG,
// junto al archivo original.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { startApp, type Session } from './harness';

const EXCALIDRAW = JSON.stringify({
  type: 'excalidraw',
  version: 2,
  source: 'synapse',
  elements: [
    {
      id: 'r1', type: 'rectangle', x: 0, y: 0, width: 120, height: 60, angle: 0, strokeColor: '#1e1e1e',
      backgroundColor: 'transparent', fillStyle: 'solid', strokeWidth: 2, strokeStyle: 'solid', roughness: 1,
      opacity: 100, groupIds: [], frameId: null, roundness: null, seed: 1, version: 1, versionNonce: 1,
      isDeleted: false, boundElements: null, updated: 1, link: null, locked: false,
    },
  ],
  appState: { viewBackgroundColor: '#ffffff' },
  files: {},
});

let s: Session;

beforeAll(async () => {
  s = await startApp({
    'flujo.mmd': 'graph TD\n  A[Inicio] --> B[Fin]\n',
    'arquitectura.d2': 'server -> db: consulta\n',
    'boceto.excalidraw': EXCALIDRAW,
  });
  await s.browser.$('.file-tree-item[data-path="flujo.mmd"]').waitForExist({ timeout: 30_000 });
}, 120_000);

afterAll(async () => {
  await s?.close();
});

async function exportFrom(path: string, label: string) {
  await s.browser.$(`.file-tree-item[data-path="${path}"]`).click({ button: 'right' });
  const item = s.browser.$(`//div[@role="menuitem"][contains(., "${label}")]`);
  await item.waitForExist({ timeout: 5_000 });
  await item.click();
}

async function waitForFile(rel: string): Promise<Buffer> {
  const file = join(s.vault, rel);
  await s.browser.waitUntil(async () => existsSync(file), { timeout: 20_000, timeoutMsg: `no se creó ${rel}` });
  return readFileSync(file);
}

const PNG_SIGNATURE = Buffer.from([0x89, 0x50, 0x4e, 0x47]);

for (const [source, base] of [['flujo.mmd', 'flujo'], ['arquitectura.d2', 'arquitectura'], ['boceto.excalidraw', 'boceto']]) {
  it(`exporta ${source} a SVG y PNG`, async () => {
    await exportFrom(source, 'Exportar como SVG');
    const svg = (await waitForFile(`${base}.svg`)).toString('utf8');
    expect(svg).toContain('<svg');

    await exportFrom(source, 'Exportar como PNG');
    const png = await waitForFile(`${base}.png`);
    expect(png.subarray(0, 4).equals(PNG_SIGNATURE)).toBe(true);
    // Ancho en la cabecera IHDR: el PNG sale al doble de escala, no vacío.
    expect(png.readUInt32BE(16)).toBeGreaterThan(100);
    expect(png.length).toBeGreaterThan(500);
  }, 60_000);
}

it('los archivos que no son diagramas no ofrecen exportar', async () => {
  await s.browser.$('.file-tree-item[data-path="flujo.svg"]').waitForExist({ timeout: 10_000 });
  await s.browser.$('.file-tree-item[data-path="flujo.svg"]').click({ button: 'right' });
  await s.browser.$('[role="menu"]').waitForExist({ timeout: 5_000 });
  const offered = await s.browser.execute(() => document.querySelector('[role="menu"]')?.textContent?.includes('Exportar como'));
  expect(offered).toBe(false);
  await s.browser.keys(['Escape']);
});
