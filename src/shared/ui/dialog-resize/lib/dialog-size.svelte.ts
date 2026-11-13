// Tamaño de un diálogo centrado que el usuario cambia arrastrando bordes o esquinas.
// Se guarda en localStorage con una clave por diálogo y vale para toda la app.

export interface DialogSize {
  w: number;
  h: number;
}

const MARGIN = 16;

export function persistedDialogSize(key: string, initial: DialogSize, min: DialogSize) {
  function load(): DialogSize {
    try {
      const v = JSON.parse(localStorage.getItem(key) ?? 'null');
      if (v && v.w > 0 && v.h > 0) return { w: v.w, h: v.h };
    } catch {
      /* sin tamaño guardado */
    }
    return initial;
  }

  let size = $state(load());
  const clampW = (w: number) => Math.max(min.w, Math.min(window.innerWidth - MARGIN, w));
  const clampH = (h: number) => Math.max(min.h, Math.min(window.innerHeight - MARGIN, h));

  /** Empieza a arrastrar un borde (`dx`/`dy` = -1, 0 o 1 según el lado). */
  function startResize(e: PointerEvent, dx: -1 | 0 | 1, dy: -1 | 0 | 1) {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    const x0 = e.clientX;
    const y0 = e.clientY;
    const w0 = clampW(size.w);
    const h0 = clampH(size.h);
    // El diálogo está centrado: crece igual hacia ambos lados, por eso el desplazamiento cuenta doble
    const move = (ev: PointerEvent) => {
      size = {
        w: dx ? clampW(w0 + 2 * dx * (ev.clientX - x0)) : w0,
        h: dy ? clampH(h0 + 2 * dy * (ev.clientY - y0)) : h0,
      };
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      try {
        localStorage.setItem(key, JSON.stringify(size));
      } catch {
        /* sin almacenamiento */
      }
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  }

  return {
    get size() {
      return size;
    },
    /** Estilo en línea con el tamaño actual, para el contenedor del diálogo. */
    get style() {
      return `width: ${size.w}px; height: ${size.h}px; max-width: calc(100vw - ${MARGIN}px); max-height: calc(100vh - ${MARGIN}px);`;
    },
    startResize,
  };
}
