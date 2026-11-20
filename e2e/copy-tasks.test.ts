// Copiar desde el modo lectura una lista de tareas: las casillas llegan como ☐ / ✅ (los correos y
// Teams quitan los <input>), con la negrita y la lista, en HTML y en texto plano.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session, paneReady } from './harness';

let s: Session;

const NOTE = [
  '# Avance',
  '',
  '- [ ] 00. Modelo de Dato',
  '- [ ] **(faltan 5 de 26)** 01. Diagrama de Secuencia',
  '- [x] 02. Diagrama de Software',
  '',
  'Párrafo sin tareas.',
  '',
].join('\n');

beforeAll(async () => {
  s = await startApp({ 'avance.md': NOTE });
  await s.browser.$('.file-tree-item[data-path="avance.md"]').waitForExist({ timeout: 30_000 });
  await s.browser.execute(() => (document.querySelector('.file-tree-item[data-path="avance.md"]') as HTMLElement).click());
  await paneReady(s);
  await s.browser.$('.markdown-reading-wrapper .markdown-body li input[type="checkbox"]').waitForExist({ timeout: 15_000 });
}, 120_000);

afterAll(async () => {
  await s?.close();
});

/** Selecciona `selector` (todo su contenido) y dispara «copiar»; devuelve lo que quedó en el portapapeles */
const copy = (selector: string) =>
  s.browser.execute((selector) => {
    const el = document.querySelector(selector)!;
    const range = document.createRange();
    range.selectNodeContents(el);
    const sel = window.getSelection()!;
    sel.removeAllRanges();
    sel.addRange(range);
    const data = new DataTransfer();
    const ev = new ClipboardEvent('copy', { clipboardData: data, bubbles: true, cancelable: true });
    el.dispatchEvent(ev);
    return { html: data.getData('text/html'), text: data.getData('text/plain'), prevented: ev.defaultPrevented };
  }, selector) as Promise<{ html: string; text: string; prevented: boolean }>;

it('la lista de tareas se copia con ☐ / ✅, negrita y viñetas', async () => {
  const out = await copy('.markdown-reading-wrapper .markdown-body ul');
  expect(out.prevented).toBe(true);
  expect(out.text).toBe(
    ['- ☐ 00. Modelo de Dato', '- ☐ (faltan 5 de 26) 01. Diagrama de Secuencia', '- ✅ 02. Diagrama de Software'].join('\n'),
  );
  expect(out.html).toStartWith('<ul>');
  expect(out.html).toContain('<strong>(faltan 5 de 26)</strong>');
  expect(out.html).toContain('✅ 02. Diagrama de Software');
  expect(out.html).not.toContain('<input');
  expect(out.html).not.toContain('class=');
}, 30_000);

it('sin casillas se copia como siempre', async () => {
  const out = await copy('.markdown-reading-wrapper .markdown-body p');
  expect(out.prevented).toBe(false);
}, 30_000);
