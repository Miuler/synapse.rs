import type { RenderOptions } from '@d2lang/d2';

export interface D2SyntaxError {
  line: number; // 1-indexed
  column: number; // 1-indexed
  endLine?: number;
  endColumn?: number;
  message: string;
}

export interface D2ValidationResult {
  valid: boolean;
  errors: D2SyntaxError[];
  errorText: string | null;
}

export interface D2RenderResult {
  svg: string | null;
  error: string | null;
  errors: D2SyntaxError[];
}

export interface D2AutocompleteItem {
  label: string;
  type?: string;
  detail?: string;
  info?: string;
  boost?: number;
  apply?: string;
}

// Catálogo de motores disponibles en el build WASM
export const D2_ENGINES = [
  { label: 'dagre', detail: 'Layout jerárquico por defecto (rápido)', apply: 'dagre' },
  { label: 'elk', detail: 'Eclipse Layout Kernel (avanzado, soporta puertos)', apply: 'elk' },
  { label: 'tala', detail: 'TALA (diseñado para diagramas de arquitectura)', apply: 'tala' },
];

// Catálogo de temas oficiales de D2 con sus IDs
export const D2_THEMES = [
  { id: 0, label: '0', detail: 'Neutral Default (Claro)' },
  { id: 1, label: '1', detail: 'Neutral Grey (Claro)' },
  { id: 3, label: '3', detail: 'Flagship Terrastruct (Claro)' },
  { id: 4, label: '4', detail: 'Cool Classics (Claro)' },
  { id: 5, label: '5', detail: 'Mixed Berry Blue (Claro)' },
  { id: 6, label: '6', detail: 'Grape Soda (Claro)' },
  { id: 7, label: '7', detail: 'Aubergine (Claro)' },
  { id: 8, label: '8', detail: 'Colorblind Clear (Claro)' },
  { id: 100, label: '100', detail: 'Vanilla Nitro Cola (Claro)' },
  { id: 101, label: '101', detail: 'Orange Creamsicle (Claro)' },
  { id: 102, label: '102', detail: 'Shirley Temple (Claro)' },
  { id: 103, label: '103', detail: 'Earth Tones (Claro)' },
  { id: 104, label: '104', detail: 'Everglade Green (Claro)' },
  { id: 105, label: '105', detail: 'Buttered Toast (Claro)' },
  { id: 200, label: '200', detail: 'Dark Mauve (Oscuro)' },
  { id: 201, label: '201', detail: 'Dark Flagship Terrastruct (Oscuro)' },
  { id: 300, label: '300', detail: 'Terminal (Oscuro / Monocromo)' },
  { id: 301, label: '301', detail: 'Terminal Grayscale' },
  { id: 302, label: '302', detail: 'Origami' },
  { id: 303, label: '303', detail: 'C4 Architecture' },
];

// Formas soportadas por D2
export const D2_SHAPES = [
  'rectangle',
  'square',
  'page',
  'parallelogram',
  'document',
  'cylinder',
  'queue',
  'package',
  'step',
  'callout',
  'stored_data',
  'person',
  'diamond',
  'oval',
  'circle',
  'hexagon',
  'cloud',
  'sql_table',
  'class',
  'sequence_diagram',
  'text',
  'code',
  'image',
];

// Direcciones
export const D2_DIRECTIONS = ['right', 'down', 'left', 'up'];

// Estilos
export const D2_STYLES = [
  'opacity',
  'stroke',
  'fill',
  'fill-pattern',
  'stroke-width',
  'stroke-dash',
  'border-radius',
  'shadow',
  '3d',
  'multiple',
  'font-size',
  'font-color',
  'animated',
  'bold',
  'italic',
  'underline',
  'double-border',
];

let d2InstancePromise: Promise<any> | null = null;

/**
 * Carga e inicializa la instancia singleton de D2 WASM.
 */
export async function ensureD2(): Promise<any> {
  if (!d2InstancePromise) {
    d2InstancePromise = (async () => {
      const module = await import('@d2lang/d2');
      const D2Class = module.D2;
      return new D2Class();
    })().catch((err) => {
      d2InstancePromise = null;
      throw err;
    });
  }
  return d2InstancePromise;
}

/**
 * Parsea errores de compilación de D2, extrayendo las posiciones exactas de línea y columna.
 * El WASM devuelve típicamente un array JSON como string en err.message:
 * [{"range":"index,1:21:28-1:22:29","errmsg":"index:2:22: maps must be terminated with }"}]
 */
export function parseD2Errors(error: unknown): D2SyntaxError[] {
  if (!error) return [];

  const rawMsg = typeof error === 'string' ? error : (error as any)?.message || String(error);
  const errors: D2SyntaxError[] = [];

  // Intenta parsear array JSON de errores
  try {
    const jsonStart = rawMsg.indexOf('[');
    const jsonEnd = rawMsg.lastIndexOf(']');
    if (jsonStart !== -1 && jsonEnd !== -1 && jsonEnd > jsonStart) {
      const jsonStr = rawMsg.slice(jsonStart, jsonEnd + 1);
      const parsed = JSON.parse(jsonStr);
      if (Array.isArray(parsed)) {
        for (const item of parsed) {
          if (item?.errmsg) {
            let line = 1;
            let column = 1;
            let endLine: number | undefined;
            let endColumn: number | undefined;

            // Formato de range: "index,1:21:28-1:22:29" (línea 0-indexed : columna 0-indexed)
            if (typeof item.range === 'string') {
              const rangeMatch = item.range.match(/(\d+):(\d+):(?:\d+)(?:-(\d+):(\d+):(?:\d+))?/);
              if (rangeMatch) {
                line = parseInt(rangeMatch[1], 10) + 1;
                column = parseInt(rangeMatch[2], 10) + 1;
                if (rangeMatch[3] !== undefined && rangeMatch[4] !== undefined) {
                  endLine = parseInt(rangeMatch[3], 10) + 1;
                  endColumn = parseInt(rangeMatch[4], 10) + 1;
                }
              }
            } else {
              // Intenta extraer del errmsg ej: "index:2:22: mensaje"
              const msgMatch = item.errmsg.match(/(?:^|:)(\d+):(\d+):\s*(.+)$/);
              if (msgMatch) {
                line = parseInt(msgMatch[1], 10);
                column = parseInt(msgMatch[2], 10);
              }
            }

            const cleanMsg = item.errmsg.replace(/^index:\d+:\d+:\s*/, '');
            errors.push({
              line,
              column,
              endLine: endLine ?? line,
              endColumn: endColumn ?? column + 1,
              message: cleanMsg,
            });
          }
        }
      }
    }
  } catch {
    // Si no es JSON parseable, usar regex directa sobre el mensaje
  }

  if (errors.length === 0) {
    // Regex de fallback para mensajes estilo "index:2:22: mensaje" o "err on line X"
    const lineMatch = rawMsg.match(/(?:index:|\bline\s+)(\d+)(?::(\d+))?[:\s]+(.+)$/i);
    if (lineMatch) {
      const line = parseInt(lineMatch[1], 10);
      const column = lineMatch[2] ? parseInt(lineMatch[2], 10) : 1;
      errors.push({
        line,
        column,
        message: lineMatch[3].trim(),
      });
    } else {
      errors.push({
        line: 1,
        column: 1,
        message: rawMsg.replace(/^Error:\s*/, '').trim(),
      });
    }
  }

  return errors;
}

/**
 * Valida la sintaxis del código D2.
 */
export async function validateD2(code: string): Promise<D2ValidationResult> {
  if (!code || !code.trim()) {
    return { valid: true, errors: [], errorText: null };
  }

  try {
    const d2 = await ensureD2();
    await d2.compile(code);
    return { valid: true, errors: [], errorText: null };
  } catch (err: any) {
    const errors = parseD2Errors(err);
    const errorText = errors.map((e) => `Línea ${e.line}, col ${e.column}: ${e.message}`).join('\n') || String(err?.message || err);
    return {
      valid: false,
      errors,
      errorText,
    };
  }
}

/**
 * Renderiza código D2 a formato SVG string.
 */
export async function renderD2Svg(
  code: string,
  options?: RenderOptions
): Promise<D2RenderResult> {
  if (!code || !code.trim()) {
    return { svg: null, error: null, errors: [] };
  }

  try {
    const d2 = await ensureD2();
    const isDark = typeof document !== 'undefined' && document.documentElement.getAttribute('data-theme') === 'dark';

    // Compilar el código a diagrama intermedio (AST + Layout)
    const compileRes = await d2.compile(code, options);

    // Si el usuario no especificó tema en vars.d2-config, aplicar el tema de Synapse
    const renderOpts: RenderOptions = {
      themeID: isDark ? 200 : 0,
      noXMLTag: true,
      ...compileRes.renderOptions,
      ...options,
    };

    const svg = await d2.render(compileRes.diagram, renderOpts);
    return { svg, error: null, errors: [] };
  } catch (err: any) {
    const errors = parseD2Errors(err);
    const errorMsg = errors.map((e) => `Línea ${e.line}, col ${e.column}: ${e.message}`).join('\n') || String(err?.message || err);
    return { svg: null, error: errorMsg, errors };
  }
}
