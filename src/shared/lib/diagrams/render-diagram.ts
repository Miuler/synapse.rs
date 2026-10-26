import { renderMermaidJsSvg } from './mermaid-service';
import { renderMermaidSvg, ensureMerman, isMermanInitialized } from './merman-service';
import { renderD2Svg } from './d2-service';

export type DiagramRendererType = 'mermaid' | 'mermaidjs' | 'merman' | 'd2';

/**
 * Renderiza código de diagrama (Mermaid o D2) a SVG utilizando el motor correspondiente.
 */
export async function renderUnifiedDiagramSvg(
  code: string,
  idPrefix = 'cm-diagram',
  renderer: DiagramRendererType = 'mermaid'
): Promise<{ svg: string | null; error: string | null; errors?: any[] }> {
  if (!code || !code.trim()) {
    return { svg: null, error: null, errors: [] };
  }

  if (renderer === 'd2') {
    return await renderD2Svg(code);
  }

  if (renderer === 'merman') {
    if (!isMermanInitialized()) {
      try {
        await ensureMerman();
      } catch (err) {
        return { svg: null, error: `Error al inicializar Merman: ${String(err)}`, errors: [] };
      }
    }
    return renderMermaidSvg(code);
  } else {
    return await renderMermaidJsSvg(code, idPrefix);
  }
}
