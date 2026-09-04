import { appSettings } from '@entities/settings';
import { renderMermaidJsSvg } from '@features/mermaid-editor';
import { renderMermaidSvg, ensureMerman, isMermanInitialized } from '@features/merman-editor';

/**
 * Renderiza código Mermaid a SVG utilizando el motor configurado
 * por el usuario en las preferencias de la aplicación (Mermaid.js o Merman WASM).
 */
export async function renderUnifiedDiagramSvg(
  code: string,
  idPrefix = 'cm-mermaid'
): Promise<{ svg: string | null; error: string | null }> {
  if (!code || !code.trim()) {
    return { svg: null, error: null };
  }

  if (appSettings.mermaidRenderer === 'merman') {
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
