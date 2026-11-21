# ADR S2: Script checker and code completion

Status: accepted (2026-11-21). Issue #10.

## Context

The script editor should check TypeScript scripts as the user types and
complete the script API (`req`, `res`, `env`, `expect`, `log`), like Bruno
and Postman do. The API is described once in `.d.ts` declarations. Checker
and completion belong below the replaceable UI (CLAUDE.md): another UI must
get the same results through the application layer. Scripts run in QuickJS,
TypeScript is stripped with oxc ([ADR S1](s1-script-runtime.md)).

## What was measured

Spike code: branch `spike/script-assist`, folder `spikes/script-assist` (not
merged). Windows 11, TypeScript 6.0.3, release builds. Seven check cases and
three completion cases against a sample `api.d.ts`, for three approaches:

- **TS in QuickJS:** the TypeScript language service (`typescript.js`) run in
  QuickJS from Rust – below the UI.
- **TS in V8:** the same driver in Node.js, standing in for a web worker in
  the web view (WebView2 is V8 too) – tied to the UI.
- **Own checks with oxc:** parser diagnostics, unresolved globals from the
  semantic analysis, unknown members of API objects read from `api.d.ts`.

| | TS in QuickJS | TS in V8 (web view) | Own checks (oxc) |
|---|---|---|---|
| Syntax error | ✓ | ✓ | ✓ |
| Unknown global (`reqq`) | ✓ | ✓ | ✓ |
| Unknown API member (`req.methd`) | ✓ | ✓ | ✓ |
| No `fetch`/`require` | ✓ | ✓ | ✓ |
| Correct script without false alarms | ✓ | ✓ | ✓ |
| Wrong argument type (`env.set('a', 1)`) | ✓ | ✓ | ✗ |
| Typo on a computed value (`res.json<T>().itmes`) | ✓ | ✓ | ✗ |
| Complete `req.` / globals | ✓ | ✓ | ✓ |
| Complete members of a computed value | ✓ | ✓ | ✗ |
| Start (load + first check, once) | ~0.25 s + ~0.7 s | ~0.1 s + ~0.1 s | 0.2 ms |
| Check after an edit | 7–9 ms | 4–8 ms | < 0.1 ms |
| Completion | 7–29 ms | 5–13 ms | < 0.1 ms |
| Memory | ~43 MB | ~45 MB | negligible |
| Size | `typescript.js` 9.1 MB (1.7 MB gzip) + lib files | same, in the web bundle | in oxc, already needed for S1 |

- The language service runs in QuickJS unchanged; it only needs an
  `Intl.Collator` for sorting completions, which a few lines of shim provide
  (QuickJS has no `Intl`).
- Own checks stay at the surface: without a type checker they cannot see
  types flowing through a script (`const body = res.json<T>()`), which is
  exactly where help is worth most in test scripts. Getting there means
  writing a type checker.
- **TypeScript 7** is a rewrite in Go and will not run in a JavaScript
  engine. TypeScript 6 is the last JavaScript version; it stays usable for
  years for checking scripts against our own declarations, and a later
  switch to the Go version (as a separate process or library) would only
  replace the adapter behind the port.

## Decision

1. **The TypeScript language service in QuickJS** checks and completes
   scripts, in `ac_scripting` behind the `ScriptAssist` port. The application
   layer exposes diagnostics and completions as view models; CodeMirror only
   shows them. This keeps the full type checker below the replaceable UI.
2. **One long-lived language service on its own worker thread**, started in
   the background when the first script editor opens (about 1 s), then kept:
   checks take ~10 ms per edit, completions ~10–30 ms. Requests carry a
   version; stale answers are dropped.
3. **oxc gives instant feedback** until then: syntax errors from the
   parser, which is needed anyway for stripping (S1).
4. **Shipping:** `typescript.js` and the needed lib files (`lib.es2024`
   chain, no DOM – so `fetch` and friends are unknown, as at run time) are
   bundled with the program, compressed (about 2 MB). The TypeScript version
   is pinned like every other tool.
5. **The script API's `.d.ts`** is the single source for the language
   service, the documentation and the host functions' contract.

## Consequences

- QuickJS runs two things: user scripts (fresh, limited runtimes per run,
  S1) and the language service (one long-lived runtime without the strict
  limits, but only ever fed the user's own editor text).
- The program grows by about 2 MB (compressed TypeScript) and uses ~45 MB
  while a script editor is open.
- Startup cost of `typescript.js` could be cut further with QuickJS bytecode;
  not needed for now.
