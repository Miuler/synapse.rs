// Git Log de todas las ramas (paleta de comandos): grafo con dos carriles, etiquetas de ramas y
// tags, filtro por rama y texto, detalles del commit y diff de un archivo cambiado.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-repolog-'));
  const vault = join(base, 'vault');
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  // Fechas fijas y crecientes: con el mismo segundo el orden de `--date-order` no es estable
  let t = 1_700_000_000;
  const git = (...args: string[]) => {
    const date = `@${(t += 60)} +0000`;
    execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', ...args], {
      cwd: vault,
      env: { ...process.env, GIT_AUTHOR_DATE: date, GIT_COMMITTER_DATE: date },
    });
  };
  git('init', '-q', '-b', 'main');
  writeFileSync(join(vault, 'nota.md'), '# Nota\n');
  git('add', '.');
  git('commit', '-qm', 'inicio');
  git('checkout', '-qb', 'lado');
  writeFileSync(join(vault, 'lado.md'), '# Lado\n');
  git('add', '.');
  git('commit', '-qm', 'trabajo en lado');
  git('tag', 'v1');
  git('checkout', '-q', 'main');
  writeFileSync(join(vault, 'nota.md'), '# Nota\n\nmás\n');
  git('commit', '-qam', 'edita nota en main');
  s = await startAppOnVault(vault);
  await s.browser.$('.file-tree-item[data-path="nota.md"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

type Row = { msg: string; refs: string[]; left: number; selected: boolean };
const rows = async () =>
  (await s.browser.execute(() =>
    [...document.querySelectorAll('.git-repo-log-dialog .repo-log-row')].map((r) => ({
      msg: r.querySelector('.message-text')?.textContent?.trim() ?? '',
      refs: [...r.querySelectorAll('.ref-label')].map((l) => `${l.classList.contains('tag') ? 'tag' : 'branch'}:${l.textContent?.trim()}`),
      left: parseFloat((r.querySelector('.graph-dot') as HTMLElement | null)?.style.left ?? '-1'),
      selected: r.classList.contains('selected'),
    })),
  )) as Row[];

it('abre el Git Log desde la paleta con todas las ramas y el grafo', async () => {
  await s.browser.$('body').click();
  await s.browser.keys(['Control', 'p']);
  const input = await s.browser.$('.synapse-command-palette input');
  await input.waitForExist({ timeout: 15_000 });
  await input.setValue('Log de todas las ramas');
  await s.browser.waitUntil(
    async () => (await s.browser.execute(() => !!document.querySelector('.synapse-command-palette .command-item'))) as boolean,
    { timeout: 15_000 },
  );
  await s.browser.keys(['Enter']);
  await s.browser.$('.git-repo-log-dialog .repo-log-row').waitForExist({ timeout: 10_000 });

  const r = await rows();
  expect(r.map((x) => x.msg)).toEqual(['edita nota en main', 'trabajo en lado', 'inicio']);
  expect(r[0].refs).toEqual(['branch:main']);
  expect(r[1].refs).toEqual(['branch:lado', 'tag:v1']);
  // Las dos puntas van en carriles distintos y se unen en «inicio»
  expect(r[0].left).not.toBe(r[1].left);
  expect(r[2].left).toBe(r[0].left);
  // Se elige el commit de HEAD y se ven sus archivos
  expect(r[0].selected).toBe(true);
  await s.browser.$('.repo-log-details .details-file[data-path="nota.md"]').waitForExist({ timeout: 15_000 });
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/git-repo-log.png`);
});

it('filtra por rama y por texto', async () => {
  const pick = (v: string) =>
    s.browser.execute((v) => {
      const sel = document.querySelector('.git-repo-log-dialog .repo-log-branch') as HTMLSelectElement;
      sel.value = v;
      sel.dispatchEvent(new Event('change', { bubbles: true }));
    }, v);
  await pick('lado');
  await s.browser.waitUntil(async () => (await rows()).length === 2, { timeout: 15_000 });
  expect((await rows()).map((x) => x.msg)).toEqual(['trabajo en lado', 'inicio']);
  await pick('');
  await s.browser.waitUntil(async () => (await rows()).length === 3, { timeout: 15_000 });

  const search = await s.browser.$('.git-repo-log-dialog .repo-log-search input');
  await search.setValue('edita');
  await s.browser.waitUntil(async () => (await rows()).length === 1, { timeout: 15_000 });
  // clearValue no emite `input` en WebKitGTK
  await s.browser.execute(() => {
    const i = document.querySelector('.git-repo-log-dialog .repo-log-search input') as HTMLInputElement;
    i.value = '';
    i.dispatchEvent(new Event('input', { bubbles: true }));
  });
  await s.browser.waitUntil(async () => (await rows()).length === 3, { timeout: 15_000 });
});

it('al elegir un commit muestra sus detalles y abre el diff de un archivo', async () => {
  await s.browser.execute(() => {
    const row = [...document.querySelectorAll('.git-repo-log-dialog .repo-log-row')].find((r) =>
      r.textContent?.includes('trabajo en lado'),
    ) as HTMLElement;
    row.click();
  });
  const file = await s.browser.$('.repo-log-details .details-file[data-path="lado.md"]');
  await file.waitForExist({ timeout: 15_000 });
  const msg = await s.browser.$('.repo-log-details .details-message').getText();
  expect(msg).toBe('trabajo en lado');

  await file.doubleClick();
  await s.browser.waitUntil(
    async () => (await s.browser.execute(() => !document.querySelector('.git-repo-log-dialog'))) as boolean,
    { timeout: 15_000 },
  );
  await s.browser.$('[data-tab-path^="diff-commit:"]').waitForExist({ timeout: 10_000 }).catch(() => {});
  const diffText = (await s.browser.execute(() => document.body.innerText)) as string;
  expect(diffText).toContain('Lado');
});

// ── Menú contextual y marca de HEAD ───────────────────────────────────────
const openLog = async () => {
  await s.browser.keys(['Control', 'p']);
  const input = await s.browser.$('.synapse-command-palette input');
  await input.waitForExist({ timeout: 15_000 });
  await input.setValue('Log de todas las ramas');
  await s.browser.waitUntil(
    async () => (await s.browser.execute(() => !!document.querySelector('.synapse-command-palette .command-item'))) as boolean,
    { timeout: 15_000 },
  );
  await s.browser.keys(['Enter']);
  await s.browser.$('.git-repo-log-dialog .repo-log-row').waitForExist({ timeout: 15_000 });
};
const rightClick = (msg: string) =>
  s.browser.execute((msg) => {
    const row = [...document.querySelectorAll('.git-repo-log-dialog .repo-log-row')].find(
      (r) => r.querySelector('.message-text')?.textContent?.trim() === msg,
    ) as HTMLElement;
    const { x, y } = row.getBoundingClientRect();
    row.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: x + 40, clientY: y + 8 }));
  }, msg);
const headRow = async () =>
  (await s.browser.execute(() => {
    const r = document.querySelector('.git-repo-log-dialog .repo-log-row.head');
    return r ? `${r.querySelector('.message-text')?.textContent?.trim()}|${r.querySelector('.head-label')?.textContent?.trim()}` : '';
  })) as string;
const setInput = (sel: string, v: string) =>
  s.browser.execute(
    (sel, v) => {
      const i = document.querySelector(sel) as HTMLInputElement;
      i.value = v;
      i.dispatchEvent(new Event('input', { bubbles: true }));
    },
    sel,
    v,
  );

it('marca el commit de HEAD y desde el menú crea un tag y una rama con checkout -b', async () => {
  await openLog();
  expect(await headRow()).toBe('edita nota en main|HEAD');

  await rightClick('inicio');
  await s.browser.$('.repo-log-menu [data-action="new-tag"]').waitForExist({ timeout: 15_000 });
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/git-repo-log-menu.png`);
  await s.browser.$('.repo-log-menu [data-action="new-tag"]').click();
  await setInput('.repo-log-prompt input[aria-label="Nombre"]', 'v0');
  await s.browser.$('.repo-log-prompt button[type="submit"]').click();
  await s.browser.waitUntil(async () => (await rows()).find((r) => r.msg === 'inicio')!.refs.includes('tag:v0'), { timeout: 15_000 });

  await rightClick('inicio');
  await s.browser.$('.repo-log-menu [data-action="new-branch"]').click();
  await setInput('.repo-log-prompt input[aria-label="Nombre"]', 'desde-inicio');
  await s.browser.$('.repo-log-prompt button[type="submit"]').click();
  // checkout -b: HEAD pasa a «inicio», en la rama nueva
  await s.browser.waitUntil(async () => (await headRow()) === 'inicio|HEAD', { timeout: 15_000 });
  expect((await rows()).find((r) => r.msg === 'inicio')!.refs).toContain('branch:desde-inicio');
  expect(execFileSync('git', ['branch', '--show-current'], { cwd: join(base, 'vault') }).toString().trim()).toBe('desde-inicio');
});

it('borra un tag y una rama con confirmación, y hace checkout de un commit con HEAD separado', async () => {
  await rightClick('inicio');
  await s.browser.$('.repo-log-menu [data-action="delete-tag"]').click();
  await s.browser.$('.repo-log-prompt [data-action="confirm"]').click();
  await s.browser.waitUntil(async () => !(await rows()).find((r) => r.msg === 'inicio')!.refs.includes('tag:v0'), { timeout: 15_000, timeoutMsg: 'borrar tag' });

  await rightClick('trabajo en lado');
  await s.browser.$('.repo-log-menu [data-action="checkout-commit"]').click();
  await s.browser.$('.repo-log-prompt [data-action="confirm"]').click();
  await s.browser.waitUntil(async () => (await headRow()) === 'trabajo en lado|HEAD (separado)', { timeout: 15_000, timeoutMsg: 'checkout de commit' });

  // «desde-inicio» ya no es la rama actual: se puede borrar (está fusionada en lado)
  await rightClick('inicio');
  await s.browser.$('.repo-log-menu [data-action="delete-branch"]').click();
  await s.browser.$('.repo-log-prompt [data-action="confirm"]').click();
  await s.browser.waitUntil(async () => !(await rows()).find((r) => r.msg === 'inicio')!.refs.includes('branch:desde-inicio'), { timeout: 15_000, timeoutMsg: 'borrar rama' });
});

it('muestra Hash como primera columna por defecto y permite reordenarlas con drag and drop', async () => {
  const headerCols = () =>
    s.browser.execute(() =>
      [...document.querySelectorAll('.git-repo-log-dialog .repo-log-table thead th')].map(
        (th) => th.getAttribute('data-col') ?? '',
      ),
    ) as Promise<string[]>;

  const firstRowCols = () =>
    s.browser.execute(() =>
      [...document.querySelectorAll('.git-repo-log-dialog .repo-log-table tbody tr:first-child td')].map((td) =>
        [...td.classList].find((c) => c.startsWith('col-'))?.replace('col-', '') ?? '',
      ),
    ) as Promise<string[]>;

  expect(await headerCols()).toEqual(['hash', 'message', 'author', 'date']);
  expect(await firstRowCols()).toEqual(['hash', 'message', 'author', 'date']);

  await s.browser.execute(() => {
    const source = document.querySelector('.git-repo-log-dialog th[data-col="author"]') as HTMLElement;
    const target = document.querySelector('.git-repo-log-dialog th[data-col="hash"]') as HTMLElement;
    if (!source || !target) throw new Error('Cabeceras no encontradas');
    const dt = new DataTransfer();
    const r = target.getBoundingClientRect();
    source.dispatchEvent(new DragEvent('dragstart', { bubbles: true, cancelable: true, dataTransfer: dt }));
    target.dispatchEvent(
      new DragEvent('dragover', { bubbles: true, cancelable: true, clientX: r.left + 4, dataTransfer: dt }),
    );
    target.dispatchEvent(
      new DragEvent('drop', { bubbles: true, cancelable: true, clientX: r.left + 4, dataTransfer: dt }),
    );
    source.dispatchEvent(new DragEvent('dragend', { bubbles: true, cancelable: true, dataTransfer: dt }));
  });

  await s.browser.waitUntil(
    async () => (await headerCols()).join(',') === 'author,hash,message,date',
    { timeout: 5_000, timeoutMsg: 'el orden de columnas no cambió tras drag-and-drop' },
  );
  expect(await firstRowCols()).toEqual(['author', 'hash', 'message', 'date']);

  const saved = (await s.browser.execute(() =>
    localStorage.getItem('synapse.gitRepoLogDialog.columns'),
  )) as string | null;
  expect(JSON.parse(saved ?? '[]')).toEqual(['author', 'hash', 'message', 'date']);

  // Cerrar y volver a abrir el diálogo para confirmar persistencia
  await s.browser.keys(['Escape']);
  await s.browser.$('.git-repo-log-dialog').waitForExist({ reverse: true, timeout: 5_000 });
  await openLog();
  expect(await headerCols()).toEqual(['author', 'hash', 'message', 'date']);
  expect(await firstRowCols()).toEqual(['author', 'hash', 'message', 'date']);
});
