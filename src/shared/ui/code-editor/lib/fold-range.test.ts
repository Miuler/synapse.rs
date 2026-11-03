import { describe, expect, it } from 'bun:test';
import { EditorState } from '@codemirror/state';
import { codeFolding, foldEffect } from '@codemirror/language';
import { braceFolding } from './brace-folding';
import { enclosingFoldRange } from './fold-range';

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
