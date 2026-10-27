import { es } from './es';
import { en } from './en';

export type LocaleSchema<T> = {
  [K in keyof T]: T[K] extends object ? LocaleSchema<T[K]> : string;
};

export type TranslationSchema = LocaleSchema<typeof es>;
export type SupportedLocale = 'es' | 'en';

export const dictionaries: Record<SupportedLocale, TranslationSchema> = {
  es,
  en,
};

export type DeepKeys<T> = T extends object
  ? {
      [K in keyof T]: K extends string
        ? T[K] extends object
          ? `${K}.${DeepKeys<T[K]>}`
          : `${K}`
        : never;
    }[keyof T]
  : never;

export type TranslationKey = DeepKeys<typeof es>;
