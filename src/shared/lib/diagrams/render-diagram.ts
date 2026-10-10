import { renderMermaidJsSvg } from './mermaid-service';
import { renderMermaidSvg, ensureMerman, isMermanInitialized } from './merman-service';

export type DiagramRendererType = 'mermaid' | 'mermaidjs' | 'merman';

/**
 * Renderiza código Mermaid a SVG utilizando el motor configurado (Mermaid.js o Merman WASM).
 */
export async function renderUnifiedDiagramSvg(
  code: string,
  idPrefix = 'cm-mermaid',
  renderer: DiagramRendererType = 'mermaid'
): Promise<{ svg: string | null; error: string | null }> {
  if (!code || !code.trim()) {
    return { svg: null, error: null };
  }

  if (renderer === 'merman') {
    if (!isMermanInitialized()) {
      try {
        await ensureMerman();
      } catch (err) {
        return { svg: null, error: `Error al inicializar Merman: ${String(err)}` };
      }
    }
    return renderMermaidSvg(code);
  } else {
    return await renderMermaidJsSvg(code, idPrefix);
  }
}
