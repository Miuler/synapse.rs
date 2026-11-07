/** Indica si la consulta filtra por encabezados (`hs:xxx` o `headings:xxx`), también negado o agrupado. */
export function hasHeadingFilter(query: string): boolean {
  return /(^|\s)[-+(]*(hs|headings):\S/.test(query);
}
