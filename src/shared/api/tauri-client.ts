/**
 * Cliente genérico de invocación IPC para Tauri 2.0.
 * Provee tipado estricto, manejo seguro de errores y compatibilidad de entorno.
 */

/** Registro de llamadas IPC, solo en desarrollo: lo leen las pruebas de rendimiento (e2e). */
export interface IpcLogEntry {
  cmd: string;
  ms: number;
}

export async function invokeTauri<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  // En producción solo se registra si el build se hizo con VITE_SYNAPSE_IPC_LOG=1 (mediciones).
  if (!import.meta.env.DEV && import.meta.env.VITE_SYNAPSE_IPC_LOG !== '1') return invokeTauriRaw<T>(cmd, args);
  const start = performance.now();
  try {
    return await invokeTauriRaw<T>(cmd, args);
  } finally {
    const log = ((window as unknown as { __synapseIpcLog?: IpcLogEntry[] }).__synapseIpcLog ??= []);
    log.push({ cmd, ms: performance.now() - start });
  }
}

async function invokeTauriRaw<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (typeof window === 'undefined') {
    throw new Error('Entorno no es un navegador');
  }

  const w = window as unknown as {
    __TAURI_INTERNALS__?: { invoke?: (cmd: string, args?: Record<string, unknown>) => Promise<T> };
    __TAURI__?: { core?: { invoke?: (cmd: string, args?: Record<string, unknown>) => Promise<T> } };
  };

  if (w.__TAURI_INTERNALS__?.invoke) {
    return await w.__TAURI_INTERNALS__.invoke(cmd, args);
  }

  if (w.__TAURI__?.core?.invoke) {
    return await w.__TAURI__.core.invoke(cmd, args);
  }

  throw new Error('Backend de Rust (Tauri IPC) no detectado. Abre la aplicación con `tauri dev`.');
}

export function isTauriEnvironment(): boolean {
  if (typeof window === 'undefined') return false;
  const w = window as unknown as Record<string, unknown>;
  return '__TAURI_INTERNALS__' in w || '__TAURI__' in w;
}

export async function toggleDevtools(): Promise<void> {
  if (isTauriEnvironment()) {
    try {
      await invokeTauri('toggle_devtools');
    } catch (e) {
      console.error('Error al alternar devtools:', e);
    }
  }
}
