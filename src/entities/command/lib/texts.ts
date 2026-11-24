import * as en from './en';
import type { CommandText } from './en';
import * as es from './es';

export type CommandId = keyof typeof en.commands;
export type CommandCategory = keyof typeof en.categories;
export type CommandLocale = 'en' | 'es';

const TEXTS = { en, es };

export function commandName(id: CommandId, locale: CommandLocale): string {
  return TEXTS[locale].commands[id].name;
}

export function commandCategory(category: CommandCategory, locale: CommandLocale): string {
  return TEXTS[locale].categories[category];
}

/** Alias con los que también se encuentra: sinónimos propios y todo lo del otro idioma. */
export function commandAliases(id: CommandId, category: CommandCategory, locale: CommandLocale): string[] {
  const other = locale === 'en' ? 'es' : 'en';
  const own: CommandText = TEXTS[locale].commands[id];
  const alias: CommandText = TEXTS[other].commands[id];
  return [...(own.keywords ?? []), alias.name, ...(alias.keywords ?? []), TEXTS[other].categories[category]];
}
