/**
 * Utilidades globales estándar provistas por Obsidian.
 */

/**
 * Normaliza una ruta eliminando barras dobles, puntos y estandarizando diagonales `/`.
 */
export function normalizePath(path: string): string {
  if (!path) return '';
  let result = path.replace(/\\/g, '/');
  result = result.replace(/\/+/g, '/');
  result = result.replace(/^\/+/, '');
  result = result.replace(/\/+$/, '');

  const segments = result.split('/');
  const normalized: string[] = [];
  for (const seg of segments) {
    if (seg === '.' || seg === '') continue;
    if (seg === '..') {
      normalized.pop();
    } else {
      normalized.push(seg);
    }
  }

  return normalized.join('/');
}

/**
 * Parsea una cadena con formato YAML en un objeto JavaScript.
 */
export function parseYaml(yaml: string): any {
  if (!yaml || typeof yaml !== 'string') return {};
  const clean = yaml.replace(/^---\s*\r?\n/, '').replace(/\r?\n---\s*$/, '');
  const lines = clean.split(/\r?\n/);
  const result: Record<string, any> = {};

  let currentKey: string | null = null;
  let currentArray: any[] | null = null;

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;

    // Manejo de elementos de array `- item`
    if (line.trimStart().startsWith('- ') && currentKey) {
      if (!currentArray) {
        currentArray = [];
        result[currentKey] = currentArray;
      }
      const itemVal = line.trimStart().slice(2).trim();
      currentArray.push(parsePrimitive(itemVal));
      continue;
    }

    const colonIdx = trimmed.indexOf(':');
    if (colonIdx === -1) continue;

    const key = trimmed.slice(0, colonIdx).trim();
    const rawVal = trimmed.slice(colonIdx + 1).trim();

    currentKey = key;
    currentArray = null;

    if (rawVal === '') {
      result[key] = {};
    } else {
      result[key] = parsePrimitive(rawVal);
    }
  }

  return result;
}

function parsePrimitive(rawVal: string): any {
  if (rawVal === 'true') return true;
  if (rawVal === 'false') return false;
  if (rawVal === 'null' || rawVal === '~') return null;
  if (!isNaN(Number(rawVal)) && rawVal !== '') return Number(rawVal);
  if ((rawVal.startsWith('"') && rawVal.endsWith('"')) || (rawVal.startsWith("'") && rawVal.endsWith("'"))) {
    return rawVal.slice(1, -1);
  }
  return rawVal;
}

/**
 * Convierte un objeto JavaScript en formato YAML legible.
 */
export function stringifyYaml(obj: any): string {
  if (!obj || typeof obj !== 'object') return String(obj);
  const lines: string[] = [];

  for (const [key, value] of Object.entries(obj)) {
    if (Array.isArray(value)) {
      lines.push(`${key}:`);
      for (const item of value) {
        lines.push(`  - ${item}`);
      }
    } else if (value !== null && typeof value === 'object') {
      lines.push(`${key}:`);
      for (const [subKey, subVal] of Object.entries(value)) {
        lines.push(`  ${subKey}: ${subVal}`);
      }
    } else {
      lines.push(`${key}: ${value}`);
    }
  }

  return lines.join('\n');
}

/**
 * Cliente HTTP ligero compatible con requestUrl de Obsidian.
 */
export async function requestUrl(params: string | {
  url: string;
  method?: string;
  headers?: Record<string, string>;
  body?: string | ArrayBuffer | Uint8Array;
}): Promise<{ status: number; text: string; json: any; arrayBuffer: ArrayBuffer }> {
  const url = typeof params === 'string' ? params : params.url;
  const method = typeof params === 'string' ? 'GET' : (params.method || 'GET');
  const headers = typeof params === 'string' ? undefined : params.headers;
  const body = typeof params === 'string' ? undefined : params.body;

  const res = await fetch(url, {
    method,
    headers,
    body: body as any,
  });

  const arrayBuffer = await res.arrayBuffer();
  const text = new TextDecoder().decode(arrayBuffer);
  let json: any = null;
  try {
    json = JSON.parse(text);
  } catch {}

  return {
    status: res.status,
    text,
    json,
    arrayBuffer,
  };
}

/**
 * Detección de plataforma de Obsidian (Platform).
 */
export const Platform = {
  isDesktop: true,
  isMacOS: typeof navigator !== 'undefined' && /Mac/i.test(navigator.userAgent),
  isWin: typeof navigator !== 'undefined' && /Win/i.test(navigator.userAgent),
  isLinux: typeof navigator !== 'undefined' && /Linux/i.test(navigator.userAgent),
  isIosApp: false,
  isAndroidApp: false,
  isMobile: false,
  isSafari: typeof navigator !== 'undefined' && /Safari/i.test(navigator.userAgent) && !/Chrome/i.test(navigator.userAgent),
};

/**
 * Interfaz e implementación de compatibilidad para Moment.js (utilizada en Excalidraw para marcas de tiempo).
 */
export interface MomentInstance {
  format(pattern?: string): string;
  valueOf(): number;
  toISOString(): string;
  toDate(): Date;
  add(amount: number, unit: string): MomentInstance;
  subtract(amount: number, unit: string): MomentInstance;
  clone(): MomentInstance;
  isValid(): boolean;
}

export function moment(dateInput?: string | number | Date | MomentInstance): MomentInstance {
  let d: Date;
  if (!dateInput) {
    d = new Date();
  } else if (dateInput instanceof Date) {
    d = new Date(dateInput.getTime());
  } else if (typeof dateInput === 'object' && 'toDate' in dateInput) {
    d = dateInput.toDate();
  } else {
    d = new Date(dateInput);
  }

  const pad = (n: number, len = 2) => String(n).padStart(len, '0');

  const instance: MomentInstance = {
    format(pattern = 'YYYY-MM-DDTHH:mm:ssZ'): string {
      const year = d.getFullYear();
      const month = d.getMonth() + 1;
      const date = d.getDate();
      const hours = d.getHours();
      const minutes = d.getMinutes();
      const seconds = d.getSeconds();
      const ms = d.getMilliseconds();

      return pattern
        .replace(/YYYY/g, String(year))
        .replace(/YY/g, String(year).slice(-2))
        .replace(/MM/g, pad(month))
        .replace(/DD/g, pad(date))
        .replace(/HH/g, pad(hours))
        .replace(/mm/g, pad(minutes))
        .replace(/ss/g, pad(seconds))
        .replace(/SSS/g, pad(ms, 3));
    },
    valueOf(): number {
      return d.getTime();
    },
    toISOString(): string {
      return d.toISOString();
    },
    toDate(): Date {
      return new Date(d.getTime());
    },
    add(amount: number, unit: string): MomentInstance {
      const copy = new Date(d.getTime());
      if (/day|d/i.test(unit)) copy.setDate(copy.getDate() + amount);
      else if (/month|m/i.test(unit)) copy.setMonth(copy.getMonth() + amount);
      else if (/year|y/i.test(unit)) copy.setFullYear(copy.getFullYear() + amount);
      else if (/hour|h/i.test(unit)) copy.setHours(copy.getHours() + amount);
      else if (/minute/i.test(unit)) copy.setMinutes(copy.getMinutes() + amount);
      else if (/second|s/i.test(unit)) copy.setSeconds(copy.getSeconds() + amount);
      return moment(copy);
    },
    subtract(amount: number, unit: string): MomentInstance {
      return this.add(-amount, unit);
    },
    clone(): MomentInstance {
      return moment(new Date(d.getTime()));
    },
    isValid(): boolean {
      return !isNaN(d.getTime());
    },
  };

  return instance;
}
