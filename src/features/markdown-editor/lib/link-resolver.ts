import { vaultApi } from '@entities/vault-item';
import { isTauriEnvironment } from '@shared/api';
import { hasExtension, isNoteTarget } from './note-target';

export { hasExtension, isNoteTarget };

export const IMAGE_EXTENSIONS = new Set([
  'png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp', 'svg', 'ico', 'avif', 'tiff', 'tif'
]);

/**
 * Comprueba si un nombre o ruta de archivo corresponde a un archivo de tipo imagen soportado.
 */
export function isImageExtension(pathOrName: string): boolean {
  if (!pathOrName) return false;
  const fileName = pathOrName.split('/').pop() || pathOrName;
  const lastDot = fileName.lastIndexOf('.');
  if (lastDot <= 0) return false;
  const ext = fileName.slice(lastDot + 1).toLowerCase();
  return IMAGE_EXTENSIONS.has(ext);
}

/**
 * Normaliza el objetivo del enlace:
 * - Separa el ancla `#` si existe.
 * - Si no tiene extensión, le añade automáticamente `.md`.
 */
export function normalizeTarget(rawTarget: string): { targetWithExt: string; anchor?: string } {
  const clean = rawTarget.trim().replace(/^\.?\/+/, '');
  const [filePart, anchorPart] = clean.split('#');

  if (!filePart) {
    return { targetWithExt: '', anchor: anchorPart };
  }

  const targetWithExt = hasExtension(filePart) ? filePart : `${filePart}.md`;
  return { targetWithExt, anchor: anchorPart };
}

/**
 * Verifica si el contenido incluye al menos una referencia WikiLink `[[...]]` o `![[...]]`.
 */
export function hasWikilinks(content: string): boolean {
  if (!content) return false;
  return /!?\[\[([^\]\n]+)\]\]/.test(content);
}

/**
 * Resuelve y transforma todos los WikiLinks en formato compatible para renderizado:
 * - Archivos de tipo imagen (`[[FILE.png]]` o `![[FILE.png]]`) se transforman a `![alt](<ruta>)` (renderizados como `<img>`).
 * - Archivos de documentos/notas (`[[FILE.md]]` o `[[FILE.txt]]`) se transforman a `[texto](<ruta>)` (renderizados como `<a>`).
 *
 * Las rutas completas se extraen directamente del DashMap de Rust en memoria.
 * Si el objetivo no incluye extensión, se autocompleta con `.md` para buscarlo en el DashMap.
 */
export async function resolveMarkdownWikilinks(content: string, baseFile?: string | null): Promise<string> {
  if (!content || !hasWikilinks(content)) {
    return content;
  }

  // Si Tauri está conectado, el backend Rust realiza la resolución directa y ultrarrápida en el DashMap
  if (isTauriEnvironment()) {
    try {
      const transformed = await vaultApi.renderMarkdownWikilinks(content, baseFile);
      if (typeof transformed === 'string') {
        return transformed;
      }
    } catch (err) {
      console.warn('Error resolviendo wikilinks mediante backend Rust:', err);
    }
  }

  // Fallback para entornos de desarrollo desconectados o pruebas unitarias
  return content.replace(/(!?)\[\[([^\]\n]+)\]\]/g, (match, isEmbedPrefix, inner) => {
    const trimmed = inner.trim();
    if (!trimmed) return match;

    const [targetPart, aliasPart] = trimmed.split('|');
    const targetClean = targetPart.trim();
    const aliasClean = aliasPart ? aliasPart.trim() : null;

    const { targetWithExt, anchor } = normalizeTarget(targetClean);
    const displayText = aliasClean || targetClean;
    const finalPath = anchor ? `${targetWithExt}#${anchor}` : targetWithExt;

    // Un embed de otra nota (`![[nota#Título]]`) es un enlace navegable, no una imagen
    const isImage = isImageExtension(targetClean) || (isEmbedPrefix === '!' && !isNoteTarget(targetClean));
    const prefix = isImage ? '!' : '';

    return finalPath.includes(' ')
      ? `${prefix}[${displayText}](<${finalPath}>)`
      : `${prefix}[${displayText}](${finalPath})`;
  });
}

export interface WikilinkToken {
  from: number;
  to: number;
  raw: string;
  target: string;
  alias?: string;
  anchor?: string;
  displayText: string;
  isImage: boolean;
  /** `![[...]]`: transclusión; si apunta a una nota se muestra como enlace navegable. */
  isEmbed: boolean;
}

/**
 * Escanea una línea en busca de WikiLinks `[[...]]` y embeds `![[...]]` para decoraciones en Live Preview.
 */
export function extractWikilinkTokens(lineText: string, lineOffset = 0): WikilinkToken[] {
  const tokens: WikilinkToken[] = [];
  const regex = /(!?)\[\[([^\]\n]+)\]\]/g;
  for (const match of lineText.matchAll(regex)) {
    if (match.index === undefined) continue;
    const raw = match[0];
    const isEmbed = match[1] === '!';
    const inner = match[2]?.trim();
    if (!inner) continue;

    const [targetPart, aliasPart] = inner.split('|');
    const targetClean = targetPart.trim();
    const aliasClean = aliasPart ? aliasPart.trim() : undefined;
    const [filePart, anchorPart] = targetClean.split('#');

    const displayText = aliasClean || targetClean;
    const isImage = isImageExtension(filePart.trim());

    tokens.push({
      from: lineOffset + match.index,
      to: lineOffset + match.index + raw.length,
      raw,
      target: filePart.trim(),
      alias: aliasClean,
      anchor: anchorPart ? anchorPart.trim() : undefined,
      displayText,
      isImage,
      isEmbed,
    });
  }
  return tokens;
}
