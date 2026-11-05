import { extractFrontmatterAndBody } from './frontmatter-extractor';
import { findHeadingSection, splitNavigationTarget } from './heading-anchor';
import { isNoteTarget } from './note-target';

/** Cómo localizar y leer las notas incrustadas; se inyecta para no depender de Tauri. */
export interface EmbedLoader {
  /** Ruta de la bóveda a la que apunta `[[destino]]` visto desde `baseFile`, o `null` si no existe. */
  resolvePath(target: string, baseFile?: string | null): Promise<string | null>;
  /** Contenido de una nota, o `null` si no se puede leer. */
  readContent(path: string): Promise<string | null>;
}

/** Niveles de `![[...]]` dentro de `![[...]]` que se expanden (evita ciclos y cascadas enormes). */
const MAX_EMBED_DEPTH = 3;

const EMBED_REGEX = /!\[\[([^\]\n|]+)(?:\|[^\]\n]*)?\]\]/g;
const FENCE_LINE = /^ {0,3}(```|~~~)/;

/** Indica si el texto tiene algún `![[nota#Título]]` que expandir. */
export function hasSectionEmbeds(content: string): boolean {
  return /!\[\[[^\]\n|#]+#[^\]\n|]+(?:\|[^\]\n]*)?\]\]/.test(content);
}

/** Texto de la sección de `heading` en una nota (con su encabezado), o `null` si la nota no lo tiene. */
export function extractSection(markdown: string, heading: string): string | null {
  const lines = extractFrontmatterAndBody(markdown).cleanContent.split(/\r?\n/);
  const section = findHeadingSection(lines, heading);
  if (!section) return null;
  return lines.slice(section.start, section.end).join('\n').trimEnd();
}

/**
 * Sustituye cada `![[nota#Título]]` por el contenido de esa sección: desde el título hasta el
 * siguiente encabezado del mismo nivel o superior. Los embeds de una nota, los de imágenes y
 * recursos y los que apuntan a algo que no existe (nota o título) se dejan tal cual, y los embeds
 * dentro de lo incrustado se expanden hasta `MAX_EMBED_DEPTH` niveles, sin repetir una sección
 * que ya se está expandiendo.
 */
export async function expandSectionEmbeds(
  content: string,
  loader: EmbedLoader,
  baseFile?: string | null,
  ancestors: ReadonlySet<string> = new Set(),
): Promise<string> {
  if (ancestors.size >= MAX_EMBED_DEPTH || !hasSectionEmbeds(content)) return content;

  const lines = content.split('\n');
  let inFence = false;
  for (let i = 0; i < lines.length; i++) {
    if (FENCE_LINE.test(lines[i])) {
      inFence = !inFence;
      continue;
    }
    if (inFence || !lines[i].includes('![[')) continue;

    // `replace` no admite funciones asíncronas: primero se resuelven y luego se sustituye
    const replacements = new Map<string, string>();
    for (const match of lines[i].matchAll(EMBED_REGEX)) {
      const raw = match[0];
      if (replacements.has(raw)) continue;
      const { path: target, anchor } = splitNavigationTarget(match[1]);
      if (!target || !anchor || anchor.startsWith('^') || !isNoteTarget(target)) continue;

      const embedded = await loadSection(target, anchor, loader, baseFile, ancestors);
      if (embedded !== null) replacements.set(raw, embedded);
    }
    for (const [raw, embedded] of replacements) {
      lines[i] = lines[i].split(raw).join(embedded);
    }
  }
  return lines.join('\n');
}

async function loadSection(
  target: string,
  anchor: string,
  loader: EmbedLoader,
  baseFile: string | null | undefined,
  ancestors: ReadonlySet<string>,
): Promise<string | null> {
  try {
    const path = await loader.resolvePath(target, baseFile);
    if (!path) return null;

    const key = `${path}#${anchor.toLowerCase()}`;
    if (ancestors.has(key)) return null;

    const markdown = await loader.readContent(path);
    if (markdown === null) return null;
    const section = extractSection(markdown, anchor);
    if (section === null) return null;

    const nested = await expandSectionEmbeds(section, loader, path, new Set([...ancestors, key]));
    // Líneas en blanco alrededor para que Markdown se interprete dentro del contenedor HTML
    return `\n<div class="note-embed" data-embed-source="${escapeAttribute(`${path}#${anchor}`)}">\n\n${nested}\n\n</div>\n`;
  } catch (err) {
    console.warn('No se pudo incrustar la sección', `${target}#${anchor}`, err);
    return null;
  }
}

function escapeAttribute(value: string): string {
  return value.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}
