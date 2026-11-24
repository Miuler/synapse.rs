// Correspondencia entre las tablas del archivo y las que se ven en lectura, para conservar la fila al
// cambiar de modo. Se emparejan por orden de aparición.

const SEPARATOR = /^\s*\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)*\|?\s*$/;
/** Sin las marcas de cita (`> | a |` también es una tabla). */
const unquote = (line: string) => line.replace(/^\s*(?:>\s?)+/, '');

/** Primera y última línea (base 0) de cada tabla Markdown, sin contar las de bloques de código. */
export function tableLineRanges(lines: string[]): { header: number; last: number }[] {
  const out: { header: number; last: number }[] = [];
  let fence: string | null = null;
  for (let i = 0; i < lines.length; i++) {
    const line = unquote(lines[i]);
    const mark = /^\s{0,3}(`{3,}|~{3,})/.exec(line)?.[1];
    if (fence) {
      if (mark && mark[0] === fence[0] && mark.length >= fence.length) fence = null;
      continue;
    }
    if (mark) {
      fence = mark;
      continue;
    }
    const next = i + 1 < lines.length ? unquote(lines[i + 1]) : '';
    if (!line.includes('|') || !next.includes('-') || !next.includes('|') || !SEPARATOR.test(next)) continue;
    let last = i + 1;
    while (last + 1 < lines.length && unquote(lines[last + 1]).includes('|')) last++;
    out.push({ header: i, last });
    i = last;
  }
  return out;
}

/** Línea del archivo de la fila `row` (0 = cabecera) de la tabla número `table`. */
export function lineOfTableRow(lines: string[], table: number, row: number): number | null {
  const range = tableLineRanges(lines)[table];
  if (!range) return null;
  return row === 0 ? range.header : Math.min(range.header + row + 1, range.last);
}

/** Tabla y fila (0 = cabecera) que hay en una línea del archivo. */
export function tableRowAtLine(lines: string[], line: number): { table: number; row: number } | null {
  const table = tableLineRanges(lines).findIndex((r) => line >= r.header && line <= r.last);
  if (table < 0) return null;
  const { header } = tableLineRanges(lines)[table];
  return { table, row: Math.max(0, line - header - 1) };
}
