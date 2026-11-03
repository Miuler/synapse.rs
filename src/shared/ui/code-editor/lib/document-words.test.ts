import { describe, expect, it } from 'bun:test';
import { EditorState } from '@codemirror/state';
import { CompletionContext, type CompletionResult } from '@codemirror/autocomplete';
import { d2Autocomplete } from './diagram-autocompletion';
import { documentWords, withDocumentWords } from './document-words';

function complete(doc: string, source = d2Autocomplete) {
  const state = EditorState.create({ doc });
  const result = withDocumentWords(source)(new CompletionContext(state, doc.length, false));
  return result && { from: result.from, labels: result.options.map((o) => o.label), words: result.options.filter((o) => o.type === 'text').map((o) => o.label) };
}

describe('palabras del documento en el autocompletado', () => {
  it('cuenta las palabras, con guiones interiores', () => {
    const words = documentWords(EditorState.create({ doc: 'api-gateway -> base_datos\napi-gateway: x-' }).doc);
    expect(words.get('api-gateway')).toBe(2);
    expect(words.has('base_datos')).toBe(true);
    expect(words.has('x')).toBe(false);
  });

  it('sugiere variables definidas en el documento', () => {
    const result = complete('servidor_web -> base\nserv');
    expect(result?.words).toContain('servidor_web');
    expect(result?.from).toBe('servidor_web -> base\n'.length);
  });

  it('no se sugiere la palabra que se está escribiendo', () => {
    expect(complete('alfa\nbeta')?.words ?? []).not.toContain('beta');
  });

  it('las del lenguaje van primero y no se repiten', () => {
    const result = complete('shape -> mi_nodo\nsha')!;
    expect(result.labels).toContain('shape: ');
    expect(result.words).not.toContain('shape');
    const firstWord = result.labels.indexOf('mi_nodo');
    expect(result.labels.slice(0, firstWord).every((l) => !result.words.includes(l))).toBe(true);
  });

  it('sin nada escrito y sin pedirlo no sugiere palabras', () => {
    const empty = (() => null) as unknown as (c: CompletionContext) => CompletionResult | null;
    expect(complete('alfa beta ', empty)).toBeNull();
  });
});
