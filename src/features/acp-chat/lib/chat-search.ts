/** Búsqueda en lo ya dibujado de una conversación, sin tocar su DOM (CSS Custom Highlight API). */

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
// Botones y opciones son interfaz, no conversación
const SKIP = 'button, select, textarea, input, svg, style, script';

/**
 * Coincidencias de `query` dentro de `root`, en orden; también dentro de lo plegado. Sin `tools`, de las
 * herramientas solo se mira el título.
 */
export function searchRanges(root: HTMLElement, query: string, tools: boolean): Range[] {
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
    if (!tools && parent.closest('.acp-tool') && !parent.closest('summary')) continue;
    const b = parent.closest(BLOCKS);
    if (b !== block) flush();
    block = b;
    nodes.push({ node: n, start: text.length });
    text += n.data;
  }
  flush();
  return ranges;
}

const ALL = 'synapse-chat-search';
const ACTIVE = 'synapse-chat-search-active';

/** Pinta las coincidencias; sin soporte del motor, la búsqueda sigue contando y desplazando. */
export function paintMatches(ranges: Range[], active: Range | null) {
  if (typeof CSS === 'undefined' || !CSS.highlights) return;
  const others = ranges.filter((r) => r !== active);
  const set = (name: string, list: Range[]) => {
    CSS.highlights.delete(name);
    if (!list.length) return;
    const h = new Highlight();
    for (const r of list) h.add(r);
    CSS.highlights.set(name, h);
  };
  set(ALL, others);
  set(ACTIVE, active ? [active] : []);
}

/** Deja `range` a la vista en `scroller` (que puede tener zoom). */
export function revealRange(scroller: HTMLElement, range: Range) {
  const box = scroller.getBoundingClientRect();
  const rect = range.getBoundingClientRect();
  if (!box.height || (rect.top >= box.top + 8 && rect.bottom <= box.bottom - 8)) return;
  const scale = box.height / scroller.offsetHeight || 1;
  scroller.scrollTop += (rect.top + rect.height / 2 - (box.top + box.height / 2)) / scale;
}
