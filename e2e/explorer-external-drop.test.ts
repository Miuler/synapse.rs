// Explorador: arrastrar archivos desde otra aplicación los copia, y arrastrar los propios los mueve.
// El gesto es real (una ventana GTK como origen y xdotool como ratón): WebKitGTK oculta los datos a la
// página y un evento simulado no lo reproduciría. Necesita X11 (Xephyr o xvfb), xdotool y PyGObject.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { execFileSync, spawn, spawnSync, type ChildProcess } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startApp, type Session } from './harness';

const GTK = "import gi;gi.require_version('Gdk','3.0');gi.require_version('Gtk','3.0');from gi.repository import Gtk";
// Nunca en el escritorio real: movería el ratón de la persona
const canDrag =
  process.env.GDK_BACKEND === 'x11' &&
  !process.env.WAYLAND_DISPLAY &&
  !!Bun.which('xdotool') &&
  spawnSync('python3', ['-c', GTK]).status === 0;
const dragIt = canDrag ? it : it.skip;

const SOURCE = { x: 850, y: 600 };
let s: Session;
let outside: string;
let source: ChildProcess | undefined;

const xd = (...args: (string | number)[]) => execFileSync('xdotool', args.map(String), { encoding: 'utf8' });

beforeAll(async () => {
  if (!canDrag) return;
  outside = mkdtempSync(join(tmpdir(), 'synapse-e2e-drop-'));
  mkdirSync(join(outside, 'carpeta/sub'), { recursive: true });
  writeFileSync(join(outside, 'informe ñ.txt'), 'desde fuera');
  writeFileSync(join(outside, 'carpeta/sub/x.md'), 'dentro');
  writeFileSync(join(outside, 'raiz.md'), 'raíz');
  writeFileSync(join(outside, 'secreto.md'), 'no soltado');
  s = await startApp({ 'nota.md': '# Nota\n', 'destino/leeme.md': 'x', 'zeta.md': 'zeta' });
  await s.browser.$('.file-tree-item[data-path="destino"]').waitForExist({ timeout: 30_000 });
}, 120_000);
afterAll(async () => {
  source?.kill();
  if (canDrag) {
    xd('mouseup', 1);
    // El puntero real sobre la ventana interfiere con las pruebas siguientes
    xd('mousemove', ...xd('getdisplaygeometry').trim().split(' ').map((n) => Number(n) - 1));
  }
  await s?.close();
  if (outside) rmSync(outside, { recursive: true, force: true });
});

async function point(selector: string, bottom = false): Promise<[number, number]> {
  return (await s.browser.execute(
    (q: string, low: boolean) => {
      const r = document.querySelector(q)!.getBoundingClientRect();
      return [Math.round(r.x + 60), Math.round(low ? r.bottom - 30 : r.y + r.height / 2)];
    },
    selector,
    bottom,
  )) as [number, number];
}

async function drag([x1, y1]: [number, number], [x2, y2]: [number, number]) {
  xd('mousemove', x1, y1);
  await Bun.sleep(200);
  xd('mousedown', 1);
  for (let i = 1; i <= 20; i++) {
    await Bun.sleep(30);
    xd('mousemove', Math.round(x1 + ((x2 - x1) * i) / 20), Math.round(y1 + ((y2 - y1) * i) / 20));
  }
  await Bun.sleep(400);
  xd('mousemove', x2 + 1, y2);
  await Bun.sleep(400);
}

async function dragFromOutside(files: string[], to: [number, number]) {
  source = spawn('python3', ['e2e/fixtures/drag-source.py', String(SOURCE.x), String(SOURCE.y), ...files.map((f) => join(outside, f))], {
    stdio: ['ignore', 'pipe', 'inherit'],
  });
  await new Promise((done) => source!.stdout!.once('data', done));
  await Bun.sleep(300);
  await drag([SOURCE.x + 60, SOURCE.y + 30], to);
}
function drop() {
  xd('mouseup', 1);
  source?.kill();
  source = undefined;
}
const until = (path: string, exists = true) =>
  s.browser.waitUntil(async () => existsSync(join(s.vault, path)) === exists, { timeout: 8_000, timeoutMsg: `${path}: no ${exists ? 'apareció' : 'desapareció'}` });

dragIt('un archivo y una carpeta de fuera se copian en la carpeta donde caen', async () => {
  await dragFromOutside(['informe ñ.txt', 'carpeta'], await point('.file-tree-item[data-path="destino"]'));
  expect(await s.browser.$('.file-tree-item[data-path="destino"].drop-target').isExisting()).toBe(true);
  drop();
  await until('destino/informe ñ.txt');
  await until('destino/carpeta/sub/x.md');
  expect(readFileSync(join(s.vault, 'destino/informe ñ.txt'), 'utf8')).toBe('desde fuera');
  // Se copia: el original sigue
  expect(existsSync(join(outside, 'informe ñ.txt'))).toBe(true);
  expect(await s.browser.$('.drop-target').isExisting()).toBe(false);
});

dragIt('en el espacio vacío se copian a la raíz, sin pisar lo que exista', async () => {
  writeFileSync(join(s.vault, 'raiz.md'), 'ya estaba');
  await dragFromOutside(['raiz.md'], await point('.sidebar-content', true));
  drop();
  await s.browser.waitUntil(async () => existsSync(join(s.vault, 'raiz.copy.md')) , { timeout: 8_000, timeoutMsg: 'no se copió con otro nombre' });
  expect(readFileSync(join(s.vault, 'raiz.md'), 'utf8')).toBe('ya estaba');
});

dragIt('lo no soltado no se puede importar', async () => {
  const importing = (file: string) =>
    s.browser.execute(
      (p: string) =>
        (window as any).__TAURI_INTERNALS__.invoke('import_external_items', { paths: [p], destDir: '' }).then(
          () => 'ok',
          (e: unknown) => String(e),
        ),
      join(outside, file),
    ) as Promise<string>;
  expect(await importing('secreto.md')).toContain('portapapeles');
  expect(existsSync(join(s.vault, 'secreto.md'))).toBe(false);
});

dragIt('arrastrar dentro del explorador mueve', async () => {
  await drag(await point('.file-tree-item[data-path="zeta.md"]'), await point('.file-tree-item[data-path="destino"]'));
  drop();
  await until('destino/zeta.md');
  await until('zeta.md', false);
});
