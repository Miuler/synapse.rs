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
// Las e2e usan el mismo binario de desarrollo (`target/debug`, el de `cargo build` o `tauri dev`), que carga
// la interfaz desde el 5173: así no hace falta una segunda compilación completa. Si el 5173 ya está ocupado,
// se reutiliza solo si es el Vite de Synapse (puede ser otro proyecto).
const VITE_PORT = 5173;
const APP = process.env.E2E_APP ?? join(ROOT, 'src-tauri/target/debug/synapsers');

/** Lo que responde en el puerto es el Vite de este proyecto (y no el de otro). */
async function servesSynapse(port: number): Promise<boolean> {
  try {
    const html = await (await fetch(`http://localhost:${port}/`)).text();
    return html.includes('<title>synapse.rs</title>');
  } catch {
    return false;
  }
}
const NEEDS_VITE = !process.env.E2E_APP;
const DRIVER = join(process.env.HOME ?? '', '.cargo/bin/tauri-driver');
// Puertos de tauri-driver y de WebKitWebDriver: E2E_DRIVER_PORT permite correr varios procesos de
// pruebas a la vez (cada uno con su par, p. ej. 4444/4445, 4446/4447…) sin pisarse.
const DRIVER_PORT = Number(process.env.E2E_DRIVER_PORT ?? 4444);
const NATIVE_PORT = DRIVER_PORT + 1;

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
    // `localhost` y no 127.0.0.1: Vite escucha solo en ::1, y con 127.0.0.1 se agotaba la espera
    // entera (30 s por archivo de pruebas) antes de comprobar localhost
    if (await isListening(port)) return;
    await Bun.sleep(200);
  }
  throw new Error(`El puerto ${port} no respondió en ${timeoutMs} ms`);
}

/** Espera a que nadie escuche en `port` (el WebDriver del archivo anterior tarda en cerrarse). */
async function waitForPortFree(port: number, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (await isListening(port)) {
    if (Date.now() > deadline) throw new Error(`El puerto ${port} sigue ocupado (¿quedó un WebDriver de otra corrida?)`);
    await Bun.sleep(100);
  }
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

/** Escala de texto de GNOME del usuario (1 si no hay GNOME o no se puede leer). */
function gnomeTextScale(): number {
  try {
    const out = Bun.spawnSync(['gsettings', 'get', 'org.gnome.desktop.interface', 'text-scaling-factor']).stdout.toString();
    const n = Number.parseFloat(out);
    return Number.isFinite(n) && n > 0 ? n : 1;
  } catch {
    return 1;
  }
}

/** Errores que significan que la sesión de WebDriver ya no existe. */
const DEAD_SESSION = /ECONNREFUSED|ECONNRESET|socket hang up|invalid session id|session deleted|Failed to fetch|fetch failed/i;

/**
 * Envuelve el navegador: si la sesión murió (el WebDriver terminó o una llamada falló por conexión),
 * cualquier uso posterior falla enseguida con el motivo, sin esperar el timeout de cada prueba.
 */
function guardBrowser(
  browser: Browser,
  deadReason: () => string | null,
  markDead: (why: string) => void,
): Browser {
  return new Proxy(browser, {
    get(target, prop, receiver) {
      const why = deadReason();
      if (why && prop !== 'deleteSession' && prop !== 'then') {
        throw new Error(`Sesión de WebDriver perdida (${why}); se omiten las pruebas que quedan.`);
      }
      const value = Reflect.get(target, prop, receiver);
      if (typeof value !== 'function') return value;
      return (...args: unknown[]) => {
        const out = value.apply(target, args);
        if (out && typeof (out as Promise<unknown>).catch === 'function' && prop !== '$' && prop !== '$$') {
          return (out as Promise<unknown>).catch((e: unknown) => {
            const msg = e instanceof Error ? e.message : String(e);
            if (DEAD_SESSION.test(msg)) markDead(msg.split('\n')[0]);
            throw e;
          });
        }
        return out;
      };
    },
  });
}

/** Vite arrancado por este proceso (null si ya corría o no hace falta) y a qué log escribe. */
let ownVite: ChildProcess | null = null;
let viteOut: ReturnType<ReturnType<typeof Bun.file>['writer']> | null = null;

async function launch(base: string, vault: string): Promise<Session> {
  const home = join(base, 'home');
  mkdirSync(home, { recursive: true });

  const children: ChildProcess[] = [];
  // Vite reenvía al terminal los avisos de la consola del webview (p. ej. de Svelte): se guardan
  // en vite.log para que las pruebas puedan revisarlos.
  const viteLog = join(base, 'vite.log');
  if (NEEDS_VITE && (await isListening(VITE_PORT)) && !(await servesSynapse(VITE_PORT))) {
    throw new Error(`El puerto ${VITE_PORT} lo ocupa otro proyecto, no el Vite de Synapse: ciérralo para correr las e2e.`);
  }
  if (NEEDS_VITE && !(await isListening(VITE_PORT))) {
    // Vite se arranca una vez por proceso y sigue vivo entre archivos (`bun test ./e2e` los corre en
    // el mismo proceso): reiniciarlo en cada uno costaba segundos. Se cierra al salir.
    // Un vigilante cierra el grupo de Vite cuando termina este proceso: bun test no emite `exit`
    // de forma fiable, y `bun x` lanza node como hijo (matar solo a `bun` dejaba Vite escuchando).
    const watch = `bun x vite --port ${VITE_PORT} --strictPort & while kill -0 ${process.pid} 2>/dev/null; do sleep 1; done; kill -- -$$`;
    const vite = spawn('sh', ['-c', watch], { cwd: ROOT, stdio: ['ignore', 'pipe', 'pipe'], detached: true });
    vite.stdout?.on('data', (d) => viteOut?.write(d));
    vite.stderr?.on('data', (d) => viteOut?.write(d));
    ownVite = vite;
  }
  if (ownVite) viteOut = Bun.file(viteLog).writer();
  if (NEEDS_VITE) {
    await waitForPort(VITE_PORT, 30_000).catch(() => {
      throw new Error('Vite no arrancó');
    });
  }

  // Con XDG_* definidas en el entorno, cambiar HOME no basta: la app usaría el perfil real del usuario.
  const env = {
    ...process.env,
    HOME: home,
    XDG_DATA_HOME: join(home, '.local/share'),
    XDG_CONFIG_HOME: join(home, '.config'),
    XDG_CACHE_HOME: join(home, '.cache'),
    XDG_STATE_HOME: join(home, '.local/state'),
    // Con la escala de texto de GNOME (p. ej. 1,25), WebKit agranda la página y WebKitWebDriver
    // hace clic en coordenadas sin escalar (cae en otro elemento): se compensa a escala 1.
    GDK_DPI_SCALE: process.env.E2E_GDK_DPI_SCALE ?? String(1 / gnomeTextScale()),
  };
  // Si el puerto sigue tomado por el WebDriver anterior, conectarse a él fallaría a mitad de la prueba
  await waitForPortFree(DRIVER_PORT, 10_000);
  // Grupo propio: tauri-driver lanza WebKitWebDriver como hijo, y si tauri-driver muere ese hijo
  // quedaba huérfano (con el puerto 4445 y la salida del proceso de pruebas abiertos)
  const driver = spawn(DRIVER, ['--port', String(DRIVER_PORT), '--native-port', String(NATIVE_PORT)], { env, stdio: ['ignore', 'inherit', 'inherit'], detached: true });
  children.push(driver);
  // Si el WebDriver se cae, las pruebas que quedan fallan al instante en vez de esperar su timeout
  let dead: string | null = null;
  driver.on('exit', (code) => (dead ??= `el WebDriver terminó (código ${code})`));
  await waitForPort(DRIVER_PORT, 15_000);

  const raw = await remote({
    hostname: '127.0.0.1',
    port: DRIVER_PORT,
    logLevel: 'warn',
    // Sin reintentos: con la conexión rota, cada pedido reintentaba 3 veces con 2 min de espera
    connectionRetryCount: 0,
    connectionRetryTimeout: 30_000,
    capabilities: {
      // @ts-expect-error capacidad propia de tauri-driver
      'tauri:options': { application: APP, args: ['--vault', vault] },
    },
  });

  const browser = guardBrowser(raw, () => dead, (why) => (dead ??= why));

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
      try {
        process.kill(-driver.pid!, 'SIGTERM');
      } catch {}
      await waitForPortFree(DRIVER_PORT, 10_000).catch(() => {});
      // Si la sesión se cortó, la app puede quedar abierta: cerrar la de esta bóveda de prueba
      spawnSync('pkill', ['-f', `synapsers --vault ${vault}`]);
      // El log de Vite de este archivo termina aquí; el siguiente abre el suyo
      await viteOut?.end();
      viteOut = null;
      rmSync(base, { recursive: true, force: true });
    },
  };
}

/**
 * Espera a que la pestaña visible termine de montar su visor (editor de código, diagrama o
 * lectura): en lugar de una pausa fija antes de decidir si hace falta F2 para ver el editor.
 */
export async function paneReady(s: Session, timeout = 10_000): Promise<void> {
  await s.browser.waitUntil(
    () =>
      s.browser.execute(() => {
        const pane = document.querySelector('.tab-pane:not(.hidden)');
        return !!pane?.querySelector('.diagram-viewer-container, .cm-content, .markdown-body');
      }) as Promise<boolean>,
    { timeout, timeoutMsg: 'la pestaña no terminó de mostrar su contenido' },
  );
}
