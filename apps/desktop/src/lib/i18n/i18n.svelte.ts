// Translations for the UI. All texts live in locales/app.json at the
// repository root (shared with Rust, which only hands out keys).

import rawMessages from '../../../../../locales/app.json';
import type { UserText } from '../types';

export const LOCALES = ['de', 'en'] as const;
export type Locale = (typeof LOCALES)[number];
export type Messages = Record<string, Record<Locale, string>>;

export const messages: Messages = rawMessages;

const FALLBACK: Locale = 'en';
const STORAGE_KEY = 'locale';

type LocaleStorage = Pick<Storage, 'getItem' | 'setItem'>;

export function isLocale(value: string): value is Locale {
  return (LOCALES as readonly string[]).includes(value);
}

/** First supported language from the user's preferences (`de-AT` counts as `de`). */
export function detectLocale(preferred: readonly string[]): Locale {
  for (const tag of preferred) {
    const base = tag.toLowerCase().split('-')[0] ?? '';
    if (isLocale(base)) {
      return base;
    }
  }
  return FALLBACK;
}

/** Replaces `{name}` placeholders; unknown placeholders stay visible. */
export function format(template: string, args: Record<string, string>): string {
  return template.replace(/\{(\w+)\}/g, (match, name: string) => args[name] ?? match);
}

/** The current language and the lookup of texts in it. */
export class I18n {
  locale = $state<Locale>(FALLBACK);
  readonly #storage: LocaleStorage | null;

  constructor(storage: LocaleStorage | null, preferred: readonly string[]) {
    this.#storage = storage;
    const stored = readStored(storage);
    this.locale = stored ?? detectLocale(preferred);
  }

  setLocale(locale: Locale): void {
    this.locale = locale;
    try {
      this.#storage?.setItem(STORAGE_KEY, locale);
    } catch {
      // Not persisted (e.g. storage disabled) – the choice still applies now.
    }
  }

  /** Text for `key` in the current language; the key itself if unknown. */
  t = (key: string, args: Record<string, string> = {}): string => {
    const entry = messages[key];
    return entry ? format(entry[this.locale], args) : key;
  };

  /** Text for a `UserText` from the backend. */
  text = (text: UserText): string => this.t(text.key, text.args);
}

function readStored(storage: LocaleStorage | null): Locale | null {
  try {
    const value = storage?.getItem(STORAGE_KEY);
    return value && isLocale(value) ? value : null;
  } catch {
    return null;
  }
}

function browserStorage(): LocaleStorage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

export const i18n = new I18n(browserStorage(), navigator.languages);
