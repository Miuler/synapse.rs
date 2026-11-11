// Diff de píxeles en memoria entre dos imágenes RGBA del mismo tamaño, con el mismo estilo que el
// diff de imágenes del backend: el fondo sin cambios atenuado y los píxeles distintos en magenta.

export interface PixelDiffResult {
  /** Imagen del diff, RGBA, del mismo tamaño que las de entrada. */
  output: Uint8ClampedArray;
  diffCount: number;
}

const DIFF_COLOR = [255, 0, 119] as const;
/** Opacidad con la que se ve el fondo sin cambios. */
const FADE_ALPHA = 0.2;
/** Diferencia mínima (0–1, en el espacio YIQ) para contar un píxel como distinto. */
const THRESHOLD = 0.1;
const MAX_YIQ_DELTA = 35215;

/** Mezcla el píxel con blanco según su alfa, como se vería sobre un fondo blanco. */
function blend(c: number, a: number): number {
  return 255 + (c - 255) * (a / 255);
}

function yiqDelta(a: Uint8ClampedArray, b: Uint8ClampedArray, i: number): number {
  const r1 = blend(a[i], a[i + 3]), g1 = blend(a[i + 1], a[i + 3]), b1 = blend(a[i + 2], a[i + 3]);
  const r2 = blend(b[i], b[i + 3]), g2 = blend(b[i + 1], b[i + 3]), b2 = blend(b[i + 2], b[i + 3]);
  const y = (r1 - r2) * 0.29889531 + (g1 - g2) * 0.58662247 + (b1 - b2) * 0.11448223;
  const ii = (r1 - r2) * 0.59597799 - (g1 - g2) * 0.2741761 - (b1 - b2) * 0.32180189;
  const q = (r1 - r2) * 0.21147017 - (g1 - g2) * 0.52261711 + (b1 - b2) * 0.31114694;
  return 0.5053 * y * y + 0.299 * ii * ii + 0.1957 * q * q;
}

/** Compara `before` y `after` (RGBA, mismo tamaño) y pinta el resultado sobre `after` atenuado. */
export function pixelDiff(before: Uint8ClampedArray, after: Uint8ClampedArray): PixelDiffResult {
  if (before.length !== after.length) throw new Error('Las imágenes deben tener el mismo tamaño');
  const output = new Uint8ClampedArray(after.length);
  const maxDelta = MAX_YIQ_DELTA * THRESHOLD * THRESHOLD;
  let diffCount = 0;
  for (let i = 0; i < after.length; i += 4) {
    if (yiqDelta(before, after, i) > maxDelta) {
      diffCount++;
      output[i] = DIFF_COLOR[0];
      output[i + 1] = DIFF_COLOR[1];
      output[i + 2] = DIFF_COLOR[2];
    } else {
      // Gris atenuado del píxel de la versión nueva
      const gray =
        blend(after[i], after[i + 3]) * 0.299 + blend(after[i + 1], after[i + 3]) * 0.587 + blend(after[i + 2], after[i + 3]) * 0.114;
      const v = 255 + (gray - 255) * FADE_ALPHA;
      output[i] = output[i + 1] = output[i + 2] = v;
    }
    output[i + 3] = 255;
  }
  return { output, diffCount };
}
