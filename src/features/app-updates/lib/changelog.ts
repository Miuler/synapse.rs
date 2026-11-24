// Novedades a partir del CHANGELOG.md (empaquetado con la app) y cuándo abrirlas al arrancar.

import { t } from '@shared/lib/i18n';

export interface ChangelogSection {
  version: string;
  /** Sección completa, con su encabezado «## [x] - fecha» */
  markdown: string;
}

const HEADING = /^##\s+\[([^\]]+)\]/;

/** Secciones de versiones publicadas, de la más nueva a la más antigua (sin «Sin publicar»). */
export function parseChangelog(raw: string): ChangelogSection[] {
  const sections: ChangelogSection[] = [];
  let current: { version: string; lines: string[] } | null = null;
  for (const line of raw.split('\n')) {
    const m = HEADING.exec(line);
    if (m) {
      if (current) sections.push({ version: current.version, markdown: current.lines.join('\n').trim() });
      current = /^(unreleased|sin publicar)$/i.test(m[1]) ? null : { version: m[1], lines: [line] };
    } else if (line.startsWith('[') && /^\[[^\]]+\]:\s*http/.test(line)) {
      // Enlaces de referencia al final del archivo: no forman parte de ninguna sección
      continue;
    } else if (current) {
      current.lines.push(line);
    }
  }
  if (current) sections.push({ version: current.version, markdown: current.lines.join('\n').trim() });
  return sections;
}

/**
 * Markdown de las novedades desde `since` (sin incluirla) hasta `current` (incluida).
 * Sin `since`, todas las versiones hasta `current`. Si `current` no está en el CHANGELOG
 * (compilación de desarrollo), se empieza por la más nueva.
 */
export function newsMarkdown(raw: string, current: string, since?: string): string {
  const sections = parseChangelog(raw);
  let start = sections.findIndex((s) => s.version === current);
  if (start < 0) start = 0;
  let end = since ? sections.findIndex((s) => s.version === since) : -1;
  if (end <= start) end = sections.length;
  const picked = sections.slice(start, end);
  const title = `# ${since && end < sections.length ? t('news.titleSince', { version: since }) : t('news.title')}`;
  return [title, ...picked.map((s) => s.markdown)].join('\n\n') + '\n';
}

/** Versión publicada justo antes de `current`, según el CHANGELOG. */
export function previousVersion(raw: string, current: string): string | undefined {
  const sections = parseChangelog(raw);
  const i = sections.findIndex((s) => s.version === current);
  return i >= 0 ? sections[i + 1]?.version : undefined;
}

/**
 * Qué novedades abrir al arrancar: `{ since }` (desde qué versión) o `null` para no abrir nada.
 * - Misma versión que la última vez: nada.
 * - Primera vez en esta versión tras usar otra: desde la que se usaba.
 * - Sin registro previo pero con ajustes guardados (se actualizó desde una versión que aún no
 *   lo guardaba): solo la versión actual.
 * - Instalación nueva (sin ajustes): nada; la bienvenida ya presenta la app.
 */
export function newsToOpenOnStartup(
  raw: string,
  current: string,
  lastSeen: string | null,
  hadPreviousSettings: boolean,
): { since?: string } | null {
  if (lastSeen === current) return null;
  if (lastSeen) return { since: lastSeen };
  if (!hadPreviousSettings) return null;
  return { since: previousVersion(raw, current) };
}
