const MAX_QUERY = 120;
const MAX_ANSWER = 240;

/** `text` en una línea, recortado a `max` caracteres. */
function short(text: string, max: number): string {
  const chars = [...text.replace(/\s+/g, ' ').trim()];
  return chars.length > max ? `${chars.slice(0, max).join('').trimEnd()}…` : chars.join('');
}

/** Markdown como texto corrido: la notificación del sistema no lo pinta. */
function plain(markdown: string): string {
  return markdown
    .replace(/^\s*(```|~~~).*$/gm, '')
    .replace(/^\s*\|?[\s:|-]*-{3,}[\s:|-]*$/gm, '')
    .replace(/^\s*(#{1,6}|>|[-*+](?= ))\s*/gm, '')
    .replace(/^\[ \]/gm, '☐')
    .replace(/^\[[xX]\]/gm, '✅')
    .replace(/!?\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/\*\*|__(?=\S)|(?<=\S)__|`|~~/g, '')
    // Cursiva: solo asteriscos que se cierran (un `*.md` suelto se queda)
    .replace(/(?<![\p{L}\p{N}*])\*(?=\S)([^*\n]*[^*\s])\*(?![\p{L}\p{N}*])/gu, '$1')
    .replace(/^\s*\||\|\s*$/gm, '')
    .replace(/ \| /g, ' · ');
}

/**
 * Texto de la notificación de fin de turno: qué agente y, según `answer`, su respuesta final o la
 * consulta con la que empezó (también si falló o no respondió con texto).
 */
export function completionNotice(
  agent: string,
  handle: string,
  query: string,
  failed: boolean,
  answer?: string,
): { title: string; body: string } {
  const reply = failed ? '' : short(plain(answer ?? ''), MAX_ANSWER);
  return {
    title: `${agent} · ${handle}`,
    body: reply || `${failed ? 'Falló' : 'Terminó'} la consulta: "${short(query, MAX_QUERY)}"`,
  };
}
