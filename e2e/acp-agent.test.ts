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

const count = (sel: string) => s.browser.execute((q) => document.querySelectorAll(q).length, sel) as Promise<number>;
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
  // Modo y modelo van junto a la línea de contexto, en un menú ▾ hacia arriba
  await s.browser.waitUntil(async () => (await count('.acp-opts-btn')) > 0, { timeout: 5_000, timeoutMsg: 'no aparece el selector de modo' });
  expect(await count('.acp-subbar select[aria-label="Modo del agente"]')).toBe(0);
  await s.browser.$('.acp-opts-btn').click();
  await s.browser.waitUntil(async () => (await text('.acp-opts-menu button')).includes('Programar'), {
    timeout: 5_000,
    timeoutMsg: 'no aparecen los modos del agente',
  });
  expect(await text('.acp-opts-head')).toEqual(['Modo']);
  await s.browser.execute(() => {
    const b = [...document.querySelectorAll('.acp-opts-menu button')].find((x) => x.textContent?.trim() === 'Programar');
    (b as HTMLButtonElement).click();
  });
  // Un modo distinto del de inicio se ve en corto
  await s.browser.waitUntil(async () => (await text('.acp-mode-chip')).some((t) => t?.includes('Programar')), {
    timeout: 5_000,
    timeoutMsg: 'no se muestra el modo elegido',
  });
  expect(await text('.acp-badge')).toContain('MCP Synapse');
  // «MCP Synapse» y ⋯ van en el título de la conversación; no hay barra aparte
  expect(await count('.acp-row .acp-badge')).toBe(1);
  expect(await count('.acp-row button[aria-label="Más acciones"]')).toBe(1);
  expect(await count('.acp-section .acp-subbar')).toBe(0);

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

  // Contexto al 85 %: «Compactar» se ve resaltado; al compactar aparece la marca y el uso baja
  await s.browser.waitUntil(async () => (await text('.acp-usage-text')).includes('85%'), {
    timeout: 5_000,
    timeoutMsg: 'no se muestra el uso de contexto',
  });
  expect(await count('.acp-compact.shown.warn')).toBe(1);
  await s.browser.$('.acp-compact').click();
  await s.browser.$('.acp-compact-marker').waitForExist({ timeout: 10_000 });
  await s.browser.waitUntil(async () => (await text('.acp-usage-text')).includes('10%'), {
    timeout: 10_000,
    timeoutMsg: 'el uso no bajó al compactar',
  });
  expect((await text('.acp-msg.agent')).some((t) => t?.includes('Resumen compactado'))).toBe(true);
  expect(await count('.acp-compact.shown')).toBe(0);

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
  // Al hacer clic en la miniatura se ve ampliada; Esc la cierra
  await s.browser.$('.acp-composer .acp-chip.image .acp-image-btn').click();
  await s.browser.$('.acp-lightbox img').waitForExist({ timeout: 5_000 });
  await s.browser.keys('Escape');
  await s.browser.waitUntil(async () => (await count('.acp-lightbox')) === 0, { timeout: 5_000, timeoutMsg: 'Esc no cerró la imagen' });

  // La conversación queda guardada en la bóveda y aparece en el historial
  const dir = join(s.vault, '.synapse/agent-sessions');
  await s.browser.waitUntil(async () => existsSync(dir) && readdirSync(dir).some((f) => f.endsWith('.json')), {
    timeout: 5_000,
    timeoutMsg: 'no se guardó la conversación en la bóveda',
  });
  // Acordeón: «＋» abre otra conversación arriba; como mucho 2 desplegadas a la vez
  await s.browser.$('button[aria-label="Nueva conversación"]').click();
  await s.browser.waitUntil(async () => (await count('.acp-section')) === 2, { timeout: 10_000, timeoutMsg: 'no se abrió la segunda conversación' });
  expect(await count('.acp-section.open')).toBe(2);
  await s.browser.$('button[aria-label="Nueva conversación"]').click();
  await s.browser.waitUntil(async () => (await count('.acp-section')) === 3, { timeout: 10_000, timeoutMsg: 'no se abrió la tercera conversación' });
  expect(await count('.acp-section.open')).toBe(2);
  // Las tres conversaciones con el mismo agente comparten un solo proceso
  const procs = Bun.spawnSync(['pgrep', '-fc', 'fake-acp-agent.ts']).stdout.toString().trim();
  expect(procs).toBe('1');

  // Fijar la primera conversación al costado: sale del acordeón a su propia columna
  await s.browser.execute(() => {
    const row = [...document.querySelectorAll('.acp-row')].find((r) => r.textContent?.includes('hola'))!;
    (row.querySelector('button[aria-label="Fijar al costado"]') as HTMLButtonElement).click();
  });
  await s.browser.waitUntil(async () => (await text('.acp-pinned .acp-row-title')).some((t) => t?.includes('hola')), {
    timeout: 5_000,
    timeoutMsg: 'la conversación no quedó fijada al costado',
  });
  expect(await count('.acp-accordion .acp-section')).toBe(2);

  // Cerrarla la manda al historial; reabrirla devuelve sus mensajes
  await s.browser.execute(() => {
    (document.querySelector('.acp-pinned button[aria-label="Cerrar conversación"]') as HTMLButtonElement).click();
  });
  await s.browser.waitUntil(async () => (await text('.acp-history option')).some((t) => t?.includes('hola')), {
    timeout: 5_000,
    timeoutMsg: 'la conversación cerrada no aparece en el historial',
  });
  expect(await count('.acp-pinned')).toBe(0);
  await s.browser.execute(() => {
    const sel = document.querySelector('.acp-history select') as HTMLSelectElement;
    sel.value = [...sel.options].find((o) => o.textContent?.includes('hola'))!.value;
    sel.dispatchEvent(new Event('change', { bubbles: true }));
  });
  await s.browser.waitUntil(async () => (await text('.acp-msg.agent')).some((t) => t?.includes('Hola desde el agente')), {
    timeout: 10_000,
    timeoutMsg: 'no se reabrió la conversación del historial',
  });

  // «Borrar todas» borra las conversaciones cerradas
  await s.browser.execute(() => {
    const row = [...document.querySelectorAll('.acp-row')].find((r) => r.textContent?.includes('hola'))!;
    (row.querySelector('button[aria-label="Cerrar conversación"]') as HTMLButtonElement).click();
  });
  await s.browser.waitUntil(async () => (await text('.acp-history option')).some((t) => t?.includes('hola')), { timeout: 5_000 });
  await s.browser.$('button=Borrar todas').click();
  await s.browser.$('.acp-danger').click();
  await s.browser.waitUntil(async () => readdirSync(dir).filter((f) => f.endsWith('.json')).length === 0, {
    timeout: 5_000,
    timeoutMsg: 'no se borraron las conversaciones',
  });
}, 120_000);

it('al reabrir la bóveda vuelve el panel con su agente y su última conversación', async () => {
  const input = s.browser.$('.acp-input textarea');
  await s.browser.waitUntil(async () => !(await input.getAttribute('disabled')), { timeout: 15_000 });
  await input.setValue('segunda conversación');
  await s.browser.keys('Enter');
  await s.browser.waitUntil(async () => (await text('.acp-msg.agent')).some((t) => t?.includes('Hola desde el agente')), {
    timeout: 10_000,
    timeoutMsg: 'no llegó la respuesta',
  });
  await s.browser.execute(() => {
    const btn = [...document.querySelectorAll('.acp-permission-actions button')].find((b) => b.textContent?.trim() === 'Permitir');
    (btn as HTMLButtonElement | undefined)?.click();
  });
  await s.browser.pause(1_000);

  // Recargar la ventana equivale a volver a abrir la bóveda
  await s.browser.refresh();
  await s.browser.$('.synapse-agent-panel').waitForExist({ timeout: 30_000 });
  await s.browser.waitUntil(async () => (await text('.acp-row-title')).some((t) => t?.includes('segunda conversación')), {
    timeout: 15_000,
    timeoutMsg: 'no se restauró la conversación de la bóveda',
  });
  expect((await text('.acp-row-title')).join(' ')).toContain('Agente falso');
  await s.browser.waitUntil(async () => (await text('.acp-msg.agent')).some((t) => t?.includes('Hola desde el agente')), {
    timeout: 10_000,
    timeoutMsg: 'la conversación restaurada no trae sus mensajes',
  });

  // Con el panel cerrado, la bóveda vuelve con el panel cerrado
  await s.browser.$('.synapse-agent-panel button[title="Cerrar panel"]').click();
  await s.browser.refresh();
  await s.browser.$('.file-tree-item').waitForExist({ timeout: 30_000 });
  await s.browser.pause(1_500);
  expect(await count('.synapse-agent-panel')).toBe(0);
}, 120_000);

it('un agente le pregunta a otra conversación y le vuelve la respuesta', async () => {
  await s.browser.$('.editor-header .agent-toggle').click();
  // Se restaura la conversación de la bóveda (#1) y se abre otra (#2)
  await s.browser.waitUntil(async () => (await count('.acp-section')) >= 1, { timeout: 30_000, timeoutMsg: 'no se restauró el panel' });
  await s.browser.waitUntil(async () => (await text('.acp-row-title')).some((t) => t?.includes('segunda conversación')), {
    timeout: 20_000,
    timeoutMsg: 'no terminó de restaurarse la conversación',
  });
  expect(await text('.acp-handle')).toEqual(['#1']);
  await s.browser.$('button[aria-label="Nueva conversación"]').click();
  await s.browser.waitUntil(async () => (await text('.acp-handle')).includes('#2'), {
    timeout: 15_000,
    timeoutMsg: 'no se abrió la conversación #2',
  });
  // El cuadro de texto de la conversación #2, cuando ya está conectada
  const ready2 = () =>
    s.browser.execute(() => {
      const section = [...document.querySelectorAll('.acp-section')].find((x) => x.querySelector('.acp-handle')?.textContent === '#2');
      const area = section?.querySelector('.acp-input textarea') as HTMLTextAreaElement | null;
      if (!area || area.disabled) return false;
      area.id = 'ask-input';
      return true;
    }) as Promise<boolean>;
  await s.browser.waitUntil(ready2, { timeout: 45_000, timeoutMsg: 'la conversación #2 no quedó conectada' });
  const input = s.browser.$('#ask-input');
  await input.setValue('pregúntale a #1: ¿tiene SignOn el flujo?');
  await s.browser.keys('Enter');

  // #2 lo envía por el MCP; #1 lo recibe marcado como de otro agente y responde
  await s.browser.waitUntil(async () => (await text('.acp-msg.agent')).some((t) => t?.includes('Enviado a #1')), {
    timeout: 20_000,
    timeoutMsg: 'el agente no envió el mensaje',
  });
  expect((await text('.acp-msg.agent')).join(' ')).not.toContain('ERROR');
  await s.browser.waitUntil(async () => (await text('.acp-from')).some((t) => t?.includes('#2')), {
    timeout: 15_000,
    timeoutMsg: 'la conversación #1 no recibió el mensaje',
  });
  // La respuesta de #1 vuelve sola a #2
  await s.browser.waitUntil(async () => (await text('.acp-from')).some((t) => t?.includes('#1')), {
    timeout: 15_000,
    timeoutMsg: 'la respuesta no volvió a #2',
  });
  expect((await text('.acp-msg.user.from-agent')).some((t) => t?.includes('Respuesta de'))).toBe(true);
  expect(await count('.acp-waiting')).toBe(0);
}, 120_000);
