// Panel del agente: se registra un agente ACP personalizado (uno falso) desde Ajustes y se conversa con él:
// nota activa y `@` como adjuntos, comandos `/`, modos, MCP de Synapse inyectado, plan, diff y permisos.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { join } from 'node:path';
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
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
  // Cada agente tiene su límite de conversaciones por proceso
  expect(await count('select[aria-label="Conversaciones por proceso de Agente falso"]')).toBe(1);
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
  // Un agente sin icono propio lleva el robot genérico
  expect(await count('.acp-row-title svg.agent-icon')).toBe(1);

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

  // `/usage` lo responde Synapse en el chat, sin pasar por el agente
  await s.browser.$('.acp-input textarea').setValue('/usage');
  await s.browser.waitUntil(async () => (await text('.acp-suggest-label')).includes('/usage'), { timeout: 5_000 });
  await s.browser.keys('Enter');
  await s.browser.waitUntil(async () => (await text('.acp-msg.info')).some((t) => t?.includes('Uso de esta conversación')), {
    timeout: 5_000,
    timeoutMsg: '/usage no mostró el resumen',
  });
  expect((await text('.acp-msg.info')).join(' ')).toContain('10.0 %');

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
  // Y la referencia queda en el texto, para poder hablar de ella
  expect(await s.browser.$('.acp-input textarea').getValue()).toBe('@nota.md ');
  await s.browser.keys(['Control', 'a']);
  await s.browser.keys('Backspace');

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
  // Al hacer clic en la miniatura se abre en una pestaña con el visor de imágenes (zoom, mover)
  await s.browser.$('.acp-composer .acp-chip.image .acp-image-btn').click();
  await s.browser.waitUntil(async () => (await text('.editor-header .tab-title, .editor-header [class*="tab"]')).some((t) => t?.includes('Agente · Imagen 1.png')), {
    timeout: 5_000,
    timeoutMsg: 'la imagen no se abrió en una pestaña',
  });
  await s.browser.$('.agent-image-pane img').waitForExist({ timeout: 5_000 });
  expect(await count('.agent-image-pane .agent-image-save')).toBe(1);

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
  // Respondido el permiso, el agente termina su turno (y la conversación se guarda)
  await s.browser.waitUntil(async () => (await count('.acp-permission-actions button')) === 0, {
    timeout: 10_000,
    timeoutMsg: 'el permiso no quedó respondido',
  });

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
  await s.browser.$('.editor-header .agent-toggle').click();
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
  // `@a:` filtra solo agentes (aquí, la conversación #1) y `@f:` solo archivos
  const suggestionLabels = () => text('.acp-suggest button .acp-suggest-label');
  await input.setValue('@a:');
  await s.browser.waitUntil(async () => (await suggestionLabels()).some((t) => t?.includes('#1')), {
    timeout: 5_000,
    timeoutMsg: '«@a:» no sugirió la conversación #1',
  });
  expect((await suggestionLabels()).every((t) => t?.includes('#') || t?.startsWith('🟢') || t?.startsWith('🟡') || t?.startsWith('⚪'))).toBe(true);
  await input.setValue('@f:');
  await s.browser.waitUntil(async () => (await suggestionLabels()).length > 0, { timeout: 5_000, timeoutMsg: '«@f:» no sugirió archivos' });
  expect((await suggestionLabels()).some((t) => t?.includes('#1'))).toBe(false);
  // `#` sugiere las otras conversaciones de esta ventana, con su título
  await input.setValue('pregúntale a ');
  await s.browser.keys('#');
  await s.browser.waitUntil(async () => (await text('.acp-suggest button')).some((t) => t?.includes('#1') && t.includes('segunda conversación')), {
    timeout: 5_000,
    timeoutMsg: '«#» no sugirió la conversación #1',
  });
  await s.browser.keys('Enter');
  // La mención se inserta con un espacio detrás; se quita para escribir «#1: …»
  await s.browser.keys('Backspace');
  await s.browser.keys(': tiene SignOn el flujo?'.split(''));
  await s.browser.keys('Enter');
  // En el mensaje enviado, la mención se ve como chip y su tooltip dice a quién apunta
  await s.browser.waitUntil(async () => (await count('.acp-msg.user .acp-mention')) > 0, {
    timeout: 5_000,
    timeoutMsg: 'la mención no quedó marcada en el mensaje',
  });
  expect(
    await s.browser.execute(() => document.querySelector('.acp-msg.user .acp-mention')?.getAttribute('title') ?? ''),
  ).toContain('segunda conversación');

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
  // El mensaje de otro agente llega cerrado; al abrirlo se ve renderizado
  const incoming = (await s.browser.execute(() =>
    [...document.querySelectorAll('details.acp-incoming')].map((d) => ({
      open: (d as HTMLDetailsElement).open,
      rendered: !!d.querySelector('.markdown p, .markdown li, .markdown code'),
    })),
  )) as { open: boolean; rendered: boolean }[];
  expect(incoming.length).toBeGreaterThan(0);
  expect(incoming.every((d) => !d.open)).toBe(true);
  expect(incoming.some((d) => d.rendered)).toBe(true);
}, 120_000);

it('un agente consulta otra bóveda con permiso del usuario', async () => {
  // Otra bóveda del usuario, en sus recientes
  const otra = join(s.vault, '..', 'mainframe');
  mkdirSync(join(otra, '.synapse'), { recursive: true });
  writeFileSync(join(otra, 'copybook.md'), '# Copybook TRX01\nCampos de la transferencia.\n');
  const recents = join(s.home, '.config/synapsers/workspaces.json');
  const reg = existsSync(recents) ? JSON.parse(readFileSync(recents, 'utf8')) : { workspaces: [] };
  reg.workspaces.push({ path: otra, display_path: otra, name: 'mainframe', last_opened_ms: 1 });
  writeFileSync(recents, JSON.stringify(reg));

  const area = () =>
    s.browser.execute(() => {
      const t = document.querySelector('.acp-section.open .acp-input textarea') as HTMLTextAreaElement | null;
      if (!t || t.disabled) return false;
      t.id = 'consult-input';
      return true;
    }) as Promise<boolean>;
  await s.browser.waitUntil(area, { timeout: 30_000, timeoutMsg: 'no hay una conversación lista' });
  await s.browser.$('#consult-input').setValue('consulta en mainframe: copybook.md');
  await s.browser.keys('Enter');

  // La primera vez Synapse pregunta si esta bóveda puede leer la otra
  await s.browser.$('.mcp-confirm').waitForExist({ timeout: 15_000 });
  expect((await text('.mcp-confirm p')).some((t) => t?.includes('«mainframe»'))).toBe(true);
  await s.browser.$('button=Permitir').click();
  await s.browser.waitUntil(async () => (await text('.acp-msg.agent')).some((t) => t?.includes('Copybook TRX01')), {
    timeout: 15_000,
    timeoutMsg: 'el agente no leyó la otra bóveda',
  });
  expect((await text('.acp-msg.agent')).some((t) => t?.includes('📚 Bóveda «mainframe»'))).toBe(true);
  expect(existsSync(join(s.vault, '.synapse/vault-access.json'))).toBe(true);
}, 120_000);

it('respuesta con tareas: casillas, copiar como Markdown y con formato, copiar selección y zoom', async () => {
  const input = s.browser.$('.acp-input textarea');
  await s.browser.waitUntil(async () => !(await input.getAttribute('disabled')), { timeout: 15_000 });
  await input.setValue('dame la lista de tareas');
  await s.browser.keys('Enter');
  await s.browser.waitUntil(async () => (await text('.acp-msg.agent')).some((t) => t?.includes('Modelo de Dato')), {
    timeout: 10_000,
    timeoutMsg: 'no llegó la lista de tareas',
  });
  // Las casillas se ven (antes el saneado las quitaba)
  const boxes = (await s.browser.execute(() => {
    const msg = [...document.querySelectorAll('.acp-msg.agent')].find((m) => m.textContent?.includes('Modelo de Dato'))!;
    msg.closest('.acp-agent-wrap')!.setAttribute('data-test', 'tareas');
    return [...msg.querySelectorAll('input[type="checkbox"]')].map((i) => ({ checked: (i as HTMLInputElement).checked, disabled: (i as HTMLInputElement).disabled }));
  })) as { checked: boolean; disabled: boolean }[];
  expect(boxes).toEqual([{ checked: false, disabled: true }, { checked: true, disabled: true }]);

  // El botón y la selección de bloques llevan el Markdown original y, en HTML, ☐ / ✅. Se captura lo que se escribe en el portapapeles
  // (evento copy o ClipboardItem); el botón se pulsa con un clic real (copiar exige un gesto del usuario)
  await s.browser.execute(() => {
    const w = window as any;
    w.__copied = {};
    const orig = DataTransfer.prototype.setData;
    DataTransfer.prototype.setData = function (type: string, data: string) {
      w.__copied[type] = data;
      return orig.call(this, type, data);
    };
    navigator.clipboard.write = async (items: ClipboardItem[]) => {
      for (const it of items) for (const t of it.types) w.__copied[t] = await (await it.getType(t)).text();
    };
  });
  const copyButton = s.browser.$('.acp-agent-wrap[data-test="tareas"] .acp-copy-reply');
  await copyButton.scrollIntoView();
  await copyButton.moveTo();
  await copyButton.click();
  await s.browser.waitUntil(async () => !!(await s.browser.execute(() => (window as any).__copied['text/plain'])), {
    timeout: 5_000,
    timeoutMsg: '«Copiar» no escribió en el portapapeles',
  });
  const captured = (await s.browser.execute(() => {
    const w = window as any;
    const out = [{ html: w.__copied['text/html'] ?? '', text: w.__copied['text/plain'] ?? '' }];
    const msg = document.querySelector('.acp-agent-wrap[data-test="tareas"] .acp-msg.agent')!;
    const ul = msg.querySelector('ul')!;
    const copyOf = (select: (range: Range) => void) => {
      const range = document.createRange();
      select(range);
      const sel = window.getSelection()!;
      sel.removeAllRanges();
      sel.addRange(range);
      const data = new DataTransfer();
      ul.dispatchEvent(new ClipboardEvent('copy', { clipboardData: data, bubbles: true, cancelable: true }));
      out.push({ html: data.getData('text/html'), text: data.getData('text/plain') });
    };
    // Un bloque completo, la respuesta entera y un fragmento (una sola tarea)
    copyOf((r) => r.selectNodeContents(ul));
    copyOf((r) => r.selectNodeContents(msg));
    copyOf((r) => r.selectNodeContents(ul.querySelector('li')!));
    return out;
  })) as { html: string; text: string }[];
  const list = '- [ ] 00. Modelo de Dato\n- [x] **02.** Diagrama de Software';
  const [button, block, whole, fragment] = captured;
  for (const copy of [button, whole]) {
    expect(copy.text).toBe(`## Avance\n\n${list}`);
    expect(copy.html).toContain('<h2>Avance</h2>');
    expect(copy.html).toContain('<strong>02.</strong>');
    expect(copy.html).not.toContain('<input');
  }
  expect(block.text).toBe(list);
  expect(block.html).toContain('<strong>02.</strong>');
  expect(block.html).not.toContain('<h2>');
  // Un fragmento se copia como antes: texto sin marcas
  expect(fragment.text).toBe('☐ 00. Modelo de Dato');
  // Con un bloque seleccionado, el botón sigue copiando la respuesta entera
  await s.browser.execute(() => {
    (window as any).__copied = {};
    const ul = document.querySelector('.acp-agent-wrap[data-test="tareas"] .acp-msg.agent ul')!;
    window.getSelection()!.selectAllChildren(ul);
  });
  await copyButton.click();
  await s.browser.waitUntil(async () => !!(await s.browser.execute(() => (window as any).__copied['text/plain'])), { timeout: 5_000 });
  expect(await s.browser.execute(() => (window as any).__copied['text/plain'])).toBe(`## Avance\n\n${list}`);

  // Zoom del chat: Ctrl++ con el foco en los mensajes, Ctrl+0 vuelve
  const zoom = () => s.browser.execute(() => (document.querySelector('.acp-messages') as HTMLElement).style.zoom) as Promise<string>;
  await s.browser.execute(() => (document.querySelector('.acp-messages') as HTMLElement).focus());
  await s.browser.keys(['Control', '=']);
  await s.browser.waitUntil(async () => (await zoom()) === '1.1', { timeout: 5_000, timeoutMsg: 'Ctrl+= no amplió el chat' });
  await s.browser.keys(['Control', '0']);
  await s.browser.waitUntil(async () => (await zoom()) === '1', { timeout: 5_000, timeoutMsg: 'Ctrl+0 no volvió al 100 %' });

  // La caja de texto crece con el contenido, con tope
  const h1 = (await s.browser.execute(() => (document.querySelector('.acp-input textarea') as HTMLElement).offsetHeight)) as number;
  const longText = Array.from({ length: 40 }, (_, i) => `línea ${i}`).join('\n');
  await s.browser.execute((text: string) => {
    const t = document.querySelector('.acp-input textarea') as HTMLTextAreaElement;
    t.value = text;
    t.dispatchEvent(new Event('input', { bubbles: true }));
  }, longText);
  const grown = (await s.browser.execute(() => {
    const t = document.querySelector('.acp-input textarea') as HTMLElement;
    return { h: t.offsetHeight, max: window.innerHeight * 0.4 };
  })) as { h: number; max: number };
  expect(grown.h).toBeGreaterThan(h1 * 2);
  expect(grown.h).toBeLessThanOrEqual(grown.max + 1);
  await input.clearValue();
}, 60_000);

it('la notificación de fin de turno lleva la respuesta final, o la consulta si así se configura', async () => {
  const input = s.browser.$('.acp-input textarea');
  // Los avisos enviados al sistema, del registro de llamadas IPC
  const notices = () =>
    s.browser.execute(() =>
      ((window as any).__synapseIpcLog ?? []).filter((e: any) => e.cmd === 'notify_if_unfocused').map((e: any) => e.args.body),
    ) as Promise<string[]>;
  const ask = async () => {
    await s.browser.waitUntil(async () => !(await s.browser.execute(() => !!document.querySelector('.acp-send[title^="Detener"]'))), { timeout: 15_000 });
    await s.browser.waitUntil(async () => !(await input.getAttribute('disabled')), { timeout: 15_000 });
    const before = (await notices()).length;
    await input.setValue('dame la lista de tareas');
    await s.browser.keys('Enter');
    await s.browser.waitUntil(async () => (await notices()).length > before, { timeout: 15_000, timeoutMsg: 'no hubo notificación' });
    return (await notices()).at(-1);
  };
  expect(await ask()).toBe('Avance ☐ 00. Modelo de Dato ✅ 02. Diagrama de Software');

  await s.browser.$('.synapse-agent-panel button[title="Ajustes de agentes"]').click();
  const select = s.browser.$('select.agent-notice-content');
  await select.waitForExist({ timeout: 5_000 });
  expect(await select.getValue()).toBe('answer');
  await select.selectByAttribute('value', 'query');
  await s.browser.keys('Escape');
  await s.browser.$('.settings-panel').waitForExist({ reverse: true, timeout: 5_000 });
  expect(await ask()).toBe('Terminó la consulta: "dame la lista de tareas"');
}, 60_000);

it('cola: mientras trabaja, Enter encola; ✕ quita; al terminar se envía; ↑ edita; Detener lo devuelve a la caja', async () => {
  const input = s.browser.$('.acp-input textarea');
  const queued = () => text('.acp-queued-text');
  const say = async (msg: string) => {
    await input.setValue(msg);
    await s.browser.keys('Enter');
  };
  await s.browser.waitUntil(async () => !(await s.browser.execute(() => !!document.querySelector('.acp-send[title^="Detener"]'))), { timeout: 15_000 });
  await say('trabajo lento');
  await s.browser.$('.acp-send[title^="Detener"]').waitForExist({ timeout: 5_000 });
  await say('eco: uno');
  await say('eco: dos');
  await s.browser.waitUntil(async () => (await queued()).length === 2, { timeout: 5_000, timeoutMsg: 'no se encolaron los dos mensajes' });
  // ✕ quita «dos» de la cola
  await s.browser.execute(() => {
    const row = [...document.querySelectorAll('.acp-queued')].find((r) => r.textContent?.includes('eco: dos'))!;
    (row.querySelector('button') as HTMLButtonElement).click();
  });
  expect(await queued()).toEqual(['eco: uno']);
  // Al terminar el turno se envía «uno» (y «dos» no)
  await s.browser.waitUntil(async () => (await text('.acp-msg.agent')).some((t) => t?.includes('Recibido «eco: uno»')), {
    timeout: 10_000,
    timeoutMsg: 'no se envió lo encolado al terminar el turno',
  });
  expect(await queued()).toEqual([]);
  await s.browser.pause(500);
  expect((await text('.acp-msg.agent')).some((t) => t?.includes('eco: dos'))).toBe(false);

  // ↑ con la caja vacía saca el último de la cola para editarlo; Detener devuelve la cola a la caja
  await s.browser.waitUntil(async () => !(await s.browser.execute(() => !!document.querySelector('.acp-send[title^="Detener"]'))), { timeout: 10_000 });
  await say('trabajo muy lento');
  await s.browser.$('.acp-send[title^="Detener"]').waitForExist({ timeout: 5_000 });
  await say('eco: tres');
  await s.browser.waitUntil(async () => (await queued()).length === 1, { timeout: 5_000 });
  await s.browser.keys('ArrowUp');
  await s.browser.waitUntil(async () => (await input.getValue()) === 'eco: tres', { timeout: 5_000, timeoutMsg: '↑ no sacó el mensaje de la cola' });
  expect(await queued()).toEqual([]);
  await s.browser.keys('Enter');
  await s.browser.waitUntil(async () => (await queued()).length === 1, { timeout: 5_000, timeoutMsg: 'no se volvió a encolar' });
  await s.browser.$('.acp-send[title^="Detener"]').click();
  await s.browser.waitUntil(async () => (await input.getValue()) === 'eco: tres', { timeout: 5_000, timeoutMsg: 'Detener no devolvió la cola a la caja' });
  expect(await queued()).toEqual([]);
  await s.browser.pause(1_000);
  expect((await text('.acp-msg.agent')).some((t) => t?.includes('Recibido «eco: tres»'))).toBe(false);
  await input.clearValue();
}, 60_000);

it('tus mensajes se ven en Markdown y «Copiar» copia el texto tal cual', async () => {
  const input = s.browser.$('.acp-input textarea');
  await s.browser.waitUntil(async () => !(await s.browser.execute(() => !!document.querySelector('.acp-send[title^="Detener"]'))), { timeout: 15_000 });
  await input.setValue('eco: **fuerte** y `codigo`');
  await s.browser.keys('Enter');
  await s.browser.waitUntil(async () => (await text('.acp-msg.agent')).some((t) => t?.includes('Recibido «eco: fuerte')), { timeout: 10_000 });
  const html = (await s.browser.execute(() => {
    const msg = [...document.querySelectorAll('.acp-msg.user')].find((m) => m.textContent?.includes('fuerte'))!;
    return { strong: msg.querySelector('strong')?.textContent, code: msg.querySelector('code')?.textContent, html: msg.innerHTML.slice(0, 300) };
  })) as { strong?: string; code?: string; html: string };
  expect({ strong: html.strong, code: html.code }).toEqual({ strong: 'fuerte', code: 'codigo' });
  // «Copiar» copia el texto original (Markdown), para reusarlo
  await s.browser.execute(() => {
    (window as any).__copiedText = null;
    navigator.clipboard.writeText = async (t: string) => {
      (window as any).__copiedText = t;
    };
    const wrap = [...document.querySelectorAll('.acp-user-wrap')].find((w) => w.textContent?.includes('fuerte'))!;
    (wrap.querySelector('.acp-copy-user') as HTMLElement).click();
  });
  await s.browser.waitUntil(async () => (await s.browser.execute(() => (window as any).__copiedText)) === 'eco: **fuerte** y `codigo`', {
    timeout: 5_000,
    timeoutMsg: '«Copiar» no copió el texto original',
  });
}, 30_000);

it('los enlaces file:// del agente abren el archivo de la bóveda y saltan a la línea', async () => {
  const input = s.browser.$('.acp-input textarea');
  await s.browser.waitUntil(async () => !(await s.browser.execute(() => !!document.querySelector('.acp-send[title^="Detener"]'))), { timeout: 15_000 });
  await input.setValue('dame enlaces a archivos');
  await s.browser.keys('Enter');
  await s.browser.waitUntil(async () => (await text('.acp-msg.agent')).some((t) => t?.includes('flujo.mmd#L2')), { timeout: 10_000 });
  // El enlace conserva su destino (antes el saneado lo quitaba)
  const href = (await s.browser.execute(() =>
    [...document.querySelectorAll('.acp-msg.agent a')].find((a) => a.textContent === 'flujo.mmd#L2')?.getAttribute('href'),
  )) as string | null;
  expect(href).toMatch(/^file:\/\/.+\/flujo\.mmd#L2$/);
  await s.browser.execute(() =>
    ([...document.querySelectorAll('.acp-msg.agent a')].find((a) => a.textContent === 'flujo.mmd#L2') as HTMLElement).click(),
  );
  await s.browser.waitUntil(
    async () => (await s.browser.execute(() => document.querySelector('.tab-title-container.active')?.getAttribute('data-path'))) === 'flujo.mmd',
    { timeout: 10_000, timeoutMsg: 'no abrió flujo.mmd' },
  );
  // Ruta de otra copia (fuera de la bóveda): se busca por el final de la ruta en el índice
  await s.browser.execute(() => (document.querySelector('.file-tree-item[data-path="nota.md"]') as HTMLElement).click());
  await s.browser.waitUntil(
    async () => (await s.browser.execute(() => document.querySelector('.tab-title-container.active')?.getAttribute('data-path'))) === 'nota.md',
    { timeout: 10_000 },
  );
  await s.browser.execute(() =>
    ([...document.querySelectorAll('.acp-msg.agent a')].find((a) => a.textContent === 'otra copia') as HTMLElement).click(),
  );
  await s.browser.waitUntil(
    async () => (await s.browser.execute(() => document.querySelector('.tab-title-container.active')?.getAttribute('data-path'))) === 'flujo.mmd',
    { timeout: 10_000, timeoutMsg: 'no encontró flujo.mmd por el final de la ruta' },
  );
}, 30_000);

it('muestra «Razonamiento…» al colapsar y evita espacio extra al expandir antes de herramientas', async () => {
  const input = s.browser.$('.acp-input textarea');
  await s.browser.waitUntil(
    async () => !(await s.browser.execute(() => !!document.querySelector('.acp-send[title^="Detener"]'))),
    { timeout: 15_000 },
  );
  await input.setValue('razonamiento y herramienta');
  await s.browser.keys('Enter');

  await s.browser.$('.acp-thought').waitForExist({ timeout: 10_000 });
  await s.browser.waitUntil(
    async () => (await text('.acp-msg.agent')).some((t) => t?.includes('Listo.')),
    { timeout: 10_000 },
  );

  // Colapsado: se ve «Razonamiento…» (no solo el emoji)
  const collapsed = (await s.browser.execute(() => {
    const t = document.querySelector('.acp-thought') as HTMLDetailsElement | null;
    const label = t?.querySelector('.acp-thought-label');
    const ellipsis = t?.querySelector('.acp-thought-ellipsis');
    const labelStyle = label ? getComputedStyle(label) : null;
    const ellipsisStyle = ellipsis ? getComputedStyle(ellipsis) : null;
    return {
      open: !!t?.open,
      summaryText: t?.querySelector('summary')?.textContent?.replace(/\s+/g, ' ').trim() ?? '',
      labelVisible: labelStyle?.display !== 'none',
      ellipsisVisible: ellipsisStyle?.display !== 'none',
    };
  })) as { open: boolean; summaryText: string; labelVisible: boolean; ellipsisVisible: boolean };

  expect(collapsed.open).toBe(false);
  expect(collapsed.labelVisible).toBe(true);
  expect(collapsed.ellipsisVisible).toBe(true);
  expect(collapsed.summaryText).toContain('Razonamiento…');

  // Expandido: desaparecen los puntos suspensivos, se recorta el cuerpo y no queda hueco extra con la herramienta siguiente
  await s.browser.execute(() => (document.querySelector('.acp-thought summary') as HTMLElement).click());
  const expanded = (await s.browser.execute(() => {
    const t = document.querySelector('.acp-thought') as HTMLDetailsElement;
    const tool = t.nextElementSibling as HTMLElement | null;
    const ellipsis = t.querySelector('.acp-thought-ellipsis')!;
    const body = t.querySelector('.acp-thought-body')!;
    return {
      open: t.open,
      ellipsisDisplay: getComputedStyle(ellipsis).display,
      bodyText: body.textContent ?? '',
      gapPx: tool ? tool.getBoundingClientRect().top - t.getBoundingClientRect().bottom : -1,
    };
  })) as { open: boolean; ellipsisDisplay: string; bodyText: string; gapPx: number };

  expect(expanded.open).toBe(true);
  expect(expanded.ellipsisDisplay).toBe('none');
  expect(expanded.bodyText).toBe('Analizando la nota.');
  expect(expanded.gapPx).toBeGreaterThanOrEqual(0);
  expect(expanded.gapPx).toBeLessThanOrEqual(14);
}, 30_000);

it('soporta elicitation/create en modo form y modo url', async () => {
  const input = s.browser.$('.acp-input textarea');
  await s.browser.waitUntil(
    async () => !(await s.browser.execute(() => !!document.querySelector('.acp-send[title^="Detener"]'))),
    { timeout: 15_000 },
  );
  await input.setValue('pedir elicitación');
  await s.browser.keys('Enter');

  // 1. Modo form: aparecen los campos del schema y el botón Aceptar está deshabilitado hasta llenar requeridos
  await s.browser.$('.acp-elicitation[data-mode="form"]').waitForExist({ timeout: 10_000 });
  const initiallyDisabled = await s.browser.execute(
    () => (document.querySelector('.acp-elicitation[data-mode="form"] button[type="submit"]') as HTMLButtonElement)?.disabled,
  );
  expect(initiallyDisabled).toBe(true);

  // Elegir opción «staging», escribir nombre y marcar checkbox
  await s.browser.execute(() => {
    const pill = [...document.querySelectorAll('.acp-elicitation-pill')].find((b) => b.textContent?.trim() === 'staging') as HTMLElement;
    pill.click();
  });
  await s.browser.$('.acp-elicitation-field[data-field="nombre"] input').setValue('demo');
  await s.browser.execute(() => {
    const chk = document.querySelector('.acp-elicitation-field[data-field="confirmar"] input[type="checkbox"]') as HTMLInputElement;
    chk.click();
  });
  await s.browser.execute(() => {
    (document.querySelector('.acp-elicitation[data-mode="form"] button[type="submit"]') as HTMLButtonElement).click();
  });

  // 2. Modo url: muestra dominio y botón Abrir enlace; el usuario rechaza
  await s.browser.$('.acp-elicitation[data-mode="url"]').waitForExist({ timeout: 10_000 });
  const domain = await s.browser.execute(
    () => document.querySelector('.acp-elicitation[data-mode="url"] .acp-elicitation-domain')?.textContent?.trim(),
  );
  expect(domain).toBe('auth.example.com');

  await s.browser.execute(() => {
    const btn = [...document.querySelectorAll('.acp-elicitation[data-mode="url"] button')].find(
      (b) => b.textContent?.trim() === 'Rechazar',
    ) as HTMLElement;
    btn.click();
  });

  // 3. El agente recibe ambas respuestas y responde con el resumen
  await s.browser.waitUntil(
    async () =>
      (await text('.acp-msg.agent')).some((t) =>
        t?.includes('Elicitación OK [accept:staging:demo:true] [url:decline]'),
      ),
    { timeout: 10_000, timeoutMsg: 'no llegó la confirmación de elicitación del agente' },
  );
}, 30_000);

it('/btw pregunta aparte con el turno en curso, sin dejar rastro; se convierte en conversación; /fork hereda el contexto', async () => {
  const input = s.browser.$('.acp-input textarea');
  const say = async (msg: string) => {
    await input.setValue(msg);
    await s.browser.keys('Enter');
  };
  const working = () => s.browser.execute(() => !!document.querySelector('.acp-send[title^="Detener"]'));
  const answered = () =>
    s.browser.waitUntil(async () => (await text('.acp-side .acp-msg.agent')).some((t) => t?.startsWith('Respuesta aparte')), {
      timeout: 20_000,
      timeoutMsg: 'no llegó la respuesta aparte',
    });
  await s.browser.waitUntil(async () => !(await working()), { timeout: 15_000 });
  await say('trabajo muy lento');
  await s.browser.$('.acp-send[title^="Detener"]').waitForExist({ timeout: 5_000 });
  const rows = await count('.acp-row');
  await say('/btw ¿qué haces?');
  await answered();
  // Con contexto y con el permiso rechazado solo; no se encola ni entra en la conversación
  expect((await text('.acp-side .acp-msg.agent')).join()).toContain('con contexto, permiso: reject');
  expect((await text('.acp-side .acp-msg.info')).join()).toContain('Permiso rechazado');
  expect(await count('.acp-queued')).toBe(0);
  expect((await text('.acp-chat:not(.acp-side .acp-chat) > .acp-messages .acp-msg.user')).join()).not.toContain('¿qué haces?');
  // El mismo chat que el principal: razonamiento, herramienta y contexto usado; sin modo ni cuota
  expect(await count('.acp-side .acp-thought')).toBe(1);
  expect((await text('.acp-side .acp-tool')).join()).toContain('Leer aparte');
  expect(await text('.acp-side .acp-usage-text')).toEqual(['25%']);
  expect(await count('.acp-side .acp-quota')).toBe(0);
  // Lo que se le mandó para empezarla, plegado
  expect(await text('.acp-side .acp-context summary')).toEqual([expect.stringContaining('Enviado al agente para iniciar este chat')]);
  expect(await s.browser.execute(() => (document.querySelector('.acp-side .acp-context') as HTMLDetailsElement).open)).toBe(false);
  // Se arrastra por la cabecera para dejar libre el chat, y el lugar se recuerda
  const place = () => s.browser.execute(() => {
    const r = document.querySelector('.acp-side')!.getBoundingClientRect();
    return { x: Math.round(r.x), y: Math.round(r.y) };
  }) as Promise<{ x: number; y: number }>;
  const before = await place();
  const grip = (await s.browser.execute(() => {
    const r = document.querySelector('.acp-side-title, .acp-side-head')!.getBoundingClientRect();
    return { x: Math.round(r.x + 20), y: Math.round(r.y + r.height / 2) };
  })) as { x: number; y: number };
  await s.browser
    .action('pointer')
    .move({ x: grip.x, y: grip.y })
    .down()
    .move({ x: grip.x - 80, y: grip.y + 60, duration: 100 })
    .up()
    .perform();
  const after = await place();
  expect(after).toEqual({ x: before.x - 80, y: before.y + 60 });
  expect(JSON.parse((await s.browser.execute(() => localStorage.getItem('synapse.agent.aside.window'))) as string)).toMatchObject(after);
  // Dentro no hay otra `/btw` ni `/fork`
  const aside = s.browser.$('.acp-side .acp-input textarea');
  await aside.setValue('/bt');
  expect(await count('.acp-side .acp-suggest-label')).toBe(0);
  await aside.setValue('/btw otra');
  await s.browser.keys('Enter');
  await s.browser.waitUntil(async () => (await text('.acp-side .acp-msg.info')).some((t) => t?.includes('no está disponible')), { timeout: 5_000 });
  await aside.clearValue();
  // En el chat principal, `/btw` avisa de que no es un comando del agente
  await input.setValue('/bt');
  await s.browser.waitUntil(async () => (await count('.acp-suggest button')) > 0, { timeout: 5_000 });
  expect(await s.browser.execute(() => document.querySelector('.acp-suggest button')!.getAttribute('title'))).toContain('no es un comando nativo');
  await input.clearValue();
  expect(await count('.acp-row')).toBe(rows);
  const answers = async () => (await text('.acp-side .acp-msg.agent')).filter((t) => t?.startsWith('Respuesta aparte'));
  const session = (await answers())[0]?.match(/\[(ses-e2e-\d+)\]/)?.[1];
  expect(session).toBeDefined();
  // Esc la recoge en una tarjeta junto a los adjuntos, sin borrarla
  await input.click();
  await s.browser.keys('Escape');
  expect(await count('.acp-side')).toBe(0);
  expect(await text('.acp-chips .acp-aside-card')).toEqual(['btw · 1']);
  // La tarjeta la reabre y desde su caja se repregunta en la misma sesión
  await s.browser.$('.acp-aside-card').click();
  await s.browser.$('.acp-side .acp-input textarea').waitForExist({ timeout: 5_000 });
  expect(await count('.acp-aside-card')).toBe(0);
  await s.browser.$('.acp-side .acp-input textarea').setValue('¿y luego?');
  await s.browser.keys('Enter');
  await s.browser.waitUntil(async () => (await answers()).length === 2, { timeout: 20_000, timeoutMsg: 'no llegó la repregunta' });
  expect((await answers())[1]).toContain(`permiso: reject) [${session}]`);
  expect(await text('.acp-side .acp-msg.user.markdown')).toEqual(['¿qué haces?', '¿y luego?']);
  // Esc dentro del panel también la recoge
  await s.browser.$('.acp-side .acp-input textarea').click();
  await s.browser.keys('Escape');
  expect(await count('.acp-side')).toBe(0);
  await s.browser.waitUntil(async () => !(await working()), { timeout: 15_000, timeoutMsg: 'el turno principal no terminó' });
  expect((await text('.acp-msg.agent')).some((t) => t?.includes('Empiezo un trabajo lento… terminado.'))).toBe(true);

  // Otra pregunta aparte: pestañas en el panel; la papelera borra solo la que está a la vista
  await say('/btw ¿y ahora?');
  await s.browser.waitUntil(async () => (await count('.acp-side-tab')) === 2 && (await answers()).length === 1, { timeout: 20_000, timeoutMsg: 'no hay dos pestañas' });
  await s.browser.$('.acp-side-remove').click();
  await s.browser.waitUntil(async () => (await text('.acp-side .acp-msg.user.markdown')).join() === '¿qué haces?,¿y luego?', { timeout: 5_000, timeoutMsg: 'no se borró' });
  expect(await count('.acp-side-tab')).toBe(0);
  await s.browser.$('.acp-side-minimize').click();
  expect(await text('.acp-aside-card')).toEqual(['btw · 1']);

  // Convertirla en conversación
  await s.browser.$('.acp-aside-card').click();
  await s.browser.$('.acp-side-promote:not([disabled])').click();
  await s.browser.waitUntil(async () => (await count('.acp-row')) === rows + 1, { timeout: 5_000, timeoutMsg: 'no se convirtió' });
  expect(await count('.acp-side')).toBe(0);
  expect(await count('.acp-aside-card')).toBe(0);
  expect((await text('.acp-fork-marker')).length).toBe(1);
  expect((await text('.acp-msg.agent')).some((t) => t?.startsWith('Respuesta aparte'))).toBe(true);

  // /fork con primer mensaje
  await say('/fork eco bifurcado');
  await s.browser.waitUntil(async () => (await text('.acp-msg.agent')).some((t) => t === 'Bifurcada con contexto'), {
    timeout: 20_000,
    timeoutMsg: 'la bifurcación no recibió el contexto',
  });
  expect(await count('.acp-row')).toBe(rows + 2);
}, 90_000);

it('lo escrito sin enviar sigue ahí al cerrar el panel y volver a abrirlo', async () => {
  await s.browser.$('.acp-input textarea').setValue('borrador a medias');
  // Sin fijar, un clic fuera lo cierra
  if (await count('.acp-pin-btn[aria-pressed="true"]')) await s.browser.$('.acp-pin-btn').click();
  await s.browser.$('.file-tree-item[data-path="otra.md"]').click();
  await s.browser.waitUntil(async () => (await count('.synapse-agent-panel:not(.is-hidden)')) === 0, { timeout: 5_000, timeoutMsg: 'el panel no se cerró' });
  await s.browser.$('.editor-header .agent-toggle').click();
  await s.browser.$('.synapse-agent-panel:not(.is-hidden)').waitForExist({ timeout: 5_000 });
  expect(await s.browser.$('.acp-input textarea').getValue()).toBe('borrador a medias');
}, 30_000);

it('cerrar el panel solo lo oculta: conserva el scroll y, si llegó algo nuevo, baja al final', async () => {
  // Deja una conversación desplegada con mensajes de sobra para hacer scroll
  const load = (n: number) =>
    s.browser.execute((n) => {
      (async () => {
        const { agentStore } = await new Function("return import('/src/entities/acp-agent/index.ts')")();
        const c = agentStore.conversations.find((x: { id: string }) => agentStore.expanded.includes(x.id));
        c.items = Array.from({ length: n }, (_, i) => ({ type: 'agent', id: 9000 + i, text: `largo ${i}\n\nmás texto` }));
      })();
    }, n);
  const view = () =>
    s.browser.execute(() => {
      const last = [...document.querySelectorAll('.acp-accordion .acp-msg.agent')].find((m) => m.textContent?.includes('largo 0'));
      let el = last?.parentElement as HTMLElement | null;
      while (el && !/auto|scroll/.test(getComputedStyle(el).overflowY)) el = el.parentElement;
      const w = window as unknown as { __node?: Element };
      const same = !w.__node || w.__node === last;
      w.__node = last;
      return { n: document.querySelectorAll('.acp-accordion .acp-msg.agent').length, top: el?.scrollTop ?? -1, max: el ? el.scrollHeight - el.clientHeight : 0, same };
    }) as Promise<{ n: number; top: number; max: number; same: boolean }>;
  const setOpen = async (open: boolean) => {
    await s.browser.execute((open) => {
      const shown = !!document.querySelector('.synapse-agent-panel:not(.is-hidden)');
      if (shown !== open) (document.querySelector('.editor-header .agent-toggle') as HTMLElement).click();
    }, open);
    await s.browser.waitUntil(async () => (await count('.synapse-agent-panel:not(.is-hidden)')) === (open ? 1 : 0), { timeout: 5_000 });
  };
  await setOpen(true);
  await load(60);
  await s.browser.waitUntil(async () => { const v = await view(); return v.n >= 60 && v.max > 300 && v.top > v.max - 5; }, { timeout: 10_000, timeoutMsg: 'no bajó al final' });
  await s.browser.execute(() => {
    let el = document.querySelector('.acp-accordion .acp-msg.agent')!.parentElement as HTMLElement | null;
    while (el && !/auto|scroll/.test(getComputedStyle(el).overflowY)) el = el.parentElement;
    el!.scrollTop = 120;
  });
  await setOpen(false);
  await setOpen(true);
  await s.browser.pause(300);
  let v = await view();
  expect(v.same).toBe(true); // los mismos nodos, sin volver a dibujar
  expect(v.top).toBe(120);
  await s.browser.waitUntil(async () => !(await count('.synapse-agent-panel[inert]')), { timeout: 5_000, timeoutMsg: 'abierto sigue sin admitir el foco' });
  // Algo nuevo con el panel oculto: al mostrarlo se ve el final
  await setOpen(false);
  // Oculto queda fuera de la ventana y no admite el foco
  const hidden = await s.browser.execute(() => {
    const p = document.querySelector('.synapse-agent-panel') as HTMLElement;
    p.querySelector('textarea')!.focus();
    return [p.getBoundingClientRect().left >= innerWidth, p.contains(document.activeElement), document.documentElement.scrollWidth <= innerWidth];
  });
  expect(hidden).toEqual([true, false, true]);
  await load(61);
  await s.browser.pause(500);
  await setOpen(true);
  await s.browser.waitUntil(async () => { v = await view(); return v.n >= 61 && v.top > v.max - 5; }, { timeout: 5_000, timeoutMsg: 'no bajó al final al mostrarlo' });
}, 60_000);

it('Ctrl+F en el panel busca en la conversación, sin distinguir tildes y también en lo plegado', async () => {
  const open = () =>
    s.browser.execute(() => {
      (async () => {
        const { agentStore } = await new Function("return import('/src/entities/acp-agent/index.ts')")();
        const c = agentStore.conversations.find((x: { id: string }) => agentStore.expanded.includes(x.id));
        c.items = [
          { type: 'agent', id: 9901, text: 'primera canción' },
          ...Array.from({ length: 60 }, (_, i) => ({ type: 'agent', id: 9910 + i, text: `relleno ${i}` })),
          { type: 'thought', id: 9980, text: 'pienso en una canción plegada' },
          { type: 'tool', id: 9979, toolCallId: 'z', title: 'leer', status: 'completed', kind: 'read', locations: [], content: [], rawInput: null, rawOutput: 'resultado con canción' },
          { type: 'agent', id: 9981, text: 'otra can**CION** final' },
        ];
      })();
    });
  await open();
  await s.browser.waitUntil(async () => (await text('.acp-msg.agent')).some((t) => t?.includes('relleno 59')), { timeout: 5_000 });
  await s.browser.execute(() => {
    const m = [...document.querySelectorAll('.acp-msg.agent')].find((x) => x.textContent?.includes('relleno 0'))!;
    (m.closest('.acp-messages') as HTMLElement).focus();
  });
  await s.browser.keys(['Control', 'f']);
  const input = s.browser.$('.find-input');
  await input.waitForExist({ timeout: 5_000, timeoutMsg: 'Ctrl+F no abrió la búsqueda' });
  await s.browser.waitUntil(() => input.isFocused(), { timeout: 5_000, timeoutMsg: 'la búsqueda no tiene el foco' });
  await s.browser.keys('cancion');
  const counter = async () => (await text('.find-count'))[0];
  await s.browser.waitUntil(async () => (await counter()) === '3/3', { timeout: 5_000, timeoutMsg: 'no encontró las tres' });
  const marked = () =>
    s.browser.execute(() => ['synapse-search', 'synapse-search-active'].map((n) => CSS.highlights.get(n)?.size ?? 0));
  // Se resaltan las que se ven (lo plegado y lo lejano no), siempre con la actual
  expect((await marked())[1]).toBe(1);
  expect((await marked())[0]).toBeLessThanOrEqual(2);
  // Anterior: abre el razonamiento plegado; al salir de él se vuelve a plegar
  const thoughtOpen = () =>
    s.browser.execute(() => [...document.querySelectorAll('.acp-thought')].some((d) => (d as HTMLDetailsElement).open && d.textContent!.includes('plegada')));
  await s.browser.keys(['Shift', 'Enter']);
  await s.browser.waitUntil(async () => (await counter()) === '2/3' && (await thoughtOpen()), { timeout: 5_000, timeoutMsg: 'no abrió lo plegado' });
  await s.browser.keys('ArrowUp');
  await s.browser.waitUntil(async () => (await counter()) === '1/3' && !(await thoughtOpen()), { timeout: 5_000, timeoutMsg: 'no volvió a plegarlo' });
  // La coincidencia queda a la vista
  const inView = await s.browser.execute(() => {
    const m = [...document.querySelectorAll('.acp-msg.agent')].find((x) => x.textContent?.includes('primera canción'))!;
    const a = m.getBoundingClientRect();
    const b = m.closest('.acp-messages')!.getBoundingClientRect();
    return a.bottom > b.top && a.top < b.bottom;
  });
  expect(inView).toBe(true);
  // El botón incluye el resultado de las herramientas; se recuerda
  await s.browser.$('.acp-search-tools').click();
  await s.browser.waitUntil(async () => (await counter()) === '4/4', { timeout: 5_000, timeoutMsg: 'no incluyó las herramientas' });
  await s.browser.keys(['Shift', 'Enter']);
  const toolOpen = () => s.browser.execute(() => [...document.querySelectorAll('.acp-tool')].some((d) => (d as HTMLDetailsElement).open && d.textContent!.includes('resultado con')));
  await s.browser.waitUntil(async () => (await counter()) === '3/4' && (await toolOpen()), { timeout: 5_000, timeoutMsg: 'no abrió la herramienta' });
  expect(await s.browser.execute(() => localStorage.getItem('synapse.agent.search-tools'))).toBe('1');
  await s.browser.$('.acp-search-tools').click();
  await s.browser.waitUntil(async () => (await counter()) === '3/3' && !(await toolOpen()), { timeout: 5_000, timeoutMsg: 'no las excluyó' });
  // Con muchísimas coincidencias (una letra, en salidas largas) solo se pintan las que se ven: no se congela
  await s.browser.execute(() => {
    (async () => {
      const { agentStore } = await new Function("return import('/src/entities/acp-agent/index.ts')")();
      const c = agentStore.conversations.find((x: { id: string }) => agentStore.expanded.includes(x.id));
      const out = Array.from({ length: 400 }, (_, i) => `línea ${i} de la salida de una herramienta`).join('\n');
      c.items = [
        ...c.items,
        ...Array.from({ length: 6 }, (_, i) => ({ type: 'tool', id: 9990 + i, toolCallId: `g${i}`, title: 'leer', status: 'completed', kind: 'read', locations: [], content: [], rawInput: null, rawOutput: out })),
      ];
    })();
  });
  await s.browser.$('.acp-search-tools').click();
  await s.browser.$('.find-input').click();
  await s.browser.keys(['Control', 'a']);
  await s.browser.keys('a');
  const started = Date.now();
  await s.browser.waitUntil(async () => Number((await counter())?.split('/')[1]) > 10_000, { timeout: 10_000, timeoutMsg: 'no contó todas' });
  expect(Date.now() - started).toBeLessThan(5_000);
  expect(await marked()).toEqual(expect.arrayContaining([1]));
  expect((await marked())[0]).toBeLessThanOrEqual(150);
  await s.browser.$('.acp-search-tools').click();
  // Esc cierra la búsqueda, no el panel
  await s.browser.keys('Escape');
  await input.waitForExist({ reverse: true, timeout: 5_000 });
  expect(await count('.synapse-agent-panel:not(.is-hidden)')).toBe(1);
  expect(await marked()).toEqual([0, 0]);
}, 60_000);

it('renombra la sesión en el agente y guarda con la conversación su nombre y sus preguntas aparte', async () => {
  // Una conversación nueva, sola
  await s.browser.execute(() => {
    (async () => {
      const { agentStore } = await new Function("return import('/src/entities/acp-agent/index.ts')")();
      for (const c of [...agentStore.conversations]) await agentStore.close(c.id);
      await agentStore.newConversation(agentStore.cwd, 'falso');
    })();
  });
  const input = s.browser.$('.acp-input textarea');
  await s.browser.waitUntil(async () => (await count('.acp-row')) === 1 && (await input.isExisting()) && !(await input.getAttribute('disabled')), {
    timeout: 15_000,
    timeoutMsg: 'no se abrió la conversación',
  });
  const say = async (msg: string) => {
    await input.setValue(msg);
    await s.browser.keys('Enter');
  };
  await say('razonamiento y herramienta');
  await s.browser.waitUntil(async () => (await text('.acp-msg.agent')).some((t) => t?.includes('Listo.')), { timeout: 10_000, timeoutMsg: 'no respondió' });
  await say('/btw ¿guardada?');
  await s.browser.waitUntil(async () => (await text('.acp-side .acp-msg.agent')).some((t) => t?.startsWith('Respuesta aparte')), { timeout: 20_000 });
  await s.browser.$('.acp-side-minimize').click();

  // El menú muestra el id de la sesión y deja renombrarla
  await s.browser.$('.acp-row button[aria-label="Más acciones"]').click();
  const id = (await text('.acp-session-id'))[0]!;
  expect(id).toMatch(/^ses-e2e-\d+$/);
  // El propio id es el botón que lo copia (sin una entrada aparte): al pulsarlo, copia y cierra el menú
  await s.browser.execute(() => {
    (window as any).__copied = null;
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText: async (v: string) => void ((window as any).__copied = v) } });
  });
  await s.browser.$('.acp-session-id').click();
  expect(await s.browser.execute(() => (window as any).__copied)).toBe(id);
  await s.browser.$('.acp-menu').waitForExist({ reverse: true, timeout: 5_000 });
  await s.browser.$('.acp-row button[aria-label="Más acciones"]').click();
  const item = (label: string) =>
    s.browser.execute((label) => {
      const b = [...document.querySelectorAll('.acp-menu button')].find((x) => x.textContent?.trim() === label);
      (b as HTMLButtonElement).click();
    }, label);
  await item('Renombrar la sesión…');
  await s.browser.$('.acp-rename input').setValue('Mi sesión');
  await s.browser.keys('Enter');
  await s.browser.waitUntil(async () => (await text('.acp-row-title'))[0]?.includes('Mi sesión') === true, { timeout: 5_000, timeoutMsg: 'no cambió el nombre' });

  // Se cierra y se reabre del historial: nombre y pregunta aparte siguen ahí
  const saved = () => {
    const dir = join(s.vault, '.synapse/agent-sessions');
    return readdirSync(dir)
      .filter((f) => f.endsWith('.json'))
      .map((f) => JSON.parse(readFileSync(join(dir, f), 'utf8')))
      .find((t) => t.native_title === 'Mi sesión');
  };
  await s.browser.waitUntil(async () => saved()?.asides?.length === 1, { timeout: 5_000, timeoutMsg: 'no se guardó' });
  const transcript = saved();
  expect(transcript.title).toBe('Mi sesión');
  expect(transcript.asides[0].items.map((i: { type: string }) => i.type)).toContain('user');
  // Lo heredado no se guarda como mensajes (solo, aparte, el texto que se le envió), y la pregunta no
  // entra en los de la conversación
  const own = transcript.asides[0].items.filter((i: { type: string }) => i.type !== 'context');
  expect(JSON.stringify(own)).not.toContain('razonamiento y herramienta');
  expect(transcript.asides[0].items[0].type).toBe('context');
  expect(JSON.stringify(transcript.items)).not.toContain('¿guardada?');
  // La pregunta aparte no tiene archivo propio
  expect(existsSync(join(s.vault, '.synapse/agent-sessions', `${transcript.asides[0].id}.json`))).toBe(false);

  await s.browser.$('.acp-row button[aria-label="Cerrar conversación"]').click();
  await s.browser.waitUntil(async () => (await count('.acp-row')) === 0, { timeout: 5_000 });
  await s.browser.waitUntil(async () => (await text('select[aria-label="Historial de conversaciones"] option')).some((t) => t?.startsWith('Mi sesión')), {
    timeout: 5_000,
    timeoutMsg: 'el historial no muestra el nombre',
  });
  await s.browser.execute((value) => {
    const select = document.querySelector('select[aria-label="Historial de conversaciones"]') as HTMLSelectElement;
    select.value = value;
    select.dispatchEvent(new Event('change', { bubbles: true }));
  }, transcript.id);
  await s.browser.$('.acp-aside-card').waitForExist({ timeout: 15_000 });
  expect((await text('.acp-row-title'))[0]).toContain('Mi sesión');
  await s.browser.$('.acp-aside-card').click();
  expect(await text('.acp-side .acp-msg.user.markdown')).toEqual(['¿guardada?']);
  // El agente falso no retoma sesiones: la repregunta va en una nueva, con el contexto reenviado
  await s.browser.$('.acp-side .acp-input textarea').setValue('¿sigues?');
  await s.browser.keys('Enter');
  await s.browser.waitUntil(async () => (await text('.acp-side .acp-msg.agent')).length === 2, { timeout: 20_000, timeoutMsg: 'no respondió al reabrir' });

  // También al volver a cargar la ventana, como al abrir de nuevo Synapse
  await s.browser.waitUntil(async () => saved()?.asides?.[0]?.items.filter((i: { type: string }) => i.type === 'agent').length === 2, { timeout: 5_000, timeoutMsg: 'no se guardó la repregunta' });
  // El agente falso no retoma sesiones: aun en una nueva, el nombre puesto se conserva
  expect(saved().native_title).toBe('Mi sesión');
  await s.browser.refresh();
  await s.browser.$('.acp-aside-card').waitForExist({ timeout: 30_000 });
  await s.browser.$('.acp-aside-card').click();
  await s.browser.waitUntil(async () => (await text('.acp-side .acp-msg.user.markdown')).map((t) => t?.split('\n')[0].trim()).join() === '¿guardada?,¿sigues?', { timeout: 5_000, timeoutMsg: 'no siguen tras recargar' });
}, 120_000);

it('con un agente que bifurca su sesión, la pregunta aparte muestra el historial heredado y la cierra al dejar de verse', async () => {
  await s.browser.$('.acp-side-minimize').click();
  await s.browser.$('.synapse-agent-panel button[title="Ajustes de agentes"]').click();
  await s.browser.$('.agent-form').waitForExist({ timeout: 5_000 });
  await s.browser.$('input[placeholder="Identificador (mi-agente)"]').setValue('bifurca');
  await s.browser.$('input[placeholder="Nombre"]').setValue('Agente que bifurca');
  await s.browser.$('input[placeholder="Comando (ruta o nombre en el PATH)"]').setValue(process.execPath);
  await s.browser.$('input[placeholder="Argumentos separados por espacios"]').setValue(`${FAKE_AGENT} --fork --resume --config`);
  await s.browser.$('.agent-form button[type="submit"]').click();
  await s.browser.waitUntil(async () => (await text('.agent-row .row-label')).some((t) => t?.startsWith('Agente que bifurca')), { timeout: 5_000 });
  await s.browser.keys('Escape');
  // Sin fijar, ese Esc puede haber cerrado el panel
  if (await count('.synapse-agent-panel.is-hidden')) await s.browser.$('.editor-header .agent-toggle').click();
  await s.browser.$('.synapse-agent-panel:not(.is-hidden)').waitForExist({ timeout: 5_000 });
  await s.browser.execute(() => {
    (async () => {
      const { agentStore } = await new Function("return import('/src/entities/acp-agent/index.ts')")();
      for (const c of [...agentStore.conversations]) await agentStore.close(c.id);
      await agentStore.refresh();
      await agentStore.newConversation(agentStore.cwd, 'bifurca');
    })();
  });
  const input = s.browser.$('.acp-input textarea');
  await s.browser.waitUntil(async () => (await count('.acp-row')) === 1 && (await input.isExisting()) && !(await input.getAttribute('disabled')), { timeout: 15_000 });
  await input.setValue('razonamiento y herramienta');
  await s.browser.keys('Enter');
  await s.browser.waitUntil(async () => (await text('.acp-msg.agent')).some((t) => t?.includes('Listo.')), { timeout: 10_000 });
  await input.setValue('/btw ¿y eso?');
  await s.browser.keys('Enter');
  await s.browser.waitUntil(async () => (await text('.acp-side .acp-msg.agent')).some((t) => t?.startsWith('Respuesta aparte')), { timeout: 20_000 });
  const asked = await text('.acp-side .acp-msg.user.markdown');
  expect(asked).toEqual([expect.stringMatching(/^razonamiento y herramienta/), '¿y eso?']);
  expect(await count('.acp-side .acp-fork-marker')).toBe(1);
  // Bifurcada, lo enviado es solo la indicación y la pregunta, sin el historial
  const sent = async () =>
    (await s.browser.execute(() => [...document.querySelectorAll('.acp-side .acp-context pre')].map((p) => p.textContent ?? ''))) as string[];
  expect(await sent()).toEqual([expect.stringMatching(/^\[Pregunta aparte[\s\S]*¿y eso\?$/)]);
  expect((await sent())[0]).not.toContain('Usuario: ');

  const answers = async () => (await text('.acp-side .acp-msg.agent')).filter((t) => t?.startsWith('Respuesta aparte'));
  const session = (await answers())[0]!.match(/\[(ses-e2e-\d+)\]/)![1];
  const side = s.browser.$('.acp-side .acp-input textarea');
  const reopen = async () => {
    await s.browser.$('.acp-aside-card').click();
    await side.waitForExist({ timeout: 5_000 });
  };
  // Minimizada, cierra su sesión; la repregunta la retoma, sin reenviar el contexto
  await s.browser.$('.acp-side-minimize').click();
  await reopen();
  await side.setValue('despacio, ¿y ahora?');
  await s.browser.keys('Enter');
  await s.browser.waitUntil(async () => (await count('.acp-side .acp-send[title^="Detener"]')) === 1, { timeout: 10_000, timeoutMsg: 'no empezó a responder' });
  // Recogida mientras contesta: termina de responder y entonces se cierra
  await s.browser.$('.acp-side-minimize').click();
  await s.browser.$('.acp-aside-card.unread').waitForExist({ timeout: 15_000, timeoutMsg: 'no terminó la respuesta' });
  await reopen();
  expect((await answers())[1]).toContain(`[${session}] retomada×1`);
  await side.setValue('¿otra vez?');
  await s.browser.keys('Enter');
  await s.browser.waitUntil(async () => (await answers()).length === 3, { timeout: 20_000, timeoutMsg: 'no respondió tras cerrarse' });
  expect((await answers())[2]).toContain(`[${session}] retomada×2`);
  // Las repreguntas no añaden otro bloque
  expect(await count('.acp-side .acp-context')).toBe(1);

  // El modelo y el esfuerzo elegidos en ella son solo suyos: no cambian los recordados para el agente
  const chip = async () => (await text('.acp-side .acp-model-chip'))[0] ?? '';
  const choose = async (label: string) => {
    await s.browser.$('.acp-side .acp-opts-btn').click();
    await s.browser.$(`.acp-side .acp-opts-menu`).$(`button=${label}`).click();
  };
  const remembered = () =>
    s.browser.execute(() => [localStorage.getItem('synapse.agent.model.bifurca'), localStorage.getItem('synapse.agent.effort.bifurca')]);
  const before = await remembered();
  await choose('high');
  await s.browser.waitUntil(async () => (await chip()).includes('high'), { timeout: 5_000, timeoutMsg: 'no cambió el esfuerzo' });
  expect(await remembered()).toEqual(before);
  const mainChip = async () =>
    (await s.browser.execute(() => document.querySelector('.acp-model-chip')?.textContent ?? '')) as string;
  expect(await mainChip()).not.toContain('high');
  // Los conserva al cerrar y reabrir la conversación
  await s.browser.execute(() => {
    (async () => {
      const { agentStore } = await new Function("return import('/src/entities/acp-agent/index.ts')")();
      const [c] = agentStore.conversations;
      const { cwd, id } = c;
      await agentStore.close(id);
      await agentStore.openFromHistory(cwd, id);
    })();
  });
  await s.browser.$('.acp-aside-card').waitForExist({ timeout: 15_000 });
  await reopen();
  await side.setValue('¿con qué modelo?');
  await s.browser.keys('Enter');
  await s.browser.waitUntil(async () => (await answers()).length === 4, { timeout: 20_000, timeoutMsg: 'no respondió al reabrir la conversación' });
  expect((await answers())[3]).toContain('modelo:uno/high');
  await s.browser.waitUntil(async () => (await chip()).includes('high'), { timeout: 5_000 });
  // El modelo, igual
  await choose('Modelo dos');
  await s.browser.waitUntil(async () => (await chip()).includes('Modelo dos'), { timeout: 5_000, timeoutMsg: 'no cambió el modelo' });
  expect(await remembered()).toEqual(before);
  await s.browser.$('.acp-side-minimize').click();
  await reopen();
  await side.setValue('¿y ahora?');
  await s.browser.keys('Enter');
  await s.browser.waitUntil(async () => (await answers()).length === 5, { timeout: 20_000, timeoutMsg: 'no respondió tras minimizar' });
  expect((await answers())[4]).toContain('modelo:dos/');
}, 120_000);
