# Architecture

Status: agreed direction before milestone M1. Decisions with lasting impact
are recorded as ADRs in [`docs/adr/`](adr/).

## Goals

- **Replaceable user interface.** Everything below the UI is UI-agnostic; a
  second UI (e.g. egui instead of Tauri) must not require changes below the
  application layer.
- **Testable core.** Domain logic is pure; I/O sits behind traits (ports)
  with fakes for tests.
- **Small start (YAGNI).** Crates appear with the milestone that needs them.

## Workspace and dependency direction

Local crates use the neutral prefix `ac_` until the product name is chosen.

```
           apps/desktop (ac_desktop, Tauri)   apps/cli (ac_cli)   [ac_egui, fallback]
                          \                        |                   /
                           +---- composition roots: wire adapters into ac_app ----+
                                       |                     |
   adapters:   ac_storage   ac_http   ac_secrets   ac_scripting   ac_interchange
                    \          |          |             |              /
                     +-------- implement the ports defined in --------+
                                        ac_app   use cases, commands/queries/events,
                                          |      view models, ports (traits)
                                        ac_domain  pure: types, rules, interpolation
```

- Dependencies point **inwards only**. `ac_domain` has no internal
  dependencies, no I/O and no async. `ac_app` depends only on `ac_domain`.
- Adapters depend on `ac_app`/`ac_domain`, never on each other or on a UI.
- Only composition roots (`apps/*`) know all crates.
- Test doubles for the ports (`FakeHttpSender`, later `FixedClock`, …) live
  in `ac_app::testing`, compiled for `ac_app`'s own tests and for other
  crates via the `testing` feature. A separate test-util crate would see a
  different copy of the port traits in `ac_app`'s unit tests.

| Crate | Milestone | Responsibility |
|---|---|---|
| `ac_domain` | M0 | Collection, folder, request, environment, variable, response, protocol types, `{{var}}` resolution |
| `ac_app` | M1 | Facade, use cases, ports, view models, events |
| `ac_http` | M1 | `HttpSender` adapter (reqwest), one shared client |
| `ac_desktop` | M1 | Tauri app: thin commands, event forwarding |
| `ac_storage` | M2 | File format, migrations, backups |
| `ac_secrets` | M3 | age encryption, recipients, key store |
| `ac_scripting` | M4 | Python runtime, sandbox, checker and completion |
| `ac_interchange` | M6 | Import (Postman, Bruno, OpenAPI), export (cURL) |
| `ac_cli` | M6 | Command line runner for CI |

## Application layer (`ac_app`)

An `App` facade built from `Ports`:

- `execute(Command) -> Result<CommandAck, AppError>` – changes state.
- `query(Query) -> Result<QueryResult, AppError>` – reads state.
- `subscribe() -> Receiver<AppEvent>` – results of long work (sending,
  scripts) arrive as events; commands return a `RunId` immediately.

**Open collections:** the app keeps only which collections are open (id and
folder). Every collection command reads the collection anew from the store,
acts on the files the store reports and answers with the tree as it is
afterwards – so changes made outside the app (an editor, `git pull`) show up
without a watcher, and there is no cache to get out of step.

Ports (traits) live in `ac_app::ports`: `HttpSender` and `CollectionStore`
today; later `SecretVault`, `KeyStore`, `ScriptEngine`, `ScriptAssist`
(diagnostics and completion), `Clock`. Their test doubles
(`FakeHttpSender`, `MemoryStore`) live in `ac_app::testing`.

View models are plain serde structs. Display texts travel as
`UserText { key, args }` and are translated only in the UI, so a language
switch also covers messages that are already shown.

## Desktop UI (Tauri)

- `apps/desktop/src-tauri` (`ac_desktop`): `main.rs` only starts; each Tauri
  command is a thin call into `ac_app`; events are forwarded to the window.
  Tauri's tokio runtime is the single background runtime.
- `apps/desktop/src`: Svelte 5 + Vite + TypeScript (strict), components by
  Atomic Design, BEM CSS, design tokens for light/dark, CodeMirror 6 as
  editor (planned).
- UI state lives in plain classes with Svelte runes in `src/lib/workspace/`:
  `TabsState` (editor tabs, routing run results to the tab that started the
  run), `CollectionsState` (open trees, remembered for the next start),
  `Dialogs` (questions to the user, awaited as promises) and `Workspace`
  (actions that touch several of them). Components only render and call
  them; tests drive them through a fake `Api`.
- **Typed boundary:** `tauri-specta` generates `src/lib/bindings.ts` from the
  Rust types (behind an optional `specta` feature in `ac_app`). The program
  `ac_desktop --bin bindings` writes it (`mise run bindings`) or checks it
  (`--check`, part of `mise run lint` and CI). A program, not a unit test:
  test binaries linking Tauri cannot start on Windows (missing application
  manifest), so `ac_desktop` has no unit tests – its logic lives in
  `ac_app`. The UI imports the types through `src/lib/types.ts`.
- Large response bodies: metadata plus a preview in the view model, the raw
  body via a binary IPC response – never one huge JSON string.
- **egui fallback:** `apps/egui` would build the same `App` with the same
  adapters and drain `AppEvent`s each frame.

## Domain model (sketch)

- IDs as newtypes over UUIDv7 (`CollectionId`, `ItemId`, `EnvironmentId`),
  stored in the files so they survive renames.
- `Request`: `HttpMethod`, URL template, params, headers (with `enabled`),
  `Body` (none, JSON, text, form, multipart, binary file reference), `Auth`
  (inherit, none, Basic, Bearer, API key, OAuth 2), scripts (pre-request,
  post-response, tests), settings.
- `Variable` value is `Plain(String)` or `Secret(SecretRef)`; secret values
  are zeroized in memory and masked in `Debug`/`Display`.
- Variable precedence: runtime > request > folder > environment > collection.
- A failed request is a normal outcome (`RunOutcome::Failed`), not an error
  that crashes anything.

## File format

TOML, one file per request, comments and order preserved on save
(`toml_edit`). Every file carries `version`; loading migrates in memory, files
are rewritten only when the user saves, atomically (temporary file, then
rename). Damaged files are reported, skipped and never overwritten; files from
a newer version open read-only. Specification: [`docs/format/v1.md`](format/v1.md);
implementation: `ac_storage`.

```
my-api/
  collection.toml        # format, version, defaults, secrets policy
  .age-recipients        # public keys (age1…, ssh-ed25519 …)
  environments/dev.toml  # variables; secrets only as a marker
  secrets/dev.age        # age-encrypted values
  users/folder.toml      # folder settings and item order
  users/get-user.toml    # one request
```

## Spikes before the big decisions

Time-boxed (2–3 days) in `spikes/<topic>` on a `spike/<topic>` branch; the
result is an ADR, spike code is never merged into the crates.

1. **S1 Python runtime:** RustPython vs. CPython (PyO3): size, start time,
   timeouts, sandbox, stdlib coverage, distribution.
2. **S2 Checker and completion:** own implementation from `.pyi` stubs plus a
   Python parser vs. a bundled language server.
3. **S3 Tauri reality check:** Linux WebKitGTK, large responses, bundling –
   go/no-go for Tauri vs. egui.
4. **S4 age keys (ADR):** identity location, SSH keys as recipients,
   re-encryption on recipient changes, CI identity.

## Milestones

- **M0** Repository foundation: hooks, CI, templates, scripts, workspace.
- **M1** Walking skeleton: send a request from the Tauri UI end to end.
- **M2** Collections on disk.
- **M3** Environments, interpolation, secrets with age.
- **M4** Python scripting with checker and completion.
- **M5** Bodies and auth.
- **M6** Import/export and command line runner.
