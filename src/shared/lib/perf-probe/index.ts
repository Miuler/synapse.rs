/**
 * Sonda de bloqueos del hilo principal, solo en desarrollo o en builds de medición
 * (VITE_SYNAPSE_IPC_LOG=1): anota en `window.__synapseLagLog` cada vez que un temporizador de 50 ms
 * llega tarde. La leen las pruebas de rendimiento (e2e).
 */
export interface LagLogEntry {
  /** Momento (performance.now) en que el temporizador por fin corrió. */
  t: number;
  /** Retraso en ms respecto de lo esperado. */
  ms: number;
}

const INTERVAL_MS = 50;
const MIN_LAG_MS = 30;

export function startMainThreadProbe(): void {
  const log = ((window as unknown as { __synapseLagLog?: LagLogEntry[] }).__synapseLagLog ??= []);
  let expected = performance.now() + INTERVAL_MS;
  setInterval(() => {
    const now = performance.now();
    const lag = now - expected;
    if (lag > MIN_LAG_MS) log.push({ t: now, ms: lag });
    expected = now + INTERVAL_MS;
  }, INTERVAL_MS);
}

export interface SpanLogEntry {
  name: string;
  start: number;
  ms: number;
}

const enabled = import.meta.env.DEV || import.meta.env.VITE_SYNAPSE_IPC_LOG === '1';

/** Anota en `window.__synapseSpanLog` un tramo que empezó en `start` (performance.now) y termina ahora. */
export function recordSpan(name: string, start: number): void {
  if (!enabled) return;
  const log = ((window as unknown as { __synapseSpanLog?: SpanLogEntry[] }).__synapseSpanLog ??= []);
  log.push({ name, start, ms: performance.now() - start });
}

/** Ejecuta `fn` y anota su duración como tramo (útil dentro de `$derived.by`). */
export function measured<T>(name: string, fn: () => T): T {
  if (!enabled) return fn();
  const start = performance.now();
  try {
    return fn();
  } finally {
    recordSpan(name, start);
  }
}
