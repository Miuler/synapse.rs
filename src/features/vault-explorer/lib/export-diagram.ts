import { vaultRepository } from '@shared/repositories';
import { parseExcalidrawContent } from '@shared/obsidian-api';
import { renderD2Svg, renderMermaidJsSvg } from '@shared/lib/diagrams';
import { isD2File, isDrawingFile, isMermaidFile } from '@entities/file-type';

export type DiagramExportFormat = 'png' | 'svg';

/** Escala del PNG respecto al tamaño del SVG, para que no salga borroso. */
const PNG_SCALE = 2;
const EXPORT_PADDING = 20;

const DIAGRAM_EXTENSIONS = ['.excalidraw.excalidraw', '.excalidraw.json', '.excalidraw.md', '.excalidraw', '.mermaid', '.mmd', '.d2'];

/** Si el archivo es un diagrama que se puede exportar (Excalidraw, Mermaid o D2). */
export function isExportableDiagram(path: string): boolean {
  return isDrawingFile(path) || isMermaidFile(path) || isD2File(path);
}

/** Ruta de la exportación: junto al diagrama, con su nombre y la extensión del formato. */
export function exportPathFor(path: string, format: DiagramExportFormat): string {
  const lower = path.toLowerCase();
  const ext = DIAGRAM_EXTENSIONS.find((e) => lower.endsWith(e));
  const dot = path.lastIndexOf('.');
  const base = ext ? path.slice(0, path.length - ext.length) : dot > path.lastIndexOf('/') ? path.slice(0, dot) : path;
  return `${base}.${format}`;
}

/**
 * Exporta el diagrama a PNG o SVG en la bóveda, junto al archivo, con el tema actual de la app.
 * Devuelve la ruta relativa del archivo creado.
 */
export async function exportDiagram(path: string, format: DiagramExportFormat): Promise<string> {
  const note = await vaultRepository.readNote(path);
  if (!note?.content?.trim()) throw new Error('El diagrama está vacío');
  const isDark = document.documentElement.getAttribute('data-theme') === 'dark';

  const bytes = isDrawingFile(path)
    ? await exportExcalidraw(note.content, format, isDark)
    : await exportCodeDiagram(path, note.content, format);

  const target = exportPathFor(path, format);
  await vaultRepository.writeBinary(target, bytes);
  return target;
}

async function exportExcalidraw(content: string, format: DiagramExportFormat, isDark: boolean): Promise<Uint8Array> {
  const { scene } = parseExcalidrawContent(content);
  const elements = scene.elements ?? [];
  if (!elements.length) throw new Error('El dibujo no tiene elementos');
  const { exportToSvg, exportToBlob } = await import('@excalidraw/excalidraw');
  const options = {
    elements,
    appState: {
      ...(scene.appState ?? {}),
      exportWithDarkMode: isDark,
      exportBackground: true,
      viewBackgroundColor: isDark ? '#121212' : '#ffffff',
      exportScale: PNG_SCALE,
    },
    files: scene.files ?? {},
    exportPadding: EXPORT_PADDING,
  };
  if (format === 'svg') {
    const svg = await exportToSvg(options);
    return new TextEncoder().encode(new XMLSerializer().serializeToString(svg));
  }
  const blob = await exportToBlob({ ...options, mimeType: 'image/png' });
  return new Uint8Array(await blob.arrayBuffer());
}

async function exportCodeDiagram(path: string, code: string, format: DiagramExportFormat): Promise<Uint8Array> {
  let result;
  if (isD2File(path)) {
    result = await renderD2Svg(code);
  } else {
    // Las etiquetas HTML de Mermaid (foreignObject) ensucian el canvas y WebKit no deja leer el PNG.
    const source = format === 'png' ? `%%{init: {"htmlLabels": false, "flowchart": {"htmlLabels": false}}}%%\n${code}` : code;
    result = await renderMermaidJsSvg(source, 'diagram-export');
  }
  if (!result.svg) throw new Error(result.error ?? 'No se pudo renderizar el diagrama');
  if (format === 'svg') return new TextEncoder().encode(result.svg);
  return svgToPng(result.svg);
}

/** Rasteriza un SVG a PNG dibujándolo en un canvas, con fondo del tema. */
async function svgToPng(svgText: string): Promise<Uint8Array> {
  const doc = new DOMParser().parseFromString(svgText, 'image/svg+xml');
  const svg = doc.documentElement;
  const { width, height } = svgSize(svg);
  // Tamaño explícito: sin él WebKit dibuja los SVG con width="100%" a 300×150.
  svg.setAttribute('width', String(width));
  svg.setAttribute('height', String(height));
  if (!svg.getAttribute('xmlns')) svg.setAttribute('xmlns', 'http://www.w3.org/2000/svg');
  const data = new XMLSerializer().serializeToString(svg);

  const image = new Image();
  image.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(data)}`;
  await image.decode();

  const canvas = document.createElement('canvas');
  canvas.width = Math.ceil(width * PNG_SCALE);
  canvas.height = Math.ceil(height * PNG_SCALE);
  const ctx = canvas.getContext('2d');
  if (!ctx) throw new Error('No se pudo crear el canvas');
  ctx.fillStyle = getComputedStyle(document.documentElement).getPropertyValue('--bg-primary').trim() || '#ffffff';
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  ctx.scale(PNG_SCALE, PNG_SCALE);
  ctx.drawImage(image, 0, 0, width, height);

  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/png'));
  if (!blob) throw new Error('No se pudo generar el PNG');
  return new Uint8Array(await blob.arrayBuffer());
}

function svgSize(svg: Element): { width: number; height: number } {
  const viewBox = svg.getAttribute('viewBox')?.split(/[\s,]+/).map(Number);
  if (viewBox?.length === 4 && viewBox[2] > 0 && viewBox[3] > 0) return { width: viewBox[2], height: viewBox[3] };
  const width = parseFloat(svg.getAttribute('width') ?? '');
  const height = parseFloat(svg.getAttribute('height') ?? '');
  return { width: width > 0 ? width : 800, height: height > 0 ? height : 600 };
}
