import { describe, expect, it } from 'bun:test';
import { fileKindName, matchesKindFilter, parseKindFilter, type KindClassifier } from './kind-filter';

const ends = (...exts: string[]) => (path: string) => exts.some((e) => path.toLowerCase().endsWith(`.${e}`));
const types: KindClassifier = {
  isPdfFile: ends('pdf'),
  isWordFile: ends('docx'),
  isExcelFile: ends('xlsx'),
  isDrawingFile: ends('excalidraw', 'excalidraw.md'),
  isMarkdownFile: (p) => ends('md')(p) && !ends('excalidraw.md')(p),
  isDiagramFile: ends('mermaid'),
  isCodeFile: ends('cbl'),
  isImageFile: ends('png'),
};

describe('kind-filter', () => {
  it('separa los tokens kind: del texto', () => {
    expect(parseKindFilter('plan KIND:PDF kind:word')).toEqual({ text: 'plan', kinds: ['pdf', 'word'] });
    expect(parseKindFilter('kind:')).toEqual({ text: 'kind:', kinds: [] });
    expect(parseKindFilter('solo texto')).toEqual({ text: 'solo texto', kinds: [] });
  });

  it('clasifica por ruta', () => {
    expect(fileKindName('a/Manual.PDF', types)).toBe('pdf');
    expect(fileKindName('d.excalidraw.md', types)).toBe('excalidraw');
    expect(fileKindName('nota.md', types)).toBe('markdown');
    expect(fileKindName('x.zzz', types)).toBeNull();
  });

  it('filtra por prefijo del tipo y sin filtros deja pasar todo', () => {
    expect(matchesKindFilter('a.pdf', ['pd'], types)).toBe(true);
    expect(matchesKindFilter('a.md', ['pdf'], types)).toBe(false);
    expect(matchesKindFilter('a.md', ['pdf', 'markdown'], types)).toBe(true);
    expect(matchesKindFilter('x.zzz', ['pdf'], types)).toBe(false);
    expect(matchesKindFilter('x.zzz', [], types)).toBe(true);
  });
});
