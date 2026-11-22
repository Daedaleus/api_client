// Implements `api/script-api.d.ts` in plain JavaScript. The host passes the
// input as JSON in `__input` and reads the result with `__output()` – no
// host functions, so a script can reach nothing but this API.
(() => {
  const input = JSON.parse(globalThis.__input);
  delete globalThis.__input;
  // Chai-style assertions from `chai.js`, for `expect(x).to…` (Bruno) and
  // `pm.expect` (Postman, see `compat.js`).
  const chai = globalThis.__chai;
  delete globalThis.__chai;
  const logs = [];
  const set = [];

  const format = (value) => {
    if (typeof value === 'string') return value;
    try {
      return JSON.stringify(value) ?? String(value);
    } catch {
      return String(value);
    }
  };
  const log = (...values) => {
    logs.push(values.map(format).join(' '));
  };

  /** Header list helpers; names compare case-insensitively. */
  const same = (a, b) => a.toLowerCase() === b.toLowerCase();
  const headerApi = (list) => ({
    getHeader: (name) => list.find(([n]) => same(n, String(name)))?.[1],
    headers: () => Object.fromEntries(list),
  });

  const requestHeaders = input.request.headers.map(([name, value]) => [name, value]);
  const removeHeader = (name) => {
    for (let i = requestHeaders.length - 1; i >= 0; i--) {
      if (same(requestHeaders[i][0], String(name))) requestHeaders.splice(i, 1);
    }
  };
  const req = {
    method: input.request.method,
    url: input.request.url,
    body: input.request.body,
    ...headerApi(requestHeaders),
    /** Replaces the first header of that name in place, else appends. */
    setHeader(name, value) {
      const entry = [String(name), String(value)];
      const at = requestHeaders.findIndex(([n]) => same(n, entry[0]));
      removeHeader(entry[0]);
      requestHeaders.splice(at < 0 ? requestHeaders.length : at, 0, entry);
    },
    removeHeader,
  };

  const variables = new Map(Object.entries(input.variables));
  const env = {
    get: (name) => variables.get(String(name)),
    has: (name) => variables.has(String(name)),
    set(name, value) {
      const entry = [String(name), String(value)];
      variables.set(entry[0], entry[1]);
      set.push(entry);
    },
  };

  const define = (name, value) =>
    Object.defineProperty(globalThis, name, { value, writable: false, configurable: false });
  define('req', req);
  define('env', env);
  define('log', log);
  define('console', { log, info: log, warn: log, error: log, debug: log });

  if (input.response) {
    const response = input.response;
    define('res', {
      status: response.status,
      timeMs: response.timeMs,
      ...headerApi(response.headers),
      text: () => response.body,
      json: () => JSON.parse(response.body),
    });
  }

  // --- Tests -------------------------------------------------------------

  /** A value as it appears in failure messages. */
  const show = (value) => {
    if (value === undefined) return 'undefined';
    if (typeof value === 'function') return 'a function';
    try {
      return JSON.stringify(value) ?? String(value);
    } catch {
      return String(value);
    }
  };
  const deepEqual = (a, b) => {
    if (Object.is(a, b)) return true;
    if (typeof a !== 'object' || typeof b !== 'object' || a === null || b === null) return false;
    if (Array.isArray(a) !== Array.isArray(b)) return false;
    const keys = Object.keys(a);
    return (
      keys.length === Object.keys(b).length &&
      keys.every((key) => Object.hasOwn(b, key) && deepEqual(a[key], b[key]))
    );
  };

  class AssertionError extends Error {
    name = 'AssertionError';
  }

  const matchers = (actual, negated) => {
    const check = (ok, description) => {
      if (Boolean(ok) === negated) {
        throw new AssertionError(`expected ${show(actual)} ${negated ? 'not ' : ''}${description}`);
      }
    };
    return {
      toBe: (expected) => check(Object.is(actual, expected), `to be ${show(expected)}`),
      toEqual: (expected) => check(deepEqual(actual, expected), `to equal ${show(expected)}`),
      toBeTruthy: () => check(actual, 'to be truthy'),
      toBeFalsy: () => check(!actual, 'to be falsy'),
      toBeDefined: () => check(actual !== undefined, 'to be defined'),
      toBeUndefined: () => check(actual === undefined, 'to be undefined'),
      toBeNull: () => check(actual === null, 'to be null'),
      toBeGreaterThan: (n) => check(actual > n, `to be greater than ${show(n)}`),
      toBeGreaterThanOrEqual: (n) => check(actual >= n, `to be at least ${show(n)}`),
      toBeLessThan: (n) => check(actual < n, `to be less than ${show(n)}`),
      toBeLessThanOrEqual: (n) => check(actual <= n, `to be at most ${show(n)}`),
      toContain: (item) =>
        check(
          actual !== null &&
            actual !== undefined &&
            typeof actual.includes === 'function' &&
            actual.includes(item),
          `to contain ${show(item)}`,
        ),
      toMatch: (pattern) =>
        check(
          typeof actual === 'string' &&
            (pattern instanceof RegExp ? pattern.test(actual) : actual.includes(String(pattern))),
          `to match ${pattern instanceof RegExp ? String(pattern) : show(pattern)}`,
        ),
      toHaveLength: (n) =>
        check(actual !== null && actual !== undefined && actual.length === n, `to have length ${n}`),
      toHaveProperty: (path, ...value) => {
        const keys = Array.isArray(path) ? path.map(String) : String(path).split('.');
        let current = actual;
        let found = true;
        for (const key of keys) {
          if (current !== null && current !== undefined && Object.hasOwn(Object(current), key)) {
            current = current[key];
          } else {
            found = false;
            break;
          }
        }
        const wanted = value.length > 0 ? ` = ${show(value[0])}` : '';
        check(
          found && (value.length === 0 || deepEqual(current, value[0])),
          `to have property ${keys.join('.')}${wanted}`,
        );
      },
    };
  };
  /** Jest-style matchers, and `.to` for chai-style checks. */
  const expect = (actual) => {
    const negated = matchers(actual, true);
    Object.defineProperty(negated, 'to', { get: () => chai(actual).not });
    return {
      ...matchers(actual, false),
      not: negated,
      get to() {
        return chai(actual);
      },
    };
  };

  const tests = [];
  const pending = [];
  /** The message of what a failed test threw. */
  const messageOf = (error) => {
    if (!(error instanceof Error)) return String(error);
    const generic = error.name === 'Error' || error.name === 'AssertionError';
    return generic ? error.message : `${error.name}: ${error.message}`;
  };
  const test = (name, check) => {
    // Recorded in the order of the calls, also for async tests.
    const entry = { name: String(name), passed: true, message: null };
    tests.push(entry);
    const fail = (error) => {
      entry.passed = false;
      entry.message = messageOf(error);
    };
    try {
      const result = check();
      if (result && typeof result.then === 'function') {
        pending.push(Promise.resolve(result).then(() => {}, fail));
      }
    } catch (error) {
      fail(error);
    }
  };
  define('test', test);
  define('expect', expect);
  /** For `compat.js`: chai-style assertions, also about an HTTP response. */
  define('__chai', chai);
  /** Waits for async tests; the host calls it after the script. */
  define('__settle', () => Promise.all(pending));

  define('__output', () =>
    JSON.stringify({
      request: {
        method: String(req.method),
        url: String(req.url),
        headers: requestHeaders,
        body: req.body === null || req.body === undefined ? null : String(req.body),
      },
      variables: set,
      logs,
      tests,
    }),
  );
})();
