// «Traer» y «Enviar» de un tablero contra un Azure DevOps y un 1Password de mentira: nada sale de la máquina.
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

type Item = { id: number; rev: number; title: string; state: string };

const items = new Map<number, Item>([[7, { id: 7, rev: 1, title: 'Preparar | entorno', state: 'To Do' }]]);
const writes: string[] = [];
const tokens: string[] = [];
const asWorkItem = (i: Item) => ({
  id: i.id,
  rev: i.rev,
  fields: {
    'System.Title': i.title,
    'System.State': i.state,
    'System.WorkItemType': 'Task',
    'System.Description': '<p>Descripción de <b>Azure</b></p>',
  },
});
/** Los comentarios de cada work item, en HTML como los devuelve Azure DevOps. */
const remoteComments = new Map<number, { id: number; text: string; author: string }[]>([[7, [{ id: 900, text: '<div>visto por <i>Ana</i></div>', author: 'Ana' }]]]);
let nextComment = 901;
const activityReads: string[] = [];

const server = Bun.serve({
  port: 0,
  hostname: '127.0.0.1',
  async fetch(request) {
    const token = atob(request.headers.get('authorization')?.replace('Basic ', '') ?? '').slice(1);
    if (!['token-falso', 'token-de-op', 'token-pegado'].includes(token)) return new Response('', { status: 401 });
    tokens.push(token);
    const path = new URL(request.url).pathname;
    const listed = path.match(/\/workItems\/(\d+)\/comments$/);
    if (request.method === 'GET' && listed) {
      activityReads.push(`comments ${listed[1]}`);
      return Response.json({
        comments: (remoteComments.get(Number(listed[1])) ?? []).map((c) => ({
          commentId: c.id,
          renderedText: c.text,
          createdBy: { displayName: c.author },
          createdDate: `2026-02-01T10:00:${String(c.id % 60).padStart(2, '0')}Z`,
        })),
      });
    }
    if (request.method === 'GET' && /\/workItems\/\d+\/updates$/.test(path)) {
      const fields = { 'System.State': { newValue: 'To Do' }, 'System.ChangedDate': { newValue: '2026-01-01T09:00:00Z' } };
      return Response.json({ value: [{ id: 1, revisedBy: { displayName: 'Luis' }, fields }] });
    }
    // Los estados de cada tipo: sin ninguno, valen los nombres habituales
    if (request.method === 'GET') return Response.json({ value: [] });
    const body = (await request.json()) as any;
    if (path === '/org/Demo/_apis/wit/wiql') return Response.json({ workItems: [...items.keys()].map((id) => ({ id })) });
    if (path === '/org/_apis/wit/workitemsbatch') {
      return Response.json({ value: (body.ids as number[]).map((id) => (items.has(id) ? asWorkItem(items.get(id)!) : null)) });
    }
    const patch = (target: Item) => {
      for (const op of body as { op: string; path: string; value: string }[]) {
        if (op.path === '/fields/System.Title') target.title = op.value;
        if (op.path === '/fields/System.State') target.state = op.value;
      }
      return Response.json(asWorkItem(target));
    };
    const comment = path.match(/\/workItems\/(\d+)\/comments$/);
    if (comment) {
      writes.push(`comment ${comment[1]} ${body.text}`);
      const id = nextComment++;
      const target = items.get(Number(comment[1]))!;
      target.rev++;
      remoteComments.set(target.id, [...(remoteComments.get(target.id) ?? []), { id, text: body.text, author: 'Yo' }]);
      return Response.json({ id });
    }
    const update = path.match(/\/workitems\/(\d+)$/);
    if (update) {
      const target = items.get(Number(update[1]))!;
      writes.push(`update ${target.id} ${body.slice(1).map((o: any) => o.value).join(',')}`);
      target.rev++;
      return patch(target);
    }
    if (path.endsWith('/workitems/$Task')) {
      const target = { id: 100 + items.size, rev: 1, title: '', state: 'To Do' };
      items.set(target.id, target);
      writes.push(`create ${target.id}`);
      return patch(target);
    }
    return Response.json({ message: `ruta inesperada ${path}` }, { status: 500 });
  },
});

/** Una conexión al servidor falso; `secret`, su referencia de 1Password. */
function writeConfig(secret?: string) {
  const config = join(s.home, '.config/synapsers');
  const connection = { id: 'pruebas', provider: 'azure_devops', name: 'Pruebas', server: `${server.url.origin}/org`, secret };
  mkdirSync(config, { recursive: true });
  writeFileSync(join(config, 'config.jsonc'), JSON.stringify({ issues: { connections: [connection] } }));
}

const board = `---
type: board
default_view: table
columns:
  - { id: todo, label: "Por hacer", color: "var(--text-muted)" }
  - { id: doing, label: "En progreso", color: "var(--accent)" }
  - { id: done, label: "Hecho", color: "#22c55e" }
azure_devops:
  project_url: ${server.url.origin}/org/Demo
---

| id | Tarea | Estado | Inicio | Duración | Depende | Progreso |
|---|---|---|---|---|---|---|
| t1 | Solo local | todo | 2026-01-01 | 1d | - | 0% |
`;

/** Un tablero con tabla ya vinculado, con un comentario enviado y otro por enviar. */
const toConvert = () => `---
type: board
default_view: kanban
columns:
  - { id: todo, label: "Por hacer", color: "var(--text-muted)" }
  - { id: doing, label: "En progreso", color: "var(--accent)" }
azure_devops:
  project_url: ${server.url.origin}/org/Demo
  links:
    ado-7:
      id: 7
      title: "Preparar | entorno"
      state: In Progress
      column: doing
      sent: ["2026-01-02T10:00:00"]
---

| id | Tarea | Estado | Inicio | Duración | Depende | Progreso |
|---|---|---|---|---|---|---|
| ado-7 | Preparar \\| entorno | doing | 2026-01-01 | 1d | - | 0% |

## Comentarios

### ado-7
- \`2026-01-02T10:00:00\` — ya enviado
- \`2026-01-03T10:00:00\` — pendiente
`;

let s: Session;
const saved = ['AZURE_DEVOPS_PAT', 'SYNAPSE_OP_PATH', 'SYNAPSE_KEYRING_FILE'].map((name) => [name, process.env[name]] as const);
// Un `op` falso: anota con qué se le llama y entrega otro token
const bin = mkdtempSync(join(tmpdir(), 'synapse-op-'));
const opCalls = join(bin, 'llamadas');
writeFileSync(join(bin, 'op'), `#!/bin/sh\necho "$@" >> '${opCalls}'\nprintf token-de-op\n`);
chmodSync(join(bin, 'op'), 0o755);

beforeAll(async () => {
  // La credencial global no debe servir a la conexión
  process.env.AZURE_DEVOPS_PAT = 'token-global';
  process.env.SYNAPSE_OP_PATH = join(bin, 'op');
  // El llavero de las pruebas es una carpeta: no se toca el del sistema
  process.env.SYNAPSE_KEYRING_FILE = join(bin, 'llavero');
  mkdirSync(join(bin, 'llavero'));
  writeFileSync(join(bin, 'llavero/issues-pruebas'), 'token-falso');
  s = await startApp({ 'equipo.board.md': board, 'convertido.board.md': toConvert() });
  writeConfig();
  await s.browser.$('.file-tree-item[data-path="equipo.board.md"]').waitForExist({ timeout: 30_000 });
  await s.browser.$('.file-tree-item[data-path="equipo.board.md"]').click();
  await s.browser.$('.issues-pull').waitForExist({ timeout: 15_000 });
}, 120_000);
afterAll(async () => {
  for (const [name, value] of saved) {
    if (value === undefined) delete process.env[name];
    else process.env[name] = value;
  }
  server.stop(true);
  await s?.close();
  rmSync(bin, { recursive: true, force: true });
});

const file = () => readFileSync(join(s.vault, 'equipo.board.md'), 'utf8');
const until = (check: () => boolean | Promise<boolean>, what: string) =>
  s.browser.waitUntil(async () => check(), { timeout: 15_000, timeoutMsg: what });
const status = () => s.browser.execute(() => document.querySelector('.issues-status')?.textContent ?? '') as Promise<string>;
const shot = async (name: string) => {
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/${name}.png`);
};

it('«Traer» añade los work items y guarda el vínculo sin tocar Azure DevOps', async () => {
  await s.browser.$('.issues-pull').click();
  await until(async () => (await status()).includes('1 nuevas'), 'no se trajo el work item');
  await until(() => file().includes('| ado-7 | Preparar \\| entorno | todo |') && file().includes('id: 7'), 'no se guardó');
  expect(writes).toEqual([]);
  await shot('azure-traer');
}, 90_000);

it('«Enviar» pide revisar, envía lo cambiado y solo crea lo marcado, sin repetir', async () => {
  // Estado de ado-7 y un comentario
  await s.browser.execute(() => {
    const row = [...document.querySelectorAll('.table-view tbody tr')].find((r) => r.querySelector('code')?.textContent === 'ado-7')!;
    const select = row.querySelector('select') as HTMLSelectElement;
    select.value = 'doing';
    select.dispatchEvent(new Event('change', { bubbles: true }));
  });
  await until(() => file().includes('entorno | doing |'), 'no se movió la tarea');

  await s.browser.$('.issues-push').click();
  await s.browser.$('.issues-confirm').waitForExist({ timeout: 15_000 });
  await shot('azure-revisar');
  expect(writes).toEqual([]);
  await s.browser.$('.issues-confirm').click();
  await until(async () => (await status()).includes('1 actualizadas, 0 creadas'), 'no se envió');
  expect(writes).toEqual(['update 7 In Progress']);

  // La tarea sin vínculo se crea solo si se marca
  await s.browser.$('.issues-push').click();
  const create = await s.browser.$('.issues-create[data-task="t1"] input');
  await create.waitForExist({ timeout: 15_000 });
  expect(await s.browser.$('.issues-confirm').isEnabled()).toBe(false);
  await create.click();
  await s.browser.$('.issues-confirm').click();
  await until(async () => (await status()).includes('1 creadas'), 'no se creó');
  await until(() => file().includes('id: 101'), 'no se guardó el vínculo nuevo');
  expect(writes).toEqual(['update 7 In Progress', 'create 101']);

  await s.browser.$('.issues-push').click();
  await until(async () => (await status()).includes('No hay nada que enviar'), 'volvió a proponer cambios');
  expect(writes).toHaveLength(2);
}, 120_000);

it('con una referencia en la configuración, el token sale de 1Password', async () => {
  expect(new Set(tokens)).toEqual(new Set(['token-falso']));
  writeConfig('op://Pruebas/Azure/token');
  tokens.length = 0;
  await s.browser.$('.issues-pull').click();
  await until(async () => (await status()).includes('0 nuevas'), 'no se trajo con el token de 1Password');
  expect(new Set(tokens)).toEqual(new Set(['token-de-op']));
  // Una sola lectura para las dos peticiones de «Traer»
  expect(readFileSync(opCalls, 'utf8')).toBe('read --no-newline op://Pruebas/Azure/token\n');
}, 90_000);

it('en Configuración se cambia la ruta de 1Password, o se pega un token que va al llavero', async () => {
  const config = () => readFileSync(join(s.home, '.config/synapsers/config.jsonc'), 'utf8');
  const keyring = join(bin, 'llavero/azure-devops-token');
  // La credencial única anterior, sin conexiones
  writeFileSync(join(s.home, '.config/synapsers/config.jsonc'), '{ "azure_devops": { "token": "op://Pruebas/Azure/token" } }');
  await s.browser.$('[aria-label="Configuración"]').click();
  await s.browser.$('.settings-nav-item').waitForExist({ timeout: 15_000 });
  await s.browser.execute(() =>
    ([...document.querySelectorAll('.settings-nav-item')].find((b) => b.textContent?.trim() === 'Conexiones') as HTMLElement).click(),
  );
  const input = await s.browser.$('.azure-token-input');
  await input.waitForExist({ timeout: 15_000 });
  await until(async () => (await input.getValue()) === 'op://Pruebas/Azure/token', 'no mostró la ruta guardada');
  expect(await input.getAttribute('type')).toBe('text');
  await input.setValue('"op://Pruebas/Otro elemento/token"');
  await s.browser.keys(['Enter']);
  await until(() => config().includes('op://Pruebas/Otro elemento/token'), 'no se guardó la ruta');
  await until(async () => (await input.getValue()) === 'op://Pruebas/Otro elemento/token', 'quedaron las comillas');

  await input.setValue('token-pegado');
  expect(await input.getAttribute('type')).toBe('password');
  await shot('azure-ajustes');
  await s.browser.keys(['Enter']);
  await until(() => existsSync(keyring), 'no se guardó en el llavero');
  expect(readFileSync(keyring, 'utf8')).toBe('token-pegado');
  expect(config()).not.toContain('op://');
  expect(config()).not.toContain('token-pegado');
  await s.browser.$('.azure-token-remove').waitForExist({ timeout: 5_000 });
  expect(await input.getValue()).toBe('');
  // En desarrollo se anotan las llamadas a Rust: esta, sin el token
  expect(await s.browser.execute(() => JSON.stringify((window as any).__synapseIpcLog))).not.toContain('token-pegado');
  await shot('azure-ajustes-llavero');
  await s.browser.keys(['Escape']);
  await s.browser.$('.synapse-settings').waitForExist({ reverse: true, timeout: 10_000 });

  // También se quita desde la configuración del tablero
  await s.browser.$('.issues-settings').click();
  await s.browser.$('.issues-dialog .azure-token-remove').waitForExist({ timeout: 15_000 });
  await shot('azure-configurar-token');
  await s.browser.$('.issues-dialog .azure-token-remove').click();
  await until(() => !existsSync(keyring), 'no se quitó del llavero');
  await s.browser.keys(['Escape']);
  await s.browser.$('.issues-dialog').waitForExist({ reverse: true, timeout: 10_000 });
}, 120_000);

it('la configuración se guarda en el tablero y un servidor ajeno no recibe el token', async () => {
  writeConfig();
  tokens.length = 0;
  await s.browser.$('.issues-settings').click();
  const url = await s.browser.$('.issues-dialog input[name="scope"]');
  await url.waitForExist({ timeout: 15_000 });
  await shot('azure-configurar');
  await url.setValue('https://ejemplo.invalid/org/Demo');
  await s.browser.$('.issues-dialog button[type="submit"]').click();
  // Con tareas vinculadas, otro proyecto se confirma
  await s.browser.$('.confirm-dialog-btn.confirm').waitForExist({ timeout: 10_000 });
  await s.browser.$('.confirm-dialog-btn.confirm').click();
  await until(() => file().includes('project_url: https://ejemplo.invalid/org/Demo') && !file().includes('id: 101'), 'no se guardó');
  await s.browser.$('.issues-pull').click();
  await until(async () => (await status()).includes('Ninguna conexión cubre https://ejemplo.invalid/org/Demo'), 'no se rechazó el servidor');
  expect(tokens).toEqual([]);
}, 90_000);

it('tras convertir un tablero, «Enviar» manda cada comentario pendiente una sola vez', async () => {
  const pane = '.tab-pane:not(.hidden)';
  const task = () => readFileSync(join(s.vault, 'convertido.board/ado-7.md'), 'utf8');
  const shown = () =>
    s.browser.execute(() => document.querySelector('.tab-pane:not(.hidden) .issues-status')?.textContent ?? '') as Promise<string>;
  const comments = () => writes.filter((w) => w.startsWith('comment'));
  expect(comments()).toEqual([]);

  await s.browser.$('.file-tree-item[data-path="convertido.board.md"]').click();
  await s.browser.$(`${pane} .board-convert button`).waitForExist({ timeout: 15_000 });
  await s.browser.$(`${pane} .board-convert button`).click();
  await s.browser.$('.confirm-dialog-btn.confirm').waitForExist({ timeout: 10_000 });
  await s.browser.$('.confirm-dialog-btn.confirm').click();
  await until(() => existsSync(join(s.vault, 'convertido.board/ado-7.md')), 'no se convirtió');
  await s.browser.$(`${pane} .card[data-task="ado-7"] .card-title`).waitForExist({ timeout: 15_000 });
  await s.browser.$(`${pane} .card[data-task="ado-7"] .card-title`).doubleClick();
  await s.browser.$('.task-dialog .comment-form input').waitForExist({ timeout: 10_000 });
  await s.browser.$('.task-dialog .comment-form input').click();
  await s.browser.keys('nuevo');
  await s.browser.keys(['Enter']);
  await until(() => task().includes('\n  nuevo\n'), 'no se guardó el comentario');
  await s.browser.$('.task-dialog .resizable-dialog-close').click();
  await s.browser.$('.task-dialog').waitForExist({ reverse: true, timeout: 5_000 });

  await s.browser.$(`${pane} .issues-push`).click();
  await s.browser.$('.issues-confirm').waitForExist({ timeout: 15_000 });
  await s.browser.$('.issues-confirm').click();
  await until(async () => (await shown()).includes('2 comentarios'), 'no se envió');
  expect(comments().sort()).toEqual(['comment 7 nuevo', 'comment 7 pendiente']);
  await until(() => /"sent":\["2026-01-02T10:00:00","2026-01-03T10:00:00","[^"]+"\]/.test(task()), 'no se anotó lo enviado');

  // Releído del disco, no vuelve a proponer nada
  await s.browser.$(`${pane} .issues-push`).click();
  await until(async () => (await shown()).includes('No hay nada que enviar'), 'volvió a proponer cambios');
  expect(comments()).toHaveLength(2);

  // Lo enviado ya tiene el id que le dio el proveedor
  await until(() => /comment · id:90\d\n  nuevo/.test(task()), 'no se anotó el id del comentario');

  // «Traer» guarda la descripción y la actividad sin repetir lo enviado
  const count = (text: string) => task().split(`\n  ${text}\n`).length - 1;
  activityReads.length = 0;
  await s.browser.$(`${pane} .issues-pull`).click();
  await until(() => task().includes('## Description\n\n> Descripción de **Azure**'), 'no se guardó la descripción');
  await until(() => task().includes('visto por *Ana*'), 'no se guardó el comentario ajeno');
  expect(task()).toMatch(/history · Luis · id:rev-1\n  State: To Do/);
  expect(task()).toMatch(/comment · Ana · id:900/);
  expect([count('pendiente'), count('nuevo'), count('ya enviado')]).toEqual([1, 1, 1]);
  expect(activityReads).toContain('comments 7');
  expect(readFileSync(join(s.vault, 'convertido.board.md'), 'utf8')).toContain('synced_at:');

  await s.browser.$(`${pane} .card[data-task="ado-7"] .card-title`).doubleClick();
  await s.browser.$('.task-dialog .history').waitForExist({ timeout: 10_000 });
  expect(await s.browser.$('.task-dialog .description').getText()).toBe('Descripción de **Azure**');
  await until(async () => (await s.browser.$('.task-dialog .comments-list').getText()).includes('Ana'), 'no se ve quién comentó');
  await shot('azure-actividad');
  await s.browser.$('.task-dialog .resizable-dialog-close').click();
  await s.browser.$('.task-dialog').waitForExist({ reverse: true, timeout: 5_000 });

  // Sin cambios en el proveedor no se vuelve a pedir la actividad
  activityReads.length = 0;
  const saved = task();
  await s.browser.$(`${pane} .issues-pull`).click();
  await until(async () => (await shown()).includes('0 nuevas, 0 actualizadas'), 'no se trajo otra vez');
  expect(activityReads).toEqual([]);
  expect(task()).toBe(saved);
  // Lo traído no se ofrece para enviar
  await s.browser.$(`${pane} .issues-push`).click();
  await until(async () => (await shown()).includes('No hay nada que enviar'), 'ofreció enviar lo traído');
  expect(comments()).toHaveLength(2);
}, 180_000);
