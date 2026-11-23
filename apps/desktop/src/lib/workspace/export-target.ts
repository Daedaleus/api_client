// Where an export or a saved response goes: the save dialog with a file
// filter and a suggested file name.

import type { SaveOptions } from '../api';
import { i18n } from '../i18n/i18n.svelte';
import type { HeaderView } from '../types';

/** File endings for common response content types (without parameters). */
const ENDINGS: Record<string, string> = {
  'application/json': 'json',
  'application/xml': 'xml',
  'text/xml': 'xml',
  'text/html': 'html',
  'text/plain': 'txt',
  'text/csv': 'csv',
  'application/pdf': 'pdf',
  'image/png': 'png',
  'image/jpeg': 'jpg',
  'image/gif': 'gif',
  'image/svg+xml': 'svg',
  'application/zip': 'zip',
};

/** The save dialog for a response body with these headers. */
export function responseOptions(headers: readonly HeaderView[]): SaveOptions {
  const contentType = headers
    .find((header) => header.name.toLowerCase() === 'content-type')
    ?.value.split(';')[0]
    ?.trim()
    .toLowerCase();
  const ending =
    (contentType && (ENDINGS[contentType] ?? (contentType.endsWith('+json') ? 'json' : null))) ??
    'bin';
  return {
    title: i18n.t('response.save_title'),
    filters: [{ name: i18n.t('import.filter_all'), extensions: ['*'] }],
    defaultPath: `response.${ending}`,
  };
}

/** Ending of an exported Postman collection, as Postman names its files. */
export const POSTMAN_SUFFIX = '.postman_collection.json';

/** Characters no file name may contain on Windows, macOS or Linux. */
const UNSAFE = /[<>:"/\\|?*]/g;

/** The save dialog for exporting the collection `name` (texts translated now). */
export function exportOptions(name: string): SaveOptions {
  const base = name.replace(UNSAFE, '_').trim() || 'collection';
  return {
    title: i18n.t('export.title_postman'),
    filters: [{ name: i18n.t('import.filter_postman'), extensions: ['json'] }],
    defaultPath: `${base}${POSTMAN_SUFFIX}`,
  };
}
