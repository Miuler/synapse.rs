import mermaid, { type MermaidConfig } from 'mermaid';

let isInitialized = false;
let renderCounter = 0;

/**
 * Inicializa la configuración global de Mermaid.js
 */
export function initMermaidJs(config?: MermaidConfig): void {
  if (!isInitialized) {
    mermaid.initialize({
      startOnLoad: false,
      securityLevel: 'loose',
      theme: 'default',
      fontFamily: 'inherit',
      suppressErrorRendering: true,
      ...config,
    });
    isInitialized = true;
  }
}

/**
 * Asegura que Mermaid.js esté inicializado antes de su uso
 */
export async function ensureMermaidJs(config?: MermaidConfig): Promise<void> {
  if (!isInitialized) {
    initMermaidJs(config);
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
): Promise<{ valid: boolean; error: string | null }> {
  if (!code || !code.trim()) {
    return { valid: true, error: null };
  }

  try {
    await ensureMermaidJs();
    await mermaid.parse(code, { suppressErrors: false });
    return { valid: true, error: null };
  } catch (err: any) {
    return { valid: false, error: String(err?.message || err) };
  }
}

/**
 * Renderiza código Mermaid a una cadena SVG utilizando la API oficial de Mermaid.js
 */
export async function renderMermaidJsSvg(
  code: string,
  idPrefix = 'mermaid-chart'
): Promise<{ svg: string | null; error: string | null }> {
  if (!code || !code.trim()) {
    return { svg: null, error: null };
  }

  try {
    await ensureMermaidJs();

    // Validar sintaxis primero para evitar residuos en el DOM
    try {
      await mermaid.parse(code, { suppressErrors: false });
    } catch (parseError: any) {
      return { svg: null, error: String(parseError?.message || parseError) };
    }

    const uniqueId = `${idPrefix}-${Date.now()}-${++renderCounter}`;
    const { svg } = await mermaid.render(uniqueId, code);

    return { svg, error: null };
  } catch (err: any) {
    return { svg: null, error: String(err?.message || err) };
  }
}

export { mermaid };
