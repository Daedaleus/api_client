// Compatibility with scripts imported from Postman and Bruno: chai-style
// assertions and the `pm` / `bru` objects, the commonly used part only
// (`compat.js`). New scripts are best written with the API above.

/** Chai-style assertion, e.g. `expect(x).to.equal(1)` or `pm.expect(x).to.be.above(2)`. */
interface ChaiAssertion {
  readonly to: ChaiAssertion;
  readonly be: ChaiAssertion;
  readonly been: ChaiAssertion;
  readonly is: ChaiAssertion;
  readonly that: ChaiAssertion;
  readonly which: ChaiAssertion;
  readonly and: ChaiAssertion;
  readonly has: ChaiAssertion;
  readonly have: ChaiAssertion;
  readonly with: ChaiAssertion;
  readonly at: ChaiAssertion;
  readonly of: ChaiAssertion;
  readonly same: ChaiAssertion;
  readonly but: ChaiAssertion;
  readonly does: ChaiAssertion;
  readonly still: ChaiAssertion;
  readonly also: ChaiAssertion;
  /** Turns the following check around. */
  readonly not: ChaiAssertion;
  /** Compare deeply in `equal` and `include`. */
  readonly deep: ChaiAssertion;
  readonly ok: ChaiAssertion;
  readonly success: ChaiAssertion;
  readonly true: ChaiAssertion;
  readonly false: ChaiAssertion;
  readonly null: ChaiAssertion;
  readonly undefined: ChaiAssertion;
  readonly NaN: ChaiAssertion;
  readonly exist: ChaiAssertion;
  readonly empty: ChaiAssertion;
  equal(expected: unknown): ChaiAssertion;
  equals(expected: unknown): ChaiAssertion;
  eq(expected: unknown): ChaiAssertion;
  eql(expected: unknown): ChaiAssertion;
  eqls(expected: unknown): ChaiAssertion;
  above(expected: number): ChaiAssertion;
  gt(expected: number): ChaiAssertion;
  greaterThan(expected: number): ChaiAssertion;
  least(expected: number): ChaiAssertion;
  gte(expected: number): ChaiAssertion;
  below(expected: number): ChaiAssertion;
  lt(expected: number): ChaiAssertion;
  lessThan(expected: number): ChaiAssertion;
  most(expected: number): ChaiAssertion;
  lte(expected: number): ChaiAssertion;
  within(low: number, high: number): ChaiAssertion;
  include(expected: unknown): ChaiAssertion;
  includes(expected: unknown): ChaiAssertion;
  contain(expected: unknown): ChaiAssertion;
  contains(expected: unknown): ChaiAssertion;
  /** A type: `"string"`, `"number"`, `"object"`, `"array"`, `"null"` … */
  a(type: string): ChaiAssertion;
  an(type: string): ChaiAssertion;
  match(pattern: RegExp): ChaiAssertion;
  matches(pattern: RegExp): ChaiAssertion;
  lengthOf(expected: number): ChaiAssertion;
  length(expected: number): ChaiAssertion;
  oneOf(list: unknown[]): ChaiAssertion;
  /** Has the property (optionally with this value); continues with the property's value. */
  property(name: string, value?: unknown): ChaiAssertion;
  keys(...names: (string | string[])[]): ChaiAssertion;
  /** Postman: `pm.response.to.have.status(200)` or `("OK")`. */
  status(code: number | string): ChaiAssertion;
  /** Postman: `pm.response.to.have.header("Content-Type")`. */
  header(name: string, value?: string): ChaiAssertion;
}

interface Matchers {
  /** Chai-style checks, e.g. `expect(x).to.equal(1)` (as in Bruno). */
  readonly to: ChaiAssertion;
}

/** A Postman variable scope; here all scopes are the script variables (`env`). */
interface PostmanVariables {
  get(name: string): string | undefined;
  has(name: string): boolean;
  set(name: string, value: string | number | boolean): void;
  /** Sets the variable to an empty text. */
  unset(name: string): void;
  /** `{{name}}` filled in. */
  replaceIn(template: string): string;
}

interface PostmanHeaders {
  get(name: string): string | undefined;
  has(name: string): boolean;
  toObject(): Record<string, string>;
  add(header: { key: string; value: string }): void;
  upsert(header: { key: string; value: string }): void;
  remove(name: string): void;
}

/** Postman's `pm`, built on this API (see the top of this file). */
declare const pm: {
  test(name: string, check: () => unknown): void;
  expect(actual: unknown): ChaiAssertion;
  environment: PostmanVariables;
  variables: PostmanVariables;
  collectionVariables: PostmanVariables;
  globals: PostmanVariables;
  iterationData: PostmanVariables;
  request: {
    method: string;
    url: { toString(): string; update(url: string): void };
    headers: PostmanHeaders;
    body: { raw: string | null; toString(): string };
  };
  /** Only in the script after the response. */
  response: {
    code: number;
    status: string;
    responseTime: number;
    json<T = any>(): T;
    text(): string;
    headers: PostmanHeaders;
    readonly to: ChaiAssertion;
  };
  info: { eventName: 'prerequest' | 'test' };
  /** Not supported: throws. */
  sendRequest(...args: unknown[]): never;
  /** Not supported: throws. */
  setNextRequest(...args: unknown[]): never;
};

/** Bruno's `bru`, built on this API; all variables are the script variables (`env`). */
declare const bru: {
  getEnvVar(name: string): string | undefined;
  setEnvVar(name: string, value: string | number | boolean): void;
  getVar(name: string): string | undefined;
  setVar(name: string, value: string | number | boolean): void;
  hasVar(name: string): boolean;
  deleteVar(name: string): void;
  getCollectionVar(name: string): string | undefined;
  getFolderVar(name: string): string | undefined;
  getRequestVar(name: string): string | undefined;
  interpolate(template: string): string;
  /** Not supported: throws. */
  sendRequest(...args: unknown[]): never;
  /** Not supported: throws. */
  runRequest(...args: unknown[]): never;
  /** Not supported: throws. */
  setNextRequest(...args: unknown[]): never;
  /** Not supported: throws. */
  sleep(ms: number): never;
};

/** Bruno's names for the request. */
interface ScriptRequest {
  getUrl(): string;
  setUrl(url: string): void;
  getMethod(): string;
  setMethod(method: string): void;
  getHeaders(): Record<string, string>;
  deleteHeader(name: string): void;
  /** The body, parsed if it is JSON. */
  getBody(): any;
  /** Sets the body; objects as JSON. */
  setBody(body: unknown): void;
}

/** Bruno's names for the response. */
interface ScriptResponse {
  getStatus(): number;
  getHeaders(): Record<string, string>;
  /** The body, parsed if it is JSON. */
  getBody(): any;
  getResponseTime(): number;
  /** The body, parsed if it is JSON. */
  readonly body: any;
  readonly responseTime: number;
}
