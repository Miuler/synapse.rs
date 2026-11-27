// Un enlace de un index.md abre el destino en su pestaña: Atrás vuelve al índice.
import { afterAll, beforeAll, it } from 'bun:test';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { Key } from 'webdriverio';
import { startApp, type Session } from './harness';

let s: Session;

beforeAll(async () => {
  process.env.SYNAPSE_OKF_INDEX = '1';
  try {
    s = await startApp({ 'proyecto/plan.md': '# Plan\n', 'nota.md': '# Nota\n' });
  } finally {
    delete process.env.SYNAPSE_OKF_INDEX;
  }
  await s.browser.$('.file-tree-item.folder[data-path="proyecto"]').waitForExist({ timeout: 30_000 });
  const index = join(s.vault, 'proyecto/index.md');
  await s.browser.waitUntil(async () => existsSync(index) && readFileSync(index, 'utf8').includes('plan.md'), {
    timeout: 20_000,
    timeoutMsg: 'no se generó proyecto/index.md',
  });
}, 120_000);
afterAll(async () => {
  await s?.close();
});

const active = () =>
  s.browser.execute(() => document.querySelector('.tab-title-container.active')?.getAttribute('data-path') ?? '') as Promise<string>;

async function waitForTab(path: string, step = '') {
  try {
    await s.browser.waitUntil(async () => (await active()) === path, { timeout: 10_000 });
  } catch {
    const tabs = await s.browser.execute(() => [...document.querySelectorAll('.tab-title-container')].map((t) => t.getAttribute('data-path')).join(', '));
    throw new Error(`${step}: se esperaba ${path}; activa «${await active()}»; pestañas: ${tabs}`);
  }
}

const planLink = () =>
  [...document.querySelectorAll<HTMLElement>('.markdown-reading-wrapper .markdown-body a')].find(
    (a) => a.textContent?.trim() === 'Plan' && a.offsetParent !== null,
  );

async function followPlanLink() {
  await s.browser.waitUntil(() => s.browser.execute(`return !!(${planLink})()`) as Promise<boolean>, {
    timeout: 10_000,
    timeoutMsg: 'el índice no muestra el enlace',
  });
  await s.browser.execute(`(${planLink})().click()`);
  await waitForTab('proyecto/plan.md');
}

async function back() {
  await s.browser.pause(300);
  await s.browser.keys([Key.Alt, Key.ArrowLeft]);
}

it('Atrás vuelve al índice del que se salió', async () => {
  // El índice recién generado tarda un momento en conocerse: se repite el clic
  await s.browser.waitUntil(
    async () => {
      await s.browser.execute(() => (document.querySelector('.file-tree-item.folder[data-path="proyecto"]') as HTMLElement).click());
      await s.browser.pause(300);
      return (await active()) === 'proyecto/index.md';
    },
    { timeout: 15_000, timeoutMsg: 'el clic en la carpeta no abrió su índice' },
  );
  await followPlanLink();
  await back();
  await waitForTab('proyecto/index.md', 'atrás');
}, 60_000);

it('también si el índice era la pestaña restaurada al abrir', async () => {
  await waitForTab('proyecto/index.md');
  // La sesión de pestañas se guarda con un poco de retardo
  await s.browser.pause(2000);
  await s.browser.refresh();
  await waitForTab('proyecto/index.md', 'restaurar');
  await followPlanLink();
  await back();
  await waitForTab('proyecto/index.md', 'atrás tras restaurar');
}, 90_000);

it('y si el índice quedó activo al cerrar otra pestaña', async () => {
  await waitForTab('proyecto/index.md');
  await s.browser.execute(() => (document.querySelector('.file-tree-item[data-path="nota.md"]') as HTMLElement).click());
  await waitForTab('nota.md');
  await s.browser.pause(500);
  await s.browser.keys([Key.Ctrl, 'w']);
  await waitForTab('proyecto/index.md');
  await followPlanLink();
  await back();
  await waitForTab('proyecto/index.md', 'atrás tras cerrar');
}, 60_000);
