// Postman (`pm`) and Bruno (`bru`, `req.getUrl()`, `res.getBody()` …)
// compatibility: the calls their scripts use most, built on the script API
// of `prelude.js`, so imported scripts run unchanged. What is not here (e.g.
// `pm.sendRequest`) fails with a clear error. Declared in `compat.d.ts`.
(() => {
  const unsupported = (name) => () => {
    throw new Error(`${name} is not supported`);
  };
  const text = (value) => (typeof value === 'string' ? value : JSON.stringify(value));
  /** `{{name}}` filled in from the variables (unknown ones stay). */
  const replaceIn = (template) =>
    String(template).replace(/\{\{\s*([^{}]+?)\s*\}\}/g, (whole, name) => env.get(name) ?? whole);

  /** One variable scope of Postman; all of them are the same variables here. */
  const scope = {
    get: (name) => env.get(name),
    has: (name) => env.has(name),
    set: (name, value) => env.set(name, value),
    unset: (name) => env.set(name, ''),
    clear: unsupported('clearing variables'),
    replaceIn,
    toObject: unsupported('listing variables'),
  };

  // --- Bruno: methods on req and res, and bru ------------------------------

  const parse = (body) => {
    if (body === null || body === undefined) return body;
    try {
      return JSON.parse(body);
    } catch {
      return body;
    }
  };
  Object.assign(req, {
    getUrl: () => req.url,
    setUrl: (url) => {
      req.url = String(url);
    },
    getMethod: () => req.method,
    setMethod: (method) => {
      req.method = String(method).toUpperCase();
    },
    getHeaders: () => req.headers(),
    deleteHeader: (name) => req.removeHeader(name),
    getBody: () => parse(req.body),
    setBody: (body) => {
      req.body = text(body);
    },
  });
  const response = globalThis.res;
  if (response) {
    Object.assign(response, {
      getStatus: () => response.status,
      getHeader: response.getHeader,
      getHeaders: () => response.headers(),
      getBody: () => parse(response.text()),
      getResponseTime: () => response.timeMs,
    });
    // `res.body` and `res.responseTime` as properties, as Bruno has them.
    Object.defineProperty(response, 'body', { get: () => parse(response.text()) });
    Object.defineProperty(response, 'responseTime', { get: () => response.timeMs });
  }
  const bru = {
    getEnvVar: (name) => env.get(name),
    setEnvVar: (name, value) => env.set(name, value),
    getVar: (name) => env.get(name),
    setVar: (name, value) => env.set(name, value),
    hasVar: (name) => env.has(name),
    deleteVar: (name) => env.set(name, ''),
    getCollectionVar: (name) => env.get(name),
    getFolderVar: (name) => env.get(name),
    getRequestVar: (name) => env.get(name),
    interpolate: replaceIn,
    sendRequest: unsupported('bru.sendRequest'),
    runRequest: unsupported('bru.runRequest'),
    setNextRequest: unsupported('bru.setNextRequest'),
    sleep: unsupported('bru.sleep'),
  };

  // --- Postman: pm ---------------------------------------------------------

  const headerList = (get, all, set, remove) => ({
    get,
    has: (name) => get(name) !== undefined,
    toObject: all,
    add: (header) => set(header.key, header.value),
    upsert: (header) => set(header.key, header.value),
    remove,
  });
  const pmRequest = {
    get method() {
      return req.method;
    },
    set method(method) {
      req.method = String(method).toUpperCase();
    },
    url: {
      toString: () => req.url,
      update: (url) => {
        req.url = String(url);
      },
    },
    headers: headerList(req.getHeader, req.headers, req.setHeader, req.removeHeader),
    body: {
      get raw() {
        return req.body;
      },
      set raw(body) {
        req.body = text(body);
      },
      toString: () => req.body ?? '',
    },
  };
  /** Standard reason phrases for `pm.response.status`. */
  const REASONS = {
    200: 'OK',
    201: 'Created',
    202: 'Accepted',
    204: 'No Content',
    301: 'Moved Permanently',
    302: 'Found',
    304: 'Not Modified',
    400: 'Bad Request',
    401: 'Unauthorized',
    403: 'Forbidden',
    404: 'Not Found',
    409: 'Conflict',
    422: 'Unprocessable Entity',
    429: 'Too Many Requests',
    500: 'Internal Server Error',
    502: 'Bad Gateway',
    503: 'Service Unavailable',
  };
  const pmResponse = response && {
    code: response.status,
    status: REASONS[response.status] ?? String(response.status),
    responseTime: response.timeMs,
    json: () => response.json(),
    text: () => response.text(),
    headers: headerList(
      response.getHeader,
      response.headers,
      unsupported('changing response headers'),
      unsupported('changing response headers'),
    ),
    get to() {
      return __chai(undefined, {
        status: response.status,
        reason: REASONS[response.status],
        header: response.getHeader,
      });
    },
  };
  const pm = {
    test,
    expect: (actual) => __chai(actual),
    environment: scope,
    variables: scope,
    collectionVariables: scope,
    globals: scope,
    iterationData: scope,
    request: pmRequest,
    response: pmResponse,
    info: { eventName: response ? 'test' : 'prerequest' },
    sendRequest: unsupported('pm.sendRequest'),
    setNextRequest: unsupported('pm.setNextRequest'),
    execution: { setNextRequest: unsupported('pm.execution.setNextRequest') },
  };

  for (const [name, value] of [
    ['pm', pm],
    // Insomnia's script API follows Postman's: imported scripts run as they are.
    ['insomnia', pm],
    ['bru', bru],
  ]) {
    Object.defineProperty(globalThis, name, { value, writable: false, configurable: false });
  }
})();
