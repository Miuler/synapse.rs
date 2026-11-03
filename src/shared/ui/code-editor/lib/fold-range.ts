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

/** Todos los bloques plegables del documento, de todos los niveles. */
export function allFoldRanges(state: EditorState): { from: number; to: number }[] {
  const ranges: { from: number; to: number }[] = [];
  for (let n = 1; n <= state.doc.lines; n++) {
    const line = state.doc.line(n);
    const range = foldable(state, line.from, line.to);
    if (range) ranges.push(range);
  }
  return ranges;
}

/** Bloques plegables dentro de `outer` (incluido el propio `outer`). */
export function foldRangesWithin(state: EditorState, outer: { from: number; to: number }): { from: number; to: number }[] {
  const ranges = [outer];
  const first = state.doc.lineAt(outer.from).number + 1;
  const last = state.doc.lineAt(outer.to).number;
  for (let n = first; n <= last; n++) {
    const line = state.doc.line(n);
    const range = foldable(state, line.from, line.to);
    if (range && range.to <= outer.to) ranges.push(range);
  }
  return ranges;
}

/** Pliegues ya hechos que empiezan en la línea de `pos`. */
export function foldsOnLine(state: EditorState, pos: number): { from: number; to: number }[] {
  const line = state.doc.lineAt(pos);
  const found: { from: number; to: number }[] = [];
  foldedRanges(state).between(line.from, line.to, (from, to) => {
    if (from >= line.from && from <= line.to) found.push({ from, to });
  });
  return found;
}

/** Pliegues ya hechos dentro de `outer` (incluido el propio `outer`). */
export function foldsWithin(state: EditorState, outer: { from: number; to: number }): { from: number; to: number }[] {
  const found: { from: number; to: number }[] = [];
  foldedRanges(state).between(outer.from, outer.to, (from, to) => {
    if (from >= outer.from && to <= outer.to) found.push({ from, to });
  });
  return found;
}
