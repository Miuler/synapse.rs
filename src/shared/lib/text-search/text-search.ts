/** Búsqueda en lo ya dibujado, sin tocar el DOM (CSS Custom Highlight API). */

/** Minúsculas y sin tildes, conservando la longitud: las posiciones valen en el texto original. */
export function foldForSearch(text: string): string {
  let out = '';
  for (const unit of text.split('')) {
    const folded = (unit.normalize('NFD')[0] ?? unit).toLowerCase();
    out += folded.length === 1 ? folded : unit;
  }
  return out;
}

/** Posiciones (sin solaparse) de `query` en `text`, ya plegado con `foldForSearch`. */
export function findAll(folded: string, query: string): number[] {
  const q = foldForSearch(query);
  const found: number[] = [];
  if (!q) return found;
  for (let i = folded.indexOf(q); i >= 0; i = folded.indexOf(q, i + q.length)) found.push(i);
  return found;
}

// Entre bloques no hay coincidencias («fin» + «inicio» no forman una palabra)
const BLOCKS = 'p, li, td, th, pre, h1, h2, h3, h4, h5, h6, blockquote, summary, div, details';
// Botones y opciones son interfaz, no contenido
const SKIP = 'button, select, textarea, input, svg, style, script';

/** Coincidencias de `query` dentro de `root`, en orden; también dentro de lo plegado. `skip` descarta texto. */
export function searchRanges(root: HTMLElement, query: string, skip?: (parent: Element) => boolean): Range[] {
  if (!foldForSearch(query).trim()) return [];
  const ranges: Range[] = [];
  let nodes: { node: Text; start: number }[] = [];
  let text = '';
  let block: Element | null = null;
  const flush = () => {
    if (nodes.length) {
      // Última pieza que empieza en `at` o antes; para un final, la que lo contiene
      const locate = (at: number) => {
        let k = nodes.length - 1;
        while (nodes[k].start > at) k--;
        return { node: nodes[k].node, offset: at - nodes[k].start };
      };
      for (const i of findAll(foldForSearch(text), query)) {
        const from = locate(i);
        const to = locate(i + query.length - 1);
        const range = document.createRange();
        range.setStart(from.node, from.offset);
        range.setEnd(to.node, to.offset + 1);
        ranges.push(range);
      }
    }
    nodes = [];
    text = '';
  };
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  for (let n = walker.nextNode() as Text | null; n; n = walker.nextNode() as Text | null) {
    const parent = n.parentElement;
    if (!n.data || !parent || parent.closest(SKIP)) continue;
    if (skip?.(parent)) continue;
    const b = parent.closest(BLOCKS);
    if (b !== block) flush();
    block = b;
    nodes.push({ node: n, start: text.length });
    text += n.data;
  }
  flush();
  return ranges;
}

const ALL = 'synapse-search';
const ACTIVE = 'synapse-search-active';
/** Lo que resalta cada barra de búsqueda abierta: los nombres de resaltado son de toda la página. */
const painted = new Map<object, { ranges: Range[]; active: Range | null }>();

/** Pinta las coincidencias de `owner`; sin soporte del motor, la búsqueda sigue contando y desplazando. */
export function paintMatches(owner: object, ranges: Range[], active: Range | null) {
  if (typeof CSS === 'undefined' || !CSS.highlights) return;
  if (ranges.length) painted.set(owner, { ranges, active });
  else painted.delete(owner);
  const all = new Highlight();
  const current = new Highlight();
  for (const p of painted.values()) {
    for (const r of p.ranges) (r === p.active ? current : all).add(r);
  }
  for (const [name, h] of [[ALL, all], [ACTIVE, current]] as const) {
    if (h.size) CSS.highlights.set(name, h);
    else CSS.highlights.delete(name);
  }
}

/** Tope de resaltados pintados a la vez: WebKit tarda unos 2 ms en dibujar cada uno. */
export const MAX_PAINTED = 150;

/**
 * Las coincidencias que se ven en `scroller` (más `active`), hasta `MAX_PAINTED`. Las de lo plegado
 * o fuera de la vista también cuestan al pintarlas, así que solo se resaltan estas.
 */
export function visibleRanges(scroller: HTMLElement, ranges: Range[], active: Range | null): Range[] {
  const box = scroller.getBoundingClientRect();
  const margin = box.height / 2;
  const out: Range[] = [];
  for (const r of ranges) {
    const rect = r.getBoundingClientRect();
    if (rect.height && rect.bottom > box.top - margin && rect.top < box.bottom + margin) {
      out.push(r);
      if (out.length >= MAX_PAINTED) break;
    }
  }
  if (active && !out.includes(active)) out.push(active);
  return out;
}

/** Deja `range` a la vista en `scroller` (que puede tener zoom). */
export function revealRange(scroller: HTMLElement, range: Range) {
  const box = scroller.getBoundingClientRect();
  const rect = range.getBoundingClientRect();
  if (!box.height || (rect.top >= box.top + 8 && rect.bottom <= box.bottom - 8)) return;
  const scale = box.height / scroller.offsetHeight || 1;
  scroller.scrollTop += (rect.top + rect.height / 2 - (box.top + box.height / 2)) / scale;
}
