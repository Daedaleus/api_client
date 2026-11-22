// End-to-end smoke test of the release build (Windows, WebView2).
//
// Starts the real program with the WebView2 debugging port open, drives the
// window through the Chrome DevTools Protocol and checks what the user sees:
//   1. a request to a local test server shows status and formatted body,
//   2. a request to a closed port shows a failure message,
//   3. a collection (copy of the storage test fixture) opens as a tree; a
//      request in it is edited, saved to its file and sent,
//   4. the same request as POST with a header and a JSON body: saved to the
//      file and received by the server exactly as entered,
//   5. with the collection's environment chosen, `{{baseUrl}}` in the URL is
//      previewed and resolved when sending,
//   6. a secret variable: saved encrypted (never in plain text on disk or in
//      the window), masked in the preview and resolved when sending,
//   7. a team member added as recipient: the list is saved, the secrets are
//      re-encrypted and still work for the user,
//   8. a collection variable is saved and used; a request variable of the
//      same name overrides it,
//   9. bearer auth with the secret as token is sent; set on the collection,
//      the request inherits it; with OAuth 2 client credentials the token is
//      fetched once from the token URL and sent; with the authorization code
//      the (fake) browser signs in once with PKCE and the token is sent,
//  10. a multipart upload and a binary body read a file relative to the
//      collection folder,
//  11. switching the certificate check off shows a warning and is saved,
//  12. a pasted cURL command (chosen in the "New" menu's import submenu)
//      opens as a new tab and is sent; a request is copied as a cURL command,
//  13. a Postman export, an OpenAPI description and a Bruno collection are
//      imported as new collections on disk (through the real IPC command: native file
//      dialogs cannot be driven from here), Postman scripts taken over; an
//      import without a folder stays
//      unsaved until "save as" moves it; a description from a URL gets its
//      relative server resolved,
//  14. large JSON responses (0.9 MB shown formatted, 10 MB shown truncated)
//      appear in time without blocking the window for long,
//  15. scripts: the pre-request script sets a header and a variable used in
//      the URL, the post-response script logs and sets a variable for the
//      next request and runs tests (one failing on purpose), a syntax error
//      is marked while the checker starts, a syntax error
//      stops the request and names its line, as does an error while running,
//  16. Ctrl+Enter sends from the script editor, Ctrl+E focuses the
//      environment picker,
//  17. a pre-request script of the collection is saved and runs before
//      sending,
//  18. sent requests are in the history (secrets masked) and open as a tab,
//  19. a path variable is filled in from the params table and a query
//      parameter switched off there is not sent,
// and the program is still running after each send.
//
// Usage: node e2e/smoke.mjs [path to the program]   (mise run e2e)
// No network access beyond 127.0.0.1; ends within TIMEOUT_MS.
//
// Local only for now: on GitHub's Windows runner the DevTools port never
// opens. Running it in CI (Linux via tauri-driver) is issue #24.
//
// The program gets its own temporary WebView2 data folder: WebView2 refuses
// to share a folder between instances started with different options, so
// without it the test would fail while the app is open on the same machine.
// Likewise its own key file for secrets, so the user's key is never touched.

import { spawn, spawnSync } from 'node:child_process';
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import http from 'node:http';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const TIMEOUT_MS = 180_000;
/** A fresh CI machine needs a while to set up the WebView2 profile. */
const WINDOW_TIMEOUT_MS = 120_000;
const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..');
const program = process.argv[2] ?? path.join(ROOT, 'target', 'release', 'api-client.exe');

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const steps = [];
/** stdout and stderr of the program, shown when the test fails. */
let output = '';
let exitCode = null;
const webviewData = mkdtempSync(path.join(os.tmpdir(), 'api-client-e2e-'));
/** A copy of the example collection the test may change. */
const collections = mkdtempSync(path.join(os.tmpdir(), 'api-client-e2e-collection-'));
const collectionDir = path.join(collections, 'shop-api');
cpSync(path.join(ROOT, 'crates/ac_storage/src/tests/fixtures/shop-api'), collectionDir, {
  recursive: true,
});
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
  rmSync(collections, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
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

/**
 * What WebView2 is doing when no window shows up: its processes with their
 * start options, the ports they listen on, the session and the installed
 * runtime version.
 */
function webviewDiagnostics() {
  const script = `
    $ErrorActionPreference = 'SilentlyContinue'
    "--- session: $([Environment]::UserInteractive) interactive, session id $((Get-Process -Id $PID).SessionId)"
    "--- WebView2 runtime:"
    Get-ItemProperty 'HKLM:\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' | ForEach-Object pv
    "--- program and WebView2 processes:"
    $procs = Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'api-client.exe' -or ($_.Name -eq 'msedgewebview2.exe' -and $_.CommandLine -match 'api-client') }
    $procs | ForEach-Object { "{0} pid {1} parent {2}: {3}" -f $_.Name, $_.ProcessId, $_.ParentProcessId, ($_.CommandLine -replace '\\s+', ' ').Substring(0, [Math]::Min(400, ($_.CommandLine -replace '\\s+', ' ').Length)) }
    "--- listening ports of these processes:"
    Get-NetTCPConnection -State Listen | Where-Object { $_.OwningProcess -in $procs.ProcessId } | ForEach-Object { "{0}:{1} pid {2}" -f $_.LocalAddress, $_.LocalPort, $_.OwningProcess }
  `;
  const result = spawnSync('powershell', ['-NoProfile', '-Command', script], { encoding: 'utf8' });
  return `--- diagnostics ---\n${result.stdout}${result.stderr}`;
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

/** Token requests (OAuth 2) received: Authorization header and form body. */
const tokenRequests = [];
/** Sign-in pages (OAuth 2 authorization code) opened, as query parameters. */
const signIns = [];

/** A JSON array of about `bytes` bytes (spike S3: large responses). */
function largeJson(bytes) {
  const item = (id) =>
    JSON.stringify({
      id,
      name: `Product ${id}`,
      price: id / 100,
      tags: ['a', 'b'],
      stock: { eu: id % 7, us: id % 11 },
    });
  const items = [];
  let size = 2;
  for (let id = 0; size < bytes; id++) {
    const next = item(id);
    items.push(next);
    size += next.length + 1;
  }
  return `[${items.join(',')}]`;
}
const largeBodies = { '/large/0.9': largeJson(900_000), '/large/10': largeJson(10_000_000) };

server = http.createServer((request, response) => {
  response.setHeader('Content-Type', 'application/json');
  response.setHeader('X-Test', 'e2e');
  if (request.url === '/spec/openapi.json') {
    // A description with a relative server, as the Swagger Petstore has.
    response.end(
      JSON.stringify({
        openapi: '3.0.4',
        info: { title: 'Pets from a URL' },
        servers: [{ url: '/api/v3' }],
        paths: { '/pet': { post: { summary: 'Add a pet' } } },
      }),
    );
    return;
  }
  if (request.url in largeBodies) {
    response.end(largeBodies[request.url]);
    return;
  }
  if (request.url === '/token') {
    let form = '';
    request.on('data', (chunk) => (form += chunk));
    request.on('end', () => {
      const fields = new URLSearchParams(form);
      tokenRequests.push({ authorization: request.headers.authorization ?? null, fields });
      const signedIn = fields.get('grant_type') === 'authorization_code';
      response.end(
        signedIn
          ? '{"access_token":"e2e-user-token","token_type":"Bearer"}'
          : '{"access_token":"e2e-token","token_type":"Bearer","expires_in":3600}',
      );
    });
    return;
  }
  if (request.url.startsWith('/authorize?')) {
    // The sign-in page: "signs in" at once and sends the browser back.
    const query = new URL(request.url, 'http://localhost').searchParams;
    signIns.push(query);
    const back = new URL(query.get('redirect_uri'));
    back.searchParams.set('code', 'e2e-code');
    back.searchParams.set('state', query.get('state'));
    response.writeHead(302, { Location: back.toString() });
    response.end();
    return;
  }
  if (!request.url.startsWith('/echo')) {
    response.end('{"hello":"world","price":1.10,"items":[1,2]}');
    return;
  }
  // /echo: what arrived, so the test can check headers and body were sent.
  let received = '';
  request.on('data', (chunk) => (received += chunk));
  request.on('end', () => {
    response.end(
      JSON.stringify({
        method: request.method,
        url: request.url,
        contentType: request.headers['content-type'] ?? null,
        xClient: request.headers['x-client'] ?? null,
        authorization: request.headers.authorization ?? null,
        body: received,
      }),
    );
  });
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const origin = `http://127.0.0.1:${server.address().port}`;
const okUrl = `${origin}/json`;
const echoUrl = `${origin}/echo`;
const closedUrl = `http://127.0.0.1:${await freePort()}/`;

// --- Start the program -----------------------------------------------------

const debugPort = await freePort();
/** The program's key file for secrets (inside the temporary data folder). */
const identityFile = path.join(webviewData, 'identity.txt');
// A CI key of the user must not mix into the test.
const environment = { ...process.env };
delete environment.API_CLIENT_IDENTITY;
app = spawn(program, [], {
  env: {
    ...environment,
    API_CLIENT_IDENTITY_FILE: identityFile,
    // The request history goes there too, never into the user's.
    API_CLIENT_DATA_DIR: webviewData,
    // The browser for OAuth 2 sign-ins: a script that follows the redirect.
    BROWSER: `'${process.execPath}' '${path.join(ROOT, 'apps', 'desktop', 'e2e', 'fake-browser.mjs')}'`,
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
let lastProbe = 'no answer yet';
const started = Date.now();
while (!page && Date.now() - started < WINDOW_TIMEOUT_MS) {
  await sleep(250);
  if (exitCode !== null) finish(`program exited during start (code ${exitCode})`);
  try {
    const targets = await (await fetch(`http://127.0.0.1:${debugPort}/json`)).json();
    page = targets.find((target) => target.type === 'page');
    lastProbe = `targets: ${JSON.stringify(targets.map((target) => target.type))}`;
  } catch (error) {
    // WebView2 not up yet
    lastProbe = `debug port: ${error.cause?.code ?? error.message}`;
  }
}
if (!page) {
  console.log(webviewDiagnostics());
  finish(
    `no window after ${(Date.now() - started) / 1000} s ` +
      `(program running: ${exitCode === null}, ${lastProbe})`,
  );
}
check(page, `window opened after ${((Date.now() - started) / 1000).toFixed(1)} s`);

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
        params: { expression, returnByValue: true, awaitPromise: true },
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

// --- 3. A saved request in a collection ---------------------------------------

/** Clicks the tree entry whose text contains `name`; true if found. */
const clickTreeEntry = (name) => `(() => {
  const entry = [...document.querySelectorAll('.tree-node__main')]
    .find((button) => button.textContent.includes(${JSON.stringify(name)}));
  entry?.click();
  return Boolean(entry);
})()`;

// The collection's environment "Dev" points at the test server (step 5).
const devFile = path.join(collectionDir, 'environments', 'dev.toml');
writeFileSync(devFile, readFileSync(devFile, 'utf8').replace('http://localhost:8080', origin));

// The app reopens remembered collections on start: remember the copy, reload.
await evaluate(`(() => {
  localStorage.setItem('openCollections', ${JSON.stringify(JSON.stringify([collectionDir]))});
  setTimeout(() => location.reload(), 0);
  return true;
})()`);
await sleep(500);
await waitFor(clickTreeEntry('Health'), 'request "Health" in the collection tree');
steps.push('collection reopened as a tree, request opened');
await waitFor(
  `document.querySelector('.request-editor__input')?.value === 'Health'`,
  'request "Health" in the editor',
);

const savedUrl = `${okUrl}?saved=1`;
await evaluate(`(() => {
  const input = document.querySelector('.url-bar__input');
  input.value = ${JSON.stringify(savedUrl)};
  input.dispatchEvent(new Event('input', { bubbles: true }));
  document.querySelector('.request-editor__header .button').click();
})()`);
const healthFile = path.join(collectionDir, 'health.toml');
let saved = '';
for (let attempt = 0; attempt < 40 && !saved.includes(savedUrl); attempt++) {
  await sleep(250);
  saved = readFileSync(healthFile, 'utf8');
}
check(saved.includes(`url = "${savedUrl}"`), 'edited URL saved to health.toml');
check(saved.includes('name = "Health"'), 'saved file keeps the request name');

await evaluate(`document.querySelector('.url-bar button[type=submit]').click()`);
const savedStatus = await waitFor(text('.response__status'), 'response of the saved request');
check(savedStatus.startsWith('200'), `saved request sent: ${savedStatus}`);
check(exitCode === null, 'program still running after saving and sending');

check(
  await evaluate(
    `(() => { const bar = document.querySelector('.sidebar'); return bar.scrollWidth <= bar.clientWidth; })()`,
  ),
  'sidebar fits its width (no sideways scrolling)',
);

// --- 4. Headers and body ------------------------------------------------------

/** Sets a form control's value the way typing would. */
/**
 * Opens the menu of the button at `trigger` and chooses the entries with
 * these ids one after another (submenus first).
 */
async function chooseFromMenu(trigger, ...ids) {
  await evaluate(`document.querySelector(${JSON.stringify(trigger)}).click()`);
  for (const id of ids) {
    const selector = `.menu [data-item="${id}"]`;
    await waitFor(`!!document.querySelector(${JSON.stringify(selector)})`, `menu entry ${id}`);
    await evaluate(`document.querySelector(${JSON.stringify(selector)}).click()`);
  }
}

const setValue = (selector, value, event = 'input') => `(() => {
  const element = document.querySelector(${JSON.stringify(selector)});
  element.value = ${JSON.stringify(value)};
  element.dispatchEvent(new Event(${JSON.stringify(event)}, { bubbles: true }));
  return true;
})()`;

const jsonBody = '{"price": 1.10}';
await evaluate(setValue('.url-bar__select', 'POST', 'change'));
await evaluate(setValue('.url-bar__input', echoUrl));
// Headers section (after Params): add a row and fill it.
await evaluate(`document.querySelectorAll('.request-editor__section')[1].click()`);
await waitFor(`!!document.querySelector('.request-editor__panel .rows-editor')`, 'headers section');
await evaluate(`[...document.querySelectorAll('.rows-editor button')].at(-1).click()`);
await waitFor(`document.querySelectorAll('.rows-editor__input').length === 2`, 'header row');
await evaluate(setValue('.rows-editor__input', 'X-Client'));
await evaluate(`(() => {
  const value = document.querySelectorAll('.rows-editor__input')[1];
  value.value = 'e2e';
  value.dispatchEvent(new Event('input', { bubbles: true }));
})()`);
// Body section: JSON with a number that must arrive exactly as typed.
await evaluate(`document.querySelectorAll('.request-editor__section')[2].click()`);
await waitFor(`!!document.querySelector('input[name=body-kind][value=json]')`, 'body section');
await evaluate(`document.querySelector('input[name=body-kind][value=json]').click()`);
await waitFor(`!!document.querySelector('.body-editor__text')`, 'body text field');
await evaluate(setValue('.body-editor__text', jsonBody));
steps.push('method, header and JSON body entered');

await evaluate(`document.querySelector('.request-editor__header .button').click()`);
saved = '';
for (let attempt = 0; attempt < 40 && !saved.includes('X-Client'); attempt++) {
  await sleep(250);
  saved = readFileSync(healthFile, 'utf8');
}
check(
  saved.includes('method = "POST"') && saved.includes('[[request.headers]]'),
  'method and header saved to health.toml',
);
check(saved.includes('type = "json"') && saved.includes('1.10'), 'JSON body saved to health.toml');

await evaluate(`document.querySelector('.url-bar button[type=submit]').click()`);
const echoed = await waitFor(
  `(() => {
    const body = document.querySelector('.response__body')?.textContent ?? '';
    return body.includes('"method": "POST"') ? body : null;
  })()`,
  'echo of the sent request',
);
check(echoed.includes('"xClient": "e2e"'), 'header received by the server');
check(echoed.includes('"contentType": "application/json"'), 'Content-Type set for the JSON body');
check(echoed.includes('{\\"price\\": 1.10}'), 'body received exactly as entered');
check(exitCode === null, 'program still running after sending headers and body');

// --- 5. Environment and variables -------------------------------------------

await evaluate(`(() => {
  const select = document.querySelector('.environment-picker__select');
  select.value = [...select.options].find((option) => option.text === 'Dev').value;
  select.dispatchEvent(new Event('change', { bubbles: true }));
})()`);
await evaluate(setValue('.url-bar__select', 'GET', 'change'));
await evaluate(setValue('.url-bar__input', '{{baseUrl}}/echo'));
const previewed = await waitFor(
  `(() => {
    const text = document.querySelector('.request-editor__preview')?.textContent ?? '';
    return text.includes(${JSON.stringify(echoUrl)}) ? text : null;
  })()`,
  'URL preview with the environment',
);
steps.push(`environment chosen, URL previewed: ${previewed.trim()}`);

await evaluate(`document.querySelector('.url-bar button[type=submit]').click()`);
const resolved = await waitFor(
  `(() => {
    const body = document.querySelector('.response__body')?.textContent ?? '';
    return body.includes('"method": "GET"') ? body : null;
  })()`,
  'echo of the request sent with the environment',
);
check(resolved.length > 0, '{{baseUrl}} resolved from the environment when sending');
check(exitCode === null, 'program still running after sending with an environment');

// --- 6. Secrets -----------------------------------------------------------------

const secretValue = 'e2e-s3cr3t';
// Open the editor of the chosen environment and add a secret variable.
await evaluate(`document.querySelector('.environment-picker .icon-button').click()`);
await waitFor(`!!document.querySelector('.environment-editor')`, 'environment editor');
await evaluate(
  `[...document.querySelectorAll('.environment-editor .rows-editor .button')].at(-1).click()`,
);
await waitFor(
  `document.querySelectorAll('.environment-editor .rows-editor__secret-input').length === 3`,
  'new variable row',
);
await evaluate(`(() => {
  const inputs = [...document.querySelectorAll('.environment-editor .rows-editor__input')];
  const [name, value] = inputs.slice(-2);
  [...document.querySelectorAll('.environment-editor .rows-editor__secret-input')].at(-1).click();
  name.value = 'token';
  name.dispatchEvent(new Event('input', { bubbles: true }));
  value.value = ${JSON.stringify(secretValue)};
  value.dispatchEvent(new Event('input', { bubbles: true }));
})()`);
await evaluate(`document.querySelector('.environment-editor [type=submit]').click()`);
await waitFor(`!document.querySelector('.environment-editor')`, 'environment saved');

const secretsFile = path.join(collectionDir, 'secrets', 'dev.age');
let encrypted = '';
for (let attempt = 0; attempt < 40 && !encrypted; attempt++) {
  await sleep(250);
  try {
    encrypted = readFileSync(secretsFile, 'utf8');
  } catch {
    // not written yet
  }
}
check(
  encrypted.startsWith('-----BEGIN AGE ENCRYPTED FILE-----') && !encrypted.includes(secretValue),
  'secret saved encrypted to secrets/dev.age',
);
const devText = readFileSync(devFile, 'utf8');
check(
  devText.includes('secret = true') && !devText.includes(secretValue),
  'dev.toml only marks it',
);
const recipients = readFileSync(path.join(collectionDir, '.age-recipients'), 'utf8');
check(/^age1/m.test(recipients), 'own public key written to .age-recipients');
check(readFileSync(identityFile, 'utf8').includes('AGE-SECRET-KEY-'), 'own key created outside');
check(
  !(await evaluate(`document.body.innerHTML.includes(${JSON.stringify(secretValue)})`)),
  'secret value not in the window',
);

await evaluate(setValue('.url-bar__input', '{{baseUrl}}/echo?token={{token}}'));
const masked = await waitFor(
  `(() => {
    const text = document.querySelector('.request-editor__preview')?.textContent ?? '';
    return text.includes('token=••••••') ? text : null;
  })()`,
  'masked secret in the URL preview',
);
check(!masked.includes(secretValue), 'secret masked in the preview');
await evaluate(`document.querySelector('.url-bar button[type=submit]').click()`);
const withSecret = await waitFor(
  `(() => {
    const body = document.querySelector('.response__body')?.textContent ?? '';
    return body.includes('token=') ? body : null;
  })()`,
  'echo of the request with the secret',
);
check(withSecret.includes(`token=${secretValue}`), 'secret decrypted and sent');
check(exitCode === null, 'program still running after sending a secret');

// --- 7. Recipients ----------------------------------------------------------------

/** A throwaway public key (its private key was never kept). */
const teamKey =
  'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOcrctpxeDVU0DQbIWlXheSUkd5TRgpltLt6mDZYjBzg test';
await evaluate(`[...document.querySelectorAll('.environment-picker .icon-button')].at(-1).click()`);
const ownKey = await waitFor(
  `document.querySelector('.recipients-dialog__input--own')?.value || null`,
  'recipients dialog with the own key',
);
check(recipients.includes(ownKey), 'dialog shows the own key from .age-recipients');
await evaluate(`(() => {
  const input = [...document.querySelectorAll('.recipients-dialog__input')].at(-1);
  input.value = ${JSON.stringify(teamKey)};
  input.dispatchEvent(new Event('input', { bubbles: true }));
  input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
})()`);
await waitFor(
  `document.querySelectorAll('.recipients-dialog__item').length === 2`,
  'team key in the list',
);
await evaluate(`document.querySelector('.recipients-dialog [type=submit]').click()`);
await waitFor(`!document.querySelector('.recipients-dialog')`, 'recipients saved');
const recipientsAfter = readFileSync(path.join(collectionDir, '.age-recipients'), 'utf8');
check(
  recipientsAfter.includes(ownKey) && recipientsAfter.includes(teamKey),
  'team key added to .age-recipients',
);
const reencrypted = readFileSync(secretsFile, 'utf8');
check(
  reencrypted !== encrypted && reencrypted.startsWith('-----BEGIN AGE ENCRYPTED FILE-----'),
  'secrets re-encrypted for the new list',
);

await evaluate(setValue('.url-bar__input', '{{baseUrl}}/echo?again=1&token={{token}}'));
await evaluate(`document.querySelector('.url-bar button[type=submit]').click()`);
const afterChange = await waitFor(
  `(() => {
    const body = document.querySelector('.response__body')?.textContent ?? '';
    return body.includes('again=1') ? body : null;
  })()`,
  'echo after re-encrypting',
);
check(afterChange.includes(`token=${secretValue}`), 'secret still decrypts after re-encrypting');
check(exitCode === null, 'program still running after changing recipients');

// --- 8. Variable precedence -----------------------------------------------------

/** Waits for an echo whose URL contains `part` and returns the body. */
const echoWith = (part, what) =>
  waitFor(
    `(() => {
      const body = document.querySelector('.response__body')?.textContent ?? '';
      return body.includes(${JSON.stringify(part)}) ? body : null;
    })()`,
    what,
  );

// Collection variable `path`, through the dialog of the collection header.
await chooseFromMenu('.sidebar__collection-header .menu-button', 'settings');
await waitFor(`!!document.querySelector('.defaults-dialog')`, 'collection variables dialog');
await evaluate(`document.querySelector('.defaults-dialog .rows-editor .button').click()`);
await waitFor(
  `document.querySelectorAll('.defaults-dialog .rows-editor__input').length === 2`,
  'collection variable row',
);
await evaluate(`(() => {
  const [name, value] = document.querySelectorAll('.defaults-dialog .rows-editor__input');
  name.value = 'path';
  name.dispatchEvent(new Event('input', { bubbles: true }));
  value.value = 'from-collection';
  value.dispatchEvent(new Event('input', { bubbles: true }));
  document.querySelector('.defaults-dialog [type=submit]').click();
})()`);
await waitFor(`!document.querySelector('.defaults-dialog')`, 'collection variables saved');
const collectionText = readFileSync(path.join(collectionDir, 'collection.toml'), 'utf8');
check(
  collectionText.includes('[[variables]]') && collectionText.includes('from-collection'),
  'collection variable saved to collection.toml',
);

await evaluate(setValue('.url-bar__input', '{{baseUrl}}/echo?scope={{path}}'));
await evaluate(`document.querySelector('.url-bar button[type=submit]').click()`);
await echoWith('scope=from-collection', 'echo with the collection variable');
steps.push('collection variable used when sending');

// A request variable of the same name is stronger.
await evaluate(`document.querySelectorAll('.request-editor__section')[4].click()`);
await waitFor(`!!document.querySelector('.request-editor__panel .rows-editor')`, 'variables tab');
await evaluate(`document.querySelector('.request-editor__panel .rows-editor .button').click()`);
await waitFor(
  `document.querySelectorAll('.request-editor__panel .rows-editor__input').length === 2`,
  'request variable row',
);
await evaluate(`(() => {
  const [name, value] = document.querySelectorAll('.request-editor__panel .rows-editor__input');
  name.value = 'path';
  name.dispatchEvent(new Event('input', { bubbles: true }));
  value.value = 'from-request';
  value.dispatchEvent(new Event('input', { bubbles: true }));
})()`);
await waitFor(
  `(document.querySelector('.request-editor__preview')?.textContent ?? '').includes('scope=from-request')`,
  'preview with the request variable',
);
await evaluate(`document.querySelector('.url-bar button[type=submit]').click()`);
await echoWith('scope=from-request', 'echo with the request variable');
steps.push('request variable overrides the collection variable');

await evaluate(`document.querySelector('.request-editor__header .button').click()`);
saved = '';
for (let attempt = 0; attempt < 40 && !saved.includes('from-request'); attempt++) {
  await sleep(250);
  saved = readFileSync(healthFile, 'utf8');
}
check(saved.includes('[[request.variables]]'), 'request variable saved to health.toml');
check(exitCode === null, 'program still running after sending with layered variables');

// --- 9. Authentication ----------------------------------------------------------

// Bearer auth on the collection, the token being the secret of step 6.
await chooseFromMenu('.sidebar__collection-header .menu-button', 'settings');
await waitFor(`!!document.querySelector('.defaults-dialog')`, 'collection settings dialog');
await evaluate(`document.querySelectorAll('.defaults-dialog__section')[1].click()`);
await waitFor(`!!document.querySelector('.defaults-dialog .auth-editor__select')`, 'auth section');
await evaluate(setValue('.defaults-dialog .auth-editor__select', 'bearer', 'change'));
await waitFor(`!!document.querySelector('.defaults-dialog .auth-editor__input')`, 'token field');
await evaluate(setValue('.defaults-dialog .auth-editor__input', '{{token}}'));
await evaluate(`document.querySelector('.defaults-dialog [type=submit]').click()`);
await waitFor(`!document.querySelector('.defaults-dialog')`, 'collection auth saved');
const withAuth = readFileSync(path.join(collectionDir, 'collection.toml'), 'utf8');
check(
  withAuth.includes('type = "bearer"') &&
    withAuth.includes('{{token}}') &&
    !withAuth.includes(secretValue),
  'collection auth saved with the variable, not the secret',
);

await evaluate(setValue('.url-bar__input', '{{baseUrl}}/echo?auth=1'));
await evaluate(`document.querySelector('.url-bar button[type=submit]').click()`);
const authorized = await echoWith('auth=1', 'echo with inherited auth');
check(
  authorized.includes(`"authorization": "Bearer ${secretValue}"`),
  'request inherits bearer auth with the decrypted secret',
);
check(exitCode === null, 'program still running after sending with auth');

// OAuth 2 client credentials on the collection: the token is fetched once.
await chooseFromMenu('.sidebar__collection-header .menu-button', 'settings');
await waitFor(`!!document.querySelector('.defaults-dialog')`, 'collection settings dialog');
await evaluate(`document.querySelectorAll('.defaults-dialog__section')[1].click()`);
await waitFor(`!!document.querySelector('.defaults-dialog .auth-editor__select')`, 'auth section');
await evaluate(setValue('.defaults-dialog .auth-editor__select', 'oauth2', 'change'));
await waitFor(
  `document.querySelectorAll('.defaults-dialog .auth-editor__input').length === 4`,
  'OAuth 2 fields',
);
await evaluate(`(() => {
  const inputs = document.querySelectorAll('.defaults-dialog .auth-editor__input');
  ['{{baseUrl}}/token', 'e2e-client', '{{token}}', 'read'].forEach((value, index) => {
    inputs[index].value = value;
    inputs[index].dispatchEvent(new Event('input', { bubbles: true }));
  });
  return true;
})()`);
await evaluate(`document.querySelector('.defaults-dialog [type=submit]').click()`);
await waitFor(`!document.querySelector('.defaults-dialog')`, 'collection OAuth 2 saved');
const withOAuth = readFileSync(path.join(collectionDir, 'collection.toml'), 'utf8');
check(
  withOAuth.includes('type = "oauth2"') &&
    withOAuth.includes('client_secret = "{{token}}"') &&
    !withOAuth.includes(secretValue) &&
    !withOAuth.includes('e2e-token'),
  'OAuth 2 saved with the variable, never the secret or the token',
);
for (const round of [1, 2]) {
  await evaluate(setValue('.url-bar__input', `{{baseUrl}}/echo?oauth=${round}`));
  await evaluate(`document.querySelector('.url-bar button[type=submit]').click()`);
  const echoed = await echoWith(`oauth=${round}`, `echo with OAuth 2 (${round})`);
  check(
    echoed.includes('"authorization": "Bearer e2e-token"'),
    `OAuth 2 token sent as bearer (${round})`,
  );
}
check(
  tokenRequests.length === 1 && tokenRequests[0]?.authorization?.startsWith('Basic '),
  `token fetched once with client authentication (${tokenRequests.length})`,
);
check(exitCode === null, 'program still running after sending with OAuth 2');

// The authorization code: the browser signs in once, with PKCE.
await chooseFromMenu('.sidebar__collection-header .menu-button', 'settings');
await waitFor(`!!document.querySelector('.defaults-dialog')`, 'collection settings dialog');
await evaluate(`document.querySelectorAll('.defaults-dialog__section')[1].click()`);
await waitFor(
  `document.querySelectorAll('.defaults-dialog .auth-editor__select').length === 2`,
  'OAuth 2 grant',
);
await evaluate(`(() => {
  const grant = document.querySelectorAll('.defaults-dialog .auth-editor__select')[1];
  grant.value = 'authorizationCode';
  grant.dispatchEvent(new Event('change', { bubbles: true }));
  return true;
})()`);
await waitFor(
  `document.querySelectorAll('.defaults-dialog .auth-editor__input').length === 6`,
  'sign-in fields',
);
await evaluate(`(() => {
  const inputs = document.querySelectorAll('.defaults-dialog .auth-editor__input');
  // Authorization URL, and no client secret: a public client.
  for (const [index, value] of [[1, '{{baseUrl}}/authorize'], [4, '']]) {
    inputs[index].value = value;
    inputs[index].dispatchEvent(new Event('input', { bubbles: true }));
  }
  return true;
})()`);
await evaluate(`document.querySelector('.defaults-dialog [type=submit]').click()`);
await waitFor(`!document.querySelector('.defaults-dialog')`, 'collection sign-in saved');
const withSignIn = readFileSync(path.join(collectionDir, 'collection.toml'), 'utf8');
check(
  withSignIn.includes('grant = "authorization-code"') && !withSignIn.includes('e2e-user-token'),
  'authorization code saved, never the token',
);
for (const round of [1, 2]) {
  await evaluate(setValue('.url-bar__input', `{{baseUrl}}/echo?pkce=${round}`));
  await evaluate(`document.querySelector('.url-bar button[type=submit]').click()`);
  const echoed = await echoWith(`pkce=${round}`, `echo after signing in (${round})`);
  check(
    echoed.includes('"authorization": "Bearer e2e-user-token"'),
    `signed-in token sent as bearer (${round})`,
  );
}
const exchange = tokenRequests.at(-1)?.fields;
check(
  signIns.length === 1 &&
    signIns[0].get('code_challenge_method') === 'S256' &&
    exchange?.get('code') === 'e2e-code' &&
    exchange?.get('client_id') === 'e2e-client' &&
    (exchange?.get('code_verifier') ?? '').length === 43,
  `signed in once with PKCE (${signIns.length} sign-ins)`,
);
check(exitCode === null, 'program still running after signing in');

// --- 10. Multipart and binary bodies ---------------------------------------------

const fileContent = 'hello from e2e';
mkdirSync(path.join(collectionDir, 'files'), { recursive: true });
writeFileSync(path.join(collectionDir, 'files', 'hello.txt'), fileContent);

await evaluate(`document.querySelectorAll('.request-editor__section')[2].click()`);
await waitFor(`!!document.querySelector('input[name=body-kind][value=multipart]')`, 'body kinds');
await evaluate(`document.querySelector('input[name=body-kind][value=multipart]').click()`);
await waitFor(`!!document.querySelector('.multipart-editor')`, 'multipart editor');
await evaluate(`document.querySelector('.multipart-editor .button').click()`);
await waitFor(
  `document.querySelectorAll('.multipart-editor__input').length === 2`,
  'multipart field',
);
await evaluate(`(() => {
  const [name, value] = document.querySelectorAll('.multipart-editor__input');
  name.value = 'upload';
  name.dispatchEvent(new Event('input', { bubbles: true }));
  document.querySelectorAll('.multipart-editor input[type=checkbox]')[1].click();
  value.value = 'files/hello.txt';
  value.dispatchEvent(new Event('input', { bubbles: true }));
})()`);
await evaluate(setValue('.url-bar__input', '{{baseUrl}}/echo?multi=1'));
await evaluate(`document.querySelector('.url-bar button[type=submit]').click()`);
const uploaded = await echoWith('multi=1', 'echo of the multipart upload');
check(
  uploaded.includes('multipart/form-data') &&
    uploaded.includes('filename=\\"hello.txt\\"') &&
    uploaded.includes(fileContent),
  'multipart upload sends the file from the collection folder',
);

await evaluate(`document.querySelector('input[name=body-kind][value=binary]').click()`);
await waitFor(`!!document.querySelector('.body-editor__file-input')`, 'binary body field');
await evaluate(setValue('.body-editor__file-input', 'files/hello.txt'));
await evaluate(setValue('.url-bar__input', '{{baseUrl}}/echo?bin=1'));
await evaluate(`document.querySelector('.url-bar button[type=submit]').click()`);
const binary = await echoWith('bin=1', 'echo of the binary body');
check(
  binary.includes('"contentType": "application/octet-stream"') &&
    binary.includes(`"body": "${fileContent}"`),
  'binary body sends the file as it is',
);
check(exitCode === null, 'program still running after sending files');

// --- 11. Certificate check --------------------------------------------------------

await evaluate(`document.querySelectorAll('.request-editor__section')[5].click()`);
await waitFor(`!!document.querySelector('.certificate-check-field__select')`, 'settings tab');
check(
  !(await evaluate(`!!document.querySelector('.request-editor__insecure')`)),
  'no warning while certificates are checked',
);
await evaluate(setValue('.certificate-check-field__select', 'off', 'change'));
await waitFor(
  `!!document.querySelector('.request-editor__insecure')`,
  'warning about the switched-off check',
);
steps.push('switching the certificate check off shows a warning');
await evaluate(`document.querySelector('.request-editor__header .button').click()`);
saved = '';
for (let attempt = 0; attempt < 40 && !saved.includes('verify_tls'); attempt++) {
  await sleep(250);
  saved = readFileSync(healthFile, 'utf8');
}
check(saved.includes('verify_tls = false'), 'switched-off check saved to health.toml');
await evaluate(setValue('.url-bar__input', '{{baseUrl}}/echo?unchecked=1'));
await evaluate(`document.querySelector('.url-bar button[type=submit]').click()`);
await echoWith('unchecked=1', 'echo without certificate check');
check(exitCode === null, 'program still running after sending without certificate check');

// --- 12. cURL ---------------------------------------------------------------------

const pasted = `curl '${echoUrl}?curl=1' -H 'X-Client: pasted' --data-raw '{"a":1}' --compressed`;
// Toolbar: Open, New, Paste cURL, Import Postman.
await chooseFromMenu('.sidebar__toolbar .menu-button', 'import', 'import-curl');
await waitFor(`!!document.querySelector('.curl-import__text')`, 'cURL import dialog');
await evaluate(setValue('.curl-import__text', pasted));
await evaluate(`document.querySelector('.curl-import [type=submit]').click()`);
await waitFor(`!document.querySelector('.curl-import')`, 'pasted command opened');
const notice = await evaluate(
  `[...document.querySelectorAll('.request-editor__note')].map((note) => note.textContent).join(' ')`,
);
check(notice.includes('--compressed'), 'ignored cURL options are pointed out');
await evaluate(`document.querySelector('.url-bar button[type=submit]').click()`);
const fromCurl = await echoWith('curl=1', 'echo of the pasted cURL command');
check(
  fromCurl.includes('"method": "POST"') && fromCurl.includes('"xClient": "pasted"'),
  'pasted cURL command sent with its method and header',
);

await evaluate(`(() => {
  window.__copied = null;
  Object.defineProperty(navigator, 'clipboard', {
    value: { writeText: async (text) => { window.__copied = text; } },
    configurable: true,
  });
  document.querySelector('.request-editor__tools .button').click();
})()`);
const copiedCurl = await waitFor(`window.__copied`, 'copied cURL command');
check(
  copiedCurl.startsWith('curl') &&
    copiedCurl.includes(`'${echoUrl}?curl=1'`) &&
    copiedCurl.includes("--header 'X-Client: pasted'"),
  'request copied as a cURL command',
);
check(exitCode === null, 'program still running after the cURL round trip');

// --- 13. Postman import -----------------------------------------------------------

const postmanFile = path.join(
  ROOT,
  'crates/ac_interchange/src/tests/fixtures/shop.postman_collection.json',
);
const importedDir = path.join(collections, 'imported-from-postman');
const importAck = await evaluate(`window.__TAURI_INTERNALS__.invoke('execute', {
  command: {
    type: 'importCollection',
    file: ${JSON.stringify(postmanFile)},
    path: ${JSON.stringify(importedDir)},
  },
})`);
check(
  importAck?.type === 'collectionImported' &&
    importAck.collection.name === 'Shop API' &&
    importAck.skipped.length === 2,
  'Postman export imported, left-out parts reported',
);
const importedSettings = readFileSync(path.join(importedDir, 'collection.toml'), 'utf8');
check(
  importedSettings.includes('name = "Shop API"') && importedSettings.includes('type = "bearer"'),
  'imported collection written with its auth',
);
const login = readFileSync(path.join(importedDir, 'auth', 'login.toml'), 'utf8');
check(login.includes('url = "{{baseUrl}}/auth/login"'), 'imported request written to its folder');
check(
  login.includes('[request.scripts]') && login.includes('pm.environment.set'),
  'imported Postman test script taken over',
);
const openapiFile = path.join(ROOT, 'crates/ac_interchange/src/tests/fixtures/shop.openapi.yaml');
const fromOpenapi = path.join(collections, 'imported-from-openapi');
const openapiAck = await evaluate(`window.__TAURI_INTERNALS__.invoke('execute', {
  command: {
    type: 'importCollection',
    file: ${JSON.stringify(openapiFile)},
    path: ${JSON.stringify(fromOpenapi)},
  },
})`);
check(
  openapiAck?.type === 'collectionImported' && openapiAck.skipped.length === 2,
  'OpenAPI description imported, left-out parts reported',
);
const openapiSettings = readFileSync(path.join(fromOpenapi, 'collection.toml'), 'utf8');
check(
  openapiSettings.includes('value = "https://eu.api.example.com/v1"'),
  'OpenAPI server written as baseUrl',
);
const brunoFile = path.join(ROOT, 'crates/ac_interchange/src/tests/fixtures/bruno-shop/bruno.json');
const fromBruno = path.join(collections, 'imported-from-bruno');
const brunoAck = await evaluate(`window.__TAURI_INTERNALS__.invoke('execute', {
  command: {
    type: 'importCollection',
    file: ${JSON.stringify(brunoFile)},
    path: ${JSON.stringify(fromBruno)},
  },
})`);
check(
  brunoAck?.type === 'collectionImported' &&
    brunoAck.collection.name === 'Shop (Bruno)' &&
    brunoAck.collection.environments.length === 1,
  'Bruno collection imported with its environment',
);
const brunoEnvironment = readFileSync(path.join(fromBruno, 'environments', 'local.toml'), 'utf8');
check(
  brunoEnvironment.includes('baseUrl') && brunoEnvironment.includes('secret = true'),
  'Bruno environment written, secrets marked',
);
// Without a target folder an import stays unsaved until "save as".
const unsavedAck = await evaluate(`window.__TAURI_INTERNALS__.invoke('execute', {
  command: { type: 'importCollection', file: ${JSON.stringify(openapiFile)}, path: null },
})`);
const unsavedRoot = unsavedAck?.collection?.root ?? '';
check(
  unsavedAck?.collection?.temporary === true && unsavedRoot.includes('api-client-unsaved'),
  'import without a folder stays unsaved in a temporary folder',
);
const savedAs = path.join(collections, 'saved-from-unsaved');
const savedAck = await evaluate(`window.__TAURI_INTERNALS__.invoke('execute', {
  command: {
    type: 'saveCollectionAs',
    collectionId: ${JSON.stringify(unsavedAck?.collection?.id ?? '')},
    path: ${JSON.stringify(savedAs)},
  },
})`);
check(
  savedAck?.collection?.temporary === false &&
    readFileSync(path.join(savedAs, 'collection.toml'), 'utf8').includes('Shop API') &&
    !existsSync(unsavedRoot),
  'save as moves the collection to the chosen folder',
);
// From a URL: downloaded through the HTTP adapter, relative server resolved.
const fromUrl = await evaluate(`window.__TAURI_INTERNALS__.invoke('import_url', {
  url: ${JSON.stringify(`${origin}/spec/openapi.json`)},
})`);
check(
  fromUrl?.collection?.name === 'Pets from a URL' &&
    fromUrl.collection.temporary === true &&
    fromUrl.collection.variables?.[0]?.value === `${origin}/api/v3`,
  `import from a URL resolves the relative server (${fromUrl?.collection?.variables?.[0]?.value})`,
);
check(exitCode === null, 'program still running after importing');

// --- 14. Large responses -----------------------------------------------------------

// Measured in the window: from the click until the body is in the page and
// painted, and the longest gap between two frames meanwhile (a frozen
// window shows as a long gap).
const measure = (expectedStart, minimum) => `new Promise((resolve) => {
  const start = performance.now();
  let last = start;
  let maxGap = 0;
  let shown = null;
  const tick = (now) => {
    maxGap = Math.max(maxGap, now - last);
    last = now;
    if (shown === null) requestAnimationFrame(tick);
    else resolve({ shownMs: Math.round(shown), maxGapMs: Math.round(maxGap) });
  };
  requestAnimationFrame(tick);
  // Formatted bodies are in the code viewer, which draws only the visible
  // lines (its full length is on the element); others are plain text.
  const done = () => {
    const viewer = document.querySelector('.response__body .code-viewer');
    const shownText = viewer
      ? (viewer.querySelector('.cm-content')?.textContent ?? '')
      : (document.querySelector('pre.response__body')?.textContent ?? '');
    const length = viewer ? Number(viewer.dataset.length) : shownText.length;
    return shownText.startsWith(${JSON.stringify(expectedStart)}) && length >= ${minimum};
  };
  new MutationObserver((_, observer) => {
    if (!done()) return;
    observer.disconnect();
    requestAnimationFrame(() => (shown = performance.now() - start));
  }).observe(document.body, { childList: true, subtree: true, characterData: true });
  document.querySelector('.url-bar button[type=submit]').click();
})`;
for (const [size, minimum, formatted] of [
  ['0.9', 900_000, true],
  ['10', 1_000_000, false],
]) {
  await evaluate(setValue('.url-bar__select', 'GET', 'change'));
  await evaluate(setValue('.url-bar__input', `${origin}/large/${size}`));
  // Lines of the code viewer join without line breaks: "[" then "  {".
  const { shownMs, maxGapMs } = await evaluate(measure(formatted ? '[  {' : '[{', minimum));
  const truncated = await evaluate(
    `[...document.querySelectorAll('.response__note')].some((note) => note.textContent.includes('MB'))`,
  );
  check(
    shownMs < 5000 && maxGapMs < 2000 && truncated !== formatted,
    `${size} MB JSON response shown ${formatted ? 'formatted' : 'truncated'} in ${shownMs} ms (longest frame gap ${maxGapMs} ms)`,
  );
}
await evaluate(`document.querySelectorAll('.response__tab')[1].click()`);
await waitFor(`!!document.querySelector('.response__headers')`, 'headers after a large response');
check(exitCode === null, 'program still running after large responses');

// --- 15. Scripts -------------------------------------------------------------------

/**
 * Replaces the text of the `index`-th script editor (CodeMirror) the way
 * typing would: focus, select all, insert.
 */
const setScript = (index, text) => `(() => {
  const content = document.querySelectorAll('.scripts-editor .cm-content')[${index}];
  content.focus();
  document.execCommand('selectAll');
  document.execCommand('insertText', false, ${JSON.stringify(text)});
  return true;
})()`;

// Step 14 left the headers of the response open.
await evaluate(
  `[...document.querySelectorAll('.response__tab')].find((tab) => tab.textContent.includes('Body')).click()`,
);
await evaluate(`document.querySelectorAll('.request-editor__section')[6].click()`);
await waitFor(
  `document.querySelectorAll('.scripts-editor .cm-content').length === 2`,
  'script editors',
);

// A syntax error is marked at once, while the TypeScript service starts…
await evaluate(setScript(0, 'const = ;'));
await waitFor(`!!document.querySelector('.scripts-editor .cm-lintRange-error')`, 'syntax mark');
check(true, 'syntax error marked while the checker starts');
await evaluate(setScript(0, 'req.method;'));
await waitFor(`!document.querySelector('.scripts-editor .cm-lintRange-error')`, 'mark removed');
// … then the checker marks a wrong member once it has started…
await evaluate(setScript(0, 'req.methd;'));
await waitFor(`!!document.querySelector('.scripts-editor .cm-lintRange-error')`, 'checker mark');
check(true, 'checker marks an unknown member of req');
// … and completion offers the API.
await evaluate(setScript(0, 'req.'));
const offered = await waitFor(
  `document.querySelector('.cm-tooltip-autocomplete')?.textContent ?? null`,
  'completion list',
);
check(
  offered.includes('setHeader') && offered.includes('method'),
  'completion offers the members of req',
);

await evaluate(
  setScript(
    0,
    "const client: string = 'from-script';\nreq.setHeader('X-Client', client);\nenv.set('fromPre', 'v1');\nlog('pre ran');",
  ),
);
await evaluate(
  setScript(
    1,
    "const echoed = res.json<{ xClient: string; method: string }>();\nlog('status ' + res.status + ' ' + echoed.xClient);\nenv.set('echoed', echoed.method);\ntest('status is 200', () => expect(res.status).toBe(200));\ntest('fails on purpose', () => expect(echoed).toHaveProperty('missing'));",
  ),
);
await evaluate(setValue('.url-bar__select', 'GET', 'change'));
await evaluate(setValue('.url-bar__input', `${echoUrl}?script={{fromPre}}`));
await evaluate(`document.querySelector('.url-bar button[type=submit]').click()`);
const scripted = await echoWith('script=v1', 'echo after the pre-request script');
check(
  scripted.includes('"xClient": "from-script"'),
  'pre-request script set a header and a variable used in the URL',
);
await waitFor(
  `[...document.querySelectorAll('.response__tab')].some((tab) => tab.textContent.includes('Log'))`,
  'log tab',
);
await evaluate(
  `[...document.querySelectorAll('.response__tab')].find((tab) => tab.textContent.includes('Log')).click()`,
);
const logText = await waitFor(
  `document.querySelector('.response__logs')?.textContent ?? null`,
  'script log',
);
check(
  logText.includes('pre ran') && logText.includes('status 200 from-script'),
  `both scripts logged (${JSON.stringify(logText)})`,
);
const testsTab = await evaluate(
  `[...document.querySelectorAll('.response__tab')].find((tab) => tab.textContent.includes('Tests'))?.textContent ?? ''`,
);
await evaluate(
  `[...document.querySelectorAll('.response__tab')].find((tab) => tab.textContent.includes('Tests')).click()`,
);
const testList = await waitFor(
  `document.querySelector('.test-results')?.textContent ?? null`,
  'test results',
);
check(
  testsTab.includes('1/2') &&
    testList.includes('status is 200') &&
    testList.includes('expected') &&
    testList.includes('to have property missing'),
  `tests listed with the failure (${JSON.stringify(testsTab)})`,
);
await evaluate(setValue('.url-bar__input', `${echoUrl}?second={{echoed}}`));
await evaluate(`document.querySelector('.url-bar button[type=submit]').click()`);
await waitFor(
  `[...document.querySelectorAll('.response__tab')].some((tab) => tab.textContent.includes('Body'))`,
  'response tabs',
);
await evaluate(
  `[...document.querySelectorAll('.response__tab')].find((tab) => tab.textContent.includes('Body')).click()`,
);
await echoWith('second=GET', 'echo with the variable from the post-response script');
check(true, 'post-response script set a variable for the next request');

// A broken script stops the request and says where.
await evaluate(setScript(0, 'log(1);\nconst = ;'));
await evaluate(`document.querySelector('.url-bar button[type=submit]').click()`);
const scriptFailure = await waitFor(
  `document.querySelector('.response__failure')?.textContent ?? null`,
  'script failure',
);
check(
  scriptFailure.includes('line 2'),
  `syntax error of the pre-request script reported (${JSON.stringify(scriptFailure)})`,
);
// So does an error while running, with the line as written.
await evaluate(setScript(0, 'type A = { x: number };\n\nconst a: any = undefined;\na.x;'));
await evaluate(`document.querySelector('.url-bar button[type=submit]').click()`);
const runtimeFailure = await waitFor(
  `(() => {
    const text = document.querySelector('.response__failure')?.textContent ?? '';
    return text.includes('TypeError') ? text : null;
  })()`,
  'runtime failure',
);
check(
  runtimeFailure.includes('line 4'),
  `runtime error of the pre-request script names its line (${JSON.stringify(runtimeFailure)})`,
);
check(exitCode === null, 'program still running after scripts');

// --- 16. Keyboard shortcuts -----------------------------------------------------------

/** Presses Ctrl + `key` as a real key press (not a synthetic DOM event). */
async function pressCtrl(key, code) {
  for (const type of ['rawKeyDown', 'keyUp']) {
    await new Promise((resolve) => {
      const id = ++nextId;
      waiting.set(id, resolve);
      socket.send(
        JSON.stringify({
          id,
          method: 'Input.dispatchKeyEvent',
          // Modifier 2 = Ctrl.
          params: {
            type,
            modifiers: 2,
            key,
            code,
            windowsVirtualKeyCode: key === 'Enter' ? 13 : key.toUpperCase().charCodeAt(0),
          },
        }),
      );
    });
  }
}

// Ctrl+Enter in the script editor sends instead of adding a line.
await evaluate(setValue('.url-bar__input', `${echoUrl}?keys=1`));
await evaluate(setScript(0, 'log(1);'));
await pressCtrl('Enter', 'Enter');
await echoWith('keys=1', 'echo sent with Ctrl+Enter');
const scriptAfter = await evaluate(
  `document.querySelectorAll('.scripts-editor .cm-content')[0].textContent`,
);
check(
  scriptAfter === 'log(1);',
  `Ctrl+Enter sent from the script editor without a new line (${JSON.stringify(scriptAfter)})`,
);
// The environment picker belongs to requests of a collection.
await waitFor(clickTreeEntry('Health'), 'request "Health" in the collection tree');
await waitFor(`!!document.querySelector('.environment-picker__select')`, 'environment picker');
await pressCtrl('e', 'KeyE');
check(
  await evaluate(`document.activeElement?.classList.contains('environment-picker__select')`),
  'Ctrl+E focuses the environment picker',
);

// --- 17. Collection scripts ---------------------------------------------------------

// A pre-request script of the collection, set in its settings dialog,
// runs before the request's own and is saved to collection.toml.
await chooseFromMenu('.sidebar__collection-header .menu-button', 'settings');
await waitFor(`!!document.querySelector('.defaults-dialog')`, 'collection settings dialog');
await evaluate(`document.querySelectorAll('.defaults-dialog__section')[2].click()`);
await waitFor(
  `document.querySelectorAll('.defaults-dialog .cm-content').length === 2`,
  'collection script editors',
);
await evaluate(`(() => {
  const content = document.querySelectorAll('.defaults-dialog .cm-content')[0];
  content.focus();
  document.execCommand('selectAll');
  document.execCommand('insertText', false, "req.setHeader('X-Client', 'from-collection');");
})()`);
await evaluate(`document.querySelector('.defaults-dialog [type=submit]').click()`);
await waitFor(`!document.querySelector('.defaults-dialog')`, 'collection scripts saved');
const collectionToml = readFileSync(path.join(collectionDir, 'collection.toml'), 'utf8');
check(
  collectionToml.includes('[scripts]') && collectionToml.includes('from-collection'),
  'collection script saved to collection.toml',
);
await evaluate(setValue('.url-bar__input', `${echoUrl}?collection=1`));
await evaluate(`document.querySelector('.url-bar button[type=submit]').click()`);
const fromCollection = await echoWith('collection=1', 'echo with the collection script');
check(
  fromCollection.includes('"xClient": "from-collection"'),
  'collection pre-request script ran before sending',
);

// --- 18. Request history --------------------------------------------------------------

// Every sent request is in the history, the secret of step 6 never.
const historyFile = path.join(webviewData, 'history.jsonl');
const historyText = readFileSync(historyFile, 'utf8');
check(
  historyText.includes('/echo') && historyText.includes('"version":1'),
  'sent requests recorded in the history file',
);
check(!historyText.includes(secretValue), 'secret masked in the history file');
await evaluate(
  `[...document.querySelectorAll('.sidebar__view')].find((tab) => tab.textContent.includes('${'Verlauf'}') || tab.textContent.includes('History')).click()`,
);
const historyCount = await waitFor(
  `document.querySelectorAll('.history-list__entry').length || null`,
  'history entries',
);
check(historyCount > 10, `history lists the sent requests (${historyCount})`);
await evaluate(`document.querySelector('.history-list__entry').click()`);
await waitFor(text('.response__status'), 'response of the history entry');
check(true, 'a history entry opens as a new tab with its response');
await evaluate(
  `[...document.querySelectorAll('.sidebar__view')].find((tab) => !tab.classList.contains('sidebar__view--active')).click()`,
);

// --- 19. Params --------------------------------------------------------------------

// A path variable gets its value in the params table; a query parameter
// switched off there leaves the URL and is not sent.
await evaluate(setValue('.url-bar__input', `${echoUrl}/:id?keep=1&drop=2`));
await evaluate(`document.querySelectorAll('.request-editor__section')[0].click()`);
await waitFor(`!!document.querySelector('.params-editor__input')`, 'path variable field');
await evaluate(setValue('.params-editor__input', '42'));
await evaluate(
  `document.querySelectorAll('.params-editor .rows-editor input[type=checkbox]')[1].click()`,
);
const withoutDrop = await waitFor(
  `(() => { const url = document.querySelector('.url-bar__input').value; return url.includes('drop') ? null : url; })()`,
  'query parameter switched off',
);
check(withoutDrop.endsWith('/:id?keep=1'), `switched-off parameter left the URL (${withoutDrop})`);
await evaluate(`document.querySelector('.url-bar button[type=submit]').click()`);
await echoWith('/echo/42?keep=1', 'echo with the path variable filled in');
check(true, 'path variable replaced when sending');

finish(null);
