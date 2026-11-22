// Where an export goes: the save dialog with a file filter and a file name
// taken from the collection.

import type { SaveOptions } from '../api';
import { i18n } from '../i18n/i18n.svelte';

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
