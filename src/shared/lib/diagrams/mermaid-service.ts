import mermaid, { type MermaidConfig } from 'mermaid';

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
  { label: 'default', detail: 'Tema clásico por defecto' },
  { label: 'neutral', detail: 'Tema neutral en escala de grises' },
  { label: 'dark', detail: 'Tema oscuro de alto contraste' },
  { label: 'forest', detail: 'Tema con tonos verdes terrosos' },
  { label: 'base', detail: 'Tema base para personalización' },
  { label: 'neo', detail: 'Tema moderno (Merman/Web)' },
  { label: 'neo-dark', detail: 'Tema moderno oscuro' },
  { label: 'redux', detail: 'Tema dinámico con colores vivos' },
  { label: 'redux-dark', detail: 'Tema redux oscuro' },
];

export const MERMAID_LOOKS = [
  { label: 'classic', detail: 'Estilo clásico vectorial limpio' },
  { label: 'handDrawn', detail: 'Estilo boceto a mano alzada' },
  { label: 'neo', detail: 'Estilo moderno' },
];

export const MERMAID_LAYOUTS = [
  { label: 'dagre', detail: 'Layout jerárquico por defecto' },
  { label: 'elk', detail: 'Eclipse Layout Kernel' },
  { label: 'tidy-tree', detail: 'Árbol ordenado' },
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
  { label: 'flowchart TD', detail: 'Diagrama de flujo de arriba a abajo' },
  { label: 'flowchart LR', detail: 'Diagrama de flujo de izquierda a derecha' },
  { label: 'sequenceDiagram', detail: 'Diagrama de secuencia' },
  { label: 'classDiagram', detail: 'Diagrama de clases UML' },
  { label: 'stateDiagram-v2', detail: 'Diagrama de estados' },
  { label: 'erDiagram', detail: 'Diagrama Entidad-Relación' },
  { label: 'gantt', detail: 'Diagrama de Gantt' },
  { label: 'pie', detail: 'Gráfico circular' },
  { label: 'gitGraph', detail: 'Historial de ramas Git' },
  { label: 'mindmap', detail: 'Mapa mental' },
  { label: 'timeline', detail: 'Línea de tiempo' },
  { label: 'architecture-beta', detail: 'Diagrama de arquitectura en la nube' },
  { label: 'xychart-beta', detail: 'Gráfico XY (barras / líneas)' },
  { label: 'quadrantChart', detail: 'Gráfico de cuadrantes' },
  { label: 'kanban', detail: 'Tablero Kanban' },
  { label: 'block-beta', detail: 'Diagrama de bloques' },
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
    const errorText = errors.map((e) => `Línea ${e.line}, col ${e.column}: ${e.message}`).join('\n') || String(err?.message || err);
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
