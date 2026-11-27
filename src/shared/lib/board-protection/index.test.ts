import { describe, expect, it } from 'bun:test';
import { boardFolder, boardLockedPaths } from './index';

describe('la carpeta de un tablero', () => {
  it('se deduce de su archivo', () => {
    expect(boardFolder('a/Equipo.board.md')).toBe('a/Equipo.board');
    expect(['a/nota.md', '.board.md', 'a/.board.md', 'Equipo.board'].map(boardFolder)).toEqual([null, null, null, null]);
  });

  it('protege lo que contiene solo mientras su tablero existe', () => {
    const paths = ['a/Equipo.board.md', 'a/Equipo.board/t1.md', 'a/Equipo.board/sub/x.png', 'a/nota.md', 'Otro.board/t1.md'];
    expect([...boardLockedPaths(paths)]).toEqual(['a/Equipo.board/t1.md', 'a/Equipo.board/sub/x.png']);
    expect(boardLockedPaths(['a/nota.md']).size).toBe(0);
  });
});
