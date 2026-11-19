// `$ref` en YAML: Ctrl+Enter o Ctrl+clic abren el archivo referido y saltan a la clave del puntero `#/...`;
// los punteros internos saltan dentro del mismo archivo; una referencia rota avisa.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session, paneReady } from './harness';

const MAIN = [
  'info:', //                                         1
  '  title: Prueba', //                               2
  'paths:', //                                        3
  '  /transfers:', //                                 4
  '    $ref: ./paths/transfers.yaml#/post/responses', // 5
  '  /otra:', //                                      6
  "    $ref: '#/components/schemas/Error'", //        7
  '  /rota:', //                                      8
  '    $ref: ./paths/no-existe.yaml', //              9
  'components:', //                                   10
  '  schemas:', //                                    11
  '    Error:', //                                    12
  '      type: object', //                            13
  '',
].join('\n');
const TRANSFERS = ['get:', '  summary: listar', 'post:', '  summary: crear', '  responses:', "    '201':", '      description: creado', ''].join('\n');

let s: Session;
beforeAll(async () => {
  s = await startApp({ 'main.yaml': MAIN, 'paths/transfers.yaml': TRANSFERS });
  await s.browser.$('.file-tree-item[data-path="main.yaml"]').waitForExist({ timeout: 30_000 });
  await s.browser.$('.file-tree-item[data-path="main.yaml"]').click();
  await paneReady(s);
  await s.browser.waitUntil(async () => (await cursor()) !== null, { timeout: 15_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
});

/** Archivo de la pestaña activa y línea (1-based) del cursor en su editor visible. */
const cursor = () =>
  s.browser.execute(() => {
    const content = [...document.querySelectorAll('.cm-content')].find((c) => c.getClientRects().length > 0) as any;
    if (!content) return null;
    const view = content.cmView?.view ?? content.cmTile?.view ?? content.parentElement?.parentElement?.cmView?.view;
    if (!view) return null;
    const tab = document.querySelector('.tab-title-container.active')?.getAttribute('data-path');
    return { tab, line: view.state.doc.lineAt(view.state.selection.main.head).number };
  }) as Promise<{ tab: string; line: number } | null>;

/** Pone el cursor dentro del valor del `$ref` de la línea `n`. */
const cursorOnLine = (n: number) =>
  s.browser.execute((n) => {
    const content = [...document.querySelectorAll('.cm-content')].find((c) => c.getClientRects().length > 0) as any;
    const view = content.cmView?.view ?? content.cmTile?.view ?? content.parentElement.parentElement.cmView?.view;
    const line = view.state.doc.line(n);
    view.dispatch({ selection: { anchor: line.to - 2 } });
    view.focus();
  }, n);

it('los $ref se subrayan como enlaces', async () => {
  const refs = (await s.browser.execute(() => [...document.querySelectorAll('.cm-json-ref')].map((e) => e.textContent))) as string[];
  expect(refs).toEqual(['./paths/transfers.yaml#/post/responses', '#/components/schemas/Error', './paths/no-existe.yaml']);
});

it('Ctrl+Enter en un puntero interno salta a la clave en el mismo archivo', async () => {
  await cursorOnLine(7);
  await s.browser.keys(['Control', 'Enter']);
  await s.browser.waitUntil(async () => (await cursor())?.line === 12, { timeout: 15_000 });
  expect((await cursor())!.tab).toBe('main.yaml');
});

it('una referencia rota avisa y no abre nada', async () => {
  await cursorOnLine(9);
  await s.browser.keys(['Control', 'Enter']);
  await s.browser.waitUntil(
    async () => ((await s.browser.execute(() => document.querySelector('.workspace-toast')?.textContent ?? '')) as string).includes('no-existe'),
    { timeout: 15_000 },
  );
  expect((await cursor())!.tab).toBe('main.yaml');
});

it('Ctrl+clic abre el archivo referido y salta a la clave del puntero', async () => {
  await s.browser.execute(() => {
    const el = [...document.querySelectorAll('.cm-json-ref')].find((e) => e.textContent?.startsWith('./paths/transfers')) as HTMLElement;
    const r = el.getBoundingClientRect();
    el.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true, button: 0, ctrlKey: true, clientX: r.left + 20, clientY: r.top + r.height / 2 }));
  });
  await s.browser.waitUntil(async () => {
    const c = await cursor();
    return c?.tab === 'paths/transfers.yaml' && c.line === 5;
  }, { timeout: 15_000, timeoutMsg: 'no abrió paths/transfers.yaml en «responses» (línea 5)' });
});
