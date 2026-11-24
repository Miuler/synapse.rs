import { expect, it } from 'bun:test';
import { lineOfTableRow, tableLineRanges, tableRowAtLine } from './table-anchor';

const LINES = [
  '# T', // 0
  '| a | b |',
  '|---|:-:|',
  '| 1 | x |',
  '| 2 | y |',
  '', // 5
  '```md',
  '| no | es |',
  '|----|----|',
  '```',
  '> | c | d |', // 10
  '> |---|---|',
  '> | 3 | z |',
  'texto a | b',
  '---',
];

it('encuentra las tablas, también citadas, y no las de código', () => {
  expect(tableLineRanges(LINES)).toEqual([
    { header: 1, last: 4 },
    { header: 10, last: 13 },
  ]);
  expect(tableLineRanges(['a | b', '---', 'texto'])).toEqual([]);
});

it('fila ↔ línea', () => {
  expect(lineOfTableRow(LINES, 0, 0)).toBe(1);
  expect(lineOfTableRow(LINES, 0, 2)).toBe(4);
  expect(lineOfTableRow(LINES, 0, 99)).toBe(4);
  expect(lineOfTableRow(LINES, 5, 0)).toBeNull();
  expect(tableRowAtLine(LINES, 1)).toEqual({ table: 0, row: 0 });
  expect(tableRowAtLine(LINES, 2)).toEqual({ table: 0, row: 0 });
  expect(tableRowAtLine(LINES, 4)).toEqual({ table: 0, row: 2 });
  expect(tableRowAtLine(LINES, 12)).toEqual({ table: 1, row: 1 });
  expect(tableRowAtLine(LINES, 0)).toBeNull();
});
