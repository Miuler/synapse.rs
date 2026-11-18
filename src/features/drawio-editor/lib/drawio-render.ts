import { appSettings } from '@entities/settings';
import { buildDrawioUrl, parseDrawioMessage } from './drawio-embed';

/** Tiempo máximo para cargar draw.io y exportar el diagrama. */
const RENDER_TIMEOUT_MS = 30_000;

/**
 * Dibuja un diagrama draw.io (XML) a PNG con el editor embebido en un iframe oculto: se carga el XML
 * y se pide `export` en formato `png`. Usa la URL de draw.io de los ajustes, así que sin red solo
 * funciona con un draw.io local. Devuelve el PNG como data URL.
 */
export function renderDrawioPng(xml: string): Promise<string> {
  const src = buildDrawioUrl(appSettings.settings.drawioUrl, false, appSettings.settings.locale ?? 'es');
  const origin = new URL(src).origin;
  const iframe = document.createElement('iframe');
  // Fuera de la vista pero con tamaño: draw.io necesita maquetar para exportar
  iframe.style.cssText = 'position:fixed;left:-10000px;top:0;width:800px;height:600px;border:0;visibility:hidden';
  iframe.setAttribute('aria-hidden', 'true');
  iframe.src = src;

  return new Promise<string>((resolve, reject) => {
    const post = (msg: Record<string, unknown>) => iframe.contentWindow?.postMessage(JSON.stringify(msg), origin);
    const finish = (fn: () => void) => {
      clearTimeout(timer);
      window.removeEventListener('message', onMessage);
      iframe.remove();
      fn();
    };
    const timer = setTimeout(() => finish(() => reject(new Error('draw.io no respondió al exportar el diagrama'))), RENDER_TIMEOUT_MS);
    const onMessage = (e: MessageEvent) => {
      if (e.source !== iframe.contentWindow) return;
      const msg = parseDrawioMessage(e.data);
      if (!msg) return;
      if (msg.event === 'init') post({ action: 'load', xml, autosave: 0 });
      else if (msg.event === 'load') post({ action: 'export', format: 'png', border: 10 });
      else if (msg.event === 'export') {
        const data = msg.data ?? '';
        finish(() => (data.startsWith('data:image/') ? resolve(data) : reject(new Error('draw.io no devolvió una imagen'))));
      }
    };
    window.addEventListener('message', onMessage);
    document.body.appendChild(iframe);
  });
}
