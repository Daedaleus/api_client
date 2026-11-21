// Spike S2: the TypeScript language service as a checker and completion
// engine for scripts. The same driver runs in QuickJS (from Rust) and in
// Node.js (V8, standing in for a web worker in the web view).
//
// `read(name)` returns a file's text or undefined: `/api.d.ts` and the
// TypeScript lib files `/lib.*.d.ts`.

/** Diagnostics cases: script → does a diagnostic mention `expect`? */
const CHECKS = [
  ['syntax error', 'const x = ;', 'Expression expected'],
  ['unknown global', 'reqq.method;', "Cannot find name 'reqq'"],
  ['unknown member', 'req.methd;', "Property 'methd' does not exist"],
  ['wrong argument type', "env.set('a', 1);", "not assignable to parameter of type 'string'"],
  ['typo on a derived value', 'const body = res.json<{ items: string[] }>();\nbody.itmes;', "Property 'itmes' does not exist"],
  ['no fetch or Node.js', "fetch('x'); require('fs');", "Cannot find name 'fetch'"],
  ['a correct script', "req.setHeader('X', env.get('token') ?? '');\nexpect(res.status).toBe(200);\nlog(res.json<{ id: number }>().id);", null],
];

/** Completion cases: script with `|` at the cursor → names that must be offered. */
const COMPLETIONS = [
  ['members of req', 'req.|', ['setHeader', 'method', 'headers']],
  ['members of a derived value', 'res.json<{ items: string[]; total: number }>().|', ['items', 'total']],
  ['globals', 'e|', ['env', 'expect']],
];

globalThis.runAssist = function runAssist(ts, read, now) {
  const lines = [];
  const say = (line) => lines.push(line);
  let script = '';
  let version = 0;
  const files = ['/api.d.ts', '/script.ts'];
  const options = {
    target: ts.ScriptTarget.ES2022,
    lib: ['lib.es2024.d.ts'],
    strict: true,
    noEmit: true,
    types: [],
  };
  const host = {
    getScriptFileNames: () => files,
    getScriptVersion: (name) => (name === '/script.ts' ? String(version) : '1'),
    getScriptSnapshot: (name) => {
      const text = name === '/script.ts' ? script : read(name);
      return text === undefined ? undefined : ts.ScriptSnapshot.fromString(text);
    },
    getCurrentDirectory: () => '/',
    getCompilationSettings: () => options,
    getDefaultLibFileName: () => '/lib.es2024.d.ts',
    fileExists: (name) => name === '/script.ts' || read(name) !== undefined,
    readFile: (name) => (name === '/script.ts' ? script : read(name)),
    useCaseSensitiveFileNames: () => true,
  };

  let start = now();
  const service = ts.createLanguageService(host, ts.createDocumentRegistry());
  const diagnostics = (text) => {
    script = text;
    version += 1;
    return [
      ...service.getSyntacticDiagnostics('/script.ts'),
      ...service.getSemanticDiagnostics('/script.ts'),
    ].map((d) => ts.flattenDiagnosticMessageText(d.messageText, '\n'));
  };
  diagnostics('');
  say(`first check (parses the lib files): ${now() - start} ms`);

  let passed = 0;
  const times = [];
  for (const [name, text, expected] of CHECKS) {
    start = now();
    const found = diagnostics(text);
    times.push(now() - start);
    const ok = expected === null ? found.length === 0 : found.some((m) => m.includes(expected));
    if (ok) passed += 1;
    else say(`  FAIL ${name}: ${JSON.stringify(found)}`);
  }
  say(`checks: ${passed}/${CHECKS.length}, per edit ${Math.min(...times)}–${Math.max(...times)} ms`);

  passed = 0;
  const completionTimes = [];
  for (const [name, text, expected] of COMPLETIONS) {
    const position = text.indexOf('|');
    script = text.replace('|', '');
    version += 1;
    start = now();
    const result = service.getCompletionsAtPosition('/script.ts', position, {});
    completionTimes.push(now() - start);
    const names = (result?.entries ?? []).map((entry) => entry.name);
    if (expected.every((wanted) => names.includes(wanted))) passed += 1;
    else say(`  FAIL ${name}: ${names.slice(0, 20).join(', ')}`);
  }
  say(
    `completions: ${passed}/${COMPLETIONS.length}, ${Math.min(...completionTimes)}–${Math.max(...completionTimes)} ms`,
  );
  return lines.join('\n');
};
