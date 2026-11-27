// «Traer» y «Enviar» de un tablero contra un Azure DevOps de mentira: nada sale de la máquina.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

type Item = { id: number; rev: number; title: string; state: string };

const items = new Map<number, Item>([[7, { id: 7, rev: 1, title: 'Preparar | entorno', state: 'To Do' }]]);
const writes: string[] = [];
const asWorkItem = (i: Item) => ({ id: i.id, rev: i.rev, fields: { 'System.Title': i.title, 'System.State': i.state, 'System.WorkItemType': 'Task' } });

const server = Bun.serve({
  port: 0,
  hostname: '127.0.0.1',
  async fetch(request) {
    if (request.headers.get('authorization') !== `Basic ${btoa(':token-falso')}`) return new Response('', { status: 401 });
    const path = new URL(request.url).pathname;
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
      return Response.json({});
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

let s: Session;
const saved = ['AZURE_DEVOPS_PAT', 'AZURE_DEVOPS_URL'].map((name) => [name, process.env[name]] as const);

beforeAll(async () => {
  process.env.AZURE_DEVOPS_PAT = 'token-falso';
  process.env.AZURE_DEVOPS_URL = server.url.origin;
  s = await startApp({ 'equipo.board.md': board });
  await s.browser.$('.file-tree-item[data-path="equipo.board.md"]').waitForExist({ timeout: 30_000 });
  await s.browser.$('.file-tree-item[data-path="equipo.board.md"]').click();
  await s.browser.$('.azure-pull').waitForExist({ timeout: 15_000 });
}, 120_000);
afterAll(async () => {
  for (const [name, value] of saved) {
    if (value === undefined) delete process.env[name];
    else process.env[name] = value;
  }
  server.stop(true);
  await s?.close();
});

const file = () => readFileSync(join(s.vault, 'equipo.board.md'), 'utf8');
const until = (check: () => boolean | Promise<boolean>, what: string) =>
  s.browser.waitUntil(async () => check(), { timeout: 15_000, timeoutMsg: what });
const status = () => s.browser.execute(() => document.querySelector('.azure-status')?.textContent ?? '') as Promise<string>;
const shot = async (name: string) => {
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/${name}.png`);
};

it('«Traer» añade los work items y guarda el vínculo sin tocar Azure DevOps', async () => {
  await s.browser.$('.azure-pull').click();
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

  await s.browser.$('.azure-push').click();
  await s.browser.$('.azure-confirm').waitForExist({ timeout: 15_000 });
  await shot('azure-revisar');
  expect(writes).toEqual([]);
  await s.browser.$('.azure-confirm').click();
  await until(async () => (await status()).includes('1 actualizadas, 0 creadas'), 'no se envió');
  expect(writes).toEqual(['update 7 In Progress']);

  // La tarea sin vínculo se crea solo si se marca
  await s.browser.$('.azure-push').click();
  const create = await s.browser.$('.azure-create[data-task="t1"] input');
  await create.waitForExist({ timeout: 15_000 });
  expect(await s.browser.$('.azure-confirm').isEnabled()).toBe(false);
  await create.click();
  await s.browser.$('.azure-confirm').click();
  await until(async () => (await status()).includes('1 creadas'), 'no se creó');
  await until(() => file().includes('id: 101'), 'no se guardó el vínculo nuevo');
  expect(writes).toEqual(['update 7 In Progress', 'create 101']);

  await s.browser.$('.azure-push').click();
  await until(async () => (await status()).includes('No hay nada que enviar'), 'volvió a proponer cambios');
  expect(writes).toHaveLength(2);
}, 120_000);

it('la configuración se guarda en el tablero y un servidor ajeno no recibe el token', async () => {
  await s.browser.$('.azure-settings').click();
  const url = await s.browser.$('.azure-dialog input[name="project_url"]');
  await url.waitForExist({ timeout: 15_000 });
  await shot('azure-configurar');
  await url.setValue('https://ejemplo.invalid/org/Demo');
  await s.browser.$('.azure-dialog button[type="submit"]').click();
  await until(() => file().includes('project_url: https://ejemplo.invalid/org/Demo') && !file().includes('id: 101'), 'no se guardó');
  await s.browser.$('.azure-pull').click();
  await until(async () => (await status()).includes('El token solo se envía a Azure DevOps'), 'no se rechazó el servidor');
}, 90_000);
