// Copiar Markdown ya pintado (modo lectura, chat del agente) con casillas de tarea: los correos y Teams web quitan
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
  if (node.nodeType === Node.TEXT_NODE) {
    const raw = node.textContent ?? '';
    // Saltos de línea del HTML entre bloques (p. ej. entre `</h2>` y `<ul>`): no son texto
    const parent = node.parentElement;
    if (!raw.trim() && (!parent || /^(DIV|UL|OL|BLOCKQUOTE|TABLE|THEAD|TBODY|TR)$/.test(parent.tagName))) return '';
    return raw.replace(/\s+/g, ' ');
  }
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

/** Casillas → símbolos, atributos limpios; devuelve el HTML y el texto plano de `box` */
function toRich(box: HTMLElement): { html: string; text: string } {
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

/**
 * Si la selección (dentro de `root`) tiene casillas de tarea, devuelve el HTML y el texto plano a
 * copiar con las casillas como símbolos; si no, `null` (se copia como siempre).
 */
export function taskAwareCopy(sel: Selection | null, root: HTMLElement): { html: string; text: string } | null {
  if (!sel || sel.isCollapsed) return null;
  const box = selectionFragment(sel, root);
  if (!box || !box.querySelector('input[type="checkbox"]')) return null;
  return toRich(box);
}

/** Todo el contenido de `el` como HTML limpio y texto plano (casillas como ☐ / ✅) */
export function richCopyOfElement(el: HTMLElement): { html: string; text: string } {
  const box = document.createElement('div');
  for (const child of el.childNodes) box.append(child.cloneNode(true));
  return toRich(box);
}

/**
 * Escribe en el portapapeles HTML y texto plano a la vez (para pegar con formato en un correo o en
 * Teams, o sin formato en otro sitio). Primero con el evento «copy» (síncrono, dentro del clic) y, si
 * el webview no lo permite, con `ClipboardItem`.
 */
export async function writeRichClipboard(data: { html: string; text: string }): Promise<boolean> {
  let done = false;
  const onCopy = (e: ClipboardEvent) => {
    if (!e.clipboardData) return;
    e.preventDefault();
    e.clipboardData.setData('text/html', data.html);
    e.clipboardData.setData('text/plain', data.text);
    done = true;
  };
  document.addEventListener('copy', onCopy, { capture: true });
  try {
    document.execCommand('copy');
  } catch {
    // sin execCommand: se prueba abajo
  } finally {
    document.removeEventListener('copy', onCopy, { capture: true });
  }
  if (done) return true;
  try {
    await navigator.clipboard.write([
      new ClipboardItem({
        'text/html': new Blob([data.html], { type: 'text/html' }),
        'text/plain': new Blob([data.text], { type: 'text/plain' }),
      }),
    ]);
    return true;
  } catch {
    await navigator.clipboard?.writeText(data.text).catch(() => {});
    return false;
  }
}
