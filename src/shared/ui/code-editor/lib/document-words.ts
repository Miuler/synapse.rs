import type { Completion, CompletionContext, CompletionResult } from '@codemirror/autocomplete';
import type { Text } from '@codemirror/state';

/** Palabras: letras, números, `_` y `-` interiores (los identificadores de D2 y Mermaid los usan). */
const WORD = /[\p{L}\p{N}_](?:[\p{L}\p{N}_-]*[\p{L}\p{N}_])?/gu;
const MIN_LENGTH = 2;
/** Se recorren como mucho estos caracteres del documento, para no frenar archivos enormes. */
const MAX_SCAN_CHARS = 200_000;

// Palabras por versión del documento: cada tecla crea un Text nuevo, pero abrir y filtrar la lista no.
const cache = new WeakMap<Text, Map<string, number>>();

/** Palabras del documento con cuántas veces aparecen. */
export function documentWords(doc: Text): Map<string, number> {
  let words = cache.get(doc);
  if (!words) {
    words = new Map();
    for (const [word] of doc.sliceString(0, MAX_SCAN_CHARS).matchAll(WORD)) {
      if (word.length >= MIN_LENGTH) words.set(word, (words.get(word) ?? 0) + 1);
    }
    cache.set(doc, words);
  }
  return words;
}

/**
 * Añade a las sugerencias del lenguaje las palabras que ya están en el documento (variables, nombres
 * de nodos…), aunque el lenguaje no sepa qué son. Van detrás de las del lenguaje y sin repetirlas.
 */
export function withDocumentWords(
  languageSource: (context: CompletionContext) => CompletionResult | null,
): (context: CompletionContext) => CompletionResult | null {
  return (context) => {
    const language = languageSource(context);
    const typed = context.matchBefore(/[\p{L}\p{N}_-]+/u);
    if (!typed && !context.explicit) return language;
    // Si el lenguaje completa desde otra posición (p. ej. tras `shape:`), sus opciones mandan.
    const from = typed?.from ?? context.pos;
    if (language && language.from !== from) return language;

    // `shape: ` del lenguaje ya cubre la palabra `shape`.
    const known = new Set(language?.options.map((o) => o.label.replace(/[\s:]+$/, '')));
    const prefix = typed?.text ?? '';
    const words: Completion[] = [];
    for (const word of documentWords(context.state.doc).keys()) {
      // No se sugiere la propia palabra que se está escribiendo.
      if (word !== prefix && !known.has(word)) words.push({ label: word, type: 'text', boost: -50 });
    }
    if (!words.length) return language;
    return { from, options: [...(language?.options ?? []), ...words], validFor: /^[\p{L}\p{N}_-]*$/u };
  };
}

/** Solo las palabras del documento, para editores sin sugerencias propias del lenguaje (markdown). */
export const documentWordsCompletion = withDocumentWords(() => null);
