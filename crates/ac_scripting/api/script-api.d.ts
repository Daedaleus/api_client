// The API of request scripts. The single description of what scripts can
// use: the checker and completion read it, the documentation is built from
// it, and `prelude.js` implements it. Nothing else is available – no
// `fetch`, timers, files or Node.js modules.

/** The request. Before sending it can be changed; after the response it is the request as sent. */
interface ScriptRequest {
  /** Method, e.g. `"POST"`. */
  method: string;
  /** URL – before sending as entered, with `{{variables}}`. */
  url: string;
  /** The body if it is JSON or text, else `null` (other bodies cannot be read or replaced). */
  body: string | null;
  /** A header's value (name compared case-insensitively), or `undefined`. */
  getHeader(name: string): string | undefined;
  /** Sets a header, replacing one of the same name. */
  setHeader(name: string, value: string): void;
  /** Removes all headers of that name. */
  removeHeader(name: string): void;
  /** All headers, by name. */
  headers(): Record<string, string>;
}

/** The response (only in the script after the response). */
interface ScriptResponse {
  /** Status code, e.g. `200`. */
  status: number;
  /** Time until the whole response was read, in milliseconds. */
  timeMs: number;
  /** A header's value (name compared case-insensitively), or `undefined`. */
  getHeader(name: string): string | undefined;
  /** All headers, by name. */
  headers(): Record<string, string>;
  /** The body as text. */
  text(): string;
  /** The body parsed as JSON; throws if it is not JSON. */
  json<T = any>(): T;
}

/** Variables: every variable that applies to the request, and the ones scripts set. */
interface ScriptVariables {
  /** A variable's value, or `undefined`. */
  get(name: string): string | undefined;
  /** Whether a variable exists. */
  has(name: string): boolean;
  /**
   * Sets a variable for this and later requests of the collection. It is the
   * strongest of all, kept in memory until the collection is closed, and never
   * written to disk.
   */
  set(name: string, value: string | number | boolean): void;
}

declare const req: ScriptRequest;
/** Only defined in the script after the response. */
declare const res: ScriptResponse;
declare const env: ScriptVariables;

/** Writes a line to the script log; values other than strings as JSON. */
declare function log(...values: unknown[]): void;

/** `console.log` and friends write to the script log too. */
declare const console: {
  log(...values: unknown[]): void;
  info(...values: unknown[]): void;
  warn(...values: unknown[]): void;
  error(...values: unknown[]): void;
  debug(...values: unknown[]): void;
};

/**
 * A named test: runs `check` and records whether it passed. A failing
 * `expect` inside fails only this test; the script goes on.
 */
declare function test(name: string, check: () => unknown): void;

/** Checks for `expect(actual)`; `.not` turns each one around. */
interface Matchers {
  /** Same value (`Object.is`). */
  toBe(expected: unknown): void;
  /** Same content, compared deeply (objects and arrays). */
  toEqual(expected: unknown): void;
  toBeTruthy(): void;
  toBeFalsy(): void;
  toBeDefined(): void;
  toBeUndefined(): void;
  toBeNull(): void;
  toBeGreaterThan(expected: number): void;
  toBeGreaterThanOrEqual(expected: number): void;
  toBeLessThan(expected: number): void;
  toBeLessThanOrEqual(expected: number): void;
  /** A string or array containing `expected`. */
  toContain(expected: unknown): void;
  /** A string matching a regular expression or containing a text. */
  toMatch(expected: RegExp | string): void;
  toHaveLength(expected: number): void;
  /** An object with this property (path like `"user.id"`), optionally with this value. */
  toHaveProperty(path: string | string[], value?: unknown): void;
}

/** Starts a check of `actual`, e.g. `expect(res.status).toBe(200)`. */
declare function expect(actual: unknown): Matchers & { not: Matchers };
