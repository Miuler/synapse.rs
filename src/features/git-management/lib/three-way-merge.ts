// Fusión de tres vías por líneas para el editor de conflictos: qué cambió cada lado respecto del
// antecesor común, dónde chocan, y a qué línea de un panel corresponde cada línea de otro.
import { diff } from '@codemirror/merge';

/** Líneas `[desde, hasta)`, desde 0. */
export type LineRange = [number, number];

export type ChunkKind = 'ours' | 'theirs' | 'both' | 'conflict';

export interface MergeChunk {
  /** Solo cambió un lado, los dos igual (`both`) o los dos distinto (`conflict`). */
  kind: ChunkKind;
  base: LineRange;
  ours: LineRange;
  theirs: LineRange;
}

export interface Hunk {
  fromA: number;
  toA: number;
  fromB: number;
  toB: number;
}

/** Líneas con su salto incluido: así «falta el salto final» también es una diferencia. */
export function splitLines(text: string): string[] {
  return text.match(/[^\n]*\n|[^\n]+$/g) ?? [];
}

/** Concatena dos bloques sin pegar la última línea del primero con la primera del segundo. */
export function joinBlocks(first: string, second: string): string {
  return first && second && !first.endsWith('\n') ? `${first}\n${second}` : first + second;
}

export function usesCrlf(text: string): boolean {
  return text.includes('\r\n');
}
export const toLf = (text: string) => text.replace(/\r\n/g, '\n');
export const fromLf = (text: string, crlf: boolean) => (crlf ? text.replace(/\n/g, '\r\n') : text);

/** Cambios de `a` a `b` por líneas: cada línea distinta es un carácter para el diff de CodeMirror. */
export function diffLines(a: string[], b: string[]): Hunk[] {
  const codes = new Map<string, string>();
  const encode = (lines: string[]) =>
    lines
      .map((line) => {
        let code = codes.get(line);
        if (code === undefined) {
          // Se salta el rango de los subrogados, que no son caracteres sueltos
          const n = codes.size + 1;
          code = String.fromCharCode(n < 0xd800 ? n : n + 0x800);
          codes.set(line, code);
        }
        return code;
      })
      .join('');
  const left = encode(a);
  const right = encode(b);
  // Más líneas distintas que códigos: todo el archivo es un solo cambio
  if (codes.size > 0xf000) return [{ fromA: 0, toA: a.length, fromB: 0, toB: b.length }];
  return diff(left, right).map(({ fromA, toA, fromB, toB }) => ({ fromA, toA, fromB, toB }));
}

const same = (a: string[], [from, to]: LineRange, b: string[], [bFrom, bTo]: LineRange) =>
  to - from === bTo - bFrom && a.slice(from, to).every((line, i) => line === b[bFrom + i]);

/** Bloques cambiados respecto del antecesor, en orden. Los que se tocan forman uno solo. */
export function mergeChunks(base: string[], ours: string[], theirs: string[]): MergeChunk[] {
  const pending = [
    ...diffLines(base, ours).map((hunk) => ({ hunk, side: 0 })),
    ...diffLines(base, theirs).map((hunk) => ({ hunk, side: 1 })),
  ].sort((x, y) => x.hunk.fromA - y.hunk.fromA || x.hunk.toA - y.hunk.toA);

  const chunks: MergeChunk[] = [];
  // Líneas que cada lado lleva de más respecto del antecesor hasta el bloque actual
  const offset = [0, 0];
  for (let i = 0; i < pending.length; ) {
    const from = pending[i].hunk.fromA;
    let to = pending[i].hunk.toA;
    const delta = [0, 0];
    const changed = [false, false];
    for (; i < pending.length && pending[i].hunk.fromA <= to; i++) {
      const { hunk, side } = pending[i];
      to = Math.max(to, hunk.toA);
      delta[side] += hunk.toB - hunk.fromB - (hunk.toA - hunk.fromA);
      changed[side] = true;
    }
    const range = (side: number): LineRange => [from + offset[side], to + offset[side] + delta[side]];
    const oursRange = range(0);
    const theirsRange = range(1);
    const kind: ChunkKind = !changed[1]
      ? 'ours'
      : !changed[0]
        ? 'theirs'
        : same(ours, oursRange, theirs, theirsRange)
          ? 'both'
          : 'conflict';
    chunks.push({ kind, base: [from, to], ours: oursRange, theirs: theirsRange });
    offset[0] += delta[0];
    offset[1] += delta[1];
  }
  return chunks;
}

/**
 * La línea de otro panel que corresponde a `line`. `anchors` son parejas (línea de origen, línea de
 * destino) crecientes; entre dos se interpola.
 */
export function mapLine(anchors: [number, number][], line: number): number {
  let previous: [number, number] = [0, 0];
  for (const anchor of anchors) {
    if (line < anchor[0]) {
      const span = anchor[0] - previous[0];
      return previous[1] + (span ? ((line - previous[0]) * (anchor[1] - previous[1])) / span : 0);
    }
    previous = anchor;
  }
  return previous[1] + (line - previous[0]);
}
