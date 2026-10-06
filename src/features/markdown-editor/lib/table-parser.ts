import type { Text } from '@codemirror/state';

export type ColumnAlignment = 'left' | 'center' | 'right' | 'none';

export interface TableData {
  headers: string[];
  alignments: ColumnAlignment[];
  rows: string[][];
}

export interface MarkdownTableBlock {
  from: number;
  to: number;
  table: TableData;
  rawText: string;
}

/**
 * Divide una fila de tabla Markdown respetando los caracteres de escape `\|`.
 */
export function splitTableRow(rowStr: string): string[] {
  let str = rowStr.trim();
  if (str.startsWith('|')) {
    str = str.slice(1);
  }
  if (str.endsWith('|') && !str.endsWith('\\|')) {
    str = str.slice(0, -1);
  }

  const cells: string[] = [];
  let current = '';
  let escaped = false;

  for (let i = 0; i < str.length; i++) {
    const char = str[i];
    if (char === '\\' && !escaped) {
      escaped = true;
      current += char;
    } else if (char === '|' && !escaped) {
      cells.push(current.trim());
      current = '';
    } else {
      escaped = false;
      current += char;
    }
  }
  cells.push(current.trim());
  return cells;
}

/**
 * Valida y extrae las alineaciones de una línea divisoria Markdown (| --- | :---: | ---: |).
 */
export function parseDelimiterRow(rowStr: string): ColumnAlignment[] | null {
  const trimmedRow = rowStr.trim();
  if (!trimmedRow.includes('-')) return null;

  const cells = splitTableRow(rowStr);
  if (cells.length === 0) return null;

  const alignments: ColumnAlignment[] = [];
  for (const cell of cells) {
    const trimmed = cell.trim();
    // Debe tener al menos un guión y opcionalmente dos puntos al inicio y/o final (:?-+:?)
    if (!/^:?-+:?$/.test(trimmed)) {
      return null;
    }
    const left = trimmed.startsWith(':');
    const right = trimmed.endsWith(':');
    if (left && right) {
      alignments.push('center');
    } else if (left) {
      alignments.push('left');
    } else if (right) {
      alignments.push('right');
    } else {
      alignments.push('none');
    }
  }
  return alignments;
}

/**
 * Parsea un bloque de texto Markdown completo que representa una tabla GFM.
 */
export function parseMarkdownTableString(rawMarkdown: string): TableData | null {
  const lines = rawMarkdown.trim().split(/\r?\n/).filter((l) => l.trim().length > 0);
  if (lines.length < 2) return null;

  const headerCells = splitTableRow(lines[0]);
  const alignments = parseDelimiterRow(lines[1]);
  if (!alignments) return null;

  const colCount = Math.max(headerCells.length, alignments.length, 1);

  while (headerCells.length < colCount) {
    headerCells.push(`Columna ${headerCells.length + 1}`);
  }
  while (alignments.length < colCount) {
    alignments.push('none');
  }

  const rows: string[][] = [];
  for (let i = 2; i < lines.length; i++) {
    const rowCells = splitTableRow(lines[i]);
    while (rowCells.length < colCount) {
      rowCells.push('');
    }
    rows.push(rowCells.slice(0, colCount));
  }

  return {
    headers: headerCells,
    alignments,
    rows,
  };
}

/**
 * Serializa los datos de la tabla de vuelta a formato Markdown GFM limpio.
 */
export function serializeMarkdownTable(table: TableData): string {
  const colCount = Math.max(
    table.headers.length,
    table.alignments.length,
    ...table.rows.map((r) => r.length),
    1
  );

  const safeHeaders: string[] = [];
  for (let c = 0; c < colCount; c++) {
    const h = (table.headers[c] || '').replace(/(?<!\\)\|/g, '\\|').trim();
    safeHeaders.push(h || `Columna ${c + 1}`);
  }

  const safeAlignments: ColumnAlignment[] = [];
  for (let c = 0; c < colCount; c++) {
    safeAlignments.push(table.alignments[c] || 'none');
  }

  const delimiterCells = safeAlignments.map((align) => {
    switch (align) {
      case 'left':
        return ':---';
      case 'center':
        return ':---:';
      case 'right':
        return '---:';
      case 'none':
      default:
        return '---';
    }
  });

  const safeRows = table.rows.map((row) => {
    const cells: string[] = [];
    for (let c = 0; c < colCount; c++) {
      const cell = (row[c] || '').replace(/(?<!\\)\|/g, '\\|').trim();
      cells.push(cell);
    }
    return `| ${cells.join(' | ')} |`;
  });

  const headerLine = `| ${safeHeaders.join(' | ')} |`;
  const delimiterLine = `| ${delimiterCells.join(' | ')} |`;

  return [headerLine, delimiterLine, ...safeRows].join('\n');
}

/**
 * Escanea el documento de CodeMirror para encontrar todos los bloques de tablas Markdown.
 */
export function findTableBlocks(doc: Text): MarkdownTableBlock[] {
  const blocks: MarkdownTableBlock[] = [];
  const lineCount = doc.lines;
  let inCodeBlock = false;

  for (let i = 1; i <= lineCount; i++) {
    const line = doc.line(i);
    const trimmed = line.text.trim();

    // Ignorar bloques de código cercados (``` o ~~~)
    if (/^```|^~~~/.test(trimmed)) {
      inCodeBlock = !inCodeBlock;
      continue;
    }
    if (inCodeBlock) continue;

    // Si la línea no tiene pipes, no puede ser encabezado de tabla
    if (!trimmed || !trimmed.includes('|')) continue;

    // Verificar si la siguiente línea es un delimitador de tabla
    if (i + 1 <= lineCount) {
      const nextLine = doc.line(i + 1);
      const nextTrimmed = nextLine.text.trim();
      const alignments = parseDelimiterRow(nextTrimmed);

      if (alignments) {
        const headerCells = splitTableRow(trimmed);
        if (headerCells.length > 0 && alignments.length > 0) {
          const from = line.from;
          let to = nextLine.to;
          const tableLines: string[] = [line.text, nextLine.text];

          const headerHasOuterPipes = trimmed.startsWith('|') || trimmed.endsWith('|');
          let j = i + 2;
          while (j <= lineCount) {
            const rowLine = doc.line(j);
            const rowTrimmed = rowLine.text.trim();
            // Una fila de tabla debe tener pipes y no estar vacía ni ser código/encabezado/lista/cita
            if (
              !rowTrimmed ||
              !rowTrimmed.includes('|') ||
              /^```|^~~~|^#{1,6}\s|^>\s|^[-*+]\s|^\d+\.\s/.test(rowTrimmed)
            ) {
              break;
            }

            // Si el encabezado usa pipes exteriores, las filas de la tabla también deben tener pipes exteriores
            if (headerHasOuterPipes && !rowTrimmed.startsWith('|') && !rowTrimmed.endsWith('|')) {
              break;
            }

            tableLines.push(rowLine.text);
            to = rowLine.to;
            j++;
          }

          const rawText = tableLines.join('\n');
          const parsed = parseMarkdownTableString(rawText);
          if (parsed) {
            blocks.push({
              from,
              to,
              table: parsed,
              rawText,
            });
          }

          // Avanzar el índice después de la tabla
          i = j - 1;
        }
      }
    }
  }

  return blocks;
}
