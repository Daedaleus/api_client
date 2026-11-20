// Preview of `{{variable}}` placeholders for display. The binding resolution
// happens in Rust when sending (ac_domain::environment::interpolate); this
// follows the same rules – names trimmed, unclosed `{{` kept, values not
// resolved again, the later enabled row wins – only to show what will be sent.

import type { VariableDraft } from './types';

const PLACEHOLDER = /\{\{([^}]*)\}\}/g;

export interface Preview {
  /** The text with every known placeholder replaced. */
  text: string;
  /** Names without a value, in order of appearance, each once. */
  missing: string[];
}

/** The enabled variables by name (later rows win). */
export function variableValues(variables: readonly VariableDraft[]): Map<string, string> {
  const values = new Map<string, string>();
  for (const variable of variables) {
    if (variable.enabled) values.set(variable.name.trim(), variable.value);
  }
  return values;
}

/** Whether `text` contains a placeholder at all. */
export function hasPlaceholders(text: string): boolean {
  return /\{\{[^}]*\}\}/.test(text);
}

/** `text` with placeholders filled from `values`; unknown names are listed. */
export function preview(text: string, values: ReadonlyMap<string, string>): Preview {
  const missing: string[] = [];
  const filled = text.replace(PLACEHOLDER, (placeholder, raw: string) => {
    const name = raw.trim();
    const value = values.get(name);
    if (value !== undefined) return value;
    if (!missing.includes(name)) missing.push(name);
    return placeholder;
  });
  return { text: filled, missing };
}
