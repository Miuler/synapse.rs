import {
  autocompletion,
  type CompletionContext,
  type CompletionResult,
  type Completion,
} from '@codemirror/autocomplete';
import { vaultApi } from '@entities/vault-item';

/**
 * Función de autocompletado para Wikilinks `[[...]]`.
 * Se activa automáticamente al escribir `[[` o cualquier texto dentro de corchetes dobles.
 */
export async function wikilinkCompletionSource(
  context: CompletionContext
): Promise<CompletionResult | null> {
  // Buscar coincidencia hacia atrás desde la posición del cursor
  // Captura `[[` seguido de caracteres que no contengan `]` ni saltos de línea
  const match = context.matchBefore(/\[\[([^\]\n]*)$/);
  if (!match) return null;

  // Si el usuario escribió `[[`, match.from es donde empieza `[[`
  // match.text es ej. `[[MiNota`
  const query = match.text.slice(2).trim(); // Texto después de `[[`

  try {
    const searchResp = await vaultApi.searchNotes(query);
    const results = searchResp.results || [];

    const options: Completion[] = results.map((r) => {
      const path = r.note_path || r.text;
      // Si termina en .md, removerlo para una presentación más limpia como estilo Obsidian
      const cleanLabel = path.endsWith('.md') ? path.slice(0, -3) : path;
      const title = r.text || cleanLabel;

      return {
        label: cleanLabel,
        detail: cleanLabel !== title ? title : (path.includes('/') ? path : undefined),
        type: 'file',
        boost: r.is_recent ? 2 : (r.score ? r.score / 100 : 0),
        apply: (view, completion, from, to) => {
          // Reemplaza desde después del `[[` hasta el cursor
          // E inserta el nombre de la nota con el cierre `]]`
          const insertText = `${completion.label}]]`;
          view.dispatch({
            changes: { from: match.from + 2, to, insert: insertText },
            selection: { anchor: match.from + 2 + insertText.length },
          });
        },
      };
    });

    // Si el usuario escribió algo que no existe exactamente en la lista,
    // ofrecer opción de crear nota con ese nombre exacto
    if (query && !options.some((o) => o.label.toLowerCase() === query.toLowerCase())) {
      options.push({
        label: query,
        detail: '(crear nueva nota)',
        type: 'text',
        boost: -1,
        apply: (view, completion, from, to) => {
          const insertText = `${completion.label}]]`;
          view.dispatch({
            changes: { from: match.from + 2, to, insert: insertText },
            selection: { anchor: match.from + 2 + insertText.length },
          });
        },
      });
    }

    return {
      from: match.from + 2,
      options,
      filter: false, // El filtrado difuso lo maneja Rust (Nucleo)
    };
  } catch (err) {
    console.error('Error al autocompletar wikilinks:', err);
    return null;
  }
}

/**
 * Extensión de CodeMirror 6 para autocompletado de Wikilinks.
 */
export const wikilinkAutocomplete = autocompletion({
  override: [wikilinkCompletionSource],
  activateOnTyping: true,
  maxRenderedOptions: 30,
});
