# Contributing

Thanks for helping! Open items and ideas are
[GitHub issues](https://github.com/Daedaleus/api_client/issues); please use the
issue forms (Bug report, Idea).

## Prerequisites

- [rustup](https://rustup.rs). The Rust version is pinned in
  `rust-toolchain.toml`; rustup installs it (with rustfmt, clippy and
  llvm-tools) automatically on the first `cargo` call.
- Optional for coverage: `cargo install cargo-llvm-cov --locked`
- From milestone M1 (desktop app): Node.js in the version from
  `.node-version` (any version manager that reads it: fnm, nvm, Volta,
  mise); packages via `npm ci`, the Tauri CLI comes as a dev dependency.

Once per clone, enable the commit message hook:

```powershell
git config core.hooksPath .githooks
```

## Build and test

```powershell
.\build.ps1             # formatting, clippy (-D warnings), tests
.\build.ps1 -Coverage   # additionally test coverage (at least 90 %)
.\coverage.ps1 -Open    # coverage report in the browser
```

On Linux/macOS run the same steps directly:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

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
