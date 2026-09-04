import { vaultRepository } from '@shared/repositories';

/**
 * Crea una nueva instancia de RegExp para buscar directivas {{#include <path>}}.
 * Usar una función constructora evita los problemas de estado (`lastIndex`) de las RegExp con bandera global.
 */
export function createIncludeRegex(): RegExp {
  return /\{\{#include\s+(?:["']?)([^"'}\n\r]+?)(?:["']?)\s*\}\}/g;
}

/**
 * Comprueba si un bloque de texto contiene al menos una directiva {{#include ...}}.
 */
export function hasIncludeDirective(code: string): boolean {
  return /\{\{#include\s+(?:["']?)([^"'}\n\r]+?)(?:["']?)\s*\}\}/.test(code);
}

/**
 * Normaliza y resuelve una ruta relativa de include con respecto a la carpeta del archivo base.
 *
 * Ejemplos:
 * - resolveRelativePath("sequences/diag.mmd", "src/diagrams_report.md") -> "src/sequences/diag.mmd"
 * - resolveRelativePath("./sequences/diag.mmd", "src/diagrams_report.md") -> "src/sequences/diag.mmd"
 * - resolveRelativePath("../diag.mmd", "src/sequences/diag.mmd") -> "src/diag.mmd"
 * - resolveRelativePath("/sequences/diag.mmd", "src/diagrams_report.md") -> "sequences/diag.mmd"
 */
export function resolveRelativePath(targetPath: string, basePath?: string | null): string {
  let cleanTarget = targetPath.trim().replace(/^['"]|['"]$/g, '').replace(/\\/g, '/');

  // Si empieza con '/', se considera relativa a la raíz de la bóveda
  if (cleanTarget.startsWith('/')) {
    return cleanTarget.replace(/^\/+/, '');
  }

  // Si no hay archivo base conocido, quitar prefijo './' y retornar
  if (!basePath) {
    return cleanTarget.replace(/^\.\//, '');
  }

  const normalizedBase = basePath.replace(/\\/g, '/');
  const lastSlash = normalizedBase.lastIndexOf('/');
  const baseDir = lastSlash >= 0 ? normalizedBase.slice(0, lastSlash) : '';

  if (cleanTarget.startsWith('./')) {
    cleanTarget = cleanTarget.slice(2);
  }

  const baseSegments = baseDir ? baseDir.split('/').filter(Boolean) : [];
  const targetSegments = cleanTarget.split('/').filter(Boolean);

  for (const seg of targetSegments) {
    if (seg === '..') {
      if (baseSegments.length > 0) {
        baseSegments.pop();
      }
    } else if (seg !== '.') {
      baseSegments.push(seg);
    }
  }

  return baseSegments.join('/');
}

/**
 * Si el archivo incluido contiene delimitadores Markdown ```mermaid ... ```,
 * los extrae para evitar anidamientos de bloques rotos.
 */
function cleanIncludedCode(content: string): string {
  let trimmed = content.trim();
  const blockMatch = trimmed.match(/^```(?:mermaid|mermair|mermai|merman)?\s*\n([\s\S]*?)\n```$/i);
  if (blockMatch) {
    return blockMatch[1].trim();
  }
  return trimmed;
}

/**
 * Preprocesa el texto sustituyendo recursivamente directivas {{#include ...}}
 * con el contenido del archivo obtenido a través de vaultRepository.
 */
export async function resolveIncludes(
  code: string,
  basePath?: string | null,
  visited = new Set<string>()
): Promise<string> {
  if (!code || typeof code !== 'string') return '';

  const regex = createIncludeRegex();
  const matches = [...code.matchAll(regex)];
  if (matches.length === 0) return code;

  let resolved = code;

  for (const match of matches) {
    const fullTag = match[0];
    const rawTarget = match[1].trim();

    const resolvedPath = resolveRelativePath(rawTarget, basePath);
    const rootPath = rawTarget.replace(/^\.\//, '').replace(/^\/+/, '');

    if (visited.has(resolvedPath) || visited.has(rootPath)) {
      throw new Error(`Ciclo de inclusión detectado: "${rawTarget}" ya ha sido incluido previamente.`);
    }

    let note = await vaultRepository.readNote(resolvedPath);
    let foundPath = resolvedPath;

    // Si no se encuentra con la ruta relativa, intentar desde la raíz de la bóveda
    if ((!note || !note.content) && rootPath !== resolvedPath) {
      const rootNote = await vaultRepository.readNote(rootPath);
      if (rootNote && rootNote.content) {
        note = rootNote;
        foundPath = rootPath;
      }
    }

    // Si aún no se encuentra y no tiene extensión .mmd, probar añadiéndola
    if ((!note || !note.content) && !resolvedPath.endsWith('.mmd')) {
      const mmdNote = await vaultRepository.readNote(`${resolvedPath}.mmd`);
      if (mmdNote && mmdNote.content) {
        note = mmdNote;
        foundPath = `${resolvedPath}.mmd`;
      }
    }

    if (note && typeof note.content === 'string') {
      const nextVisited = new Set(visited);
      nextVisited.add(foundPath);

      const cleanedContent = cleanIncludedCode(note.content);
      const expandedContent = await resolveIncludes(cleanedContent, foundPath, nextVisited);
      resolved = resolved.replace(fullTag, expandedContent);
    } else {
      throw new Error(
        `No se pudo incluir el archivo: "${rawTarget}".\nRuta buscada: "${resolvedPath}"${
          rootPath !== resolvedPath ? ` (o "${rootPath}")` : ''
        }`
      );
    }
  }

  return resolved;
}
