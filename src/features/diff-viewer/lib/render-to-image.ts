// Convierte una versión de cualquier archivo que Synapse sabe dibujar (imagen, Excalidraw, diagrama de
// texto o draw.io) en una imagen (data URL), para compararla píxel a píxel con otra de otro tipo.
import { parseExcalidrawContent } from '@shared/obsidian-api';
import { isDiagramFile, isDrawingFile, isDrawioFile, isImageFile, isPdfFile } from '@entities/file-type';
import { renderDrawioPng } from '@features/drawio-editor';
import { loadSide, type DiffSource } from './diff-sides';
import { renderSvg, svgDataUrl } from './diagram-pixel-diff';
import { renderKindFrom, type RenderKind } from './render-kind';
import { t } from '@shared/lib/i18n';

export function renderKindOf(path: string): RenderKind | null {
  return renderKindFrom({
    image: isImageFile(path),
    drawing: isDrawingFile(path),
    diagram: isDiagramFile(path),
    drawio: isDrawioFile(path),
    pdf: isPdfFile(path),
  });
}

async function excalidrawDataUrl(raw: string): Promise<string> {
  const { scene } = parseExcalidrawContent(raw);
  const elements = (scene.elements ?? []).filter((e: any) => !e.isDeleted);
  if (!elements.length) throw new Error(t('diffViewer.errors.drawingEmpty'));
  const { exportToCanvas } = await import('@excalidraw/excalidraw');
  const canvas = await exportToCanvas({
    elements,
    files: scene.files ?? {},
    appState: { exportBackground: true, viewBackgroundColor: '#ffffff', exportWithDarkMode: false },
    exportPadding: 20,
  });
  return canvas.toDataURL('image/png');
}

/** La versión como imagen, o `null` si no existe en ese lado. */
export async function renderSideToImage(source: DiffSource): Promise<string | null> {
  const kind = renderKindOf(source.path);
  if (!kind) throw new Error(t('diffViewer.errors.cannotConvertImage', { path: source.path }));
  const side = await loadSide(source, kind === 'image');
  if (!side.exists) return null;
  if (kind === 'image') {
    if (!side.dataUrl) throw new Error(t('diffViewer.errors.cannotReadImage'));
    return side.dataUrl;
  }
  const text = side.content ?? '';
  if (kind === 'drawing') return excalidrawDataUrl(text);
  if (kind === 'drawio') return renderDrawioPng(text);
  return svgDataUrl(await renderSvg(source.path, text));
}
