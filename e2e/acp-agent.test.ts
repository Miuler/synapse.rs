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

it('respuesta con tareas: casillas, copiar Markdown / con formato, copiar selección y zoom', async () => {
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

  // «Con formato» y la selección copian ☐ / ✅. Se captura lo que se escribe en el portapapeles
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
  const rich = s.browser.$('.acp-agent-wrap[data-test="tareas"] .acp-copy-rich');
  await rich.scrollIntoView();
  await rich.moveTo();
  await rich.click();
  await s.browser.waitUntil(async () => !!(await s.browser.execute(() => (window as any).__copied['text/plain'])), {
    timeout: 5_000,
    timeoutMsg: '«Con formato» no escribió en el portapapeles',
  });
  const captured = (await s.browser.execute(() => {
    const w = window as any;
    const out = [{ html: w.__copied['text/html'] ?? '', text: w.__copied['text/plain'] ?? '' }];
    const wrap = document.querySelector('.acp-agent-wrap[data-test="tareas"]')!;
    const ul = wrap.querySelector('.acp-msg.agent ul')!;
    const range = document.createRange();
    range.selectNodeContents(ul);
    const sel = window.getSelection()!;
    sel.removeAllRanges();
    sel.addRange(range);
    const data = new DataTransfer();
    ul.dispatchEvent(new ClipboardEvent('copy', { clipboardData: data, bubbles: true, cancelable: true }));
    out.push({ html: data.getData('text/html'), text: data.getData('text/plain') });
    return out;
  })) as { html: string; text: string }[];
  expect(captured[0].text).toBe('Avance\n- ☐ 00. Modelo de Dato\n- ✅ 02. Diagrama de Software');
  expect(captured[0].html).toContain('<h2>Avance</h2>');
  expect(captured[0].html).toContain('<strong>02.</strong>');
  expect(captured[0].html).not.toContain('<input');
  expect(captured[1].text).toBe('- ☐ 00. Modelo de Dato\n- ✅ 02. Diagrama de Software');

  // Zoom del chat: Ctrl++ con el foco en los mensajes, Ctrl+0 vuelve
  const zoom = () => s.browser.execute(() => (document.querySelector('.acp-messages') as HTMLElement).style.zoom) as Promise<string>;
  await s.browser.execute(() => (document.querySelector('.acp-messages') as HTMLElement).focus());
  await s.browser.keys(['Control', '=']);
  await s.browser.waitUntil(async () => (await zoom()) === '1.1', { timeout: 5_000, timeoutMsg: 'Ctrl+= no amplió el chat' });
  await s.browser.keys(['Control', '0']);
  await s.browser.waitUntil(async () => (await zoom()) === '1', { timeout: 5_000, timeoutMsg: 'Ctrl+0 no volvió al 100 %' });

  // La caja de texto crece con el contenido, con tope
  const h1 = (await s.browser.execute(() => (document.querySelector('.acp-input textarea') as HTMLElement).offsetHeight)) as number;
  await input.setValue(Array.from({ length: 40 }, (_, i) => `línea ${i}`).join('\n'));
  const grown = (await s.browser.execute(() => {
    const t = document.querySelector('.acp-input textarea') as HTMLElement;
    return { h: t.offsetHeight, max: window.innerHeight * 0.4 };
  })) as { h: number; max: number };
  expect(grown.h).toBeGreaterThan(h1 * 2);
  expect(grown.h).toBeLessThanOrEqual(grown.max + 1);
  await input.clearValue();
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
  await say('trabajo lento');
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
