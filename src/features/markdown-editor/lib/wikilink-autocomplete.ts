import {
  autocompletion,
  type CompletionContext,
  type CompletionResult,
  type Completion,
} from '@codemirror/autocomplete';
import { vaultApi } from '@entities/vault-item';
import { documentWordsCompletion } from '@shared/ui/code-editor';

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
    if (query.startsWith('#')) {
      const needle = query.slice(1).trim().toLowerCase();
      const searchResp = await vaultApi.searchNotes('.board/');
      const results = (searchResp.results || []).filter((r) => {
        const fullPath = r.note_path || r.text || '';
        if (!fullPath.includes('.board/')) return false;
        if (!needle) return true;
        return fullPath.toLowerCase().includes(needle) || (r.text ?? '').toLowerCase().includes(needle);
      });

      const options: Completion[] = results.map((r) => {
        const fullPath = r.note_path || r.text;
        const boardIdx = fullPath.lastIndexOf('.board/');
        const boardStart = fullPath.lastIndexOf('/', boardIdx);
        const boardFolder = fullPath.slice(boardStart + 1, boardIdx + 6); // Nombre.board
        const fileName = fullPath.slice(boardIdx + 7);
        const taskId = fileName.endsWith('.md') ? fileName.slice(0, -3) : fileName;
        const ref = `${boardFolder}/${taskId}`;
        const title = r.text && r.text !== taskId ? r.text : undefined;

        return {
          label: ref,
          detail: title,
          type: 'text',
          boost: r.is_recent ? 2 : (r.score ? r.score / 100 : 0),
          apply: (view, completion, _from, to) => {
            const insertText = `${completion.label}]]`;
            view.dispatch({
              changes: { from: match.from + 2, to, insert: insertText },
              selection: { anchor: match.from + 2 + insertText.length },
            });
          },
        };
      });

      return {
        from: match.from + 2,
        options,
        filter: false,
      };
    }

    const searchResp = await vaultApi.searchNotes(query);
    const results = searchResp.results || [];

    const options: Completion[] = results.map((r) => {
      const fullPath = r.note_path || r.text;
      // Obtener únicamente el nombre del archivo (sin directorios)
      const fileName = fullPath.includes('/') ? fullPath.split('/').pop() || fullPath : fullPath;
      // Si termina en .md, removerlo para una presentación más limpia como estilo Obsidian
      const cleanName = fileName.endsWith('.md') ? fileName.slice(0, -3) : fileName;
      const title = r.text || cleanName;

      return {
        label: cleanName,
        detail: cleanName !== title ? title : undefined,
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

/** Palabras del documento, salvo dentro de `[[...]]`, donde se sugieren notas. */
function documentWordsOutsideWikilinks(context: CompletionContext): CompletionResult | null {
  if (context.matchBefore(/\[\[([^\]\n]*)$/)) return null;
  return documentWordsCompletion(context);
}

/**
 * Extensión de CodeMirror 6 para autocompletado de Wikilinks y de las palabras del documento.
 */
export const wikilinkAutocomplete = autocompletion({
  override: [wikilinkCompletionSource, documentWordsOutsideWikilinks],
  activateOnTyping: true,
  maxRenderedOptions: 30,
});
