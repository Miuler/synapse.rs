import { parseYaml } from '@shared/obsidian-api/utils';

export interface ExtractedFrontmatter {
  hasFrontmatter: boolean;
  metadata: Record<string, any>;
  cleanContent: string;
}

/**
 * Separa el bloque de frontmatter YAML inicial (--- ... ---) del cuerpo de Markdown
 * y parsea los metadatos en un diccionario de clave-valor.
 */
export function extractFrontmatterAndBody(markdown: string): ExtractedFrontmatter {
  if (!markdown || typeof markdown !== 'string') {
    return { hasFrontmatter: false, metadata: {}, cleanContent: '' };
  }

  // Coincide con bloque frontmatter al inicio del archivo (incluyendo frontmatter vacío)
  const match = markdown.match(/^---\r?\n([\s\S]*?)(?:\r?\n)?---(?:\r?\n|$)/);
  if (!match) {
    return { hasFrontmatter: false, metadata: {}, cleanContent: markdown };
  }

  const rawYaml = match[1];
  const cleanContent = markdown.slice(match[0].length);

  let metadata: Record<string, any> = {};
  try {
    const parsed = parseYaml(rawYaml);
    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
      metadata = parsed;
    }
  } catch (err) {
    console.warn('Error al parsear frontmatter en extractFrontmatterAndBody:', err);
  }

  return {
    hasFrontmatter: true,
    metadata,
    cleanContent,
  };
}
