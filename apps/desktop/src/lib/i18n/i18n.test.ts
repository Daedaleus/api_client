import { describe, expect, it, vi } from 'vitest';

import { I18n, LOCALES, detectLocale, format, i18n, messages } from './i18n.svelte';

function placeholders(text: string): string[] {
  return [...text.matchAll(/\{(\w+)\}/g)].map((match) => match[1] ?? '').sort();
}

function memoryStorage(initial: Record<string, string> = {}) {
  const data = new Map(Object.entries(initial));
  return {
    getItem: (key: string) => data.get(key) ?? null,
    setItem: (key: string, value: string) => void data.set(key, value),
    data,
  };
}

describe('translation file', () => {
  it('has every language with the same placeholders for each key', () => {
    for (const [key, entry] of Object.entries(messages)) {
      const reference = placeholders(entry.en);
      for (const locale of LOCALES) {
        expect(typeof entry[locale], `${key} (${locale})`).toBe('string');
        expect(placeholders(entry[locale]), `${key} (${locale})`).toEqual(reference);
      }
    }
  });

  it('contains every key used in the sources', () => {
    const sources = import.meta.glob<string>('/src/**/*.{ts,svelte}', {
      query: '?raw',
      import: 'default',
      eager: true,
    });
    const used = new Set<string>();
    for (const [file, source] of Object.entries(sources)) {
      if (file.endsWith('.test.ts')) continue;
      for (const match of source.matchAll(/\bt\(\s*'([\w.]+)'/g)) {
        used.add(match[1] ?? '');
      }
      // Keys built from a list, e.g. t(`response.${name}`), are listed here.
      for (const match of source.matchAll(/\bt\(\s*`([\w.]+)\$\{/g)) {
        for (const key of Object.keys(messages)) {
          if (key.startsWith(match[1] ?? '')) used.add(key);
        }
      }
    }
    expect(used.size).toBeGreaterThan(5);
    for (const key of used) {
      expect(messages, key).toHaveProperty([key]);
    }
  });
});

describe('detectLocale', () => {
  it('takes the first supported language, ignoring regions', () => {
    expect(detectLocale(['fr-FR', 'de-AT', 'en'])).toBe('de');
    expect(detectLocale(['EN-us'])).toBe('en');
  });

  it('falls back to English', () => {
    expect(detectLocale(['fr', 'it'])).toBe('en');
    expect(detectLocale([])).toBe('en');
  });
});

describe('format', () => {
  it('fills placeholders and keeps unknown ones', () => {
    expect(format('{a} and {b}', { a: '1' })).toBe('1 and {b}');
  });
});

describe('I18n', () => {
  it('prefers the stored language over the system language', () => {
    expect(new I18n(memoryStorage({ locale: 'de' }), ['en']).locale).toBe('de');
    expect(new I18n(memoryStorage({ locale: 'xx' }), ['de']).locale).toBe('de');
    expect(new I18n(null, ['de-DE']).locale).toBe('de');
  });

  it('stores a changed language and translates with it', () => {
    const storage = memoryStorage();
    const texts = new I18n(storage, ['en']);
    expect(texts.t('request.send')).toBe('Send');
    texts.setLocale('de');
    expect(storage.data.get('locale')).toBe('de');
    expect(texts.t('request.send')).toBe('Senden');
    expect(texts.text({ key: 'error.connect', args: { detail: 'x' } })).toBe(
      'Verbindung fehlgeschlagen: x',
    );
  });

  it('shows the key for unknown texts', () => {
    expect(new I18n(null, ['en']).t('no.such.key')).toBe('no.such.key');
  });

  it('survives storage that throws', () => {
    const broken = {
      getItem: vi.fn(() => {
        throw new Error('denied');
      }),
      setItem: vi.fn(() => {
        throw new Error('denied');
      }),
    };
    const texts = new I18n(broken, ['de']);
    expect(texts.locale).toBe('de');
    texts.setLocale('en');
    expect(texts.locale).toBe('en');
  });

  it('has a shared instance for the app', () => {
    expect(LOCALES).toContain(i18n.locale);
  });
});
