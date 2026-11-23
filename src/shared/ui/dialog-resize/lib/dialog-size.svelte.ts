// Tamaño de un diálogo centrado que el usuario cambia arrastrando bordes o esquinas.
// Se guarda en localStorage con una clave por diálogo y vale para toda la app.

export interface DialogSize {
  w: number;
  h: number;
}

export interface DialogSizeOptions {
  /** Si es `false` (ej. popover anclado a un borde), el arrastre aplica 1x en vez de 2x. */
  centered?: boolean;
  /** Sobrescribe el centrado horizontal (2x cuando está centrado horizontalmente, 1x si no). */
  centeredX?: boolean;
  /** Sobrescribe el centrado vertical (2x cuando está centrado verticalmente, 1x si está anclado arriba). */
  centeredY?: boolean;
  /** Margen reservado de alto respecto al viewport (`100vh - margin`). */
  maxHeightMargin?: number;
  /** Claves heredadas de localStorage para conservar tamaños guardados previamente. */
  legacyKeys?: { w: string; h: string };
}

const MARGIN = 16;

export function persistedDialogSize(
  key: string,
  initial: DialogSize,
  min: DialogSize,
  options?: DialogSizeOptions,
) {
  const factorX = (options?.centeredX ?? options?.centered ?? true) ? 2 : 1;
  const factorY = (options?.centeredY ?? options?.centered ?? true) ? 2 : 1;
  const hMargin = options?.maxHeightMargin ?? MARGIN;

  function load(): DialogSize {
    try {
      const v = JSON.parse(localStorage.getItem(key) ?? 'null');
      if (v && v.w > 0 && v.h > 0) return { w: v.w, h: v.h };
      if (options?.legacyKeys) {
        const lw = Number.parseInt(localStorage.getItem(options.legacyKeys.w) ?? '', 10);
        const lh = Number.parseInt(localStorage.getItem(options.legacyKeys.h) ?? '', 10);
        if (lw > 0 || lh > 0) {
          return {
            w: lw > 0 ? lw : initial.w,
            h: lh > 0 ? lh : initial.h,
          };
        }
      }
    } catch {
      /* sin tamaño guardado */
    }
    return initial;
  }

  let size = $state(load());
  let resizing = $state(false);
  const clampW = (w: number) => Math.max(min.w, Math.min(window.innerWidth - MARGIN, w));
  const clampH = (h: number) => Math.max(min.h, Math.min(window.innerHeight - hMargin, h));

  /** Empieza a arrastrar un borde (`dx`/`dy` = -1, 0 o 1 según el lado). */
  function startResize(e: PointerEvent, dx: -1 | 0 | 1, dy: -1 | 0 | 1) {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    resizing = true;
    document.body.style.userSelect = 'none';
    const x0 = e.clientX;
    const y0 = e.clientY;
    const w0 = clampW(size.w);
    const h0 = clampH(size.h);
    const move = (ev: PointerEvent) => {
      size = {
        w: dx ? clampW(w0 + factorX * dx * (ev.clientX - x0)) : w0,
        h: dy ? clampH(h0 + factorY * dy * (ev.clientY - y0)) : h0,
      };
    };
    const up = () => {
      resizing = false;
      document.body.style.userSelect = '';
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      window.removeEventListener('pointercancel', up);
      try {
        localStorage.setItem(key, JSON.stringify({ w: Math.round(size.w), h: Math.round(size.h) }));
      } catch {
        /* sin almacenamiento */
      }
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
    window.addEventListener('pointercancel', up);
  }

  return {
    get size() {
      return size;
    },
    get resizing() {
      return resizing;
    },
    /** Estilo en línea con el tamaño actual, para el contenedor del diálogo. */
    get style() {
      return `width: ${size.w}px; height: ${size.h}px; max-width: calc(100vw - ${MARGIN}px); max-height: calc(100vh - ${hMargin}px);`;
    },
    startResize,
  };
}
