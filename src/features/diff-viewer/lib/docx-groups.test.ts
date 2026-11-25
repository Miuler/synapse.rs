import { expect, test } from 'bun:test';
import type { DocxDiffEntry } from '../api/docx-diff.api';
import { countChanges, groupEntries } from './docx-groups';

const entries = (s: string): DocxDiffEntry[] =>
  [...s].map((c) => ({
    kind: c === '+' ? 'added' : c === '-' ? 'removed' : 'equal',
    block: { type: 'text', text: c, heading: null, listLevel: null },
  }));
const shape = (s: string) => groupEntries(entries(s)).map((g) => `${g.type}@${g.start}:${g.entries.length}`);

test('pliega lo que no cambia y deja contexto', () => {
  expect(shape('==========+==========')).toEqual(['fold@0:8', 'entries@8:5', 'fold@13:8']);
  // Dos bloques fuera de contexto no se pliegan
  expect(shape('====+')).toEqual(['entries@0:5']);
  expect(shape('=====+')).toEqual(['fold@0:3', 'entries@3:3']);
  expect(shape('')).toEqual([]);
});

test('no pierde ningún bloque y cuenta los cambios', () => {
  const all = entries('======-===========+====+=======');
  expect(groupEntries(all).flatMap((g) => g.entries)).toEqual(all);
  expect(countChanges(all)).toEqual({ added: 2, removed: 1, modified: 0 });
});
