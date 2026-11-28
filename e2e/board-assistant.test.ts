// El asistente de configuración: con el origen del tablero abierto se conversa con el agente, que
// prueba consultas y cambia el borrador; guardar o descartar es cosa del usuario. Contra un Azure
// DevOps de mentira: nada sale de la máquina.
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

const GOOD = "SELECT [System.Id] FROM WorkItems WHERE [System.WorkItemType] = 'Bug'";

const server = Bun.serve({
  port: 0,
  hostname: '127.0.0.1',
  async fetch(request) {
    const path = new URL(request.url).pathname;
    if (path.endsWith('/states')) return Response.json({ value: [{ name: 'New', category: 'Proposed' }] });
    if (path.endsWith('/_apis/wit/wiql')) {
      const { query } = (await request.json()) as { query: string };
      if (query === 'MAL') return Response.json({ message: 'TF51005: la consulta no es válida' }, { status: 400 });
      return Response.json({ workItems: [{ id: 1 }, { id: 2 }] });
    }
    if (path.endsWith('/_apis/wit/workitemsbatch')) {
      const { ids } = (await request.json()) as { ids: number[] };
      return Response.json({
        value: ids.map((id) => ({ id, rev: 1, fields: { 'System.Title': `Elemento ${id}`, 'System.State': 'New', 'System.WorkItemType': 'Bug' } })),
      });
    }
    return Response.json({ message: `ruta inesperada ${path}` }, { status: 500 });
  },
});

let s: Session;
const keyring = mkdtempSync(join(tmpdir(), 'synapse-llavero-'));
const saved = process.env.SYNAPSE_KEYRING_FILE;
const scope = () => `${server.url.origin}/org/Demo`;

beforeAll(async () => {
  process.env.SYNAPSE_KEYRING_FILE = keyring;
  writeFileSync(join(keyring, 'issues-pruebas'), 'token-falso');
  s = await startApp({ 'nota.md': '# Nota\n' });
  const config = join(s.home, '.config/synapsers');
  mkdirSync(config, { recursive: true });
  const connection = { id: 'pruebas', provider: 'azure_devops', name: 'Pruebas', server: `${server.url.origin}/org` };
  writeFileSync(join(config, 'config.jsonc'), JSON.stringify({ issues: { connections: [connection] } }));
  const agent = {
    id: 'asistente',
    name: 'Asistente falso',
    source: 'custom',
    installed: true,
    launch: { command: process.execPath, args: [join(import.meta.dir, 'fixtures/fake-config-agent.ts')] },
  };
  writeFileSync(join(config, 'agents.json'), JSON.stringify([agent]));
  await s.browser.$('.file-tree-item[data-path="nota.md"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  if (saved === undefined) delete process.env.SYNAPSE_KEYRING_FILE;
  else process.env.SYNAPSE_KEYRING_FILE = saved;
  server.stop(true);
  await s?.close();
  rmSync(keyring, { recursive: true, force: true });
});

const board = () => readFileSync(join(s.vault, 'Tablero.board.md'), 'utf8');
const until = (check: () => boolean | Promise<boolean>, what: string) =>
  s.browser.waitUntil(async () => check(), { timeout: 20_000, timeoutMsg: what });
const panel = () => s.browser.$('.source-panel');
const value = (selector: string) =>
  s.browser.execute((q) => (document.querySelector(q) as HTMLInputElement | null)?.value ?? null, selector) as Promise<string | null>;
const marks = () => s.browser.execute(() => document.querySelectorAll('.source-panel .agent-mark').length) as Promise<number>;
const answers = () =>
  s.browser.execute(() => [...document.querySelectorAll('.acp-msg.agent')].map((e) => e.textContent ?? '')) as Promise<string[]>;
/** Con eventos, no con `setValue`: tras el clic derecho el controlador escribe sin mayúsculas. */
const fill = (selector: string, text: string) =>
  s.browser.execute(
    (selector, text) => {
      const input = document.querySelector(selector) as HTMLInputElement;
      input.value = text;
      input.dispatchEvent(new Event('input', { bubbles: true }));
    },
    selector,
    text,
  );
const shot = async (name: string) => {
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/${name}.png`);
};

/** Pide ayuda al agente y devuelve su respuesta. */
async function ask() {
  const before = (await answers()).length;
  await fill('.acp-input textarea', 'ayuda');
  await s.browser.$('.acp-input textarea').click();
  await s.browser.keys(['Enter']);
  await until(async () => {
    const all = await answers();
    return all.length > before && /cambio:|sin configuración|fallo:/.test(all.at(-1)!);
  }, 'el agente no respondió');
  return (await answers()).at(-1)!;
}

it('con la configuración abierta, el agente prueba, corrige y deja el borrador sin guardar', async () => {
  await s.browser.$('.file-tree-item[data-path="nota.md"]').click({ button: 'right' });
  await s.browser.$('.context-menu-item').waitForExist({ timeout: 5_000 });
  await s.browser.execute(() =>
    ([...document.querySelectorAll('.context-menu-item')].find((i) => i.textContent?.includes('Nuevo tablero')) as HTMLElement).click(),
  );
  await panel().waitForExist({ timeout: 15_000 });
  expect(await panel().getText()).toContain('el agente del chat puede ver la conexión');
  await fill('.source-panel input[name="scope"]', scope());

  // El chat se usa sin cerrar la configuración
  await s.browser.$('.editor-header .agent-toggle').click();
  await s.browser.$('.synapse-agent-panel').waitForExist({ timeout: 10_000 });
  await until(
    () => s.browser.execute(() => [...document.querySelectorAll('.acp-select option')].some((o) => o.textContent?.trim() === 'Asistente falso')),
    'no aparece el agente',
  );
  await s.browser.execute(() => {
    const select = document.querySelector('.acp-select') as HTMLSelectElement;
    select.value = 'asistente';
    select.dispatchEvent(new Event('change', { bubbles: true }));
  });
  await until(async () => !(await s.browser.$('.acp-input textarea').getAttribute('disabled')), 'no conectó');
  const before = board();

  const answer = await ask();
  expect(answer).toContain('conexión:Pruebas proyecto:Demo');
  expect(answer).toContain('mala:rechazada');
  expect(answer).toContain('buena:2');
  expect(answer).toContain('cambio:hecho');
  await until(async () => (await value('.source-panel textarea[name="query"]')) === GOOD, 'no se ve la consulta');
  expect(await value('.source-panel input[data-column="done"]')).toBe('Closed, Done');
  // Marcados: el modo, la consulta y los estados de la columna
  expect(await marks()).toBe(3);
  expect(board()).toBe(before);
  // El panel queda dentro del tablero: no tapa el explorador ni el chat
  const inside = await s.browser.execute(() => {
    const box = (selector: string) => document.querySelector(selector)!.getBoundingClientRect();
    const [panel, editor, chat] = ['.source-panel', '.tab-pane:not(.hidden) .board-editor', '.acp-input textarea'].map(box);
    return panel.left >= editor.left && panel.top >= editor.top && panel.right <= editor.right && panel.right <= chat.left;
  });
  expect(inside).toBe(true);
  await shot('asistente-cambios');

  // Escape en el chat no cierra la configuración
  await s.browser.$('.acp-input textarea').click();
  await s.browser.keys(['Escape']);
  await s.browser.pause(300);
  expect(await panel().isExisting()).toBe(true);

  // Lo que toca el usuario deja de estar marcado
  await fill('.source-panel input[data-column="done"]', 'Closed');
  await until(async () => (await marks()) === 2, 'sigue marcado lo editado');

  await s.browser.$('.source-panel button.cancel').click();
  await panel().waitForExist({ reverse: true, timeout: 10_000 });
  expect(board()).toBe(before);
  expect(await ask()).toContain('No hay ninguna configuración abierta');
}, 180_000);

it('al guardar el usuario, queda lo que propuso el agente', async () => {
  await s.browser.$('.tab-pane:not(.hidden) .issues-settings').click();
  await panel().waitForExist({ timeout: 15_000 });
  await fill('.source-panel input[name="scope"]', scope());
  // Con la configuración abierta no se sincroniza
  expect(await s.browser.$('.tab-pane:not(.hidden) .issues-pull').isEnabled()).toBe(false);
  expect(await ask()).toContain('cambio:hecho');
  await until(async () => (await marks()) === 3, 'no se marcaron los cambios');
  await s.browser.$('.source-panel button[type="submit"]').click();
  await panel().waitForExist({ reverse: true, timeout: 15_000 });
  await until(() => board().includes("[System.WorkItemType] = 'Bug'"), 'no se guardó la consulta');
  expect(board()).toMatch(/states:\s+done:\s+- Closed\s+- Done/);
  expect(board()).not.toContain('token-falso');
}, 120_000);
