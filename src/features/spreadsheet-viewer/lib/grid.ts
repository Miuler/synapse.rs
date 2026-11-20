// Geometría de la rejilla del visor de hojas de cálculo: posiciones de filas y columnas visibles,
// celdas combinadas y el tramo (filas × columnas) que hay que pintar según el desplazamiento.
import type { CellStyle, Merge, SheetLayout } from '../model/types';

export const DEFAULT_COL_PX = 64;
export const DEFAULT_ROW_PX = 20;

export interface Geometry {
  /** Columnas visibles (sin las ocultas), con su ancho y su posición */
  cols: number[];
  colPx: number[];
  colLeft: number[];
  colIndex: Map<number, number>;
  width: number;
  /** Filas visibles, con su alto y su posición */
  rows: number[];
  rowPx: number[];
  rowTop: number[];
  rowIndex: Map<number, number>;
  height: number;
  /** Celdas combinadas ya colocadas (las que tienen alguna fila y columna visible) */
  merges: MergeBox[];
  /** Clave `fila:col` de las celdas dentro de una combinada (el ancla incluida) */
  covered: Set<string>;
}

export interface MergeBox extends Merge {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** Geometría de la hoja; `zoom` multiplica todas las medidas (crece todo, no solo la letra) */
export function buildGeometry(layout: SheetLayout, zoom = 1): Geometry {
  const hiddenRows = new Set(layout.hiddenRows);
  const hiddenCols = new Set(layout.hiddenCols);
  const widths = new Map(layout.colWidths);
  const heights = new Map(layout.rowHeights);
  // Como en Excel, la rejilla sigue vacía más allá de lo usado
  const nCols = Math.max(layout.cols + 2, 26);
  const nRows = Math.max(layout.rows + 20, 60);

  const cols: number[] = [];
  const colPx: number[] = [];
  const colLeft: number[] = [];
  const colIndex = new Map<number, number>();
  let x = 0;
  for (let c = 1; c <= nCols; c++) {
    if (hiddenCols.has(c)) continue;
    const w = widths.get(c);
    const px = Math.round((w ? Math.max(12, w * 7 + 5) : DEFAULT_COL_PX) * zoom);
    colIndex.set(c, cols.length);
    cols.push(c);
    colPx.push(px);
    colLeft.push(x);
    x += px;
  }

  const rows: number[] = [];
  const rowPx: number[] = [];
  const rowTop: number[] = [];
  const rowIndex = new Map<number, number>();
  let y = 0;
  for (let r = 1; r <= nRows; r++) {
    if (hiddenRows.has(r)) continue;
    const h = heights.get(r);
    const px = Math.round((h ? Math.max(8, (h * 4) / 3) : DEFAULT_ROW_PX) * zoom);
    rowIndex.set(r, rows.length);
    rows.push(r);
    rowPx.push(px);
    rowTop.push(y);
    y += px;
  }

  const merges: MergeBox[] = [];
  const covered = new Set<string>();
  for (const m of layout.merges) {
    const ri = rangeIdx(rows, m.r1, m.r2);
    const ci = rangeIdx(cols, m.c1, m.c2);
    if (!ri || !ci) continue;
    merges.push({
      ...m,
      top: rowTop[ri[0]],
      height: rowTop[ri[1]] + rowPx[ri[1]] - rowTop[ri[0]],
      left: colLeft[ci[0]],
      width: colLeft[ci[1]] + colPx[ci[1]] - colLeft[ci[0]],
    });
    for (let r = m.r1; r <= m.r2; r++) for (let c = m.c1; c <= m.c2; c++) covered.add(`${r}:${c}`);
  }

  return { cols, colPx, colLeft, colIndex, width: x, rows, rowPx, rowTop, rowIndex, height: y, merges, covered };
}

/** Índices (en `list`, ordenada) del primer y último valor dentro de `[a, b]` */
function rangeIdx(list: number[], a: number, b: number): [number, number] | null {
  const i = list.findIndex((v) => v >= a);
  if (i < 0 || list[i] > b) return null;
  let j = i;
  while (j + 1 < list.length && list[j + 1] <= b) j++;
  return [i, j];
}

/** Último índice con `arr[i] <= v` (0 si ninguno) */
export function bisect(arr: number[], v: number): number {
  let lo = 0;
  let hi = arr.length - 1;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (arr[mid] <= v) lo = mid;
    else hi = mid - 1;
  }
  return lo;
}

export interface Span {
  from: number;
  to: number;
  padStart: number;
  padEnd: number;
}

/** Tramo de índices a pintar para `[start, start+size]` con un margen, más el relleno antes/después */
export function span(pos: number[], px: number[], total: number, start: number, size: number, margin: number): Span {
  if (!pos.length) return { from: 0, to: -1, padStart: 0, padEnd: 0 };
  const from = bisect(pos, Math.max(0, start - margin));
  const to = Math.min(pos.length - 1, bisect(pos, start + size + margin));
  return { from, to, padStart: pos[from], padEnd: total - (pos[to] + px[to]) };
}

/** Tramo de filas y columnas a pintar según el desplazamiento */
export function visibleWindow(
  g: Geometry,
  scrollTop: number,
  scrollLeft: number,
  viewH: number,
  viewW: number,
): { rows: Span; cols: Span } {
  return {
    rows: span(g.rowTop, g.rowPx, g.height, scrollTop, viewH, 120),
    cols: span(g.colLeft, g.colPx, g.width, scrollLeft, viewW, 100),
  };
}

/** Combinadas que tocan el tramo */
export function visibleMerges(g: Geometry, top: number, bottom: number, left: number, right: number): MergeBox[] {
  return g.merges.filter((m) => m.top < bottom && m.top + m.height > top && m.left < right && m.left + m.width > left);
}

export function colName(c: number): string {
  let s = '';
  while (c > 0) {
    const m = (c - 1) % 26;
    s = String.fromCharCode(65 + m) + s;
    c = Math.floor((c - 1) / 26);
  }
  return s;
}

/** CSS de un estilo de celda (sin la alineación, que depende de si es número) */
export function styleToCss(st: CellStyle): string {
  const out: string[] = [];
  if (st.bold) out.push('font-weight:700');
  if (st.italic) out.push('font-style:italic');
  const deco = [st.underline && 'underline', st.strike && 'line-through'].filter(Boolean).join(' ');
  if (deco) out.push(`text-decoration:${deco}`);
  if (st.color) out.push(`color:${st.color}`);
  if (st.bg) out.push(`background:${st.bg}`);
  // En `em` respecto a la letra base de la rejilla (11 pt), que ya lleva el zoom
  if (st.size10) out.push(`font-size:${(st.size10 / 110).toFixed(3)}em`);
  // La celda es una columna flex: el contenido va abajo salvo que diga otra cosa
  if (st.valign) out.push(`justify-content:${st.valign === 'top' ? 'flex-start' : 'center'}`);
  if (st.wrap) out.push('white-space:pre-wrap');
  if (st.borderTop) out.push(`border-top:${st.borderTop}`);
  if (st.borderRight) out.push(`border-right:${st.borderRight}`);
  if (st.borderBottom) out.push(`border-bottom:${st.borderBottom}`);
  if (st.borderLeft) out.push(`border-left:${st.borderLeft}`);
  if (st.align) out.push(`text-align:${st.align}`);
  return out.join(';');
}
