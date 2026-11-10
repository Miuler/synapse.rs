/** Anclas a encabezados de una nota: `[[nota#Título]]`, `![[nota#Título]]` y `[texto](nota.md#Título)`. */

/** Petición de ir a un encabezado; el `id` distingue pulsar dos veces el mismo enlace. */
export interface HeadingTarget {
  text: string;
  id: number;
}

export interface NavigationTarget {
  path: string;
  anchor?: string;
}

/**
 * Separa `ruta#ancla`. Con anclas encadenadas (`nota#Título#Subtítulo`) vale la última,
 * como en Obsidian. `#etiqueta` devuelve ruta vacía.
 */
export function splitNavigationTarget(target: string): NavigationTarget {
  const parts = target.split('#');
  const path = parts[0].trim();
  const anchor = parts.length > 1 ? parts[parts.length - 1].trim() : '';
  return anchor ? { path, anchor } : { path };
}

/** Texto de un encabezado sin marcado ni mayúsculas, para comparar el ancla con el título. */
export function normalizeHeadingText(text: string): string {
  return text
    .replace(/!?\[\[([^\]|]*)(?:\|([^\]]*))?\]\]/g, (_, target: string, alias?: string) => alias ?? target)
    .replace(/!?\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/[`*_~=]/g, '')
    .replace(/\s+/g, ' ')
    .trim()
    .toLowerCase()
    .normalize('NFC');
}

const HEADING_LINE = /^ {0,3}(#{1,6})[ \t]+(.*?)(?:[ \t]+#+)?[ \t]*$/;
const FENCE_LINE = /^ {0,3}(```|~~~)/;

/**
 * Índice (base 0) de la primera línea que es un encabezado con ese texto, o -1.
 * Ignora el frontmatter y los bloques de código, donde un `# comentario` no es un título.
 */
export function findHeadingLine(lines: Iterable<string>, heading: string): number {
  const wanted = normalizeHeadingText(heading);
  if (!wanted) return -1;

  let index = 0;
  let inFence = false;
  let inFrontmatter = false;
  for (const line of lines) {
    if (index === 0 && line.trim() === '---') {
      inFrontmatter = true;
    } else if (inFrontmatter) {
      if (line.trim() === '---' || line.trim() === '...') inFrontmatter = false;
    } else if (FENCE_LINE.test(line)) {
      inFence = !inFence;
    } else if (!inFence) {
      const match = HEADING_LINE.exec(line);
      if (match && normalizeHeadingText(match[2]) === wanted) return index;
    }
    index++;
  }
  return -1;
}

/**
 * Líneas `[start, end)` de la sección de un encabezado: desde el propio encabezado hasta el
 * siguiente de su mismo nivel o superior (los subtítulos de nivel inferior forman parte de la sección).
 */
export function findHeadingSection(lines: string[], heading: string): { start: number; end: number } | null {
  const start = findHeadingLine(lines, heading);
  if (start === -1) return null;

  const level = HEADING_LINE.exec(lines[start])![1].length;
  let inFence = false;
  for (let i = start + 1; i < lines.length; i++) {
    if (FENCE_LINE.test(lines[i])) {
      inFence = !inFence;
    } else if (!inFence) {
      const match = HEADING_LINE.exec(lines[i]);
      if (match && match[1].length <= level) return { start, end: i };
    }
  }
  return { start, end: lines.length };
}

export interface HeadingNode {
  level: number;
  /** Texto legible del encabezado (sin marcado de enlaces ni énfasis). */
  text: string;
  /** Texto tal como está en el archivo, para ir a él con `HeadingTarget`. */
  raw: string;
  line: number;
  children: HeadingNode[];
}

function displayHeadingText(text: string): string {
  return text
    .replace(/!?\[\[([^\]|]*)(?:\|([^\]]*))?\]\]/g, (_, target: string, alias?: string) => alias ?? target)
    .replace(/!?\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/[`*_~=]/g, '')
    .replace(/\s+/g, ' ')
    .trim();
}

/**
 * Encabezados de una nota en árbol: cada uno cuelga del anterior de nivel menor. Ignora el
 * frontmatter y los bloques de código, igual que `findHeadingLine`.
 */
export function extractHeadingTree(content: string): HeadingNode[] {
  const roots: HeadingNode[] = [];
  const stack: HeadingNode[] = [];
  let inFence = false;
  let inFrontmatter = false;
  content.split(/\r?\n/).forEach((line, index) => {
    if (index === 0 && line.trim() === '---') {
      inFrontmatter = true;
      return;
    }
    if (inFrontmatter) {
      if (line.trim() === '---' || line.trim() === '...') inFrontmatter = false;
      return;
    }
    if (FENCE_LINE.test(line)) {
      inFence = !inFence;
      return;
    }
    if (inFence) return;
    const match = HEADING_LINE.exec(line);
    if (!match) return;
    const node: HeadingNode = {
      level: match[1].length,
      text: displayHeadingText(match[2]) || match[2],
      raw: match[2],
      line: index,
      children: [],
    };
    while (stack.length > 0 && stack[stack.length - 1].level >= node.level) stack.pop();
    if (stack.length > 0) stack[stack.length - 1].children.push(node);
    else roots.push(node);
    stack.push(node);
  });
  return roots;
}
