// The driver in Node.js (V8), as a stand-in for a web worker in the web view.
// Usage: node js/node.mjs <path to node_modules/typescript>
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const typescriptDir = path.resolve(process.argv[2]);
const require = createRequire(import.meta.url);

const start = performance.now();
const ts = require(path.join(typescriptDir, 'lib', 'typescript.js'));
console.log(`# TypeScript ${ts.version} language service in Node.js ${process.version} (V8)\n`);
console.log(`load typescript.js: ${Math.round(performance.now() - start)} ms`);

const read = (name) => {
  try {
    if (name === '/api.d.ts') return readFileSync(path.join(here, 'api.d.ts'), 'utf8');
    return readFileSync(path.join(typescriptDir, 'lib', name.slice(1)), 'utf8');
  } catch {
    return undefined;
  }
};
await import('./driver.js');
console.log(globalThis.runAssist(ts, read, () => Math.round(performance.now())));
console.log(`heap used: ${Math.round(process.memoryUsage().heapUsed / 1e6)} MB`);
