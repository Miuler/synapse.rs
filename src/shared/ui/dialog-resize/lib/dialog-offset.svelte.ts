// Desplazamiento de un diálogo centrado que el usuario mueve arrastrando su cabecera. Se recuerda
// por diálogo mientras la app está abierta (no entre arranques: al volver a abrir Synapse, centrado).

import type { DialogSize } from './dialog-size.svelte';

/** Lo que queda visible del diálogo como mínimo al arrastrarlo hacia un borde. */
const VISIBLE = 80;

const offsets = new Map<string, { x: number; y: number }>();

export function dialogOffset(key: string) {
  let offset = $state(offsets.get(key) ?? { x: 0, y: 0 });

  /** Limita el desplazamiento para que la cabecera no salga de la ventana. */
  function clamp(x: number, y: number, size: DialogSize) {
    const w = Math.min(size.w, window.innerWidth);
    const h = Math.min(size.h, window.innerHeight);
    const maxX = (window.innerWidth + w) / 2 - VISIBLE;
    const maxYUp = (window.innerHeight - h) / 2;
    const maxYDown = (window.innerHeight + h) / 2 - VISIBLE;
    return {
      x: Math.max(-maxX, Math.min(maxX, x)),
      y: Math.max(-maxYUp, Math.min(maxYDown, y)),
    };
  }

  function set(next: { x: number; y: number }) {
    offset = next;
    offsets.set(key, next);
  }

  /** Empieza a mover el diálogo desde la cabecera (no desde sus botones o campos). */
  function startDrag(e: PointerEvent, size: DialogSize) {
    if (e.button !== 0) return;
    if ((e.target as HTMLElement).closest('button, input, select, textarea, a')) return;
    e.preventDefault();
    const x0 = e.clientX - offset.x;
    const y0 = e.clientY - offset.y;
    const move = (ev: PointerEvent) => set(clamp(ev.clientX - x0, ev.clientY - y0, size));
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  }

  return {
    get offset() {
      return offset;
    },
    /** Estilo en línea: el diálogo sigue centrado y se desplaza lo que el usuario lo movió. */
    style(size: DialogSize) {
      const { x, y } = clamp(offset.x, offset.y, size);
      return x || y ? `translate: ${x}px ${y}px;` : '';
    },
    startDrag,
    /** Vuelve a centrarlo. */
    reset() {
      set({ x: 0, y: 0 });
    },
  };
}
