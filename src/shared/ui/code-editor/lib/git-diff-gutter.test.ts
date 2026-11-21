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
