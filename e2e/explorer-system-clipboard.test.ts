// Explorador: «Copiar» deja los archivos en el portapapeles del sistema (como un explorador de
// archivos) y «Pegar» trae lo copiado fuera de la instancia (otra instancia, el gestor de archivos…).
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnVault, type Session } from './harness';

let s: Session;
let base: string;
let vault: string;
let outside: string;

beforeAll(async () => {
  base = mkdtempSync(join(tmpdir(), 'synapse-e2e-sysclip-'));
  vault = join(base, 'vault');
  outside = join(base, 'fuera');
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  mkdirSync(join(vault, 'destino'));
  mkdirSync(outside);
  writeFileSync(join(vault, 'nota.md'), '# Nota\n');
  writeFileSync(join(vault, 'destino', 'leeme.md'), 'x\n');
  writeFileSync(join(outside, 'informe con espacios.md'), 'desde fuera\n');
  s = await startAppOnVault(vault);
  await s.browser.$('.file-tree-item[data-path="nota.md"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(base, { recursive: true, force: true });
});

const invoke = (cmd: string, args?: Record<string, unknown>) =>
  s.browser.execute(
    (c: string, a: Record<string, unknown> | undefined) => (window as any).__TAURI_INTERNALS__.invoke(c, a ?? {}),
    cmd,
    args,
  );

async function menuOn(path: string, item: RegExp) {
  await s.browser.execute((p: string) => {
    const el = document.querySelector(`.file-tree-item[data-path="${p}"]`)!;
    const { x, y } = el.getBoundingClientRect();
    el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: x + 5, clientY: y + 5 }));
  }, path);
  await s.browser.$('.context-menu-item').waitForExist({ timeout: 5_000 });
  // El menú consulta el portapapeles del sistema al abrirse
  await s.browser.pause(300);
  return s.browser.execute((src: string) => {
    const re = new RegExp(src);
    const el = [...document.querySelectorAll('.context-menu-item')].find((i) => re.test(i.textContent?.trim() ?? ''));
    if (!el) return null;
    const label = el.textContent?.trim() ?? '';
    (el as HTMLElement).click();
    return label;
  }, item.source);
}

it('«Copiar» pone el archivo en el portapapeles del sistema', async () => {
  expect(await menuOn('nota.md', /^Copiar( Ctrl|$)/)).toMatch(/^Copiar/);
  await s.browser.waitUntil(async () => ((await invoke('read_system_clipboard_files')) as string[]).length > 0, {
    timeout: 5_000,
    timeoutMsg: 'el portapapeles del sistema no tiene el archivo',
  });
  expect(await invoke('read_system_clipboard_files')).toEqual([join(vault, 'nota.md')]);
});

it('«Pegar» trae los archivos copiados fuera de la instancia', async () => {
  // Como si se hubiera copiado en otra instancia o en el gestor de archivos
  await invoke('copy_files_to_system_clipboard', { paths: [join(outside, 'informe con espacios.md')] });
  const label = await menuOn('destino', /^Pegar/);
  expect(label).toContain('Pegar (1 archivo de fuera)');
  await s.browser.waitUntil(async () => existsSync(join(vault, 'destino', 'informe con espacios.md')), {
    timeout: 5_000,
    timeoutMsg: 'no se pegó el archivo de fuera',
  });
  expect(readFileSync(join(vault, 'destino', 'informe con espacios.md'), 'utf8')).toBe('desde fuera\n');
});
