import { describe, expect, it } from 'bun:test';
import { EditorState } from '@codemirror/state';
import { codeFolding, foldEffect, foldedRanges, unfoldEffect } from '@codemirror/language';
import { braceFolding } from './brace-folding';
import { allFoldRanges, enclosingFoldRange, foldRangesWithin, foldsOnLine, foldsWithin } from './fold-range';

const doc = 'a: {\n  b: {\n    c\n    d\n  }\n  e\n}\nf';

function stateWith(text = doc) {
  return EditorState.create({ doc: text, extensions: [codeFolding(), braceFolding({ lineComment: '#' })] });
}
const at = (state: EditorState, needle: string) => state.doc.toString().indexOf(needle);
const text = (state: EditorState, r: { from: number; to: number } | null) => r && state.doc.sliceString(r.from, r.to);

describe('bloque que contiene al cursor', () => {
  it('dentro de un bloque devuelve el bloque más interno', () => {
    const state = stateWith();
    expect(text(state, enclosingFoldRange(state, at(state, 'c')))).toBe('\n    c\n    d\n  ');
    expect(text(state, enclosingFoldRange(state, at(state, 'e')))).toBe('\n  b: {\n    c\n    d\n  }\n  e\n');
  });

  it('en la línea que abre el bloque devuelve ese bloque', () => {
    const state = stateWith();
    expect(text(state, enclosingFoldRange(state, at(state, 'b:')))).toBe('\n    c\n    d\n  ');
  });

  it('si el interno ya está plegado devuelve el exterior', () => {
    let state = stateWith();
    const inner = enclosingFoldRange(state, at(state, 'c'))!;
    state = state.update({ effects: foldEffect.of(inner) }).state;
    expect(text(state, enclosingFoldRange(state, at(state, 'b:')))).toBe('\n  b: {\n    c\n    d\n  }\n  e\n');
  });

  it('fuera de todo bloque no devuelve nada', () => {
    const state = stateWith();
    expect(enclosingFoldRange(state, at(state, 'f'))).toBeNull();
  });
});

describe('pliegues de varios niveles', () => {
  const count = (state: EditorState) => {
    let n = 0;
    foldedRanges(state).between(0, state.doc.length, () => {
      n++;
    });
    return n;
  };

  it('allFoldRanges incluye los bloques internos', () => {
    const state = stateWith();
    expect(allFoldRanges(state).map((r) => text(state, r))).toEqual([
      '\n  b: {\n    c\n    d\n  }\n  e\n',
      '\n    c\n    d\n  ',
    ]);
  });

  it('plegar todo y abrir el exterior deja el interno plegado (como zM + zo en Vim)', () => {
    let state = stateWith();
    state = state.update({ effects: allFoldRanges(state).map((r) => foldEffect.of(r)) }).state;
    expect(count(state)).toBe(2);
    const [outer] = foldsOnLine(state, at(state, 'a:'));
    state = state.update({ effects: unfoldEffect.of(outer) }).state;
    expect(count(state)).toBe(1);
    expect(foldsOnLine(state, at(state, 'b:')).length).toBe(1);
  });

  it('foldRangesWithin y foldsWithin cubren el bloque y sus hijos (zC y zO)', () => {
    let state = stateWith();
    const outer = enclosingFoldRange(state, at(state, 'e'))!;
    expect(foldRangesWithin(state, outer).length).toBe(2);
    state = state.update({ effects: foldRangesWithin(state, outer).map((r) => foldEffect.of(r)) }).state;
    expect(foldsWithin(state, outer).length).toBe(2);
    state = state.update({ effects: foldsWithin(state, outer).map((r) => unfoldEffect.of(r)) }).state;
    expect(count(state)).toBe(0);
  });
});
