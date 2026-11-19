// Genera las capturas de docs/ (arquitectura como código y agentes). No es una prueba de regresión:
// solo corre con E2E_SHOTS=/ruta (p. ej. E2E_SHOTS=docs bun test e2e/docs-shots.test.ts).
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { join } from 'node:path';
import { startApp, paneReady, type Session } from './harness';

const SHOTS = process.env.E2E_SHOTS;
const FAKE = join(import.meta.dir, 'fixtures/docs-acp-agent.ts');
let s: Session;

const D2 = `direction: down
vars: { d2-config: { layout-engine: elk } }

usuario: Cliente { shape: person }
correo: Servidor de correo { style.stroke-dash: 4 }
sistema: Tienda online {
  web: Web (Svelte)
  api: API de pedidos
  broker: Broker de eventos { shape: queue }
  db: PostgreSQL { shape: cylinder }
  web -> api: HTTPS
  api -> db: SQL
  api -> broker: publica eventos
  notificaciones: Notificaciones
  broker -> notificaciones: consume
}
usuario -> sistema.web
sistema.notificaciones -> correo: envía avisos
sistema.api -> pagos: cobra { style.stroke-dash: 4 }
pagos: Pasarela de pagos { style.stroke-dash: 4 }
`;

const MERMAID = `sequenceDiagram
  autonumber
  actor C as Cliente
  participant W as Web
  participant A as API de pedidos
  participant B as Broker
  participant N as Notificaciones
  C->>W: Confirma el pedido
  W->>A: POST /pedidos
  A->>B: PedidoCreado
  A-->>W: 201 Created
  B->>N: PedidoCreado
  N-->>C: Correo de confirmación
`;

const ADR = `---
tags: [arquitectura/adr]
estado: aceptado
---
# ADR-0002 · Comunicación por eventos

## Contexto
La API de pedidos no debe conocer a quien reacciona a un pedido. Ver el diagrama de contenedores: [[sistema.d2]].

## Decisión
Publicamos **eventos de dominio** en un broker. Cada consumidor decide qué hacer.

## Consecuencias
- Desacopla la API de notificaciones, facturación y analítica.
- Consistencia eventual: el correo puede llegar unos segundos después.

## Decisión anterior (transclusión)
![[adr-0001-monolito-modular#Decisión]]

Relacionado: [[adr-0001-monolito-modular#Decisión]] · diagrama: [[secuencia-pedido.mmd]]
`;

const ADR1 = `---
tags: [arquitectura/adr]
estado: reemplazado
---
# ADR-0001 · Monolito modular

## Decisión
Empezar con un monolito modular y separar cuando duela.
`;

beforeAll(async () => {
  s = await startApp({
    'sistema.d2': D2,
    'secuencia-pedido.mmd': MERMAID,
    'adr/adr-0002-eventos.md': ADR,
    'adr/adr-0001-monolito-modular.md': ADR1,
    'README.md': '# Arquitectura de la tienda\n\nTodo es código: diagramas D2/Mermaid y decisiones en ADR.\n',
  });
  await s.browser.setWindowSize(1500, 940);
  await s.browser.execute(() => localStorage.setItem('synapse_settings', JSON.stringify({ vimMode: false })));
  await s.browser.refresh();
  await s.browser.$('.file-tree-item[data-path="sistema.d2"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(() => s?.close());

const shot = (name: string) => s.browser.saveScreenshot(`${SHOTS}/${name}.png`);
const open = async (path: string) => {
  if (path.startsWith('adr/') && !(await s.browser.$(`.file-tree-item[data-path="${path}"]`).isExisting()))
    await s.browser.$('.file-tree-item.folder[data-path="adr"]').click();
  await s.browser.$(`.file-tree-item[data-path="${path}"]`).click();
  await paneReady(s);
};

it('captura: arquitectura como código', async () => {
  if (!SHOTS) return;
  await open('sistema.d2');
  await s.browser.$('.cm-editor .cm-content').waitForExist({ timeout: 10_000 }).catch(async () => s.browser.keys(['F2']));
  await s.browser.waitUntil(() => s.browser.execute(() => !!document.querySelector('.preview-pane svg')) as Promise<boolean>, { timeout: 60_000 });
  await s.browser.pause(800);
  await shot('docs_d2_editor_y_diagrama');

  await open('secuencia-pedido.mmd');
  await s.browser.waitUntil(() => s.browser.execute(() => !!document.querySelector('.tab-pane:not(.hidden) .preview-pane svg, .tab-pane:not(.hidden) svg')) as Promise<boolean>, { timeout: 30_000 });
  await s.browser.pause(800);
  await shot('docs_mermaid_secuencia');

  await open('adr/adr-0002-eventos.md');
  await s.browser.pause(1500);
  await shot('docs_adr_enlaces_y_transclusion');
  expect(true).toBe(true);
}, 180_000);

it('captura: agente sobre el diagrama', async () => {
  if (!SHOTS) return;
  await open('sistema.d2');
  await s.browser.$('.editor-header .agent-toggle').click();
  await s.browser.$('.synapse-agent-panel').waitForExist({ timeout: 5_000 });
  await s.browser.$('.synapse-agent-panel button[title="Ajustes de agentes"]').click();
  await s.browser.$('.agent-form').waitForExist({ timeout: 5_000 });
  await s.browser.$('input[placeholder="Identificador (mi-agente)"]').setValue('claude-demo');
  await s.browser.$('input[placeholder="Nombre"]').setValue('Claude');
  await s.browser.$('input[placeholder="Comando (ruta o nombre en el PATH)"]').setValue(process.execPath);
  await s.browser.$('input[placeholder="Argumentos separados por espacios"]').setValue(FAKE);
  await s.browser.$('.agent-form button[type="submit"]').click();
  await s.browser.pause(500);
  await s.browser.keys('Escape');
  await s.browser.execute(() => {
    const sel = document.querySelector('.acp-select') as HTMLSelectElement;
    sel.value = 'claude-demo';
    sel.dispatchEvent(new Event('change', { bubbles: true }));
  });
  const input = s.browser.$('.acp-input textarea');
  await s.browser.waitUntil(async () => !(await input.getAttribute('disabled')), { timeout: 15_000 });
  await input.setValue('Añade un servicio de notificaciones que consuma los eventos del broker y envíe correos. Documenta la decisión.');
  await s.browser.keys('Enter');
  await s.browser.waitUntil(
    () => s.browser.execute(() => (document.querySelector('.acp-msg.agent')?.textContent ?? '').includes('render_diagram')) as Promise<boolean>,
    { timeout: 20_000 },
  );
  await s.browser.pause(800);
  await shot('docs_agente_conversacion');
}, 180_000);
