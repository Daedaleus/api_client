// Preview of `{{variable}}` placeholders for display. The binding resolution
// happens in Rust when sending (ac_domain::environment::interpolate); this
// follows the same rules – names trimmed, unclosed `{{` kept, values not
// resolved again, the later enabled row wins – only to show what will be sent.

import type { VariableDraft } from './types';

const PLACEHOLDER = /\{\{([^}]*)\}\}/g;

/** Shown in place of a secret value (the UI never gets the real one). */
export const SECRET_MASK = '••••••';

export interface Preview {
  /** The text with every known placeholder replaced. */
  text: string;
  /** Names without a value, in order of appearance, each once. */
  missing: string[];
}

/** The enabled variables by name (later rows win); secrets masked. */
export function variableValues(variables: readonly VariableDraft[]): Map<string, string> {
  const values = new Map<string, string>();
  for (const variable of variables) {
    if (variable.enabled) {
      values.set(variable.name.trim(), variable.secret ? SECRET_MASK : variable.value);
    }
  }
  return values;
}

/**
 * The values of several variable lists, weakest first (collection,
 * environment, folders outer to inner, request): stronger lists win, as when
 * sending (ac_domain::scope).
 */
export function scopeValues(layers: readonly (readonly VariableDraft[])[]): Map<string, string> {
  const values = new Map<string, string>();
  for (const layer of layers) {
    for (const [name, value] of variableValues(layer)) values.set(name, value);
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

/** Where a variable's value comes from (the strongest place that has it). */
export type VariableSource = 'collection' | 'environment' | 'folder' | 'request';

/** A variable as the editor suggests and explains it. */
export interface VariableInfo {
  /** The value; secrets masked. */
  value: string;
  source: VariableSource;
}

/** The variables of several lists with where they come from, weakest first; stronger lists win. */
export function scopeInfo(
  layers: readonly { source: VariableSource; variables: readonly VariableDraft[] }[],
): Map<string, VariableInfo> {
  const info = new Map<string, VariableInfo>();
  for (const layer of layers) {
    for (const [name, value] of variableValues(layer.variables)) {
      if (name !== '') info.set(name, { value, source: layer.source });
    }
  }
  return info;
}

/** An unfinished `{{name` right before the caret: where the name starts and what is typed of it. */
export interface Suggestion {
  start: number;
  typed: string;
}

/** The `{{…` being typed at `caret`, or `null` if the caret is not inside an open placeholder. */
export function suggestionAt(text: string, caret: number): Suggestion | null {
  const before = text.slice(0, caret);
  const open = before.lastIndexOf('{{');
  if (open < 0) return null;
  const typed = before.slice(open + 2);
  if (/[{}]/.test(typed)) return null;
  return { start: open + 2, typed: typed.trimStart() };
}

/** Names of `variables` that fit what is typed: starting with it first, then containing it. */
export function suggestionsFor(
  variables: ReadonlyMap<string, VariableInfo>,
  typed: string,
): string[] {
  const wanted = typed.toLowerCase();
  const names = [...variables.keys()].sort((a, b) => a.localeCompare(b));
  const starting = names.filter((name) => name.toLowerCase().startsWith(wanted));
  const containing = names.filter(
    (name) => !starting.includes(name) && name.toLowerCase().includes(wanted),
  );
  return [...starting, ...containing];
}

/** `text` with the placeholder at `suggestion` completed to `{{name}}`; the caret goes after it. */
export function complete(
  text: string,
  caret: number,
  suggestion: Suggestion,
  name: string,
): { text: string; caret: number } {
  const after = text.slice(caret);
  const closing = after.startsWith('}}') ? 2 : 0;
  const completed = `${text.slice(0, suggestion.start)}${name}}}`;
  return { text: completed + after.slice(closing), caret: completed.length };
}

/** The placeholders in `text` with what they are: known variables with value and source, or unknown. */
export function placeholdersIn(
  text: string,
  variables: ReadonlyMap<string, VariableInfo>,
): { name: string; info: VariableInfo | null }[] {
  const found: { name: string; info: VariableInfo | null }[] = [];
  for (const match of text.matchAll(PLACEHOLDER)) {
    const name = match[1]!.trim();
    if (found.some((entry) => entry.name === name)) continue;
    found.push({ name, info: variables.get(name) ?? null });
  }
  return found;
}
