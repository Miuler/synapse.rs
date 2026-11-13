import { Marked } from 'marked';
import DOMPurify from 'dompurify';

const marked = new Marked({ gfm: true, breaks: true });

/** Markdown de las respuestas del agente, ya saneado para insertarlo como HTML. */
export function renderAgentMarkdown(text: string): string {
  const html = marked.parse(text, { async: false }) as string;
  return DOMPurify.sanitize(html, { FORBID_TAGS: ['style', 'form', 'input', 'iframe'] });
}

/** Ruta relativa a la bóveda de un enlace `file://` que apunta dentro de ella, o `null`. */
export function vaultPathOfLink(href: string, cwd: string | null): string | null {
  if (!cwd || !href.startsWith('file://')) return null;
  let path: string;
  try {
    path = decodeURIComponent(new URL(href).pathname);
  } catch {
    return null;
  }
  const root = cwd.replace(/\/$/, '') + '/';
  return path.startsWith(root) ? path.slice(root.length) : null;
}

/** Valor crudo de una herramienta como texto legible. */
export function formatRaw(value: unknown): string {
  if (typeof value === 'string') return value;
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}
