// Panel del agente: se registra un agente ACP personalizado (uno falso) desde Ajustes y se conversa con él:
// nota activa y `@` como adjuntos, comandos `/`, modos, MCP de Synapse inyectado, plan, diff y permisos.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { join } from 'node:path';
import { existsSync, readdirSync } from 'node:fs';
import { startApp, type Session } from './harness';

let s: Session;
const FAKE_AGENT = join(import.meta.dir, 'fixtures/fake-acp-agent.ts');

beforeAll(async () => {
  s = await startApp({ 'nota.md': '# Nota\n', 'otra.md': '# Otra\n', 'flujo.mmd': 'graph TD\n  A[Inicio] --> B[Fin]\n' });
  await s.browser.$('.file-tree-item[data-path="nota.md"]').waitForExist({ timeout: 30_000 });
  await s.browser.$('.file-tree-item[data-path="nota.md"]').click();
}, 120_000);
afterAll(() => s?.close());

const text = (sel: string) =>
  s.browser.execute((q) => [...document.querySelectorAll(q)].map((e) => e.textContent?.trim()), sel) as Promise<
    string[]
  >;

it('registra un agente personalizado y conversa con él', async () => {
  await s.browser.$('.editor-header .agent-toggle').click();
  await s.browser.$('.synapse-agent-panel').waitForExist({ timeout: 5_000 });
  // Por defecto no hay ningún agente elegido
  expect(await s.browser.execute(() => (document.querySelector('.acp-select') as HTMLSelectElement).value)).toBe('');

  // Ajustes → Agentes de IA → agregar el agente falso
  await s.browser.$('.synapse-agent-panel button[title="Ajustes de agentes"]').click();
  await s.browser.$('.agent-form').waitForExist({ timeout: 5_000 });
  await s.browser.$('input[placeholder="Identificador (mi-agente)"]').setValue('falso');
  await s.browser.$('input[placeholder="Nombre"]').setValue('Agente falso');
  await s.browser.$('input[placeholder="Comando (ruta o nombre en el PATH)"]').setValue(process.execPath);
  await s.browser.$('input[placeholder="Argumentos separados por espacios"]').setValue(FAKE_AGENT);
  await s.browser.$('.agent-form button[type="submit"]').click();
  await s.browser.waitUntil(async () => (await text('.agent-row .row-label')).some((t) => t?.startsWith('Agente falso')), {
    timeout: 5_000,
    timeoutMsg: 'el agente personalizado no aparece en Ajustes',
  });
  await s.browser.keys('Escape');

  // Elegir el agente en la cabecera del panel: abre la sesión
  await s.browser.waitUntil(
    async () => (await text('.acp-select option')).some((t) => t === 'Agente falso'),
    { timeout: 5_000, timeoutMsg: 'el agente no aparece en el selector' },
  );
  await s.browser.execute(() => {
    const select = document.querySelector('.acp-select') as HTMLSelectElement;
    select.value = 'falso';
    select.dispatchEvent(new Event('change', { bubbles: true }));
  });
  const input = s.browser.$('.acp-input textarea');
  await s.browser.waitUntil(async () => !(await input.getAttribute('disabled')), {
    timeout: 15_000,
    timeoutMsg: 'el agente no quedó conectado',
  });

  // Modos que anuncia el agente
  await s.browser.waitUntil(async () => (await text('.acp-subbar option')).includes('Programar'), {
    timeout: 5_000,
    timeoutMsg: 'no aparecen los modos del agente',
  });
  expect(await text('.acp-badge')).toContain('MCP Synapse');

  // La nota activa viaja como etiqueta
  expect((await text('.acp-composer .acp-chip')).some((t) => t?.includes('nota.md'))).toBe(true);

  // `/` sugiere los comandos del agente
  await input.setValue('/res');
  await s.browser.waitUntil(async () => (await text('.acp-suggest-label')).includes('/resumir'), {
    timeout: 5_000,
    timeoutMsg: 'no se sugieren los comandos del agente',
  });

  // `@` menciona otra nota y la deja como etiqueta
  await input.setValue('hola @otr');
  await s.browser.waitUntil(async () => (await text('.acp-suggest-detail')).includes('otra.md'), {
    timeout: 5_000,
    timeoutMsg: 'no se sugiere la nota al escribir @',
  });
  await s.browser.keys('Enter');
  await s.browser.waitUntil(async () => (await text('.acp-composer .acp-chip')).some((t) => t?.includes('otra.md')), {
    timeout: 5_000,
    timeoutMsg: 'la nota mencionada no quedó como etiqueta',
  });
  await s.browser.keys('Enter');

  // Respuesta en streaming y petición de permiso
  await s.browser.waitUntil(async () => (await text('.acp-msg.agent')).some((t) => t?.includes('Hola desde el agente')), {
    timeout: 10_000,
    timeoutMsg: 'no llegó la respuesta del agente',
  });
  const reply = (await text('.acp-msg.agent')).join(' ');
  // Llegaron el texto y las dos notas como enlaces, y el MCP de Synapse se inyectó en la sesión
  expect(reply).toContain('link:nota.md');
  expect(reply).toContain('link:otra.md');
  // El agente falso solo habla stdio: llega por el puente a la instancia abierta y ve sus herramientas
  expect(reply).toMatch(/mcp synapse_instance:stdio tools:[1-9]\d*/);
  // Instrucciones de sesión: usar el MCP de esta instancia para las notas
  expect(reply).toContain('[instr:si]');
  // El MCP marca el diagrama y lo devuelve dibujado como PNG por la ventana
  expect(reply).toContain('diagram:mermaid img:image/png:ok');
  // Plan y herramienta con diff
  expect(await text('.acp-plan-entry')).toEqual(['Leer la nota', 'Editar la nota']);
  await s.browser.$('.acp-tool summary').click();
  expect(await text('.acp-diff-line.add')).toContain('+ nuevo');
  expect(await text('.acp-diff-line.del')).toContain('- viejo');
  // Detalles crudos de la herramienta y respuesta en markdown
  expect((await text('.acp-tool .acp-tool-text')).join(' ')).toContain('"relative_path": "nota.md"');
  expect(await s.browser.execute(() => document.querySelector('.acp-msg.agent strong')?.textContent)).toBe('negrita');
  await s.browser.$('.acp-permission').waitForExist({ timeout: 5_000 });
  expect(await text('.acp-permission-title')).toContain('Editar nota.md');
  await s.browser.execute(() => {
    const btn = [...document.querySelectorAll('.acp-permission-actions button')].find((b) => b.textContent?.trim() === 'Permitir');
    (btn as HTMLButtonElement).click();
  });

  await s.browser.waitUntil(
    async () => (await text('.acp-msg.agent')).some((t) => t?.includes('(permiso: allow)')),
    { timeout: 10_000, timeoutMsg: 'el agente no recibió el permiso' },
  );
  expect(await text('.acp-permission-done')).toContain('Permitir');

  // `@this` vuelve a adjuntar el archivo de la pestaña visible
  expect((await text('.acp-composer .acp-chip')).some((t) => t?.includes('nota.md'))).toBe(false);
  await s.browser.$('.acp-input textarea').click();
  await s.browser.keys('@thi'.split(''));
  await s.browser.waitUntil(async () => (await text('.acp-suggest-label')).includes('@this'), {
    timeout: 5_000,
    timeoutMsg: '@this no aparece en las sugerencias',
  });
  await s.browser.keys('Enter');
  await s.browser.waitUntil(async () => (await text('.acp-composer .acp-chip')).some((t) => t?.includes('nota.md')), {
    timeout: 5_000,
    timeoutMsg: '@this no adjuntó la nota activa',
  });

  // Pegar una imagen del portapapeles la deja como adjunto con miniatura
  await s.browser.execute(() => {
    const png = Uint8Array.from(atob('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=='), (c) => c.charCodeAt(0));
    const dt = new DataTransfer();
    dt.items.add(new File([png], 'captura.png', { type: 'image/png' }));
    const ev = new ClipboardEvent('paste', { clipboardData: dt, bubbles: true, cancelable: true });
    document.querySelector('.acp-input textarea')!.dispatchEvent(ev);
  });
  await s.browser.waitUntil(async () => (await text('.acp-composer .acp-chip.image')).some((t) => t?.includes('Imagen 1.png')), {
    timeout: 5_000,
    timeoutMsg: 'la imagen pegada no aparece como adjunto',
  });

  // La conversación queda guardada en la bóveda y aparece en el historial
  const dir = join(s.vault, '.synapse/agent-sessions');
  await s.browser.waitUntil(async () => existsSync(dir) && readdirSync(dir).some((f) => f.endsWith('.json')), {
    timeout: 5_000,
    timeoutMsg: 'no se guardó la conversación en la bóveda',
  });
  await s.browser.waitUntil(
    async () => (await text('.acp-history option')).some((t) => t?.includes('Fake') || t?.includes('·')),
    { timeout: 5_000, timeoutMsg: 'la conversación no aparece en el historial' },
  );
  // Nueva conversación: vacía; borrar todas deja el historial vacío
  await s.browser.$('button[aria-label="Nueva conversación"]').click();
  await s.browser.waitUntil(async () => (await text('.acp-msg.agent')).length === 0, { timeout: 10_000 });
  // Retomar desde el historial devuelve la conversación (y su agente)
  await s.browser.execute(() => {
    const sel = document.querySelector('.acp-history select') as HTMLSelectElement;
    sel.value = [...sel.options].find((o) => o.value && o.value !== '__new__')!.value;
    sel.dispatchEvent(new Event('change', { bubbles: true }));
  });
  await s.browser.waitUntil(async () => (await text('.acp-msg.agent')).some((t) => t?.includes('Hola desde el agente')), {
    timeout: 10_000,
    timeoutMsg: 'no se retomó la conversación del historial',
  });
  expect(await s.browser.execute(() => (document.querySelector('.acp-select') as HTMLSelectElement).value)).toBe('falso');
  await s.browser.$('button=Borrar todas').click();
  await s.browser.$('.acp-danger').click();
  await s.browser.waitUntil(async () => readdirSync(dir).filter((f) => f.endsWith('.json')).length === 0, {
    timeout: 5_000,
    timeoutMsg: 'no se borraron las conversaciones',
  });
}, 120_000);
