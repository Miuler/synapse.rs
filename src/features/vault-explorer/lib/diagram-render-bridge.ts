import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { invokeTauri } from '@shared/api';
import { renderDiagram } from './export-diagram';

function toBase64(bytes: Uint8Array): string {
  let bin = '';
  const CHUNK = 0x8000;
  for (let i = 0; i < bytes.length; i += CHUNK) bin += String.fromCharCode(...bytes.subarray(i, i + CHUNK));
  return btoa(bin);
}

/**
 * Atiende las peticiones del MCP (`render_diagram`): dibuja el diagrama con los motores de la
 * ventana y devuelve el PNG al backend.
 */
export function listenDiagramRenderRequests(): Promise<UnlistenFn> {
  return listen<{ requestId: string; relativePath: string; content?: string | null }>('mcp://render-diagram', async ({ payload }) => {
    try {
      const bytes = await renderDiagram(payload.relativePath, 'png', payload.content);
      await invokeTauri('mcp_diagram_rendered', {
        requestId: payload.requestId,
        data: toBase64(bytes),
        mimeType: 'image/png',
        error: null,
      });
    } catch (e) {
      await invokeTauri('mcp_diagram_rendered', {
        requestId: payload.requestId,
        data: null,
        mimeType: null,
        error: e instanceof Error ? e.message : String(e),
      }).catch(() => {});
    }
  });
}
