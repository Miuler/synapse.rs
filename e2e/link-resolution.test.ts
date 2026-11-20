// Regla única de enlaces: tal cual, el único con ese nombre, o (si hay varios) una lista para
// elegir: Enter solo abre, Ctrl+Enter abre y fija el enlace en la nota. Una imagen incrustada
// ambigua avisa en lugar de mostrar una cualquiera.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { startApp, type Session, paneReady } from './harness';

let s: Session;
const PLAN = ['# Plan', '', 'Ver [[informe]] y [el único](unico.md).', '', '![[foto.png]]', ''].join('\n');

beforeAll(async () => {
  s = await startApp({
    'notas/plan.md': PLAN,
    'a/informe.md': '# Informe A\n',
    'a/b/informe.md': '# Informe B\n',
    'otro/unico.md': '# Único\n',
  });
  const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==', 'base64');
  writeFileSync(join(s.vault, 'a/foto.png'), png);
  writeFileSync(join(s.vault, 'otro/foto.png'), png);
  // La nota está dentro de «notas/»: se abre la carpeta en el explorador
  await s.browser.$('.file-tree-item[data-path="notas"]').waitForExist({ timeout: 30_000 });
  await s.browser.execute(() => (document.querySelector('.file-tree-item[data-path="notas"]') as HTMLElement).click());
  await s.browser.$('.file-tree-item[data-path="notas/plan.md"]').waitForExist({ timeout: 10_000 });
  await s.browser.execute(() => (document.querySelector('.file-tree-item[data-path="notas/plan.md"]') as HTMLElement).click());
  await paneReady(s);
  await s.browser.$('.markdown-reading-wrapper .markdown-body a').waitForExist({ timeout: 15_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
});

const clickLink = (text: string) =>
  s.browser.execute((text) => {
    const a = [...document.querySelectorAll('.markdown-reading-wrapper .markdown-body a')].find((x) => x.textContent?.trim() === text) as HTMLElement;
    a.click();
  }, text);
const activeTab = () =>
  s.browser.execute(() => document.querySelector('.tab-title-container.active, .tab-title-container[aria-selected="true"]')?.getAttribute('data-path') ?? '') as Promise<string>;
const candidates = () =>
  s.browser.execute(() => [...document.querySelectorAll('.link-chooser-item')].map((e) => e.getAttribute('data-path'))) as Promise<string[]>;
const plan = () => readFileSync(join(s.vault, 'notas/plan.md'), 'utf8');

it('un enlace con un solo archivo de ese nombre se abre directamente', async () => {
  await clickLink('el único');
  await s.browser.waitUntil(async () => (await activeTab()) === 'otro/unico.md', { timeout: 10_000, timeoutMsg: 'no abrió otro/unico.md' });
  expect(await s.browser.$('.link-chooser').isExisting()).toBe(false);
  await s.browser.execute(() => (document.querySelector('.file-tree-item[data-path="notas/plan.md"]') as HTMLElement).click());
  await s.browser.$('.markdown-reading-wrapper .markdown-body a').waitForExist({ timeout: 10_000 });
}, 30_000);

it('wikilink ambiguo: lista para elegir; Enter solo abre y no toca la nota', async () => {
  await clickLink('informe');
  await s.browser.$('.link-chooser').waitForExist({ timeout: 10_000, timeoutMsg: 'no apareció la lista para elegir' });
  expect(await candidates()).toEqual(['a/b/informe.md', 'a/informe.md']);
  await s.browser.keys('ArrowDown');
  await s.browser.keys('Enter');
  await s.browser.waitUntil(async () => (await activeTab()) === 'a/informe.md', { timeout: 10_000, timeoutMsg: 'Enter no abrió el elegido' });
  expect(await s.browser.$('.link-chooser').isExisting()).toBe(false);
  expect(plan()).toBe(PLAN);
  await s.browser.execute(() => (document.querySelector('.file-tree-item[data-path="notas/plan.md"]') as HTMLElement).click());
  await s.browser.$('.markdown-reading-wrapper .markdown-body a').waitForExist({ timeout: 10_000 });
}, 30_000);

it('Ctrl+Enter abre y fija el enlace en la nota', async () => {
  await clickLink('informe');
  await s.browser.$('.link-chooser').waitForExist({ timeout: 10_000 });
  await s.browser.keys(['Control', 'Enter']);
  await s.browser.waitUntil(async () => (await activeTab()) === 'a/b/informe.md', { timeout: 10_000 });
  await s.browser.waitUntil(async () => plan().includes('[[a/b/informe|informe]]'), { timeout: 10_000, timeoutMsg: 'no se fijó el enlace en la nota' });
  // Solo cambia el enlace: guardar no añade frontmatter ni `title`
  expect(plan()).toBe(PLAN.replace('[[informe]]', '[[a/b/informe|informe]]'));
}, 30_000);

it('una imagen incrustada ambigua avisa y ofrece elegir', async () => {
  await s.browser.execute(() => (document.querySelector('.file-tree-item[data-path="notas/plan.md"]') as HTMLElement).click());
  await s.browser.$('.reading-link-ambiguous').waitForExist({ timeout: 15_000, timeoutMsg: 'no apareció el aviso de imagen ambigua' });
  expect(await s.browser.execute(() => document.querySelector('.reading-link-ambiguous')?.textContent)).toContain('Hay 2 archivos «foto.png»');
  await s.browser.execute(() => (document.querySelector('.reading-link-ambiguous button') as HTMLElement).click());
  await s.browser.$('.link-chooser').waitForExist({ timeout: 10_000 });
  expect(await candidates()).toEqual(['a/foto.png', 'otro/foto.png']);
  await s.browser.keys('Escape');
  await s.browser.$('.link-chooser').waitForExist({ timeout: 5_000, reverse: true });
}, 30_000);
