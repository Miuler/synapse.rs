/** Utilidades para mostrar atajos de teclado según la plataforma del usuario. */

export function isMacPlatform(): boolean {
  if (typeof navigator === 'undefined') return false;
  return /mac|iphone|ipad/i.test(navigator.platform || navigator.userAgent || '');
}

const MAC_KEYS: Record<string, string> = {
  ctrl: '⌘',
  control: '⌘',
  alt: '⌥',
  shift: '⇧',
  enter: '↵',
  escape: 'Esc',
};

const COMMON_KEYS: Record<string, string> = {
  arrowleft: '←',
  arrowright: '→',
  arrowup: '↑',
  arrowdown: '↓',
  pageup: 'PgUp',
  pagedown: 'PgDn',
  escape: 'Esc',
  enter: '↵',
};

export function formatKey(key: string, mac = isMacPlatform()): string {
  const lower = key.trim().toLowerCase();
  if (mac && MAC_KEYS[lower]) return MAC_KEYS[lower];
  if (COMMON_KEYS[lower]) return COMMON_KEYS[lower];
  return key.length === 1 ? key.toUpperCase() : key;
}

/**
 * Convierte "Ctrl+W, Ctrl+F4" en alternativas ([["Ctrl","W"],["Ctrl","F4"]]) ya formateadas.
 * Una "+" final (p. ej. "Ctrl++") se interpreta como la tecla "+".
 */
export function parseShortcut(shortcut: string, mac = isMacPlatform()): string[][] {
  return shortcut
    .split(/\s*,\s+|\s*,(?=[A-Za-z])/)
    .map((alt) => alt.trim())
    .filter(Boolean)
    .map((alt) => {
      const parts = alt.endsWith('++') ? [...alt.slice(0, -2).split('+'), '+'] : alt.split('+');
      return parts.filter(Boolean).map((k) => formatKey(k, mac));
    });
}

/** Solo la combinación principal (la primera), como texto: "Ctrl+W" o "⌘W" en macOS. */
export function primaryShortcutText(shortcut: string | undefined, mac = isMacPlatform()): string {
  if (!shortcut) return '';
  const first = parseShortcut(shortcut, mac)[0] ?? [];
  return mac ? first.join('') : first.join('+');
}
