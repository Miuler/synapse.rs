// Merge desde el Git Log: menú del commit, lista de conflictos, editor de tres paneles, confirmar,
// deshacer y cancelar.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let vault: string;

const git = (...args: string[]) => execFileSync('git', args, { cwd: vault }).toString().trim();
const read = (file: string) => readFileSync(join(vault, file), 'utf8');
const lines = (...l: string[]) => l.map((x) => `${x}\n`).join('');
const merging = () => existsSync(join(vault, '.git', 'MERGE_HEAD'));

// `main` y `otra` chocan en la línea «dos»; cada una cambia además otra línea sin conflicto
// El relleno hace que el archivo no quepa en el panel (desplazamiento sincronizado)
const filler = Array.from({ length: 80 }, (_, i) => `relleno ${i}`);
const base = ['uno', 'dos', 'tres', 'cuatro', 'cinco', 'seis', 'siete', ...filler];
const ours = base.with(1, 'dos mío').with(3, 'cuatro mío');
const theirs = base.with(1, 'dos suyo').with(5, 'seis suyo');

beforeAll(async () => {
  vault = mkdtempSync(join(tmpdir(), 'synapse-e2e-merge-'));
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  git('init', '-q', '-b', 'main');
  git('config', 'user.name', 'e2e');
  git('config', 'user.email', 'e2e@local');
  git('config', 'commit.gpgsign', 'false');
  writeFileSync(join(vault, '.gitignore'), '.synapse/\n');
  writeFileSync(join(vault, 'nota.md'), lines(...base));
  writeFileSync(join(vault, 'viejo.md'), 'viejo\n');
  git('add', '.');
  git('commit', '-qm', 'inicio');
  git('checkout', '-q', '-b', 'otra');
  writeFileSync(join(vault, 'nota.md'), lines(...theirs));
  writeFileSync(join(vault, 'viejo.md'), 'viejo cambiado\n');
  git('commit', '-qam', 'suyo');
  git('checkout', '-q', 'main');
  writeFileSync(join(vault, 'nota.md'), lines(...ours));
  git('rm', '-q', 'viejo.md');
  git('commit', '-qam', 'mio');

  s = await startAppOnVault(vault);
  await s.browser.$('.git-branch-badge').waitForExist({ timeout: 30_000 });
  await openLog();
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(vault, { recursive: true, force: true });
});

const until = (check: () => boolean | Promise<boolean>, what: string) =>
  s.browser.waitUntil(async () => check(), { timeout: 15_000, timeoutMsg: what });

async function openLog() {
  await s.browser.keys(['Control', 'p']);
  const input = await s.browser.$('.synapse-command-palette input');
  await input.waitForExist({ timeout: 15_000 });
  await input.setValue('Log de todas las ramas');
  await s.browser.$('.synapse-command-palette .command-item').waitForExist({ timeout: 15_000 });
  await s.browser.keys(['Enter']);
  await s.browser.$('.git-repo-log-dialog .repo-log-row').waitForExist({ timeout: 15_000 });
}

const press = async (selector: string) => {
  await s.browser.$(selector).waitForEnabled({ timeout: 10_000 });
  await s.browser.execute((selector) => (document.querySelector(selector) as HTMLElement).click(), selector);
};

async function startMerge() {
  await until(
    () =>
      s.browser.execute(() => {
        const row = [...document.querySelectorAll('.git-repo-log-dialog .repo-log-row')].find(
          (r) => r.querySelector('.message-text')?.textContent?.trim() === 'suyo',
        );
        row?.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: 300, clientY: 200 }));
        return !!row;
      }) as Promise<boolean>,
    'el log no muestra «suyo»',
  );
  await press('.repo-log-menu [data-action="merge"]');
  await press('.repo-log-prompt [data-action="confirm"]');
  await s.browser.$('.git-merge-dialog .git-merge-row[data-path="nota.md"]').waitForExist({ timeout: 15_000 });
}

const paneText = (pane: string) =>
  s.browser.execute(
    (pane) => [...document.querySelectorAll(`.git-merge-dialog .merge-pane.${pane} .cm-line`)].map((l) => l.textContent).join('\n'),
    pane,
  ) as Promise<string>;
const counter = () =>
  s.browser.execute(() => {
    const el = document.querySelector<HTMLElement>('.git-merge-dialog .merge-counter');
    return `${el?.dataset.changes}/${el?.dataset.conflicts}`;
  }) as Promise<string>;
const blockAction = (pane: string, chunk: number, action: string) =>
  press(`.merge-pane.${pane} .merge-actions[data-chunk="${chunk}"] [data-action="${action}"]`);

it('cancelar el merge devuelve la rama como estaba', async () => {
  const before = git('rev-parse', 'HEAD');
  await startMerge();
  expect(merging()).toBe(true);
  await press('.git-merge-dialog [data-action="abort-merge"]');
  await press('.git-merge-dialog [data-action="confirm-abort"]');
  await until(() => !merging(), 'no se canceló');
  expect(git('rev-parse', 'HEAD')).toBe(before);
  expect(read('nota.md')).toBe(lines(...ours));
}, 60_000);

it('resuelve en el editor de tres paneles, confirma el merge y lo deshace', async () => {
  const before = git('rev-parse', 'HEAD');
  await openLog();
  await startMerge();

  // El archivo borrado en la rama actual solo admite elegir una versión
  const row = '.git-merge-dialog .git-merge-row[data-path="viejo.md"]';
  expect(await s.browser.$(`${row} [data-action="open-merge"]`).isEnabled()).toBe(false);

  // Los nombres de las columnas caen sobre sus celdas
  const columns = (await s.browser.execute(() =>
    [...document.querySelectorAll('.git-merge-dialog .git-merge-row')].map((r) =>
      [...r.children].slice(0, 3).map((c) => Math.round(c.getBoundingClientRect().left)).join(),
    ),
  )) as string[];
  expect(new Set(columns).size).toBe(1);

  // Su diff muestra lo que se borraría: Enter lo abre, porque no hay nada que fusionar
  await s.browser.execute(() => (document.querySelector('.git-merge-dialog .git-merge-list') as HTMLElement).focus());
  await s.browser.keys(['ArrowDown']);
  await s.browser.keys(['Enter']);
  await s.browser.$('.git-merge-dialog .merge-diff .cm-mergeView').waitForExist({ timeout: 15_000 });
  const side = (name: 'a' | 'b') =>
    s.browser.execute(
      (name) => [...document.querySelectorAll(`.git-merge-dialog .cm-merge-${name} .cm-line`)].map((l) => l.textContent).join('\n').trim(),
      name,
    ) as Promise<string>;
  expect(await side('a')).toBe('viejo');
  expect(await side('b')).toBe('viejo cambiado');
  expect(await s.browser.$('.git-merge-dialog [data-note="deleted"]').getText()).toContain('main');
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/git-merge-diff.png`);
  await s.browser.keys(['Escape']);
  await s.browser.$('.git-merge-dialog .git-merge-list').waitForExist({ timeout: 10_000 });
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/git-merge-lista.png`);

  await press(`${row} [data-action="take-theirs"]`);
  await s.browser.$(row).waitForExist({ reverse: true, timeout: 15_000 });

  // Cerrar no cancela: el log lo avisa y permite retomarlo
  await s.browser.keys(['Escape']);
  await s.browser.$('.git-merge-dialog').waitForExist({ reverse: true, timeout: 10_000 });
  await openLog();
  await press('.git-repo-log-dialog [data-action="resolve-merge"]');
  await s.browser.$('.git-merge-dialog .git-merge-list').waitForExist({ timeout: 15_000 });

  // Con el teclado: la lista tiene el foco y Enter abre el editor
  await until(() => s.browser.execute(() => document.activeElement?.classList.contains('git-merge-list') ?? false) as Promise<boolean>, 'la lista no tiene el foco');
  await s.browser.keys(['Enter']);
  await s.browser.$('.git-merge-dialog .merge-pane.result .cm-content').waitForExist({ timeout: 15_000 });
  await until(async () => (await counter()) === '2/1', 'el contador no empieza en 2 cambios y 1 conflicto');
  expect(await paneText('result')).toContain('uno\ndos\ntres\ncuatro');
  // Al desplazar un panel le siguen los otros dos, y F7 vuelve a un cambio pendiente
  const scrolls = () =>
    s.browser.execute(() =>
      [...document.querySelectorAll('.git-merge-dialog .merge-pane .cm-scroller')].map((e) => Math.round(e.scrollTop)),
    ) as Promise<number[]>;
  await s.browser.execute(() => {
    const scroller = document.querySelector('.git-merge-dialog .merge-pane.result .cm-scroller')!;
    scroller.dispatchEvent(new WheelEvent('wheel', { bubbles: true }));
    scroller.scrollTop = 700;
  });
  await until(async () => (await scrolls()).every((top) => Math.abs(top - 700) < 40), 'los paneles no se desplazan juntos');
  await s.browser.keys(['F7']);
  await until(async () => (await scrolls()).every((top) => top < 100), 'F7 no volvió a un cambio');
  expect(await paneText('ours')).toContain('dos mío');
  expect(await paneText('theirs')).toContain('seis suyo');
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/git-merge-editor.png`);

  await press('.git-merge-dialog [data-action="apply-all"]');
  await until(async () => (await counter()) === '0/1', 'no se aplicaron los cambios sin conflicto');
  expect(await paneText('result')).toContain('cuatro mío\ncinco\nseis suyo');

  // Conflicto: primero lo mío y después lo suyo; deshacer quita lo último y el bloque vuelve a pedirlo
  await blockAction('ours', 0, 'apply');
  await blockAction('theirs', 0, 'apply');
  await until(async () => (await counter()) === '0/0', 'el conflicto sigue pendiente');
  expect(await paneText('result')).toContain('dos mío\ndos suyo\ntres');
  await press('.git-merge-dialog [data-action="undo"]');
  await until(async () => (await counter()) === '0/1', 'deshacer no devolvió el conflicto');
  expect(await paneText('result')).not.toContain('dos suyo');
  await blockAction('theirs', 0, 'ignore');
  await until(async () => (await counter()) === '0/0', 'descartar no resolvió el bloque');

  // Corrección a mano en el resultado
  // Se coloca el cursor con el ratón en la última línea con texto («siete»)
  const content = await s.browser.$('.git-merge-dialog .merge-pane.result .cm-content');
  await content.$('.cm-line*=siete').click();
  await s.browser.keys(['End']);
  await s.browser.keys(' fin'.split(''));
  await until(async () => (await paneText('result')).includes('siete fin'), 'no se pudo editar el resultado');

  // Escape con cambios hechos pregunta antes de salir, sin cerrar el diálogo
  await s.browser.keys(['Escape']);
  await s.browser.$('.git-merge-dialog [data-action="confirm-merge-editor"]').waitForExist({ timeout: 10_000 });
  await press('.git-merge-dialog .merge-footer button:first-of-type');
  expect(await paneText('result')).toContain('siete fin');

  await press('.git-merge-dialog [data-action="apply-result"]');
  await s.browser.$('.git-merge-dialog [data-action="commit-merge"]').waitForEnabled({ timeout: 15_000 });
  const expected = lines('uno', 'dos mío', 'tres', 'cuatro mío', 'cinco', 'seis suyo', 'siete fin', ...filler);
  expect(read('nota.md')).toBe(expected);
  if (process.env.E2E_SHOTS) await s.browser.saveScreenshot(`${process.env.E2E_SHOTS}/git-merge-listo.png`);

  await press('.git-merge-dialog [data-action="commit-merge"]');
  await until(() => !merging() && git('rev-list', '--parents', '-1', 'HEAD').split(' ').length === 3, 'no hay commit de merge');
  expect(git('log', '-1', '--format=%s')).toBe("Merge branch 'otra'");
  expect(read('viejo.md')).toBe('viejo cambiado\n');
  expect(git('status', '--porcelain')).toBe('');

  // El log vuelve a abrirse y ofrece deshacerlo
  await press('.git-repo-log-dialog [data-action="undo-history"]');
  await until(() => git('rev-parse', 'HEAD') === before, 'no se deshizo el merge');
  expect(read('nota.md')).toBe(lines(...ours));
}, 90_000);
