import {
  dictionaries,
  type SupportedLocale,
  type TranslationKey,
  type TranslationSchema,
} from './locales';

const STORAGE_KEY = 'synapse_locale';

export const SUPPORTED_LOCALES: Array<{ id: SupportedLocale; label: string }> = [
  { id: 'es', label: 'Español' },
  { id: 'en', label: 'English' },
];

function getInitialLocale(): SupportedLocale {
  if (typeof window === 'undefined') return 'es';
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === 'es' || saved === 'en') {
      return saved;
    }
    if (navigator.language && navigator.language.toLowerCase().startsWith('en')) {
      return 'en';
    }
  } catch (e) {
    // fallback a español
  }
  return 'es';
}

class I18nManager {
  locale = $state<SupportedLocale>(getInitialLocale());

  setLocale(locale: SupportedLocale) {
    this.locale = locale;
    if (typeof localStorage !== 'undefined') {
      try {
        localStorage.setItem(STORAGE_KEY, locale);
      } catch (e) {
        console.warn('No se pudo guardar locale en localStorage:', e);
      }
    }
  }

  toggleLocale(): SupportedLocale {
    const next: SupportedLocale = this.locale === 'es' ? 'en' : 'es';
    this.setLocale(next);
    return next;
  }

  t(key: TranslationKey, params?: Record<string, string | number>): string {
    // Referenciar this.locale para registrar reactividad en Svelte 5 Runes
    const currentLocale = this.locale;
    const dict = dictionaries[currentLocale] || dictionaries.es;

    const segments = key.split('.');
    let value: any = dict;
    for (const segment of segments) {
      if (value && typeof value === 'object' && segment in value) {
        value = value[segment];
      } else {
        value = this.getFallbackValue(key);
        break;
      }
    }

    if (typeof value !== 'string') {
      value = this.getFallbackValue(key);
      if (typeof value !== 'string') {
        return key;
      }
    }

    if (params) {
      return value.replace(/\{([a-zA-Z0-9_]+)\}/g, (_: string, match: string) => {
        return params[match] !== undefined ? String(params[match]) : `{${match}}`;
      });
    }

    return value;
  }

  private getFallbackValue(key: TranslationKey): any {
    const segments = key.split('.');
    let value: any = dictionaries.es;
    for (const segment of segments) {
      if (value && typeof value === 'object' && segment in value) {
        value = value[segment];
      } else {
        return key;
      }
    }
    return value;
  }
}

export const i18n = new I18nManager();
export const t = (key: TranslationKey, params?: Record<string, string | number>) =>
  i18n.t(key, params);
