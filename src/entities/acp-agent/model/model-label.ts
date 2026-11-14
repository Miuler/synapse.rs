/** Fabricantes que se quitan del nombre del modelo (junto con la versión que los sigue). */
const VENDORS = /\b(google|anthropic|openai|gemini|claude|gpt|mistral|meta|llama)\b(\s+\d+(?:\.\d+)*)?/gi;

const EFFORT: Record<string, string> = {
  minimal: 'Min',
  low: 'L',
  medium: 'M',
  med: 'M',
  high: 'H',
  xhigh: 'XH',
  'x-high': 'XH',
  max: 'Max',
};

const MAX_LABEL = 12;

function cut(text: string, max: number): string {
  return text.length > max ? `${text.slice(0, max - 1).trimEnd()}…` : text;
}

/** Esfuerzo entre paréntesis («(Low)») a una letra, y el nombre sin él. */
function splitEffort(name: string): { base: string; effort: string } {
  const m = /\s*\(([^)]+)\)\s*$/.exec(name);
  const effort = m ? EFFORT[m[1].trim().toLowerCase()] : undefined;
  return effort ? { base: name.slice(0, m!.index), effort } : { base: name, effort: '' };
}

function label(name: string, keepVendorVersion: boolean): string {
  const { base, effort } = splitEffort(name.trim());
  // Modelo desconocido (sin fabricante ni esfuerzo que reconozcamos): nombre completo, sin cortar
  VENDORS.lastIndex = 0;
  if (!effort && !VENDORS.test(base)) return name.trim();
  VENDORS.lastIndex = 0;
  let short = base
    .replace(VENDORS, (_, _vendor, version) => (keepVendorVersion && version ? version.trim() : ''))
    .replace(/\s+/g, ' ')
    .trim();
  if (!short) short = base.trim();
  const suffix = effort ? `·${effort}` : '';
  return cut(short, Math.max(4, MAX_LABEL - suffix.length)) + suffix;
}

/**
 * Nombre corto de cada modelo, para la línea de contexto: sin fabricante, con el esfuerzo a una
 * letra («Gemini 4 Argon (Low)» → «Argon·L», «Claude Opus 5.5 (High)» → «Opus 5.5·H»). Si dos
 * quedan iguales, se conserva la versión del fabricante para distinguirlos. Un modelo que no
 * reconocemos (otro fabricante, sin esfuerzo) se muestra con su nombre completo.
 */
export function shortModelLabels(names: string[]): string[] {
  const first = names.map((n) => label(n, false));
  return first.map((l, i) => (first.indexOf(l) !== first.lastIndexOf(l) ? label(names[i], true) : l));
}

export function shortModelLabel(name: string, all: string[] = [name]): string {
  const i = all.indexOf(name);
  return i >= 0 ? shortModelLabels(all)[i] : label(name, false);
}

/** Icono y nombre corto de un modo del agente. */
export function modeBadge(id: string, name: string): string {
  const key = `${id} ${name}`.toLowerCase();
  const icon = /plan/.test(key)
    ? '◐'
    : /auto|bypass|yolo/.test(key)
      ? '⚡'
      : /accept|edit/.test(key)
        ? '✎'
        : /ask|pregunt/.test(key)
          ? '?'
          : /code|program/.test(key)
            ? '⌨'
            : '•';
  const short = (name || id).replace(/\b(mode|modo)\b/gi, '').trim() || name || id;
  return `${icon} ${cut(short, 10)}`;
}
