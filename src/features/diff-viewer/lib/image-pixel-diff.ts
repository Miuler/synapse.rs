// Diff de píxeles entre dos imágenes cualquiera (p. ej. «Comparar archivos» con dos PNG). Las dos se
// dibujan desde la esquina superior izquierda sobre un lienzo blanco del tamaño de la mayor, así
// imágenes de distinto tamaño también se comparan y lo que sobra de una cuenta como diferencia.
import type { GitImageDiffResult } from '@shared/repositories';
import { pixelDiff } from './pixel-diff';

/** Lado máximo del lienzo, para no reventar la memoria con imágenes enormes. */
const MAX_SIDE = 8000;

function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error('No se pudo cargar una de las imágenes'));
    img.src = src;
  });
}

export async function imagePixelDiff(beforeSrc: string, afterSrc: string, path: string): Promise<GitImageDiffResult> {
  const [before, after] = await Promise.all([loadImage(beforeSrc), loadImage(afterSrc)]);
  const width = Math.min(MAX_SIDE, Math.max(before.naturalWidth, after.naturalWidth, 1));
  const height = Math.min(MAX_SIDE, Math.max(before.naturalHeight, after.naturalHeight, 1));

  const pixelsOf = (img: HTMLImageElement) => {
    const canvas = document.createElement('canvas');
    canvas.width = width;
    canvas.height = height;
    const ctx = canvas.getContext('2d', { willReadFrequently: true });
    if (!ctx) throw new Error('No se pudo crear el lienzo');
    ctx.fillStyle = '#ffffff';
    ctx.fillRect(0, 0, width, height);
    ctx.drawImage(img, 0, 0);
    return ctx.getImageData(0, 0, width, height).data;
  };

  const { output, diffCount } = pixelDiff(pixelsOf(before), pixelsOf(after));
  const out = document.createElement('canvas');
  out.width = width;
  out.height = height;
  out.getContext('2d')!.putImageData(new ImageData(new Uint8ClampedArray(output), width, height), 0, 0);

  const total = width * height;
  return {
    path,
    is_staged: false,
    diff_data_url: out.toDataURL('image/png'),
    diff_count: diffCount,
    width,
    height,
    total_pixels: total,
    diff_percentage: (diffCount / total) * 100,
    has_differences: diffCount > 0,
  };
}
