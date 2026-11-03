/** Estado de los renders de diagramas en curso, para la barra de estado. */
export const diagramRenderStatus = $state({
  /** Renders en curso (puede haber varios visores abiertos). */
  active: 0,
  /** Motor del render en curso o del último (D2, Mermaid…). */
  engine: '',
  /** `performance.now()` al empezar el render en curso. */
  startedAt: 0,
  /** Duración del último render terminado, en ms. */
  lastMs: null as number | null,
});

const MIN_DELAY_MS = 400;
const MAX_DELAY_MS = 1500;

export interface RenderScheduler {
  /** Pide un render tras una pausa al escribir (más larga si el diagrama tarda en renderizarse). */
  schedule(code: string): void;
  /** Render inmediato, aunque el código no haya cambiado (Ctrl+Enter). */
  now(code: string): void;
  destroy(): void;
}

/**
 * Agrupa los renders de un visor: espera a que se deje de escribir y nunca encola más de uno. Antes
 * cada tecla lanzaba un render, y con un layout lento (TALA tarda segundos) se acumulaban y el
 * diagrama llegaba con mucho retraso. Si llega código nuevo durante un render, al terminar se hace
 * uno solo con el último.
 */
export function createRenderScheduler(engine: string, task: (code: string) => Promise<void>): RenderScheduler {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let running = false;
  let pending: string | null = null;
  let lastRendered: string | null = null;
  let lastMs = 0;
  let destroyed = false;

  async function run(code: string) {
    running = true;
    const start = performance.now();
    diagramRenderStatus.active++;
    diagramRenderStatus.engine = engine;
    diagramRenderStatus.startedAt = start;
    try {
      await task(code);
      lastRendered = code;
    } finally {
      lastMs = performance.now() - start;
      diagramRenderStatus.active--;
      diagramRenderStatus.engine = engine;
      diagramRenderStatus.lastMs = Math.round(lastMs);
      running = false;
      if (pending !== null && !destroyed) {
        const next = pending;
        pending = null;
        void run(next);
      }
    }
  }

  function request(code: string, force: boolean) {
    if (!force && code === lastRendered && pending === null) return;
    if (running) {
      pending = code;
      return;
    }
    void run(code);
  }

  function clearTimer() {
    if (timer) clearTimeout(timer);
    timer = null;
  }

  return {
    schedule(code) {
      clearTimer();
      // El primer render no espera; después, la pausa se ajusta a lo que tardó el anterior.
      if (lastRendered === null && !running) return request(code, false);
      const delay = Math.min(MAX_DELAY_MS, Math.max(MIN_DELAY_MS, lastMs));
      timer = setTimeout(() => request(code, false), delay);
    },
    now(code) {
      clearTimer();
      request(code, true);
    },
    destroy() {
      destroyed = true;
      clearTimer();
    },
  };
}
