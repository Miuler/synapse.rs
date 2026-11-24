// Al abrir un resultado de la búsqueda global, el buscador del editor queda cargado con el término.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { Key } from 'webdriverio';
import { startApp, type Session, paneReady } from './harness';

let s: Session;

beforeAll(async () => {
  s = await startApp({
    'inicio.md': '# Inicio\n\nNada que ver.\n',
    'notas.md': '# Notas\n\nEl murcielagoazul vuela.\n\nOtro murcielagoazul duerme.\n',
    'datos.yaml': 'animal: zorrovioleta\notro: zorrovioleta\n',
  });
  const item = s.browser.$('.file-tree-item[data-path="inicio.md"]');
  await item.waitForExist({ timeout: 30_000 });
  await item.click();
  await paneReady(s);
}, 120_000);

afterAll(async () => {
  await s?.close();
});

async function openHit(term: string, path: string) {
  await s.browser.keys([Key.Ctrl, Key.Shift, 'f']);
  const input = s.browser.$('input[placeholder*="Ctrl+Shift+F"]');
  await input.waitForExist({ timeout: 5_000 });
  await input.setValue(term);
  const hit = s.browser.$(`.fts-hit-item[data-value*="${path}"]`);
  await hit.waitForExist({ timeout: 30_000, timeoutMsg: 'sin resultado (índice sin terminar)' });
  await s.browser.keys(['Enter']);
}

it('una nota Markdown se abre con el buscador cargado con el término', async () => {
  await openHit('murcielagoazul', 'notas.md');
  await s.browser.waitUntil(
    () =>
      s.browser.execute(() => {
        const v = (document.querySelector('.reading-search-input, .cm-search input[name="search"]') as HTMLInputElement | null)?.value;
        return v === 'murcielagoazul';
      }) as Promise<boolean>,
    { timeout: 10_000, timeoutMsg: 'buscador sin el término' },
  );
}, 90_000);

it('un archivo YAML se abre con el panel de búsqueda cargado con el término', async () => {
  await openHit('zorrovioleta', 'datos.yaml');
  await s.browser.waitUntil(
    () =>
      s.browser.execute(() => {
        const inputs = [...document.querySelectorAll<HTMLInputElement>('.cm-search input[name="search"]')];
        return inputs.some((i) => i.value === 'zorrovioleta');
      }) as Promise<boolean>,
    { timeout: 10_000, timeoutMsg: 'panel sin el término' },
  );
  expect(true).toBe(true);
}, 90_000);
