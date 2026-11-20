// Visor de hojas de cálculo: un .xlsx se abre en una pestaña (no en la app del sistema) con sus
// valores, la negrita y el relleno, la celda combinada, la fórmula en la barra y en «Fórmulas», una
// fórmula sin resultado guardado, y las pestañas de hojas (la oculta solo al pedirla).
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { strToU8, zipSync } from 'fflate';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;

const istr = (ref: string, text: string, style = '') => `<c r="${ref}" t="inlineStr"${style}><is><t>${text}</t></is></c>`;

function buildXlsx(): Uint8Array {
  const sheet1 = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<cols><col min="1" max="1" width="20" customWidth="1"/></cols>
<sheetData>
<row r="1">${istr('A1', 'Producto', ' s="1"')}${istr('B1', 'Monto', ' s="1"')}</row>
<row r="2">${istr('A2', 'Café')}<c r="B2"><v>10</v></c></row>
<row r="3">${istr('A3', 'Pan')}<c r="B3"><v>5</v></c></row>
<row r="4">${istr('A4', 'Total')}<c r="B4" s="2"><f>SUM(B2:B3)</f><v>15</v></c></row>
<row r="5">${istr('A5', 'Combinada')}</row>
<row r="6"><c r="B6"><f>B4*2</f></c></row>
</sheetData>
<mergeCells count="1"><mergeCell ref="A5:B5"/></mergeCells>
</worksheet>`;
  const sheet2 = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>
<row r="1">${istr('A1', 'Notas')}</row></sheetData></worksheet>`;
  const sheet3 = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>
<row r="1">${istr('A1', 'Secreto')}</row></sheetData></worksheet>`;
  const styles = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<fonts count="2"><font><sz val="11"/><name val="Calibri"/></font><font><b/><sz val="11"/><name val="Calibri"/></font></fonts>
<fills count="3"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill>
<fill><patternFill patternType="solid"><fgColor rgb="FFFFFF00"/></patternFill></fill></fills>
<borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
<cellXfs count="3"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
<xf numFmtId="0" fontId="1" fillId="0" borderId="0" xfId="0" applyFont="1"/>
<xf numFmtId="0" fontId="0" fillId="2" borderId="0" xfId="0" applyFill="1"/></cellXfs>
</styleSheet>`;
  const workbook = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<sheets><sheet name="Ventas" sheetId="1" r:id="rId1"/><sheet name="Notas" sheetId="2" r:id="rId2"/>
<sheet name="Oculta" sheetId="3" state="hidden" r:id="rId3"/></sheets></workbook>`;
  const rel = (id: string, type: string, target: string) =>
    `<Relationship Id="${id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/${type}" Target="${target}"/>`;
  const wbRels = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
${rel('rId1', 'worksheet', 'worksheets/sheet1.xml')}${rel('rId2', 'worksheet', 'worksheets/sheet2.xml')}
${rel('rId3', 'worksheet', 'worksheets/sheet3.xml')}${rel('rId4', 'styles', 'styles.xml')}</Relationships>`;
  const rootRels = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
${rel('rId1', 'officeDocument', 'xl/workbook.xml')}</Relationships>`;
  const ws = 'application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml';
  const types = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
<Override PartName="/xl/worksheets/sheet1.xml" ContentType="${ws}"/>
<Override PartName="/xl/worksheets/sheet2.xml" ContentType="${ws}"/>
<Override PartName="/xl/worksheets/sheet3.xml" ContentType="${ws}"/>
<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>
</Types>`;
  return zipSync({
    '[Content_Types].xml': strToU8(types),
    '_rels/.rels': strToU8(rootRels),
    'xl/workbook.xml': strToU8(workbook),
    'xl/_rels/workbook.xml.rels': strToU8(wbRels),
    'xl/styles.xml': strToU8(styles),
    'xl/worksheets/sheet1.xml': strToU8(sheet1),
    'xl/worksheets/sheet2.xml': strToU8(sheet2),
    'xl/worksheets/sheet3.xml': strToU8(sheet3),
  });
}

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-sheet-'));
  mkdirSync(join(base, '.synapse'), { recursive: true });
  writeFileSync(join(base, 'nota.md'), '# Nota\n');
  writeFileSync(join(base, 'ventas.xlsx'), buildXlsx());
  s = await startAppOnVault(base);
  await s.browser.$('.file-tree-item[data-path="ventas.xlsx"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

const cell = (ref: string) =>
  s.browser.execute((ref) => {
    const td = document.querySelector(`.spreadsheet-viewer .sheet-cell[data-cell="${ref}"]`) as HTMLElement | null;
    if (!td) return null;
    const cs = getComputedStyle(td);
    return {
      text: td.textContent ?? '',
      bold: Number(cs.fontWeight) >= 700,
      bg: cs.backgroundColor,
      merged: td.classList.contains('merged'),
      width: td.getBoundingClientRect().width,
      formula: td.classList.contains('has-formula'),
      noCache: td.classList.contains('no-cache'),
    };
  }, ref) as Promise<{ text: string; bold: boolean; bg: string; merged: boolean; width: number; formula: boolean; noCache: boolean } | null>;

it('abre el .xlsx en una pestaña con valores, estilos, combinadas y fórmulas', async () => {
  await s.browser.execute(() => (document.querySelector('.file-tree-item[data-path="ventas.xlsx"]') as HTMLElement).click());
  await s.browser.$('.spreadsheet-viewer .sheet-cell[data-cell="A1"]').waitForExist({ timeout: 20_000 });
  expect((await cell('A2'))?.text).toBe('Café');
  expect(await s.browser.execute(() => !!document.querySelector('.spreadsheet-viewer .open-external'))).toBe(true);
  expect((await cell('A1'))?.bold).toBe(true);
  const total = await cell('B4');
  expect(total?.text).toBe('15');
  expect(total?.formula).toBe(true);
  expect(total?.bg).toBe('rgb(255, 255, 0)');
  const a5 = await cell('A5');
  expect(a5?.merged).toBe(true);
  expect(a5?.width).toBeGreaterThan((await cell('A4'))!.width + 30);
  expect(await cell('B5')).toBeNull();
  const sinCache = await cell('B6');
  expect(sinCache?.noCache).toBe(true);
  expect(sinCache?.text).toBe('=B4*2');

  // Barra de fórmulas
  await s.browser.execute(() =>
    document.querySelector('.spreadsheet-viewer .sheet-cell[data-cell="B4"]')!.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true })),
  );
  await s.browser.waitUntil(
    async () => (await s.browser.execute(() => document.querySelector('.spreadsheet-viewer .formula-text')?.textContent?.trim())) === '=SUM(B2:B3)',
    { timeout: 5_000, timeoutMsg: 'la barra no muestra la fórmula' },
  );
  // «Fórmulas» muestra la fórmula en la celda
  await s.browser.execute(() => (document.querySelector('.spreadsheet-viewer .show-formulas') as HTMLElement).click());
  await s.browser.waitUntil(async () => (await cell('B4'))?.text === '=SUM(B2:B3)', { timeout: 5_000 });
  await s.browser.execute(() => (document.querySelector('.spreadsheet-viewer .show-formulas') as HTMLElement).click());
}, 60_000);

it('pestañas de hojas: cambia de hoja y la oculta solo aparece al pedirla', async () => {
  const tabs = () =>
    s.browser.execute(() => [...document.querySelectorAll('.spreadsheet-viewer .sheet-tab')].map((t) => t.textContent?.trim())) as Promise<string[]>;
  expect(await tabs()).toEqual(['Ventas', 'Notas']);
  await s.browser.execute(() =>
    ([...document.querySelectorAll('.spreadsheet-viewer .sheet-tab')].find((t) => t.textContent?.includes('Notas')) as HTMLElement).click(),
  );
  await s.browser.waitUntil(async () => (await cell('A1'))?.text === 'Notas', { timeout: 5_000 });
  await s.browser.execute(() => (document.querySelector('.spreadsheet-viewer .toggle-hidden') as HTMLElement).click());
  await s.browser.waitUntil(async () => (await tabs()).includes('Oculta'), { timeout: 5_000 });
  // Alt+AvPág pasa a la siguiente hoja (la oculta, ya visible)
  await s.browser.execute(() => (document.querySelector('.spreadsheet-viewer .sheet-scroll') as HTMLElement).focus());
  await s.browser.keys(['Alt', 'PageDown']);
  await s.browser.waitUntil(async () => (await cell('A1'))?.text === 'Secreto', { timeout: 5_000, timeoutMsg: 'Alt+AvPág no cambió de hoja' });
}, 60_000);

it('zoom: Ctrl++ agranda toda la rejilla (anchos, altos y letra), Ctrl+0 vuelve', async () => {
  await s.browser.execute(() =>
    ([...document.querySelectorAll('.spreadsheet-viewer .sheet-tab')].find((t) => t.textContent?.includes('Ventas')) as HTMLElement).click(),
  );
  await s.browser.waitUntil(async () => (await cell('A1'))?.text === 'Producto', { timeout: 5_000 });
  const box = () =>
    s.browser.execute(() => {
      const c = document.querySelector('.spreadsheet-viewer .sheet-cell[data-cell="A2"]') as HTMLElement;
      const r = c.getBoundingClientRect();
      return { w: r.width, h: r.height, font: parseFloat(getComputedStyle(c).fontSize) };
    }) as Promise<{ w: number; h: number; font: number }>;
  const b100 = await box();
  await s.browser.execute(() => (document.querySelector('.spreadsheet-viewer .sheet-scroll') as HTMLElement).focus());
  for (const k of [['Control', '='], ['Control', '=']]) await s.browser.keys(k);
  await s.browser.waitUntil(async () => Math.abs((await box()).w / b100.w - 1.2) < 0.03, { timeout: 5_000, timeoutMsg: 'no creció el ancho' });
  const b120 = await box();
  expect(b120.h / b100.h).toBeCloseTo(1.2, 1);
  expect(b120.font / b100.font).toBeCloseTo(1.2, 1);
  await s.browser.saveScreenshot('/tmp/claude-1000/-home-miuler-src-github-com-Miuler-synapse-rs/47a613e4-73f4-47e2-9153-fa7e86d7bbd0/scratchpad/sheet-120.png');
  await s.browser.keys(['Control', '0']);
  await s.browser.waitUntil(async () => Math.abs((await box()).w - b100.w) < 1, { timeout: 5_000, timeoutMsg: 'Ctrl+0 no volvió al 100 %' });
}, 60_000);
