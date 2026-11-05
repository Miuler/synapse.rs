/** Filtro `kind:xxx` de Quick Open (Ctrl+O): limita la lista al tipo de archivo indicado. */

/** Subconjunto de `fileTypesManager` que necesita la clasificación. */
export interface KindClassifier {
  isPdfFile(path: string): boolean;
  isWordFile(path: string): boolean;
  isExcelFile(path: string): boolean;
  isDrawingFile(path: string): boolean;
  isMarkdownFile(path: string): boolean;
  isDiagramFile(path: string): boolean;
  isCodeFile(path: string): boolean;
  isImageFile(path: string): boolean;
}

export interface ParsedKindQuery {
  /** Consulta sin los tokens `kind:`. */
  text: string;
  /** Valores de los tokens `kind:` en minúsculas. */
  kinds: string[];
}

/** Separa los tokens `kind:xxx` (sin distinguir mayúsculas) del resto de la consulta. */
export function parseKindFilter(query: string): ParsedKindQuery {
  const kinds: string[] = [];
  const rest: string[] = [];
  for (const token of query.split(/\s+/).filter(Boolean)) {
    const match = /^kind:(.+)$/i.exec(token);
    if (match) kinds.push(match[1].toLowerCase());
    else rest.push(token);
  }
  return { text: rest.join(' '), kinds };
}

/** Tipo de un archivo según su ruta; mismos nombres que el filtro `kind:` del índice y del backend. */
export function fileKindName(path: string, types: KindClassifier): string | null {
  if (types.isPdfFile(path)) return 'pdf';
  if (types.isWordFile(path)) return 'word';
  if (types.isExcelFile(path)) return 'excel';
  if (types.isDrawingFile(path)) return 'excalidraw';
  if (types.isMarkdownFile(path)) return 'markdown';
  if (types.isDiagramFile(path)) return 'mermaid';
  if (types.isCodeFile(path)) return 'code';
  if (types.isImageFile(path)) return 'image';
  return null;
}

/** `true` si no hay filtros o el tipo del archivo empieza por alguno (`kind:pd` ya encuentra los PDF). */
export function matchesKindFilter(path: string, kinds: string[], types: KindClassifier): boolean {
  if (kinds.length === 0) return true;
  const kind = fileKindName(path, types);
  return kind !== null && kinds.some((k) => kind.startsWith(k));
}
