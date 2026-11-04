import { describe, expect, it } from 'bun:test';
import { EditorState } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import { occurrenceSelection, selectOccurrences } from './rename-occurrences';

function select(doc: string, cursor: number) {
  const state = EditorState.create({ doc, selection: { anchor: cursor }, extensions: EditorState.allowMultipleSelections.of(true) });
  const selection = occurrenceSelection(state);
  return selection && { texts: selection.ranges.map((r) => doc.slice(r.from, r.to)), froms: selection.ranges.map((r) => r.from), main: selection.mainIndex };
}

describe('seleccionar coincidencias (Shift+F6)', () => {
  it('selecciona solo coincidencias de palabra completa', () => {
    const doc = 'server -> db\nservers: x\nmyserver\nserver.shape: y';
    const result = select(doc, 2)!;
    expect(result.froms).toEqual([0, doc.lastIndexOf('server.')]);
    expect(result.texts).toEqual(['server', 'server']);
  });

  it('la selección principal es la del cursor', () => {
    const doc = 'a -> b\nb -> a';
    expect(select(doc, doc.length - 1)!.main).toBe(1);
  });

  it('sin palabra bajo el cursor no hace nada', () => {
    expect(select('a ->  b', 5)).toBeNull();
  });

  it('solo funciona si el editor tiene foco', () => {
    let dispatched = false;
    const viewWithoutFocus = {
      hasFocus: false,
      state: EditorState.create({ doc: 'foo foo', selection: { anchor: 0 } }),
      dispatch: () => {
        dispatched = true;
      },
    } as unknown as EditorView;

    expect(selectOccurrences(viewWithoutFocus)).toBe(false);
    expect(dispatched).toBe(false);

    const viewWithFocus = {
      hasFocus: true,
      state: EditorState.create({ doc: 'foo foo', selection: { anchor: 0 } }),
      dispatch: () => {
        dispatched = true;
      },
    } as unknown as EditorView;

    expect(selectOccurrences(viewWithFocus)).toBe(true);
    expect(dispatched).toBe(true);
  });
});
