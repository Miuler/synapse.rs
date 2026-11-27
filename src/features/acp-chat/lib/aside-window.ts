// Lugar y tamaño de la ventana de las preguntas aparte, recordados entre arranques.

export interface WindowBox {
  x: number;
  y: number;
  w: number;
  h: number;
}

const KEY = 'synapse.agent.aside.window';
const SIZE = { w: 420, h: 460 };
/** Lo mínimo que queda a la vista al arrastrarla hacia un borde. */
const VISIBLE = { w: 120, h: 40 };
const GAP = 12;

/**
 * La posición guardada, sin salirse de la ventana. Sin ella, junto al chat (a su izquierda, para
 * seguir trabajando en él) y alineada con su borde inferior.
 */
export function placeWindow(
  saved: WindowBox | null,
  chat: { left: number; bottom: number } | null,
  viewport: { w: number; h: number },
): WindowBox {
  const w = Math.min(saved?.w ?? SIZE.w, viewport.w);
  const h = Math.min(saved?.h ?? SIZE.h, viewport.h);
  const x = saved?.x ?? (chat?.left ?? viewport.w) - w - GAP;
  const y = saved?.y ?? (chat?.bottom ?? viewport.h) - h - GAP;
  const clamp = (v: number, max: number) => Math.max(0, Math.min(max, v));
  return { x: clamp(x, viewport.w - VISIBLE.w), y: clamp(y, viewport.h - VISIBLE.h), w, h };
}

export function loadWindow(): WindowBox | null {
  try {
    const box = JSON.parse(localStorage.getItem(KEY) ?? 'null');
    return box && [box.x, box.y, box.w, box.h].every(Number.isFinite) ? box : null;
  } catch {
    return null;
  }
}

export function saveWindow(box: WindowBox) {
  try {
    localStorage.setItem(KEY, JSON.stringify(box));
  } catch {
    /* sin almacenamiento: se recoloca la próxima vez */
  }
}
