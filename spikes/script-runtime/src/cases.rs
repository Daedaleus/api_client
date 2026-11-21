//! What scripts in an API client need from the language. Every case is a
//! script whose completion value is `true` – or, for `Async`, a promise that
//! resolves to `true` after the job queue has run.

/// How a case is evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The script's value must be `true`.
    Sync,
    /// The script's value is a promise that must resolve to `true`.
    Async,
}

/// One coverage case.
pub struct Case {
    pub name: &'static str,
    pub kind: Kind,
    pub source: &'static str,
}

const fn sync(name: &'static str, source: &'static str) -> Case {
    Case {
        name,
        kind: Kind::Sync,
        source,
    }
}

const fn asynchronous(name: &'static str, source: &'static str) -> Case {
    Case {
        name,
        kind: Kind::Async,
        source,
    }
}

pub const CASES: &[Case] = &[
    sync(
        "ES2015 classes, destructuring, spread",
        r"
        class A { constructor(x) { this.x = x } get double() { return this.x * 2 } }
        const { x, ...rest } = { x: 1, y: 2, z: 3 };
        const [first, , third = 9] = [1, 2];
        new A(x).double === 2 && Object.keys(rest).length === 2 && third === 9 && [...'ab'].length === 2
    ",
    ),
    sync(
        "template literals, tagged",
        r"
        const tag = (s, ...v) => s.raw.join('|') + v.join(',');
        `a${1 + 1}b` === 'a2b' && tag`x${1}y` === 'x|y1'
    ",
    ),
    sync(
        "optional chaining, nullish coalescing",
        r"
        const o = { a: { b: 0 } };
        o?.a?.b === 0 && (o.missing?.deep ?? 'd') === 'd' && (o.a.b ?? 5) === 0
    ",
    ),
    asynchronous(
        "async/await, Promise.all, allSettled, any",
        r"
        (async () => {
            const all = await Promise.all([1, Promise.resolve(2)]);
            const settled = await Promise.allSettled([Promise.reject(new Error('x'))]);
            const any = await Promise.any([Promise.reject(1), Promise.resolve(3)]);
            return all[1] === 2 && settled[0].status === 'rejected' && any === 3;
        })()
    ",
    ),
    sync(
        "generators, iterators, Symbol",
        r"
        function* g() { yield 1; yield 2 }
        const it = { *[Symbol.iterator]() { yield* g() } };
        [...it].join() === '1,2' && typeof Symbol('s') === 'symbol'
    ",
    ),
    asynchronous(
        "async iteration (for await)",
        r"
        (async () => {
            async function* g() { yield 1; yield 2 }
            let sum = 0;
            for await (const v of g()) sum += v;
            return sum === 3;
        })()
    ",
    ),
    sync(
        "Map, Set, WeakMap, WeakRef",
        r"
        const m = new Map([[1, 'a']]); const s = new Set([1, 1, 2]);
        const w = new WeakMap(); const k = {}; w.set(k, 1);
        m.get(1) === 'a' && s.size === 2 && w.get(k) === 1 && typeof WeakRef === 'function'
    ",
    ),
    sync(
        "RegExp: named groups, lookbehind, \\p{…}, sticky, matchAll, d flag",
        r"
        const m = /(?<year>\d{4})-(?<month>\d\d)/.exec('2026-11');
        const behind = /(?<=\$)\d+/.exec('cost $42')[0];
        const letters = 'Grüße'.match(/\p{L}/gu).length;
        const all = [...'a1b2'.matchAll(/\d/g)].map(x => x[0]).join('');
        const indices = /b/d.exec('ab').indices[0][0];
        m.groups.year === '2026' && behind === '42' && letters === 5 && all === '12' && indices === 1 && /a/y.sticky
    ",
    ),
    sync(
        "JSON round trip with Unicode and reviver",
        r#"
        const text = JSON.stringify({ name: 'Grüße 😀', n: [1, 2.5, null] });
        const back = JSON.parse(text, (k, v) => (k === 'n' ? v.length : v));
        back.name === 'Grüße 😀' && back.n === 3 && JSON.stringify(' ') === '" "'
    "#,
    ),
    sync(
        "Date parsing and ISO strings",
        r"
        const d = new Date('2026-11-21T10:00:00Z');
        d.toISOString() === '2026-11-21T10:00:00.000Z' && d.getUTCDay() === 6 && !isNaN(Date.parse('2026-11-21'))
    ",
    ),
    sync(
        "BigInt",
        r"
        (2n ** 64n).toString() === '18446744073709551616' && BigInt('12') + 1n === 13n
    ",
    ),
    sync(
        "ES2019–2023 array and object helpers",
        r"
        [1, [2, [3]]].flat(Infinity).length === 3 && [1, 2].flatMap(x => [x, x]).length === 4 &&
        Object.fromEntries([['a', 1]]).a === 1 && [1, 2, 3].at(-1) === 3 &&
        [1, 2, 3].findLast(x => x < 3) === 2 && [3, 1].toSorted().join() === '1,3' &&
        'a-b-c'.replaceAll('-', '') === 'abc' && Object.hasOwn({ a: 1 }, 'a')
    ",
    ),
    sync(
        "ES2024: Object.groupBy, Promise.withResolvers",
        r"
        const groups = Object.groupBy([1, 2, 3], x => (x % 2 ? 'odd' : 'even'));
        groups.odd.length === 2 && typeof Promise.withResolvers === 'function'
    ",
    ),
    sync(
        "ES2025: Set methods, iterator helpers",
        r"
        new Set([1, 2]).union(new Set([3])).size === 3 &&
        [1, 2, 3].values().map(x => x * 2).toArray().join() === '2,4,6'
    ",
    ),
    sync(
        "class fields, private methods, static blocks",
        r"
        class C { #secret = 1; static count; static { C.count = 5 } #get() { return this.#secret } read() { return this.#get() } }
        new C().read() === 1 && C.count === 5
    ",
    ),
    sync(
        "Error cause and stack",
        r"
        let ok = false;
        try { throw new Error('outer', { cause: new TypeError('inner') }) }
        catch (e) { ok = e.cause instanceof TypeError && typeof e.stack === 'string' }
        ok
    ",
    ),
    sync(
        "Proxy and Reflect",
        r"
        const p = new Proxy({}, { get: (t, k) => (k === 'x' ? 42 : Reflect.get(t, k)) });
        p.x === 42 && Reflect.ownKeys({ a: 1 }).length === 1
    ",
    ),
    sync(
        "typed arrays, ArrayBuffer, DataView",
        r"
        const b = new ArrayBuffer(4); new DataView(b).setUint16(0, 258);
        new Uint8Array(b)[1] === 2 && Float64Array.from([1.5])[0] === 1.5
    ",
    ),
    sync(
        "String normalize and Unicode",
        r"
        'é'.normalize('NFC') === 'é' && 'abc'.padStart(5, '-') === '--abc' && [...'😀'].length === 1
    ",
    ),
    sync(
        "Intl (number and date formatting)",
        r"
        typeof Intl === 'object' && new Intl.NumberFormat('de-DE').format(1234.5) === '1.234,5'
    ",
    ),
    sync(
        "no file or network access by default",
        r"
        typeof fetch === 'undefined' && typeof require === 'undefined' && typeof process === 'undefined' &&
        typeof Deno === 'undefined' && typeof XMLHttpRequest === 'undefined' && typeof setTimeout === 'undefined'
    ",
    ),
];

/// Typical work of a test script: build and parse a large JSON body, look
/// things up in it, run regular expressions and sort.
pub const BENCHMARK: &str = r"
    const items = [];
    for (let i = 0; i < 20000; i++) items.push({ id: i, name: 'Product ' + i, price: i / 100, tags: ['a', 'b'] });
    const body = JSON.stringify(items);          // ~1.3 MB, like a large response
    const parsed = JSON.parse(body);
    let total = 0;
    for (const item of parsed) if (/Product \d+5$/.test(item.name)) total += item.price;
    const sorted = parsed.map(i => i.price).sort((a, b) => b - a);
    total > 0 && sorted[0] === 199.99 && body.length > 1000000
";

/// An endless loop, to check that a script can be stopped.
pub const ENDLESS: &str = "while (true) {}";

/// Allocates without end, to check a memory limit.
pub const MEMORY_HOG: &str = "const a = []; while (true) a.push(new Array(100000).fill(1));";

/// Uses the host API the app would provide (`req`, `env`, `log`).
pub const HOST_API: &str = r"
    req.setHeader('X-Trace', 'abc');
    env.set('token', 'from-script');
    log('method ' + req.method);
    req.method === 'POST' && env.get('baseUrl') === 'https://api.example.com' && req.headers()['X-Trace'] === 'abc'
";

/// A regular expression with catastrophic backtracking: no loop in the
/// script, so only a wall-clock interrupt inside the regex engine stops it.
pub const BACKTRACKING: &str = "/^(a+)+$/.test('a'.repeat(40) + 'b')";
