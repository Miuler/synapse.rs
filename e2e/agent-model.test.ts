// Modelo y esfuerzo anunciados en `configOptions` (Claude): se eligen en el menú y se recuerdan.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { join } from 'node:path';
import { startApp, type Session } from './harness';

let s: Session;
beforeAll(async () => {
  s = await startApp({ 'nota.md': '# Nota\n', 'flujo.mmd': 'graph TD\n  A --> B\n' });
  await s.browser.$('.file-tree-item[data-path="nota.md"]').waitForExist({ timeout: 30_000 });
  await s.browser.$('.editor-header .agent-toggle').click();
  await s.browser.$('.synapse-agent-panel button[title="Ajustes de agentes"]').click();
  await s.browser.$('.agent-form').waitForExist({ timeout: 5_000 });
  await s.browser.$('input[placeholder="Identificador (mi-agente)"]').setValue('falso');
  await s.browser.$('input[placeholder="Nombre"]').setValue('Agente falso');
  await s.browser.$('input[placeholder="Comando (ruta o nombre en el PATH)"]').setValue(process.execPath);
  await s.browser.$('input[placeholder="Argumentos separados por espacios"]').setValue(`${join(import.meta.dir, 'fixtures/fake-acp-agent.ts')} --config`);
  await s.browser.$('.agent-form button[type="submit"]').click();
  await s.browser.waitUntil(() => s.browser.execute(() => [...document.querySelectorAll('.agent-row .row-label')].some((e) => e.textContent?.includes('Agente falso'))), { timeout: 5_000 });
  await s.browser.keys('Escape');
  await s.browser.waitUntil(() => s.browser.execute(() => [...document.querySelectorAll('.acp-select option')].some((e) => e.textContent === 'Agente falso')), { timeout: 5_000 });
  await s.browser.execute(() => {
    const select = document.querySelector('.acp-select') as HTMLSelectElement;
    select.value = 'falso';
    select.dispatchEvent(new Event('change', { bubbles: true }));
  });
  await s.browser.$('.acp-opts-btn').waitForExist({ timeout: 20_000 });
}, 120_000);
afterAll(() => s?.close());

const chip = () => s.browser.execute(() => document.querySelector('.acp-model-chip')?.textContent?.trim() ?? '') as Promise<string>;
const heads = () => s.browser.execute(() => [...document.querySelectorAll('.acp-opts-head')].map((e) => e.textContent)) as Promise<string[]>;
const open = async () => {
  await s.browser.$('.acp-opts-btn').click();
  await s.browser.$('.acp-opts-menu').waitForExist({ timeout: 5_000 });
};
const choose = (label: string) =>
  s.browser.execute((l) => ([...document.querySelectorAll('.acp-opts-menu button')].find((b) => b.textContent?.trim() === l) as HTMLElement).click(), label);
const waitChip = (want: string) =>
  s.browser.waitUntil(async () => (await chip()) === want, { timeout: 5_000, timeoutMsg: `la etiqueta no quedó en «${want}»` });

it('muestra modelo y esfuerzo, y el esfuerzo depende del modelo', async () => {
  await waitChip('Modelo uno');
  await open();
  expect(await heads()).toEqual(['Modelo', 'Esfuerzo', 'Modo']);
  await choose('high');
  await waitChip('Modelo uno · high');
  await open();
  await choose('Modelo dos');
  await waitChip('Modelo dos');
  await open();
  expect(await heads()).toEqual(['Modelo', 'Modo']);
  await choose('Modelo uno');
  await waitChip('Modelo uno · high');
});

it('una conversación nueva usa el modelo y el esfuerzo recordados', async () => {
  await open();
  await choose('low');
  await waitChip('Modelo uno · low');
  await s.browser.$('button[aria-label="Nueva conversación"]').click();
  await s.browser.waitUntil(async () => (await s.browser.execute(() => document.querySelectorAll('.acp-model-chip').length)) === 2, { timeout: 15_000, timeoutMsg: 'no abrió la segunda' });
  await waitChip('Modelo uno · low');
}, 30_000);
