// Diff de un documento de Word: cambios de texto, tablas e imágenes, y las dos versiones lado a lado.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { crc32, deflateSync } from 'node:zlib';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;
let vault: string;
const git = (...args: string[]) =>
  execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', ...args], { cwd: vault });

/** PNG de 40×30 de un solo color (para que la versión vieja y la nueva difieran). */
function png(r: number, g: number, b: number): Buffer {
  const w = 40;
  const h = 30;
  const crcTable = Array.from({ length: 256 }, (_, n) => {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    return c >>> 0;
  });
  const crc = (buf: Buffer) => {
    let c = 0xffffffff;
    for (const byte of buf) c = crcTable[(c ^ byte) & 0xff] ^ (c >>> 8);
    return (c ^ 0xffffffff) >>> 0;
  };
  const chunk = (type: string, data: Buffer) => {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    const td = Buffer.concat([Buffer.from(type), data]);
    const c = Buffer.alloc(4);
    c.writeUInt32BE(crc(td));
    return Buffer.concat([len, td, c]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr.set([8, 2, 0, 0, 0], 8);
  const row = Buffer.concat([Buffer.from([0]), Buffer.from(Array.from({ length: w }, () => [r, g, b]).flat())]);
  const raw = Buffer.concat(Array.from({ length: h }, () => row));
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ihdr), chunk('IDAT', deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]);
}

/** ZIP sin comprimir. */
function zip(files: Record<string, string | Buffer>): Buffer {
  const parts: Buffer[] = [];
  const central: Buffer[] = [];
  let offset = 0;
  for (const [name, content] of Object.entries(files)) {
    const data = Buffer.from(content as string);
    const n = Buffer.from(name);
    const h = Buffer.alloc(30);
    h.writeUInt32LE(0x04034b50, 0);
    h.writeUInt16LE(20, 4);
    h.writeUInt16LE(33, 12);
    h.writeUInt32LE(crc32(data), 14);
    h.writeUInt32LE(data.length, 18);
    h.writeUInt32LE(data.length, 22);
    h.writeUInt16LE(n.length, 26);
    const c = Buffer.alloc(46);
    c.writeUInt32LE(0x02014b50, 0);
    c.writeUInt16LE(20, 4);
    h.copy(c, 6, 4, 30);
    c.writeUInt32LE(offset, 42);
    central.push(c, n);
    parts.push(h, n, data);
    offset += 30 + n.length + data.length;
  }
  const dir = Buffer.concat(central);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(Object.keys(files).length, 8);
  end.writeUInt16LE(Object.keys(files).length, 10);
  end.writeUInt32LE(dir.length, 12);
  end.writeUInt32LE(offset, 16);
  return Buffer.concat([...parts, dir, end]);
}

const p = (t: string, pr = '') => `<w:p>${pr}<w:r><w:t xml:space="preserve">${t}</w:t></w:r></w:p>`;
const table = (rows: string[][]) =>
  `<w:tbl><w:tblGrid><w:gridCol w:w="2000"/><w:gridCol w:w="2000"/></w:tblGrid>${rows.map((r) => `<w:tr>${r.map((c) => `<w:tc>${p(c)}</w:tc>`).join('')}</w:tr>`).join('')}</w:tbl>`;

function docx(o: { day: string; amount: string; relId: string; image: Buffer; bold?: boolean }): Buffer {
  const NS =
    'xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"';
  const drawing = `<w:p><w:r><w:drawing><wp:inline><wp:extent cx="400000" cy="300000"/><wp:docPr id="1" name="foto" descr="una foto"/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:blipFill><a:blip r:embed="${o.relId}"/></pic:blipFill><pic:spPr><a:xfrm><a:ext cx="400000" cy="300000"/></a:xfrm></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>`;
  const body = [
    p('Contrato', '<w:pPr><w:pStyle w:val="Ttulo1"/></w:pPr>'),
    ...Array.from({ length: 12 }, (_, i) => p(`Cláusula ${i} sin cambios`)),
    // Los runs partidos y los rsid no son un cambio
    o.bold ? '<w:p w:rsidR="00FF"><w:r><w:t>Texto igual par</w:t></w:r><w:r><w:rPr><w:b/></w:rPr><w:t>tido</w:t></w:r></w:p>' : p('Texto igual partido'),
    p(`El pago de A &amp; B vence el ${o.day} próximo &lt;script&gt;`),
    table([['Campo', 'Valor'], ['monto', o.amount], ['moneda', 'PEN']]),
    drawing,
  ].join('');
  return zip({
    '[Content_Types].xml':
      '<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/></Types>',
    '_rels/.rels':
      '<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>',
    'word/_rels/document.xml.rels': `<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rSt" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="${o.relId}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.png"/></Relationships>`,
    'word/styles.xml': `<?xml version="1.0"?><w:styles ${NS}><w:style w:type="paragraph" w:styleId="Ttulo1"><w:name w:val="heading 1"/></w:style></w:styles>`,
    'word/document.xml': `<?xml version="1.0" encoding="UTF-8"?><w:document ${NS}><w:body>${body}</w:body></w:document>`,
    'word/media/image1.png': o.image,
  });
}

const red = () => png(200, 40, 40);
const blue = () => png(40, 40, 200);

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-diffdocx-'));
  vault = join(base, 'vault');
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  writeFileSync(join(vault, 'contrato.docx'), docx({ day: 'lunes', amount: '10', relId: 'rId4', image: red() }));
  writeFileSync(join(vault, 'formato.docx'), docx({ day: 'lunes', amount: '10', relId: 'rId4', image: red() }));
  git('init', '-q');
  git('add', '.');
  git('commit', '-qm', 'inicio');
  writeFileSync(join(vault, 'contrato.docx'), docx({ day: 'martes', amount: '20', relId: 'rId9', image: blue(), bold: true }));
  // Solo cambian el formato y el identificador de la imagen
  writeFileSync(join(vault, 'formato.docx'), docx({ day: 'lunes', amount: '10', relId: 'rId9', image: red(), bold: true }));
  s = await startAppOnVault(vault);
  await s.browser.$('.file-tree-item[data-path="contrato.docx"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

async function openDiff(file: string) {
  await s.browser.execute((f: string) => {
    const el = document.querySelector(`.file-tree-item[data-path="${f}"]`)!;
    const { x, y } = el.getBoundingClientRect();
    el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: x + 5, clientY: y + 5 }));
  }, file);
  await s.browser.$('.context-menu-item').waitForExist({ timeout: 5_000 });
  const picked = await s.browser.execute(() => {
    const item = [...document.querySelectorAll('.context-menu-item')].find((i) => /^Diff( visual| \(| Ctrl|$)|^Ver cambios/.test(i.textContent?.trim() ?? ''));
    (item as HTMLElement | undefined)?.click();
    return item?.textContent?.trim() ?? null;
  });
  expect(picked).not.toBeNull();
}

const all = (sel: string) =>
  s.browser.execute((q: string) => [...document.querySelectorAll(q)].map((e) => e.textContent?.trim()), sel) as Promise<string[]>;

it('muestra los cambios de texto, tabla e imagen', async () => {
  await openDiff('contrato.docx');
  await s.browser.$('.docx-diff .docx-entry.modified').waitForExist({ timeout: 15_000 });
  expect(await all('.docx-diff .stats')).toEqual(['~3']);
  expect(await all('.docx-diff .docx-p del')).toEqual(['lunes', '10']);
  expect(await all('.docx-diff .docx-p ins')).toEqual(['martes', '20']);
  // Las entidades se conservan y nada se interpreta como HTML
  expect((await all('.docx-diff .docx-entry.modified .docx-p')).join()).toContain('A & B vence el lunesmartes próximo <script>');
  // De la tabla, solo la celda que cambió
  expect(await all('.docx-diff td.cell-modified')).toHaveLength(1);
  expect(await all('.docx-diff tr.row-modified')).toHaveLength(1);
  // La imagen, antes y después
  await s.browser.waitUntil(
    () =>
      s.browser.execute(() => {
        const imgs = [...document.querySelectorAll<HTMLImageElement>('.docx-images img')];
        return imgs.length === 2 && imgs.every((i) => i.naturalWidth === 40) && imgs[0].src !== imgs[1].src;
      }),
    { timeout: 10_000, timeoutMsg: 'no se ven las dos imágenes' },
  );
  // Lo igual se pliega y se puede abrir
  const before = (await all('.docx-entry.equal')).length;
  await s.browser.$('.docx-fold').click();
  await s.browser.waitUntil(async () => (await all('.docx-entry.equal')).length > before, { timeout: 5_000 });
  expect(await all('.docx-heading.h1')).toEqual(['Contrato']);
  // Ir de un cambio a otro
  await s.browser.$('.docx-diff-bar button[aria-label="Cambio siguiente"]').click();
  await s.browser.waitUntil(async () => (await all('.docx-diff .pos'))[0] === '1/3', { timeout: 5_000 });
}, 60_000);

it('dibuja las dos versiones lado a lado', async () => {
  await s.browser.$('button*=2 Paneles').click();
  await s.browser.waitUntil(
    async () => {
      const t = await all('.docx-pane-body');
      return t.length === 2 && t[0].includes('lunes') && t[1].includes('martes');
    },
    { timeout: 15_000, timeoutMsg: 'no se dibujaron los dos documentos' },
  );
  // Sin vista de texto: es un binario
  expect((await all('.diff-toolbar .toggle-btn')).filter((t) => t === 'Texto')).toEqual([]);
  await s.browser.$('button*=Cambios').click();
  await s.browser.$('.docx-diff .docx-entry.modified').waitForDisplayed({ timeout: 5_000 });
}, 30_000);

it('si solo cambia el formato, lo dice', async () => {
  await openDiff('formato.docx');
  await s.browser.waitUntil(async () => (await all('.docx-diff .docx-state')).some((t) => t.includes('como el formato')), {
    timeout: 15_000,
    timeoutMsg: 'no avisa de que solo cambia el formato',
  });
}, 30_000);
