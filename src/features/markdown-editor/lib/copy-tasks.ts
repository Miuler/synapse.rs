// Copiar desde el modo lectura una selección con casillas de tarea: los correos y Teams web quitan
// los `<input type="checkbox">` al pegar, así que se cambian por símbolos (☐ pendiente, ✅ hecha).
// Se copia como HTML limpio (sin clases ni estilos del tema) y como texto plano.

export const TASK_OPEN = '☐';
export const TASK_DONE = '✅';

/** Atributos que se conservan al copiar (el resto, clases, `data-*`, estilos del tema…, se quita) */
const KEEP_ATTRS = new Set(['href', 'src', 'alt', 'title', 'colspan', 'rowspan']);

/** Contenido de la selección dentro de `root`, con la lista que lo envuelve si se copian `<li>` sueltos */
function selectionFragment(sel: Selection, root: HTMLElement): HTMLElement | null {
  const box = document.createElement('div');
  for (let i = 0; i < sel.rangeCount; i++) {
    const range = sel.getRangeAt(i);
    if (!root.contains(range.commonAncestorContainer)) continue;
    const frag = range.cloneContents();
    const ancestor =
      range.commonAncestorContainer instanceof Element
        ? range.commonAncestorContainer
        : range.commonAncestorContainer.parentElement;
    // Si toda la selección cae dentro de un mismo <li>, su casilla no entra en el rango: se añade
    const li = ancestor?.closest('li');
    const ownBox = li?.querySelector(':scope > input[type="checkbox"]');
    if (li && ownBox && !frag.querySelector('input[type="checkbox"]')) {
      frag.prepend(ownBox.cloneNode());
    }
    // `<li>` sueltos: se envuelven en su lista para que se peguen como lista
    const list = ancestor?.closest('ul, ol');
    if (list && [...frag.childNodes].some((n) => n.nodeName === 'LI')) {
      const wrapper = document.createElement(list.nodeName.toLowerCase());
      wrapper.append(frag);
      box.append(wrapper);
    } else {
      box.append(frag);
    }
  }
  return box.childNodes.length ? box : null;
}

function cleanAttributes(root: HTMLElement) {
  for (const el of root.querySelectorAll('*')) {
    for (const attr of [...el.attributes]) {
      if (!KEEP_ATTRS.has(attr.name)) el.removeAttribute(attr.name);
    }
  }
}

/** Texto plano: una línea por elemento de lista (con su sangría) y párrafos separados */
function toPlain(node: Node, depth = 0): string {
  if (node.nodeType === Node.TEXT_NODE) return (node.textContent ?? '').replace(/\s+/g, ' ');
  if (!(node instanceof Element)) return '';
  const tag = node.tagName;
  if (tag === 'BR') return '\n';
  if (tag === 'UL' || tag === 'OL') {
    return [...node.children]
      .filter((c) => c.tagName === 'LI')
      .map((li) => toPlain(li, depth))
      .join('');
  }
  if (tag === 'LI') {
    // Como en la lista: viñeta y casilla («- ☐ texto»)
    let inline = '';
    let nested = '';
    for (const child of node.childNodes) {
      if (child instanceof Element && (child.tagName === 'UL' || child.tagName === 'OL')) nested += toPlain(child, depth + 1);
      else inline += toPlain(child, depth);
    }
    return `${'  '.repeat(depth)}- ${inline.trim()}\n${nested}`;
  }
  const inner = [...node.childNodes].map((c) => toPlain(c, depth)).join('');
  return /^(P|H[1-6]|BLOCKQUOTE|PRE|TABLE|TR|DIV)$/.test(tag) ? `${inner.trim()}\n` : inner;
}

/**
 * Si la selección (dentro de `root`) tiene casillas de tarea, devuelve el HTML y el texto plano a
 * copiar con las casillas como símbolos; si no, `null` (se copia como siempre).
 */
export function taskAwareCopy(sel: Selection | null, root: HTMLElement): { html: string; text: string } | null {
  if (!sel || sel.isCollapsed) return null;
  const box = selectionFragment(sel, root);
  if (!box || !box.querySelector('input[type="checkbox"]')) return null;
  for (const input of box.querySelectorAll('input[type="checkbox"]')) {
    const done = (input as HTMLInputElement).hasAttribute('checked') || (input as HTMLInputElement).checked;
    // `marked` ya deja un espacio tras la casilla; se añade solo si falta
    const spaced = /^\s/.test(input.nextSibling?.textContent ?? '');
    input.replaceWith(document.createTextNode(`${done ? TASK_DONE : TASK_OPEN}${spaced ? '' : ' '}`));
  }
  cleanAttributes(box);
  const text = [...box.childNodes].map((n) => toPlain(n)).join('').replace(/\n{3,}/g, '\n\n').trim();
  return { html: box.innerHTML, text };
}
