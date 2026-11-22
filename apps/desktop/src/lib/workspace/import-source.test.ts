import { beforeEach, describe, expect, it } from 'vitest';

import { i18n } from '../i18n/i18n.svelte';
import { isFolder, sourceFile, sourceOptions } from './import-source';

describe('import sources', () => {
  beforeEach(() => i18n.setLocale('de'));

  it('are files except for Bruno, which is chosen by its folder', () => {
    expect(isFolder('postman')).toBe(false);
    expect(isFolder('openapi')).toBe(false);
    expect(isFolder('bruno')).toBe(true);
    expect(sourceFile('postman', '/x/shop.json')).toBe('/x/shop.json');
    expect(sourceFile('bruno', '/home/me/shop')).toBe('/home/me/shop/bruno.json');
    expect(sourceFile('bruno', '/home/me/shop/')).toBe('/home/me/shop/bruno.json');
    expect(sourceFile('bruno', 'C:\\apis\\shop')).toBe('C:\\apis\\shop\\bruno.json');
  });

  it('name their dialogs and filters in the current language', () => {
    expect(sourceOptions('postman')).toEqual({
      title: 'Postman-Collection importieren',
      filters: [
        { name: 'Postman-Collection (JSON)', extensions: ['json'] },
        { name: 'Alle Dateien', extensions: ['*'] },
      ],
    });
    expect(sourceOptions('har').filters?.[0]).toEqual({
      name: 'HAR-Datei',
      extensions: ['har', 'json'],
    });
    expect(sourceOptions('insomnia').title).toBe('Insomnia-Export importieren');
  });
});
