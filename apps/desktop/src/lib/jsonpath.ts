// A JSONPath subset to narrow down JSON responses, without `eval`:
//
//   $                 the whole document
//   .name  ['name']   a member ("name" with quotes too)
//   [0]  [-1]         an element (negative counts from the end)
//   [*]  .*           all members or elements
//   ..name  ..*       a member at any depth / everything below
//   [1:3]  [::2]      a slice (start, end, step)
//   [0,2]  ['a','b']  several
//
// Filter expressions (`[?(@.x > 1)]`) are not supported.

/** Why a path cannot be used: a translation key and arguments. */
export class JsonPathError extends Error {
  constructor(
    readonly key: string,
    readonly args: Record<string, string> = {},
  ) {
    super(key);
  }
}

type Selector =
  | { kind: 'member'; name: string }
  | { kind: 'index'; index: number }
  | { kind: 'all' }
  | { kind: 'slice'; start: number | null; end: number | null; step: number | null }
  | { kind: 'union'; selectors: Selector[] };

interface Step {
  /** `..`: also every member/element below. */
  deep: boolean;
  selector: Selector;
}

/** The values `path` selects in `document`, in document order. */
export function queryJson(document: unknown, path: string): unknown[] {
  let current: unknown[] = [document];
  for (const step of parse(path.trim())) {
    const sources = step.deep ? current.flatMap(descendants) : current;
    current = sources.flatMap((value) => select(value, step.selector));
  }
  return current;
}

function parse(path: string): Step[] {
  if (!path.startsWith('$')) throw new JsonPathError('jsonpath.start');
  const steps: Step[] = [];
  let at = 1;
  while (at < path.length) {
    let deep = false;
    if (path.startsWith('..', at)) {
      deep = true;
      at += 2;
    } else if (path[at] === '.') {
      at += 1;
    } else if (path[at] !== '[') {
      throw new JsonPathError('jsonpath.unexpected', { at: String(at + 1) });
    }
    if (path[at] === '[') {
      const close = closingBracket(path, at);
      steps.push({ deep, selector: bracket(path.slice(at + 1, close), at) });
      at = close + 1;
    } else if (path[at] === '*') {
      steps.push({ deep, selector: { kind: 'all' } });
      at += 1;
    } else {
      const name = /^[^.[\]\s]+/.exec(path.slice(at))?.[0];
      if (!name) throw new JsonPathError('jsonpath.unexpected', { at: String(at + 1) });
      steps.push({ deep, selector: { kind: 'member', name } });
      at += name.length;
    }
  }
  return steps;
}

/** Index of the `]` that closes the `[` at `open` (quotes respected). */
function closingBracket(path: string, open: number): number {
  let quote: string | null = null;
  for (let at = open + 1; at < path.length; at += 1) {
    const char = path[at];
    if (quote) {
      if (char === '\\') at += 1;
      else if (char === quote) quote = null;
    } else if (char === "'" || char === '"') {
      quote = char;
    } else if (char === ']') {
      return at;
    }
  }
  throw new JsonPathError('jsonpath.unclosed', { at: String(open + 1) });
}

function bracket(content: string, open: number): Selector {
  const parts = splitTopLevel(content).map((part) => part.trim());
  if (parts.length > 1) {
    return { kind: 'union', selectors: parts.map((part) => single(part, open)) };
  }
  return single(parts[0] ?? '', open);
}

function single(part: string, open: number): Selector {
  if (part === '*') return { kind: 'all' };
  const quoted = /^(['"])(.*)\1$/s.exec(part);
  if (quoted) return { kind: 'member', name: (quoted[2] ?? '').replace(/\\(.)/g, '$1') };
  if (/^-?\d+$/.test(part)) return { kind: 'index', index: Number(part) };
  const slice = /^(-?\d*):(-?\d*)(?::(-?\d*))?$/.exec(part);
  if (slice) {
    const number = (text: string | undefined) => (text ? Number(text) : null);
    return {
      kind: 'slice',
      start: number(slice[1]),
      end: number(slice[2]),
      step: number(slice[3]),
    };
  }
  throw new JsonPathError('jsonpath.selector', { selector: part, at: String(open + 1) });
}

/** Splits at commas outside quotes. */
function splitTopLevel(content: string): string[] {
  const parts: string[] = [];
  let quote: string | null = null;
  let current = '';
  for (let at = 0; at < content.length; at += 1) {
    const char = content[at]!;
    if (quote) {
      if (char === '\\') {
        current += char + (content[at + 1] ?? '');
        at += 1;
        continue;
      }
      if (char === quote) quote = null;
    } else if (char === "'" || char === '"') {
      quote = char;
    } else if (char === ',') {
      parts.push(current);
      current = '';
      continue;
    }
    current += char;
  }
  parts.push(current);
  return parts;
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function children(value: unknown): unknown[] {
  if (Array.isArray(value)) return value;
  if (isObject(value)) return Object.values(value);
  return [];
}

/** `value` and everything below it, parents first. */
function descendants(value: unknown): unknown[] {
  return [value, ...children(value).flatMap(descendants)];
}

function select(value: unknown, selector: Selector): unknown[] {
  switch (selector.kind) {
    case 'all':
      return children(value);
    case 'member':
      return isObject(value) && Object.hasOwn(value, selector.name) ? [value[selector.name]] : [];
    case 'index': {
      if (!Array.isArray(value)) return [];
      const index = selector.index < 0 ? value.length + selector.index : selector.index;
      return index >= 0 && index < value.length ? [value[index]] : [];
    }
    case 'slice':
      return Array.isArray(value) ? slice(value, selector) : [];
    case 'union':
      return selector.selectors.flatMap((part) => select(value, part));
  }
}

function slice(
  values: unknown[],
  { start, end, step }: { start: number | null; end: number | null; step: number | null },
): unknown[] {
  const stride = step ?? 1;
  if (stride === 0) return [];
  const length = values.length;
  const clamp = (index: number) =>
    index < 0
      ? Math.max(length + index, stride > 0 ? 0 : -1)
      : Math.min(index, stride > 0 ? length : length - 1);
  const from = start === null ? (stride > 0 ? 0 : length - 1) : clamp(start);
  const to = end === null ? (stride > 0 ? length : -1) : clamp(end);
  const picked: unknown[] = [];
  for (let index = from; stride > 0 ? index < to : index > to; index += stride) {
    picked.push(values[index]);
  }
  return picked;
}
