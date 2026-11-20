# Notes for working on this project

Project-specific rules (section "Project" at the end) take precedence over the
general rules.

> **Status:** milestone M1 (walking skeleton). Architecture, crates,
> milestones and spikes: [docs/architecture.md](docs/architecture.md).
> Building and conventions: [CONTRIBUTING.md](CONTRIBUTING.md). Decisions
> marked *(open)* are still to be made together with the maintainer.

Open items and ideas are GitHub issues (`gh issue list`), grouped by GitHub
milestones (M1–M6, "Spikes and decisions") – never a TODO file in the repo.
Every change goes through a pull request with auto-merge (squash); link the
issue it closes ("Closes #N").

## Collaboration

- Talk to the maintainer **in German**; short, concrete, honest. Report
  results as they are: say clearly what is not verified or not finished.
- **Commit and push only when asked.** An approval covers the discussed
  workflow, not everything that follows.
- Decisions that belong to the maintainer (scope, costs, publishing,
  deleting, renaming, account/repo settings): ask briefly, with a
  recommendation. Decide everything with a clear default yourself and mention it.
- Before steps that stop a running application or touch user data, announce
  them or ask first.
- If a refactoring gets disproportionately large, stop and state the current
  status honestly instead of muddling on.
- Reviews "through the eyes of a software architect": assessment, prioritized
  points, then implement step by step and tick them off.
- Never pass personal data (e-mail, names, paths) to third-party services and
  never write it into public files.

## Language in code and docs

- README, CONTRIBUTING, SECURITY, this file, issues, PR texts and commit
  messages in **English**.
- **Code comments and doc comments in English**, as are identifiers (types,
  functions, variables, files).
- **Bilingual interface (German/English)** via i18n: language taken from the
  operating system, changeable in the settings. Display texts only in the
  translation file, never as string literals in code. New texts always in all
  languages; a test checks that every key has all languages and the same
  placeholders, and that every used key exists.
- Texts short and understandable for casual users; technical terms (request,
  header, environment, collection …) as established in API tooling.
- Create translated texts only when displaying, never store them in fields –
  otherwise the old language remains after a language switch.

## Code quality and domain

These principles apply **regardless of the programming language**; the
language only determines how they are implemented idiomatically.

- **Principles:** Clean Code (meaningful names, small units with one job),
  SOLID, DRY, KISS, YAGNI, separation of concerns, loose coupling and high
  cohesion, composition over inheritance.
- **Domain first** (Domain-Driven Design where it fits): domain terms as own
  types, modules by domain instead of technical layers, one shared language
  between code and docs.
- **Architecture:** domain logic independent of UI, storage and network
  (Ports and Adapters / Clean Architecture); dependencies point inwards,
  external services behind interfaces so they can be replaced in tests.
- Briefly justify far-reaching decisions (code comment or docs) so they stay
  traceable.
- **No magic strings/numbers:** define domain terms once as a type (enums,
  newtypes, constants) and never compare via display texts. Identifiers as own
  types instead of loose numbers (e.g. `struct RequestId(Uuid)`) where they
  could be mixed up; a deliberate alias is justified in the code.
- Values from foreign protocols/formats (HTTP methods, status codes, header
  names, content types, import formats) bundled as types in their own module,
  not scattered as literals.
- Rule of thumb: files < ~400 lines, functions < ~100 lines, otherwise split by
  responsibility. Large modules as folders with submodules; the module file
  describes the structure at the top.
- UI is presentation and wiring only; logic lives in domain modules so it stays
  testable. The UI library only in the UI layer.
- Compute nothing expensive per frame/request: cache results (e.g. generation
  counter), long work in the background, result back via message. One fixed
  background worker/runtime instead of one per action.

## User interface

- **Atomic Design** for UI elements: atoms (button, label, icon), molecules
  (URL bar from method picker, input and send button), organisms (request
  editor, response viewer, collection tree), templates and pages. Reusable
  building blocks first; pages only compose them.
- **BEM** for CSS classes where CSS is used: `block__element--modifier`; no
  nesting via element selectors, no IDs for styling.
- Design values (colors, spacing, font sizes) centrally as tokens/variables;
  light and dark theme.
- Accessibility: contrast, full keyboard operation (shortcuts for send, new
  request, switch environment), meaningful labels.

## Error handling

- One-off errors and command line: generic error type (`anyhow`).
- Where the caller distinguishes kinds: own error enums (`thiserror`). Their
  text stays technical/English; translation only when displayed
  (`user_text()` or similar).
- Never swallow errors: report write and network errors. A failed request is a
  normal result for the user (show status, timing, error details), not a crash.

## User data

- **Local first:** collections, environments and history stay on the user's
  machine; nothing about the user is sent to third parties. No account, no
  telemetry, no cloud sync.
- **Never lose user data:** back up damaged files instead of overwriting them,
  never overwrite unreadable ones. Caches may be discarded, user data not.
- Version stored formats (field `version`) so later changes can be migrated.
- **Secrets** (tokens, passwords, API keys in environments) never in plain text
  in files meant for git and masked in the UI and in logs. They are encrypted
  with **[age](https://age-encryption.org/)** (Rust crate `age`), so a
  collection can be encrypted for **multiple recipients** (team members, CI)
  and the encrypted files can be committed safely. Private keys stay on the
  user's machine, never in the collection folder.

## Network

- HTTP sending behind an interface (trait) so tests can replace it; one shared
  client per application.
- The user decides what is sent: no hidden requests, no added tracking
  headers. Default User-Agent with program name and version, overridable.
- TLS verification on by default; switching it off (per request/collection) is
  clearly visible.

## Tests

- New logic gets tests. **At least 90 % line coverage** excluding UI and
  program start; a script measures it, CI enforces it.
- Tests touch **neither the internet nor real user data**: HTTP against a local
  test server or a fake; time via an injectable clock; temporary folders.
- Contract tests against real services allowed only as `#[ignore]` / manual.
- Test data and helpers central (`testutil/`), tests separated per module.
- **Frontend (TypeScript/Svelte) is tested too:** `svelte-check` with strict
  TypeScript, ESLint/Prettier, Vitest for logic (stores, i18n, editor glue;
  also ≥ 90 % lines) and Testing Library for component behaviour; the Rust
  side is replaced by Tauri's `mockIPC`. A few end-to-end flows via
  WebdriverIO + `tauri-driver`.
- **Tool versions are pinned:** Rust in `rust-toolchain.toml` (rustup), all
  other tools (Node from M1, cargo-llvm-cov, cargo-deny, git-cliff) in
  `mise.toml`; npm packages via committed lock file and `npm ci`, Tauri CLI as
  dev dependency. CI reads the same files.

## Before every commit

- `mise run ci`: formatting, clippy with `-D warnings`, tests, coverage. The
  tasks live in `mise.toml` and are the single place for build commands – CI
  calls the same tasks. Keep tasks plain commands (no shell features), so
  they work in sh, cmd and PowerShell.

## Git workflow

- `main` is protected (ruleset) once the repo is on GitHub: **every change via
  branch and pull request**. No force push, no deleting. Required checks:
  formatting, builds of the main platforms, test coverage, PR title.
- **Squash merge only**; the PR title becomes the commit message. Delete merged
  branches automatically. Auto-merge allowed when the maintainer agrees.
- **Commit messages and PR titles:** Conventional Commits with the matching
  gitmoji **after** the colon, English, description lower case without a
  trailing period:

  ```
  <type>(<optional scope>): <icon> <description>
  feat(http): ✨ add multipart form bodies
  ```

  | Type | Icon | Type | Icon |
  |---|---|---|---|
  | `feat` | ✨ | `perf` | ⚡️ |
  | `fix` | 🐛 | `test` | ✅ |
  | `docs` | 📝 | `build` | 📦️ |
  | `style` | 🎨 | `ci` | 👷 |
  | `refactor` | ♻️ | `chore` | 🔧 |
  | `revert` | ⏪️ | Breaking | `!` before the colon |

  Release commits: `chore(release): 🔧 vX.Y.Z`.
- Checked locally by hook (`.githooks/commit-msg`, enabled with
  `git config core.hooksPath .githooks`) and in CI on the PR title with the
  same script. Hook scripts with LF (`.gitattributes`) and byte-wise comparison
  (`LC_ALL=C`), otherwise emojis fail under Git for Windows.
- Dependabot commits as `build(deps)` / `ci(deps)`; don't bundle large version
  jumps automatically.
- Open items and ideas as **GitHub issues**, not as a TODO file, created
  following the repo's issue forms (same `### …` sections in the same order,
  empty fields as `_No response_`, the form's labels). Bugs = "Bug report",
  everything else = "Idea".

## CI (GitHub Actions)

- On push to `main` and on PRs: formatting, lint, tests, release build on
  Windows, Linux and macOS; coverage once (Linux). Windows and Linux required,
  macOS optional.
- Generous time limit (an uncached Windows run takes > 30 min).
- Weekly dependency audit (`cargo-deny`, exceptions only with justification in
  the config); Dependabot for crates and actions.
- Actions pinned to a major version; current versions.

## Releases

- Bump the version via PR, then tag the merged commit on `main` with `vX.Y.Z`
  and push the tag; CI builds and publishes and checks tag and version match.
- Every shipped file: **virus scan** (Microsoft Defender for Windows, ClamAV
  for all), **SHA-256** per file plus `SHA256SUMS.txt`, **signed provenance**
  (`actions/attest-build-provenance`, verifiable with `gh attestation verify`).
- Release notes: "What's changed" from commit messages (git-cliff,
  `cliff.toml`, grouped by type with icon), then download and verification
  instructions. Preview: `git cliff --unreleased`.

## Public repository – checklist

- Before: check files and full history for secrets and personal data; decide
  the commit author address deliberately.
- `LICENSE` (compatible with the dependencies), README following GitHub
  conventions, `CONTRIBUTING.md`, `SECURITY.md`, issue forms (bug, idea; blank
  issues off, security reports private), PR template with checklist,
  description and topics.
- Screenshots generated by the application itself (own mode), reproducible and
  without changing settings.
- After going public: private vulnerability reporting, Dependabot alerts and
  security updates, secret scanning with push protection, CodeQL, ruleset for
  `main`.

## Windows pitfalls

- PowerShell scripts as UTF-8 **with BOM** and CRLF, otherwise Windows
  PowerShell 5.1 shows umlauts wrongly. Write JSON files without BOM.
- No `2>&1` on native programs in PowerShell 5.1 (counted as an error).
- Don't change code via shell/Python heredocs when backslashes occur – use the
  edit tool and check the result.
- Git Bash on Windows runners has `sha256sum` but no `shasum` (macOS the
  other way round).
- Never start GUI programs without a console from scripts without a time limit;
  use the debug build with command line arguments for output.

## Rust

- Edition 2024, Cargo workspace: library crates for the domain plus a thin
  program (`main.rs` only starts); diagnostic/CLI in its own module or crate.
- Tests per module in `src/tests/<module>.rs`, included via
  `#[cfg(test)] #[path = "tests/<module>.rs"] mod tests;`.
- Coverage with `cargo-llvm-cov` via `mise run coverage`.

## Project

- **Purpose:** a fast, fully open-source API client in Rust – an alternative
  to Bruno / Postman. **No premium tier, no paid features, no account, no
  cloud**: every feature is available to everyone.
- **Target group:** developers and testers who build, debug and document
  HTTP APIs and want their collections versioned in git next to their code.
- **Core ideas (planned):**
  - Collections as plain, human-readable text files in a folder (git-friendly,
    diffable, one file per request) – no proprietary database. **Own file
    format** (not bound to Bruno's `.bru`); versioned, documented in the repo,
    compatibility with other tools only via import/export.
  - **Scripting in Python** (pre-request, post-response, tests). The app
    offers editor support like Bruno/Postman: a **checker** (syntax and lint
    errors, unknown API names) and **code completion** for the script API
    (`req`, `res`, `env` …). The script API is described once as typed stubs
    (`.pyi`), used both by the checker/completion and as documentation.
    Scripts run sandboxed: no file or network access beyond the provided API
    unless the user allows it.
  - Requests: methods, URL with path/query params, headers, bodies (JSON,
    text, form, multipart, binary), auth (Basic, Bearer, API key, OAuth 2).
  - Environments and variables with `{{variable}}` interpolation; secrets
    encrypted with age.
  - Response viewer: status, timing, size, headers, pretty-printed body.
  - Import from Postman and Bruno collections, OpenAPI; export to cURL.
  - Later: GraphQL, WebSocket, gRPC, a CLI runner for CI.
- **Architecture (draft):** domain crate (collection, request, environment,
  variable resolution – no I/O), storage crate (file format, migration),
  secrets crate (age encryption, recipients), HTTP crate (sending, behind a
  trait), scripting crate (Python runtime, sandbox, checker/completion behind
  traits), import/export crate, app crate (UI), CLI crate.
- **Replaceable UI:** the user interface must be swappable. Everything below
  the UI (domain, storage, secrets, HTTP, scripting incl. checker/completion)
  lives in UI-agnostic crates; an application crate exposes use cases
  (commands, queries, events) and view models as plain Rust types. A UI crate
  only renders view models and sends commands – no domain logic, no direct
  file or network access. A second UI (e.g. egui instead of Tauri) must be
  possible without touching the crates below.
- **UI framework (current direction):** Tauri 2 with a web frontend and
  CodeMirror 6 as script editor; egui remains the fallback for a pure-Rust UI.
  Not final until a prototype has been evaluated.
- **License:** MIT OR Apache-2.0 (dual). Dependencies must be compatible
  (checked by `cargo-deny`, `deny.toml`).
- **Open decisions** *(open)*: final UI framework, project name, Python runtime
  (embedded pure-Rust interpreter such as RustPython vs. CPython via PyO3 –
  trade-off: no system Python needed and easier sandboxing vs. full
  compatibility and libraries) and the tooling for checker/completion (e.g.
  Ruff for linting, a Python language server or own completion from the
  stubs).
- **Name caution:** "Bruno" and "Postman" are third-party names – use them only
  to describe compatibility/import, not as or in the project name or branding.
- **Repository and branch:** <https://github.com/Daedaleus/api_client>
  (public), branch `main`. "api_client" is the repository name; the product
  name is still open.
