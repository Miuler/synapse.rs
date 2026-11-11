// Diff de píxeles entre dos versiones de un dibujo de Excalidraw. Las dos se exportan a imagen con
// el mismo marco (la unión de los límites de ambas escenas) y la misma escala, para que cada píxel
// corresponda al mismo punto del lienzo aunque se hayan agregado o movido elementos en los bordes.
import { parseExcalidrawContent } from '@shared/obsidian-api';
import type { GitImageDiffResult } from '@shared/repositories';
import { pixelDiff } from './pixel-diff';

const PADDING = 20;
/** Lado máximo de la imagen exportada, para no reventar la memoria con dibujos enormes. */
const MAX_SIDE = 4000;

interface Scene {
  elements: any[];
  files: Record<string, any>;
}

function sceneOf(raw: string): Scene {
  if (!raw.trim()) return { elements: [], files: {} };
  const doc = parseExcalidrawContent(raw);
  return {
    elements: (doc.scene.elements || []).filter((e: any) => !e.isDeleted),
    files: doc.scene.files || {},
  };
}

export async function excalidrawPixelDiff(
  beforeRaw: string,
  afterRaw: string,
  path: string,
  isStaged: boolean,
): Promise<GitImageDiffResult> {
  const { exportToCanvas, getCommonBounds, convertToExcalidrawElements } = await import('@excalidraw/excalidraw');
  const before = sceneOf(beforeRaw);
  const after = sceneOf(afterRaw);
  const all = [...before.elements, ...after.elements];
  if (all.length === 0) throw new Error('Las dos versiones del dibujo están vacías');

  const [minX, minY, maxX, maxY] = getCommonBounds(all);
  const width = maxX - minX + PADDING * 2;
  const height = maxY - minY + PADDING * 2;
  const scale = Math.min(1, MAX_SIDE / Math.max(width, height));
  const outW = Math.max(1, Math.round(width * scale));
  const outH = Math.max(1, Math.round(height * scale));

  // Un rectángulo invisible del tamaño de la unión fija el mismo marco en las dos exportaciones
  const frame = convertToExcalidrawElements([
    {
      type: 'rectangle',
      x: minX,
      y: minY,
      width: maxX - minX,
      height: maxY - minY,
      opacity: 0,
      strokeColor: 'transparent',
      backgroundColor: 'transparent',
    } as any,
  ]);

  const render = async (scene: Scene) => {
    const canvas = await exportToCanvas({
      elements: [...frame, ...scene.elements],
      files: scene.files,
      appState: { exportBackground: true, viewBackgroundColor: '#ffffff', exportWithDarkMode: false },
      exportPadding: PADDING,
      getDimensions: () => ({ width: outW, height: outH, scale }),
    });
    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error('No se pudo leer el lienzo exportado');
    return ctx.getImageData(0, 0, outW, outH).data;
  };

  const [beforePixels, afterPixels] = await Promise.all([render(before), render(after)]);
  const { output, diffCount } = pixelDiff(beforePixels, afterPixels);

  const out = document.createElement('canvas');
  out.width = outW;
  out.height = outH;
  out.getContext('2d')!.putImageData(new ImageData(output, outW, outH), 0, 0);

  const total = outW * outH;
  return {
    path,
    is_staged: isStaged,
    diff_data_url: out.toDataURL('image/png'),
    diff_count: diffCount,
    width: outW,
    height: outH,
    total_pixels: total,
    diff_percentage: (diffCount / total) * 100,
    has_differences: diffCount > 0,
  };
}
