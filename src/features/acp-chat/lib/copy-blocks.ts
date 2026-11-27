// Copiar una selección de una respuesta del agente: los bloques completos (párrafos, listas, tablas…)
// van como su Markdown original en texto plano, y con formato en HTML.
import { richCopyOfNodes } from '@shared/lib/rich-copy';
import { markdownBlocks } from './markdown';

const squash = (text: string) => text.replace(/\s+/g, '');

/**
 * Índices del primer y último bloque de `message` si la selección son bloques completos suyos; `null`
 * si es un fragmento, si sale del mensaje o si es solo código (se pega tal cual, sin las comillas).
 */
function selectedBlocks(range: Range, message: Element): [number, number] | null {
  const touched: number[] = [];
  let covered = '';
  for (const [i, block] of [...message.children].entries()) {
    if (!range.intersectsNode(block)) continue;
    const part = document.createRange();
    part.selectNodeContents(block);
    if (range.compareBoundaryPoints(Range.START_TO_START, part) > 0) part.setStart(range.startContainer, range.startOffset);
    if (range.compareBoundaryPoints(Range.END_TO_END, part) < 0) part.setEnd(range.endContainer, range.endOffset);
    const text = part.toString().trim();
    // Solo roza el borde (p. ej. un triple clic acaba al principio del bloque siguiente)
    if (!text) continue;
    if (text !== block.textContent?.trim()) return null;
    touched.push(i);
    covered += text;
  }
  if (!touched.length || squash(covered) !== squash(range.toString())) return null;
  const [first, last] = [touched[0], touched[touched.length - 1]];
  if (first === last && message.children[first].tagName === 'PRE') return null;
  return [first, last];
}

/**
 * Lo que copiar si la selección (dentro de `root`) son bloques completos de una respuesta, marcada
 * con `data-reply`; `sourceOf` da el Markdown de la respuesta con ese id. Si no, `null`.
 */
export function blockCopy(
  sel: Selection | null,
  root: Element,
  sourceOf: (id: string) => string | undefined,
): { html: string; text: string } | null {
  if (!sel || sel.isCollapsed || sel.rangeCount !== 1) return null;
  const range = sel.getRangeAt(0);
  for (const message of root.querySelectorAll<HTMLElement>('[data-reply]')) {
    if (!range.intersectsNode(message)) continue;
    const selected = selectedBlocks(range, message);
    const source = selected && sourceOf(message.dataset.reply ?? '');
    if (!selected || !source) continue;
    const [first, last] = selected;
    // Qué trozos del Markdown pintan el primer y el último elemento seleccionado
    const blocks = markdownBlocks(source);
    let element = 0;
    let from = -1;
    let to = -1;
    for (const [i, block] of blocks.entries()) {
      const end = element + block.elements;
      if (first >= element && first < end) from = i;
      if (last >= element && last < end) to = i;
      element = end;
    }
    // Si las cuentas no cuadran (HTML suelto en la respuesta), se copia como siempre
    if (element !== message.children.length || from < 0 || to < 0) return null;
    const text = blocks.slice(from, to + 1).map((b) => b.raw).join('').replace(/^\n+|\s+$/g, '');
    return { html: richCopyOfNodes([...message.children].slice(first, last + 1)).html, text };
  }
  return null;
}
