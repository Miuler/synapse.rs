// Niveles de zoom de los visores de documentos (Word, hojas de cálculo): de 50 % a 200 % de 10 en 10
// y de 200 % a 300 % de 25 en 25, como en un visor de PDF.

export const VIEWER_ZOOM_LEVELS = [
  0.5, 0.6, 0.7, 0.8, 0.9, 1, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 2, 2.25, 2.5, 2.75, 3,
];

/** Nivel siguiente (`dir` = 1) o anterior (`dir` = -1) a `zoom` */
export function stepViewerZoom(zoom: number, dir: 1 | -1): number {
  const eps = 1e-6;
  if (dir > 0) return VIEWER_ZOOM_LEVELS.find((l) => l > zoom + eps) ?? VIEWER_ZOOM_LEVELS[VIEWER_ZOOM_LEVELS.length - 1];
  return [...VIEWER_ZOOM_LEVELS].reverse().find((l) => l < zoom - eps) ?? VIEWER_ZOOM_LEVELS[0];
}
