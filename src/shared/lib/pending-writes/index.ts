/**
 * Puente para forzar el guardado de lo que está en buffer (editores con cambios sin persistir)
 * antes de operaciones que trabajan sobre el archivo en disco: renombrar, mover, `git add`, etc.
 * El dueño del buffer (WorkspacePage) registra el flusher; cualquier feature puede invocarlo.
 */
type Flusher = (paths?: string[]) => Promise<void>;

let flusher: Flusher | null = null;

/** Registra el flusher; devuelve la función para desregistrarlo. */
export function registerPendingWritesFlusher(fn: Flusher): () => void {
  flusher = fn;
  return () => {
    if (flusher === fn) flusher = null;
  };
}

/**
 * Persiste los cambios pendientes. Con `paths` solo afecta a esos archivos o a los que cuelgan
 * de ellos (carpetas); sin argumentos, a todos. Respeta la regla de archivo bloqueado.
 */
export async function flushPendingWrites(paths?: string[]): Promise<void> {
  if (!flusher) return;
  try {
    await flusher(paths);
  } catch (err) {
    console.warn('No se pudieron guardar los cambios pendientes:', err);
  }
}
