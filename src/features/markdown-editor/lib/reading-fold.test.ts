import { expect, it } from 'bun:test';
import { ancestorKeys, hiddenBlocks, type FoldBlock } from './reading-fold';

const b = (level: number, key = ''): FoldBlock => ({ level, key });
// # A, texto, ## A1, texto, ### A1a, texto, ## A2, texto, # B, texto
const note = [b(1, 'A'), b(0), b(2, 'A1'), b(0), b(3, 'A1a'), b(0), b(2, 'A2'), b(0), b(1, 'B'), b(0)];
const visible = (...keys: string[]) =>
  hiddenBlocks(note, new Set(keys)).flatMap((h, i) => (h ? [] : [note[i].key || `t${i}`]));

it('plegar un título oculta sus subtítulos hasta otro del mismo nivel o superior', () => {
  expect(visible()).toHaveLength(note.length);
  expect(visible('A')).toEqual(['A', 'B', 't9']);
  expect(visible('A1')).toEqual(['A', 't1', 'A1', 'A2', 't7', 'B', 't9']);
  expect(visible('A1a')).not.toContain('t5');
});

it('un subtítulo plegado sigue plegado al abrir su título, y el último llega al final', () => {
  expect(visible('A', 'A1')).toEqual(visible('A'));
  expect(visible('A1')).not.toContain('A1a');
  expect(visible('B')).not.toContain('t9');
});

it('los títulos que contienen a un bloque', () => {
  expect(ancestorKeys(note, 5)).toEqual(['A1a', 'A1', 'A']);
  expect(ancestorKeys(note, 7)).toEqual(['A2', 'A']);
  expect(ancestorKeys(note, 4)).toEqual(['A1', 'A']);
  expect(ancestorKeys(note, 8)).toEqual([]);
});
