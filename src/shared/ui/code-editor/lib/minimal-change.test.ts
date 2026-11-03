import { describe, expect, it } from 'bun:test';
import { minimalChange } from './minimal-change';

const apply = (before: string, c: { from: number; to: number; insert: string }) => before.slice(0, c.from) + c.insert + before.slice(c.to);

describe('cambio mínimo al recargar el contenido', () => {
  it('solo toca el tramo distinto', () => {
    expect(minimalChange('a -> b\nb -> c\n', 'a -> b\nb -> d\n')).toEqual({ from: 12, to: 13, insert: 'd' });
  });

  it('inserciones y borrados al inicio, al final y en medio', () => {
    for (const [before, after] of [['abc', 'xabc'], ['abc', 'abcx'], ['abc', 'ac'], ['aaa', 'aa'], ['', 'abc'], ['abc', ''], ['abc', 'abc']]) {
      expect(apply(before, minimalChange(before, after))).toBe(after);
    }
  });
});
