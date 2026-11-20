// End-to-end smoke test of the release build (Windows, WebView2).
//
// Starts the real program with the WebView2 debugging port open, drives the
// window through the Chrome DevTools Protocol and checks what the user sees:
//   1. a request to a local test server shows status and formatted body,
//   2. a request to a closed port shows a failure message,
// and the program is still running after each send.
//
// Usage: node e2e/smoke.mjs [path to the program]   (mise run e2e)
// No network access beyond 127.0.0.1; ends within TIMEOUT_MS.
//
// The program gets its own temporary WebView2 data folder: WebView2 refuses
// to share a folder between instances started with different options, so
// without it the test would fail while the app is open on the same machine.

import { spawn, spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import http from 'node:http';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const TIMEOUT_MS = 60_000;
const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..');
const program = process.argv[2] ?? path.join(ROOT, 'target', 'release', 'api-client.exe');

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const steps = [];
/** stdout and stderr of the program, shown when the test fails. */
let output = '';
let exitCode = null;
const webviewData = mkdtempSync(path.join(os.tmpdir(), 'api-client-e2e-'));
let app;
let server;

const deadline = setTimeout(() => finish(`timed out after ${TIMEOUT_MS / 1000} s`), TIMEOUT_MS);

function finish(failure) {
  clearTimeout(deadline);
  for (const step of steps) console.log(`  ✓ ${step}`);
  if (failure) {
    console.log(`  ✗ ${failure}`);
    if (output.trim()) console.log(`--- program output ---\n${output.slice(-4000)}`);
  }
  stopProgram();
  server?.close();
  // WebView2 may hold files a moment longer; leftovers in %TEMP% are harmless.
  rmSync(webviewData, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
  console.log(failure ? 'E2E smoke test FAILED' : 'E2E smoke test passed');
  process.exit(failure ? 1 : 0);
}

/** Ends the program and its WebView2 helper processes (the whole tree). */
function stopProgram() {
  if (!app?.pid || exitCode !== null) return;
  spawnSync('taskkill', ['/pid', String(app.pid), '/t', '/f'], { stdio: 'ignore' });
}

function check(condition, message) {
  if (!condition) finish(message);
  steps.push(message);
}

/** A TCP port nobody listens on (bound once, then released). */
async function freePort() {
  const probe = net.createServer();
  await new Promise((resolve) => probe.listen(0, '127.0.0.1', resolve));
  const { port } = probe.address();
  await new Promise((resolve) => probe.close(resolve));
  return port;
}

// --- Local test server -----------------------------------------------------

server = http.createServer((_request, response) => {
  response.setHeader('Content-Type', 'application/json');
  response.setHeader('X-Test', 'e2e');
  response.end('{"hello":"world","price":1.10,"items":[1,2]}');
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const okUrl = `http://127.0.0.1:${server.address().port}/json`;
const closedUrl = `http://127.0.0.1:${await freePort()}/`;

// --- Start the program -----------------------------------------------------

const debugPort = await freePort();
app = spawn(program, [], {
  env: {
    ...process.env,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${debugPort}`,
    WEBVIEW2_USER_DATA_FOLDER: webviewData,
    RUST_BACKTRACE: '1',
  },
  stdio: ['ignore', 'pipe', 'pipe'],
});
app.stdout.on('data', (chunk) => (output += chunk));
app.stderr.on('data', (chunk) => (output += chunk));
app.on('error', (error) => finish(`cannot start ${program}: ${error.message}`));
app.on('exit', (code) => (exitCode = code));

let page;
for (let attempt = 0; attempt < 120 && !page; attempt++) {
  await sleep(250);
  if (exitCode !== null) finish(`program exited during start (code ${exitCode})`);
  try {
    const targets = await (await fetch(`http://127.0.0.1:${debugPort}/json`)).json();
    page = targets.find((target) => target.type === 'page');
  } catch {
    // WebView2 not up yet
  }
}
check(page, 'window opened');

// --- DevTools connection ---------------------------------------------------

const socket = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve) => socket.addEventListener('open', resolve));
let nextId = 0;
const waiting = new Map();
socket.addEventListener('message', (event) => {
  const message = JSON.parse(event.data);
  waiting.get(message.id)?.(message);
});

function evaluate(expression) {
  return new Promise((resolve) => {
    const id = ++nextId;
    waiting.set(id, (message) => resolve(message.result?.result?.value));
    socket.send(
      JSON.stringify({
        id,
        method: 'Runtime.evaluate',
        params: { expression, returnByValue: true },
      }),
    );
  });
}

/** Polls `expression` until it returns something truthy. */
async function waitFor(expression, what) {
  for (let attempt = 0; attempt < 80; attempt++) {
    if (exitCode !== null) finish(`program exited (code ${exitCode}) while waiting for ${what}`);
    const value = await evaluate(expression);
    if (value) return value;
    await sleep(250);
  }
  finish(`no ${what}`);
}

async function send(url) {
  await evaluate(`(() => {
    const input = document.querySelector('.url-bar__input');
    input.value = ${JSON.stringify(url)};
    input.dispatchEvent(new Event('input', { bubbles: true }));
    document.querySelector('.url-bar button[type=submit]').click();
  })()`);
}

const text = (selector) =>
  `document.querySelector('${selector}')?.textContent.trim().replace(/\\s+/g, ' ')`;

await waitFor(`!!document.querySelector('.url-bar__input')`, 'URL field');
steps.push('URL field shown');

// --- 1. Successful request ---------------------------------------------------

await send(okUrl);
const status = await waitFor(text('.response__status'), 'response status');
check(status.startsWith('200'), `status shown: ${status}`);
const body = await evaluate(`document.querySelector('.response__body')?.textContent ?? ''`);
check(body.includes('"price": 1.10'), 'JSON body formatted, number kept exactly');
check(exitCode === null, 'program still running after send');

// --- 2. Failing request ------------------------------------------------------

await send(closedUrl);
const failure = await waitFor(text('.response__failure'), 'failure message');
check(failure.length > 0, `failure shown: ${failure.slice(0, 60)}…`);
await sleep(500);
check(exitCode === null, 'program still running after a failed send');

finish(null);
