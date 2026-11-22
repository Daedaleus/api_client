# Scripts

Every request can have two scripts, in TypeScript or plain JavaScript:

- **Before the request** – runs before `{{variables}}` are filled in. It can
  change method, URL, headers and a JSON or text body, and set variables.
- **After the response** – runs once the response has arrived. It can read
  the response and set variables, e.g. a token for the following requests.

Scripts are edited in the request's *Scripts* section and saved in the
request file ([format](format/v1.md#scripts)). What they log appears in the
*Log* tab of the response.

The collection and every folder can have the same two scripts (*Collection
variables & auth* or the folder's settings, section *Scripts*), e.g. a
login or a common header for all requests in it. Before sending they run
from the outside in – collection, outer folders, inner folders, then the
request's own script – and after the response in reverse order. Each sees
what the ones before changed and set; one that fails stops the rest, and
its error names the folder or collection. The command line runner runs
them too.

```ts
// Before the request
req.setHeader('X-Request-Id', String(Date.now()));
if (!env.has('token')) log('no token yet – log in first');

// After the response
const body = res.json<{ token: string }>();
env.set('token', body.token);
log('logged in, status', res.status);
```

## API

The complete API with all types is
[`crates/ac_scripting/api/script-api.d.ts`](../crates/ac_scripting/api/script-api.d.ts).

| Name | What |
|---|---|
| `req` | The request: `method`, `url`, `body` (JSON or text, else `null`), `getHeader`, `setHeader`, `removeHeader`, `headers()`. Before sending changes take effect; afterwards it is the request as sent. |
| `res` | The response (only after it): `status`, `timeMs`, `getHeader`, `headers()`, `text()`, `json()`. |
| `env` | Variables: `get`, `has` see every variable that applies to the request; `set` sets one. |
| `log(...)` / `console.log(...)` | A line in the log; values other than strings as JSON. |
| `test(name, check)` | A named test; see below. |
| `expect(value)` | Checks inside a test, e.g. `expect(res.status).toBe(200)`. |

**Variables set with `env.set`** are the strongest of all – stronger than
the request's own – and apply to this and the following requests of the
collection until it is closed. They are kept in memory only and never
written to disk, so a token a script stores cannot end up in git.

## Tests

```ts
test('status is 200', () => {
  expect(res.status).toBe(200);
});

test('returns the product', () => {
  const product = res.json();
  expect(product).toHaveProperty('id', 42);
  expect(product.tags).toContain('tea');
  expect(res.getHeader('content-type')).toMatch(/json/);
});
```

A failing `expect` fails only its test; the script goes on. Results appear
in the response's *Tests* tab (passed of total, with the reason for each
failure). Tests may be `async`. `expect` outside a test fails the whole
script.

Checks: `toBe`, `toEqual` (deep), `toBeTruthy`, `toBeFalsy`, `toBeDefined`,
`toBeUndefined`, `toBeNull`, `toBeGreaterThan`, `toBeGreaterThanOrEqual`,
`toBeLessThan`, `toBeLessThanOrEqual`, `toContain`, `toMatch`,
`toHaveLength`, `toHaveProperty`; each one can be turned around with `.not`
(`expect(res.status).not.toBe(500)`).

The [command line runner](cli.md) runs the tests too: a request passes only
when all its tests pass, and the JUnit report lists every test.

## Limits

Scripts run in a sandbox ([ADR S1](adr/s1-script-runtime.md)): a fresh
JavaScript engine (QuickJS) per run, stopped after **5 seconds** or **64 MB**
of memory. There is nothing but the API above – no `fetch`, timers, files,
`require` or Node.js modules, no `Intl`.

A script that fails stops what follows: a broken script **before** the
request means the request is not sent; one **after** the response leaves
the response visible with the error above it. The command line runner
counts a failing script after the response as a failed request.

TypeScript types are removed before running, not checked. Syntax errors and
errors while running are reported with the line of the script as written,
e.g. `TypeError: cannot read property 'length' of undefined (line 9)`. The
editor checks types and completes the API while you type
([ADR S2](adr/s2-script-assist.md)).
