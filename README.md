# api_client

[![CI](https://github.com/Daedaleus/api_client/actions/workflows/ci.yml/badge.svg)](https://github.com/Daedaleus/api_client/actions/workflows/ci.yml)
[![Audit](https://github.com/Daedaleus/api_client/actions/workflows/audit.yml/badge.svg)](https://github.com/Daedaleus/api_client/actions/workflows/audit.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

A fast, fully open-source desktop API client written in Rust – an
alternative to tools like Bruno or Postman. **No premium tier, no account,
no cloud:** every feature is available to everyone, and your collections stay
on your machine.

> **Status:** early development. There is nothing to download yet. The
> product name is not final; `api_client` is the working name.

## Planned features

- **Collections as plain files** – one human-readable TOML file per request,
  made to be versioned in git next to your code.
- **Requests** – all methods, path and query parameters, headers, bodies
  (JSON, text, form, multipart, binary) and auth (Basic, Bearer, API key,
  OAuth 2).
- **Environments and variables** with `{{variable}}` interpolation.
- **Secrets encrypted with [age](https://age-encryption.org/)** for several
  recipients, so a team (and CI) can share them safely through git.
- **TypeScript scripts and tests** before and after a request
  ([`docs/scripting.md`](docs/scripting.md)), sandboxed; a checker and code
  completion in the editor follow.
- **Import** from Postman, Bruno and OpenAPI; **export** to cURL.
- **Command line runner** for CI ([`docs/cli.md`](docs/cli.md)).
- Interface in **English and German**, light and dark theme.

## Privacy and security

- No telemetry, no account, no sync – nothing about you leaves your machine.
- The app only sends the requests you send.
- Secrets are never stored in plain text in collection files.

Found a vulnerability? Please see [SECURITY.md](SECURITY.md).

## Contributing

Building, tests and conventions: [CONTRIBUTING.md](CONTRIBUTING.md).
Architecture: [docs/architecture.md](docs/architecture.md). Ideas and bugs go
into [issues](https://github.com/Daedaleus/api_client/issues).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

"Bruno" and "Postman" are names of third-party products; this project is not
affiliated with them.
