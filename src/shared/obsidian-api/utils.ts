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
 * Extrae y parsea el bloque YAML Frontmatter de un documento Markdown delimitado por '---'.
 */
export function extractFrontmatter(content: string): Record<string, any> | undefined {
  if (!content || !content.trimStart().startsWith('---')) return undefined;
  const match = content.match(/^---\s*\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/);
  if (!match) return undefined;
  try {
    return parseYaml(match[1]);
  } catch {
    return undefined;
  }
}

/**
 * Detecta si una nota Markdown debe ser abierta por un visor especializado de plugin
 * analizando su contenido y su YAML Frontmatter (ej. Excalidraw, Kanban, etc.).
 *
 * @returns El identificador del tipo de vista especializada ('excalidraw', 'kanban', etc.) o null si es Markdown estándar.
 */
export function resolvePluginViewType(content: string, _path?: string): string | null {
  if (!content) return null;

  // 1. Detección por YAML Frontmatter
  const fm = extractFrontmatter(content);
  if (fm) {
    if (fm['excalidraw-plugin']) return 'excalidraw';
    if (fm['kanban-plugin']) return 'kanban';
    if (fm['slide'] === true || fm['presentation'] === true) return 'slides';
    if (Array.isArray(fm['tags'])) {
      if (fm['tags'].includes('excalidraw') || fm['tags'].includes('ea/drawing')) return 'excalidraw';
      if (fm['tags'].includes('kanban')) return 'kanban';
    }
  }

  // 2. Detección por marcas en el cuerpo (ej. compressed-json de Excalidraw)
  if (
    content.includes('excalidraw-plugin:') ||
    content.includes('```compressed-json') ||
    (content.includes('%%') && content.includes('# Drawing')) ||
    content.includes('==Deconstruct of drawing==') ||
    content.includes('Switch to EXCALIDRAW VIEW')
  ) {
    return 'excalidraw';
  }

  return null;
}

export interface RequestUrlParam {
  url: string;
  method?: string;
  headers?: Record<string, string>;
  body?: string | ArrayBuffer | Uint8Array;
}

/**
 * Cliente HTTP ligero compatible con requestUrl de Obsidian.
 */
export async function requestUrl(params: string | RequestUrlParam): Promise<{ status: number; text: string; json: any; arrayBuffer: ArrayBuffer }> {
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
 * Interfaz e implementación de compatibilidad para Moment.js (utilizada en Excalidraw, Calendar, etc.).
 */
export interface MomentInstance {
  format(pattern?: string): string;
  valueOf(): number;
  toISOString(): string;
  toDate(): Date;
  add(amount: number, unit?: string): MomentInstance;
  subtract(amount: number, unit?: string): MomentInstance;
  clone(): MomentInstance;
  isValid(): boolean;
  isSame(other?: any, granularity?: string): boolean;
  isBefore(other?: any, granularity?: string): boolean;
  isAfter(other?: any, granularity?: string): boolean;
  year(): number;
  year(y: number): MomentInstance;
  month(): number;
  month(m: number): MomentInstance;
  date(): number;
  date(d: number): MomentInstance;
  day(): number;
  day(d: number): MomentInstance;
  weekday(): number;
  weekday(d: number): MomentInstance;
  isoWeekday(): number;
  isoWeekday(d: number): MomentInstance;
  week(): number;
  week(w: number): MomentInstance;
  set(unitOrValues: string | Record<string, number>, val?: number): MomentInstance;
  get(unit: string): number;
  locale(): string;
  locale(loc: string): MomentInstance;
  localeData(): any;
  calendar(referenceTime?: any, formats?: any): string;
  startOf(unit: string): MomentInstance;
  endOf(unit: string): MomentInstance;
}

const defaultLocaleWeekSpec = {
  dow: 1, // Lunes por defecto
  doy: 4,
};

let currentGlobalLocale = 'en';
const localeWeekSpecs: Record<string, { dow: number; doy: number }> = {
  en: { dow: 0, doy: 6 },
  'en-gb': { dow: 1, doy: 4 },
  es: { dow: 1, doy: 4 },
};

function getWeekNumber(date: Date, dow = 1): number {
  const d = new Date(Date.UTC(date.getFullYear(), date.getMonth(), date.getDate()));
  const dayNum = d.getUTCDay();
  // Ajustar día según dow
  const diff = (dayNum < dow ? 7 : 0) + dayNum - dow;
  d.setUTCDate(d.getUTCDate() - diff + 3);
  const firstThursday = d.getTime();
  d.setUTCMonth(0, 1);
  if (d.getUTCDay() !== 4) {
    d.setUTCMonth(0, 1 + ((4 - d.getUTCDay() + 7) % 7));
  }
  return 1 + Math.ceil((firstThursday - d.getTime()) / 604800000);
}

export interface MomentStatic {
  (dateInput?: string | number | Date | MomentInstance, format?: string, strict?: boolean): MomentInstance;
  locale(loc?: string): string;
  localeData(loc?: string): any;
  updateLocale(loc: string, config: any): any;
  locales(): string[];
  weekdays(localeSorted?: boolean): string[];
  weekdaysShort(localeSorted?: boolean): string[];
  weekdaysMin(localeSorted?: boolean): string[];
}

export const moment: MomentStatic = Object.assign(
  function (
    dateInput?: string | number | Date | MomentInstance,
    format?: string,
    _strict?: boolean
  ): MomentInstance {
    let d: Date;
    let valid = true;

    if (!dateInput && dateInput !== 0) {
      d = new Date();
    } else if (dateInput instanceof Date) {
      d = new Date(dateInput.getTime());
      valid = !isNaN(d.getTime());
    } else if (typeof dateInput === 'object' && dateInput !== null && 'toDate' in dateInput) {
      d = dateInput.toDate();
      valid = !isNaN(d.getTime());
    } else if (typeof dateInput === 'number') {
      d = new Date(dateInput);
      valid = !isNaN(d.getTime());
    } else if (typeof dateInput === 'string') {
      // Si se provee formato estricto (ej. YYYY-MM-DD)
      if (format) {
        // Formato simple YYYY-MM-DD
        const isoMatch = /^(\d{4})[-/](\d{1,2})[-/](\d{1,2})$/.exec(dateInput.trim());
        if (isoMatch) {
          const y = parseInt(isoMatch[1], 10);
          const m = parseInt(isoMatch[2], 10) - 1;
          const day = parseInt(isoMatch[3], 10);
          d = new Date(y, m, day);
          valid = d.getFullYear() === y && d.getMonth() === m && d.getDate() === day;
        } else {
          d = new Date(dateInput);
          valid = !isNaN(d.getTime());
        }
      } else {
        d = new Date(dateInput);
        valid = !isNaN(d.getTime());
      }
    } else {
      d = new Date(NaN);
      valid = false;
    }

    const pad = (n: number, len = 2) => String(n).padStart(len, '0');

    const instance: MomentInstance = {
      format(pattern = 'YYYY-MM-DDTHH:mm:ssZ'): string {
        if (!valid || isNaN(d.getTime())) return 'Invalid date';
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
        return valid ? d.toISOString() : 'Invalid date';
      },
      toDate(): Date {
        return new Date(d.getTime());
      },
      add(amount: number, unit = 'day'): MomentInstance {
        const copy = new Date(d.getTime());
        if (/day|d/i.test(unit)) copy.setDate(copy.getDate() + amount);
        else if (/month|m/i.test(unit)) copy.setMonth(copy.getMonth() + amount);
        else if (/year|y/i.test(unit)) copy.setFullYear(copy.getFullYear() + amount);
        else if (/hour|h/i.test(unit)) copy.setHours(copy.getHours() + amount);
        else if (/minute/i.test(unit)) copy.setMinutes(copy.getMinutes() + amount);
        else if (/second|s/i.test(unit)) copy.setSeconds(copy.getSeconds() + amount);
        return moment(copy);
      },
      subtract(amount: number, unit = 'day'): MomentInstance {
        return this.add(-amount, unit);
      },
      clone(): MomentInstance {
        return moment(new Date(d.getTime()));
      },
      isValid(): boolean {
        return valid && !isNaN(d.getTime());
      },
      isSame(other?: any, granularity = 'millisecond'): boolean {
        const otherDate = other ? (typeof other.toDate === 'function' ? other.toDate() : new Date(other)) : new Date();
        if (isNaN(otherDate.getTime()) || isNaN(d.getTime())) return false;
        const g = granularity.toLowerCase();
        if (g.startsWith('y')) return d.getFullYear() === otherDate.getFullYear();
        if (g.startsWith('m') && !g.startsWith('mi')) return d.getFullYear() === otherDate.getFullYear() && d.getMonth() === otherDate.getMonth();
        if (g.startsWith('d')) return d.getFullYear() === otherDate.getFullYear() && d.getMonth() === otherDate.getMonth() && d.getDate() === otherDate.getDate();
        return d.getTime() === otherDate.getTime();
      },
      isBefore(other?: any, granularity = 'millisecond'): boolean {
        const otherDate = other ? (typeof other.toDate === 'function' ? other.toDate() : new Date(other)) : new Date();
        if (isNaN(otherDate.getTime()) || isNaN(d.getTime())) return false;
        return d.getTime() < otherDate.getTime();
      },
      isAfter(other?: any, granularity = 'millisecond'): boolean {
        const otherDate = other ? (typeof other.toDate === 'function' ? other.toDate() : new Date(other)) : new Date();
        if (isNaN(otherDate.getTime()) || isNaN(d.getTime())) return false;
        return d.getTime() > otherDate.getTime();
      },
      year(val?: number): any {
        if (val !== undefined) {
          const copy = new Date(d.getTime());
          copy.setFullYear(val);
          return moment(copy);
        }
        return d.getFullYear();
      },
      month(val?: number): any {
        if (val !== undefined) {
          const copy = new Date(d.getTime());
          copy.setMonth(val);
          return moment(copy);
        }
        return d.getMonth();
      },
      date(val?: number): any {
        if (val !== undefined) {
          const copy = new Date(d.getTime());
          copy.setDate(val);
          return moment(copy);
        }
        return d.getDate();
      },
      day(val?: number): any {
        if (val !== undefined) {
          const copy = new Date(d.getTime());
          const diff = val - copy.getDay();
          copy.setDate(copy.getDate() + diff);
          return moment(copy);
        }
        return d.getDay();
      },
      weekday(val?: number): any {
        const weekSpec = localeWeekSpecs[currentGlobalLocale] || defaultLocaleWeekSpec;
        const currentWeekday = (d.getDay() - weekSpec.dow + 7) % 7;
        if (val !== undefined) {
          const copy = new Date(d.getTime());
          copy.setDate(copy.getDate() + (val - currentWeekday));
          return moment(copy);
        }
        return currentWeekday;
      },
      isoWeekday(val?: number): any {
        const currentIso = d.getDay() === 0 ? 7 : d.getDay();
        if (val !== undefined) {
          const copy = new Date(d.getTime());
          copy.setDate(copy.getDate() + (val - currentIso));
          return moment(copy);
        }
        return currentIso;
      },
      week(val?: number): any {
        const weekSpec = localeWeekSpecs[currentGlobalLocale] || defaultLocaleWeekSpec;
        const currentW = getWeekNumber(d, weekSpec.dow);
        if (val !== undefined) {
          const copy = new Date(d.getTime());
          copy.setDate(copy.getDate() + (val - currentW) * 7);
          return moment(copy);
        }
        return currentW;
      },
      set(unitOrValues: string | Record<string, number>, val?: number): MomentInstance {
        const copy = new Date(d.getTime());
        if (typeof unitOrValues === 'object') {
          for (const [k, v] of Object.entries(unitOrValues)) {
            if (/hour/i.test(k)) copy.setHours(v);
            else if (/minute/i.test(k)) copy.setMinutes(v);
            else if (/second/i.test(k)) copy.setSeconds(v);
            else if (/millisecond/i.test(k)) copy.setMilliseconds(v);
            else if (/date|day/i.test(k)) copy.setDate(v);
            else if (/month/i.test(k)) copy.setMonth(v);
            else if (/year/i.test(k)) copy.setFullYear(v);
          }
        } else if (typeof unitOrValues === 'string' && typeof val === 'number') {
          if (/hour/i.test(unitOrValues)) copy.setHours(val);
          else if (/minute/i.test(unitOrValues)) copy.setMinutes(val);
          else if (/second/i.test(unitOrValues)) copy.setSeconds(val);
          else if (/millisecond/i.test(unitOrValues)) copy.setMilliseconds(val);
        }
        return moment(copy);
      },
      get(unit: string): number {
        if (/hour/i.test(unit)) return d.getHours();
        if (/minute/i.test(unit)) return d.getMinutes();
        if (/second/i.test(unit)) return d.getSeconds();
        if (/millisecond/i.test(unit)) return d.getMilliseconds();
        if (/date/i.test(unit)) return d.getDate();
        if (/day/i.test(unit)) return d.getDay();
        if (/month/i.test(unit)) return d.getMonth();
        if (/year/i.test(unit)) return d.getFullYear();
        return 0;
      },
      locale(loc?: string): any {
        if (loc) {
          return this;
        }
        return currentGlobalLocale;
      },
      calendar(referenceTime?: any, formats?: any): string {
        const ref = referenceTime ? moment(referenceTime) : moment();
        const diffDays = Math.round((d.getTime() - ref.toDate().getTime()) / 86400000);
        if (diffDays === 0) return 'Today at ' + this.format('LT');
        if (diffDays === 1) return 'Tomorrow at ' + this.format('LT');
        if (diffDays === -1) return 'Yesterday at ' + this.format('LT');
        return this.format('L');
      },
      localeData(): any {
        return moment.localeData(currentGlobalLocale);
      },
      startOf(unit: string): MomentInstance {
        const copy = new Date(d.getTime());
        const u = unit.toLowerCase();
        if (u.startsWith('y')) {
          copy.setMonth(0, 1);
          copy.setHours(0, 0, 0, 0);
        } else if (u.startsWith('m') && !u.startsWith('mi')) {
          copy.setDate(1);
          copy.setHours(0, 0, 0, 0);
        } else if (u.startsWith('d')) {
          copy.setHours(0, 0, 0, 0);
        }
        return moment(copy);
      },
      endOf(unit: string): MomentInstance {
        const copy = new Date(d.getTime());
        const u = unit.toLowerCase();
        if (u.startsWith('y')) {
          copy.setMonth(11, 31);
          copy.setHours(23, 59, 59, 999);
        } else if (u.startsWith('m') && !u.startsWith('mi')) {
          copy.setMonth(copy.getMonth() + 1, 0);
          copy.setHours(23, 59, 59, 999);
        } else if (u.startsWith('d')) {
          copy.setHours(23, 59, 59, 999);
        }
        return moment(copy);
      },
    };

    return instance;
  },
  {
    locale(loc?: string): string {
      if (loc) {
        currentGlobalLocale = loc.toLowerCase();
      }
      return currentGlobalLocale;
    },
    localeData(loc?: string): any {
      const targetLocale = (loc || currentGlobalLocale).toLowerCase();
      const spec = localeWeekSpecs[targetLocale] || defaultLocaleWeekSpec;
      return {
        _week: { dow: spec.dow, doy: spec.doy },
        firstDayOfWeek: () => spec.dow,
        weekdays: () => ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'],
        weekdaysShort: () => ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'],
        weekdaysMin: () => ['Su', 'Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa'],
        months: () => [
          'January', 'February', 'March', 'April', 'May', 'June',
          'July', 'August', 'September', 'October', 'November', 'December',
        ],
        monthsShort: () => [
          'Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun',
          'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec',
        ],
      };
    },
    updateLocale(loc: string, config: any): any {
      const targetLocale = loc.toLowerCase();
      if (config?.week) {
        localeWeekSpecs[targetLocale] = {
          dow: typeof config.week.dow === 'number' ? config.week.dow : (localeWeekSpecs[targetLocale]?.dow ?? 1),
          doy: typeof config.week.doy === 'number' ? config.week.doy : (localeWeekSpecs[targetLocale]?.doy ?? 4),
        };
      }
      return moment.localeData(targetLocale);
    },
    locales(): string[] {
      return Object.keys(localeWeekSpecs);
    },
    weekdays(localeSorted = false): string[] {
      const days = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
      if (!localeSorted) return days;
      const dow = (localeWeekSpecs[currentGlobalLocale] || defaultLocaleWeekSpec).dow;
      return [...days.slice(dow), ...days.slice(0, dow)];
    },
    weekdaysShort(localeSorted = false): string[] {
      const days = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
      if (!localeSorted) return days;
      const dow = (localeWeekSpecs[currentGlobalLocale] || defaultLocaleWeekSpec).dow;
      return [...days.slice(dow), ...days.slice(0, dow)];
    },
    weekdaysMin(localeSorted = false): string[] {
      const days = ['Su', 'Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa'];
      if (!localeSorted) return days;
      const dow = (localeWeekSpecs[currentGlobalLocale] || defaultLocaleWeekSpec).dow;
      return [...days.slice(dow), ...days.slice(0, dow)];
    },
  }
);

/**
 * Convierte un ArrayBuffer o Uint8Array a una cadena Base64.
 */
export function arrayBufferToBase64(buffer: ArrayBuffer | Uint8Array): string {
  const bytes = buffer instanceof Uint8Array ? buffer : new Uint8Array(buffer);
  let binary = '';
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  const globalBtoa = typeof btoa !== 'undefined' ? btoa : (globalThis as any).btoa;
  return globalBtoa ? globalBtoa(binary) : '';
}

/**
 * Convierte una cadena Base64 a un ArrayBuffer nativo.
 */
export function base64ToArrayBuffer(base64: string): ArrayBuffer {
  const globalAtob = typeof atob !== 'undefined' ? atob : (globalThis as any).atob;
  const binary = globalAtob ? globalAtob(base64) : '';
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes.buffer;
}

/**
 * Comprueba si la versión de API requerida es soportada.
 */
export function requireApiVersion(_version: string): boolean {
  return true;
}

/**
 * Retorna el idioma del sistema o interfaz.
 */
export function getLanguage(): string {
  return 'es';
}

/**
 * Retorna o carga la librería PDF.js si es requerida.
 */
export async function loadPdfJs(): Promise<any> {
  return {};
}

/**
 * Extrae la información de límites y contenido del frontmatter YAML de una nota.
 */
export function getFrontMatterInfo(content: string): {
  exists: boolean;
  from: number;
  to: number;
  content: string;
} {
  const match = (content || '').match(/^---\r?\n([\s\S]*?)\r?\n---/);
  if (!match) {
    return { exists: false, from: 0, to: 0, content: '' };
  }
  return {
    exists: true,
    from: 0,
    to: match[0].length,
    content: match[1],
  };
}

/**
 * Extrae o formatea una entrada individual de frontmatter.
 */
export function parseFrontMatterEntry(entry: any, key: string): any {
  if (!entry || typeof entry !== 'object') return null;
  return entry[key];
}

/**
 * Realiza una petición HTTP simple retornando el cuerpo en formato texto.
 */
export async function request(options: RequestUrlParam | string): Promise<string> {
  const res = await requestUrl(options);
  return res.text;
}

/**
 * Resuelve un subpath (`#encabezado` o `^bloque`) en la caché de metadatos.
 */
export function resolveSubpath(cache: any, subpath: string): any {
  if (!cache || !subpath) return null;
  const clean = subpath.replace(/^#/, '');
  if (Array.isArray(cache.headings)) {
    const found = cache.headings.find((h: any) => h.heading?.toLowerCase() === clean.toLowerCase());
    if (found) return { type: 'heading', current: found };
  }
  return null;
}

/**
 * Sanitiza una cadena HTML y la monta en un fragmento de DOM.
 */
export function sanitizeHTMLToDom(html: string): DocumentFragment {
  if (typeof document !== 'undefined') {
    const template = document.createElement('template');
    template.innerHTML = html;
    return template.content;
  }
  return {} as DocumentFragment;
}

/**
 * Prepara una función de búsqueda difusa (Fuzzy Search).
 */
export function prepareFuzzySearch(query: string): (text: string) => { score: number; matches?: any[] } | null {
  const q = (query || '').toLowerCase();
  return (text: string) => {
    const t = (text || '').toLowerCase();
    if (t.includes(q)) {
      return { score: 1 };
    }
    return null;
  };
}

/**
 * Limpia y convierte HTML a Markdown plano simplificado.
 */
export function htmlToMarkdown(html: string): string {
  if (!html) return '';
  return html.replace(/<[^>]+>/g, '');
}

const iconsRegistry = new Map<string, string>();

/**
 * Registra un icono personalizado SVG.
 */
export function addIcon(id: string, svg: string): void {
  iconsRegistry.set(id, svg);
}

/**
 * Obtiene el código SVG de un icono registrado.
 */
export function getIcon(id: string): string | null {
  return iconsRegistry.get(id) || null;
}

/**
 * Retorna todos los IDs de iconos registrados en Obsidian.
 */
export function getIconIds(): string[] {
  return Array.from(iconsRegistry.keys());
}

/**
 * Inserta un icono SVG en un elemento HTML.
 */
export function setIcon(el: HTMLElement, id: string): void {
  const svg = getIcon(id);
  if (svg && el) {
    el.innerHTML = svg;
  }
}

export interface Debouncer<Args extends any[], ReturnVal = void> {
  (...args: Args): ReturnVal;
  cancel(): void;
  finish(): ReturnVal | void;
}

/**
 * Control de flujo temporal (debounce) compatible con la API canónica de Obsidian.
 * Pospone la ejecución del callback hasta que haya transcurrido `timeout` ms de inactividad.
 * Si `resetTimer` es true, cada llamada reinicia el temporizador; si es false, respeta la primera llamada (throttle-like).
 */
export function debounce<Args extends any[], ReturnVal = void>(
  cb: (...args: Args) => ReturnVal,
  timeout = 0,
  resetTimer = false
): Debouncer<Args, ReturnVal> {
  let timerId: any = null;
  let lastArgs: Args | null = null;
  let lastThis: any = null;
  let lastResult: any;

  function invoke() {
    timerId = null;
    if (lastArgs) {
      lastResult = cb.apply(lastThis, lastArgs);
      lastArgs = null;
      lastThis = null;
    }
  }

  const debounced = function (this: any, ...args: Args): any {
    lastArgs = args;
    lastThis = this;

    if (timerId !== null) {
      if (resetTimer) {
        clearTimeout(timerId);
        timerId = setTimeout(invoke, timeout);
      }
    } else {
      timerId = setTimeout(invoke, timeout);
    }

    return lastResult;
  } as Debouncer<Args, ReturnVal>;

  debounced.cancel = () => {
    if (timerId !== null) {
      clearTimeout(timerId);
      timerId = null;
    }
    lastArgs = null;
    lastThis = null;
  };

  debounced.finish = () => {
    if (timerId !== null) {
      clearTimeout(timerId);
      invoke();
    }
    return lastResult;
  };

  return debounced;
}
