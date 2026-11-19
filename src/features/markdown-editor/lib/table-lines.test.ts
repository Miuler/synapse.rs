import { describe, expect, it } from 'bun:test';
import { layoutTableLines } from './table-extension';

const lines = (texts: string[]) => {
  let pos = 0;
  return texts.map((text) => {
    const l = { from: pos, text };
    pos += text.length + 1;
    return l;
  });
};

describe('layoutTableLines', () => {
  it('divide en celdas con pipes exteriores y calcula el ancho de cada columna', () => {
    const src = lines(['| API | CUD |', '|---|---|', '| [[core.yaml]] | x |']);
    const { lines: out, widths } = layoutTableLines(src);
    expect(out.map((l) => l.kind)).toEqual(['header', 'separator', 'row']);
    expect(out[0].cells.length).toBe(2);
    expect(out[0].pipes).toEqual([0, 6, 12]);
    expect(widths).toEqual([13, 3]);
    // La celda va entre las pipes, sin incluirlas
    expect(src[2].text.slice(out[2].cells[0].from - src[2].from, out[2].cells[0].to - src[2].from).trim()).toBe('[[core.yaml]]');
  });

  it('sin pipes exteriores también hay celdas', () => {
    const { lines: out } = layoutTableLines(lines(['a | b', '--|--', '1 | 2']));
    expect(out[2].cells.length).toBe(2);
  });

  it('no corta celdas en pipes escapadas ni dentro de código', () => {
    const { lines: out } = layoutTableLines(lines(['| a | b |', '|---|---|', '| x \\| y | `p|q` |']));
    expect(out[2].cells.length).toBe(2);
    expect(out[2].pipes.length).toBe(3);
  });

  it('una celda vacía en medio se conserva', () => {
    const { lines: out } = layoutTableLines(lines(['| a | b | c |', '|---|---|---|', '| 1 |  | 3 |']));
    expect(out[2].cells.map((c) => c.col)).toEqual([0, 1, 2]);
  });
});
