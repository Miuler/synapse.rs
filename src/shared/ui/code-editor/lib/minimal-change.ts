/**
 * Cambio mínimo para pasar de `before` a `after`: solo el tramo distinto entre el prefijo y el
 * sufijo comunes. Reemplazar el documento entero manda el cursor al inicio; así, al recargar el
 * archivo desde disco, el cursor se queda donde estaba si su zona no cambió.
 */
export function minimalChange(before: string, after: string): { from: number; to: number; insert: string } {
  const max = Math.min(before.length, after.length);
  let start = 0;
  while (start < max && before.charCodeAt(start) === after.charCodeAt(start)) start++;
  let end = 0;
  while (end < max - start && before.charCodeAt(before.length - 1 - end) === after.charCodeAt(after.length - 1 - end)) end++;
  return { from: start, to: before.length - end, insert: after.slice(start, after.length - end) };
}
