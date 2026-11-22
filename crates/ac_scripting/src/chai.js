// Chai-style assertions (`expect(x).to.equal(1)`), the common subset that
// Postman and Bruno scripts use. Evaluated before `prelude.js`, which takes
// `__chai` for `expect(…).to` and `pm.expect`, and removes the global.
(() => {
  class AssertionError extends Error {
    name = 'AssertionError';
  }

  const show = (value) => {
    if (value === undefined) return 'undefined';
    if (typeof value === 'function') return 'a function';
    if (typeof value === 'number' && Number.isNaN(value)) return 'NaN';
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
  const typeName = (value) => {
    if (value === null) return 'null';
    if (Array.isArray(value)) return 'array';
    return typeof value;
  };
  const includes = (container, item, deep) => {
    if (typeof container === 'string') return container.includes(String(item));
    if (Array.isArray(container)) {
      return container.some((element) => (deep ? deepEqual(element, item) : Object.is(element, item)));
    }
    if (container !== null && typeof container === 'object' && item !== null && typeof item === 'object') {
      return Object.keys(item).every((key) =>
        deep ? deepEqual(container[key], item[key]) : Object.is(container[key], item[key]),
      );
    }
    return false;
  };

  /** Words that only make the sentence read well. */
  const CHAIN = ['to', 'be', 'been', 'is', 'that', 'which', 'and', 'has', 'have', 'with', 'at', 'of', 'same', 'but', 'does', 'still', 'also'];

  /**
   * An assertion about `actual`. `http` describes an HTTP response for
   * Postman's `pm.response.to.have.status(200)` and friends.
   */
  const assertion = (actual, http) => {
    const flags = { negate: false, deep: false };
    const self = {};
    const check = (ok, description) => {
      if (Boolean(ok) === flags.negate) {
        const subject = http ? `response ${show(http.status)}` : show(actual);
        throw new AssertionError(`expected ${subject} ${flags.negate ? 'not ' : ''}${description}`);
      }
      return self;
    };
    const getter = (name, run) => Object.defineProperty(self, name, { get: run, enumerable: false });
    for (const word of CHAIN) getter(word, () => self);
    getter('not', () => {
      flags.negate = !flags.negate;
      return self;
    });
    getter('deep', () => {
      flags.deep = true;
      return self;
    });
    getter('ok', () =>
      http ? check(http.status >= 200 && http.status < 300, 'to be ok (2xx)') : check(actual, 'to be truthy'),
    );
    getter('success', () => check(http && http.status >= 200 && http.status < 300, 'to be a success (2xx)'));
    getter('true', () => check(actual === true, 'to be true'));
    getter('false', () => check(actual === false, 'to be false'));
    getter('null', () => check(actual === null, 'to be null'));
    getter('undefined', () => check(actual === undefined, 'to be undefined'));
    getter('NaN', () => check(Number.isNaN(actual), 'to be NaN'));
    getter('exist', () => check(actual !== null && actual !== undefined, 'to exist'));
    getter('empty', () => {
      const size =
        actual !== null && typeof actual === 'object' && !Array.isArray(actual)
          ? Object.keys(actual).length
          : actual?.length;
      return check(size === 0, 'to be empty');
    });

    const equal = (expected) =>
      check(
        flags.deep ? deepEqual(actual, expected) : Object.is(actual, expected),
        `to ${flags.deep ? 'deeply ' : ''}equal ${show(expected)}`,
      );
    const eql = (expected) => check(deepEqual(actual, expected), `to deeply equal ${show(expected)}`);
    const above = (n) => check(actual > n, `to be above ${show(n)}`);
    const least = (n) => check(actual >= n, `to be at least ${show(n)}`);
    const below = (n) => check(actual < n, `to be below ${show(n)}`);
    const most = (n) => check(actual <= n, `to be at most ${show(n)}`);
    const include = (item) => check(includes(actual, item, flags.deep), `to include ${show(item)}`);
    const a = (type) => check(typeName(actual) === String(type).toLowerCase(), `to be a ${type}`);
    const methods = {
      equal,
      equals: equal,
      eq: equal,
      eql,
      eqls: eql,
      above,
      gt: above,
      greaterThan: above,
      least,
      gte: least,
      below,
      lt: below,
      lessThan: below,
      most,
      lte: most,
      within: (low, high) => check(actual >= low && actual <= high, `to be within ${low}..${high}`),
      include,
      includes: include,
      contain: include,
      contains: include,
      a,
      an: a,
      match: (pattern) =>
        check(typeof actual === 'string' && pattern.test(actual), `to match ${String(pattern)}`),
      matches: (pattern) =>
        check(typeof actual === 'string' && pattern.test(actual), `to match ${String(pattern)}`),
      lengthOf: (n) =>
        check(actual !== null && actual !== undefined && actual.length === n, `to have length ${n}`),
      length: (n) =>
        check(actual !== null && actual !== undefined && actual.length === n, `to have length ${n}`),
      oneOf: (list) => check(list.some((item) => deepEqual(item, actual)), `to be one of ${show(list)}`),
      property(name, ...value) {
        const has = actual !== null && actual !== undefined && Object.hasOwn(Object(actual), name);
        const wanted = value.length > 0 ? ` = ${show(value[0])}` : '';
        check(
          has && (value.length === 0 || deepEqual(actual[name], value[0])),
          `to have property ${show(name)}${wanted}`,
        );
        return has && !flags.negate ? assertion(actual[name]) : self;
      },
      keys(...names) {
        const wanted = names.flat();
        const own = actual !== null && typeof actual === 'object' ? Object.keys(actual) : [];
        return check(wanted.every((key) => own.includes(key)), `to have keys ${show(wanted)}`);
      },
      // HTTP responses (Postman).
      status(code) {
        if (!http) throw new AssertionError('status() needs pm.response');
        return typeof code === 'number'
          ? check(http.status === code, `to have status ${code}`)
          : check(http.reason === String(code), `to have status ${show(code)}`);
      },
      header(name, ...value) {
        if (!http) throw new AssertionError('header() needs pm.response');
        const found = http.header(String(name));
        const wanted = value.length > 0 ? ` = ${show(value[0])}` : '';
        return check(
          found !== undefined && (value.length === 0 || found === value[0]),
          `to have header ${show(name)}${wanted}`,
        );
      },
    };
    for (const [name, run] of Object.entries(methods)) {
      Object.defineProperty(self, name, { value: run, enumerable: false });
    }
    return self;
  };

  globalThis.__chai = assertion;
})();
