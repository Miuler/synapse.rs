import type { TFile, CachedMetadataLike as CachedMetadata } from './types';
import { parseYaml, stringifyYaml } from './utils';
import lzString from 'lz-string';

export interface ExcalidrawScene {
  type?: string;
  version?: number;
  source?: string;
  elements: any[];
  appState: Record<string, any>;
  files: Record<string, any>;
}

export interface ParsedExcalidrawDocument {
  isMarkdown: boolean;
  frontmatter?: Record<string, any>;
  scene: ExcalidrawScene;
  textElements: Array<{ text: string; id: string }>;
  wikilinks: string[];
}

/**
 * Regex para extraer wikilinks en textos: [[Destino]] o [[Destino|Alias]]
 */
const WIKILINK_REGEX = /\[\[([^\]|#]+)(?:#[^\]|]+)?(?:\|[^\]]+)?\]\]/g;

/**
 * Determina si un archivo dado es reconocido como un archivo de dibujo Excalidraw.
 * Comprueba:
 * 1. Extensiones: `.excalidraw`, `.excalidraw.json`, `.excalidraw.md`.
 * 2. Frontmatter YAML con clave `excalidraw-plugin`.
 */
export function isExcalidrawFile(
  file: TFile | { name: string; path: string },
  cache?: CachedMetadata | null
): boolean {
  const lowerName = file.name.toLowerCase();
  if (
    lowerName.endsWith('.excalidraw') ||
    lowerName.endsWith('.excalidraw.json') ||
    lowerName.endsWith('.excalidraw.md')
  ) {
    return true;
  }

  if (cache?.frontmatter && cache.frontmatter['excalidraw-plugin']) {
    return true;
  }

  return false;
}

/**
 * Determina si una cadena de texto corresponde a un dibujo Excalidraw en formato Markdown u Obsidian.
 */
export function isExcalidrawMarkdown(content: string): boolean {
  if (!content) return false;
  return (
    content.includes('excalidraw-plugin:') ||
    content.includes('```compressed-json') ||
    (content.includes('%%') && content.includes('# Drawing')) ||
    content.includes('==Deconstruct of drawing==') ||
    content.includes('Switch to EXCALIDRAW VIEW')
  );
}

/**
 * Extrae todos los enlaces wikilinks (`[[Nota]]`) presentes en los textos de los elementos de Excalidraw.
 */
export function extractWikilinksFromElements(elements: readonly any[]): string[] {
  const links = new Set<string>();
  if (!Array.isArray(elements)) return [];

  for (const el of elements) {
    if (!el || el.isDeleted) continue;

    // 1. Texto directo del elemento
    if (typeof el.text === 'string') {
      let match: RegExpExecArray | null;
      const re = new RegExp(WIKILINK_REGEX.source, 'g');
      while ((match = re.exec(el.text)) !== null) {
        if (match[1]) links.add(match[1].trim());
      }
    }

    // 2. Campo link directo del elemento
    if (typeof el.link === 'string' && el.link.startsWith('[[')) {
      let match: RegExpExecArray | null;
      const re = new RegExp(WIKILINK_REGEX.source, 'g');
      while ((match = re.exec(el.link)) !== null) {
        if (match[1]) links.add(match[1].trim());
      }
    }
  }

  return Array.from(links);
}

/**
 * Parsea el contenido de un archivo Excalidraw, ya sea en formato Markdown con frontmatter
 * YAML o en formato JSON puro.
 */
export function parseExcalidrawContent(raw: string): ParsedExcalidrawDocument {
  const trimmed = (raw || '').trim();

  // Caso 1: Archivo Markdown con frontmatter o comentarios de Excalidraw
  if (trimmed.startsWith('---') || isExcalidrawMarkdown(trimmed)) {
    return parseExcalidrawMarkdown(trimmed);
  }

  // Caso 2: Archivo JSON puro
  try {
    const parsed = JSON.parse(trimmed || '{}');
    const scene: ExcalidrawScene = {
      type: parsed.type || 'excalidraw',
      version: parsed.version || 2,
      source: parsed.source || 'synapse',
      elements: Array.isArray(parsed.elements) ? parsed.elements : [],
      appState: parsed.appState || {},
      files: parsed.files || {},
    };

    const textElements = scene.elements
      .filter((el) => el.type === 'text' && !el.isDeleted)
      .map((el) => ({ text: el.text || '', id: el.id || '' }));

    const wikilinks = extractWikilinksFromElements(scene.elements);

    return {
      isMarkdown: false,
      scene,
      textElements,
      wikilinks,
    };
  } catch {
    return {
      isMarkdown: false,
      scene: { type: 'excalidraw', version: 2, elements: [], appState: {}, files: {} },
      textElements: [],
      wikilinks: [],
    };
  }
}

/**
 * Parsea un documento Markdown formateado por obsidian-excalidraw-plugin.
 */
function parseExcalidrawMarkdown(raw: string): ParsedExcalidrawDocument {
  let frontmatter: Record<string, any> = {};
  let body = raw;

  // 1. Extraer Frontmatter YAML si existe
  const fmMatch = raw.match(/^---\r?\n([\s\S]*?)\r?\n---(\r?\n|$)/);
  if (fmMatch) {
    try {
      frontmatter = parseYaml(fmMatch[1]) || {};
    } catch {
      frontmatter = {};
    }
    body = raw.slice(fmMatch[0].length);
  }

  // 2. Extraer bloque JSON incrustado entre %% ... %% o en bloques ```json / ```compressed-json
  let jsonString = '';

  // Intento A: Bloque ```compressed-json (formato comprimido LZ-String estándar de Obsidian Excalidraw)
  const compressedMatch = body.match(/```compressed\-json\s*([\s\S]*?)\s*```/i);
  if (compressedMatch && compressedMatch[1]) {
    try {
      const cleanB64 = compressedMatch[1].replace(/[\r\n\s]/g, '');
      const decompressed = lzString.decompressFromBase64(cleanB64);
      if (decompressed) {
        jsonString = decompressed;
      }
    } catch (e) {
      console.warn('[ExcalidrawParser] Error al descomprimir compressed-json:', e);
    }
  }

  // Intento B: Entre marcadores %% # Drawing \n ```json ... ``` \n %%
  if (!jsonString) {
    const drawingBlockMatch = body.match(/%%\s*# Drawing\s*```json\s*([\s\S]*?)\s*```\s*%%/i);
    if (drawingBlockMatch && drawingBlockMatch[1]) {
      jsonString = drawingBlockMatch[1].trim();
    }
  }

  // Intento C: Cualquier bloque ```json ... ``` dentro de comentarios %% ... %%
  if (!jsonString) {
    const commentBlockMatch = body.match(/%%[\s\S]*?```json\s*([\s\S]*?)\s*```[\s\S]*?%%/i);
    if (commentBlockMatch && commentBlockMatch[1]) {
      jsonString = commentBlockMatch[1].trim();
    }
  }

  // Intento D: Primer bloque ```json
  if (!jsonString) {
    const codeBlockMatch = body.match(/```json\s*([\s\S]*?)\s*```/i);
    if (codeBlockMatch && codeBlockMatch[1]) {
      jsonString = codeBlockMatch[1].trim();
    }
  }

  let scene: ExcalidrawScene;
  if (jsonString) {
    try {
      const parsed = JSON.parse(jsonString);
      scene = {
        type: parsed.type || 'excalidraw',
        version: parsed.version || 2,
        source: parsed.source || 'https://github.com/zsviczian/obsidian-excalidraw-plugin',
        elements: Array.isArray(parsed.elements) ? parsed.elements : [],
        appState: parsed.appState || {},
        files: parsed.files || {},
      };
    } catch {
      scene = { type: 'excalidraw', version: 2, elements: [], appState: {}, files: {} };
    }
  } else {
    scene = { type: 'excalidraw', version: 2, elements: [], appState: {}, files: {} };
  }

  // Sincronizar elementos de texto modificados directamente en la sección '# Text Elements'
  const textElementsMatch = body.match(/# Text Elements([\s\S]*?)(?:%%|# Embedded Files|$)/i);
  if (textElementsMatch && textElementsMatch[1]) {
    const rawLines = textElementsMatch[1].split('\n');
    for (const line of rawLines) {
      const trimmedLine = line.trim();
      const blockIdMatch = trimmedLine.match(/^(.*?)\s*\^([a-zA-Z0-9_\-]+)$/);
      if (blockIdMatch) {
        const textValue = blockIdMatch[1].trim();
        const blockId = blockIdMatch[2].trim();
        const found = scene.elements.find((el) => el.id === blockId);
        if (found) {
          found.text = textValue;
        }
      }
    }
  }

  const textElements = scene.elements
    .filter((el) => el.type === 'text' && !el.isDeleted)
    .map((el) => ({ text: el.text || '', id: el.id || '' }));

  const wikilinks = extractWikilinksFromElements(scene.elements);

  return {
    isMarkdown: true,
    frontmatter,
    scene,
    textElements,
    wikilinks,
  };
}

/**
 * Serializa un objeto de escena Excalidraw al formato canónico de Markdown compatible con
 * `obsidian-excalidraw-plugin`.
 *
 * Preserva o genera el frontmatter con `excalidraw-plugin: parsed`, sincroniza la sección
 * `# Text Elements` (con identificadores de bloque `^id`) y serializa el bloque JSON dentro de `%% ... %%`.
 */
export function serializeExcalidrawMarkdown(
  scene: ExcalidrawScene | { elements: any[]; appState?: any; files?: any },
  existingContent?: string
): string {
  let frontmatter: Record<string, any> = {
    'excalidraw-plugin': 'parsed',
    tags: ['ea/drawing'],
  };

  // Si había contenido existente, preservar metadatos de frontmatter previos
  if (existingContent && existingContent.startsWith('---')) {
    const fmMatch = existingContent.match(/^---\r?\n([\s\S]*?)\r?\n---/);
    if (fmMatch) {
      try {
        const parsedFm = parseYaml(fmMatch[1]);
        if (parsedFm && typeof parsedFm === 'object') {
          frontmatter = {
            ...parsedFm,
            'excalidraw-plugin': 'parsed', // Siempre parsed para dibujo
          };
        }
      } catch {
        // Mantener por defecto
      }
    }
  }

  const elements = scene.elements || [];
  const appState = scene.appState || {};
  const files = scene.files || {};

  // 1. Extraer elementos de texto para deconstrucción en Markdown
  const textLines = elements
    .filter((el) => el.type === 'text' && !el.isDeleted && el.text)
    .map((el) => {
      const cleanText = el.text.replace(/\r?\n/g, ' ');
      const blockId = el.id ? ` ^${el.id}` : '';
      return `${cleanText}${blockId}`;
    });

  const textSection =
    textLines.length > 0
      ? textLines.join('\n\n')
      : '%% Elementos de texto aparecerán aquí al añadir texto al lienzo %%';

  // 2. Serializar escena en JSON compacto y ordenado
  const sceneJson = JSON.stringify(
    {
      type: 'excalidraw',
      version: 2,
      source: 'https://github.com/zsviczian/obsidian-excalidraw-plugin',
      elements,
      appState: {
        theme: appState.theme,
        viewBackgroundColor: appState.viewBackgroundColor,
        currentItemStrokeColor: appState.currentItemStrokeColor,
        currentItemBackgroundColor: appState.currentItemBackgroundColor,
        currentItemFillStyle: appState.currentItemFillStyle,
        currentItemStrokeWidth: appState.currentItemStrokeWidth,
        gridSize: appState.gridSize,
      },
      files,
    },
    null,
    2
  );

  // 3. Ensamblar documento canónico de Obsidian Excalidraw
  const yml = stringifyYaml(frontmatter).trim();
  const wasCompressed = !!existingContent && existingContent.includes('```compressed-json');

  let drawingBlock: string;
  if (wasCompressed) {
    const compressed = lzString.compressToBase64(sceneJson);
    const chunked = compressed.match(/.{1,250}/g)?.join('\n\n') || compressed;
    drawingBlock = `%%
## Drawing
\`\`\`compressed-json
${chunked}
\`\`\`
%%`;
  } else {
    drawingBlock = `%%
# Drawing
\`\`\`json
${sceneJson}
\`\`\`
%%`;
  }

  return `---

${yml}

---
==Deconstruct of drawing==
# Text Elements
${textSection}

${drawingBlock}
`;
}
