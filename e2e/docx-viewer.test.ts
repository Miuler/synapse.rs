// Visor de Word: un .docx se abre en una pestaña (no en la aplicación del sistema) con su texto,
// la negrita y la tabla, y con el botón «Abrir en la aplicación». El .doc antiguo no se abre aquí.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { copyFileSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-docx-'));
  mkdirSync(join(base, '.synapse'), { recursive: true });
  writeFileSync(join(base, 'nota.md'), '# Nota\n');
  // Generado con LibreOffice desde HTML (negrita, párrafo y una tabla de 2×2)
  copyFileSync(join(import.meta.dir, 'fixtures/informe.docx'), join(base, 'informe.docx'));
  s = await startAppOnVault(base);
  await s.browser.$('.file-tree-item[data-path="informe.docx"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

it('abre el .docx en una pestaña con su texto, la negrita y la tabla', async () => {
  await s.browser.execute(() => (document.querySelector('.file-tree-item[data-path="informe.docx"]') as HTMLElement).click());
  await s.browser.waitUntil(
    async () => (await s.browser.execute(() => document.querySelector('.docx-viewer section.docx')?.textContent ?? '')).includes('Informe trimestral'),
    { timeout: 20_000, timeoutMsg: 'no se pintó el documento' },
  );
  const info = (await s.browser.execute(() => {
    const v = document.querySelector('.docx-viewer')!;
    const bold = [...v.querySelectorAll('section.docx span')].find((e) => e.textContent === 'Informe trimestral') as HTMLElement;
    return {
      bold: bold ? Number(getComputedStyle(bold).fontWeight) >= 700 : false,
      cells: [...v.querySelectorAll('section.docx td')].map((td) => td.textContent?.trim()),
      status: v.querySelector('.docx-status')?.textContent?.trim(),
      button: !!v.querySelector('.open-external'),
    };
  })) as { bold: boolean; cells: string[]; status: string; button: boolean };
  expect(info.bold).toBe(true);
  expect(info.cells).toEqual(['Producto', 'Monto', 'Café', '10']);
  expect(info.status).toBe('Solo lectura');
  expect(info.button).toBe(true);
}, 60_000);
