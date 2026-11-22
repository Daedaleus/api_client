// The TypeScript language service for the script editor (ADR S2), set up
// once per app in its own QuickJS runtime. `typescript.js` has defined the
// global `ts`; the host passes the declaration files in `__files` (name →
// text) and calls `__check(source)` and `__complete(source, offset)`, both
// returning JSON. Offsets are JavaScript string offsets.
(() => {
  // QuickJS has no Intl; the service sorts completions with Intl.Collator.
  // A plain comparison is enough for API names.
  globalThis.Intl = {
    Collator: function Collator() {
      return { compare: (a, b) => (a < b ? -1 : a > b ? 1 : 0) };
    },
  };

  const ts = globalThis.ts;
  const files = globalThis.__files;
  delete globalThis.__files;
  const SCRIPT = '/script.ts';
  let script = '';
  let version = 0;

  // A module (so top-level `await` is fine, as at run time) that sees the
  // script API from `/api.d.ts` and the ES2024 library – no DOM, so `fetch`
  // and friends are unknown, as when the script runs.
  const options = {
    target: ts.ScriptTarget.ES2022,
    module: ts.ModuleKind.ESNext,
    moduleDetection: ts.ModuleDetectionKind.Force,
    lib: ['lib.es2024.d.ts'],
    strict: true,
    noEmit: true,
    types: [],
  };
  const read = (name) => (name === SCRIPT ? script : files[name]);
  const host = {
    getScriptFileNames: () => ['/api.d.ts', SCRIPT],
    getScriptVersion: (name) => (name === SCRIPT ? String(version) : '1'),
    getScriptSnapshot: (name) => {
      const text = read(name);
      return text === undefined ? undefined : ts.ScriptSnapshot.fromString(text);
    },
    getCurrentDirectory: () => '/',
    getCompilationSettings: () => options,
    getDefaultLibFileName: () => '/lib.es2024.d.ts',
    fileExists: (name) => read(name) !== undefined,
    readFile: read,
    useCaseSensitiveFileNames: () => true,
  };
  const service = ts.createLanguageService(host, ts.createDocumentRegistry());

  const update = (source) => {
    if (source !== script) {
      script = source;
      version += 1;
    }
  };
  const severity = (category) =>
    category === ts.DiagnosticCategory.Error
      ? 'error'
      : category === ts.DiagnosticCategory.Warning
        ? 'warning'
        : 'hint';

  globalThis.__check = (source) => {
    update(source);
    const found = [
      ...service.getSyntacticDiagnostics(SCRIPT),
      ...service.getSemanticDiagnostics(SCRIPT),
    ];
    return JSON.stringify(
      found.map((diagnostic) => {
        const from = diagnostic.start ?? 0;
        return {
          from,
          to: from + (diagnostic.length ?? 0),
          severity: severity(diagnostic.category),
          message: ts.flattenDiagnosticMessageText(diagnostic.messageText, '\n'),
        };
      }),
    );
  };

  /** TypeScript's element kinds as the editor's completion kinds. */
  const KINDS = {
    method: 'function',
    function: 'function',
    'local function': 'function',
    property: 'property',
    getter: 'property',
    setter: 'property',
    var: 'variable',
    'local var': 'variable',
    let: 'variable',
    const: 'variable',
    parameter: 'variable',
    class: 'type',
    interface: 'type',
    type: 'type',
    enum: 'type',
    keyword: 'keyword',
  };
  /** More would only be scrolled past. */
  const MAX_COMPLETIONS = 300;

  globalThis.__complete = (source, offset) => {
    update(source);
    const result = service.getCompletionsAtPosition(SCRIPT, offset, {});
    const entries = [...(result?.entries ?? [])].sort(
      (a, b) => a.sortText.localeCompare(b.sortText) || a.name.localeCompare(b.name),
    );
    return JSON.stringify(
      entries
        .slice(0, MAX_COMPLETIONS)
        .map((entry) => ({ label: entry.name, kind: KINDS[entry.kind] ?? 'other' })),
    );
  };

  // Parses the library files now, so the first check is quick.
  globalThis.__check('');
})();
