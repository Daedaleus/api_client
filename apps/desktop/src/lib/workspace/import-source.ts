// Where an import comes from: per format the dialog to choose it in, with
// a file filter, so users see what they can import.

import type { FileFilter, PickOptions } from '../api';
import { i18n } from '../i18n/i18n.svelte';

/** The formats the app imports. */
export type ImportFormat = 'postman' | 'insomnia' | 'openapi' | 'har' | 'bruno';

/** The file a Bruno collection is recognised by, in its folder. */
export const BRUNO_MARKER = 'bruno.json';

interface Source {
  /** Title of the file or folder dialog. */
  title: string;
  /** File filter; `null` for a folder. */
  filter: { name: string; extensions: string[] } | null;
}

/** Texts are looked up when the dialog opens, in the current language. */
const SOURCES: Record<ImportFormat, Source> = {
  postman: {
    title: 'import.title_postman',
    filter: { name: 'import.filter_postman', extensions: ['json'] },
  },
  insomnia: {
    title: 'import.title_insomnia',
    filter: { name: 'import.filter_insomnia', extensions: ['json', 'yaml', 'yml'] },
  },
  har: {
    title: 'import.title_har',
    filter: { name: 'import.filter_har', extensions: ['har', 'json'] },
  },
  openapi: {
    title: 'import.title_openapi',
    filter: { name: 'import.filter_openapi', extensions: ['json', 'yaml', 'yml'] },
  },
  bruno: { title: 'import.title_bruno', filter: null },
};

/** Whether the source of `format` is a folder (else a file). */
export function isFolder(format: ImportFormat): boolean {
  return SOURCES[format].filter === null;
}

/** The dialog options for choosing the source of `format`. */
export function sourceOptions(format: ImportFormat): PickOptions {
  const { title, filter } = SOURCES[format];
  const filters: FileFilter[] = filter
    ? [
        { name: i18n.t(filter.name), extensions: filter.extensions },
        { name: i18n.t('import.filter_all'), extensions: ['*'] },
      ]
    : [];
  return { title: i18n.t(title), filters };
}

/** The dialog options for the folder of the new collection. */
export function targetOptions(): PickOptions {
  return { title: i18n.t('import.title_target') };
}

/**
 * The file the backend imports: the chosen file, or for a Bruno folder its
 * `bruno.json` (with the folder's own path separator).
 */
export function sourceFile(format: ImportFormat, picked: string): string {
  if (!isFolder(format)) return picked;
  const separator = picked.includes('\\') && !picked.includes('/') ? '\\' : '/';
  const folder = picked.endsWith(separator) ? picked.slice(0, -1) : picked;
  return `${folder}${separator}${BRUNO_MARKER}`;
}
