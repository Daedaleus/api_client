# ADR S1: Script runtime

Status: accepted (2026-11-21). Issue #9.

## Context

Pre-request scripts, post-response scripts and tests are written in
TypeScript (plain JavaScript works too). They need a JavaScript engine
embedded in the program – no Node.js or Deno on the user's machine – and a
way to turn TypeScript into JavaScript. Scripts come with collections from
other people, so the engine must be a sandbox: no file or network access
beyond the API the app provides, and a script that loops forever or eats
memory must not hang or crash the app.

## What was measured

Spike code: branch `spike/script-runtime`, folder `spikes/script-runtime`
(not merged). Windows 11, release builds (`lto = "thin"`, stripped), median
of several runs. The same 21 language cases, host API and benchmark for
every engine.

| | QuickJS (`rquickjs` 0.14, quickjs-ng) | Boa 0.22 | Boa 0.22 with Intl |
|---|---|---|---|
| Added to the program | **+1.2 MB** | +10.7 MB | +25.9 MB |
| Cold start (engine, context, `1 + 1`) | **0.11 ms** | 0.32 ms | 0.35 ms |
| Benchmark (build and parse 1.3 MB JSON, regex, sort) | **74 ms** | ~420 ms | 469 ms |
| Language cases (ES2015–ES2025, async, RegExp, BigInt …) | 20/21, no `Intl` | 20/21, no `Intl` | 21/21 |
| Stop an endless loop | **wall clock**, after 200 ms; context still usable | only a loop iteration limit (50 M ≈ 0.5 s) | same |
| Stop catastrophic regex backtracking | **yes**, after 200 ms | **no** – still running after 5 s | same |
| Memory limit | **yes** (32 MB: stopped after 28 ms) | none | none |
| No file/network access by default | yes (no `fetch`, `require`, `process`, timers) | yes | yes |
| Host API (`req`, `env`, `log` from Rust) | plain closures with shared state | `Copy` closures or GC-traced captures | same |
| Promise resolved from Rust (async host functions) | yes | yes | yes |
| Build | compiles C (`cc`); a C compiler is present wherever Rust builds (MSVC, gcc/clang) | pure Rust | pure Rust, +160 s build |
| License | MIT | MIT / Unlicense | same |

The language cases cover what scripts in an API client use: classes,
destructuring, template literals, optional chaining, `async`/`await` with
`Promise.all`/`allSettled`/`any`, async iteration, generators, `Map`/`Set`,
RegExp (named groups, lookbehind, `\p{…}`, `d` flag), JSON with Unicode,
`Date`, `BigInt`, ES2019–ES2025 helpers (`Object.groupBy`, Set methods,
iterator helpers), private class members, `Error` causes, `Proxy`, typed
arrays, Unicode normalisation – all pass in both engines. Only `Intl` is
missing without Boa's 15 MB ICU data.

**TypeScript with oxc** (`oxc_parser`, `oxc_transformer`, `oxc_codegen`
0.159): +2.5 MB; 0.5 ms for the first script, then ~0.01 ms. Types,
interfaces, generics, `as` and `!` are removed, enums become plain
JavaScript; the result ran unchanged in QuickJS. Syntax errors come back as
diagnostics with positions. Types are not checked at run time (that is the
checker's job, S2).

**V8** (`deno_core`) was not measured: its build downloads a prebuilt V8
library, and it would add tens of MB and a second async runtime for
scripts that QuickJS runs in a fraction of a millisecond.

## Decision

1. **QuickJS through `rquickjs`** runs the scripts. It is the only candidate
   that can stop any script by wall-clock time (also inside the regex
   engine) and limit its memory – both are needed for scripts from
   collections of other people – and it is the smallest and fastest.
   Boa stays the fallback if a pure-Rust build becomes a requirement; its
   missing interrupt would then have to be solved first.
2. **TypeScript is stripped with oxc** before running. oxc's parser and
   semantic analysis are also a building block for the checker (S2).
3. **One fresh runtime per script run** (0.11 ms): no state leaks from one
   request or collection to the next. Each run gets a time limit (default
   e.g. 5 s, configurable) through the interrupt handler and a memory limit
   (e.g. 64 MB). Scripts run on a blocking worker thread (a QuickJS runtime
   is not `Send`); results come back as events like request runs.
4. **The script API is provided by the host** (`req`, `res`, `env`, `expect`,
   `log`, later sending requests as promises) and described in `.d.ts`;
   nothing else is available – no `fetch`, timers or modules unless the API
   adds them.
5. **No `Intl`:** formatting helpers, if scripts need them, become part of
   the script API instead of 15 MB of ICU data.
6. **Error positions:** stripping changes lines; oxc's source maps map
   script errors back to the TypeScript the user wrote.

## Consequences

- `ac_scripting` depends on `rquickjs` (C code built by `cc`) and the oxc
  crates; `cargo-deny` already allows MIT.
- Scripts imported from Postman or Bruno (JavaScript, using their own APIs
  such as `pm.*` or `bru.*`) need a translation layer or a compatibility
  shim later; the engine itself runs them.
- Test scripts that rely on Node.js modules (`require('crypto')`, `Buffer`)
  do not run; common needs (hashing, Base64, UUIDs) belong in the script
  API.
