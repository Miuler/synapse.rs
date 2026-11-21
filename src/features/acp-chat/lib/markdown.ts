import { Marked } from 'marked';
import DOMPurify from 'dompurify';
import type { Mention } from '@entities/acp-agent';
import { splitMentions } from './mentions';

const marked = new Marked({ gfm: true, breaks: true });

/** De los `<input>`, solo se dejan las casillas de tarea de las listas, desactivadas */
function keepOnlyTaskCheckboxes(node: Element) {
  if (node.nodeName !== 'INPUT') return;
  if ((node as HTMLInputElement).type !== 'checkbox') node.remove();
  else node.setAttribute('disabled', '');
}

/** Markdown de las respuestas del agente, ya saneado para insertarlo como HTML. */
export function renderAgentMarkdown(text: string): string {
  const html = marked.parse(text, { async: false }) as string;
  DOMPurify.addHook('afterSanitizeElements', keepOnlyTaskCheckboxes);
  try {
    return DOMPurify.sanitize(html, { FORBID_TAGS: ['style', 'form', 'iframe'] });
  } finally {
    DOMPurify.removeHook('afterSanitizeElements', keepOnlyTaskCheckboxes);
  }
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

const escapeHtml = (t: string) => t.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

/** Mensaje del usuario como Markdown (lo básico), con sus menciones `@…` resaltadas. */
export function renderUserMarkdown(text: string, mentions?: Mention[]): string {
  const source = splitMentions(text, mentions)
    .map((part) =>
      'mention' in part
        ? `<span class="acp-mention" title="${escapeHtml(part.mention.title)}">${escapeHtml(part.mention.token)}</span>`
        : part.text,
    )
    .join('');
  return renderAgentMarkdown(source);
}
