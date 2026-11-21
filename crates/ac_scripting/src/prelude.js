// Implements `api/script-api.d.ts` in plain JavaScript. The host passes the
// input as JSON in `__input` and reads the result with `__output()` – no
// host functions, so a script can reach nothing but this API.
(() => {
  const input = JSON.parse(globalThis.__input);
  delete globalThis.__input;
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
    }),
  );
})();
