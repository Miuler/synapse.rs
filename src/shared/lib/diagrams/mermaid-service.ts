import mermaid, { type MermaidConfig } from 'mermaid';
import { t } from '@shared/lib/i18n';

export interface MermaidSyntaxError {
  line: number;
  column: number;
  endLine?: number;
  endColumn?: number;
  message: string;
}

export interface MermaidValidationResult {
  valid: boolean;
  error: string | null;
  errors: MermaidSyntaxError[];
}

let isInitialized = false;
let currentTheme = 'default';
let renderCounter = 0;

// Catálogos para autocompletado de Mermaid
export const MERMAID_THEMES = [
  { label: 'default', get detail() { return t('diagramsCatalog.mermaidThemeDefault'); } },
  { label: 'neutral', get detail() { return t('diagramsCatalog.mermaidThemeNeutral'); } },
  { label: 'dark', get detail() { return t('diagramsCatalog.mermaidThemeDark'); } },
  { label: 'forest', get detail() { return t('diagramsCatalog.mermaidThemeForest'); } },
  { label: 'base', get detail() { return t('diagramsCatalog.mermaidThemeBase'); } },
  { label: 'neo', get detail() { return t('diagramsCatalog.mermaidThemeNeo'); } },
  { label: 'neo-dark', get detail() { return t('diagramsCatalog.mermaidThemeNeoDark'); } },
  { label: 'redux', get detail() { return t('diagramsCatalog.mermaidThemeRedux'); } },
  { label: 'redux-dark', get detail() { return t('diagramsCatalog.mermaidThemeReduxDark'); } },
];

export const MERMAID_LOOKS = [
  { label: 'classic', get detail() { return t('diagramsCatalog.mermaidLookClassic'); } },
  { label: 'handDrawn', get detail() { return t('diagramsCatalog.mermaidLookHandDrawn'); } },
  { label: 'neo', get detail() { return t('diagramsCatalog.mermaidLookNeo'); } },
];

export const MERMAID_LAYOUTS = [
  { label: 'dagre', get detail() { return t('diagramsCatalog.mermaidLayoutDagre'); } },
  { label: 'elk', get detail() { return t('diagramsCatalog.mermaidLayoutElk'); } },
  { label: 'tidy-tree', get detail() { return t('diagramsCatalog.mermaidLayoutTidyTree'); } },
];

export const MERMAID_CURVES = [
  'basis',
  'bumpX',
  'bumpY',
  'cardinal',
  'catmullRom',
  'linear',
  'monotoneX',
  'monotoneY',
  'natural',
  'step',
  'stepAfter',
  'stepBefore',
];

export const MERMAID_DIAGRAM_TYPES = [
  { label: 'flowchart TD', get detail() { return t('diagramsCatalog.mermaidTypeFlowTd'); } },
  { label: 'flowchart LR', get detail() { return t('diagramsCatalog.mermaidTypeFlowLr'); } },
  { label: 'sequenceDiagram', get detail() { return t('diagramsCatalog.mermaidTypeSeq'); } },
  { label: 'classDiagram', get detail() { return t('diagramsCatalog.mermaidTypeClass'); } },
  { label: 'stateDiagram-v2', get detail() { return t('diagramsCatalog.mermaidTypeState'); } },
  { label: 'erDiagram', get detail() { return t('diagramsCatalog.mermaidTypeEr'); } },
  { label: 'gantt', get detail() { return t('diagramsCatalog.mermaidTypeGantt'); } },
  { label: 'pie', get detail() { return t('diagramsCatalog.mermaidTypePie'); } },
  { label: 'gitGraph', get detail() { return t('diagramsCatalog.mermaidTypeGit'); } },
  { label: 'mindmap', get detail() { return t('diagramsCatalog.mermaidTypeMindmap'); } },
  { label: 'timeline', get detail() { return t('diagramsCatalog.mermaidTypeTimeline'); } },
  { label: 'architecture-beta', get detail() { return t('diagramsCatalog.mermaidTypeArch'); } },
  { label: 'xychart-beta', get detail() { return t('diagramsCatalog.mermaidTypeXy'); } },
  { label: 'quadrantChart', get detail() { return t('diagramsCatalog.mermaidTypeQuadrant'); } },
  { label: 'kanban', get detail() { return t('diagramsCatalog.mermaidTypeKanban'); } },
  { label: 'block-beta', get detail() { return t('diagramsCatalog.mermaidTypeBlock'); } },
];

/**
 * Extrae la información estructurada de posición a partir de un error de parseo de Mermaid
 */
export function parseMermaidErrors(err: any): MermaidSyntaxError[] {
  if (!err) return [];

  const rawMsg = String(err?.message || err);
  const errors: MermaidSyntaxError[] = [];

  // 1. Mermaid a menudo incluye un objeto err.hash con loc
  if (err?.hash?.loc) {
    const loc = err.hash.loc;
    errors.push({
      line: loc.first_line || 1,
      column: loc.first_column !== undefined ? loc.first_column + 1 : 1,
      endLine: loc.last_line || loc.first_line || 1,
      endColumn: loc.last_column !== undefined ? loc.last_column + 1 : undefined,
      message: rawMsg.replace(/Parse error on line \d+:\s*/i, '').trim(),
    });
    return errors;
  }

  // 2. Extraer "Parse error on line X:"
  const match = rawMsg.match(/Parse error on line (\d+)(?::\s*([\s\S]*))?/i);
  if (match) {
    const line = parseInt(match[1], 10);
    const detail = (match[2] || rawMsg).split('\n')[0].trim();
    errors.push({
      line,
      column: 1,
      message: detail || rawMsg,
    });
    return errors;
  }

  // 3. Fallback genérico
  errors.push({
    line: 1,
    column: 1,
    message: rawMsg,
  });
  return errors;
}

/**
 * Actualiza el tema de Mermaid.js según el modo claro u oscuro
 */
export function updateMermaidTheme(isDark: boolean): void {
  const targetTheme = isDark ? 'dark' : 'default';
  if (targetTheme !== currentTheme || !isInitialized) {
    currentTheme = targetTheme;
    mermaid.initialize({
      startOnLoad: false,
      securityLevel: 'loose',
      theme: targetTheme as any,
      fontFamily: 'inherit',
      suppressErrorRendering: true,
    });
    isInitialized = true;
  }
}

/**
 * Inicializa la configuración global de Mermaid.js
 */
export function initMermaidJs(config?: MermaidConfig): void {
  const isDark = typeof document !== 'undefined' && document.documentElement.getAttribute('data-theme') === 'dark';
  currentTheme = isDark ? 'dark' : 'default';
  mermaid.initialize({
    startOnLoad: false,
    securityLevel: 'loose',
    theme: currentTheme as any,
    fontFamily: 'inherit',
    suppressErrorRendering: true,
    ...config,
  });
  isInitialized = true;
}

/**
 * Asegura que Mermaid.js esté inicializado antes de su uso
 */
export async function ensureMermaidJs(config?: MermaidConfig): Promise<void> {
  const isDark = typeof document !== 'undefined' && document.documentElement.getAttribute('data-theme') === 'dark';
  if (!isInitialized) {
    initMermaidJs(config);
  } else {
    updateMermaidTheme(isDark);
  }
}

/**
 * Indica si Mermaid.js ha sido inicializado
 */
export function isMermaidJsInitialized(): boolean {
  return isInitialized;
}

/**
 * Valida la sintaxis del código Mermaid usando el parser de Mermaid.js
 */
export async function validateMermaidJs(
  code: string
): Promise<MermaidValidationResult> {
  if (!code || !code.trim()) {
    return { valid: true, error: null, errors: [] };
  }

  try {
    await ensureMermaidJs();
    await mermaid.parse(code, { suppressErrors: false });
    return { valid: true, error: null, errors: [] };
  } catch (err: any) {
    const errors = parseMermaidErrors(err);
    const errorText = errors.map((e) => t('diagramsCatalog.lineCol', { line: e.line, col: e.column, message: e.message })).join('\n') || String(err?.message || err);
    return { valid: false, error: errorText, errors };
  }
}

/**
 * Renderiza código Mermaid a una cadena SVG utilizando la API oficial de Mermaid.js
 */
export async function renderMermaidJsSvg(
  code: string,
  idPrefix = 'mermaid-chart'
): Promise<{ svg: string | null; error: string | null; errors: MermaidSyntaxError[] }> {
  if (!code || !code.trim()) {
    return { svg: null, error: null, errors: [] };
  }

  try {
    await ensureMermaidJs();

    // Validar sintaxis primero para capturar posición exacta
    try {
      await mermaid.parse(code, { suppressErrors: false });
    } catch (parseError: any) {
      const errors = parseMermaidErrors(parseError);
      return { svg: null, error: String(parseError?.message || parseError), errors };
    }

    const uniqueId = `${idPrefix}-${Date.now()}-${++renderCounter}`;
    const { svg } = await mermaid.render(uniqueId, code);

    return { svg, error: null, errors: [] };
  } catch (err: any) {
    const errors = parseMermaidErrors(err);
    return { svg: null, error: String(err?.message || err), errors };
  }
}

export { mermaid };
