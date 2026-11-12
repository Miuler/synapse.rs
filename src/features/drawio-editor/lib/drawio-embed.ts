/**
 * Protocolo del modo embed de draw.io (`embed=1&proto=json`): el editor vive en un iframe y habla con
 * la app por `postMessage` con mensajes JSON. Eventos que envía: `init`, `load`, `save`, `export`,
 * `exit`, `autosave`. Acciones que recibe: `load`, `export`, `status`, `configure`.
 */

export type DrawioFormat = 'xml' | 'xmlpng';

export interface DrawioEvent {
  event: 'init' | 'load' | 'save' | 'export' | 'exit' | 'autosave' | 'configure' | string;
  xml?: string;
  data?: string;
  format?: string;
  modified?: boolean;
}

/** URL del iframe con las opciones del modo embed y el tema de la app. */
/** `exitButton`: muestra «Salir» (en `.drawio.png` vuelve al visor de imagen). */
export function buildDrawioUrl(base: string, dark: boolean, lang = 'es', exitButton = false): string {
  const url = new URL(base);
  url.searchParams.set('embed', '1');
  url.searchParams.set('proto', 'json');
  url.searchParams.set('spin', '1');
  url.searchParams.set('saveAndExit', '0');
  if (!exitButton) url.searchParams.set('noExitBtn', '1');
  url.searchParams.set('libraries', '1');
  url.searchParams.set('lang', lang);
  url.searchParams.set('dark', dark ? '1' : '0');
  return url.toString();
}

/** Interpreta un mensaje del iframe; ignora los que no son del protocolo. */
export function parseDrawioMessage(data: unknown): DrawioEvent | null {
  if (typeof data !== 'string' || !data.startsWith('{')) return null;
  try {
    const msg = JSON.parse(data);
    return msg && typeof msg.event === 'string' ? (msg as DrawioEvent) : null;
  } catch {
    return null;
  }
}

/** Diagrama vacío para un `.drawio` sin contenido. */
export const EMPTY_DRAWIO_XML =
  '<mxfile><diagram name="Página-1"><mxGraphModel><root><mxCell id="0"/><mxCell id="1" parent="0"/></root></mxGraphModel></diagram></mxfile>';

export function bytesToDataUrl(bytes: Uint8Array, mime = 'image/png'): string {
  let binary = '';
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return `data:${mime};base64,${btoa(binary)}`;
}

export function dataUrlToBytes(dataUrl: string): Uint8Array {
  const base64 = dataUrl.slice(dataUrl.indexOf(',') + 1);
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes;
}
