import { describe, expect, it } from 'bun:test';
import { sharedReads } from './shared-reads';

describe('lecturas compartidas', () => {
  it('comparte la que está en marcha y vuelve a leer cuando termina o cambia algo', async () => {
    const asked: string[] = [];
    const read = sharedReads(async (path) => (asked.push(path), asked.length));
    expect(await Promise.all([read('a'), read('a'), read('b')])).toEqual([1, 1, 2]);
    expect(await read('a')).toBe(3);
    const before = read('a');
    read.forget();
    expect(await Promise.all([before, read('a')])).toEqual([4, 5]);
    expect(asked).toEqual(['a', 'b', 'a', 'a', 'a']);
  });

  it('un fallo no se queda guardado', async () => {
    let calls = 0;
    const read = sharedReads(async () => {
      if (++calls === 1) throw new Error('no');
      return calls;
    });
    await expect(read('a')).rejects.toThrow('no');
    expect(await read('a')).toBe(2);
  });
});
