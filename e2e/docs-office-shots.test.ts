// Capturas de los visores integrados (Word, Excel, PDF y OpenAPI) para docs/.
// Uso: E2E_SHOTS=docs bun test e2e/docs-office-shots.test.ts
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { copyFileSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { strToU8, zipSync } from 'fflate';
import { startAppOnVault, type Session } from './harness';

const SHOTS = process.env.E2E_SHOTS;
let s: Session;
let base: string;

const istr = (ref: string, text: string, style = '') => `<c r="${ref}" t="inlineStr"${style}><is><t>${text}</t></is></c>`;

function buildXlsx(): Uint8Array {
  const sheet1 = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<cols>
  <col min="1" max="1" width="24" customWidth="1"/>
  <col min="2" max="2" width="14" customWidth="1"/>
  <col min="3" max="3" width="14" customWidth="1"/>
  <col min="4" max="4" width="16" customWidth="1"/>
</cols>
<sheetData>
<row r="1">${istr('A1', 'Componente', ' s="1"')}${istr('B1', 'Unidades', ' s="1"')}${istr('C1', 'Costo USD', ' s="1"')}${istr('D1', 'Subtotal', ' s="1"')}</row>
<row r="2">${istr('A2', 'API Gateway (Rust)')}<c r="B2"><v>3</v></c><c r="C2"><v>120</v></c><c r="D2"><f>B2*C2</f><v>360</v></c></row>
<row r="3">${istr('A3', 'Broker de Eventos')}<c r="B3"><v>2</v></c><c r="C3"><v>180</v></c><c r="D3"><f>B3*C3</f><v>360</v></c></row>
<row r="4">${istr('A4', 'PostgreSQL HA')}<c r="B4"><v>2</v></c><c r="C4"><v>250</v></c><c r="D4"><f>B4*C4</f><v>500</v></c></row>
<row r="5">${istr('A5', 'Total Mensual', ' s="1"')}${istr('B5', '')}${istr('C5', '')}<c r="D5" s="2"><f>SUM(D2:D4)</f><v>1220</v></c></row>
</sheetData>
</worksheet>`;
  const sheet2 = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>
<row r="1">${istr('A1', 'Notas de arquitectura')}</row></sheetData></worksheet>`;
  const styles = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<fonts count="2"><font><sz val="11"/><name val="Calibri"/></font><font><b/><sz val="11"/><name val="Calibri"/></font></fonts>
<fills count="3"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill>
<fill><patternFill patternType="solid"><fgColor rgb="FFD9F2E6"/></patternFill></fill></fills>
<borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
<cellXfs count="3"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
<xf numFmtId="0" fontId="1" fillId="0" borderId="0" xfId="0" applyFont="1"/>
<xf numFmtId="0" fontId="1" fillId="2" borderId="0" xfId="0" applyFont="1" applyFill="1"/></cellXfs>
</styleSheet>`;
  const workbook = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<sheets><sheet name="Infraestructura" sheetId="1" r:id="rId1"/><sheet name="Notas" sheetId="2" r:id="rId2"/></sheets></workbook>`;
  const rel = (id: string, type: string, target: string) =>
    `<Relationship Id="${id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/${type}" Target="${target}"/>`;
  const wbRels = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
${rel('rId1', 'worksheet', 'worksheets/sheet1.xml')}${rel('rId2', 'worksheet', 'worksheets/sheet2.xml')}${rel('rId3', 'styles', 'styles.xml')}</Relationships>`;
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
  });
}

function buildPdf(): Uint8Array {
  const stream = [
    'BT',
    '/F1 20 Tf 60 740 Td (Synapse.rs - Guia de Arquitectura) Tj',
    '/F1 12 Tf 0 -36 Td (1. Local-first y formatos abiertos: Markdown, D2, Mermaid, OpenAPI.) Tj',
    '0 -22 Td (2. Indexacion nativa en Rust con Tantivy dentro de PDF, Word y Excel.) Tj',
    '0 -22 Td (3. Preparado para agentes de IA con ACP, MCP y OKF v0.3.) Tj',
    'ET',
  ].join('\n');
  const objs = [
    '1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n',
    '2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n',
    '3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >> endobj\n',
    `4 0 obj << /Length ${stream.length} >>\nstream\n${stream}\nendstream\nendobj\n`,
    '5 0 obj << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> endobj\n',
  ];
  let pdf = '%PDF-1.4\n';
  const offsets: number[] = [];
  for (const o of objs) {
    offsets.push(pdf.length);
    pdf += o;
  }
  const xref = pdf.length;
  pdf += `xref\n0 ${objs.length + 1}\n0000000000 65535 f \n`;
  for (const off of offsets) pdf += `${String(off).padStart(10, '0')} 00000 n \n`;
  pdf += `trailer << /Size ${objs.length + 1} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`;
  return new TextEncoder().encode(pdf);
}

const OPENAPI = `openapi: 3.0.3
info:
  title: API de Pedidos · Synapse
  version: 1.2.0
  description: Documentación interactiva embebida con Scalar (sin CDN ni telemetría).
paths:
  /pedidos:
    get:
      summary: Listar pedidos recientes
      responses:
        '200':
          description: Lista paginada de pedidos
    post:
      summary: Crear un nuevo pedido
      responses:
        '201':
          description: Pedido creado y evento publicado
`;

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-docs-office-'));
  mkdirSync(join(base, '.synapse'), { recursive: true });
  copyFileSync(join(import.meta.dir, 'fixtures/informe.docx'), join(base, 'informe.docx'));
  writeFileSync(join(base, 'presupuesto.xlsx'), buildXlsx());
  writeFileSync(join(base, 'guia-arquitectura.pdf'), buildPdf());
  writeFileSync(join(base, 'openapi.yaml'), OPENAPI);
  s = await startAppOnVault(base);
  await s.browser.setWindowSize(1500, 940);
  await s.browser.$('.file-tree-item[data-path="informe.docx"]').waitForExist({ timeout: 30_000 });
}, 120_000);

afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

const shot = (name: string) => s.browser.saveScreenshot(`${SHOTS}/${name}.png`);

it('captura: Word, Excel, PDF y OpenAPI', async () => {
  if (!SHOTS) return;

  await s.browser.execute(() => (document.querySelector('.file-tree-item[data-path="informe.docx"]') as HTMLElement).click());
  await s.browser.waitUntil(
    async () => (await s.browser.execute(() => document.querySelector('.docx-viewer section.docx')?.textContent ?? '')).includes('Informe trimestral'),
    { timeout: 20_000 },
  );
  await s.browser.pause(600);
  await shot('docs_word_visor');

  await s.browser.execute(() => (document.querySelector('.file-tree-item[data-path="presupuesto.xlsx"]') as HTMLElement).click());
  await s.browser.waitUntil(
    async () => await s.browser.execute(() => !!document.querySelector('.spreadsheet-viewer')),
    { timeout: 20_000 },
  );
  await s.browser.pause(800);
  await shot('docs_excel_visor');

  await s.browser.execute(() => (document.querySelector('.file-tree-item[data-path="guia-arquitectura.pdf"]') as HTMLElement).click());
  await s.browser.waitUntil(
    async () => await s.browser.execute(() => !!document.querySelector('.tab-pane:not(.hidden) .synapse-pdf-container') && !document.querySelector('.tab-pane:not(.hidden) .synapse-pdf-loading')),
    { timeout: 20_000 },
  );
  await s.browser.pause(1500);
  await shot('docs_pdf_visor');

  await s.browser.execute(() => (document.querySelector('.file-tree-item[data-path="openapi.yaml"]') as HTMLElement).click());
  await s.browser.waitUntil(
    async () => await s.browser.execute(() => !!document.querySelector('.tab-pane:not(.hidden) .scalar-app, .tab-pane:not(.hidden) .openapi-viewer')),
    { timeout: 20_000 },
  );
  await s.browser.pause(1000);
  await shot('docs_openapi_visor');

  expect(true).toBe(true);
}, 180_000);
