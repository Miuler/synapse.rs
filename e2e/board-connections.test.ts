// Dos conexiones de Azure DevOps creadas en Configuración: cada tablero usa la de su organización y su
// token. Contra un servidor de mentira y un llavero de mentira: nada sale de la máquina.
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

const TOKENS: Record<string, string> = { org: 'token-uno', otra: 'token-dos' };
const PEOPLE: Record<string, string> = { org: 'Persona Uno', otra: 'Persona Dos' };
/** Qué token llegó a cada organización. */
const received: { org: string; token: string }[] = [];

const server = Bun.serve({
  port: 0,
  hostname: '127.0.0.1',
  async fetch(request) {
    const path = new URL(request.url).pathname;
    const org = path.split('/')[1];
    const token = atob(request.headers.get('authorization')?.replace('Basic ', '') ?? '').slice(1);
    received.push({ org, token });
    if (token !== TOKENS[org]) return new Response('', { status: 401 });
    if (path.endsWith('/_apis/connectionData')) return Response.json({ authenticatedUser: { providerDisplayName: PEOPLE[org] } });
    if (path.endsWith('/_apis/wit/wiql')) return Response.json({ workItems: [{ id: org === 'org' ? 1 : 2 }] });
    if (path.endsWith('/_apis/wit/workitemsbatch')) {
      const { ids } = (await request.json()) as { ids: number[] };
      return Response.json({
        value: ids.map((id) => ({ id, rev: 1, fields: { 'System.Title': `Tarea de ${org}`, 'System.State': 'To Do', 'System.WorkItemType': 'Task' } })),
      });
    }
    return Response.json({ message: `ruta inesperada ${path}` }, { status: 500 });
  },
});

const board = (org: string) => `---
type: board
default_view: table
columns:
  - { id: todo, label: "Por hacer", color: "var(--text-muted)" }
azure_devops:
  project_url: ${server.url.origin}/${org}/Demo
---

| id | Tarea | Estado | Inicio | Duración | Depende | Progreso |
|---|---|---|---|---|---|---|
`;

let s: Session;
const keyring = mkdtempSync(join(tmpdir(), 'synapse-llavero-'));
const saved = ['AZURE_DEVOPS_PAT', 'ADO_PAT', 'SYNAPSE_KEYRING_FILE'].map((name) => [name, process.env[name]] as const);

beforeAll(async () => {
  delete process.env.AZURE_DEVOPS_PAT;
  delete process.env.ADO_PAT;
  process.env.SYNAPSE_KEYRING_FILE = keyring;
  s = await startApp({ 'uno.board.md': board('org'), 'dos.board.md': board('otra') });
  await s.browser.$('.file-tree-item[data-path="uno.board.md"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  for (const [name, value] of saved) {
    if (value === undefined) delete process.env[name];
    else process.env[name] = value;
  }
  server.stop(true);
  await s?.close();
  rmSync(keyring, { recursive: true, force: true });
});

const config = () => readFileSync(join(s.home, '.config/synapsers/config.jsonc'), 'utf8');
const stored = (id: string) => join(keyring, `issues-${id}`);
const until = (check: () => boolean | Promise<boolean>, what: string) =>
  s.browser.waitUntil(async () => check(), { timeout: 15_000, timeoutMsg: what });
const text = (selector: string) =>
  s.browser.execute((selector) => document.querySelector(selector)?.textContent ?? '', selector) as Promise<string>;
const shot = async (name: string) => {
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/${name}.png`);
};

async function openSettings() {
  await s.browser.$('[aria-label="Configuración"]').click();
  await s.browser.$('.settings-nav-item').waitForExist({ timeout: 15_000 });
  await s.browser.execute(() =>
    ([...document.querySelectorAll('.settings-nav-item')].find((b) => b.textContent?.trim() === 'Conexiones') as HTMLElement).click(),
  );
  await s.browser.$('.connections').waitForExist({ timeout: 15_000 });
}

async function add(name: string, org: string, token: string) {
  await s.browser.$('.connections .add').click();
  await s.browser.$('.connections input[name="name"]').setValue(name);
  await s.browser.$('.connections input[name="server"]').setValue(`${server.url.origin}/${org}`);
  await s.browser.$('.connections input[name="credential"]').setValue(token);
  await s.browser.$('.connections form button[type="submit"]').click();
  await s.browser.$(`.connection[data-connection="${name.toLowerCase()}"]`).waitForExist({ timeout: 15_000 });
}

const row = (id: string) => `.connection[data-connection="${id}"]`;

it('se crean dos conexiones del mismo proveedor sin que su token quede en la configuración', async () => {
  await openSettings();
  await add('Uno', 'org', TOKENS.org);
  await add('Dos', 'otra', TOKENS.otra);
  expect(readFileSync(stored('uno'), 'utf8')).toBe(TOKENS.org);
  expect(readFileSync(stored('dos'), 'utf8')).toBe(TOKENS.otra);
  expect(config()).toContain(`${server.url.origin}/otra`);
  expect(config()).not.toContain('token-');
  expect(await text('.connections')).not.toContain('token-');
  // En desarrollo se anotan las llamadas a Rust: la que guarda, sin sus datos
  expect(await s.browser.execute(() => JSON.stringify((window as any).__synapseIpcLog))).not.toContain('token-');
}, 120_000);

it('«Probar» dice con qué cuenta entra cada una y explica el rechazo sin mostrar el token', async () => {
  await s.browser.$(`${row('uno')} .test`).click();
  await until(async () => (await text(`${row('uno')} .result`)).includes('Conectado como Persona Uno'), 'no se probó');
  await s.browser.$(`${row('dos')} .test`).click();
  await until(async () => (await text(`${row('dos')} .result`)).includes('Conectado como Persona Dos'), 'no se probó');
  await shot('conexiones');

  await s.browser.$(`${row('dos')} .edit`).click();
  await s.browser.$('.connections input[name="credential"]').setValue('token-malo');
  await shot('conexiones-editar');
  await s.browser.$('.connections form button[type="submit"]').click();
  await until(() => readFileSync(stored('dos'), 'utf8') === 'token-malo', 'no se cambió el token');
  await s.browser.$(`${row('dos')} .test`).click();
  await until(async () => (await text(`${row('dos')} .result`)).includes('rechazó el token'), 'no se explicó el rechazo');
  expect(await text('.connections')).not.toContain('token-malo');

  // Cambiar solo el nombre conserva el token; se vuelve a poner el bueno
  await s.browser.$(`${row('dos')} .edit`).click();
  await s.browser.$('.connections input[name="credential"]').setValue(TOKENS.otra);
  await s.browser.$('.connections form button[type="submit"]').click();
  await until(() => readFileSync(stored('dos'), 'utf8') === TOKENS.otra, 'no se repuso el token');
  await s.browser.keys(['Escape']);
  await s.browser.$('.synapse-settings').waitForExist({ reverse: true, timeout: 10_000 });
}, 120_000);

async function pull(file: string, org: string) {
  received.length = 0;
  await s.browser.$(`.file-tree-item[data-path="${file}"]`).click();
  const pane = '.tab-pane:not(.hidden)';
  await s.browser.$(`${pane} .issues-pull`).waitForExist({ timeout: 15_000 });
  await s.browser.$(`${pane} .issues-pull`).click();
  await until(() => readFileSync(join(s.vault, file), 'utf8').includes(`Tarea de ${org}`), `no se trajo ${file}`);
  expect(new Set(received.map((r) => `${r.org} ${r.token}`))).toEqual(new Set([`${org} ${TOKENS[org]}`]));
}

it('cada tablero usa la conexión de su organización', async () => {
  await pull('uno.board.md', 'org');
  await pull('dos.board.md', 'otra');
}, 120_000);

it('eliminar una conexión pide confirmación y quita su token; su tablero deja de sincronizar', async () => {
  await openSettings();
  await s.browser.$(`${row('dos')} .remove`).click();
  await s.browser.$('.confirm-dialog').waitForExist({ timeout: 10_000 });
  await shot('conexiones-eliminar');
  expect(existsSync(stored('dos'))).toBe(true);
  await s.browser.$('.confirm-dialog-btn.confirm').click();
  await s.browser.$(row('dos')).waitForExist({ reverse: true, timeout: 10_000 });
  expect(existsSync(stored('dos'))).toBe(false);
  expect(existsSync(stored('uno'))).toBe(true);
  // El diálogo de ajustes sigue abierto tras confirmar
  expect(await s.browser.$('.synapse-settings').isExisting()).toBe(true);
  await s.browser.keys(['Escape']);
  await s.browser.$('.synapse-settings').waitForExist({ reverse: true, timeout: 10_000 });

  received.length = 0;
  await s.browser.$('.tab-pane:not(.hidden) .issues-pull').click();
  await until(async () => (await text('.tab-pane:not(.hidden) .issues-status')).includes('Ninguna conexión cubre'), 'siguió sincronizando');
  expect(received).toEqual([]);
}, 120_000);

it('sin conexiones, el token que ya se tenía se convierte en la primera conexión sin borrarlo', async () => {
  // Sin ninguna conexión y con la credencial única de antes (una ruta de 1Password)
  writeFileSync(join(s.home, '.config/synapsers/config.jsonc'), '{ "azure_devops": { "token": "op://Pruebas/Azure/token" } }');
  await openSettings();
  await s.browser.$('.connections .from-global').waitForExist({ timeout: 10_000 });
  await shot('conexiones-sin-ninguna');
  await s.browser.$('.connections .from-global').click();
  const url = s.browser.$('.connections input[name="server"]');
  expect(await url.getValue()).toBe('https://dev.azure.com/');
  expect(await s.browser.$('.connections input[name="use_global"]').isSelected()).toBe(true);
  expect(await s.browser.$('.connections input[name="credential"]').isExisting()).toBe(false);
  await url.setValue('https://dev.azure.com/organizacion');
  await s.browser.$('.connections form button[type="submit"]').click();
  await s.browser.$(row('azure-devops')).waitForExist({ timeout: 10_000 });
  expect(await text(`${row('azure-devops')} .credential`)).toContain('op://Pruebas/Azure/token');
  const saved = JSON.parse(config().replace(/^\s*\/\/.*$/gm, ''));
  expect(saved.issues.connections[0]).toMatchObject({ server: 'https://dev.azure.com/organizacion', secret: 'op://Pruebas/Azure/token' });
  // La credencial anterior sigue para la versión instalada
  expect(saved.azure_devops.token).toBe('op://Pruebas/Azure/token');
  expect(await s.browser.$('.connections .from-global').isExisting()).toBe(false);
}, 90_000);
