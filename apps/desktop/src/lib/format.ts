const UNITS = ['B', 'KB', 'MB', 'GB'] as const;
const STEP = 1024;

/** Human-readable size, e.g. `1.2 KB`. */
export function formatBytes(bytes: number): string {
  let value = bytes;
  let unit = 0;
  while (value >= STEP && unit < UNITS.length - 1) {
    value /= STEP;
    unit += 1;
  }
  const digits = unit === 0 ? 0 : 1;
  return `${value.toFixed(digits)} ${UNITS[unit]}`;
}

/** Human-readable duration, e.g. `142 ms` or `1.25 s`. */
export function formatDuration(milliseconds: number): string {
  if (milliseconds < 1000) {
    return `${milliseconds} ms`;
  }
  return `${(milliseconds / 1000).toFixed(2)} s`;
}

/**
 * Lines longer than this are shown as plain text instead of in the code
 * viewer: it draws only the visible lines, but always whole lines, and a
 * megabyte on one line (a minified body) would freeze it.
 */
export const MAX_CODE_LINE = 20_000;

/** Whether no line of `text` is longer than {@link MAX_CODE_LINE}. */
export function fitsCodeView(text: string): boolean {
  let start = 0;
  while (start <= text.length) {
    const end = text.indexOf('\n', start);
    const lineEnd = end < 0 ? text.length : end;
    if (lineEnd - start > MAX_CODE_LINE) return false;
    if (end < 0) return true;
    start = end + 1;
  }
  return true;
}
