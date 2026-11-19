// Colores del tema para lo que se dibuja en canvas/WebGL (grafo, láser, diff de píxeles): el CSS no
// llega ahí, así que se leen las variables con getComputedStyle y se redibuja al cambiar de tema.

/** Color resuelto de una variable CSS (`--laser`), o `fallback` si no está definida. */
export function cssVar(name: string, fallback: string, el: Element = document.documentElement): string {
  const v = getComputedStyle(el).getPropertyValue(name).trim();
  return v || fallback;
}

/**
 * El mismo color con otra opacidad, para el canvas (que no acepta `color-mix`). Entiende `#rgb`,
 * `#rrggbb`, `#rrggbbaa` y `rgb()/rgba()`; con otro formato devuelve el color tal cual.
 */
export function withAlpha(color: string, alpha: number): string {
  const c = color.trim();
  let r: number, g: number, b: number;
  const hex = /^#([0-9a-f]{3}|[0-9a-f]{6}|[0-9a-f]{8})$/i.exec(c);
  if (hex) {
    const h = hex[1].length === 3 ? [...hex[1]].map((x) => x + x).join('') : hex[1].slice(0, 6);
    [r, g, b] = [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16));
  } else {
    const m = /^rgba?\(\s*([\d.]+)[\s,]+([\d.]+)[\s,]+([\d.]+)/i.exec(c);
    if (!m) return c;
    [r, g, b] = [m[1], m[2], m[3]].map(Number);
  }
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

/**
 * Avisa cuando puede haber cambiado el tema: atributos de <html> (`data-theme`, clase, estilo) o
 * estilos añadidos o reescritos en <head> (temas de usuario). Devuelve la función para dejar de oír.
 */
export function onThemeChange(cb: () => void): () => void {
  let queued = false;
  const fire = () => {
    if (queued) return;
    queued = true;
    requestAnimationFrame(() => {
      queued = false;
      cb();
    });
  };
  const html = new MutationObserver(fire);
  html.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme', 'class', 'style'] });
  const head = new MutationObserver(fire);
  head.observe(document.head, { childList: true, subtree: true, characterData: true });
  return () => {
    html.disconnect();
    head.disconnect();
  };
}
