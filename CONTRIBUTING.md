# Contributing

Thanks for helping! Open items and ideas are
[GitHub issues](https://github.com/Daedaleus/api_client/issues); please use the
issue forms (Bug report, Idea).

## Prerequisites

- [rustup](https://rustup.rs). The Rust version is pinned in
  `rust-toolchain.toml`; rustup installs it (with rustfmt, clippy and
  llvm-tools) automatically on the first `cargo` call.
- [mise](https://mise.jdx.dev) for the other tools and the tasks
  (Windows: `winget install jdx.mise`). The versions are pinned in
  `mise.toml`, including Node.js; npm packages come via `npm ci` (done by the
  tasks) and the Tauri CLI as a dev dependency.
- Linux only, for Tauri: `libwebkit2gtk-4.1-dev libxdo-dev libssl-dev
  libayatana-appindicator3-dev librsvg2-dev` (Debian/Ubuntu names).

Once per clone:

```sh
git config core.hooksPath .githooks   # commit message hook
mise trust                            # allow this repo's mise.toml
mise install                          # tools in the pinned versions
```

## Build and test

The same tasks run locally and in CI, on Windows, Linux and macOS:

| Command | What it does |
|---|---|
| `mise run dev` | Desktop app in development mode with hot reload |
| `mise run ci` | Everything CI checks – run it before every commit |
| `mise run fmt` | Format all code (Rust and frontend) |
| `mise run lint` | Formatting, clippy with `-D warnings`, svelte-check, ESLint, Prettier |
| `mise run test` | All tests (cargo and Vitest) |
| `mise run coverage` | Line coverage, at least 90 % each; reports in `target/llvm-cov/html/` and `apps/desktop/coverage/` |
| `mise run audit` | Dependencies: advisories, licenses, sources (`deny.toml`) |
| `mise run bindings` | Regenerate `apps/desktop/src/lib/bindings.ts` after changing Rust types the UI uses (`lint` fails while it is stale) |
| `mise run e2e` | Windows: release build, then drives the real window (send to a local server and to a closed port) – part of `ci` |
| `mise run release` | Release build of the desktop app (`target/release/`) |
| `mise run install` | Windows: release build, copy to `dist\` and a desktop shortcut "API Client" |
| `mise run changelog` | Preview of the changelog for unreleased commits |

`mise tasks` lists them all. Without mise, the plain `cargo` and `npm`
commands in `mise.toml` work as well.

## Structure

A Cargo workspace; details and dependency rules in
[docs/architecture.md](docs/architecture.md).

| Path | Content |
|---|---|
| `crates/ac_domain` | Domain model: requests, responses, protocol types. No I/O. |
| `crates/ac_app` | Application layer: commands, events, view models, ports; test doubles behind the `testing` feature. |
| `crates/ac_http` | HTTP adapter (reqwest). |
| `crates/…` | Further crates join with their milestone (storage, secrets, scripting). |
| `apps/desktop` | Desktop app: Svelte 5 + TypeScript frontend (`src/`), Tauri shell (`src-tauri/`, crate `ac_desktop`). Only wiring and presentation. |
| `locales/app.json` | All display texts in German and English, shared by Rust and the frontend. |
| `spikes/` | Time-boxed experiments, not part of the workspace. |
| `docs/` | Architecture, file format, decision records (`docs/adr/`). |

Rust tests live per module in `src/tests/<module>.rs` and are included with
`#[cfg(test)] #[path = "tests/<module>.rs"] mod tests;`. Frontend tests sit
next to the code as `*.test.ts` (Vitest, Testing Library); they use a fake
`Api` instead of the Tauri backend. Tests touch neither the internet nor real
user data.

## Commit messages

[Conventional Commits](https://www.conventionalcommits.org) with the matching
[gitmoji](https://gitmoji.dev) **after** the colon, in English, description in
lower case without a trailing period:

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

The hook `.githooks/commit-msg` checks this locally, the workflow
`pr-title.yml` checks pull request titles.

## Pull requests

`main` is protected: every change goes through a pull request and is squash
merged, so the **PR title becomes the commit message**. Please fill in the PR
checklist.
