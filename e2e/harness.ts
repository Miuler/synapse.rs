// Arnés de pruebas de interfaz (WebDriver) para la app de escritorio real.
//
// Aísla todo en un directorio temporal: HOME propio (configuración, recientes y caché del webview)
// y una bóveda de ejemplo. Nunca toca la bóveda ni la configuración del usuario.
//
// Requisitos (solo desarrollo): `cargo install tauri-driver --locked`, `WebKitWebDriver`
// (paquete webkitgtk) y `cargo build` en src-tauri (binario en modo dev, que carga Vite).
import { setDefaultTimeout } from 'bun:test';
import { spawn, spawnSync, type ChildProcess } from 'child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'fs';
import { tmpdir } from 'os';
import { dirname, join, resolve } from 'path';
import { remote, type Browser } from 'webdriverio';

// Las acciones sobre la app real tardan más que el límite por defecto de bun test (5 s)
setDefaultTimeout(60_000);

const ROOT = resolve(import.meta.dir, '..');
// E2E_APP permite usar otro binario, p. ej. el de producción (target/release/synapsers), que ya
// trae el front incluido y no necesita Vite.
const APP = process.env.E2E_APP ?? join(ROOT, 'src-tauri/target/debug/synapsers');
const NEEDS_VITE = !process.env.E2E_APP;
const DRIVER = join(process.env.HOME ?? '', '.cargo/bin/tauri-driver');

export interface Session {
  browser: Browser;
  vault: string;
  home: string;
  /** Texto reenviado por Vite desde la consola del webview (vacío si Vite ya estaba corriendo). */
  consoleLog(): Promise<string>;
  close(): Promise<void>;
}

async function waitForPort(port: number, timeoutMs: number): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      await fetch(`http://127.0.0.1:${port}`);
      return;
    } catch {
      await Bun.sleep(200);
    }
  }
  throw new Error(`El puerto ${port} no respondió en ${timeoutMs} ms`);
}

async function isListening(port: number): Promise<boolean> {
  try {
    await fetch(`http://localhost:${port}`);
    return true;
  } catch {
    return false;
  }
}

/** Escribe la bóveda de ejemplo: { ruta relativa: contenido }. */
export function writeVault(vault: string, files: Record<string, string>): void {
  for (const [rel, content] of Object.entries(files)) {
    const path = join(vault, rel);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, content);
  }
  mkdirSync(join(vault, '.synapse'), { recursive: true });
}

export async function startApp(files: Record<string, string>): Promise<Session> {
  const base = mkdtempSync(join(tmpdir(), 'synapse-e2e-'));
  const vault = join(base, 'vault');
  writeVault(vault, files);
  return launch(base, vault);
}

/**
 * Abre una bóveda existente (p. ej. la real del usuario) con un HOME temporal. No escribe archivos
 * de prueba ni borra nada de la bóveda al cerrar; la app sí actualiza su `.synapse/` como en uso
 * normal.
 */
export async function startAppOnVault(vault: string): Promise<Session> {
  const base = mkdtempSync(join(tmpdir(), 'synapse-e2e-'));
  return launch(base, vault);
}

async function launch(base: string, vault: string): Promise<Session> {
  const home = join(base, 'home');
  mkdirSync(home, { recursive: true });

  const children: ChildProcess[] = [];
  // Vite reenvía al terminal los avisos de la consola del webview (p. ej. de Svelte): se guardan
  // en vite.log para que las pruebas puedan revisarlos.
  const viteLog = join(base, 'vite.log');
  if (NEEDS_VITE && !(await isListening(5173))) {
    const out = Bun.file(viteLog).writer();
    const vite = spawn('bun', ['x', 'vite', '--port', '5173', '--strictPort'], { cwd: ROOT, stdio: ['ignore', 'pipe', 'pipe'] });
    vite.stdout?.on('data', (d) => out.write(d));
    vite.stderr?.on('data', (d) => out.write(d));
    children.push(vite);
  }
  if (NEEDS_VITE) {
    await waitForPort(5173, 30_000).catch(async () => {
      // vite escucha en ::1 (localhost)
      if (!(await isListening(5173))) throw new Error('Vite no arrancó');
    });
  }

  const env = { ...process.env, HOME: home };
  const driver = spawn(DRIVER, ['--port', '4444'], { env, stdio: ['ignore', 'inherit', 'inherit'] });
  children.push(driver);
  await waitForPort(4444, 15_000);

  const browser = await remote({
    hostname: '127.0.0.1',
    port: 4444,
    logLevel: 'warn',
    capabilities: {
      // @ts-expect-error capacidad propia de tauri-driver
      'tauri:options': { application: APP, args: ['--vault', vault] },
    },
  });

  return {
    browser,
    vault,
    home,
    async consoleLog() {
      const f = Bun.file(viteLog);
      return (await f.exists()) ? await f.text() : '';
    },
    async close() {
      await browser.deleteSession().catch(() => {});
      for (const c of children.reverse()) c.kill();
      // Si la sesión se cortó, la app puede quedar abierta: cerrar la de esta bóveda de prueba
      spawnSync('pkill', ['-f', `synapsers --vault ${vault}`]);
      rmSync(base, { recursive: true, force: true });
    },
  };
}
