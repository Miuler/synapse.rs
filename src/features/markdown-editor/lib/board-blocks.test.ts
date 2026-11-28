import { describe, expect, it } from 'bun:test';
import { findBoardBlocks } from './board-blocks';

describe('bloques `board` de una nota', () => {
  it('da su sitio y su contenido', () => {
    const note = 'antes\n```board\nboard: A.board\nview: summary\n```\ndespués';
    const [block] = findBoardBlocks(note);
    expect(note.slice(block.from, block.to)).toBe('```board\nboard: A.board\nview: summary\n```');
    expect(block.code).toBe('board: A.board\nview: summary');
    expect(note.slice(block.from + block.codeOffset).startsWith('board: A.board')).toBe(true);
  });

  it('no toma un ejemplo escrito dentro de otro bloque, ni otros lenguajes', () => {
    const note = '````markdown\n```board\nboard: A\n```\n````\n```boardx\n```\n~~~ts\n```board\n~~~\n~~~board\nboard: B\n~~~';
    expect(findBoardBlocks(note).map((b) => b.code)).toEqual(['board: B']);
  });

  it('un bloque sin cerrar todavía no lo es', () => {
    expect(findBoardBlocks('```board\nboard: A')).toEqual([]);
    const [empty] = findBoardBlocks('```board\n```');
    expect([empty.code, empty.codeOffset]).toEqual(['', 9]);
  });
});
