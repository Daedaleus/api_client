// Remembering small UI settings between starts (open collections, chosen
// environments) in the web view's storage. Storage may be missing or throw
// (disabled); then nothing is remembered, but the app works the same.

export type KeyValueStorage = Pick<Storage, 'getItem' | 'setItem'>;

/** The stored JSON value under `key`, or `fallback` if missing or unreadable. */
export function readJson<T>(
  storage: KeyValueStorage | null,
  key: string,
  fallback: T,
  isValid: (value: unknown) => value is T,
): T {
  try {
    const value: unknown = JSON.parse(storage?.getItem(key) ?? 'null');
    return isValid(value) ? value : fallback;
  } catch {
    return fallback;
  }
}

/** Stores `value` as JSON under `key`; failures are ignored. */
export function writeJson(storage: KeyValueStorage | null, key: string, value: unknown): void {
  try {
    storage?.setItem(key, JSON.stringify(value));
  } catch {
    // Not remembered for the next start – still works now.
  }
}
