// Rows (headers, query parameters, form fields) as plain text and back, for
// pasting many at once: one `Name: value` per line, switched-off rows as
// `// Name: value`.
//
// Reading is lenient, so lines copied from developer tools (`Accept: …`) and
// from URLs (`page=2`) both work: a line is split at its first `:` or `=`,
// whichever comes first – header names and parameter names contain
// neither, values may contain both (`Cookie: a=b`, `next=http://…`).

/** What text mode reads and writes. */
export interface TextRow {
  name: string;
  value: string;
  enabled: boolean;
}

/** Marks a switched-off row. */
const DISABLED = '//';
/** Separates name and value when writing. */
const SEPARATOR = ': ';

/** The rows as text, one per line (rows without name and value left out). */
export function rowsToText(rows: readonly TextRow[]): string {
  return rows
    .filter((row) => row.name.trim() !== '' || row.value.trim() !== '')
    .map((row) => `${row.enabled ? '' : `${DISABLED} `}${row.name}${SEPARATOR}${row.value}`)
    .join('\n');
}

/** The rows in `text`; empty lines are skipped. */
export function textToRows(text: string): TextRow[] {
  return text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line !== '' && line !== DISABLED)
    .map((line) => {
      const enabled = !line.startsWith(DISABLED);
      const content = enabled ? line : line.slice(DISABLED.length).trimStart();
      const at = splitAt(content);
      return at < 0
        ? { name: content, value: '', enabled }
        : {
            name: content.slice(0, at).trim(),
            value: content.slice(at + 1).trim(),
            enabled,
          };
    });
}

/** Index of the first `:` or `=`, or -1. */
function splitAt(line: string): number {
  const colon = line.indexOf(':');
  const equals = line.indexOf('=');
  if (colon < 0) return equals;
  if (equals < 0) return colon;
  return Math.min(colon, equals);
}
