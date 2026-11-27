import { describe, expect, it } from 'bun:test';
import { joinBlocks, mapLine, mergeChunks, splitLines, toLf, fromLf, type MergeChunk } from './three-way-merge';

const chunksOf = (base: string, ours: string, theirs: string) =>
  mergeChunks(splitLines(base), splitLines(ours), splitLines(theirs));

/** El resultado tomando de cada bloque el lado que cambió (en conflicto, los dos). */
function applyAll(base: string, ours: string, theirs: string): string {
  const [b, o, t] = [base, ours, theirs].map(splitLines);
  let out = '';
  let at = 0;
  for (const c of mergeChunks(b, o, t)) {
    out += b.slice(at, c.base[0]).join('');
    const left = o.slice(...c.ours).join('');
    const right = t.slice(...c.theirs).join('');
    out += c.kind === 'theirs' ? right : c.kind === 'conflict' ? joinBlocks(left, right) : left;
    at = c.base[1];
  }
  return out + b.slice(at).join('');
}

describe('fusión de tres vías', () => {
  const base = '1\n2\n3\n4\n5\n6\n';

  it('separa lo de cada lado y los conflictos', () => {
    expect(chunksOf(base, '1\nmio\n3\n4\n5\n6\n', '1\nsuyo\n3\n4\n5\nseis\n')).toEqual([
      { kind: 'conflict', base: [1, 2], ours: [1, 2], theirs: [1, 2] },
      { kind: 'theirs', base: [5, 6], ours: [5, 6], theirs: [5, 6] },
    ] satisfies MergeChunk[]);
  });

  it('desplaza los rangos de cada lado tras insertar y borrar', () => {
    const ours = '0\n1\n2\n3\n4\n5\n6\n';
    const theirs = '1\n2\n3\n4\n6\n';
    expect(chunksOf(base, ours, theirs)).toEqual([
      { kind: 'ours', base: [0, 0], ours: [0, 1], theirs: [0, 0] },
      { kind: 'theirs', base: [4, 5], ours: [5, 6], theirs: [4, 4] },
    ]);
    expect(applyAll(base, ours, theirs)).toBe('0\n1\n2\n3\n4\n6\n');
  });

  it('el mismo cambio en los dos lados no es conflicto', () => {
    expect(chunksOf(base, '1\nx\n3\n4\n5\n6\n', '1\nx\n3\n4\n5\n6\n').map((c) => c.kind)).toEqual(['both']);
  });

  it('une los cambios que se tocan en un solo conflicto', () => {
    const [chunk, ...rest] = chunksOf(base, '1\na\nb\n4\n5\n6\n', '1\n2\n3\nc\n5\n6\n');
    expect(rest).toEqual([]);
    expect(chunk).toEqual({ kind: 'conflict', base: [1, 4], ours: [1, 4], theirs: [1, 4] });
  });

  it('sin antecesor, el archivo entero es un conflicto', () => {
    expect(chunksOf('', 'a\n', 'b\n')).toEqual([{ kind: 'conflict', base: [0, 0], ours: [0, 1], theirs: [0, 1] }]);
  });

  it('respeta la falta de salto final y no pega líneas', () => {
    expect(applyAll('a\nb', 'a\nb', 'a\nb\nc')).toBe('a\nb\nc');
    expect(applyAll('a', 'mio', 'suyo')).toBe('mio\nsuyo');
    expect(joinBlocks('', 'x')).toBe('x');
  });

  it('conserva CRLF de ida y vuelta', () => {
    expect(fromLf(toLf('a\r\nb\r\n'), true)).toBe('a\r\nb\r\n');
  });

  it('aguanta muchas líneas distintas', () => {
    const lines = Array.from({ length: 20_000 }, (_, i) => `línea ${i}\n`);
    const changed = lines.with(15_000, 'otra\n');
    expect(mergeChunks(lines, changed, lines)).toEqual([
      { kind: 'ours', base: [15_000, 15_001], ours: [15_000, 15_001], theirs: [15_000, 15_001] },
    ]);
  });

  it('traslada líneas entre paneles interpolando entre anclas', () => {
    const anchors: [number, number][] = [[2, 2], [4, 8]];
    expect(mapLine(anchors, 1)).toBe(1);
    expect(mapLine(anchors, 3)).toBe(5);
    expect(mapLine(anchors, 6)).toBe(10);
  });
});
