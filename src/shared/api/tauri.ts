export interface RealNote {
  relative_path: string;
  title: string;
  content: string;
}

/**
 * Función auxiliar para invocar comandos IPC en Tauri 2.0 con soporte multiplataforma.
 */
export async function invokeTauri<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
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
