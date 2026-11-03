import { foldService } from '@codemirror/language';
import type { EditorState } from '@codemirror/state';

interface BraceFoldingOptions {
  /** Inicio de comentario de línea (`#` en D2, `%%` en Mermaid): lo que sigue no cuenta. */
  lineComment: string;
}

/** Límite de caracteres que se recorren buscando la llave de cierre. */
const MAX_SCAN_CHARS = 200_000;

/**
 * Plegado entre `{` y su `}` para lenguajes sin árbol sintáctico (D2 y Mermaid son
 * `StreamLanguage`). Se pliega desde la última `{` sin cerrar de la línea hasta su pareja, si esta
 * está en otra línea. Ignora llaves dentro de cadenas y comentarios.
 */
export function braceFolding(options: BraceFoldingOptions) {
  return foldService.of((state, lineStart, lineEnd) => findBraceRange(state, lineStart, lineEnd, options));
}

export function findBraceRange(
  state: EditorState,
  lineStart: number,
  lineEnd: number,
  { lineComment }: BraceFoldingOptions,
): { from: number; to: number } | null {
  const line = state.doc.sliceString(lineStart, lineEnd);
  const open = lastUnclosedBrace(line, lineComment);
  if (open === -1) return null;

  const from = lineStart + open + 1;
  const limit = Math.min(state.doc.length, from + MAX_SCAN_CHARS);
  const text = state.doc.sliceString(from, limit);
  let depth = 1;
  let quote: string | null = null;
  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    if (quote) {
      if (ch === '\\') i++;
      else if (ch === quote || ch === '\n') quote = null;
      continue;
    }
    if (text.startsWith(lineComment, i)) {
      const nl = text.indexOf('\n', i);
      if (nl === -1) break;
      i = nl;
      continue;
    }
    if (ch === '"' || ch === "'") quote = ch;
    else if (ch === '{') depth++;
    else if (ch === '}' && --depth === 0) {
      const to = from + i;
      return to > lineEnd ? { from, to } : null;
    }
  }
  return null;
}

/** Posición de la última `{` de la línea que no se cierra en la misma línea, o -1. */
function lastUnclosedBrace(line: string, lineComment: string): number {
  const open: number[] = [];
  let quote: string | null = null;
  for (let i = 0; i < line.length; i++) {
    const ch = line[i];
    if (quote) {
      if (ch === '\\') i++;
      else if (ch === quote) quote = null;
      continue;
    }
    if (line.startsWith(lineComment, i)) break;
    if (ch === '"' || ch === "'") quote = ch;
    else if (ch === '{') open.push(i);
    else if (ch === '}') open.pop();
  }
  return open.length ? open[open.length - 1] : -1;
}
