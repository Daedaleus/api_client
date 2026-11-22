# api_client documentation

A fast, fully open-source desktop API client written in Rust. Collections
are plain files in a folder, made to live in git next to your code. No
account, no cloud, no premium tier – everything stays on your machine.

## For users

- **[User manual](manual/README.md)** – getting started, requests,
  collections, environments and secrets, import, history, shortcuts.
- [Scripts and tests](scripting.md) – TypeScript before and after a request.
- [Command line runner](cli.md) – running a collection in CI.
- [Collection file format](format/v1.md) – the TOML files on disk.

## For contributors

- [Architecture](architecture.md) – crates, milestones, decisions.
- Decisions: [script runtime](adr/s1-script-runtime.md),
  [script checker](adr/s2-script-assist.md), [Tauri](adr/s3-tauri.md),
  [age keys](adr/s4-age-keys.md).
- [Contributing](https://github.com/Daedaleus/api_client/blob/main/CONTRIBUTING.md)
  and the [source code](https://github.com/Daedaleus/api_client).
