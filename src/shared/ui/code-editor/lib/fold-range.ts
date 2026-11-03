import { foldable, foldedRanges } from '@codemirror/language';
import type { EditorState } from '@codemirror/state';

/** Cuántas líneas hacia arriba se busca el inicio del bloque que contiene al cursor. */
const MAX_LINES_UP = 5_000;

/**
 * Bloque plegable más interno que contiene `pos` y que aún no está plegado (como `zc` en Vim:
 * dentro de un bloque pliega ese bloque, y repetido pliega el siguiente hacia afuera).
 */
export function enclosingFoldRange(state: EditorState, pos: number): { from: number; to: number } | null {
  const folded = foldedRanges(state);
  const isFolded = (from: number, to: number) => {
    let found = false;
    folded.between(from, to, (a, b) => {
      if (a === from && b === to) found = true;
    });
    return found;
  };
  const cursorLine = state.doc.lineAt(pos).number;
  for (let n = cursorLine; n >= 1 && cursorLine - n <= MAX_LINES_UP; n--) {
    const line = state.doc.line(n);
    const range = foldable(state, line.from, line.to);
    if (!range) continue;
    const contains = n === cursorLine || (range.from <= pos && pos <= range.to);
    if (contains && !isFolded(range.from, range.to)) return range;
  }
  return null;
}
