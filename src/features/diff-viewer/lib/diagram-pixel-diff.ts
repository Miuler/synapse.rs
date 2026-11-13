// Diff de píxeles entre dos versiones de un diagrama (Mermaid, D2 o PlantUML). Cada versión se
// renderiza a SVG, se convierte en imagen en memoria y se comparan con el mismo diff que las imágenes.
// No se escribe nada en disco.
import { renderD2Svg, renderMermaidJsSvg, renderPlantUmlSvg } from '@shared/lib/diagrams';
import type { GitImageDiffResult } from '@shared/repositories';
import { appSettings } from '@entities/settings';
import { isD2File, isPlantUmlFile } from '@entities/file-type';
import { imagePixelDiff } from './image-pixel-diff';

async function renderSvg(path: string, code: string): Promise<string> {
  if (!code.trim()) throw new Error('Una de las versiones del diagrama está vacía');
  let result: { svg: string | null; error: string | null };
  if (isD2File(path)) {
    result = await renderD2Svg(code);
  } else if (isPlantUmlFile(path)) {
    result = await renderPlantUmlSvg(code, appSettings.settings.plantumlServer);
  } else {
    // Sin etiquetas HTML (foreignObject): ensucian el canvas y WebKit no deja leer sus píxeles
    const source = `%%{init: {"htmlLabels": false, "flowchart": {"htmlLabels": false}}}%%\n${code}`;
    result = await renderMermaidJsSvg(source, `diff-${Math.random().toString(36).slice(2)}`);
  }
  if (!result.svg) throw new Error(result.error ?? 'No se pudo renderizar el diagrama');
  return result.svg;
}

/** SVG como data URL con tamaño explícito: sin él WebKit lo dibuja a 300×150. */
function svgDataUrl(svgText: string): string {
  const svg = new DOMParser().parseFromString(svgText, 'image/svg+xml').documentElement;
  const viewBox = svg.getAttribute('viewBox')?.split(/[\s,]+/).map(Number);
  let width = parseFloat(svg.getAttribute('width') ?? '');
  let height = parseFloat(svg.getAttribute('height') ?? '');
  if (viewBox?.length === 4 && viewBox[2] > 0 && viewBox[3] > 0) {
    width = viewBox[2];
    height = viewBox[3];
  }
  svg.setAttribute('width', String(Math.ceil(width || 800)));
  svg.setAttribute('height', String(Math.ceil(height || 600)));
  if (!svg.getAttribute('xmlns')) svg.setAttribute('xmlns', 'http://www.w3.org/2000/svg');
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(new XMLSerializer().serializeToString(svg))}`;
}

export async function diagramPixelDiff(beforeCode: string, afterCode: string, path: string): Promise<GitImageDiffResult> {
  const [before, after] = await Promise.all([renderSvg(path, beforeCode), renderSvg(path, afterCode)]);
  return imagePixelDiff(svgDataUrl(before), svgDataUrl(after), path);
}
