import { Facet } from '@codemirror/state';

/**
 * Facet de CodeMirror para proporcionar la ruta relativa del archivo
 * Markdown que se está editando actualmente en la bóveda.
 */
export const activeFilePathFacet = Facet.define<string | null, string | null>({
  combine: (values) => (values.length > 0 ? values[values.length - 1] : null),
});
