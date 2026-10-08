import { vaultRepository } from '@shared/repositories';
import { resolveRelativePath } from './include-resolver';

export interface ImageToken {
  from: number;
  to: number;
  raw: string;
  src: string;
  alt: string;
  title?: string;
  width?: string;
  height?: string;
}

const imageUrlCache = new Map<string, string>();

/**
 * Parsea especificaciones de tamaño comunes en Markdown / Obsidian, ej:
 * "Mi foto|300" -> width: "300px"
 * "Mi foto|300x200" -> width: "300px", height: "200px"
 */
export function parseImageDimensions(altText: string): { cleanAlt: string; width?: string; height?: string } {
  const pipeIdx = altText.lastIndexOf('|');
  if (pipeIdx === -1) {
    return { cleanAlt: altText };
  }

  const dimPart = altText.slice(pipeIdx + 1).trim();
  const cleanAlt = altText.slice(0, pipeIdx).trim();

  const dimMatch = dimPart.match(/^(\d+)(?:x(\d+))?$/);
  if (dimMatch) {
    return {
      cleanAlt,
      width: `${dimMatch[1]}px`,
      height: dimMatch[2] ? `${dimMatch[2]}px` : undefined,
    };
  }

  return { cleanAlt: altText };
}

/**
 * Escanea un texto o línea buscando imágenes en formato Markdown estándar `![alt](url)`
 * y formato transclusión WikiLink `![[file]]`.
 */
export function extractImageTokens(lineText: string, lineOffset = 0): ImageToken[] {
  const tokens: ImageToken[] = [];

  // 1. Formato Markdown estándar: ![alt](src "title")
  const mdRegex = /!\[([^\]]*)\]\(([^)\s]+)(?:\s+["']([^"']*)["'])?\)/g;
  for (const match of lineText.matchAll(mdRegex)) {
    if (match.index === undefined) continue;
    const raw = match[0];
    const rawAlt = match[1] || '';
    const src = match[2] || '';
    const title = match[3];

    const { cleanAlt, width, height } = parseImageDimensions(rawAlt);

    tokens.push({
      from: lineOffset + match.index,
      to: lineOffset + match.index + raw.length,
      raw,
      src,
      alt: cleanAlt,
      title,
      width,
      height,
    });
  }

  // 2. Formato WikiLink: ![[path|alt]] o [[path|alt]] (para archivos de imagen)
  const wikiRegex = /!?\[\[([^\]|]+)(?:\|([^\]]*))?\]\]/g;
  for (const match of lineText.matchAll(wikiRegex)) {
    if (match.index === undefined) continue;
    const raw = match[0];
    const src = match[1]?.trim() || '';
    const rawAlt = match[2]?.trim() || '';

    const filePart = src.split('#')[0].trim();

    // Solo considerar imágenes comunes si es sintaxis wiki
    const isImageExt = /\.(?:png|jpe?g|webp|gif|svg|bmp|ico|avif|tiff?)$/i.test(filePart);
    if (!isImageExt) continue;

    const { cleanAlt, width, height } = parseImageDimensions(rawAlt || filePart);

    tokens.push({
      from: lineOffset + match.index,
      to: lineOffset + match.index + raw.length,
      raw,
      src,
      alt: cleanAlt || filePart,
      width,
      height,
    });
  }

  return tokens;
}

/**
 * Resuelve una ruta de imagen (local relativa a la bóveda o remota) a una URL válida
 * para ser consumida de forma segura por el WebView.
 */
export async function resolveVaultImageUrl(rawSrc: string, basePath?: string | null): Promise<string> {
  let cleanSrc = rawSrc.trim().replace(/^['"]|['"]$/g, '');
  if (!cleanSrc) return '';

  // Quitar posible ancla (#seccion) para la carga física de la imagen
  cleanSrc = cleanSrc.split('#')[0].trim();

  // Decodificar posibles caracteres escapados (por ejemplo %20 para espacios)
  try {
    cleanSrc = decodeURIComponent(cleanSrc);
  } catch {
    // Mantener original si la decodificación falla
  }

  // Protocolos remotos o URLs ya preparadas
  if (/^(?:https?:\/\/|data:|blob:|asset:\/\/)/i.test(cleanSrc)) {
    return cleanSrc;
  }

  const cacheKey = `${basePath ?? ''}::${cleanSrc}`;
  if (imageUrlCache.has(cacheKey)) {
    return imageUrlCache.get(cacheKey)!;
  }

  // 1. Resolver ruta a través del backend Rust (Data URL directa o DashMap)
  if (vaultRepository.isConnected()) {
    try {
      const dataUrl = await vaultRepository.readAssetDataUrl(cleanSrc, basePath);
      if (dataUrl) {
        imageUrlCache.set(cacheKey, dataUrl);
        return dataUrl;
      }
      const absPath = await vaultRepository.resolveAssetFilePath(cleanSrc, basePath);
      if (absPath) {
        const url = vaultRepository.resolveAssetUrl(absPath);
        imageUrlCache.set(cacheKey, url);
        return url;
      }
    } catch (err) {
      console.warn(`Error resolviendo ruta para imagen "${cleanSrc}":`, err);
    }
  }

  // 2. Fallback para entorno desconectado o pruebas
  const resolvedPath = resolveRelativePath(cleanSrc, basePath);
  const fallbackUrl = vaultRepository.resolveAssetUrl(resolvedPath);
  return fallbackUrl;
}
