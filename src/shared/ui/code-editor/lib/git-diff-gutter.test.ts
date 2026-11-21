import { describe, expect, it } from 'bun:test';
import { computeLineDiff } from './git-diff-gutter';

describe('computeLineDiff', () => {
  it('returns empty when base and current are identical', () => {
    expect(computeLineDiff('a\nb\nc', 'a\nb\nc')).toEqual([]);
  });

  it('marks all lines as added when base is empty (untracked file)', () => {
    expect(computeLineDiff('', 'line 1\nline 2')).toEqual([
      { line: 1, kind: 'added' },
      { line: 2, kind: 'added' },
    ]);
  });

  it('marks inserted lines as added', () => {
    expect(computeLineDiff('a\nc', 'a\nb\nc')).toEqual([{ line: 2, kind: 'added' }]);
  });

  it('marks replaced lines as modified', () => {
    expect(computeLineDiff('a\nb\nc', 'a\nB\nc')).toEqual([{ line: 2, kind: 'modified' }]);
  });

  it('marks modified + extra added lines in the same hunk', () => {
    expect(computeLineDiff('a\nb\nc', 'a\nB1\nB2\nc')).toEqual([
      { line: 2, kind: 'modified' },
      { line: 3, kind: 'added' },
    ]);
  });

  it('marks pure deletions on the preceding line', () => {
    expect(computeLineDiff('a\nb\nc', 'a\nc')).toEqual([{ line: 1, kind: 'deleted' }]);
  });

  it('normalizes CRLF line endings', () => {
    expect(computeLineDiff('a\r\nb\r\nc', 'a\nb\nc')).toEqual([]);
  });
});

import { charDiff, computeHunks } from './git-diff-gutter';

describe('computeHunks', () => {
  it('guarda el texto de antes y el de ahora de cada bloque', () => {
    expect(computeHunks('a\nb\nc\nd', 'a\nB\nc')).toEqual([
      { kind: 'modified', startLine: 2, newCount: 1, oldLines: ['b'], newLines: ['B'] },
      { kind: 'deleted', startLine: 3, newCount: 0, oldLines: ['d'], newLines: [] },
    ]);
    expect(computeHunks('a\nc', 'a\nb\nc')).toEqual([{ kind: 'added', startLine: 2, newCount: 1, oldLines: [], newLines: ['b'] }]);
  });
});

describe('charDiff', () => {
  it('marca solo la letra que cambió', () => {
    expect(charDiff('reintentos: 3', 'reintentos: 5')).toEqual({
      before: [{ text: 'reintentos: ', changed: false }, { text: '3', changed: true }],
      after: [{ text: 'reintentos: ', changed: false }, { text: '5', changed: true }],
    });
  });

  it('marca lo borrado y lo añadido en medio de la línea', () => {
    const d = charDiff('url: /api/v1/pagos', 'url: /api/v2/pagos/lote');
    expect(d.before.filter((s) => s.changed).map((s) => s.text)).toEqual(['1']);
    expect(d.after.filter((s) => s.changed).map((s) => s.text)).toEqual(['2', '/lote']);
  });
});
