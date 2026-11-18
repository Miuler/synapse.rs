// Qué archivos se pueden convertir en imagen para compararlos píxel a píxel, y cuándo hace falta
// renderizar los dos lados por separado (tipos distintos o draw.io) en vez del diff propio del tipo.

/** Cómo se convierte un archivo en imagen. */
export type RenderKind = 'image' | 'drawing' | 'diagram' | 'drawio';

export interface FileKindFlags {
  /** Imagen (PNG, JPG, SVG, WebP…; también `.drawio.png` / `.drawio.svg`). */
  image: boolean;
  /** Dibujo de Excalidraw. */
  drawing: boolean;
  /** Diagrama de texto (Mermaid, D2, PlantUML). */
  diagram: boolean;
  /** Diagrama draw.io en XML (`.drawio`). */
  drawio: boolean;
  pdf: boolean;
}

export function renderKindFrom(f: FileKindFlags): RenderKind | null {
  if (f.drawing) return 'drawing';
  if (f.drawio) return 'drawio';
  if (f.diagram) return 'diagram';
  if (f.pdf) return null;
  return f.image ? 'image' : null;
}

/**
 * Si el diff de píxeles se hace renderizando cada lado a imagen: cuando los dos se pueden dibujar y
 * son de tipos distintos (imagen contra Excalidraw, Mermaid contra D2…) o son draw.io, que no tiene
 * visor propio en el diff.
 */
export function needsRenderedPixelDiff(before: RenderKind | null, after: RenderKind | null, sameKind: boolean): boolean {
  if (!before || !after) return false;
  return !sameKind || before === 'drawio' || after === 'drawio';
}
