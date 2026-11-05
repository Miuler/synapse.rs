/** Clasificación de los destinos de enlaces `[[...]]` (sin dependencias de la app). */

/**
 * Comprueba si un texto o nombre de archivo tiene una extensión válida (ej: .txt, .md, .png).
 */
export function hasExtension(nameOrPath: string): boolean {
  const fileName = nameOrPath.split('/').pop() || nameOrPath;
  const lastDot = fileName.lastIndexOf('.');
  if (lastDot <= 0) return false;
  const ext = fileName.slice(lastDot + 1);
  return ext.length > 0 && ext.length <= 10 && !ext.includes(' ') && !ext.includes('/') && !ext.includes('\\');
}

/**
 * Indica si el objetivo de un enlace es una nota Markdown (sin extensión o `.md`/`.markdown`),
 * a diferencia de una imagen, un PDF u otro recurso.
 */
export function isNoteTarget(target: string): boolean {
  const file = target.split('#')[0].trim();
  if (!hasExtension(file)) return true;
  return /\.(?:md|markdown)$/i.test(file);
}
