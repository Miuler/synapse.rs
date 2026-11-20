// «Ver archivo» de una versión de un OpenAPI en el historial de Git: se ve con Scalar, como en Lectura,
// y el botón Raw muestra el YAML.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;

const spec = (title: string) => `openapi: 3.0.3
info:
  title: ${title}
  version: 1.0.0
paths:
  /mascotas:
    get:
      summary: Listar mascotas
      responses:
        '200':
          description: OK
`;

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-openapi-rev-'));
  const vault = join(base, 'vault');
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  const git = (...args: string[]) =>
    execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', ...args], { cwd: vault });
  git('init', '-q');
  writeFileSync(join(vault, 'api.yaml'), spec('API Antigua'));
  git('add', '.');
  git('commit', '-qm', 'primera');
  writeFileSync(join(vault, 'api.yaml'), spec('API Nueva'));
  git('commit', '-qam', 'segunda');
  s = await startAppOnVault(vault);
  await s.browser.$('.file-tree-item[data-path="api.yaml"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

const paneText = () =>
  s.browser.execute(() => (document.querySelector('.tab-pane:not(.hidden)') as HTMLElement | null)?.innerText ?? '') as Promise<string>;

it('la versión antigua de un OpenAPI se ve con Scalar y con Raw como texto', async () => {
  await s.browser.execute(() => {
    const el = document.querySelector('.file-tree-item[data-path="api.yaml"]')!;
    const { x, y } = el.getBoundingClientRect();
    el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: x + 5, clientY: y + 5 }));
  });
  await s.browser.$('.context-menu-item').waitForExist({ timeout: 5_000 });
  await s.browser.execute(() => {
    ([...document.querySelectorAll('.context-menu-item')].find((i) => i.textContent?.includes('Historial de Git')) as HTMLElement).click();
  });
  await s.browser.waitUntil(async () => (await s.browser.$$('.git-log-dialog .git-log-row').length) === 2, { timeout: 10_000 });
  const rows = await s.browser.$$('.git-log-dialog .git-log-row');
  await rows[1].$('.view-btn').click();

  await s.browser.$('.tab-pane:not(.hidden) .revision-viewer-wrapper .scalar-app').waitForExist({ timeout: 20_000, timeoutMsg: 'Scalar no se montó en la versión' });
  await s.browser.waitUntil(async () => (await paneText()).includes('API Antigua'), { timeout: 20_000 });
  expect(await paneText()).toContain('Listar mascotas');

  await s.browser.execute(() => {
    ([...document.querySelectorAll('.tab-pane:not(.hidden) .toggle-btn')].find((b) => b.textContent?.includes('Raw')) as HTMLElement).click();
  });
  await s.browser.$('.tab-pane:not(.hidden) .revision-code-pre').waitForExist({ timeout: 5_000 });
  expect(await s.browser.$('.tab-pane:not(.hidden) .revision-code-pre').getText()).toContain('openapi: 3.0.3');
  expect(await s.browser.$('.tab-pane:not(.hidden) .scalar-app').isExisting()).toBe(false);
}, 60_000);
