// Resaltado de sintaxis de los paneles del merge, según la extensión del archivo.
import type { Extension } from '@codemirror/state';
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { tags } from '@lezer/highlight';
import { yaml } from '@codemirror/lang-yaml';
import { json } from '@codemirror/lang-json';
import { sql } from '@codemirror/lang-sql';
import { markdown } from '@codemirror/lang-markdown';

const highlight = HighlightStyle.define([
  { tag: tags.heading, color: 'var(--accent, #0969da)', fontWeight: 'bold' },
  { tag: tags.keyword, color: 'var(--syntax-keyword, #cf222e)' },
  { tag: tags.comment, color: 'var(--syntax-comment, #6e7781)', fontStyle: 'italic' },
  { tag: tags.string, color: 'var(--syntax-string, #0a3069)' },
  { tag: [tags.variableName, tags.propertyName], color: 'var(--syntax-variable, #953800)' },
  { tag: [tags.number, tags.bool, tags.null], color: 'var(--syntax-number, #0550ae)' },
]);

function language(path: string): Extension[] {
  const lower = path.toLowerCase();
  if (/\.ya?ml$/.test(lower)) return [yaml()];
  if (/\.jsonc?$/.test(lower)) return [json()];
  if (lower.endsWith('.sql')) return [sql()];
  if (/\.(md|markdown)$/.test(lower)) return [markdown()];
  return [];
}

export const codeSyntax = (path: string): Extension[] => [syntaxHighlighting(highlight), ...language(path)];
