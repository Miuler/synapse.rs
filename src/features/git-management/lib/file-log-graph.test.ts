import { describe, expect, it } from 'bun:test';
import { buildFileLogGraph } from './file-log-graph';

describe('buildFileLogGraph', () => {
  it('una sola rama queda en la columna 0', () => {
    const rows = buildFileLogGraph([
      { commit_hash: 'c', parents: ['b'] },
      { commit_hash: 'b', parents: ['a'] },
      { commit_hash: 'a', parents: [] },
    ]);
    expect(rows.map((r) => r.col)).toEqual([0, 0, 0]);
    expect(rows[2].segments.filter((s) => s.half === 'bottom')).toEqual([]);
  });

  it('una fusión abre una segunda columna para la otra rama', () => {
    // m fusiona x (rama principal) e y (otra rama); los dos agregaron el archivo
    const rows = buildFileLogGraph([
      { commit_hash: 'top', parents: ['m'] },
      { commit_hash: 'm', parents: ['y', 'x'] },
      { commit_hash: 'x', parents: [] },
      { commit_hash: 'y', parents: [] },
    ]);
    expect(rows.map((r) => r.col)).toEqual([0, 0, 1, 0]);
    expect(rows[1].segments.filter((s) => s.half === 'bottom').map((s) => [s.from, s.to])).toEqual([[0, 0], [0, 1]]);
    // En la fila de x, la rama de y sigue de largo por la columna 0
    expect(rows[2].segments).toContainEqual({ from: 0, to: 0, half: 'top', color: rows[0].color });
    expect(rows[2].color).not.toBe(rows[0].color);
  });

  it('ignora padres que no están en la lista', () => {
    const rows = buildFileLogGraph([{ commit_hash: 'a', parents: ['fuera'] }]);
    expect(rows[0].segments).toEqual([]);
  });
});
