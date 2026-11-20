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
  `mise.toml`; from milestone M1 this includes Node.js, npm packages come via
  `npm ci` and the Tauri CLI as a dev dependency.

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
| `mise run ci` | Everything CI checks – run it before every commit |
| `mise run fmt` | Format all code |
| `mise run lint` | Check formatting, clippy with `-D warnings` |
| `mise run test` | All tests |
| `mise run coverage` | Line coverage, at least 90 %; report in `target/llvm-cov/html/index.html` |
| `mise run audit` | Dependencies: advisories, licenses, sources (`deny.toml`) |
| `mise run release` | Release build |
| `mise run changelog` | Preview of the changelog for unreleased commits |

`mise tasks` lists them all. Without mise, the plain `cargo` commands in
`mise.toml` work as well.

## Structure

A Cargo workspace; details and dependency rules in
[docs/architecture.md](docs/architecture.md).

| Path | Content |
|---|---|
| `crates/ac_domain` | Domain model: requests, environments, variables. No I/O. |
| `crates/…` | Further crates join with their milestone (application layer, HTTP, storage, secrets, scripting). |
| `apps/` | User interfaces (desktop app, later command line). Only wiring and presentation. |
| `spikes/` | Time-boxed experiments, not part of the workspace. |
| `docs/` | Architecture, file format, decision records (`docs/adr/`). |

Tests live per module in `src/tests/<module>.rs` and are included with
`#[cfg(test)] #[path = "tests/<module>.rs"] mod tests;`. Tests touch neither
the internet nor real user data.

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
