// Forma de los comandos `spreadsheet_*` (dominio `spreadsheet` en Rust, en camelCase)
export interface CellStyle {
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
  strike?: boolean;
  color?: string;
  bg?: string;
  /** Tamaño en puntos ×10 */
  size10?: number;
  align?: 'left' | 'center' | 'right' | 'justify';
  valign?: 'top' | 'middle';
  wrap?: boolean;
  borderTop?: string;
  borderRight?: string;
  borderBottom?: string;
  borderLeft?: string;
}

/** Celda que contiene lo buscado */
export interface SheetMatch {
  sheet: number;
  r: number;
  c: number;
}
export interface SheetMatches {
  matches: SheetMatch[];
  /** Hay más de las que se devuelven */
  more: boolean;
}

export interface Cell {
  r: number;
  c: number;
  /** Valor formateado (el resultado guardado si es una fórmula) */
  v: string;
  /** Fórmula sin «=» */
  f?: string;
  /** Es un número o una fecha */
  n?: boolean;
  /** Índice en `CellBlock.styles` */
  s?: number;
}

export interface SheetInfo {
  name: string;
  hidden: boolean;
}

export interface WorkbookInfo {
  sheets: SheetInfo[];
  /** Sin estilos (formatos leídos con calamine) */
  valuesOnly: boolean;
  /** Sin estilos por el tamaño del archivo */
  large: boolean;
}

export interface Merge {
  r1: number;
  c1: number;
  r2: number;
  c2: number;
}

export interface SheetLayout {
  rows: number;
  cols: number;
  cellCount: number;
  tabColor?: string | null;
  merges: Merge[];
  colWidths: [number, number][];
  rowHeights: [number, number][];
  hiddenCols: number[];
  hiddenRows: number[];
}

export interface CellBlock {
  cells: Cell[];
  styles: CellStyle[];
}
