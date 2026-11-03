import { describe, expect, it } from 'bun:test';
import { EditorState } from '@codemirror/state';
import { findBraceRange } from './brace-folding';

const d2 = { lineComment: '#' };

function rangeAtLine(doc: string, lineNumber: number, options = d2) {
  const state = EditorState.create({ doc });
  const line = state.doc.line(lineNumber);
  const range = findBraceRange(state, line.from, line.to, options);
  return range && { text: state.doc.sliceString(range.from, range.to) };
}

describe('plegado por llaves', () => {
  it('pliega un bloque hasta su llave de cierre', () => {
    const doc = 'server: {\n  shape: rectangle\n  label: web\n}\nnext';
    expect(rangeAtLine(doc, 1)).toEqual({ text: '\n  shape: rectangle\n  label: web\n' });
  });

  it('respeta bloques anidados', () => {
    const doc = 'a: {\n  b: {\n    c\n  }\n  d\n}';
    expect(rangeAtLine(doc, 1)?.text).toBe('\n  b: {\n    c\n  }\n  d\n');
    expect(rangeAtLine(doc, 2)?.text).toBe('\n    c\n  ');
  });

  it('no pliega llaves que se cierran en la misma línea', () => {
    expect(rangeAtLine('a: { b }\nc', 1)).toBeNull();
  });

  it('ignora llaves en cadenas y comentarios', () => {
    const doc = 'a: {\n  label: "}"\n  # } comentario\n  b\n}';
    expect(rangeAtLine(doc, 1)?.text).toBe('\n  label: "}"\n  # } comentario\n  b\n');
    expect(rangeAtLine('# a: {\nb\n}', 1)).toBeNull();
  });

  it('usa el comentario de Mermaid', () => {
    const doc = 'classDef x {\n  %% }\n  fill: red\n}';
    expect(rangeAtLine(doc, 1, { lineComment: '%%' })?.text).toBe('\n  %% }\n  fill: red\n');
  });

  it('sin llave de cierre no pliega', () => {
    expect(rangeAtLine('a: {\n  b', 1)).toBeNull();
  });
});
