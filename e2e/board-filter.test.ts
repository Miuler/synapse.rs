// Dos tableros del mismo proyecto con filtros distintos: al crearlos se pregunta de dónde salen sus
// tareas, cada uno trae las suyas y las que dejan de cumplir el filtro quedan marcadas. Contra un Azure
// DevOps de mentira: nada sale de la máquina.
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

const ANA = 'ana@ejemplo.test';
const items = [
  { id: 1, type: 'Task', assignee: ANA },
  { id: 2, type: 'Bug', assignee: 'luis@ejemplo.test' },
  { id: 3, type: 'Bug', assignee: ANA },
];

/** Los valores de `campo IN (…)` en la consulta; sin esa condición, `null`. */
const chosen = (query: string, field: string) =>
  query.match(new RegExp(`\\[System\\.${field}\\] IN \\(([^)]*)\\)`))?.[1].match(/'[^']*'/g)?.map((v) => v.slice(1, -1)) ?? null;

const server = Bun.serve({
  port: 0,
  hostname: '127.0.0.1',
  async fetch(request) {
    const path = new URL(request.url).pathname;
    if (path.endsWith('/_apis/wit/workitemtypes')) return Response.json({ value: [{ name: 'Task' }, { name: 'Bug' }] });
    if (path.endsWith('/states')) return Response.json({ value: [{ name: 'New', category: 'Proposed' }] });
    if (path.includes('/classificationnodes/')) return Response.json({ name: 'Demo' });
    if (path.endsWith('/_apis/wit/tags')) return Response.json({ count: 0, value: [] });
    if (path.endsWith('/_apis/wit/wiql')) {
      const { query } = (await request.json()) as { query: string };
      const types = chosen(query, 'WorkItemType');
      const people = chosen(query, 'AssignedTo');
      const found = items.filter((i) => (!types || types.includes(i.type)) && (!people || people.includes(i.assignee)));
      return Response.json({ workItems: found.map(({ id }) => ({ id })) });
    }
    if (path.endsWith('/_apis/wit/workitemsbatch')) {
      const { ids } = (await request.json()) as { ids: number[] };
      return Response.json({
        value: items
          .filter((i) => ids.includes(i.id))
          .map((i) => ({
            id: i.id,
            rev: 1,
            fields: {
              'System.Title': `Elemento ${i.id}`,
              'System.State': 'New',
              'System.WorkItemType': i.type,
              'System.AssignedTo': { displayName: i.assignee, uniqueName: i.assignee },
            },
          })),
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
  await s.browser.$('.file-tree-item[data-path="nota.md"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  if (saved === undefined) delete process.env.SYNAPSE_KEYRING_FILE;
  else process.env.SYNAPSE_KEYRING_FILE = saved;
  server.stop(true);
  await s?.close();
  rmSync(keyring, { recursive: true, force: true });
});

const pane = '.tab-pane:not(.hidden)';
const file = (name: string) => readFileSync(join(s.vault, name), 'utf8');
const until = (check: () => boolean | Promise<boolean>, what: string) =>
  s.browser.waitUntil(async () => check(), { timeout: 15_000, timeoutMsg: what });
const dialog = () => s.browser.$('.issues-dialog');
const status = () =>
  s.browser.execute(() => document.querySelector('.tab-pane:not(.hidden) .issues-status')?.textContent ?? '') as Promise<string>;
const shot = async (name: string) => {
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/${name}.png`);
};
/** Los archivos de work items de un tablero. */
const tasks = (name: string) => {
  const folder = join(s.vault, name.slice(0, -'.md'.length));
  if (!existsSync(folder)) return [];
  return readdirSync(folder)
    .filter((f) => /^ado-\d+\.md$/.test(f))
    .map((f) => ({ id: f.slice(0, -'.md'.length), text: readFileSync(join(folder, f), 'utf8') }));
};
const rows = (name: string) => tasks(name).map((t) => t.id);

async function newBoard() {
  await s.browser.$('.file-tree-item[data-path="nota.md"]').click({ button: 'right' });
  await s.browser.$('.context-menu-item').waitForExist({ timeout: 5_000 });
  await s.browser.execute(() =>
    ([...document.querySelectorAll('.context-menu-item')].find((i) => i.textContent?.includes('Nuevo tablero')) as HTMLElement).click(),
  );
  await dialog().waitForExist({ timeout: 15_000 });
}

/**
 * Pone un texto en un campo. No con `setValue`: tras el clic derecho del explorador, el controlador
 * escribe sin mayúsculas (`http;//` en vez de `http://`).
 */
const fill = (selector: string, value: string) =>
  s.browser.execute(
    (selector, value) => {
      const input = document.querySelector(selector) as HTMLInputElement;
      input.value = value;
      input.dispatchEvent(new Event('input', { bubbles: true }));
    },
    selector,
    value,
  );

/** Escribe en el buscador de un criterio y elige con Enter. */
async function pick(criterion: string, text: string, expected: string) {
  await s.browser.$(`.issues-dialog input#${criterion}`).click();
  await fill(`.issues-dialog input#${criterion}`, text);
  await until(
    () => s.browser.execute((c) => document.querySelector(`#${c}-options li.highlighted`)?.textContent?.trim() ?? '', criterion).then((v) => v === expected),
    `no se ofreció ${expected}`,
  );
  await s.browser.keys(['Enter']);
  await until(
    () => s.browser.execute((c) => document.querySelector(`[data-criterion="${c}"] .chips`)?.textContent ?? '', criterion).then((v) => String(v).includes(expected)),
    `no quedó elegido ${expected}`,
  );
}

async function save() {
  await s.browser.$('.issues-dialog button[type="submit"]').click();
}

async function pull() {
  await s.browser.$(`${pane} .issues-pull`).click();
  await until(async () => (await status()).includes('Traído'), 'no se trajo');
}

it('al crear un tablero se pide el origen y su filtro, y trae solo lo que lo cumple', async () => {
  await newBoard();
  expect(await s.browser.$('.issues-title').getText()).toBe('¿De dónde salen las tareas?');
  await fill('.issues-dialog input[name="scope"]', scope());
  await s.browser.$('.issues-dialog input[name="mode"][value="criteria"]').click();
  await pick('type', 'Bu', 'Bug');
  // Escape cierra la lista sin cerrar el diálogo
  await s.browser.$('.issues-dialog input#assignee').click();
  await until(
    () => s.browser.execute(() => document.querySelector('#assignee-options')?.textContent?.includes('Yo') ?? false),
    'no se ofreció «Yo»',
  );
  await shot('filtro-criterios');
  await s.browser.keys(['Escape']);
  await s.browser.$('#assignee-options').waitForExist({ reverse: true, timeout: 5_000 });
  expect(await dialog().isExisting()).toBe(true);
  await save();
  await dialog().waitForExist({ reverse: true, timeout: 15_000 });
  await until(() => file('Tablero.board.md').includes("[System.WorkItemType] IN ('Bug')"), 'no se guardó el filtro');
  expect(file('Tablero.board.md')).toContain('filter:\n    type:\n    - Bug\n');

  await pull();
  await until(() => rows('Tablero.board.md').length === 2, 'no se guardó lo traído');
  expect(rows('Tablero.board.md').sort()).toEqual(['ado-2', 'ado-3']);
}, 120_000);

it('otro tablero del mismo proyecto, filtrado por persona, trae las suyas', async () => {
  await newBoard();
  await fill('.issues-dialog input[name="scope"]', scope());
  await s.browser.$('.issues-dialog input[name="mode"][value="criteria"]').click();
  // Una persona que el proveedor no lista se escribe
  await pick('assignee', ANA, ANA);
  await s.browser.keys(['Escape']);
  await save();
  await dialog().waitForExist({ reverse: true, timeout: 15_000 });
  await pull();
  await until(() => rows('Tablero 1.board.md').length === 2, 'no se guardó lo traído');
  expect(rows('Tablero 1.board.md').sort()).toEqual(['ado-1', 'ado-3']);
  expect(rows('Tablero.board.md').sort()).toEqual(['ado-2', 'ado-3']);
}, 120_000);

it('al cambiar el filtro, lo que ya no lo cumple sigue en el tablero y queda marcado', async () => {
  await s.browser.$('.file-tree-item[data-path="Tablero.board.md"]').click();
  await s.browser.$(`${pane} .issues-settings`).click();
  await dialog().waitForExist({ timeout: 15_000 });
  expect(await s.browser.$('.issues-dialog input[name="mode"][value="criteria"]').isSelected()).toBe(true);
  await s.browser.$('.issues-dialog [data-criterion="type"] .chip button').click();
  await pick('type', 'Ta', 'Task');
  await s.browser.keys(['Escape']);
  await save();
  await dialog().waitForExist({ reverse: true, timeout: 15_000 });
  await pull();
  await until(async () => (await status()).includes('Fuera del filtro: '), 'no se avisó');
  await until(() => rows('Tablero.board.md').length === 3, 'no se añadió la nueva');
  expect(rows('Tablero.board.md').sort()).toEqual(['ado-1', 'ado-2', 'ado-3']);
  await until(() => tasks('Tablero.board.md').filter((t) => t.text.includes('"outside":true')).length === 2, 'no se guardó la marca');
  const marked = () =>
    s.browser.execute(() =>
      [...document.querySelectorAll('.tab-pane:not(.hidden) .card')]
        .filter((c) => c.querySelector('.outside-badge'))
        .map((c) => (c as HTMLElement).dataset.task)
        .sort(),
    ) as Promise<string[]>;
  await until(async () => (await marked()).join() === 'ado-2,ado-3', 'no se marcaron las tarjetas');
  await shot('filtro-fuera');
}, 120_000);

it('cambiar de proyecto con tareas vinculadas pide confirmación antes de desvincularlas', async () => {
  const before = file('Tablero.board.md');
  await s.browser.$(`${pane} .issues-settings`).click();
  await dialog().waitForExist({ timeout: 15_000 });
  await fill('.issues-dialog input[name="scope"]', `${server.url.origin}/org/Otro`);
  await s.browser.$('.issues-dialog input[name="mode"][value="mine"]').click();
  await save();
  await s.browser.$('.confirm-dialog').waitForExist({ timeout: 10_000 });
  await shot('filtro-desvincular');
  await s.browser.$('.confirm-dialog-btn.cancel').click();
  await s.browser.$('.confirm-dialog').waitForExist({ reverse: true, timeout: 10_000 });
  expect(await dialog().isExisting()).toBe(true);
  expect(file('Tablero.board.md')).toBe(before);

  await save();
  await s.browser.$('.confirm-dialog').waitForExist({ timeout: 10_000 });
  await s.browser.$('.confirm-dialog-btn.confirm').click();
  await dialog().waitForExist({ reverse: true, timeout: 10_000 });
  await until(
    () => file('Tablero.board.md').includes('/org/Otro') && !tasks('Tablero.board.md').some((t) => t.text.includes('- sync:')),
    'no se desvincularon',
  );
  expect(rows('Tablero.board.md')).toHaveLength(3);
}, 90_000);
